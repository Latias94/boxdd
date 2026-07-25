//! Owned Box2D recording sessions and recording byte buffers.

use crate::core::world_core::{ActivityState, WorldCore};
#[cfg(not(target_arch = "wasm32"))]
use crate::{Aabb, MoverPlaneResult};
use crate::{
    ApiError, ApiResult, BodyDef, BodyId, BodyType, Counters, Filter, Position, QueryFilter,
    RayResult, ShapeDef, ShapeId, Vec2, World, WorldCastOutput,
};
use boxdd_sys::ffi;
use core::ptr::NonNull;
use std::rc::Rc;

mod session_joints;
mod session_shapes_chains;
mod session_world_body;

#[cfg(test)]
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_SIZE_CHECK: Cell<bool> = const { Cell::new(false) };
}

/// Validated initial capacity for a native recording buffer.
///
/// Zero selects Box2D's default capacity. Positive values only preallocate;
/// the native append-only buffer can still grow while recording.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RecordingCapacity(i32);

impl RecordingCapacity {
    /// Use Box2D's default initial capacity.
    pub const DEFAULT: Self = Self(0);
    /// Largest initial capacity representable by the pinned Box2D ABI.
    pub const MAX_BYTES: u32 = i32::MAX as u32;

    /// Construct a capacity after checking the native signed-int boundary.
    pub fn new(byte_capacity: u64) -> ApiResult<Self> {
        let byte_capacity = i32::try_from(byte_capacity).map_err(|_| ApiError::InvalidArgument)?;
        Ok(Self(byte_capacity))
    }

    /// Return the requested initial capacity in bytes.
    pub const fn bytes(self) -> u32 {
        self.0 as u32
    }

    #[inline]
    const fn as_i32(self) -> i32 {
        self.0
    }
}

impl Default for RecordingCapacity {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u64> for RecordingCapacity {
    type Error = ApiError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<usize> for RecordingCapacity {
    type Error = ApiError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(u64::try_from(value).map_err(|_| ApiError::InvalidArgument)?)
    }
}

/// Material-mixer callbacks that replay must reinstall before its first step.
///
/// Box2D records neither friction nor restitution mixer results. This metadata
/// intentionally records requirements, not callback pointers or host state.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct MixerRequirements {
    friction: bool,
    restitution: bool,
}

impl MixerRequirements {
    /// Describe the mixer sidecar that accompanies externally persisted recording bytes.
    ///
    /// The native Box2D recording stream does not encode this wrapper metadata. Persist these
    /// flags with the bytes and provide the same values to [`crate::ReplayPlayer::open_bytes`].
    pub const fn new(friction: bool, restitution: bool) -> Self {
        Self {
            friction,
            restitution,
        }
    }

    #[inline]
    fn capture(core: &WorldCore) -> Self {
        let (friction, restitution) = core.mixer_presence();
        Self {
            friction,
            restitution,
        }
    }

    /// Whether replay must install a friction mixer.
    pub const fn requires_friction(self) -> bool {
        self.friction
    }

    /// Whether replay must install a restitution mixer.
    pub const fn requires_restitution(self) -> bool {
        self.restitution
    }

    /// Whether replay can use both default Box2D mixing rules.
    pub const fn is_empty(self) -> bool {
        !self.friction && !self.restitution
    }
}

/// An owned recording byte stream and its replay requirements.
///
/// The bytes are copied out of Box2D before its native recording allocation is
/// destroyed, so they remain valid independently of the source world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recording {
    bytes: Box<[u8]>,
    mixer_requirements: MixerRequirements,
}

impl Recording {
    /// Borrow the complete native recording stream.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Return the mixer set that a future replay configuration must provide.
    pub const fn mixer_requirements(&self) -> MixerRequirements {
        self.mixer_requirements
    }

    /// Consume the recording and return its owned bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes.into_vec()
    }

    /// Consume the recording into its independently owned replay inputs.
    pub fn into_parts(self) -> (Vec<u8>, MixerRequirements) {
        (self.bytes.into_vec(), self.mixer_requirements)
    }
}

impl AsRef<[u8]> for Recording {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

struct NativeRecording {
    raw: NonNull<ffi::b2Recording>,
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
    fn new(capacity: RecordingCapacity) -> ApiResult<Self> {
        let raw = unsafe { ffi::b2CreateRecording(capacity.as_i32()) };
        NonNull::new(raw)
            .map(|raw| Self {
                raw,
                #[cfg(test)]
                lifecycle_probe: None,
            })
            .ok_or(ApiError::RecordingAllocationFailed)
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

    fn checked_size(&self) -> ApiResult<usize> {
        #[cfg(test)]
        if FAIL_NEXT_SIZE_CHECK.with(|fail| fail.replace(false)) {
            return Err(ApiError::InvalidNativeRecording);
        }

        let raw_size = unsafe { ffi::b2Recording_GetSize(self.as_ptr()) };
        let size = usize::try_from(raw_size)
            .map_err(|_| ApiError::NegativeFfiOutputCount { count: raw_size })?;
        if size == 0 {
            return Err(ApiError::InvalidNativeRecording);
        }
        Ok(size)
    }

    fn copy_bytes(&self) -> ApiResult<Box<[u8]>> {
        let size = self.checked_size()?;

        let data = unsafe { ffi::b2Recording_GetData(self.as_ptr()) };
        if data.is_null() {
            return Err(ApiError::InvalidNativeRecording);
        }

        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| ApiError::FfiOutputAllocationFailed)?;
        // SAFETY: Box2D reports `size` initialized bytes at a non-null pointer.
        // Recording has stopped, and `self` keeps the native allocation alive
        // for the duration of this copy.
        let source = unsafe { core::slice::from_raw_parts(data, size) };
        bytes.extend_from_slice(source);
        Ok(bytes.into_boxed_slice())
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

struct RecordingActivity {
    core: Rc<WorldCore>,
    active: bool,
}

impl RecordingActivity {
    fn begin(core: Rc<WorldCore>) -> ApiResult<Self> {
        core.set_activity(ActivityState::Idle, ActivityState::Recording)?;
        Ok(Self { core, active: true })
    }

    fn finish(&mut self) -> ApiResult<()> {
        if !self.active {
            return Ok(());
        }
        self.core.finish_recording_activity()?;
        self.active = false;
        Ok(())
    }
}

impl Drop for RecordingActivity {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// An active recording that exclusively borrows its wrapper-owned world.
///
/// Existing handles remain valid Rust values, but their operations observe the
/// central world activity gate and return [`ApiError::WorldBusy`]. Use the
/// methods on this session for the recording-safe, session-owned surface.
#[must_use = "dropping the session stops recording and discards its bytes"]
pub struct RecordingSession<'world> {
    world: &'world mut World,
    native: Option<NativeRecording>,
    activity: Option<RecordingActivity>,
    mixer_requirements: MixerRequirements,
    stop_pending: bool,
}

impl World {
    /// Start recording at a step boundary.
    ///
    /// Panics on invalid world activity or unsupported callback wiring. Use
    /// [`World::try_start_recording`] for a recoverable error.
    pub fn start_recording(&mut self, capacity: RecordingCapacity) -> RecordingSession<'_> {
        self.try_start_recording(capacity)
            .expect("world cannot start a Box2D recording")
    }

    /// Start an owned recording session at a step boundary.
    ///
    /// Box2D cannot record custom-filter or pre-solve callback decisions, so
    /// either installed callback is rejected before allocating or mutating
    /// native recording state.
    pub fn try_start_recording(
        &mut self,
        capacity: RecordingCapacity,
    ) -> ApiResult<RecordingSession<'_>> {
        crate::world::check_world_available(self.core())?;
        ensure_supported_callbacks(self.core())?;

        let mixer_requirements = MixerRequirements::capture(self.core());
        let native = NativeRecording::new(capacity)?;
        let activity = RecordingActivity::begin(self.core_rc())?;
        let session = RecordingSession {
            world: self,
            native: Some(native),
            activity: Some(activity),
            mixer_requirements,
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

    fn activity_mut(&mut self) -> &mut RecordingActivity {
        self.activity
            .as_mut()
            .expect("an active recording session owns its activity guard")
    }

    fn check_access(&self) -> ApiResult<()> {
        crate::world::check_recording_world_available(self.world.core())
    }

    fn query_target(&self) -> crate::query::QueryTarget {
        crate::query::QueryTarget::recording(self.world.core_rc())
    }

    fn stop_native(&mut self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        if !self.stop_pending {
            return Ok(());
        }
        self.stop_pending = false;
        unsafe { ffi::b2World_StopRecording(self.world.raw()) };
        #[cfg(test)]
        self.native().record_stop();
        self.activity_mut().finish()?;
        // Owned handles may be dropped while ordinary aliases are gated. Their
        // native destruction must not leak past the session boundary and affect
        // a later step after recording has already stopped.
        self.world.core().process_deferred_destroys();
        Ok(())
    }

    fn defer_native_cleanup(&mut self) {
        let stop_pending = core::mem::replace(&mut self.stop_pending, false);
        let raw_world = self.world.raw();
        let native = self
            .native
            .take()
            .expect("a recording session owns its native buffer until cleanup");
        let mut activity = self
            .activity
            .take()
            .expect("a recording session owns its activity guard until cleanup");
        crate::core::callback_state::defer_callback_cleanup_or_forget(move || {
            if stop_pending {
                unsafe { ffi::b2World_StopRecording(raw_world) };
                #[cfg(test)]
                native.record_stop();
            }
            let _ = activity.finish();
            activity.core.process_deferred_destroys();
            drop(native);
        });
    }

    /// Return the mixer requirements captured atomically at session start.
    pub const fn mixer_requirements(&self) -> MixerRequirements {
        self.mixer_requirements
    }

    /// Step the recorded world.
    pub fn step(&mut self, time_step: f32, sub_steps: i32) {
        self.try_step(time_step, sub_steps)
            .expect("recording session could not step its world");
    }

    /// Try to step the recorded world.
    pub fn try_step(&mut self, time_step: f32, sub_steps: i32) -> ApiResult<()> {
        World::try_step_while_recording(self.world, time_step, sub_steps)
    }

    /// Return the recorded world's current gravity.
    pub fn gravity(&self) -> Vec2 {
        self.try_gravity()
            .expect("recording session could not read world gravity")
    }

    pub fn try_gravity(&self) -> ApiResult<Vec2> {
        self.check_access()?;
        Ok(Vec2::from_raw(unsafe {
            ffi::b2World_GetGravity(self.world.raw())
        }))
    }

    /// Set gravity and append the mutation to the recording.
    pub fn set_gravity<V: Into<Vec2>>(&mut self, gravity: V) {
        self.try_set_gravity(gravity)
            .expect("recording session received invalid gravity")
    }

    pub fn try_set_gravity<V: Into<Vec2>>(&mut self, gravity: V) -> ApiResult<()> {
        crate::world::try_world_set_gravity_with_access(
            self.world,
            gravity,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Return a counters snapshot while keeping ordinary aliases gated.
    pub fn counters(&self) -> Counters {
        self.try_counters()
            .expect("recording session could not read world counters")
    }

    pub fn try_counters(&self) -> ApiResult<Counters> {
        self.check_access()?;
        Ok(Counters::from_raw(unsafe {
            ffi::b2World_GetCounters(self.world.raw())
        }))
    }

    /// Create a body and return its world-branded value identifier.
    pub fn create_body(&mut self, def: BodyDef) -> BodyId {
        self.try_create_body(def)
            .expect("recording session could not create a body")
    }

    pub fn try_create_body(&mut self, def: BodyDef) -> ApiResult<BodyId> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::body::check_body_def_valid(&def)?;
        crate::world::try_create_body_id_with_access(
            self.world,
            def,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Destroy a body after validating its world identity and live generation.
    pub fn destroy_body(&mut self, body: BodyId) {
        self.try_destroy_body(body)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_destroy_body(&mut self, body: BodyId) -> ApiResult<()> {
        WorldCore::destroy_body_now_with_access(
            self.world.core(),
            body,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set a body's absolute position and rotation and record the mutation.
    pub fn set_body_position_and_rotation(
        &mut self,
        body: BodyId,
        position: Position,
        angle_radians: f32,
    ) {
        self.try_set_body_position_and_rotation(body, position, angle_radians)
            .expect("recording session received an invalid body transform")
    }

    pub fn try_set_body_position_and_rotation(
        &mut self,
        body: BodyId,
        position: Position,
        angle_radians: f32,
    ) -> ApiResult<()> {
        crate::world::try_set_body_position_and_rotation_with_access(
            self.world.core(),
            body,
            position,
            angle_radians,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set a body's linear velocity and record the mutation.
    pub fn set_body_linear_velocity<V: Into<Vec2>>(&mut self, body: BodyId, velocity: V) {
        self.try_set_body_linear_velocity(body, velocity)
            .expect("recording session received an invalid linear velocity")
    }

    pub fn try_set_body_linear_velocity<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        velocity: V,
    ) -> ApiResult<()> {
        crate::world::try_set_body_linear_velocity_with_access(
            self.world.core(),
            body,
            velocity,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set a body's angular velocity and record the mutation.
    pub fn set_body_angular_velocity(&mut self, body: BodyId, angular_velocity: f32) {
        self.try_set_body_angular_velocity(body, angular_velocity)
            .expect("recording session received an invalid angular velocity")
    }

    pub fn try_set_body_angular_velocity(
        &mut self,
        body: BodyId,
        angular_velocity: f32,
    ) -> ApiResult<()> {
        crate::world::try_set_body_angular_velocity_with_access(
            self.world.core(),
            body,
            angular_velocity,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Apply a linear impulse at the center of mass and record the mutation.
    pub fn body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) {
        self.try_body_apply_linear_impulse_to_center(body, impulse, wake)
            .expect("recording session received an invalid linear impulse")
    }

    pub fn try_body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_body_apply_linear_impulse_to_center_with_access(
            self.world.core(),
            body,
            impulse,
            wake,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Apply an angular impulse and record the mutation.
    pub fn body_apply_angular_impulse(&mut self, body: BodyId, impulse: f32, wake: bool) {
        self.try_body_apply_angular_impulse(body, impulse, wake)
            .expect("recording session received an invalid angular impulse")
    }

    pub fn try_body_apply_angular_impulse(
        &mut self,
        body: BodyId,
        impulse: f32,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_body_apply_angular_impulse_with_access(
            self.world.core(),
            body,
            impulse,
            wake,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Clear accumulated forces and record the mutation.
    pub fn body_clear_forces(&mut self, body: BodyId) {
        self.try_body_clear_forces(body)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_clear_forces(&mut self, body: BodyId) -> ApiResult<()> {
        crate::world::try_body_clear_forces_with_access(
            self.world.core(),
            body,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Change a body's motion type and record the mutation.
    pub fn set_body_type(&mut self, body: BodyId, body_type: BodyType) {
        self.try_set_body_type(body, body_type)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_set_body_type(&mut self, body: BodyId, body_type: BodyType) -> ApiResult<()> {
        crate::world::try_set_body_type_with_access(
            self.world.core(),
            body,
            body_type,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Create a circle shape attached to a recorded body.
    pub fn create_circle_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        circle: &crate::shapes::Circle,
    ) -> ShapeId {
        self.try_create_circle_shape(body, def, circle)
            .expect("recording session could not create a circle shape")
    }

    pub fn try_create_circle_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        circle: &crate::shapes::Circle,
    ) -> ApiResult<ShapeId> {
        crate::shapes::try_create_circle_shape_for_body_with_access(
            self.world.core(),
            body,
            def,
            circle,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Create a segment shape attached to a recorded body.
    pub fn create_segment_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        segment: &crate::shapes::Segment,
    ) -> ShapeId {
        self.try_create_segment_shape(body, def, segment)
            .expect("recording session could not create a segment shape")
    }

    pub fn try_create_segment_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        segment: &crate::shapes::Segment,
    ) -> ApiResult<ShapeId> {
        crate::shapes::try_create_segment_shape_for_body_with_access(
            self.world.core(),
            body,
            def,
            segment,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Create an orphan chain-segment shape attached directly to a body.
    pub fn create_chain_segment_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> ShapeId {
        self.try_create_chain_segment_shape(body, def, chain_segment)
            .expect("recording session could not create a chain-segment shape")
    }

    pub fn try_create_chain_segment_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> ApiResult<ShapeId> {
        crate::shapes::try_create_chain_segment_shape_for_body_with_access(
            self.world.core(),
            body,
            def,
            chain_segment,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Create a capsule shape attached to a recorded body.
    pub fn create_capsule_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        capsule: &crate::shapes::Capsule,
    ) -> ShapeId {
        self.try_create_capsule_shape(body, def, capsule)
            .expect("recording session could not create a capsule shape")
    }

    pub fn try_create_capsule_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        capsule: &crate::shapes::Capsule,
    ) -> ApiResult<ShapeId> {
        crate::shapes::try_create_capsule_shape_for_body_with_access(
            self.world.core(),
            body,
            def,
            capsule,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Create a polygon shape attached to a recorded body.
    pub fn create_polygon_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        polygon: &crate::shapes::Polygon,
    ) -> ShapeId {
        self.try_create_polygon_shape(body, def, polygon)
            .expect("recording session could not create a polygon shape")
    }

    pub fn try_create_polygon_shape(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        polygon: &crate::shapes::Polygon,
    ) -> ApiResult<ShapeId> {
        crate::shapes::try_create_polygon_shape_for_body_with_access(
            self.world.core(),
            body,
            def,
            polygon,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Destroy an orphan shape after validating its identity and parent status.
    pub fn destroy_shape(&mut self, shape: ShapeId, update_body_mass: bool) {
        self.try_destroy_shape(shape, update_body_mass)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_destroy_shape(&mut self, shape: ShapeId, update_body_mass: bool) -> ApiResult<()> {
        WorldCore::destroy_shape_now_with_access(
            self.world.core(),
            shape,
            update_body_mass,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Replace an orphan shape's geometry with a circle and record the mutation.
    pub fn shape_set_circle(&mut self, shape: ShapeId, circle: &crate::shapes::Circle) {
        self.try_shape_set_circle(shape, circle)
            .expect("recording session received invalid circle geometry")
    }

    pub fn try_shape_set_circle(
        &mut self,
        shape: ShapeId,
        circle: &crate::shapes::Circle,
    ) -> ApiResult<()> {
        crate::world::try_world_shape_set_circle_with_access(
            self.world.core(),
            shape,
            circle,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Replace an orphan shape's geometry with a segment and record the mutation.
    pub fn shape_set_segment(&mut self, shape: ShapeId, segment: &crate::shapes::Segment) {
        self.try_shape_set_segment(shape, segment)
            .expect("recording session received invalid segment geometry")
    }

    pub fn try_shape_set_segment(
        &mut self,
        shape: ShapeId,
        segment: &crate::shapes::Segment,
    ) -> ApiResult<()> {
        crate::world::try_world_shape_set_segment_with_access(
            self.world.core(),
            shape,
            segment,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Replace an orphan shape's geometry with a chain segment and record the mutation.
    pub fn shape_set_chain_segment(
        &mut self,
        shape: ShapeId,
        chain_segment: &crate::shapes::ChainSegment,
    ) {
        self.try_shape_set_chain_segment(shape, chain_segment)
            .expect("recording session received invalid chain-segment geometry")
    }

    pub fn try_shape_set_chain_segment(
        &mut self,
        shape: ShapeId,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> ApiResult<()> {
        crate::shapes::try_shape_set_chain_segment_checked_with_access(
            self.world.core(),
            shape,
            chain_segment,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Replace an orphan shape's geometry with a capsule and record the mutation.
    pub fn shape_set_capsule(&mut self, shape: ShapeId, capsule: &crate::shapes::Capsule) {
        self.try_shape_set_capsule(shape, capsule)
            .expect("recording session received invalid capsule geometry")
    }

    pub fn try_shape_set_capsule(
        &mut self,
        shape: ShapeId,
        capsule: &crate::shapes::Capsule,
    ) -> ApiResult<()> {
        crate::world::try_world_shape_set_capsule_with_access(
            self.world.core(),
            shape,
            capsule,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Replace an orphan shape's geometry with a polygon and record the mutation.
    pub fn shape_set_polygon(&mut self, shape: ShapeId, polygon: &crate::shapes::Polygon) {
        self.try_shape_set_polygon(shape, polygon)
            .expect("recording session received invalid polygon geometry")
    }

    pub fn try_shape_set_polygon(
        &mut self,
        shape: ShapeId,
        polygon: &crate::shapes::Polygon,
    ) -> ApiResult<()> {
        crate::world::try_world_shape_set_polygon_with_access(
            self.world.core(),
            shape,
            polygon,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Replace a shape's surface material and record the mutation.
    pub fn shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &crate::SurfaceMaterial,
    ) {
        self.try_shape_set_surface_material(shape, material)
            .expect("recording session received invalid surface material")
    }

    pub fn try_shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &crate::SurfaceMaterial,
    ) -> ApiResult<()> {
        crate::world::try_world_shape_set_surface_material_with_access(
            self.world.core(),
            shape,
            material,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set a shape's collision filter and record the mutation.
    pub fn shape_set_filter(&mut self, shape: ShapeId, filter: Filter) {
        self.try_shape_set_filter(shape, filter)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_shape_set_filter(&mut self, shape: ShapeId, filter: Filter) -> ApiResult<()> {
        crate::shapes::try_shape_set_filter_with_access(
            self.world.core(),
            shape,
            filter,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set shape density and record the mutation.
    pub fn shape_set_density(&mut self, shape: ShapeId, density: f32, update_body_mass: bool) {
        self.try_shape_set_density(shape, density, update_body_mass)
            .expect("recording session received invalid shape density")
    }

    pub fn try_shape_set_density(
        &mut self,
        shape: ShapeId,
        density: f32,
        update_body_mass: bool,
    ) -> ApiResult<()> {
        crate::shapes::try_shape_set_density_with_access(
            self.world.core(),
            shape,
            density,
            update_body_mass,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set shape friction and record the mutation.
    pub fn shape_set_friction(&mut self, shape: ShapeId, friction: f32) {
        self.try_shape_set_friction(shape, friction)
            .expect("recording session received invalid shape friction")
    }

    pub fn try_shape_set_friction(&mut self, shape: ShapeId, friction: f32) -> ApiResult<()> {
        crate::shapes::try_shape_set_friction_with_access(
            self.world.core(),
            shape,
            friction,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set shape restitution and record the mutation.
    pub fn shape_set_restitution(&mut self, shape: ShapeId, restitution: f32) {
        self.try_shape_set_restitution(shape, restitution)
            .expect("recording session received invalid shape restitution")
    }

    pub fn try_shape_set_restitution(&mut self, shape: ShapeId, restitution: f32) -> ApiResult<()> {
        crate::shapes::try_shape_set_restitution_with_access(
            self.world.core(),
            shape,
            restitution,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Set the application material identifier and record the mutation.
    pub fn shape_set_user_material(&mut self, shape: ShapeId, material: u64) {
        self.try_shape_set_user_material(shape, material)
            .expect("recording session received an invalid ShapeId")
    }

    pub fn try_shape_set_user_material(&mut self, shape: ShapeId, material: u64) -> ApiResult<()> {
        crate::shapes::try_shape_set_user_material_with_access(
            self.world.core(),
            shape,
            material,
            crate::core::world_core::WorldAccess::Recording,
        )
    }

    /// Collect shapes overlapping an AABB and append the query to the recording.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_aabb(&self, origin: Position, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        crate::query::overlap_aabb_checked_impl(self.query_target(), origin, aabb, filter)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_overlap_aabb(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>> {
        crate::query::try_overlap_aabb_impl(self.query_target(), origin, aabb, filter)
    }

    /// Collect shapes overlapping a proxy and append the query to the recording.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_polygon_points<I, P>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::query::overlap_polygon_points_checked_impl(
            self.query_target(),
            origin,
            points,
            radius,
            filter,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_overlap_polygon_points<I, P>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::query::try_overlap_polygon_points_impl(
            self.query_target(),
            origin,
            points,
            radius,
            filter,
        )
    }

    /// Cast a ray and collect every hit while recording the query and results.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        crate::query::cast_ray_all_checked_impl(self.query_target(), origin, translation, filter)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        crate::query::try_cast_ray_all_impl(self.query_target(), origin, translation, filter)
    }

    /// Cast a ray and return its closest hit while recording the query and result.
    pub fn cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Option<RayResult> {
        self.cast_ray_closest_with_stats(origin, translation, filter)
            .hit
    }

    pub fn try_cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Option<RayResult>> {
        self.try_cast_ray_closest_with_stats(origin, translation, filter)
            .map(|result| result.hit)
    }

    /// Cast a ray and record its complete closest-hit result, including traversal statistics.
    pub fn cast_ray_closest_with_stats<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::ClosestRayCastResult {
        crate::query::cast_ray_closest_with_stats_checked_impl(
            self.query_target(),
            origin,
            translation,
            filter,
        )
    }

    pub fn try_cast_ray_closest_with_stats<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<crate::ClosestRayCastResult> {
        crate::query::try_cast_ray_closest_with_stats_impl(
            self.query_target(),
            origin,
            translation,
            filter,
        )
    }

    /// Cast a convex proxy and record the query and all hit results.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_shape_points<I, P, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::query::cast_shape_points_checked_impl(
            self.query_target(),
            origin,
            points,
            radius,
            translation,
            filter,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_cast_shape_points<I, P, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::query::try_cast_shape_points_impl(
            self.query_target(),
            origin,
            points,
            radius,
            translation,
            filter,
        )
    }

    /// Collide a capsule mover and record its resulting planes.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        crate::query::collide_mover_checked_impl(
            self.query_target(),
            origin,
            c1,
            c2,
            radius,
            filter,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<MoverPlaneResult>> {
        crate::query::try_collide_mover_impl(self.query_target(), origin, c1, c2, radius, filter)
    }

    /// Cast a capsule mover and record its limiting fraction.
    #[allow(clippy::too_many_arguments)]
    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        crate::query::cast_mover_checked_impl(
            self.query_target(),
            origin,
            c1,
            c2,
            radius,
            translation,
            filter,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: Position,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<f32> {
        crate::query::try_cast_mover_impl(
            self.query_target(),
            origin,
            c1,
            c2,
            radius,
            translation,
            filter,
        )
    }

    /// Test a point against one shape and record the query result.
    pub fn shape_test_point(&self, shape: ShapeId, point: Position) -> bool {
        self.try_shape_test_point(shape, point)
            .expect("recording session received an invalid shape point query")
    }

    pub fn try_shape_test_point(&self, shape: ShapeId, point: Position) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        self.world
            .core()
            .check_shape_with_access(shape, crate::core::world_core::WorldAccess::Recording)?;
        crate::shapes::try_shape_test_point_checked_impl(shape, point)
    }

    /// Cast a ray against one shape and record the query result.
    pub fn shape_ray_cast<VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: Position,
        translation: VT,
    ) -> WorldCastOutput {
        self.try_shape_ray_cast(shape, origin, translation)
            .expect("recording session received an invalid shape ray query")
    }

    pub fn try_shape_ray_cast<VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: Position,
        translation: VT,
    ) -> ApiResult<WorldCastOutput> {
        crate::core::callback_state::check_not_in_callback()?;
        self.world
            .core()
            .check_shape_with_access(shape, crate::core::world_core::WorldAccess::Recording)?;
        let translation = translation.into();
        let origin = crate::body::check_valid_body_position(origin)?;
        crate::shapes::check_shape_vec2_valid(translation)?;
        self.world
            .core()
            .check_shape_with_access(shape, crate::core::world_core::WorldAccess::Recording)?;
        crate::shapes::try_shape_ray_cast_checked_impl(shape, origin, translation)
    }

    /// Explicitly reject custom-filter installation while recording.
    pub fn try_set_custom_filter<F>(&mut self, _filter: F) -> ApiResult<()>
    where
        F: Fn(crate::ShapeId, crate::ShapeId) -> bool + Send + Sync + 'static,
    {
        self.check_access()?;
        Err(ApiError::RecordingCustomFilterUnsupported)
    }

    /// Explicitly reject pre-solve installation while recording.
    pub fn try_set_pre_solve<F>(&mut self, _pre_solve: F) -> ApiResult<()>
    where
        F: Fn(crate::ShapeId, crate::ShapeId, crate::Position, crate::Vec2) -> bool
            + Send
            + Sync
            + 'static,
    {
        self.check_access()?;
        Err(ApiError::RecordingPreSolveUnsupported)
    }

    /// Stop recording and copy the complete native stream into owned Rust bytes.
    pub fn finish(self) -> Recording {
        self.try_finish()
            .expect("Box2D returned an invalid recording buffer")
    }

    /// Try to stop recording and copy out the complete native stream.
    ///
    /// Returns [`ApiError::InCallback`] if called from a Box2D callback. The consumed session is
    /// then cleaned up after the outermost native owner boundary returns to Rust.
    pub fn try_finish(mut self) -> ApiResult<Recording> {
        self.stop_native()?;
        let bytes = self.native().copy_bytes()?;
        Ok(Recording {
            bytes,
            mixer_requirements: self.mixer_requirements,
        })
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

fn ensure_supported_callbacks(core: &WorldCore) -> ApiResult<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if core
            .custom_filter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
        {
            return Err(ApiError::RecordingCustomFilterUnsupported);
        }
        if core
            .pre_solve
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
        {
            return Err(ApiError::RecordingPreSolveUnsupported);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = core;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Body, BodyDef, QueryFilter, ShapeDef, shapes};

    fn query_world_with_shape() -> World {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyDef::default());
        world.create_circle_shape_for(body, &ShapeDef::default(), &shapes::circle(Vec2::ZERO, 0.5));
        world
    }

    fn assert_probe_counts(probe: &RecordingLifecycleProbe, stops: usize, destroys: usize) {
        assert_eq!(probe.stops.load(Ordering::SeqCst), stops);
        assert_eq!(probe.destroys.load(Ordering::SeqCst), destroys);
    }

    #[test]
    fn try_finish_from_another_world_query_defers_stop_and_destroy() {
        let query_world = query_world_with_shape();
        let mut recorded_world = World::new(crate::WorldDef::default()).unwrap();
        let probe = Arc::new(RecordingLifecycleProbe::default());
        let mut session = recorded_world.start_recording(RecordingCapacity::default());
        session.native.as_mut().unwrap().lifecycle_probe = Some(Arc::clone(&probe));
        let mut session = Some(session);
        let mut finish_error = None;
        let bounds = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let filter = QueryFilter::default();

        let completed = query_world.visit_overlap_aabb(Position::ZERO, bounds, filter, |_| {
            finish_error = session.take().unwrap().try_finish().err();
            assert_eq!(finish_error, Some(ApiError::InCallback));
            assert_probe_counts(&probe, 0, 0);
            false
        });

        assert!(!completed);
        assert_eq!(finish_error, Some(ApiError::InCallback));
        assert_probe_counts(&probe, 1, 1);
    }

    #[test]
    fn drop_from_another_world_query_defers_stop_and_destroy() {
        let query_world = query_world_with_shape();
        let mut recorded_world = World::new(crate::WorldDef::default()).unwrap();
        let probe = Arc::new(RecordingLifecycleProbe::default());
        let mut session = recorded_world.start_recording(RecordingCapacity::default());
        session.native.as_mut().unwrap().lifecycle_probe = Some(Arc::clone(&probe));
        let mut session = Some(session);
        let bounds = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let filter = QueryFilter::default();

        let completed = query_world.visit_overlap_aabb(Position::ZERO, bounds, filter, |_| {
            drop(session.take());
            assert_probe_counts(&probe, 0, 0);
            false
        });

        assert!(!completed);
        assert_probe_counts(&probe, 1, 1);
    }

    #[test]
    fn attached_recording_header_failure_is_raii_recoverable() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        FAIL_NEXT_SIZE_CHECK.with(|fail| fail.set(true));

        assert!(matches!(
            world.try_start_recording(RecordingCapacity::default()),
            Err(ApiError::InvalidNativeRecording)
        ));
        assert_eq!(world.core().activity(), ActivityState::Idle);
        assert!(world.try_counters().is_ok());

        let recording = world.start_recording(RecordingCapacity::default()).finish();
        assert!(!recording.is_empty());
    }

    #[test]
    fn recording_activity_finish_failure_can_be_retried() {
        let world = World::new(crate::WorldDef::default()).unwrap();
        let core = world.core_rc();
        let mut activity = RecordingActivity {
            core: Rc::clone(&core),
            active: true,
        };

        assert_eq!(activity.finish(), Err(ApiError::WorldBusy));
        assert!(activity.active);
        core.set_activity(ActivityState::Idle, ActivityState::Recording)
            .unwrap();
        assert_eq!(activity.finish(), Ok(()));
        assert!(!activity.active);
        assert_eq!(core.activity(), ActivityState::Idle);
    }

    #[test]
    fn recording_capacity_checks_native_signed_boundary() {
        assert_eq!(RecordingCapacity::new(0).unwrap().bytes(), 0);
        assert_eq!(
            RecordingCapacity::new(u64::from(RecordingCapacity::MAX_BYTES))
                .unwrap()
                .bytes(),
            RecordingCapacity::MAX_BYTES
        );
        assert_eq!(
            RecordingCapacity::new(u64::from(RecordingCapacity::MAX_BYTES) + 1),
            Err(ApiError::InvalidArgument)
        );
    }

    #[test]
    fn preexisting_scoped_handle_observes_recording_activity_gate() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let id = world.create_body_id(BodyDef::default());
        let scoped = Body::new(world.core_rc(), id);

        let session = world.start_recording(RecordingCapacity::default());
        assert_eq!(scoped.try_position(), Err(ApiError::WorldBusy));
        drop(session);
        assert!(scoped.try_position().is_ok());
    }
}
