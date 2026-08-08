//! Owned, preflighted Box2D recording playback.
//!
//! A [`ReplayPlayer`] owns the native player and its internal world. It never exposes that world,
//! native identifiers, or pointers. Inspection is available only through closure-scoped views,
//! and every authorized player mutation attempt advances an epoch before further validation.

pub(crate) mod preflight;

#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{MaterialMixCb, MaterialMixCtx, WorkerCallbackState};
use crate::core::foundation::ReplayLease;
use crate::core::identity_registry::ActiveIdentityRegistry;
use crate::core::length_scale::is_safe_length_units_per_meter;
use crate::id::{IdBrand, WorldToken};
use crate::{
    Aabb, BodyType, Error, FoundationActivityError, MixerIdentities, Position, QueryFilter,
    Recording, Result, Vec2, WorkerCount, WorldTransform,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::{DebugDraw, DebugDrawOptions, MaterialMixInput, MixerId};
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
/// Callback reentry is rejected without advancing the value. Once callback availability is
/// established, it advances before lifecycle, native-health, argument, and native validation for
/// every step, seek, restart, and keyframe-policy mutation attempt.
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
    /// Largest keyframe ring budget accepted by the reviewed native writer.
    pub const MAX_BYTES: u64 = crate::RecordingLimits::MAX_BYTES as u64;

    /// Construct a policy with explicit non-zero values.
    ///
    /// Upstream treats zero as "keep the previous value", so the safe API rejects zero instead of
    /// presenting it as a successful mutation. Budgets above the repository-wide allocation
    /// ceiling are rejected rather than silently clamped.
    pub fn new(budget_bytes: u64, min_interval_frames: u64) -> Result<Self> {
        if budget_bytes == 0 || budget_bytes > Self::MAX_BYTES || min_interval_frames == 0 {
            return Err(Error::InvalidReplayKeyframePolicy);
        }
        let budget_bytes =
            usize::try_from(budget_bytes).map_err(|_| Error::InvalidReplayKeyframePolicy)?;
        let min_interval_frames =
            i32::try_from(min_interval_frames).map_err(|_| Error::InvalidReplayKeyframePolicy)?;
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
    #[cfg(not(target_arch = "wasm32"))]
    friction_mixer_id: Option<MixerId>,
    #[cfg(not(target_arch = "wasm32"))]
    restitution_mixer_id: Option<MixerId>,
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
    pub fn with_friction_mixer<F>(mut self, identity: MixerId, mixer: F) -> Self
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        replace_mixer_callback(&mut self.friction_mixer, Box::new(mixer));
        self.friction_mixer_id = Some(identity);
        self
    }

    /// Install the deterministic restitution mixer required by the recording sidecar.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_restitution_mixer<F>(mut self, identity: MixerId, mixer: F) -> Self
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        replace_mixer_callback(&mut self.restitution_mixer, Box::new(mixer));
        self.restitution_mixer_id = Some(identity);
        self
    }

    /// Return the exact material-mixer behavior identities represented by this configuration.
    pub fn mixer_identities(&self) -> MixerIdentities {
        #[cfg(not(target_arch = "wasm32"))]
        {
            debug_assert_eq!(
                self.friction_mixer.is_some(),
                self.friction_mixer_id.is_some()
            );
            debug_assert_eq!(
                self.restitution_mixer.is_some(),
                self.restitution_mixer_id.is_some()
            );
            MixerIdentities::new(self.friction_mixer_id, self.restitution_mixer_id)
        }
        #[cfg(target_arch = "wasm32")]
        {
            MixerIdentities::default()
        }
    }
}

impl fmt::Debug for ReplayConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplayConfig")
            .field("worker_count", &self.worker_count)
            .field("mixer_identities", &self.mixer_identities())
            .finish()
    }
}

fn check_mixer_identities(required: MixerIdentities, config: &ReplayConfig) -> Result<()> {
    let provided = config.mixer_identities();
    if required == provided {
        Ok(())
    } else {
        Err(Error::ReplayMixerIdentityMismatch)
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
    let previous = target.replace(replacement);
    let mut panic = crate::core::callback_state::PanicSlot::default();
    panic.run_cleanup(|| drop(previous));
    panic.resume_or_forget();
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
    owner: crate::core::material_mix_registry::OwnedMaterialMixSlot,
    registered: bool,
    friction: Option<Arc<MaterialMixCtx>>,
    restitution: Option<Arc<MaterialMixCtx>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ReplayMixers {
    fn install(
        world: ffi::b2WorldId,
        worker: &Arc<WorkerCallbackState>,
        friction: Option<(MixerId, Box<MaterialMixCb>)>,
        restitution: Option<(MixerId, Box<MaterialMixCb>)>,
    ) -> Result<Option<Self>> {
        if friction.is_none() && restitution.is_none() {
            return Ok(None);
        }

        let friction = friction.map(|(identity, cb)| {
            (
                identity,
                Arc::new(MaterialMixCtx {
                    worker: Arc::clone(worker),
                    cb,
                }),
            )
        });
        let restitution = restitution.map(|(identity, cb)| {
            (
                identity,
                Arc::new(MaterialMixCtx {
                    worker: Arc::clone(worker),
                    cb,
                }),
            )
        });
        let mut owner = crate::core::material_mix_registry::OwnedMaterialMixSlot::default();
        let mut slot = None;

        if let Some((identity, context)) = friction.as_ref() {
            let registration = crate::core::material_mix_registry::MaterialMixerRegistration::new(
                *identity,
                Arc::clone(context),
            );
            let update = match owner.set_friction(registration) {
                Ok(update) => update,
                Err(failure) => {
                    return Self::install_failed(owner, friction, restitution, failure);
                }
            };
            slot = Some(update.slot());
            update.into_retired().resume_drop_panics();
        }
        if let Some((identity, context)) = restitution.as_ref() {
            let registration = crate::core::material_mix_registry::MaterialMixerRegistration::new(
                *identity,
                Arc::clone(context),
            );
            let update = match owner.set_restitution(registration) {
                Ok(update) => update,
                Err(failure) => {
                    return Self::install_failed(owner, friction, restitution, failure);
                }
            };
            match slot {
                Some(existing) => debug_assert_eq!(existing, update.slot()),
                None => slot = Some(update.slot()),
            }
            update.into_retired().resume_drop_panics();
        }
        let Some(slot) = slot else {
            let retired = owner.detach_after_native_destroyed();
            let mut panic = crate::core::callback_state::PanicSlot::default();
            retired.drain_panics(&mut panic);
            panic.run_cleanup(|| drop(friction));
            panic.run_cleanup(|| drop(restitution));
            panic.resume_or_forget();
            return Err(Error::CallbackSlotsExhausted);
        };
        let installed = Self {
            owner,
            registered: true,
            friction: friction.map(|(_, context)| context),
            restitution: restitution.map(|(_, context)| context),
        };

        if installed.friction.is_some() {
            note_replay_native_call();
            unsafe {
                ffi::b2World_SetFrictionCallback(
                    world,
                    crate::core::material_mix_registry::friction_callback(slot),
                );
            }
        }
        if installed.restitution.is_some() {
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

    fn install_failed(
        mut owner: crate::core::material_mix_registry::OwnedMaterialMixSlot,
        friction: Option<(MixerId, Arc<MaterialMixCtx>)>,
        restitution: Option<(MixerId, Arc<MaterialMixCtx>)>,
        failure: crate::core::material_mix_registry::MaterialMixOperationFailure,
    ) -> Result<Option<Self>> {
        let retired = owner.detach_after_native_destroyed();
        let mut panic = crate::core::callback_state::PanicSlot::default();
        failure.into_retired().drain_panics(&mut panic);
        retired.drain_panics(&mut panic);
        panic.run_cleanup(|| drop(friction));
        panic.run_cleanup(|| drop(restitution));
        panic.resume_or_forget();
        Err(Error::CallbackSlotsExhausted)
    }

    fn unregister(&mut self) -> crate::core::callback_state::PanicSlot {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        if !self.registered {
            return panic;
        }
        let retired = match self.owner.release_all() {
            Ok(retired) => retired,
            Err(failure) => failure.into_retired(),
        };
        self.registered = false;
        retired.drain_panics(&mut panic);
        panic
    }

    fn activate_snapshot(
        &self,
    ) -> core::result::Result<
        Option<crate::core::material_mix_registry::ActiveMaterialMixSnapshot>,
        crate::core::material_mix_registry::MaterialMixRegistryError,
    > {
        self.owner.activate_snapshot()
    }

    fn into_callbacks(
        mut self,
    ) -> (
        Option<Arc<MaterialMixCtx>>,
        Option<Arc<MaterialMixCtx>>,
        crate::core::callback_state::PanicSlot,
    ) {
        let panic = self.unregister();
        (self.friction.take(), self.restitution.take(), panic)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ReplayMixers {
    fn drop(&mut self) {
        let mut panic = self.unregister();
        let friction = self.friction.take();
        let restitution = self.restitution.take();
        panic.run_cleanup(|| drop(friction));
        panic.run_cleanup(|| drop(restitution));
        panic.resume_or_forget();
    }
}

struct ReplayResources {
    player: Option<NonNull<ffi::b2RecPlayer>>,
    owner_token: Option<WorldToken>,
    #[cfg(not(target_arch = "wasm32"))]
    mixers: Option<ReplayMixers>,
    identities: Option<Arc<ActiveIdentityRegistry>>,
    #[cfg(not(target_arch = "wasm32"))]
    worker_callbacks: Option<Arc<WorkerCallbackState>>,
    lease: Option<ReplayLease>,
    previous_length_scale_bits: u32,
    native_attempted: bool,
}

impl ReplayResources {
    fn new(lease: ReplayLease, previous_length_scale_bits: u32) -> Self {
        Self {
            player: None,
            owner_token: None,
            #[cfg(not(target_arch = "wasm32"))]
            mixers: None,
            identities: None,
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks: None,
            lease: Some(lease),
            previous_length_scale_bits,
            native_attempted: false,
        }
    }

    fn player(&self) -> NonNull<ffi::b2RecPlayer> {
        self.player.expect("live replay resources own a player")
    }

    fn owner_token(&self) -> WorldToken {
        self.owner_token
            .expect("live replay resources own a callback owner token")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn worker_callbacks(&self) -> &WorkerCallbackState {
        self.worker_callbacks
            .as_deref()
            .expect("live replay resources own worker callback state")
    }

    fn take_for_deferred_shutdown(&mut self) -> Self {
        Self {
            player: self.player.take(),
            owner_token: self.owner_token.take(),
            #[cfg(not(target_arch = "wasm32"))]
            mixers: self.mixers.take(),
            identities: self.identities.take(),
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks: self.worker_callbacks.take(),
            lease: self.lease.take(),
            previous_length_scale_bits: self.previous_length_scale_bits,
            native_attempted: core::mem::take(&mut self.native_attempted),
        }
    }

    fn shutdown(&mut self) -> Result<()> {
        debug_assert!(!crate::core::callback_state::in_callback());
        if let Some(player) = self.player.take() {
            destroy_native_player(player.as_ptr());
        }

        let scale_error = if self.native_attempted {
            self.native_attempted = false;
            note_replay_native_call();
            let observed = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
            if observed != self.previous_length_scale_bits {
                Some(Error::ReplayLengthScaleNotRestored {
                    expected: self.previous_length_scale_bits,
                    observed,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Native destruction has joined every worker. No callback can resolve an identity after
        // this point, so the replay-local registry can release every registration.
        if let Some(identities) = self.identities.take() {
            identities.clear();
        }

        #[cfg(not(target_arch = "wasm32"))]
        let mut panic = crate::core::callback_state::PanicSlot::default();
        #[cfg(target_arch = "wasm32")]
        let panic = crate::core::callback_state::PanicSlot::default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(mixers) = self.mixers.take() {
                let (friction, restitution, mixer_panic) = mixers.into_callbacks();
                panic.absorb(mixer_panic);
                panic.run_cleanup(|| drop(friction));
                panic.run_cleanup(|| drop(restitution));
            }
            if let Some(worker_callbacks) = self.worker_callbacks.take() {
                worker_callbacks.drain_panics(&mut panic);
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
    static REPLAY_FAIL_HEALTH_AFTER_RESTART: Cell<bool> = const { Cell::new(false) };
    static REPLAY_RESTART_CALLS: Cell<usize> = const { Cell::new(0) };
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

#[inline]
fn restart_native_player(player: *mut ffi::b2RecPlayer) {
    note_replay_native_call();
    #[cfg(test)]
    REPLAY_RESTART_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
    unsafe { ffi::b2RecPlayer_Restart(player) };
    #[cfg(test)]
    if REPLAY_FAIL_HEALTH_AFTER_RESTART.with(|armed| armed.replace(false)) {
        REPLAY_FORCE_UNHEALTHY.with(|current| current.set(true));
    }
}

impl Drop for ReplayResources {
    fn drop(&mut self) {
        if crate::core::callback_state::in_callback() {
            let resources = self.take_for_deferred_shutdown();
            let owner = crate::core::callback_state::CallbackOwnerToken::world(
                resources
                    .owner_token
                    .expect("live replay resources own a callback owner token"),
            );
            crate::core::callback_state::defer_callback_cleanup_or_forget(owner, move || {
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
/// This type is intentionally neither `Send` nor `Sync`. Native creation synchronously copies the
/// private stream, so the player remains independent of the source [`Recording`] after `open`.
#[must_use = "dropping the replay player destroys its internal world and releases exclusivity"]
pub struct ReplayPlayer {
    resources: ReplayResources,
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
            .field("info", &self.info)
            .field("epoch", &self.epoch())
            .field("frame", &self.frame())
            .field("lifecycle", &self.lifecycle.get())
            .finish()
    }
}

impl ReplayPlayer {
    /// Open an opaque process-local recording and enforce its mixer behavior identities.
    pub fn open(
        foundation: &'static crate::Foundation,
        recording: &Recording,
        config: ReplayConfig,
    ) -> Result<Self> {
        crate::core::callback_state::check_not_in_callback()?;
        check_mixer_identities(recording.mixer_identities(), &config)?;
        Self::open_preflighted(
            foundation,
            recording.native_stream(),
            recording.preflight_info(),
            config,
        )
    }

    fn open_preflighted(
        foundation: &'static crate::Foundation,
        bytes: &[u8],
        preflight_info: preflight::PreflightInfo,
        config: ReplayConfig,
    ) -> Result<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        let mut config = config;
        let input_size =
            i32::try_from(bytes.len()).map_err(|_| Error::InvalidNativeReplayMetadata)?;

        let lease = foundation
            .acquire_replay_lease()
            .map_err(|error| match error {
                FoundationActivityError::ReplayUnavailable { activity }
                    if activity.replay_active =>
                {
                    Error::FoundationActivity(FoundationActivityError::ReplayActive)
                }
                error => Error::FoundationActivity(error),
            })?;
        note_replay_native_call();
        let previous_length_scale_bits = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
        let mut resources = ReplayResources::new(lease, previous_length_scale_bits);
        resources.native_attempted = true;
        let raw_player = create_native_player(
            bytes.as_ptr().cast(),
            input_size,
            config.worker_count.as_i32(),
        );
        let Some(player) = NonNull::new(raw_player) else {
            resources.shutdown()?;
            return Err(Error::ReplayNativeCreateFailed);
        };
        resources.player = Some(player);

        if !native_player_is_healthy(player) {
            resources.shutdown()?;
            return Err(Error::ReplayNativeCreateFailed);
        }
        note_replay_native_call();
        let world = unsafe { ffi::b2RecPlayer_GetWorldId(player.as_ptr()) };
        note_replay_native_call();
        if !unsafe { ffi::b2World_IsValid(world) } {
            resources.shutdown()?;
            return Err(Error::ReplayNativeCreateFailed);
        }

        let info = match read_replay_info(player, config.worker_count) {
            Ok(info) => info,
            Err(error) => {
                resources.shutdown()?;
                return Err(error);
            }
        };
        if info.frame_count as usize != preflight_info.steps
            || info.length_units_per_meter.to_bits()
                != preflight_info.length_units_per_meter.to_bits()
        {
            resources.shutdown()?;
            return Err(Error::InvalidNativeReplayMetadata);
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
        resources.owner_token = Some(token);
        resources.identities = Some(identities);
        #[cfg(not(target_arch = "wasm32"))]
        let worker_callbacks = WorkerCallbackState::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let friction_mixer = config.friction_mixer_id.zip(config.friction_mixer.take());
            let restitution_mixer = config
                .restitution_mixer_id
                .zip(config.restitution_mixer.take());
            resources.mixers =
                ReplayMixers::install(world, &worker_callbacks, friction_mixer, restitution_mixer)?;
            resources.worker_callbacks = Some(worker_callbacks);
        }

        Ok(Self {
            resources,
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
    /// Returns [`Error::InCallback`] without entering native code when called from a Box2D
    /// callback. Destruction is deferred to the outer owner-call boundary; when no such boundary
    /// exists, the native player and its replay lease are deliberately retained.
    pub fn close(mut self) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.lifecycle.set(ReplayLifecycle::Closed);
        self.resources.shutdown()
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
    pub fn is_healthy(&self) -> Result<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        if self.lifecycle.get() != ReplayLifecycle::Live {
            return Ok(false);
        }
        if native_player_is_healthy(self.resources.player()) {
            Ok(true)
        } else {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            Ok(false)
        }
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
    pub fn step(&mut self) -> Result<ReplayStatus> {
        self.begin_mutation()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resources.worker_callbacks().begin_call()?;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let material_mix = self.activate_material_mix_snapshot()?;
        crate::core::callback_state::run_replay_step_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(self.resources.owner_token()),
            || {
                #[cfg(not(target_arch = "wasm32"))]
                let _material_mix = material_mix;
                note_replay_native_call();
                unsafe { ffi::b2RecPlayer_StepFrame(self.player_ptr()) }
            },
            |stepped, _panic| {
                #[cfg(not(target_arch = "wasm32"))]
                _panic.absorb(self.take_worker_panic());
                stepped.map(|stepped| self.status_after_native_call(stepped))
            },
        )
    }

    /// Seek forward or backward to a recorded frame, clamping only at the validated stream end.
    pub fn seek(&mut self, target_frame: u64) -> Result<ReplayStatus> {
        self.begin_mutation_attempt()?;
        let target_frame = i32::try_from(target_frame).map_err(|_| Error::ReplayFrameOutOfRange)?;
        self.mutation_state_preflight()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resources.worker_callbacks().begin_call()?;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let material_mix = self.activate_material_mix_snapshot()?;
        crate::core::callback_state::run_replay_seek_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(self.resources.owner_token()),
            || {
                #[cfg(not(target_arch = "wasm32"))]
                let _material_mix = material_mix;
                note_replay_native_call();
                unsafe { ffi::b2RecPlayer_SeekFrame(self.player_ptr(), target_frame) };
            },
            |native, _panic| {
                #[cfg(not(target_arch = "wasm32"))]
                _panic.absorb(self.take_worker_panic());
                native.map(|()| self.status_after_native_call(true))
            },
        )
    }

    /// Restore the seed snapshot and reset replay to frame zero.
    pub fn restart(&mut self) -> Result<ReplayStatus> {
        self.begin_mutation()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resources.worker_callbacks().begin_call()?;
        }
        crate::core::callback_state::run_replay_restart_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(self.resources.owner_token()),
            || {
                restart_native_player(self.player_ptr());
            },
            |native, _panic| {
                #[cfg(not(target_arch = "wasm32"))]
                _panic.absorb(self.take_worker_panic());
                native.map(|()| self.status_after_native_call(true))
            },
        )
    }

    /// Replace the keyframe policy and clear the existing native keyframe ring.
    pub fn set_keyframe_policy(&mut self, policy: ReplayKeyframePolicy) -> Result<()> {
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
    /// The visitor's `Error` is returned directly, so fallible view reads compose with `?`
    /// without producing a nested `Result`.
    ///
    /// Returns [`Error::InCallback`] before native replay activity when called from a Box2D
    /// callback.
    pub fn with_view<R>(
        &self,
        visit: impl for<'view> FnOnce(ReplayView<'view>) -> Result<R>,
    ) -> Result<R> {
        let visit = crate::core::callback_state::PendingUserValue::new(visit);
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
        visit.into_inner()(ReplayView {
            player: self,
            epoch: self.epoch(),
            body_count,
            query_count,
        })
    }

    /// Draw the player-owned world and optionally one recorded frame query.
    ///
    /// `None` draws every recorded query after the world. A panic from `drawer` is contained while
    /// native code is active and resumes only after this player call returns to Rust.
    /// Returns [`Error::InCallback`] before native replay activity when called from a Box2D
    /// callback.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn draw(
        &mut self,
        drawer: &mut impl DebugDraw,
        options: DebugDrawOptions,
        query_index: Option<u32>,
    ) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.ensure_native_healthy()?;
        let query_index = match query_index {
            Some(index) => {
                note_replay_native_call();
                let count = self.native_metadata(checked_native_count(unsafe {
                    ffi::b2RecPlayer_GetFrameQueryCount(self.player_ptr())
                }))?;
                if usize::try_from(index).map_or(true, |index| index >= count) {
                    return Err(Error::ReplayQueryOutOfRange);
                }
                i32::try_from(index).map_err(|_| Error::ReplayQueryOutOfRange)?
            }
            None => -1,
        };
        note_replay_native_call();
        let world = unsafe { ffi::b2RecPlayer_GetWorldId(self.player_ptr()) };
        crate::core::callback_state::run_replay_draw_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(self.resources.owner_token()),
            || {
                crate::debug_draw::draw_replay_player(
                    self.player_ptr(),
                    world,
                    drawer,
                    options,
                    query_index,
                )
            },
            |native, panic| {
                native.map(|result| match result {
                    Ok(callback_panic) => {
                        panic.absorb(callback_panic);
                        self.ensure_native_healthy()
                    }
                    Err(error) => Err(error),
                })
            },
        )
    }

    fn begin_mutation(&self) -> Result<()> {
        self.begin_mutation_attempt()?;
        self.mutation_state_preflight()
    }

    fn begin_mutation_attempt(&self) -> Result<()> {
        Self::mutation_callback_preflight()?;
        self.advance_mutation_epoch()
    }

    fn mutation_callback_preflight() -> Result<()> {
        crate::core::callback_state::check_not_in_callback()
    }

    fn mutation_state_preflight(&self) -> Result<()> {
        if self.lifecycle.get() != ReplayLifecycle::Live {
            return Err(Error::ReplayNativeFailure);
        }
        self.ensure_native_healthy()
    }

    fn advance_mutation_epoch(&self) -> Result<()> {
        let Some(next) = self.epoch().checked_next() else {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            return Err(Error::ReplayEpochExhausted);
        };
        self.epoch.set(next);
        Ok(())
    }

    fn status_after_native_call(&self, mutated: bool) -> Result<ReplayStatus> {
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
            Err(Error::ReplayNativeFailure)
        } else {
            Ok(ReplayStatus::Advanced { frame })
        }
    }

    fn ensure_native_healthy(&self) -> Result<()> {
        if self.lifecycle.get() != ReplayLifecycle::Live {
            return Err(Error::ReplayNativeFailure);
        }
        if native_player_is_healthy(self.resources.player()) {
            Ok(())
        } else {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            Err(Error::ReplayNativeFailure)
        }
    }

    fn refresh_observation(&self) -> Result<ReplayObservation> {
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

    fn native_metadata<T>(&self, result: Result<T>) -> Result<T> {
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
    fn take_worker_panic(&self) -> crate::core::callback_state::PanicSlot {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        self.resources.worker_callbacks().drain_panics(&mut panic);
        panic
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn activate_material_mix_snapshot(
        &self,
    ) -> Result<Option<crate::core::material_mix_registry::ActiveMaterialMixSnapshot>> {
        let snapshot = match self.resources.mixers.as_ref() {
            Some(mixers) => mixers.activate_snapshot(),
            None => Ok(None),
        };
        snapshot.map_err(|_| {
            self.lifecycle.set(ReplayLifecycle::Terminal);
            Error::ReplayNativeFailure
        })
    }

    fn player_ptr(&self) -> *mut ffi::b2RecPlayer {
        self.resources.player().as_ptr()
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
    pub fn body(&self, ordinal: usize) -> Result<Option<ReplayBodyView<'_>>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.check_epoch()?;
        if ordinal >= self.body_count || ordinal > i32::MAX as usize {
            return Ok(None);
        }
        note_replay_native_call();
        let raw = unsafe { ffi::b2RecPlayer_GetBodyId(self.player.player_ptr(), ordinal as i32) };
        if raw.index1 <= 0 {
            return Ok(None);
        }
        if !self.player.live_body_is_owned(raw) {
            return Err(Error::ReplayNativeFailure);
        }
        Ok(Some(ReplayBodyView {
            player: self.player,
            epoch: self.epoch,
            ordinal,
            raw,
        }))
    }

    /// Return the number of spatial queries captured for the current replayed frame.
    pub const fn query_count(&self) -> usize {
        self.query_count
    }

    /// Inspect one recorded query without exposing its native shape identifier.
    pub fn query(&self, index: usize) -> Result<Option<ReplayQueryView<'_>>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.check_epoch()?;
        if index >= self.query_count || index > i32::MAX as usize {
            return Ok(None);
        }
        note_replay_native_call();
        let raw = unsafe { ffi::b2RecPlayer_GetFrameQuery(self.player.player_ptr(), index as i32) };
        let Some(kind) = ReplayQueryKind::from_raw(raw.type_) else {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return Err(Error::InvalidNativeReplayMetadata);
        };
        if matches!(
            kind,
            ReplayQueryKind::ShapeTestPoint | ReplayQueryKind::ShapeRayCast
        ) && !self
            .player
            .raw_object_is_owned(raw.shape.index1, raw.shape.world0)
        {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return Err(Error::InvalidNativeReplayMetadata);
        }
        let Ok(hit_count) = usize::try_from(raw.hitCount) else {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return Err(Error::InvalidNativeReplayMetadata);
        };
        // SAFETY: the replay preflight validates recorded mover bounds before this conversion.
        let aabb = Aabb::from_raw_unvalidated(raw.aabb);
        let origin = Position::from_raw(raw.origin);
        let translation = Vec2::from_raw(raw.translation);
        if !aabb.is_valid() || !origin.is_valid() || !translation.is_valid() {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            return Err(Error::InvalidNativeReplayMetadata);
        }
        Ok(Some(ReplayQueryView {
            player: self.player,
            epoch: self.epoch,
            index,
            kind,
            filter: QueryFilter(raw.filter),
            aabb,
            origin,
            translation,
            hit_count,
        }))
    }

    fn check_epoch(&self) -> Result<()> {
        if self.player.epoch() == self.epoch {
            self.player.ensure_native_healthy()
        } else {
            Err(Error::ReplayNativeFailure)
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
    pub fn is_valid(&self) -> Result<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        if self.player.epoch() != self.epoch || !self.player.is_healthy()? {
            return Ok(false);
        }
        Ok(self.player.live_body_is_owned(self.raw))
    }

    /// Return the body's simulation type.
    ///
    /// An unknown native discriminant returns [`Error::InvalidNativeBodyType`] and permanently
    /// terminalizes the replay player. Later native operations fail with
    /// [`Error::ReplayNativeFailure`] before entering Box2D.
    pub fn body_type(&self) -> Result<BodyType> {
        self.try_check_valid()?;
        self.resolve_body_type_output(replay_body_type_raw(self.raw))
    }

    /// Return the world-space body position.
    pub fn position(&self) -> Result<Position> {
        self.try_check_valid()?;
        note_replay_native_call();
        let value = Position::from_raw(unsafe { ffi::b2Body_GetPosition(self.raw) });
        self.validate_output(value, Position::is_valid)
    }

    /// Return the world-space body transform.
    pub fn transform(&self) -> Result<WorldTransform> {
        self.try_check_valid()?;
        note_replay_native_call();
        let value =
            WorldTransform::from_raw_unvalidated(unsafe { ffi::b2Body_GetTransform(self.raw) });
        self.validate_output(value, WorldTransform::is_valid)
    }

    /// Return the body's linear velocity.
    pub fn linear_velocity(&self) -> Result<Vec2> {
        self.try_check_valid()?;
        note_replay_native_call();
        let value = Vec2::from_raw(unsafe { ffi::b2Body_GetLinearVelocity(self.raw) });
        self.validate_output(value, Vec2::is_valid)
    }

    /// Return the body's angular velocity.
    pub fn angular_velocity(&self) -> Result<f32> {
        self.try_check_valid()?;
        note_replay_native_call();
        let value = unsafe { ffi::b2Body_GetAngularVelocity(self.raw) };
        self.validate_output(value, crate::is_valid_float)
    }

    fn try_check_valid(&self) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        if self.player.epoch() != self.epoch {
            return Err(Error::ReplayNativeFailure);
        }
        self.player.ensure_native_healthy()?;
        if self.player.live_body_is_owned(self.raw) {
            Ok(())
        } else {
            Err(Error::ReplayNativeFailure)
        }
    }

    fn resolve_body_type_output(&self, raw: ffi::b2BodyType) -> Result<BodyType> {
        BodyType::decode_native(raw).inspect_err(|_| {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
        })
    }

    fn validate_output<T>(&self, value: T, validate: impl FnOnce(T) -> bool) -> Result<T>
    where
        T: Copy,
    {
        if validate(value) {
            Ok(value)
        } else {
            self.player.lifecycle.set(ReplayLifecycle::Terminal);
            Err(Error::InvalidNativeReplayMetadata)
        }
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
    pub fn hit(&self, hit_index: usize) -> Result<Option<ReplayQueryHitView<'_>>> {
        crate::core::callback_state::check_not_in_callback()?;
        if self.player.epoch() != self.epoch {
            return Err(Error::ReplayNativeFailure);
        }
        self.player.ensure_native_healthy()?;
        if hit_index >= self.hit_count
            || self.index > i32::MAX as usize
            || hit_index > i32::MAX as usize
        {
            return Ok(None);
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
            return Err(Error::InvalidNativeReplayMetadata);
        }
        let geometry = match self.kind {
            ReplayQueryKind::CastRay
            | ReplayQueryKind::CastShape
            | ReplayQueryKind::CastRayClosest => {
                let point = Position::from_raw(raw.point);
                let normal = Vec2::from_raw(raw.normal);
                if !point.is_valid()
                    || !(0.0..=1.0).contains(&raw.fraction)
                    || !replay_query_hit_normal_is_valid(normal, raw.fraction)
                {
                    self.player.lifecycle.set(ReplayLifecycle::Terminal);
                    return Err(Error::InvalidNativeReplayMetadata);
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
        Ok(Some(ReplayQueryHitView {
            _player: self.player,
            epoch: self.epoch,
            geometry,
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ReplayQueryHitGeometry {
    point: Position,
    normal: Vec2,
    fraction: f32,
}

fn replay_query_hit_normal_is_valid(normal: Vec2, fraction: f32) -> bool {
    if !normal.is_valid() {
        return false;
    }
    let length_squared = normal.x * normal.x + normal.y * normal.y;
    let is_unit = length_squared.is_finite() && (1.0 - length_squared).abs() < 100.0 * f32::EPSILON;
    if fraction == 0.0 {
        normal == Vec2::ZERO || is_unit
    } else {
        is_unit
    }
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

fn checked_native_count(count: i32) -> Result<usize> {
    usize::try_from(count).map_err(|_| Error::InvalidNativeReplayMetadata)
}

fn read_replay_observation(
    player: NonNull<ffi::b2RecPlayer>,
    frame_count: u32,
) -> Result<ReplayObservation> {
    #[cfg(test)]
    REPLAY_OBSERVATION_READS.with(|reads| reads.set(reads.get().saturating_add(1)));

    let player = player.as_ptr();
    note_replay_native_call();
    let frame = u32::try_from(unsafe { ffi::b2RecPlayer_GetFrame(player) })
        .map_err(|_| Error::InvalidNativeReplayMetadata)?;
    if frame > frame_count {
        return Err(Error::InvalidNativeReplayMetadata);
    }

    note_replay_native_call();
    let first_divergence = if unsafe { ffi::b2RecPlayer_HasDiverged(player) } {
        note_replay_native_call();
        let first = u32::try_from(unsafe { ffi::b2RecPlayer_GetDivergeFrame(player) })
            .map_err(|_| Error::InvalidNativeReplayMetadata)?;
        if first > frame {
            return Err(Error::InvalidNativeReplayMetadata);
        }
        Some(first)
    } else {
        None
    };

    note_replay_native_call();
    let min_interval_frames =
        u32::try_from(unsafe { ffi::b2RecPlayer_GetKeyframeMinInterval(player) })
            .map_err(|_| Error::InvalidNativeReplayMetadata)?;
    note_replay_native_call();
    let effective_interval_frames =
        u32::try_from(unsafe { ffi::b2RecPlayer_GetKeyframeInterval(player) })
            .map_err(|_| Error::InvalidNativeReplayMetadata)?;
    if min_interval_frames == 0 || effective_interval_frames < min_interval_frames {
        return Err(Error::InvalidNativeReplayMetadata);
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
) -> Result<ReplayInfo> {
    note_replay_native_call();
    let raw = unsafe { ffi::b2RecPlayer_GetInfo(player.as_ptr()) };
    let frame_count =
        u32::try_from(raw.frameCount).map_err(|_| Error::InvalidNativeReplayMetadata)?;
    let native_worker_count = WorkerCount::from_native(raw.workerCount)
        .map_err(|_| Error::InvalidNativeReplayMetadata)?;
    let sub_step_count =
        u32::try_from(raw.subStepCount).map_err(|_| Error::InvalidNativeReplayMetadata)?;
    // SAFETY: replay frame decoding validates stored world bounds before publication.
    let bounds = Aabb::from_raw_unvalidated(raw.bounds);
    if native_worker_count != worker_count
        || !raw.timeStep.is_finite()
        || raw.timeStep < 0.0
        || !is_safe_length_units_per_meter(raw.lengthScale)
        || !bounds.is_valid()
    {
        return Err(Error::InvalidNativeReplayMetadata);
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
    use crate::{BodyType, DebugDrawOptions, QueryFilter, RecordingLimits, ShapeDef, shapes};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn replay_query_hit_normal_validation_matches_live_queries() {
        assert!(replay_query_hit_normal_is_valid(Vec2::ZERO, 0.0));
        assert!(replay_query_hit_normal_is_valid(Vec2::new(1.0, 0.0), 0.5));
        assert!(!replay_query_hit_normal_is_valid(Vec2::ZERO, 0.5));
        assert!(!replay_query_hit_normal_is_valid(Vec2::new(2.0, 0.0), 0.5));
        assert!(!replay_query_hit_normal_is_valid(
            Vec2::new(f32::NAN, 0.0),
            0.5,
        ));
    }

    struct FailpointReset;

    struct PanicOnDrop(Arc<AtomicUsize>);

    struct InvokeOnDrop<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("replay configuration closure drop panic");
            }
        }
    }

    impl Drop for FailpointReset {
        fn drop(&mut self) {
            REPLAY_CREATE_FAILPOINT.with(|current| current.set(None));
            REPLAY_FORCE_UNHEALTHY.with(|current| current.set(false));
            REPLAY_FAIL_HEALTH_AFTER_RESTART.with(|current| current.set(false));
            REPLAY_NATIVE_CALLS.with(|calls| calls.set(0));
        }
    }

    fn one_step_recording() -> Recording {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        drop(session.step(1.0 / 60.0, 1).unwrap());
        let recording = session.finish().unwrap();
        drop(world);
        recording
    }

    fn one_query_recording() -> Recording {
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
                    .body_builder()
                    .build()
                    .unwrap(),
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
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        drop(session.step(1.0 / 60.0, 1).unwrap());
        assert!(
            session
                .query()
                .unwrap()
                .cast_ray_closest(
                    Position::new(-2.0, 0.0),
                    Vec2::new(4.0, 0.0),
                    QueryFilter::default(),
                )
                .unwrap()
                .is_some()
        );
        let recording = session.finish().unwrap();
        drop(world);
        recording
    }

    fn dual_mixer_recording() -> Recording {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world
            .set_friction_callback(MixerId::from_bytes([0x61; 32]), |a, b| {
                a.coefficient.max(b.coefficient)
            })
            .unwrap();
        world
            .set_restitution_callback(MixerId::from_bytes([0x62; 32]), |a, b| {
                a.coefficient.max(b.coefficient)
            })
            .unwrap();
        let recording = world
            .start_recording(RecordingLimits::default())
            .unwrap()
            .finish()
            .unwrap();
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

    fn reset_restart_calls() {
        REPLAY_RESTART_CALLS.with(|calls| calls.set(0));
    }

    fn restart_calls() -> usize {
        REPLAY_RESTART_CALLS.with(Cell::get)
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
        let mut player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        let epoch = player.epoch();
        let observation = player.observation.get();
        let lifecycle = player.lifecycle.get();
        reset_native_calls();

        let callback_guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(player.step(), Err(Error::InCallback));
        assert_eq!(player.seek(0), Err(Error::InCallback));
        assert_eq!(player.seek(u64::MAX), Err(Error::InCallback));
        assert_eq!(player.restart(), Err(Error::InCallback));
        assert_eq!(player.set_keyframe_policy(policy), Err(Error::InCallback));

        assert_eq!(player.is_healthy(), Err(Error::InCallback));

        let visited = Cell::new(false);
        assert_eq!(
            player.with_view(|_| {
                visited.set(true);
                Ok(())
            }),
            Err(Error::InCallback)
        );
        assert!(!visited.get());
        assert_eq!(
            player.draw(&mut NoopDrawer, DebugDrawOptions::default(), None),
            Err(Error::InCallback)
        );
        assert_eq!(player.epoch(), epoch);
        assert_eq!(player.observation.get(), observation);
        assert_eq!(player.frame(), observation.frame);
        assert_eq!(player.lifecycle.get(), lifecycle);
        assert_eq!(native_calls(), 0);

        drop(callback_guard);
        assert_eq!(player.seek(u64::MAX), Err(Error::ReplayFrameOutOfRange));
        assert!(player.epoch() > epoch);
        assert_eq!(native_calls(), 0);
        drop(player);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejected_view_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REJECTED_REPLAY_VIEW";
        const TEST_NAME: &str =
            "replay::tests::rejected_view_cleanup_during_outer_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer rejected replay-view unwind remains primary";

        if std::env::var_os(CHILD).is_some() {
            let dropped = Arc::new(AtomicUsize::new(0));
            let rejected = std::rc::Rc::new(Cell::new(false));
            let rejected_from_drop = std::rc::Rc::clone(&rejected);
            let dropped_from_drop = Arc::clone(&dropped);
            let result = catch_unwind(AssertUnwindSafe(|| {
                let recording = one_step_recording();
                let player = ReplayPlayer::open(
                    crate::Foundation::initialize_default().unwrap(),
                    &recording,
                    ReplayConfig::default(),
                )
                .unwrap();
                let _failpoint = FailpointReset;
                REPLAY_FORCE_UNHEALTHY.with(|current| current.set(true));
                let _visit = InvokeOnDrop(Some(move || {
                    let marker = PanicOnDrop(dropped_from_drop);
                    rejected_from_drop.set(matches!(
                        player.with_view(move |_| {
                            let _ = &marker;
                            Ok(())
                        }),
                        Err(Error::ReplayNativeFailure)
                    ));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert!(rejected.get());
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
            eprintln!("boxdd-outer-unwind-rejected-replay-view: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind rejected replay-view child process must start");
        assert!(
            output.status.success(),
            "outer-unwind rejected replay-view child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-rejected-replay-view: completed"),
            "outer-unwind rejected replay-view child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn mixer_identity_mismatches_never_reach_native_creation() {
        let recording = dual_mixer_recording();
        let friction = MixerId::from_bytes([0x61; 32]);
        let restitution = MixerId::from_bytes([0x62; 32]);

        let mismatches = [
            ReplayConfig::default(),
            ReplayConfig::default()
                .with_friction_mixer(friction, |a, b| a.coefficient.max(b.coefficient)),
            ReplayConfig::default()
                .with_friction_mixer(restitution, |a, b| a.coefficient.max(b.coefficient))
                .with_restitution_mixer(friction, |a, b| a.coefficient.max(b.coefficient)),
            ReplayConfig::default()
                .with_friction_mixer(MixerId::from_bytes([0x63; 32]), |a, b| {
                    a.coefficient.max(b.coefficient)
                })
                .with_restitution_mixer(MixerId::from_bytes([0x64; 32]), |a, b| {
                    a.coefficient.max(b.coefficient)
                }),
        ];

        for config in mismatches {
            reset_native_calls();
            assert!(matches!(
                ReplayPlayer::open(
                    crate::Foundation::initialize_default().unwrap(),
                    &recording,
                    config
                ),
                Err(Error::ReplayMixerIdentityMismatch)
            ));
            assert_eq!(native_calls(), 0);
        }

        reset_native_calls();
        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default()
                .with_friction_mixer(friction, |a, b| a.coefficient.max(b.coefficient))
                .with_restitution_mixer(restitution, |a, b| a.coefficient.max(b.coefficient)),
        )
        .unwrap();
        assert!(native_calls() > 0);
        drop(player);
    }

    #[test]
    fn replay_views_reject_callback_before_native_activity_but_work_in_the_view_closure() {
        let recording = one_query_recording();
        let mut player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        player.step().unwrap();

        player
            .with_view(|view| {
                let body = view.body(0)?.expect("recorded body");
                let query = view.query(0)?.expect("recorded query");
                assert!(query.hit(0)?.is_some());
                reset_native_calls();

                let callback_guard = crate::core::callback_state::CallbackGuard::enter();
                assert!(matches!(view.body(0), Err(Error::InCallback)));
                assert!(matches!(view.query(0), Err(Error::InCallback)));
                assert_eq!(body.is_valid(), Err(Error::InCallback));
                assert_eq!(body.body_type(), Err(Error::InCallback));
                assert!(matches!(body.position(), Err(Error::InCallback)));
                assert!(matches!(body.transform(), Err(Error::InCallback)));
                assert!(matches!(body.linear_velocity(), Err(Error::InCallback)));
                assert!(matches!(body.angular_velocity(), Err(Error::InCallback)));
                assert!(matches!(query.hit(0), Err(Error::InCallback)));
                assert_eq!(native_calls(), 0);
                drop(callback_guard);

                assert!(body.is_valid()?);
                assert_eq!(body.body_type()?, BodyType::Static);
                assert!(view.body(0)?.is_some());
                assert!(view.query(0)?.is_some());
                assert!(query.hit(0)?.is_some());
                assert!(native_calls() > 0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn unknown_replay_body_type_is_precise_then_terminal_without_more_native_calls() {
        let recording = one_query_recording();
        let mut player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        player.step().unwrap();

        player
            .with_view(|view| {
                let body = view.body(0)?.expect("recorded body");
                let raw = ffi::b2BodyType_b2_bodyTypeCount;
                reset_native_calls();
                let get_type = ReplayBodyGetTypeOverride::install(raw);

                assert_eq!(body.body_type(), Err(Error::InvalidNativeBodyType { raw }));
                assert_eq!(get_type.calls(), 1);
                let native_after_unknown = native_calls();
                assert!(native_after_unknown > 0);
                assert_eq!(body.body_type(), Err(Error::ReplayNativeFailure));
                assert!(matches!(view.body(0), Err(Error::ReplayNativeFailure)));
                assert_eq!(get_type.calls(), 1);
                assert_eq!(native_calls(), native_after_unknown);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn replay_close_and_drop_defer_native_shutdown_until_the_owner_callback_returns() {
        let recording = one_step_recording();
        reset_destroy_calls();

        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        reset_native_calls();
        let owner =
            crate::core::callback_state::CallbackOwnerToken::world(player.resources.owner_token());
        crate::core::callback_state::run_test_owner_callback_boundary(
            owner,
            || {
                let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
                assert_eq!(player.close(), Err(Error::InCallback));
                assert_eq!(native_calls(), 0);
                assert_eq!(destroy_calls(), 0);
            },
            |native, _panic| native,
        );
        assert_eq!(destroy_calls(), 1);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);

        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        reset_native_calls();
        let owner =
            crate::core::callback_state::CallbackOwnerToken::world(player.resources.owner_token());
        crate::core::callback_state::run_test_owner_callback_boundary(
            owner,
            || {
                let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
                drop(player);
                assert_eq!(native_calls(), 0);
                assert_eq!(destroy_calls(), 1);
            },
            |native, _panic| native,
        );
        assert_eq!(destroy_calls(), 2);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }

    #[test]
    fn deferred_replay_drop_runs_all_mixer_cleanup_before_resuming() {
        let recording = dual_mixer_recording();
        let friction_dropped = Arc::new(AtomicUsize::new(0));
        let restitution_dropped = Arc::new(AtomicUsize::new(0));
        let friction_marker = PanicOnDrop(Arc::clone(&friction_dropped));
        let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
        let config = ReplayConfig::default()
            .with_friction_mixer(MixerId::from_bytes([0x61; 32]), move |a, b| {
                let _ = &friction_marker;
                a.coefficient.max(b.coefficient)
            })
            .with_restitution_mixer(MixerId::from_bytes([0x62; 32]), move |a, b| {
                let _ = &restitution_marker;
                a.coefficient.max(b.coefficient)
            });
        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            config,
        )
        .unwrap();
        let owner =
            crate::core::callback_state::CallbackOwnerToken::world(player.resources.owner_token());
        let panic = catch_unwind(AssertUnwindSafe(|| {
            crate::core::callback_state::run_test_owner_callback_boundary(
                owner,
                || {
                    let _callback = crate::core::callback_state::CallbackGuard::enter();
                    drop(player);
                    assert_eq!(friction_dropped.load(Ordering::SeqCst), 0);
                    assert_eq!(restitution_dropped.load(Ordering::SeqCst), 0);
                },
                |native, _panic| native,
            );
        }));
        assert!(panic.is_err());
        assert_eq!(friction_dropped.load(Ordering::SeqCst), 1);
        assert_eq!(restitution_dropped.load(Ordering::SeqCst), 1);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }

    #[test]
    fn replay_config_drops_every_user_callback_before_resuming_a_panic() {
        let friction_dropped = Arc::new(AtomicUsize::new(0));
        let restitution_dropped = Arc::new(AtomicUsize::new(0));
        let friction_marker = PanicOnDrop(Arc::clone(&friction_dropped));
        let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
        let config = ReplayConfig::default()
            .with_friction_mixer(MixerId::from_bytes([0x41; 32]), move |a, b| {
                let _ = &friction_marker;
                a.coefficient.max(b.coefficient)
            })
            .with_restitution_mixer(MixerId::from_bytes([0x42; 32]), move |a, b| {
                let _ = &restitution_marker;
                a.coefficient.max(b.coefficient)
            });

        let panic = catch_unwind(AssertUnwindSafe(|| drop(config)));

        assert!(panic.is_err());
        assert_eq!(friction_dropped.load(Ordering::SeqCst), 1);
        assert_eq!(restitution_dropped.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn replacing_a_mixer_cleans_up_every_panicking_callback() {
        let previous_dropped = Arc::new(AtomicUsize::new(0));
        let replacement_dropped = Arc::new(AtomicUsize::new(0));
        let restitution_dropped = Arc::new(AtomicUsize::new(0));
        let previous_marker = PanicOnDrop(Arc::clone(&previous_dropped));
        let replacement_marker = PanicOnDrop(Arc::clone(&replacement_dropped));
        let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
        let config = ReplayConfig::default()
            .with_friction_mixer(MixerId::from_bytes([0x43; 32]), move |a, b| {
                let _ = &previous_marker;
                a.coefficient.max(b.coefficient)
            })
            .with_restitution_mixer(MixerId::from_bytes([0x44; 32]), move |a, b| {
                let _ = &restitution_marker;
                a.coefficient.max(b.coefficient)
            });

        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = config.with_friction_mixer(MixerId::from_bytes([0x45; 32]), move |a, b| {
                let _ = &replacement_marker;
                a.coefficient.max(b.coefficient)
            });
        }));

        assert!(panic.is_err());
        assert_eq!(previous_dropped.load(Ordering::SeqCst), 1);
        assert_eq!(replacement_dropped.load(Ordering::SeqCst), 1);
        assert_eq!(restitution_dropped.load(Ordering::SeqCst), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn mixer_replacement_during_outer_unwind_commits_callbacks_and_identities() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REPLAY_MIXER_REPLACEMENT";
        const TEST_NAME: &str =
            "replay::tests::mixer_replacement_during_outer_unwind_commits_callbacks_and_identities";
        const PRIMARY_PANIC: &str = "outer replay mixer replacement unwind remains primary";

        if std::env::var_os(CHILD).is_some() {
            let old_friction_dropped = Arc::new(AtomicUsize::new(0));
            let old_restitution_dropped = Arc::new(AtomicUsize::new(0));
            let old_friction_marker = PanicOnDrop(Arc::clone(&old_friction_dropped));
            let old_restitution_marker = PanicOnDrop(Arc::clone(&old_restitution_dropped));
            let config = ReplayConfig::default()
                .with_friction_mixer(MixerId::from_bytes([0x51; 32]), move |a, b| {
                    let _ = &old_friction_marker;
                    a.coefficient.max(b.coefficient)
                })
                .with_restitution_mixer(MixerId::from_bytes([0x52; 32]), move |a, b| {
                    let _ = &old_restitution_marker;
                    a.coefficient.max(b.coefficient)
                });
            let new_friction = MixerId::from_bytes([0x61; 32]);
            let new_restitution = MixerId::from_bytes([0x62; 32]);
            let committed = std::rc::Rc::new(std::cell::RefCell::new(None));
            let committed_from_drop = std::rc::Rc::clone(&committed);

            let result = catch_unwind(AssertUnwindSafe(|| {
                let _replace = InvokeOnDrop(Some(move || {
                    let config = config
                        .with_friction_mixer(new_friction, |a, b| a.coefficient.max(b.coefficient))
                        .with_restitution_mixer(new_restitution, |a, b| {
                            a.coefficient.max(b.coefficient)
                        });
                    committed_from_drop.replace(Some(config));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));

            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert_eq!(old_friction_dropped.load(Ordering::SeqCst), 1);
            assert_eq!(old_restitution_dropped.load(Ordering::SeqCst), 1);
            let config = committed
                .borrow_mut()
                .take()
                .expect("the replacement must return a committed replay configuration");
            assert!(config.friction_mixer.is_some());
            assert!(config.restitution_mixer.is_some());
            let identities = config.mixer_identities();
            assert_eq!(identities.friction(), Some(new_friction));
            assert_eq!(identities.restitution(), Some(new_restitution));
            let recording = dual_mixer_recording();
            let player = ReplayPlayer::open(
                crate::Foundation::initialize_default().unwrap(),
                &recording,
                config,
            )
            .expect("the committed mixer cohort must be usable by replay");
            drop(player);
            eprintln!("boxdd-outer-unwind-replay-mixer-replacement: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind replay mixer replacement child process must start");
        assert!(
            output.status.success(),
            "outer-unwind replay mixer replacement child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-replay-mixer-replacement: completed"),
            "outer-unwind replay mixer replacement child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn null_native_creation_releases_exclusivity_without_destroying_a_player() {
        let recording = one_step_recording();
        reset_destroy_calls();
        let failpoint = set_create_failpoint(ReplayCreateFailpoint::Null);

        let error = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap_err();
        drop(failpoint);

        assert_eq!(error, Error::ReplayNativeCreateFailed);
        assert_eq!(destroy_calls(), 0);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
        drop(
            crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap(),
        );
    }

    #[test]
    fn creation_panic_releases_exclusivity_during_unwind() {
        let recording = one_step_recording();
        reset_destroy_calls();
        let failpoint = set_create_failpoint(ReplayCreateFailpoint::Panic);

        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = ReplayPlayer::open(
                crate::Foundation::initialize_default().unwrap(),
                &recording,
                ReplayConfig::default(),
            );
        }));
        drop(failpoint);

        assert!(panic.is_err());
        assert_eq!(destroy_calls(), 0);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
        drop(
            crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap(),
        );
    }

    #[test]
    fn drop_and_explicit_close_each_destroy_the_native_player_once() {
        let recording = one_step_recording();
        reset_destroy_calls();

        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        assert!(crate::Foundation::get().unwrap().activity().replay_active);
        drop(player);
        assert_eq!(destroy_calls(), 1);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);

        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        player.close().unwrap();
        assert_eq!(destroy_calls(), 2);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }

    #[test]
    fn failed_length_scale_verification_is_consumed_exactly_once() {
        let observed = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
        let expected = observed ^ 1;
        let mut resources = ReplayResources {
            player: None,
            owner_token: None,
            mixers: None,
            identities: None,
            worker_callbacks: None,
            lease: None,
            previous_length_scale_bits: expected,
            native_attempted: true,
        };

        assert_eq!(
            resources.shutdown(),
            Err(Error::ReplayLengthScaleNotRestored { expected, observed })
        );
        assert!(!resources.native_attempted);
        assert_eq!(resources.shutdown(), Ok(()));
    }

    #[test]
    fn replay_resource_drop_reports_length_scale_restoration_failure() {
        let observed = unsafe { ffi::b2GetLengthUnitsPerMeter() }.to_bits();
        let expected = observed ^ 1;
        let resources = ReplayResources {
            player: None,
            owner_token: None,
            mixers: None,
            identities: None,
            worker_callbacks: None,
            lease: None,
            previous_length_scale_bits: expected,
            native_attempted: true,
        };

        let panic = catch_unwind(AssertUnwindSafe(|| drop(resources)));
        assert!(panic.is_err());
    }

    #[test]
    fn unhealthy_native_state_is_terminal_after_advancing_the_epoch() {
        let recording = one_step_recording();
        let mut player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        let epoch = player.epoch();
        let cached_frame = player.frame();
        let cached_end = player.is_at_end();
        let cached_divergence = player.has_diverged();
        let cached_keyframes = player.keyframe_policy();
        let failpoint = FailpointReset;
        REPLAY_FORCE_UNHEALTHY.with(|current| current.set(true));

        assert_eq!(player.step(), Err(Error::ReplayNativeFailure));
        assert!(player.epoch() > epoch);
        assert!(!player.is_healthy().unwrap());

        drop(failpoint);
        let reads = REPLAY_OBSERVATION_READS.with(Cell::get);
        assert_eq!(player.frame(), cached_frame);
        assert_eq!(player.is_at_end(), cached_end);
        assert_eq!(player.has_diverged(), cached_divergence);
        assert_eq!(player.keyframe_policy(), cached_keyframes);
        assert_eq!(REPLAY_OBSERVATION_READS.with(Cell::get), reads);

        let terminal_epoch = player.epoch();
        assert_eq!(player.step(), Err(Error::ReplayNativeFailure));
        assert!(player.epoch() > terminal_epoch);
        assert_eq!(REPLAY_OBSERVATION_READS.with(Cell::get), reads);

        drop(player);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }

    #[test]
    fn restart_post_native_health_failure_terminalizes_after_owner_boundary_cleanup() {
        let recording = one_step_recording();
        let mut player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        player.step().unwrap();
        let epoch = player.epoch();
        let cached_frame = player.frame();
        let cached_end = player.is_at_end();
        let cached_divergence = player.has_diverged();
        let cached_keyframes = player.keyframe_policy();
        let observation_reads = REPLAY_OBSERVATION_READS.with(Cell::get);
        let failpoint = FailpointReset;
        REPLAY_FAIL_HEALTH_AFTER_RESTART.with(|armed| armed.set(true));
        reset_native_calls();
        reset_restart_calls();

        assert_eq!(player.restart(), Err(Error::ReplayNativeFailure));
        assert!(player.epoch() > epoch);
        assert_eq!(restart_calls(), 1);
        assert!(native_calls() >= 2);
        assert!(!crate::core::callback_state::in_callback());
        assert_eq!(crate::core::callback_state::owner_frame_count_for_test(), 0);
        assert_eq!(player.frame(), cached_frame);
        assert_eq!(player.is_at_end(), cached_end);
        assert_eq!(player.has_diverged(), cached_divergence);
        assert_eq!(player.keyframe_policy(), cached_keyframes);
        assert_eq!(REPLAY_OBSERVATION_READS.with(Cell::get), observation_reads);
        assert!(!player.is_healthy().unwrap());

        let terminal_epoch = player.epoch();
        assert_eq!(player.restart(), Err(Error::ReplayNativeFailure));
        assert!(player.epoch() > terminal_epoch);
        assert_eq!(restart_calls(), 1);
        assert_eq!(crate::core::callback_state::owner_frame_count_for_test(), 0);

        drop(failpoint);

        drop(player);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }

    #[test]
    fn epoch_exhaustion_terminalizes_before_native_mutation() {
        let recording = one_step_recording();
        let mut player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        player.epoch.set(ReplayEpoch(u64::MAX));
        let frame = player.frame();

        assert_eq!(player.step(), Err(Error::ReplayEpochExhausted));
        assert_eq!(player.frame(), frame);
        assert!(!player.is_healthy().unwrap());

        drop(player);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }

    #[test]
    fn foreign_world_object_metadata_terminalizes_the_player() {
        let recording = one_step_recording();
        let player = ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default(),
        )
        .unwrap();
        let foreign = ffi::b2BodyId {
            index1: 1,
            world0: player.world0 ^ 1,
            generation: 1,
        };

        assert!(!player.live_body_is_owned(foreign));
        assert_eq!(player.lifecycle.get(), ReplayLifecycle::Terminal);

        drop(player);
        assert!(!crate::Foundation::get().unwrap().activity().replay_active);
    }
}
