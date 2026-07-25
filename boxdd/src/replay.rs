//! Owned, preflighted Box2D recording playback.
//!
//! A [`ReplayPlayer`] owns the native player and its internal world. It never exposes that world,
//! native identifiers, or pointers. Inspection is available only through closure-scoped views,
//! and every player mutation advances an epoch before entering native code.

mod preflight;

#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{MaterialMixCb, MaterialMixCtx, WorkerCallbackState};
use crate::core::foundation::{ReplayLease, acquire_replay_lease, acquire_transient_lease};
use crate::core::identity_registry::ActiveIdentityRegistry;
use crate::id::{IdBrand, WorldToken};
use crate::{
    Aabb, ApiError, BodyType, FoundationActivityError, MixerRequirements, Position, QueryFilter,
    Recording, Vec2, WorkerCount, WorldTransform,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::{DebugDraw, DebugDrawOptions, MaterialMixInput};
use boxdd_sys::ffi;
use core::cell::Cell;
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

#[inline]
fn note_replay_native_call() {
    #[cfg(test)]
    REPLAY_NATIVE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
}

#[inline]
fn replay_body_type_raw(body: ffi::b2BodyId) -> ffi::b2BodyType {
    note_replay_native_call();
    #[cfg(test)]
    {
        REPLAY_BODY_GET_TYPE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        if let Some(raw) = REPLAY_BODY_GET_TYPE_OVERRIDE.with(Cell::get) {
            return raw;
        }
    }
    unsafe { ffi::b2Body_GetType(body) }
}

/// Monotonic observation epoch for one replay player.
///
/// After callback availability is established, the value advances before every step, seek,
/// restart, and keyframe-policy mutation, including calls that later fail validation or native
/// health checks. Callback reentry is rejected without advancing the epoch.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReplayEpoch(u64);

impl ReplayEpoch {
    const INITIAL: Self = Self(1);

    /// Return the opaque epoch sequence value.
    pub const fn get(self) -> u64 {
        self.0
    }

    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// A malformed recording rejected by the complete Rust preflight parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayMalformedError {
    detail: String,
}

impl ReplayMalformedError {
    fn from_preflight(error: impl fmt::Debug) -> Self {
        Self {
            detail: format!("{error:?}"),
        }
    }

    /// Return a diagnostic description without exposing parser implementation types.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ReplayMalformedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for ReplayMalformedError {}

/// Replay construction, lifecycle, or validated-operation failure.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ReplayError {
    /// The complete recording preflight rejected malformed or incompatible bytes.
    #[error("malformed Box2D recording: {0}")]
    Malformed(ReplayMalformedError),

    /// The wrapper sidecar and configured callback set are not identical.
    #[error("replay mixer set does not match the recording sidecar")]
    MixerSetMismatch {
        /// Callback set persisted alongside the recording bytes.
        required: MixerRequirements,
        /// Callback set supplied by the replay configuration.
        provided: MixerRequirements,
    },

    /// Process-global Box2D activity prevented replay exclusivity.
    #[error(transparent)]
    Foundation(#[from] FoundationActivityError),

    /// A shared safe-wrapper validation failed before native replay work.
    #[error(transparent)]
    Api(#[from] ApiError),

    /// The validated recording is larger than Box2D's signed input-size ABI.
    #[error("recording size exceeds Box2D's signed input-size ABI")]
    InputTooLarge,

    /// Box2D rejected a stream that passed repository-owned preflight.
    #[error("Box2D failed to create a replay player after successful preflight")]
    NativeCreateFailed,

    /// The native player reported a fatal reader or restore failure.
    #[error("Box2D replay entered a terminal native failure state")]
    NativeFailure,

    /// Native player metadata violated the pinned ABI contract.
    #[error("Box2D returned invalid replay metadata")]
    InvalidNativeMetadata,

    /// A requested frame cannot be represented by the pinned signed-int ABI.
    #[error("replay frame is outside the supported native range")]
    FrameOutOfRange,

    /// A recorded query index cannot be represented or is not present in the current frame.
    #[error("replay query index is outside the current frame")]
    QueryOutOfRange,

    /// A keyframe policy contains a zero or ABI-unrepresentable value.
    #[error("invalid replay keyframe policy")]
    InvalidKeyframePolicy,

    /// The replay epoch counter cannot represent another mutation.
    #[error("replay observation epoch exhausted")]
    EpochExhausted,

    /// Native destruction failed to restore the process-global length scale exactly.
    #[error(
        "Box2D replay did not restore length units per meter (expected bits {expected:#010x}, observed {observed:#010x})"
    )]
    LengthScaleNotRestored {
        /// Exact pre-replay `f32` bits.
        expected: u32,
        /// Exact bits observed after native player destruction.
        observed: u32,
    },
}

/// Result of a replay step, seek, or restart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ReplayStatus {
    /// The player is healthy and positioned at a non-terminal frame.
    Advanced {
        /// Current replay frame.
        frame: u32,
    },
    /// The validated stream has reached its end.
    End {
        /// Final replay frame.
        frame: u32,
    },
    /// Replay remains inspectable, but deterministic state first diverged at this frame.
    Diverged {
        /// Current replay frame.
        frame: u32,
        /// First frame whose recorded state or query result did not reproduce.
        first_divergence: u32,
    },
}

/// Validated policy for replay seek keyframes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayKeyframePolicy {
    budget_bytes: usize,
    min_interval_frames: i32,
}

impl ReplayKeyframePolicy {
    /// Construct a policy with explicit non-zero values.
    ///
    /// Upstream treats zero as "keep the previous value", so the safe API rejects zero instead of
    /// presenting it as a successful mutation.
    pub fn new(budget_bytes: u64, min_interval_frames: u64) -> Result<Self, ReplayError> {
        if budget_bytes == 0 || min_interval_frames == 0 {
            return Err(ReplayError::InvalidKeyframePolicy);
        }
        let budget_bytes =
            usize::try_from(budget_bytes).map_err(|_| ReplayError::InvalidKeyframePolicy)?;
        let min_interval_frames =
            i32::try_from(min_interval_frames).map_err(|_| ReplayError::InvalidKeyframePolicy)?;
        Ok(Self {
            budget_bytes,
            min_interval_frames,
        })
    }

    /// Return the configured memory budget.
    pub const fn budget_bytes(self) -> usize {
        self.budget_bytes
    }

    /// Return the finest requested spacing between keyframes.
    pub const fn min_interval_frames(self) -> u32 {
        self.min_interval_frames as u32
    }
}

/// Current keyframe-ring policy and memory use.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayKeyframeState {
    /// Maximum memory retained by native keyframes.
    pub budget_bytes: usize,
    /// Finest requested keyframe spacing.
    pub min_interval_frames: u32,
    /// Current spacing after any native ring widening.
    pub effective_interval_frames: u32,
    /// Bytes currently allocated by native keyframes.
    pub allocated_bytes: usize,
}

/// Immutable metadata resolved when a validated player opens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReplayInfo {
    /// Total number of recorded simulation steps.
    pub frame_count: u32,
    /// Worker count used by the replay world.
    pub worker_count: WorkerCount,
    /// Time step recorded by the first frame, or zero for a recording without steps.
    pub time_step: f32,
    /// Sub-step count recorded by the first frame, or zero for a recording without steps.
    pub sub_step_count: u32,
    /// Length scale temporarily installed while the player is alive.
    pub length_units_per_meter: f32,
    /// Bounds accumulated by the native recording.
    pub bounds: Aabb,
}

/// Configuration consumed by [`ReplayPlayer`] creation.
#[derive(Default)]
pub struct ReplayConfig {
    worker_count: WorkerCount,
    #[cfg(not(target_arch = "wasm32"))]
    friction_mixer: Option<Box<MaterialMixCb>>,
    #[cfg(not(target_arch = "wasm32"))]
    restitution_mixer: Option<Box<MaterialMixCb>>,
}

impl ReplayConfig {
    /// Create a serial replay with Box2D's default material mixing rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the validated replay worker count.
    #[must_use]
    pub fn with_worker_count(mut self, worker_count: WorkerCount) -> Self {
        self.worker_count = worker_count;
        self
    }

    /// Install the deterministic friction mixer required by the recording sidecar.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_friction_mixer<F>(mut self, mixer: F) -> Self
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        replace_mixer_callback(&mut self.friction_mixer, Box::new(mixer));
        self
    }

    /// Install the deterministic restitution mixer required by the recording sidecar.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_restitution_mixer<F>(mut self, mixer: F) -> Self
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        replace_mixer_callback(&mut self.restitution_mixer, Box::new(mixer));
        self
    }

    /// Return the exact callback presence represented by this configuration.
    pub fn mixer_set(&self) -> MixerRequirements {
        #[cfg(not(target_arch = "wasm32"))]
        {
            MixerRequirements::new(
                self.friction_mixer.is_some(),
                self.restitution_mixer.is_some(),
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            MixerRequirements::default()
        }
    }
}

impl fmt::Debug for ReplayConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplayConfig")
            .field("worker_count", &self.worker_count)
            .field("mixer_set", &self.mixer_set())
            .finish()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ReplayConfig {
    fn drop(&mut self) {
        drop_mixer_callbacks(self.friction_mixer.take(), self.restitution_mixer.take());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn drop_mixer_callbacks(
    friction: Option<Box<MaterialMixCb>>,
    restitution: Option<Box<MaterialMixCb>>,
) {
    let mut panic = crate::core::callback_state::PanicSlot::default();
    panic.run_cleanup(|| drop(friction));
    panic.run_cleanup(|| drop(restitution));
    panic.resume_or_forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn replace_mixer_callback(
    target: &mut Option<Box<MaterialMixCb>>,
    replacement: Box<MaterialMixCb>,
) {
    let previous = target.take();
    let mut replacement = Some(replacement);
    let mut panic = crate::core::callback_state::PanicSlot::default();
    panic.run_cleanup(|| drop(previous));
    if panic.has_panicked() {
        panic.run_cleanup(|| drop(replacement.take()));
        panic.resume_or_forget();
    }
    *target = replacement;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReplayLifecycle {
    Live,
    Terminal,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReplayObservation {
    frame: u32,
    at_end: bool,
    first_divergence: Option<u32>,
    keyframes: ReplayKeyframeState,
}

#[cfg(not(target_arch = "wasm32"))]
struct ReplayMixers {
    slot: usize,
    registered: bool,
    friction: Option<Box<MaterialMixCtx>>,
    restitution: Option<Box<MaterialMixCtx>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ReplayMixers {
    fn install(
        world: ffi::b2WorldId,
        worker: &Arc<WorkerCallbackState>,
        friction: Option<Box<MaterialMixCb>>,
        restitution: Option<Box<MaterialMixCb>>,
    ) -> Result<Option<Self>, ReplayError> {
        if friction.is_none() && restitution.is_none() {
            return Ok(None);
        }

        let Some(slot) = crate::core::material_mix_registry::acquire_slot() else {
            drop_mixer_callbacks(friction, restitution);
            return Err(ApiError::CallbackSlotsExhausted.into());
        };
        let friction = friction.map(|cb| {
            Box::new(MaterialMixCtx {
                worker: Arc::clone(worker),
                cb,
            })
        });
        let restitution = restitution.map(|cb| {
            Box::new(MaterialMixCtx {
                worker: Arc::clone(worker),
                cb,
            })
        });
        let installed = Self {
            slot,
            registered: true,
            friction,
            restitution,
        };

        if let Some(context) = installed.friction.as_deref() {
            let pointer = core::ptr::from_ref(context).cast_mut();
            crate::core::material_mix_registry::set_friction_ptr(slot, pointer);
            note_replay_native_call();
            unsafe {
                ffi::b2World_SetFrictionCallback(
                    world,
                    crate::core::material_mix_registry::friction_callback(slot),
                );
            }
        }
        if let Some(context) = installed.restitution.as_deref() {
            let pointer = core::ptr::from_ref(context).cast_mut();
            crate::core::material_mix_registry::set_restitution_ptr(slot, pointer);
            note_replay_native_call();
            unsafe {
                ffi::b2World_SetRestitutionCallback(
                    world,
                    crate::core::material_mix_registry::restitution_callback(slot),
                );
            }
        }

        Ok(Some(installed))
    }

    fn unregister(&mut self) {
        if !self.registered {
            return;
        }
        if self.friction.is_some() {
            crate::core::material_mix_registry::set_friction_ptr(self.slot, core::ptr::null_mut());
        }
        if self.restitution.is_some() {
            crate::core::material_mix_registry::set_restitution_ptr(
                self.slot,
                core::ptr::null_mut(),
            );
        }
        crate::core::material_mix_registry::release_slot(self.slot);
        self.registered = false;
    }

    fn into_callbacks(mut self) -> (Option<Box<MaterialMixCtx>>, Option<Box<MaterialMixCtx>>) {
        self.unregister();
        (self.friction.take(), self.restitution.take())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ReplayMixers {
    fn drop(&mut self) {
        self.unregister();
    }
}

struct ReplayResources {
    player: Option<NonNull<ffi::b2RecPlayer>>,
    input: Option<preflight::ValidatedRecording>,
    #[cfg(not(target_arch = "wasm32"))]
    mixers: Option<ReplayMixers>,
    identities: Option<Arc<ActiveIdentityRegistry>>,
    #[cfg(not(target_arch = "wasm32"))]
    worker_callbacks: Option<Arc<WorkerCallbackState>>,
    lease: Option<ReplayLease>,
    previous_length_scale_bits: u32,
    native_attempted: bool,
    #[cfg(test)]
    input_drop_probe: Option<Arc<std::sync::atomic::AtomicBool>>,
}

impl ReplayResources {
    fn new(
        lease: ReplayLease,
        previous_length_scale_bits: u32,
        input: preflight::ValidatedRecording,
    ) -> Self {
        Self {
            player: None,
            input: Some(input),
            #[cfg(not(target_arch = "wasm32"))]
            mixers: None,
            identities: None,
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks: None,
            lease: Some(lease),
            previous_length_scale_bits,
            native_attempted: false,
            #[cfg(test)]
            input_drop_probe: None,
        }
    }

    fn player(&self) -> NonNull<ffi::b2RecPlayer> {
        self.player.expect("live replay resources own a player")
    }

    fn input(&self) -> &preflight::ValidatedRecording {
        self.input
            .as_ref()
            .expect("live replay resources retain validated input")
    }

    fn take_for_deferred_shutdown(&mut self) -> Self {
        Self {
            player: self.player.take(),
            input: self.input.take(),
            #[cfg(not(target_arch = "wasm32"))]
            mixers: self.mixers.take(),
            identities: self.identities.take(),
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks: self.worker_callbacks.take(),
            lease: self.lease.take(),
            previous_length_scale_bits: self.previous_length_scale_bits,
            native_attempted: core::mem::take(&mut self.native_attempted),
            #[cfg(test)]
            input_drop_probe: self.input_drop_probe.take(),
        }
    }

    fn shutdown(&mut self) -> Result<(), ReplayError> {
        debug_assert!(!crate::core::callback_state::in_callback());
        if let Some(player) = self.player.take() {
            destroy_native_player(player.as_ptr());
        }

        let scale_error = if self.native_attempted {
            self.native_attempted = false;
            note_replay_native_call();
            let observed = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
            if observed != self.previous_length_scale_bits {
                Some(ReplayError::LengthScaleNotRestored {
                    expected: self.previous_length_scale_bits,
                    observed,
                })
            } else {
                None
            }
        } else {
            None
        };

        drop(self.input.take());
        #[cfg(test)]
        if let Some(probe) = self.input_drop_probe.take() {
            probe.store(true, std::sync::atomic::Ordering::SeqCst);
        }

        // Native destruction has joined every worker. No callback can resolve an identity after
        // this point, and the process-global registry must not retain a dead replay token.
        if let Some(identities) = self.identities.take() {
            identities.clear_and_uninstall();
        }

        #[cfg(not(target_arch = "wasm32"))]
        let mut panic = crate::core::callback_state::PanicSlot::default();
        #[cfg(target_arch = "wasm32")]
        let panic = crate::core::callback_state::PanicSlot::default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(mixers) = self.mixers.take() {
                let (friction, restitution) = mixers.into_callbacks();
                panic.run_cleanup(|| drop(friction));
                panic.run_cleanup(|| drop(restitution));
            }
            if let Some(worker_callbacks) = self.worker_callbacks.take()
                && let Some(payload) = worker_callbacks.take_panic()
            {
                panic.run_cleanup(|| drop(payload));
            }
        }

        if scale_error.is_some() {
            // Do not make the corrupted global foundation available to ordinary worlds.
            if let Some(lease) = self.lease.take() {
                core::mem::forget(lease);
            }
        } else {
            drop(self.lease.take());
        }
        panic.resume_or_forget();
        match scale_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReplayCreateFailpoint {
    Null,
    Panic,
}

#[cfg(test)]
thread_local! {
    static REPLAY_CREATE_FAILPOINT: Cell<Option<ReplayCreateFailpoint>> = const { Cell::new(None) };
    static REPLAY_CREATE_CALLS: Cell<usize> = const { Cell::new(0) };
    static REPLAY_DESTROY_CALLS: Cell<usize> = const { Cell::new(0) };
    static REPLAY_NATIVE_CALLS: Cell<usize> = const { Cell::new(0) };
    static REPLAY_BODY_GET_TYPE_OVERRIDE: Cell<Option<ffi::b2BodyType>> = const { Cell::new(None) };
    static REPLAY_BODY_GET_TYPE_CALLS: Cell<usize> = const { Cell::new(0) };
    static REPLAY_FORCE_UNHEALTHY: Cell<bool> = const { Cell::new(false) };
    static REPLAY_OBSERVATION_READS: Cell<usize> = const { Cell::new(0) };
}

#[inline]
fn create_native_player(
    data: *const core::ffi::c_void,
    size: i32,
    worker_count: i32,
) -> *mut ffi::b2RecPlayer {
    note_replay_native_call();
    #[cfg(test)]
    REPLAY_CREATE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));

    #[cfg(test)]
    if let Some(failpoint) = REPLAY_CREATE_FAILPOINT.with(Cell::get) {
        match failpoint {
            ReplayCreateFailpoint::Null => return core::ptr::null_mut(),
            ReplayCreateFailpoint::Panic => panic!("replay creation failpoint"),
        }
    }

    unsafe { ffi::b2RecPlayer_Create(data, size, worker_count) }
}

#[inline]
fn destroy_native_player(player: *mut ffi::b2RecPlayer) {
    note_replay_native_call();
    #[cfg(test)]
    REPLAY_DESTROY_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
    unsafe { ffi::b2RecPlayer_Destroy(player) }
}

impl Drop for ReplayResources {
    fn drop(&mut self) {
        if crate::core::callback_state::in_callback() {
            let resources = self.take_for_deferred_shutdown();
            crate::core::callback_state::defer_callback_cleanup_or_forget(move || {
                drop(resources);
            });
            return;
        }
        if let Err(error) = self.shutdown() {
            if std::thread::panicking() {
                return;
            }
            panic!("failed to shut down Box2D replay safely: {error}");
        }
    }
}

/// Exclusive owner of a validated Box2D replay player and its internal world.
///
/// This type is intentionally neither `Send` nor `Sync`. Source bytes are copied during preflight,
/// then retained for the player's complete lifetime independently of the caller's buffer.
#[must_use = "dropping the replay player destroys its internal world and releases exclusivity"]
pub struct ReplayPlayer {
    resources: ReplayResources,
    #[cfg(not(target_arch = "wasm32"))]
    worker_callbacks: Arc<WorkerCallbackState>,
    world0: u16,
    info: ReplayInfo,
    observation: Cell<ReplayObservation>,
    epoch: Cell<ReplayEpoch>,
    lifecycle: Cell<ReplayLifecycle>,
    _owner_thread: PhantomData<Rc<()>>,
}

impl fmt::Debug for ReplayPlayer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplayPlayer")
            .field("input_len", &self.input_len())
            .field("info", &self.info)
            .field("epoch", &self.epoch())
            .field("frame", &self.frame())
            .field("lifecycle", &self.lifecycle.get())
            .finish()
    }
}

impl ReplayPlayer {
    /// Open a wrapper-owned recording and enforce its captured mixer requirements.
    pub fn open_recording(
        recording: &Recording,
        config: ReplayConfig,
    ) -> Result<Self, ReplayError> {
        Self::open_bytes(recording.as_bytes(), recording.mixer_requirements(), config)
    }

    /// Open native recording bytes with their separately persisted mixer sidecar.
    ///
    /// The input is copied before this function returns. The sidecar must be stored alongside raw
    /// native bytes because Box2D does not encode wrapper mixer requirements in its stream.
    pub fn open_bytes(
        bytes: &[u8],
        requirements: MixerRequirements,
        config: ReplayConfig,
    ) -> Result<Self, ReplayError> {
        #[cfg(not(target_arch = "wasm32"))]
        let mut config = config;
        crate::core::callback_state::check_not_in_callback()?;
        let provided = config.mixer_set();
        if requirements != provided {
            return Err(ReplayError::MixerSetMismatch {
                required: requirements,
                provided,
            });
        }

        let input = {
            let _preflight_lease = acquire_transient_lease()?;
            preflight::preflight_recording(bytes).map_err(|error| {
                ReplayError::Malformed(ReplayMalformedError::from_preflight(error))
            })?
        };
        let input_size =
            i32::try_from(input.bytes().len()).map_err(|_| ReplayError::InputTooLarge)?;

        let lease = acquire_replay_lease()?;
        note_replay_native_call();
        let previous_length_scale_bits = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
        let mut resources = ReplayResources::new(lease, previous_length_scale_bits, input);
        resources.native_attempted = true;
        let raw_player = create_native_player(
            resources.input().bytes().as_ptr().cast(),
            input_size,
            config.worker_count.as_i32(),
        );
        let Some(player) = NonNull::new(raw_player) else {
            resources.shutdown()?;
            return Err(ReplayError::NativeCreateFailed);
        };
        resources.player = Some(player);

        if !native_player_is_healthy(player) {
            resources.shutdown()?;
            return Err(ReplayError::NativeCreateFailed);
        }
        note_replay_native_call();
        let world = unsafe { ffi::b2RecPlayer_GetWorldId(player.as_ptr()) };
        note_replay_native_call();
        if !unsafe { ffi::b2World_IsValid(world) } {
            resources.shutdown()?;
            return Err(ReplayError::NativeCreateFailed);
        }

        let info = match read_replay_info(player, config.worker_count) {
            Ok(info) => info,
            Err(error) => {
                resources.shutdown()?;
                return Err(error);
            }
        };
        let preflight_info = resources.input().info();
        if info.frame_count as usize != preflight_info.steps
            || info.length_units_per_meter.to_bits()
                != preflight_info.length_units_per_meter.to_bits()
        {
            resources.shutdown()?;
            return Err(ReplayError::InvalidNativeMetadata);
        }
        let observation = match read_replay_observation(player, info.frame_count) {
            Ok(observation) => observation,
            Err(error) => {
                resources.shutdown()?;
                return Err(error);
            }
        };

        let token = WorldToken::allocate()?;
        let brand = IdBrand::new(world, token)?;
        let identities = ActiveIdentityRegistry::new(brand);
        resources.identities = Some(Arc::clone(&identities));
        #[cfg(not(target_arch = "wasm32"))]
        let worker_callbacks = WorkerCallbackState::new(brand, identities);
        #[cfg(not(target_arch = "wasm32"))]
        {
            resources.worker_callbacks = Some(Arc::clone(&worker_callbacks));
            resources.mixers = ReplayMixers::install(
                world,
                &worker_callbacks,
                config.friction_mixer.take(),
                config.restitution_mixer.take(),
            )?;
        }

        Ok(Self {
            resources,
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks,
            world0: brand.world0(),
            info,
            observation: Cell::new(observation),
            epoch: Cell::new(ReplayEpoch::INITIAL),
            lifecycle: Cell::new(ReplayLifecycle::Live),
            _owner_thread: PhantomData,
        })
    }

    /// Explicitly destroy the player, verify global scale restoration, and release exclusivity.
    ///
    /// Returns [`ApiError::InCallback`] without entering native code when called from a Box2D
    /// callback. Destruction is deferred to the outer owner-call boundary; when no such boundary
    /// exists, the native player and its replay lease are deliberately retained.
    pub fn close(mut self) -> Result<(), ReplayError> {
        crate::core::callback_state::check_not_in_callback()?;
        self.lifecycle.set(ReplayLifecycle::Closed);
        self.resources.shutdown()
    }

    /// Return the retained Rust-owned input length.
    pub fn input_len(&self) -> usize {
        self.resources.input().bytes().len()
    }

    /// Return immutable recording metadata cached at construction.
    pub const fn info(&self) -> ReplayInfo {
        self.info
    }

    /// Return the current mutation epoch.
    pub fn epoch(&self) -> ReplayEpoch {
        self.epoch.get()
    }

    /// Return the number of replayed steps.
    pub fn frame(&self) -> u32 {
        self.observation.get().frame
    }

    /// Return whether the native reader remains healthy.
    ///
    /// # Panics
    ///
    /// Panics when called from a Box2D callback, before querying the native player.
    pub fn is_healthy(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        self.ensure_native_healthy().is_ok()
    }

    /// Return whether the replay has reached its validated end.
    pub fn is_at_end(&self) -> bool {
        self.observation.get().at_end
    }

    /// Return whether deterministic state has diverged.
    pub fn has_diverged(&self) -> bool {
        self.observation.get().first_divergence.is_some()
    }

    /// Advance by one recorded step.
    pub fn step(&mut self) -> Result<ReplayStatus, ReplayError> {
        self.begin_mutation()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.worker_callbacks.clear_panic();
        }
        note_replay_native_call();
        let stepped = unsafe { ffi::b2RecPlayer_StepFrame(self.player_ptr()) };
        let status = self.status_after_native_call(stepped);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resume_worker_panic();
        }
        status
    }

    /// Seek forward or backward to a recorded frame, clamping only at the validated stream end.
    pub fn seek(&mut self, target_frame: u64) -> Result<ReplayStatus, ReplayError> {
        self.begin_mutation()?;
        let target_frame = i32::try_from(target_frame).map_err(|_| ReplayError::FrameOutOfRange)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.worker_callbacks.clear_panic();
        }
        note_replay_native_call();
        unsafe { ffi::b2RecPlayer_SeekFrame(self.player_ptr(), target_frame) };
        let status = self.status_after_native_call(true);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resume_worker_panic();
        }
        status
    }

    /// Restore the seed snapshot and reset replay to frame zero.
    pub fn restart(&mut self) -> Result<ReplayStatus, ReplayError> {
        self.begin_mutation()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.worker_callbacks.clear_panic();
        }
        note_replay_native_call();
        unsafe { ffi::b2RecPlayer_Restart(self.player_ptr()) };
        let status = self.status_after_native_call(true);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resume_worker_panic();
        }
        status
    }

    /// Replace the keyframe policy and clear the existing native keyframe ring.
    pub fn set_keyframe_policy(&mut self, policy: ReplayKeyframePolicy) -> Result<(), ReplayError> {
        self.begin_mutation()?;
        note_replay_native_call();
        unsafe {
            ffi::b2RecPlayer_SetKeyframePolicy(
                self.player_ptr(),
                policy.budget_bytes,
                policy.min_interval_frames,
            )
        };
        self.refresh_observation().map(|_| ())
    }

    /// Return current keyframe policy and allocation metrics.
    pub fn keyframe_policy(&self) -> ReplayKeyframeState {
        self.observation.get().keyframes
    }

    /// Inspect the internal world through views that cannot escape this closure.
    ///
    /// Returns [`ApiError::InCallback`] before native replay activity when called from a Box2D
    /// callback.
    pub fn with_view<R>(
        &self,
        visit: impl for<'view> FnOnce(ReplayView<'view>) -> R,
    ) -> Result<R, ReplayError> {
        crate::core::callback_state::check_not_in_callback()?;
        self.ensure_native_healthy()?;
        let player = self.resources.player();
        note_replay_native_call();
        let body_count = self.native_metadata(checked_native_count(unsafe {
            ffi::b2RecPlayer_GetBodyCount(player.as_ptr())
        }))?;
        note_replay_native_call();
        let query_count = self.native_metadata(checked_native_count(unsafe {
            ffi::b2RecPlayer_GetFrameQueryCount(player.as_ptr())
        }))?;
        Ok(visit(ReplayView {
            player: self,
            epoch: self.epoch(),
            body_count,
            query_count,
        }))
    }

    /// Draw the player-owned world and optionally one recorded frame query.
    ///
    /// `None` draws every recorded query after the world. A panic from `drawer` is contained while
    /// native code is active and resumes only after this player call returns to Rust.
    /// Returns [`ApiError::InCallback`] before native replay activity when called from a Box2D
    /// callback.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn draw(
        &mut self,
        drawer: &mut impl DebugDraw,
        options: DebugDrawOptions,
        query_index: Option<u32>,
    ) -> Result<(), ReplayError> {
        crate::core::callback_state::check_not_in_callback()?;
        self.ensure_native_healthy()?;
        let query_index = match query_index {
            Some(index) => {
                note_replay_native_call();
                let count = self.native_metadata(checked_native_count(unsafe {
                    ffi::b2RecPlayer_GetFrameQueryCount(self.player_ptr())
                }))?;
                if usize::try_from(index).map_or(true, |index| index >= count) {
                    return Err(ReplayError::QueryOutOfRange);
                }
                i32::try_from(index).map_err(|_| ReplayError::QueryOutOfRange)?
            }
            None => -1,
        };
        note_replay_native_call();
        let world = unsafe { ffi::b2RecPlayer_GetWorldId(self.player_ptr()) };
        let panic = crate::debug_draw::draw_replay_player(
            self.player_ptr(),
            world,
            drawer,
            options,
            query_index,
        )?;
        let status = self.ensure_native_healthy();
        panic.resume_or_forget();
        status
    }

    fn begin_mutation(&self) -> Result<(), ReplayError> {
        crate::core::callback_state::check_not_in_callback()?;
        let Some(next) = self.epoch().checked_next() else {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            return Err(ReplayError::EpochExhausted);
        };
        self.epoch.set(next);
        if self.lifecycle.get() != ReplayLifecycle::Live {
            return Err(ReplayError::NativeFailure);
        }
        self.ensure_native_healthy()
    }

    fn status_after_native_call(&self, mutated: bool) -> Result<ReplayStatus, ReplayError> {
        let observation = self.refresh_observation()?;
        let frame = observation.frame;
        if let Some(first_divergence) = observation.first_divergence {
            Ok(ReplayStatus::Diverged {
                frame,
                first_divergence,
            })
        } else if observation.at_end {
            Ok(ReplayStatus::End { frame })
        } else if !mutated {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            Err(ReplayError::NativeFailure)
        } else {
            Ok(ReplayStatus::Advanced { frame })
        }
    }

    fn ensure_native_healthy(&self) -> Result<(), ReplayError> {
        if self.lifecycle.get() != ReplayLifecycle::Live {
            return Err(ReplayError::NativeFailure);
        }
        if native_player_is_healthy(self.resources.player()) {
            Ok(())
        } else {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            Err(ReplayError::NativeFailure)
        }
    }

    fn refresh_observation(&self) -> Result<ReplayObservation, ReplayError> {
        self.ensure_native_healthy()?;
        let observation =
            match read_replay_observation(self.resources.player(), self.info.frame_count) {
                Ok(observation) => observation,
                Err(error) => {
                    self.lifecycle.set(ReplayLifecycle::Terminal);
                    return Err(error);
                }
            };
        self.observation.set(observation);
        Ok(observation)
    }

    fn native_metadata<T>(&self, result: Result<T, ReplayError>) -> Result<T, ReplayError> {
        result.inspect_err(|_| {
            self.lifecycle.set(ReplayLifecycle::Terminal);
        })
    }

    fn raw_object_is_owned(&self, index1: i32, world0: u16) -> bool {
        index1 > 0 && world0 == self.world0
    }

    fn live_body_is_owned(&self, raw: ffi::b2BodyId) -> bool {
        if raw.index1 <= 0 {
            return false;
        }
        if raw.world0 != self.world0 {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            return false;
        }
        note_replay_native_call();
        if !unsafe { ffi::b2Body_IsValid(raw) } {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            return false;
        }
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resume_worker_panic(&self) {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        if let Some(payload) = self.worker_callbacks.take_panic() {
            panic.capture(payload);
        }
        panic.resume_or_forget();
    }

    fn player_ptr(&self) -> *mut ffi::b2RecPlayer {
        self.resources.player().as_ptr()
    }
}

impl Drop for ReplayPlayer {
    fn drop(&mut self) {
        self.lifecycle.set(ReplayLifecycle::Closed);
        if crate::core::callback_state::in_callback() {
            return;
        }
        if let Err(error) = self.resources.shutdown() {
            if std::thread::panicking() {
                return;
            }
            panic!("failed to shut down Box2D replay safely: {error}");
        }
    }
}

/// Closure-scoped read access to one replay epoch.
pub struct ReplayView<'view> {
    player: &'view ReplayPlayer,
    epoch: ReplayEpoch,
    body_count: usize,
    query_count: usize,
}

impl ReplayView<'_> {
    /// Return the observation epoch shared by every child view.
    pub const fn epoch(&self) -> ReplayEpoch {
        self.epoch
    }

    /// Return cached recording metadata.
    pub const fn info(&self) -> ReplayInfo {
        self.player.info
    }

    /// Return the creation-order body-slot span, including destroyed holes.
    pub const fn body_count(&self) -> usize {
        self.body_count
    }

    /// Inspect a live body by its stable replay creation ordinal.
    ///
    /// # Panics
    ///
    /// Panics when called from a Box2D callback, before querying the native player.
    pub fn body(&self, ordinal: usize) -> Option<ReplayBodyView<'_>> {
        crate::core::callback_state::assert_not_in_callback();
        self.check_epoch().ok()?;
        if ordinal >= self.body_count || ordinal > i32::MAX as usize {
            return None;
        }
        note_replay_native_call();
        let raw = unsafe { ffi::b2RecPlayer_GetBodyId(self.player.player_ptr(), ordinal as i32) };
        if !self.player.live_body_is_owned(raw) {
            return None;
        }
        Some(ReplayBodyView {
            player: self.player,
            epoch: self.epoch,
            ordinal,
            raw,
        })
    }

    /// Return the number of spatial queries captured for the current replayed frame.
    pub const fn query_count(&self) -> usize {
        self.query_count
    }

    /// Inspect one recorded query without exposing its native shape identifier.
    ///
    /// # Panics
    ///
    /// Panics when called from a Box2D callback, before querying the native player.
    pub fn query(&self, index: usize) -> Option<ReplayQueryView<'_>> {
        crate::core::callback_state::assert_not_in_callback();
        self.check_epoch().ok()?;
        if index >= self.query_count || index > i32::MAX as usize {
            return None;
        }
        note_replay_native_call();
        let raw = unsafe { ffi::b2RecPlayer_GetFrameQuery(self.player.player_ptr(), index as i32) };
        let Some(kind) = ReplayQueryKind::from_raw(raw.type_) else {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return None;
        };
        if matches!(
            kind,
            ReplayQueryKind::ShapeTestPoint | ReplayQueryKind::ShapeRayCast
        ) && !self
            .player
            .raw_object_is_owned(raw.shape.index1, raw.shape.world0)
        {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return None;
        }
        let Ok(hit_count) = usize::try_from(raw.hitCount) else {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return None;
        };
        let aabb = Aabb::from_raw(raw.aabb);
        let origin = Position::from_raw(raw.origin);
        let translation = Vec2::from_raw(raw.translation);
        if !aabb.is_valid() || !origin.is_valid() || !translation.is_valid() {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return None;
        }
        Some(ReplayQueryView {
            player: self.player,
            epoch: self.epoch,
            index,
            kind,
            filter: QueryFilter(raw.filter),
            aabb,
            origin,
            translation,
            hit_count,
        })
    }

    fn check_epoch(&self) -> Result<(), ReplayError> {
        if self.player.epoch() == self.epoch {
            self.player.ensure_native_healthy()
        } else {
            Err(ReplayError::NativeFailure)
        }
    }
}

/// A body in a single closure-scoped replay epoch.
pub struct ReplayBodyView<'view> {
    player: &'view ReplayPlayer,
    epoch: ReplayEpoch,
    ordinal: usize,
    raw: ffi::b2BodyId,
}

impl ReplayBodyView<'_> {
    /// Return the observation epoch that authorizes this view.
    pub const fn epoch(&self) -> ReplayEpoch {
        self.epoch
    }

    /// Return the stable creation ordinal used by the replay outliner.
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Return whether this body remains live in the current epoch.
    ///
    /// # Panics
    ///
    /// Panics when called from a Box2D callback, before querying the native player.
    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        self.check_epoch() && self.player.live_body_is_owned(self.raw)
    }

    /// Return the body's simulation type.
    ///
    /// # Panics
    ///
    /// Panics if the view is stale, the replay is terminal, the body is no longer valid, this is
    /// called from a Box2D callback, or Box2D returns an unknown body-type discriminant.
    pub fn body_type(&self) -> BodyType {
        self.try_body_type()
            .expect("replay body is unavailable or Box2D returned an unknown body type")
    }

    /// Try to return the body's simulation type.
    ///
    /// An unknown native discriminant returns [`ApiError::InvalidNativeBodyType`] and permanently
    /// terminalizes the replay player. Later native operations fail with
    /// [`ReplayError::NativeFailure`] before entering Box2D.
    pub fn try_body_type(&self) -> Result<BodyType, ReplayError> {
        crate::core::callback_state::check_not_in_callback()?;
        self.try_check_valid()?;
        self.resolve_body_type_output(replay_body_type_raw(self.raw))
    }

    /// Return the world-space body position.
    pub fn position(&self) -> Position {
        self.assert_valid();
        note_replay_native_call();
        Position::from_raw(unsafe { ffi::b2Body_GetPosition(self.raw) })
    }

    /// Return the world-space body transform.
    pub fn transform(&self) -> WorldTransform {
        self.assert_valid();
        note_replay_native_call();
        WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(self.raw) })
    }

    /// Return the body's linear velocity.
    pub fn linear_velocity(&self) -> Vec2 {
        self.assert_valid();
        note_replay_native_call();
        Vec2::from_raw(unsafe { ffi::b2Body_GetLinearVelocity(self.raw) })
    }

    /// Return the body's angular velocity.
    pub fn angular_velocity(&self) -> f32 {
        self.assert_valid();
        note_replay_native_call();
        unsafe { ffi::b2Body_GetAngularVelocity(self.raw) }
    }

    fn check_epoch(&self) -> bool {
        self.player.epoch() == self.epoch && self.player.is_healthy()
    }

    fn try_check_valid(&self) -> Result<(), ReplayError> {
        if self.player.epoch() != self.epoch {
            return Err(ReplayError::NativeFailure);
        }
        self.player.ensure_native_healthy()?;
        if self.player.live_body_is_owned(self.raw) {
            Ok(())
        } else {
            Err(ReplayError::NativeFailure)
        }
    }

    fn resolve_body_type_output(&self, raw: ffi::b2BodyType) -> Result<BodyType, ReplayError> {
        BodyType::decode_native(raw).map_err(|error| {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            error.into()
        })
    }

    fn assert_valid(&self) {
        assert!(
            self.is_valid(),
            "replay body view is stale or no longer valid in this epoch"
        );
    }
}

/// Kind of a recorded spatial query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ReplayQueryKind {
    /// World AABB overlap.
    OverlapAabb,
    /// World shape-proxy overlap.
    OverlapShape,
    /// World ray cast with a visitor.
    CastRay,
    /// World shape cast with a visitor.
    CastShape,
    /// Mover collision query.
    CollideMover,
    /// Closest world ray cast.
    CastRayClosest,
    /// Mover cast.
    CastMover,
    /// Shape point test.
    ShapeTestPoint,
    /// Shape-local ray cast. The native inspection ABI does not expose its cast result.
    ShapeRayCast,
}

impl ReplayQueryKind {
    fn from_raw(raw: ffi::b2RecQueryType) -> Option<Self> {
        match raw {
            ffi::b2RecQueryType_b2_recQueryOverlapAABB => Some(Self::OverlapAabb),
            ffi::b2RecQueryType_b2_recQueryOverlapShape => Some(Self::OverlapShape),
            ffi::b2RecQueryType_b2_recQueryCastRay => Some(Self::CastRay),
            ffi::b2RecQueryType_b2_recQueryCastShape => Some(Self::CastShape),
            ffi::b2RecQueryType_b2_recQueryCollideMover => Some(Self::CollideMover),
            ffi::b2RecQueryType_b2_recQueryCastRayClosest => Some(Self::CastRayClosest),
            ffi::b2RecQueryType_b2_recQueryCastMover => Some(Self::CastMover),
            ffi::b2RecQueryType_b2_recQueryShapeTestPoint => Some(Self::ShapeTestPoint),
            ffi::b2RecQueryType_b2_recQueryShapeRayCast => Some(Self::ShapeRayCast),
            _ => None,
        }
    }
}

/// One recorded query in a single replay epoch.
pub struct ReplayQueryView<'view> {
    player: &'view ReplayPlayer,
    epoch: ReplayEpoch,
    index: usize,
    kind: ReplayQueryKind,
    filter: QueryFilter,
    aabb: Aabb,
    origin: Position,
    translation: Vec2,
    hit_count: usize,
}

impl ReplayQueryView<'_> {
    /// Return the observation epoch that authorizes this view.
    pub const fn epoch(&self) -> ReplayEpoch {
        self.epoch
    }

    /// Return the recorded query family.
    pub const fn kind(&self) -> ReplayQueryKind {
        self.kind
    }

    /// Return the recorded collision filter.
    pub const fn filter(&self) -> QueryFilter {
        self.filter
    }

    /// Return the recorded bounds, or a zero AABB when this query family does not use one.
    pub const fn aabb(&self) -> Aabb {
        self.aabb
    }

    /// Return the recorded world-space origin.
    pub const fn origin(&self) -> Position {
        self.origin
    }

    /// Return the recorded cast translation, or zero when this query family does not use one.
    pub const fn translation(&self) -> Vec2 {
        self.translation
    }

    /// Return the number of results exposed through the native replay hit pool.
    ///
    /// Shape-local ray casts report zero here even when their separately stored native result hit.
    pub const fn hit_count(&self) -> usize {
        self.hit_count
    }

    /// Inspect one recorded result without exposing its native shape identifier.
    ///
    /// World ray and world shape-cast hits expose point, normal, and fraction values. Overlap and
    /// mover-plane hits deliberately expose only their occurrence. Shape-local ray-cast results
    /// are not present in Box2D's public replay hit pool.
    ///
    /// # Panics
    ///
    /// Panics when called from a Box2D callback, before querying the native player.
    pub fn hit(&self, hit_index: usize) -> Option<ReplayQueryHitView<'_>> {
        crate::core::callback_state::assert_not_in_callback();
        if self.player.epoch() != self.epoch
            || !self.player.is_healthy()
            || hit_index >= self.hit_count
            || self.index > i32::MAX as usize
            || hit_index > i32::MAX as usize
        {
            return None;
        }
        note_replay_native_call();
        let raw = unsafe {
            ffi::b2RecPlayer_GetFrameQueryHit(
                self.player.player_ptr(),
                self.index as i32,
                hit_index as i32,
            )
        };
        if !self
            .player
            .raw_object_is_owned(raw.shape.index1, raw.shape.world0)
        {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return None;
        }
        let geometry = match self.kind {
            ReplayQueryKind::CastRay
            | ReplayQueryKind::CastShape
            | ReplayQueryKind::CastRayClosest => {
                let point = Position::from_raw(raw.point);
                let normal = Vec2::from_raw(raw.normal);
                if !point.is_valid() || !normal.is_valid() || !(0.0..=1.0).contains(&raw.fraction) {
                    self.player.lifecycle.set(ReplayLifecycle::Terminal);
                    return None;
                }
                Some(ReplayQueryHitGeometry {
                    point,
                    normal,
                    fraction: raw.fraction,
                })
            }
            ReplayQueryKind::OverlapAabb
            | ReplayQueryKind::OverlapShape
            | ReplayQueryKind::CollideMover
            | ReplayQueryKind::CastMover
            | ReplayQueryKind::ShapeTestPoint
            | ReplayQueryKind::ShapeRayCast => None,
        };
        Some(ReplayQueryHitView {
            _player: self.player,
            epoch: self.epoch,
            geometry,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ReplayQueryHitGeometry {
    point: Position,
    normal: Vec2,
    fraction: f32,
}

/// One recorded query result in a single replay epoch.
pub struct ReplayQueryHitView<'view> {
    _player: &'view ReplayPlayer,
    epoch: ReplayEpoch,
    geometry: Option<ReplayQueryHitGeometry>,
}

impl ReplayQueryHitView<'_> {
    /// Return the observation epoch that authorizes this view.
    pub const fn epoch(&self) -> ReplayEpoch {
        self.epoch
    }

    /// Return the ray/cast hit point when this query family records geometric hit data.
    pub const fn point(&self) -> Option<Position> {
        match self.geometry {
            Some(geometry) => Some(geometry.point),
            None => None,
        }
    }

    /// Return the ray/cast hit normal when this query family records geometric hit data.
    pub const fn normal(&self) -> Option<Vec2> {
        match self.geometry {
            Some(geometry) => Some(geometry.normal),
            None => None,
        }
    }

    /// Return the ray/cast fraction when this query family records geometric hit data.
    pub const fn fraction(&self) -> Option<f32> {
        match self.geometry {
            Some(geometry) => Some(geometry.fraction),
            None => None,
        }
    }
}

fn native_player_is_healthy(player: NonNull<ffi::b2RecPlayer>) -> bool {
    #[cfg(test)]
    if REPLAY_FORCE_UNHEALTHY.with(Cell::get) {
        return false;
    }
    note_replay_native_call();
    unsafe { boxdd_sys::adapter::boxddRecPlayer_IsHealthy(player.as_ptr()) }
}

fn checked_native_count(count: i32) -> Result<usize, ReplayError> {
    usize::try_from(count).map_err(|_| ReplayError::InvalidNativeMetadata)
}

fn read_replay_observation(
    player: NonNull<ffi::b2RecPlayer>,
    frame_count: u32,
) -> Result<ReplayObservation, ReplayError> {
    #[cfg(test)]
    REPLAY_OBSERVATION_READS.with(|reads| reads.set(reads.get().saturating_add(1)));

    let player = player.as_ptr();
    note_replay_native_call();
    let frame = u32::try_from(unsafe { ffi::b2RecPlayer_GetFrame(player) })
        .map_err(|_| ReplayError::InvalidNativeMetadata)?;
    if frame > frame_count {
        return Err(ReplayError::InvalidNativeMetadata);
    }

    note_replay_native_call();
    let first_divergence = if unsafe { ffi::b2RecPlayer_HasDiverged(player) } {
        note_replay_native_call();
        let first = u32::try_from(unsafe { ffi::b2RecPlayer_GetDivergeFrame(player) })
            .map_err(|_| ReplayError::InvalidNativeMetadata)?;
        if first > frame {
            return Err(ReplayError::InvalidNativeMetadata);
        }
        Some(first)
    } else {
        None
    };

    note_replay_native_call();
    let min_interval_frames =
        u32::try_from(unsafe { ffi::b2RecPlayer_GetKeyframeMinInterval(player) })
            .map_err(|_| ReplayError::InvalidNativeMetadata)?;
    note_replay_native_call();
    let effective_interval_frames =
        u32::try_from(unsafe { ffi::b2RecPlayer_GetKeyframeInterval(player) })
            .map_err(|_| ReplayError::InvalidNativeMetadata)?;
    if min_interval_frames == 0 || effective_interval_frames < min_interval_frames {
        return Err(ReplayError::InvalidNativeMetadata);
    }

    note_replay_native_call();
    let at_end = unsafe { ffi::b2RecPlayer_IsAtEnd(player) };
    note_replay_native_call();
    let budget_bytes = unsafe { ffi::b2RecPlayer_GetKeyframeBudget(player) };
    note_replay_native_call();
    let allocated_bytes = unsafe { ffi::b2RecPlayer_GetKeyframeBytes(player) };

    Ok(ReplayObservation {
        frame,
        at_end,
        first_divergence,
        keyframes: ReplayKeyframeState {
            budget_bytes,
            min_interval_frames,
            effective_interval_frames,
            allocated_bytes,
        },
    })
}

fn read_replay_info(
    player: NonNull<ffi::b2RecPlayer>,
    worker_count: WorkerCount,
) -> Result<ReplayInfo, ReplayError> {
    note_replay_native_call();
    let raw = unsafe { ffi::b2RecPlayer_GetInfo(player.as_ptr()) };
    let frame_count =
        u32::try_from(raw.frameCount).map_err(|_| ReplayError::InvalidNativeMetadata)?;
    let native_worker_count = WorkerCount::from_native(raw.workerCount)
        .map_err(|_| ReplayError::InvalidNativeMetadata)?;
    let sub_step_count =
        u32::try_from(raw.subStepCount).map_err(|_| ReplayError::InvalidNativeMetadata)?;
    let bounds = Aabb::from_raw(raw.bounds);
    if native_worker_count != worker_count
        || !raw.timeStep.is_finite()
        || raw.timeStep < 0.0
        || !raw.lengthScale.is_finite()
        || raw.lengthScale <= 0.0
        || !bounds.is_valid()
    {
        return Err(ReplayError::InvalidNativeMetadata);
    }
    Ok(ReplayInfo {
        frame_count,
        worker_count: native_worker_count,
        time_step: raw.timeStep,
        sub_step_count,
        length_units_per_meter: raw.lengthScale,
        bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BodyBuilder, BodyType, DebugDrawOptions, QueryFilter, RecordingCapacity, ShapeDef, World,
        WorldDef, shapes,
    };
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct FailpointReset;

    struct PanicOnDrop(Arc<AtomicBool>);

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            if !self.0.swap(true, Ordering::SeqCst) {
                panic!("replay configuration closure drop panic");
            }
        }
    }

    impl Drop for FailpointReset {
        fn drop(&mut self) {
            REPLAY_CREATE_FAILPOINT.with(|current| current.set(None));
            REPLAY_FORCE_UNHEALTHY.with(|current| current.set(false));
            REPLAY_NATIVE_CALLS.with(|calls| calls.set(0));
        }
    }

    fn one_step_recording() -> Recording {
        let mut world = World::new(WorldDef::default()).unwrap();
        world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let mut session = world.start_recording(RecordingCapacity::default());
        session.step(1.0 / 60.0, 1);
        let recording = session.finish();
        drop(world);
        recording
    }

    fn one_query_recording() -> Recording {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyBuilder::new().build());
        world.create_circle_shape_for(body, &ShapeDef::default(), &shapes::circle(Vec2::ZERO, 0.5));
        let mut session = world.start_recording(RecordingCapacity::default());
        session.step(1.0 / 60.0, 1);
        assert!(
            session
                .cast_ray_closest(
                    Position::new(-2.0, 0.0),
                    Vec2::new(4.0, 0.0),
                    QueryFilter::default(),
                )
                .is_some()
        );
        let recording = session.finish();
        drop(world);
        recording
    }

    fn set_create_failpoint(failpoint: ReplayCreateFailpoint) -> FailpointReset {
        REPLAY_CREATE_FAILPOINT.with(|current| current.set(Some(failpoint)));
        FailpointReset
    }

    fn reset_destroy_calls() {
        REPLAY_DESTROY_CALLS.with(|calls| calls.set(0));
    }

    fn destroy_calls() -> usize {
        REPLAY_DESTROY_CALLS.with(Cell::get)
    }

    fn reset_native_calls() {
        REPLAY_NATIVE_CALLS.with(|calls| calls.set(0));
    }

    fn native_calls() -> usize {
        REPLAY_NATIVE_CALLS.with(Cell::get)
    }

    struct ReplayBodyGetTypeOverride;

    impl ReplayBodyGetTypeOverride {
        fn install(raw: ffi::b2BodyType) -> Self {
            REPLAY_BODY_GET_TYPE_OVERRIDE.with(|current| {
                assert_eq!(current.replace(Some(raw)), None);
            });
            REPLAY_BODY_GET_TYPE_CALLS.with(|calls| calls.set(0));
            Self
        }

        fn calls(&self) -> usize {
            REPLAY_BODY_GET_TYPE_CALLS.with(Cell::get)
        }
    }

    impl Drop for ReplayBodyGetTypeOverride {
        fn drop(&mut self) {
            REPLAY_BODY_GET_TYPE_OVERRIDE.with(|current| current.set(None));
            REPLAY_BODY_GET_TYPE_CALLS.with(|calls| calls.set(0));
        }
    }

    struct NoopDrawer;

    impl DebugDraw for NoopDrawer {}

    #[test]
    fn replay_public_native_entries_reject_callback_before_native_activity() {
        let recording = one_step_recording();
        let policy = ReplayKeyframePolicy::new(1024 * 1024, 1).unwrap();
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        let epoch = player.epoch();
        let observation = player.observation.get();
        let lifecycle = player.lifecycle.get();
        reset_native_calls();

        let callback_guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(player.step(), Err(ReplayError::Api(ApiError::InCallback)));
        assert_eq!(player.seek(0), Err(ReplayError::Api(ApiError::InCallback)));
        assert_eq!(
            player.restart(),
            Err(ReplayError::Api(ApiError::InCallback))
        );
        assert_eq!(
            player.set_keyframe_policy(policy),
            Err(ReplayError::Api(ApiError::InCallback))
        );

        let health = catch_unwind(AssertUnwindSafe(|| player.is_healthy()));
        assert!(health.is_err());

        let visited = Cell::new(false);
        assert_eq!(
            player.with_view(|_| visited.set(true)),
            Err(ReplayError::Api(ApiError::InCallback))
        );
        assert!(!visited.get());
        assert_eq!(
            player.draw(&mut NoopDrawer, DebugDrawOptions::default(), None),
            Err(ReplayError::Api(ApiError::InCallback))
        );
        assert_eq!(player.epoch(), epoch);
        assert_eq!(player.observation.get(), observation);
        assert_eq!(player.frame(), observation.frame);
        assert_eq!(player.lifecycle.get(), lifecycle);
        assert_eq!(native_calls(), 0);

        drop(callback_guard);
        drop(player);
    }

    #[test]
    fn replay_views_reject_callback_before_native_activity_but_work_in_the_view_closure() {
        let recording = one_query_recording();
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.step().unwrap();

        player
            .with_view(|view| {
                let body = view.body(0).expect("recorded body");
                let query = view.query(0).expect("recorded query");
                assert!(query.hit(0).is_some());
                reset_native_calls();

                let callback_guard = crate::core::callback_state::CallbackGuard::enter();
                assert!(
                    catch_unwind(AssertUnwindSafe(|| view.body(0))).is_err(),
                    "ReplayView::body must assert before native replay activity"
                );
                assert!(
                    catch_unwind(AssertUnwindSafe(|| view.query(0))).is_err(),
                    "ReplayView::query must assert before native replay activity"
                );
                assert!(
                    catch_unwind(AssertUnwindSafe(|| body.is_valid())).is_err(),
                    "ReplayBodyView::is_valid must assert before native replay activity"
                );
                assert_eq!(
                    body.try_body_type(),
                    Err(ReplayError::Api(ApiError::InCallback))
                );
                assert!(
                    catch_unwind(AssertUnwindSafe(|| query.hit(0))).is_err(),
                    "ReplayQueryView::hit must assert before native replay activity"
                );
                assert_eq!(native_calls(), 0);
                drop(callback_guard);

                assert!(body.is_valid());
                assert_eq!(body.try_body_type().unwrap(), BodyType::Static);
                assert!(view.body(0).is_some());
                assert!(view.query(0).is_some());
                assert!(query.hit(0).is_some());
                assert!(native_calls() > 0);
            })
            .unwrap();
    }

    #[test]
    fn unknown_replay_body_type_is_precise_then_terminal_without_more_native_calls() {
        let recording = one_query_recording();
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.step().unwrap();

        player
            .with_view(|view| {
                let body = view.body(0).expect("recorded body");
                let raw = ffi::b2BodyType_b2_bodyTypeCount;
                reset_native_calls();
                let get_type = ReplayBodyGetTypeOverride::install(raw);

                assert_eq!(
                    body.try_body_type(),
                    Err(ReplayError::Api(ApiError::InvalidNativeBodyType { raw }))
                );
                assert_eq!(get_type.calls(), 1);
                let native_after_unknown = native_calls();
                assert!(native_after_unknown > 0);
                assert_eq!(body.try_body_type(), Err(ReplayError::NativeFailure));
                assert!(view.body(0).is_none());
                assert_eq!(get_type.calls(), 1);
                assert_eq!(native_calls(), native_after_unknown);
            })
            .unwrap();
    }

    #[test]
    fn infallible_replay_body_type_terminalizes_before_its_unknown_native_panic() {
        let recording = one_query_recording();
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.step().unwrap();

        player
            .with_view(|view| {
                let body = view.body(0).expect("recorded body");
                let raw = ffi::b2BodyType_b2_bodyTypeCount;
                reset_native_calls();
                let get_type = ReplayBodyGetTypeOverride::install(raw);

                assert!(catch_unwind(AssertUnwindSafe(|| body.body_type())).is_err());
                assert_eq!(get_type.calls(), 1);
                let native_after_unknown = native_calls();
                assert!(native_after_unknown > 0);
                assert_eq!(body.try_body_type(), Err(ReplayError::NativeFailure));
                assert_eq!(get_type.calls(), 1);
                assert_eq!(native_calls(), native_after_unknown);
            })
            .unwrap();
    }

    #[test]
    fn replay_close_and_drop_defer_native_shutdown_until_the_owner_callback_returns() {
        let recording = one_step_recording();
        reset_destroy_calls();

        let input_dropped = Arc::new(AtomicBool::new(false));
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.resources.input_drop_probe = Some(Arc::clone(&input_dropped));
        reset_native_calls();
        let owner_scope = crate::core::callback_state::OwnerCallScope::enter();
        let callback_guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(player.close(), Err(ReplayError::Api(ApiError::InCallback)));
        assert_eq!(native_calls(), 0);
        assert_eq!(destroy_calls(), 0);
        assert!(!input_dropped.load(Ordering::SeqCst));
        drop(callback_guard);
        owner_scope.finish(Ok(()), std::iter::empty());
        assert_eq!(destroy_calls(), 1);
        assert!(input_dropped.load(Ordering::SeqCst));
        assert!(!crate::foundation().activity().replay_active);

        let input_dropped = Arc::new(AtomicBool::new(false));
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.resources.input_drop_probe = Some(Arc::clone(&input_dropped));
        reset_native_calls();
        let owner_scope = crate::core::callback_state::OwnerCallScope::enter();
        let callback_guard = crate::core::callback_state::CallbackGuard::enter();
        drop(player);
        assert_eq!(native_calls(), 0);
        assert_eq!(destroy_calls(), 1);
        assert!(!input_dropped.load(Ordering::SeqCst));
        drop(callback_guard);
        owner_scope.finish(Ok(()), std::iter::empty());
        assert_eq!(destroy_calls(), 2);
        assert!(input_dropped.load(Ordering::SeqCst));
        assert!(!crate::foundation().activity().replay_active);
    }

    #[test]
    fn replay_config_drops_every_user_callback_before_resuming_a_panic() {
        let friction_dropped = Arc::new(AtomicBool::new(false));
        let restitution_dropped = Arc::new(AtomicBool::new(false));
        let friction_marker = PanicOnDrop(Arc::clone(&friction_dropped));
        let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
        let config = ReplayConfig::default()
            .with_friction_mixer(move |a, b| {
                let _ = &friction_marker;
                a.coefficient.max(b.coefficient)
            })
            .with_restitution_mixer(move |a, b| {
                let _ = &restitution_marker;
                a.coefficient.max(b.coefficient)
            });

        let panic = catch_unwind(AssertUnwindSafe(|| drop(config)));

        assert!(panic.is_err());
        assert!(friction_dropped.load(Ordering::SeqCst));
        assert!(restitution_dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn replacing_a_mixer_cleans_up_every_panicking_callback() {
        let previous_dropped = Arc::new(AtomicBool::new(false));
        let replacement_dropped = Arc::new(AtomicBool::new(false));
        let restitution_dropped = Arc::new(AtomicBool::new(false));
        let previous_marker = PanicOnDrop(Arc::clone(&previous_dropped));
        let replacement_marker = PanicOnDrop(Arc::clone(&replacement_dropped));
        let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
        let config = ReplayConfig::default()
            .with_friction_mixer(move |a, b| {
                let _ = &previous_marker;
                a.coefficient.max(b.coefficient)
            })
            .with_restitution_mixer(move |a, b| {
                let _ = &restitution_marker;
                a.coefficient.max(b.coefficient)
            });

        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = config.with_friction_mixer(move |a, b| {
                let _ = &replacement_marker;
                a.coefficient.max(b.coefficient)
            });
        }));

        assert!(panic.is_err());
        assert!(previous_dropped.load(Ordering::SeqCst));
        assert!(replacement_dropped.load(Ordering::SeqCst));
        assert!(restitution_dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn null_native_creation_releases_exclusivity_without_destroying_a_player() {
        let recording = one_step_recording();
        reset_destroy_calls();
        let failpoint = set_create_failpoint(ReplayCreateFailpoint::Null);

        let error = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap_err();
        drop(failpoint);

        assert_eq!(error, ReplayError::NativeCreateFailed);
        assert_eq!(destroy_calls(), 0);
        assert!(!crate::foundation().activity().replay_active);
        drop(World::new(WorldDef::default()).unwrap());
    }

    #[test]
    fn creation_panic_releases_exclusivity_during_unwind() {
        let recording = one_step_recording();
        reset_destroy_calls();
        let failpoint = set_create_failpoint(ReplayCreateFailpoint::Panic);

        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = ReplayPlayer::open_recording(&recording, ReplayConfig::default());
        }));
        drop(failpoint);

        assert!(panic.is_err());
        assert_eq!(destroy_calls(), 0);
        assert!(!crate::foundation().activity().replay_active);
        drop(World::new(WorldDef::default()).unwrap());
    }

    #[test]
    fn drop_and_explicit_close_each_destroy_the_native_player_once() {
        let recording = one_step_recording();
        reset_destroy_calls();

        let player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        assert!(crate::foundation().activity().replay_active);
        drop(player);
        assert_eq!(destroy_calls(), 1);
        assert!(!crate::foundation().activity().replay_active);

        let player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.close().unwrap();
        assert_eq!(destroy_calls(), 2);
        assert!(!crate::foundation().activity().replay_active);
    }

    #[test]
    fn failed_length_scale_verification_is_consumed_exactly_once() {
        let observed = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
        let expected = observed ^ 1;
        let mut resources = ReplayResources {
            player: None,
            input: None,
            mixers: None,
            identities: None,
            worker_callbacks: None,
            lease: None,
            previous_length_scale_bits: expected,
            native_attempted: true,
            input_drop_probe: None,
        };

        assert_eq!(
            resources.shutdown(),
            Err(ReplayError::LengthScaleNotRestored { expected, observed })
        );
        assert!(!resources.native_attempted);
        assert_eq!(resources.shutdown(), Ok(()));
    }

    #[test]
    fn unhealthy_native_state_is_terminal_after_advancing_the_epoch() {
        let recording = one_step_recording();
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        let epoch = player.epoch();
        let cached_frame = player.frame();
        let cached_end = player.is_at_end();
        let cached_divergence = player.has_diverged();
        let cached_keyframes = player.keyframe_policy();
        let failpoint = FailpointReset;
        REPLAY_FORCE_UNHEALTHY.with(|current| current.set(true));

        assert_eq!(player.step(), Err(ReplayError::NativeFailure));
        assert!(player.epoch() > epoch);
        assert!(!player.is_healthy());

        drop(failpoint);
        let reads = REPLAY_OBSERVATION_READS.with(Cell::get);
        assert_eq!(player.frame(), cached_frame);
        assert_eq!(player.is_at_end(), cached_end);
        assert_eq!(player.has_diverged(), cached_divergence);
        assert_eq!(player.keyframe_policy(), cached_keyframes);
        assert_eq!(REPLAY_OBSERVATION_READS.with(Cell::get), reads);

        let terminal_epoch = player.epoch();
        assert_eq!(player.step(), Err(ReplayError::NativeFailure));
        assert!(player.epoch() > terminal_epoch);
        assert_eq!(REPLAY_OBSERVATION_READS.with(Cell::get), reads);

        drop(player);
        assert!(!crate::foundation().activity().replay_active);
    }

    #[test]
    fn epoch_exhaustion_terminalizes_before_native_mutation() {
        let recording = one_step_recording();
        let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        player.epoch.set(ReplayEpoch(u64::MAX));
        let frame = player.frame();

        assert_eq!(player.step(), Err(ReplayError::EpochExhausted));
        assert_eq!(player.frame(), frame);
        assert!(!player.is_healthy());

        drop(player);
        assert!(!crate::foundation().activity().replay_active);
    }

    #[test]
    fn foreign_world_object_metadata_terminalizes_the_player() {
        let recording = one_step_recording();
        let player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
        let foreign = ffi::b2BodyId {
            index1: 1,
            world0: player.world0 ^ 1,
            generation: 1,
        };

        assert!(!player.live_body_is_owned(foreign));
        assert_eq!(player.lifecycle.get(), ReplayLifecycle::Terminal);

        drop(player);
        assert!(!crate::foundation().activity().replay_active);
    }
}
