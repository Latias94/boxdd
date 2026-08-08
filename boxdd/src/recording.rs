//! Owned Box2D recording sessions and opaque process-local recordings.

#[cfg(test)]
use crate::core::world_core::ActivityState;
use crate::core::world_core::{RecordingActivityLease, WorldCore};
use crate::{
    BodyBuilder, BodyDef, BodyId, ChainId, Counters, Error, JointId, Result, ShapeId, Vec2, World,
};
use boxdd_sys::ffi;
use core::fmt;
use core::ptr::NonNull;

mod session_joints;
mod session_world_body;

#[cfg(test)]
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_SIZE_CHECK: Cell<bool> = const { Cell::new(false) };
    static FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS: Cell<Option<usize>> = const { Cell::new(None) };
    static RECORDING_GET_SIZE_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn check_native_recording_gravity(gravity: Vec2) -> Result<Vec2> {
    if gravity.is_valid() {
        Ok(gravity)
    } else {
        Err(Error::InvalidNativeOutput {
            operation: "RecordingSession::gravity",
            output: "gravity",
            constraint: "a finite vector",
        })
    }
}

/// Hard total-byte limit for one native recording session.
///
/// The native writer preallocates only a small prefix and grows on demand up to this limit. The
/// repository-wide safety policy caps every producer at 256 MiB.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RecordingLimits(i32);

impl RecordingLimits {
    /// Hard upper bound enforced by the reviewed native writer.
    pub const MAX_BYTES: u32 = 256 * 1024 * 1024;
    /// Default limits for ordinary recordings.
    pub const DEFAULT: Self = Self(Self::MAX_BYTES as i32);

    /// Construct a hard byte limit after validating the native writer policy.
    pub fn new(max_bytes: u64) -> Result<Self> {
        if max_bytes == 0 || max_bytes > u64::from(Self::MAX_BYTES) {
            return Err(Error::invalid_argument(
                "RecordingLimits::new",
                "max_bytes",
                "an integer in 1..=268435456",
            ));
        }
        Ok(Self(i32::try_from(max_bytes).map_err(|_| {
            Error::invalid_argument(
                "RecordingLimits::new",
                "max_bytes",
                "an integer representable by a positive native int",
            )
        })?))
    }

    /// Return the hard total native-stream limit in bytes.
    pub const fn max_bytes(self) -> u32 {
        self.0 as u32
    }

    #[inline]
    const fn as_i32(self) -> i32 {
        self.0
    }
}

impl Default for RecordingLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Stable application-defined identity for one material-mixing behavior version.
///
/// The identity describes behavior, not a callback address or Rust type. Change it whenever the
/// deterministic mixing rule or any data it depends on changes.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MixerId([u8; 32]);

impl MixerId {
    /// Construct a caller-declared behavior identity from application-owned stable bytes.
    ///
    /// The wrapper checks exact equality across recording and replay. It does not inspect callback
    /// code or cryptographically prove that two callbacks with the same identifier behave alike.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the stable identity bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for MixerId {
    fn from(value: [u8; 32]) -> Self {
        Self::from_bytes(value)
    }
}

/// Caller-declared identities of the optional friction and restitution mixers.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct MixerIdentities {
    friction: Option<MixerId>,
    restitution: Option<MixerId>,
}

impl MixerIdentities {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const fn new(friction: Option<MixerId>, restitution: Option<MixerId>) -> Self {
        Self {
            friction,
            restitution,
        }
    }

    /// Return the friction mixer identity, if one is installed.
    pub const fn friction(self) -> Option<MixerId> {
        self.friction
    }

    /// Return the restitution mixer identity, if one is installed.
    pub const fn restitution(self) -> Option<MixerId> {
        self.restitution
    }

    /// Whether both Box2D default mixing rules are in use.
    pub const fn is_empty(self) -> bool {
        self.friction.is_none() && self.restitution.is_none()
    }

    fn capture(core: &WorldCore) -> Self {
        core.mixer_identities()
    }
}

/// An opaque, process-local Box2D recording accepted by [`crate::ReplayPlayer`].
///
/// Box2D recording format version 3 contains raw native object representations and is not a
/// portable or stable serialization format. Safe Rust therefore neither imports nor exports its
/// bytes. A recording can only be produced by [`crate::RecordingSession::finish`] and replayed by
/// this build in the current process.
pub struct Recording {
    native: Box<[u8]>,
    mixer_identities: MixerIdentities,
    preflight: crate::replay::preflight::PreflightInfo,
}

impl Recording {
    /// Return the exact material-mixer behavior identities required for replay.
    pub const fn mixer_identities(&self) -> MixerIdentities {
        self.mixer_identities
    }

    pub(crate) fn native_stream(&self) -> &[u8] {
        &self.native
    }

    pub(crate) const fn preflight_info(&self) -> crate::replay::preflight::PreflightInfo {
        self.preflight
    }

    fn from_native(native: &[u8], mixer_identities: MixerIdentities) -> Result<Self> {
        let preflight = crate::replay::preflight::validate_recording(native)
            .map_err(|_| Error::RecordingOutputValidationFailed)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(native.len())
            .map_err(|_| Error::RecordingStorageAllocationFailed)?;
        owned.extend_from_slice(native);
        Ok(Self {
            native: owned.into_boxed_slice(),
            mixer_identities,
            preflight,
        })
    }
}

impl fmt::Debug for Recording {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Recording")
            .field("mixer_identities", &self.mixer_identities)
            .finish_non_exhaustive()
    }
}

struct NativeRecording {
    raw: NonNull<ffi::b2Recording>,
    max_bytes: usize,
    #[cfg(test)]
    lifecycle_probe: Option<Arc<RecordingLifecycleProbe>>,
}

#[cfg(test)]
#[derive(Default)]
struct RecordingLifecycleProbe {
    stops: AtomicUsize,
    destroys: AtomicUsize,
}

impl NativeRecording {
    fn new(limits: RecordingLimits) -> Result<Self> {
        let raw = unsafe { ffi::b2CreateRecording(limits.as_i32()) };
        let raw = NonNull::new(raw).ok_or(Error::RecordingAllocationFailed)?;
        Ok(Self {
            raw,
            max_bytes: limits.max_bytes() as usize,
            #[cfg(test)]
            lifecycle_probe: None,
        })
    }

    #[inline]
    fn as_ptr(&self) -> *mut ffi::b2Recording {
        self.raw.as_ptr()
    }

    #[cfg(test)]
    fn record_stop(&self) {
        if let Some(probe) = &self.lifecycle_probe {
            probe.stops.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn checked_size(&self) -> Result<usize> {
        #[cfg(test)]
        if FAIL_NEXT_SIZE_CHECK.with(|fail| fail.replace(false)) {
            return Err(Error::InvalidNativeRecording);
        }

        let size = self.read_size()?;
        if size == 0 || size > self.max_bytes {
            return Err(Error::InvalidNativeRecording);
        }
        Ok(size)
    }

    fn check_status(&self) -> Result<()> {
        self.read_size().map(|_| ())
    }

    fn read_size(&self) -> Result<usize> {
        #[cfg(test)]
        if FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| match fail.get() {
            Some(0) => {
                fail.set(None);
                true
            }
            Some(remaining) => {
                fail.set(Some(remaining - 1));
                false
            }
            None => false,
        }) {
            return Err(Error::RecordingLimitExceeded);
        }

        #[cfg(test)]
        RECORDING_GET_SIZE_CALLS.with(|calls| {
            calls.set(
                calls
                    .get()
                    .checked_add(1)
                    .expect("recording size-read test counter overflow"),
            );
        });
        let raw_size = unsafe { ffi::b2Recording_GetSize(self.as_ptr()) };
        match raw_size {
            -1 => Err(Error::RecordingLimitExceeded),
            -2 => Err(Error::RecordingOperationTooLarge),
            -3 => Err(Error::InvalidNativeRecording),
            value if value < 0 => Err(Error::NegativeFfiOutputCount { count: value }),
            value => {
                usize::try_from(value).map_err(|_| Error::NegativeFfiOutputCount { count: value })
            }
        }
    }

    fn bytes(&self) -> Result<&[u8]> {
        let size = self.checked_size()?;

        let data = unsafe { ffi::b2Recording_GetData(self.as_ptr()) };
        if data.is_null() || data.addr().checked_add(size).is_none() {
            return Err(Error::InvalidNativeRecording);
        }

        // SAFETY: Box2D reports `size` initialized bytes at a non-null pointer.
        // Recording has stopped, and `self` keeps the native allocation alive
        // for the duration of this borrow.
        Ok(unsafe { core::slice::from_raw_parts(data, size) })
    }
}

impl Drop for NativeRecording {
    fn drop(&mut self) {
        unsafe { ffi::b2DestroyRecording(self.as_ptr()) };
        #[cfg(test)]
        if let Some(probe) = &self.lifecycle_probe {
            probe.destroys.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// An active recording that exclusively borrows its wrapper-owned world.
///
/// Object operations are available only through capabilities borrowed from this session. The
/// capability types are identical to those returned by [`World`], while their sealed access
/// proof authorizes recording activity and checks the native writer after every operation.
///
/// A native writer failure is detected at the operation boundary and permanently seals the
/// session: the triggering call and every later session operation return the sticky error. Box2D
/// may already have applied the triggering operation to the world; this boundary is fail-closed
/// for the recording stream, not a transactional rollback of world state.
#[must_use = "dropping the session stops recording and discards its bytes"]
pub struct RecordingSession<'world> {
    world: &'world mut World,
    native: Option<NativeRecording>,
    activity: Option<RecordingActivityLease>,
    mixer_identities: MixerIdentities,
    stop_pending: bool,
}

impl World {
    /// Start an owned recording session at a step boundary.
    ///
    /// Box2D cannot record custom-filter or pre-solve callback decisions, so
    /// either installed callback is rejected before allocating or mutating
    /// native recording state.
    pub fn start_recording(&mut self, limits: RecordingLimits) -> Result<RecordingSession<'_>> {
        crate::world::check_world_available(self)?;
        ensure_supported_callbacks(self.core())?;

        let mixer_identities = MixerIdentities::capture(self.core());
        let native = NativeRecording::new(limits)?;
        let activity = self.core().begin_recording_activity()?;
        let session = RecordingSession {
            world: self,
            native: Some(native),
            activity: Some(activity),
            mixer_identities,
            stop_pending: true,
        };

        // Construct the complete RAII owner before entering C. If a foreign
        // panic mechanism unwinds this call after Box2D attaches the pointer,
        // `RecordingSession::drop` stops the world before freeing the buffer.
        unsafe {
            ffi::b2World_StartRecording(session.world.raw(), session.native().as_ptr());
        }
        // The native API reports failure by leaving the recording empty. Check
        // that it attached and wrote the header/snapshot before exposing a live
        // session to Safe Rust.
        session.native().checked_size()?;
        Ok(session)
    }
}

impl RecordingSession<'_> {
    fn native(&self) -> &NativeRecording {
        self.native
            .as_ref()
            .expect("an active recording session owns its native buffer")
    }

    fn activity_mut(&mut self) -> &mut RecordingActivityLease {
        self.activity
            .as_mut()
            .expect("an active recording session owns its activity guard")
    }

    fn check_access(&self) -> Result<()> {
        crate::world::check_recording_world_available(self.world)?;
        self.native().check_status()
    }

    fn stop_native(&mut self) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.world.retire_completed_step();
        if !self.stop_pending {
            return Ok(());
        }
        let status_before_stop = self.native().check_status();
        self.stop_pending = false;
        unsafe { ffi::b2World_StopRecording(self.world.raw()) };
        #[cfg(test)]
        self.native().record_stop();
        let status_after_stop = self.native().check_status();
        let activity_result = self.activity_mut().finish();
        status_before_stop?;
        status_after_stop?;
        activity_result
    }

    fn defer_native_cleanup(&mut self) {
        let stop_pending = core::mem::replace(&mut self.stop_pending, false);
        let raw_world = self.world.raw();
        self.world.retire_completed_step();
        let native = self
            .native
            .take()
            .expect("a recording session owns its native buffer until cleanup");
        let mut activity = self
            .activity
            .take()
            .expect("a recording session owns its activity guard until cleanup");
        let owner =
            crate::core::callback_state::CallbackOwnerToken::world(self.world.core().brand.token());
        // Owner frames run every detach action before any callback-transferred world owner.
        crate::core::callback_state::defer_callback_cleanup_or_forget(owner, move || {
            if stop_pending {
                unsafe { ffi::b2World_StopRecording(raw_world) };
                #[cfg(test)]
                native.record_stop();
            }
            let _ = activity.finish();
            drop(native);
        });
    }

    /// Return the mixer behavior identities captured atomically at session start.
    pub const fn mixer_identities(&self) -> MixerIdentities {
        self.mixer_identities
    }

    /// Acquire a body capability after validating the id under recording activity.
    pub fn body(&mut self, id: BodyId) -> Result<crate::Body<'_>> {
        Ok(crate::Body::new(crate::world::BodyProof::acquire(
            self, id,
        )?))
    }

    /// Acquire a shape capability after validating the id under recording activity.
    pub fn shape(&mut self, id: ShapeId) -> Result<crate::Shape<'_>> {
        Ok(crate::Shape::new(crate::world::ShapeProof::acquire(
            self, id,
        )?))
    }

    /// Acquire an untyped joint capability after validating the id under recording activity.
    pub fn joint(&mut self, id: JointId) -> Result<crate::Joint<'_>> {
        Ok(crate::Joint::new(crate::world::JointProof::acquire(
            self, id,
        )?))
    }

    /// Acquire a chain capability after validating the id under recording activity.
    pub fn chain(&mut self, id: ChainId) -> Result<crate::Chain<'_>> {
        Ok(crate::Chain::new(crate::world::ChainProof::acquire(
            self, id,
        )?))
    }

    /// Step the recorded world and return its lazily materialized event capability.
    ///
    /// A recording-writer failure discovered after Box2D advances is retained by
    /// [`crate::CompletedStep::post_step_error`]. An outer error means native simulation was not
    /// called.
    pub fn step(&mut self, time_step: f32, sub_steps: i32) -> Result<crate::CompletedStep<'_>> {
        self.check_access()?;
        let native = self
            .native
            .as_ref()
            .expect("an active recording session owns its native buffer");
        let (pending, worker_error) =
            World::step_while_recording_after_preflight(self.world, time_step, sub_steps)?;
        let post_step_error = worker_error.or_else(|| native.check_status().err());
        let contact_epoch = pending.commit();
        core::result::Result::Ok(crate::CompletedStep::after_validated_step(
            self,
            contact_epoch,
            post_step_error,
        ))
    }

    pub fn gravity(&self) -> Result<Vec2> {
        crate::world::run_owner_call(self, |world| {
            check_native_recording_gravity(Vec2::from_raw(unsafe {
                ffi::b2World_GetGravity(world.raw_world())
            }))
        })
    }

    pub fn set_gravity<V: Into<Vec2>>(&mut self, gravity: V) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_set_gravity(world, gravity)
        })
    }

    pub fn counters(&self) -> Result<Counters> {
        crate::world::run_owner_call(self, |world| {
            Counters::from_native("RecordingSession::counters", unsafe {
                ffi::b2World_GetCounters(world.raw_world())
            })
        })
    }

    pub fn create_body(&mut self, def: BodyDef) -> Result<BodyId> {
        crate::world::create_body_id(self, def)
    }

    /// Construct body defaults carrying the recorded world's length-scale provenance.
    #[must_use]
    pub fn body_def(&self) -> BodyDef {
        BodyDef::with_length_scale(self.world.core().length_scale())
    }

    /// Start a body builder carrying the recorded world's length-scale provenance.
    #[must_use]
    pub fn body_builder(&self) -> BodyBuilder {
        self.body_def().into()
    }

    /// Construct a joint base after proving that both body ids belong to this active recording.
    pub fn joint_base(&self, body_a: BodyId, body_b: BodyId) -> Result<crate::JointBase> {
        crate::world::joint_base_for_owner(self, body_a, body_b)
    }

    /// Stop recording and copy the private native stream into an opaque recording.
    ///
    /// Returns [`Error::InCallback`] when called from a Box2D callback. The consumed session is
    /// then cleaned up after the outermost native owner boundary returns to Rust.
    pub fn finish(mut self) -> Result<Recording> {
        self.stop_native()?;
        let mixer_identities = self.mixer_identities;
        Recording::from_native(self.native().bytes()?, mixer_identities)
    }
}

impl crate::world::OwnerAdapter for RecordingSession<'_> {
    fn capability_core(&self) -> &WorldCore {
        self.world.core()
    }

    fn capability_completed_step(&self) -> &crate::events::CompletedStepState {
        self.world.completed_step_state()
    }

    fn capability_preflight(&self) -> Result<()> {
        self.check_access()
    }

    fn capability_postflight(&self) -> Result<()> {
        self.native().check_status()
    }
}

impl Drop for RecordingSession<'_> {
    fn drop(&mut self) {
        // Native detach must precede activity release and native allocation
        // destruction. `stop_pending` makes this idempotent after `finish`.
        if crate::core::callback_state::in_callback() {
            self.defer_native_cleanup();
        } else {
            let _ = self.stop_native();
        }
    }
}

fn ensure_supported_callbacks(core: &WorldCore) -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if core
            .custom_filter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
        {
            return Err(Error::RecordingCustomFilterUnsupported);
        }
        if core
            .pre_solve
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
        {
            return Err(Error::RecordingPreSolveUnsupported);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = core;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Aabb, ChainDef, DistanceJointDef, DynamicTree, Position, QueryFilter, ShapeDef, shapes,
    };

    #[test]
    fn recording_gravity_rejects_invalid_native_vectors() {
        assert_eq!(
            check_native_recording_gravity(Vec2::new(f32::NAN, 0.0)),
            Err(Error::InvalidNativeOutput {
                operation: "RecordingSession::gravity",
                output: "gravity",
                constraint: "a finite vector",
            })
        );
        assert_eq!(
            check_native_recording_gravity(Vec2::new(-9.8, 0.0)),
            Ok(Vec2::new(-9.8, 0.0))
        );
    }

    struct WorldShutdownProbe {
        recording: Arc<RecordingLifecycleProbe>,
        recording_finished_first: Arc<AtomicBool>,
        drops: Arc<AtomicUsize>,
    }

    impl Drop for WorldShutdownProbe {
        fn drop(&mut self) {
            let recording_finished = self.recording.stops.load(Ordering::SeqCst) == 1
                && self.recording.destroys.load(Ordering::SeqCst) == 1;
            self.recording_finished_first
                .store(recording_finished, Ordering::SeqCst);
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn query_world_with_shape() -> World {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        world
            .body(body)
            .unwrap()
            .create_circle(
                &ShapeDef::default(),
                &shapes::circle(Vec2::ZERO, 0.5).unwrap(),
            )
            .unwrap();
        world
    }

    fn minimum_empty_world_recording_limit() -> u32 {
        const SEARCH_CEILING_BYTES: u32 = 1024 * 1024;

        let mut lower = 1;
        let mut upper = SEARCH_CEILING_BYTES;
        {
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let session = world
                .start_recording(RecordingLimits::new(u64::from(upper)).unwrap())
                .expect("an empty-world recording must fit within the search ceiling");
            drop(session);
        }
        while lower < upper {
            let midpoint = lower + (upper - lower) / 2;
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            match world.start_recording(RecordingLimits::new(u64::from(midpoint)).unwrap()) {
                Ok(session) => {
                    drop(session);
                    upper = midpoint;
                }
                Err(Error::RecordingLimitExceeded) => lower = midpoint + 1,
                Err(error) => {
                    panic!("unexpected recording-start error at {midpoint} bytes: {error}")
                }
            }
        }
        lower
    }

    fn assert_probe_counts(probe: &RecordingLifecycleProbe, stops: usize, destroys: usize) {
        assert_eq!(probe.stops.load(Ordering::SeqCst), stops);
        assert_eq!(probe.destroys.load(Ordering::SeqCst), destroys);
    }

    #[test]
    fn finish_from_post_native_query_visitor_cleans_up_immediately() {
        let query_world = query_world_with_shape();
        let mut recorded_world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let probe = Arc::new(RecordingLifecycleProbe::default());
        let mut session = recorded_world
            .start_recording(RecordingLimits::default())
            .unwrap();
        session.native.as_mut().unwrap().lifecycle_probe = Some(Arc::clone(&probe));
        let mut session = Some(session);
        let mut finish_error = None;
        let bounds = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0)).unwrap();
        let filter = QueryFilter::default();

        let completed = query_world
            .query()
            .unwrap()
            .visit_overlap_aabb(Position::ZERO, bounds, filter, |_| {
                finish_error = session.take().unwrap().finish().err();
                assert!(finish_error.is_none());
                assert_probe_counts(&probe, 1, 1);
                false
            })
            .unwrap();

        assert!(!completed);
        assert!(finish_error.is_none());
        assert_probe_counts(&probe, 1, 1);
    }

    #[test]
    fn drop_from_post_native_query_visitor_cleans_up_immediately() {
        let query_world = query_world_with_shape();
        let mut recorded_world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let probe = Arc::new(RecordingLifecycleProbe::default());
        let mut session = recorded_world
            .start_recording(RecordingLimits::default())
            .unwrap();
        session.native.as_mut().unwrap().lifecycle_probe = Some(Arc::clone(&probe));
        let mut session = Some(session);
        let bounds = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0)).unwrap();
        let filter = QueryFilter::default();

        let completed = query_world
            .query()
            .unwrap()
            .visit_overlap_aabb(Position::ZERO, bounds, filter, |_| {
                drop(session.take());
                assert_probe_counts(&probe, 1, 1);
                false
            })
            .unwrap();

        assert!(!completed);
        assert_probe_counts(&probe, 1, 1);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn dynamic_tree_callback_cleans_recording_before_world_teardown_and_resumes_primary_panic() {
        let query_world = query_world_with_shape();
        let mut recorded_world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let recording_probe = Arc::new(RecordingLifecycleProbe::default());
        let mut session = recorded_world
            .start_recording(RecordingLimits::default())
            .unwrap();
        session.native.as_mut().unwrap().lifecycle_probe = Some(Arc::clone(&recording_probe));
        let mut session = Some(session);

        let recording_finished_first = Arc::new(AtomicBool::new(false));
        let world_payload_drops = Arc::new(AtomicUsize::new(0));
        let mut doomed_world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        doomed_world
            .set_user_data(WorldShutdownProbe {
                recording: Arc::clone(&recording_probe),
                recording_finished_first: Arc::clone(&recording_finished_first),
                drops: Arc::clone(&world_payload_drops),
            })
            .unwrap();
        let doomed_raw = doomed_world.raw();
        let mut doomed_world = Some(doomed_world);

        let bounds = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0)).unwrap();
        let mut tree = DynamicTree::new().unwrap();
        tree.create_proxy(bounds, u64::MAX, 7).unwrap();

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            query_world
                .query()
                .unwrap()
                .visit_overlap_aabb(Position::ZERO, bounds, QueryFilter::default(), |_| {
                    tree.query_all(bounds, &mut |_, _| {
                        drop(session.take());
                        drop(doomed_world.take());
                        assert_probe_counts(&recording_probe, 0, 0);
                        assert!(unsafe { ffi::b2World_IsValid(doomed_raw) });
                        panic!("dynamic tree recording cleanup panic");
                    })
                    .unwrap();
                    false
                })
                .unwrap();
        }));

        drop(session);
        let payload = panic.expect_err("the callback panic must resume after owner cleanup");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"dynamic tree recording cleanup panic")
        );
        assert_probe_counts(&recording_probe, 1, 1);
        assert!(recording_finished_first.load(Ordering::SeqCst));
        assert_eq!(world_payload_drops.load(Ordering::SeqCst), 1);
        assert!(!unsafe { ffi::b2World_IsValid(doomed_raw) });
        assert_eq!(recorded_world.core().activity(), ActivityState::Idle);
        assert!(recorded_world.counters().is_ok());
    }

    #[test]
    fn attached_recording_header_failure_is_raii_recoverable() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        FAIL_NEXT_SIZE_CHECK.with(|fail| fail.set(true));

        assert!(matches!(
            world.start_recording(RecordingLimits::default()),
            Err(Error::InvalidNativeRecording)
        ));
        assert_eq!(world.core().activity(), ActivityState::Idle);
        assert!(world.counters().is_ok());

        world
            .start_recording(RecordingLimits::default())
            .unwrap()
            .finish()
            .unwrap();
    }

    #[test]
    fn recording_activity_lease_finishes_idempotently() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let core = world.core();
        let mut activity = core.begin_recording_activity().unwrap();

        assert_eq!(core.activity(), ActivityState::Recording);
        assert_eq!(activity.finish(), Ok(()));
        assert_eq!(activity.finish(), Ok(()));
        assert_eq!(core.activity(), ActivityState::Idle);
    }

    #[test]
    fn recording_limits_check_native_writer_boundaries() {
        let default = RecordingLimits::default();
        assert_eq!(default.max_bytes(), RecordingLimits::MAX_BYTES);

        let exact = RecordingLimits::new(u64::from(RecordingLimits::MAX_BYTES)).unwrap();
        assert_eq!(exact.max_bytes(), RecordingLimits::MAX_BYTES);
        assert!(matches!(
            RecordingLimits::new(0),
            Err(Error::InvalidArgument { .. })
        ));
        assert!(matches!(
            RecordingLimits::new(u64::from(RecordingLimits::MAX_BYTES) + 1),
            Err(Error::InvalidArgument { .. })
        ));
    }

    #[test]
    fn recording_session_issues_the_common_object_capabilities() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        let body_a = session
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let body_b = session
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let shape = {
            let mut body: crate::Body<'_> = session.body(body_a).unwrap();
            body.create_circle(
                &ShapeDef::default(),
                &shapes::circle(crate::Vec2::ZERO, 0.5).unwrap(),
            )
            .unwrap()
        };
        let chain = {
            let def = ChainDef::builder()
                .points([
                    [-2.0_f32, 0.0],
                    [-1.0_f32, 0.0],
                    [1.0_f32, 0.0],
                    [2.0_f32, 0.0],
                ])
                .build()
                .unwrap();
            let mut body: crate::Body<'_> = session.body(body_a).unwrap();
            body.create_chain(&def).unwrap()
        };
        let joint = session
            .create_distance_joint(&DistanceJointDef::new(
                session.joint_base(body_a, body_b).unwrap(),
            ))
            .unwrap();

        let body: crate::Body<'_> = session.body(body_a).unwrap();
        assert_eq!(body.id(), body_a);
        let shape_handle: crate::Shape<'_> = session.shape(shape).unwrap();
        assert_eq!(shape_handle.id(), shape);

        let chain_handle: crate::Chain<'_> = session.chain(chain).unwrap();
        assert_eq!(chain_handle.id(), chain);

        let joint_handle: crate::Joint<'_> = session.joint(joint).unwrap();
        assert_eq!(joint_handle.id(), joint);
        let typed: crate::DistanceJoint<'_> = joint_handle.into_distance().unwrap();
        assert_eq!(typed.id(), joint);
    }

    #[test]
    fn creation_postflight_failures_compensate_before_identity_publication() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        let body_a = session
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let body_b = session
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();

        let before = session.counters().unwrap();
        let compensations = session.world.core().creation_compensation_count_for_test();
        FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| fail.set(Some(1)));
        assert_eq!(
            session.create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def()
            ),
            Err(Error::RecordingLimitExceeded)
        );
        assert_eq!(session.counters().unwrap().body_count, before.body_count);

        let mut body = session.body(body_a).unwrap();
        FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| fail.set(Some(1)));
        assert_eq!(
            body.create_circle(
                &ShapeDef::default(),
                &shapes::circle(crate::Vec2::ZERO, 0.5).unwrap(),
            ),
            Err(Error::RecordingLimitExceeded)
        );
        assert_eq!(session.counters().unwrap().shape_count, before.shape_count);

        let chain_def = ChainDef::builder()
            .points([
                [-2.0_f32, 0.0],
                [-1.0_f32, 0.0],
                [1.0_f32, 0.0],
                [2.0_f32, 0.0],
            ])
            .build()
            .unwrap();
        let mut body = session.body(body_a).unwrap();
        FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| fail.set(Some(1)));
        assert_eq!(
            body.create_chain(&chain_def),
            Err(Error::RecordingLimitExceeded)
        );
        let after_chain = session.counters().unwrap();
        assert_eq!(after_chain.shape_count, before.shape_count);

        let joint_def = DistanceJointDef::new(session.joint_base(body_a, body_b).unwrap());
        FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| fail.set(Some(1)));
        assert_eq!(
            session.create_distance_joint(&joint_def),
            Err(Error::RecordingLimitExceeded)
        );
        assert_eq!(session.counters().unwrap().joint_count, before.joint_count);
        assert_eq!(
            session.world.core().creation_compensation_count_for_test(),
            compensations + 4
        );
        assert_eq!(
            session.world.core().lifecycle(),
            crate::core::world_core::LifecycleState::Live
        );
    }

    #[test]
    fn shape_kind_tracks_a_native_setter_when_recording_postflight_fails() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_id = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let shape_id = world
            .body(body_id)
            .unwrap()
            .create_circle(
                &ShapeDef::default(),
                &shapes::circle(crate::Vec2::ZERO, 0.5).unwrap(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        let mut shape = session.shape(shape_id).unwrap();
        let polygon = shapes::square_polygon(0.75).unwrap();

        FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| fail.set(Some(1)));
        assert_eq!(
            shape.set_polygon(&polygon),
            Err(Error::RecordingLimitExceeded)
        );

        assert_eq!(shape.shape_type(), Ok(crate::ShapeType::Polygon));
        let actual_polygon = shape.polygon().unwrap();
        assert_eq!(actual_polygon.vertices(), polygon.vertices());
        assert_eq!(actual_polygon.normals(), polygon.normals());
        assert_eq!(actual_polygon.centroid(), polygon.centroid());
        assert_eq!(actual_polygon.radius(), polygon.radius());
    }

    #[test]
    fn failed_creation_compensation_poisons_the_world() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        session
            .world
            .core()
            .fail_next_creation_compensation_for_test();
        FAIL_STATUS_AFTER_SUCCESSFUL_CHECKS.with(|fail| fail.set(Some(1)));

        assert_eq!(
            session.create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def()
            ),
            Err(Error::RecordingLimitExceeded)
        );
        assert_eq!(
            session.world.core().lifecycle(),
            crate::core::world_core::LifecycleState::Poisoned
        );
    }

    #[test]
    fn recording_step_returns_the_common_lazy_event_capability() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();

        let completed: crate::CompletedStep<'_> = session.step(0.0, 1).unwrap();
        assert_eq!(completed.post_step_error(), None);
        drop(completed);

        assert_eq!(
            session.world.event_storage().getter_calls_for_test(),
            [0; 4]
        );
    }

    #[test]
    fn event_access_checks_the_native_recording_size_once() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        let completed = session.step(0.0, 1).unwrap();

        let before_materialization = RECORDING_GET_SIZE_CALLS.with(Cell::get);
        assert!(completed.body_events().unwrap().is_empty());
        assert_eq!(
            RECORDING_GET_SIZE_CALLS.with(Cell::get),
            before_materialization + 1
        );

        let before_cached_access = RECORDING_GET_SIZE_CALLS.with(Cell::get);
        assert!(completed.body_events().unwrap().is_empty());
        assert_eq!(
            RECORDING_GET_SIZE_CALLS.with(Cell::get),
            before_cached_access + 1
        );
    }

    #[test]
    fn recording_step_postflight_error_preserves_native_advancement_and_event_lifetime() {
        let limit = minimum_empty_world_recording_limit();
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut session = world
            .start_recording(RecordingLimits::new(u64::from(limit)).unwrap())
            .unwrap();

        let completed = session.step(0.0, 1).unwrap();
        assert!(matches!(
            completed.post_step_error(),
            Some(Error::RecordingLimitExceeded | Error::RecordingOperationTooLarge)
        ));
        drop(completed);
        assert!(!session.world.completed_step_active_for_test());

        drop(session);
        assert!(world.counters().is_ok());
    }

    #[test]
    fn finish_and_drop_retire_a_forgotten_recording_step() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();

        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        core::mem::forget(session.step(0.0, 1).unwrap());
        assert!(session.world.completed_step_active_for_test());
        session.finish().unwrap();
        assert!(!world.completed_step_active_for_test());
        assert!(world.counters().is_ok());

        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        core::mem::forget(session.step(0.0, 1).unwrap());
        assert!(session.world.completed_step_active_for_test());
        drop(session);
        assert!(!world.completed_step_active_for_test());
        assert!(world.counters().is_ok());
    }
}
