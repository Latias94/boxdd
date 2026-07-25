use super::*;

/// Error type for world creation and operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid world definition: {0}")]
    InvalidDefinition(#[from] crate::error::ApiError),

    #[error("failed to create Box2D world")]
    CreateFailed,

    #[error("the process exhausted its Rust world identity space")]
    IdentityExhausted,

    #[error(transparent)]
    FoundationActivity(#[from] crate::FoundationActivityError),
}

#[inline]
fn world_def_cookie_is_valid(def: &WorldDef) -> crate::error::ApiResult<bool> {
    let _lease = crate::core::foundation::transient_native_lease()?;
    Ok(def.0.internalValue == unsafe { ffi::b2DefaultWorldDef() }.internalValue)
}

#[inline]
pub(crate) fn assert_world_gravity_valid(gravity: Vec2) {
    assert!(
        gravity.is_valid(),
        "gravity must be a valid Box2D vector, got {:?}",
        gravity
    );
}

#[inline]
pub(crate) fn check_world_gravity_valid(gravity: Vec2) -> crate::error::ApiResult<()> {
    if gravity.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn assert_non_negative_finite_world_scalar(name: &str, value: f32) {
    assert!(
        crate::is_valid_float(value) && value >= 0.0,
        "{name} must be finite and >= 0.0, got {value}"
    );
}

#[inline]
pub(crate) fn check_non_negative_finite_world_scalar(value: f32) -> crate::error::ApiResult<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn assert_positive_finite_world_scalar(name: &str, value: f32) {
    assert!(
        crate::is_valid_float(value) && value > 0.0,
        "{name} must be finite and > 0.0, got {value}"
    );
}

#[inline]
pub(crate) fn check_positive_finite_world_scalar(value: f32) -> crate::error::ApiResult<()> {
    if crate::is_valid_float(value) && value > 0.0 {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn check_world_task_system_valid(def: &WorldDef) -> crate::error::ApiResult<()> {
    #[cfg(target_arch = "wasm32")]
    if def.0.enqueueTask.is_some()
        || def.0.finishTask.is_some()
        || def.0.frictionCallback.is_some()
        || def.0.restitutionCallback.is_some()
    {
        return Err(crate::error::ApiError::InvalidArgument);
    }
    if def.0.enqueueTask.is_some() == def.0.finishTask.is_some() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn check_world_def_valid(def: &WorldDef) -> crate::error::ApiResult<()> {
    check_world_gravity_valid(def.gravity())?;
    check_non_negative_finite_world_scalar(def.restitution_threshold())?;
    check_non_negative_finite_world_scalar(def.hit_event_threshold())?;
    check_non_negative_finite_world_scalar(def.contact_hertz())?;
    check_non_negative_finite_world_scalar(def.contact_damping_ratio())?;
    check_non_negative_finite_world_scalar(def.contact_speed())?;
    check_positive_finite_world_scalar(def.maximum_linear_speed())?;
    WorkerCount::try_from(def.0.workerCount)?;
    WorldCapacity::try_from_raw(def.0.capacity)?;
    check_world_task_system_valid(def)?;
    if world_def_cookie_is_valid(def)? {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

/// World definition builder for constructing a simulation world.
#[doc(alias = "world_def")]
#[doc(alias = "worlddef")]
#[derive(Clone, Debug)]
pub struct WorldDef(pub(crate) ffi::b2WorldDef);

impl Default for WorldDef {
    fn default() -> Self {
        let _lease = crate::core::foundation::assert_transient_native_lease();
        // SAFETY: FFI call to obtain a plain value struct
        let mut def = unsafe { ffi::b2DefaultWorldDef() };
        // Upstream encodes its serial default as zero. Keep every Safe Rust
        // definition inside the runtime setter's explicit `[1, MAX]` domain.
        def.workerCount = WorkerCount::default().as_i32();
        Self(def)
    }
}

impl WorldDef {
    pub fn builder() -> WorldBuilder {
        WorldBuilder::from(Self::default())
    }

    /// Construct from the raw Box2D world definition value.
    ///
    /// # Safety
    /// `raw` must have been initialized by `b2DefaultWorldDef` from the same Box2D ABI as this
    /// crate. Its internal cookie, worker count, capacity values, and every pointer field must
    /// remain valid for all native operations that can observe them. `World::new` rejects worker
    /// counts outside `[1, B2_MAX_WORKERS]`, unsupported target concurrency, negative capacities,
    /// and a task callback pair where only one callback is present before calling Box2D.
    ///
    /// In particular, `frictionCallback` and `restitutionCallback` can be invoked concurrently on
    /// Box2D worker threads for the full lifetime of each world created from this definition. They
    /// must not unwind across the C ABI, call or mutate Box2D, re-enter the world, or access shared
    /// application state without synchronization. They must return a finite, non-negative mixing
    /// coefficient. Any code or global state they access must remain valid until the world has
    /// finished stepping and has been destroyed.
    ///
    /// `enqueueTask`, `finishTask`, and `userTaskContext` must satisfy the complete task-system
    /// contract documented by [`WorldDef::set_task_system_raw`]. `userData` and any other raw
    /// pointer must likewise obey its native ownership, aliasing, synchronization, and lifetime
    /// requirements. This constructor cannot validate any of these obligations. On `wasm32`,
    /// validation rejects every raw callback pointer because no shared provider function table is
    /// qualified.
    pub unsafe fn from_raw(raw: ffi::b2WorldDef) -> Self {
        Self(raw)
    }

    pub fn gravity(&self) -> crate::types::Vec2 {
        crate::types::Vec2::from_raw(self.0.gravity)
    }

    pub fn restitution_threshold(&self) -> f32 {
        self.0.restitutionThreshold
    }

    pub fn hit_event_threshold(&self) -> f32 {
        self.0.hitEventThreshold
    }

    pub fn contact_hertz(&self) -> f32 {
        self.0.contactHertz
    }

    pub fn contact_damping_ratio(&self) -> f32 {
        self.0.contactDampingRatio
    }

    pub fn contact_speed(&self) -> f32 {
        self.0.contactSpeed
    }

    pub fn maximum_linear_speed(&self) -> f32 {
        self.0.maximumLinearSpeed
    }

    pub fn is_sleep_enabled(&self) -> bool {
        self.0.enableSleep
    }

    pub fn is_continuous_enabled(&self) -> bool {
        self.0.enableContinuous
    }

    pub fn is_contact_softening_enabled(&self) -> bool {
        self.0.enableContactSoftening
    }

    pub fn worker_count(&self) -> WorkerCount {
        WorkerCount::try_from(self.0.workerCount)
            .expect("WorldDef contains an invalid raw worker count")
    }

    pub fn capacity(&self) -> WorldCapacity {
        WorldCapacity::try_from_raw(self.0.capacity)
            .expect("WorldDef contains an invalid raw capacity")
    }

    /// Returns whether raw task-system callbacks are installed on this definition.
    pub fn has_task_system_raw(&self) -> bool {
        self.0.enqueueTask.is_some() || self.0.finishTask.is_some()
    }

    /// Install raw Box2D task-system callbacks on this definition.
    ///
    /// # Safety
    /// The callback function pointers and `user_task_context` must remain valid and callable until
    /// every world created from this definition has finished all pending tasks and is destroyed.
    /// Any state reachable through the context must support all concurrent access performed by the
    /// configured worker count.
    ///
    /// Both callbacks must be non-null and `worker_count` must be in Box2D's supported range for
    /// Box2D to select the external task system. `World::new` rejects zero, negative, excessive,
    /// target-unsupported, and half-configured task-system definitions before native creation.
    ///
    /// For each non-null Box2D task and task-context pair passed to `enqueue_task`, the task system
    /// must invoke `task(task_context)` exactly once with the unchanged pointer. Returning null from
    /// `enqueue_task` declares that invocation complete synchronously; Box2D will not call
    /// `finish_task` for it. Returning a non-null task handle declares that the handle remains
    /// valid until Box2D passes it to `finish_task`; `finish_task` must wait for the task's single
    /// invocation to complete before it returns and may then release the handle. The callbacks
    /// must not lose, duplicate, detach beyond that finish boundary, or prematurely free a task.
    ///
    /// Neither callback, nor Rust code used to execute the supplied Box2D task, may unwind across
    /// its C ABI boundary. Worker-side code must not call, mutate, or re-enter any Box2D world, and
    /// application code must not concurrently mutate the world while tasks are outstanding.
    /// `worker_count` and the callback pair must also form a configuration accepted by the linked
    /// Box2D version.
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn set_task_system_raw(
        &mut self,
        worker_count: i32,
        enqueue_task: ffi::b2EnqueueTaskCallback,
        finish_task: ffi::b2FinishTaskCallback,
        user_task_context: *mut core::ffi::c_void,
    ) {
        self.0.workerCount = worker_count;
        self.0.enqueueTask = enqueue_task;
        self.0.finishTask = finish_task;
        self.0.userTaskContext = user_task_context;
    }

    /// Remove any raw Box2D task-system callbacks from this definition.
    pub fn clear_task_system_raw(&mut self) {
        self.0.workerCount = WorkerCount::default().as_i32();
        self.0.enqueueTask = None;
        self.0.finishTask = None;
        self.0.userTaskContext = core::ptr::null_mut();
    }

    pub fn into_raw(self) -> ffi::b2WorldDef {
        self.0
    }

    pub fn validate(&self) -> crate::error::ApiResult<()> {
        check_world_def_valid(self)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WorldDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            gravity: crate::types::Vec2,
            restitution_threshold: f32,
            hit_event_threshold: f32,
            contact_hertz: f32,
            contact_damping_ratio: f32,
            contact_speed: f32,
            maximum_linear_speed: f32,
            enable_sleep: bool,
            enable_continuous: bool,
            enable_contact_softening: bool,
            worker_count: WorkerCount,
            capacity: WorldCapacity,
        }
        let r = Repr {
            gravity: crate::types::Vec2::from_raw(self.0.gravity),
            restitution_threshold: self.0.restitutionThreshold,
            hit_event_threshold: self.0.hitEventThreshold,
            contact_hertz: self.0.contactHertz,
            contact_damping_ratio: self.0.contactDampingRatio,
            contact_speed: self.0.contactSpeed,
            maximum_linear_speed: self.0.maximumLinearSpeed,
            enable_sleep: self.0.enableSleep,
            enable_continuous: self.0.enableContinuous,
            enable_contact_softening: self.0.enableContactSoftening,
            worker_count: self.worker_count(),
            capacity: self.capacity(),
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for WorldDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            gravity: Option<crate::types::Vec2>,
            #[serde(default)]
            restitution_threshold: Option<f32>,
            #[serde(default)]
            hit_event_threshold: Option<f32>,
            #[serde(default)]
            contact_hertz: Option<f32>,
            #[serde(default)]
            contact_damping_ratio: Option<f32>,
            #[serde(default)]
            contact_speed: Option<f32>,
            #[serde(default)]
            maximum_linear_speed: Option<f32>,
            #[serde(default)]
            enable_sleep: Option<bool>,
            #[serde(default)]
            enable_continuous: Option<bool>,
            #[serde(default)]
            enable_contact_softening: Option<bool>,
            #[serde(default)]
            worker_count: Option<WorkerCount>,
            #[serde(default)]
            capacity: Option<WorldCapacity>,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut b = WorldDef::default();
        if let Some(g) = r.gravity {
            b.0.gravity = g.into_raw();
        }
        if let Some(v) = r.restitution_threshold {
            b.0.restitutionThreshold = v;
        }
        if let Some(v) = r.hit_event_threshold {
            b.0.hitEventThreshold = v;
        }
        if let Some(v) = r.contact_hertz {
            b.0.contactHertz = v;
        }
        if let Some(v) = r.contact_damping_ratio {
            b.0.contactDampingRatio = v;
        }
        if let Some(v) = r.contact_speed {
            b.0.contactSpeed = v;
        }
        if let Some(v) = r.maximum_linear_speed {
            b.0.maximumLinearSpeed = v;
        }
        if let Some(v) = r.enable_sleep {
            b.0.enableSleep = v;
        }
        if let Some(v) = r.enable_continuous {
            b.0.enableContinuous = v;
        }
        if let Some(v) = r.enable_contact_softening {
            b.0.enableContactSoftening = v;
        }
        if let Some(v) = r.worker_count {
            b.0.workerCount = v.as_i32();
        }
        if let Some(v) = r.capacity {
            b.0.capacity = v.into_raw();
        }
        Ok(b)
    }
}

/// Fluent builder for `WorldDef`.
///
/// Chain configuration calls and finish with `build()`. All fields map 1:1 to
/// the upstream `b2WorldDef`.
#[doc(alias = "world_builder")]
#[doc(alias = "worldbuilder")]
#[derive(Clone, Debug)]
pub struct WorldBuilder {
    def: WorldDef,
}

impl From<WorldDef> for WorldBuilder {
    fn from(def: WorldDef) -> Self {
        Self { def }
    }
}

impl WorldBuilder {
    /// Set gravity vector in meters per second squared.
    pub fn gravity<V: Into<Vec2>>(mut self, g: V) -> Self {
        self.def.0.gravity = g.into().into_raw();
        self
    }

    /// Restitution threshold (m/s) under which collisions don't bounce.
    pub fn restitution_threshold(mut self, v: f32) -> Self {
        self.def.0.restitutionThreshold = v;
        self
    }

    /// Impulse magnitude that generates hit events.
    pub fn hit_event_threshold(mut self, v: f32) -> Self {
        self.def.0.hitEventThreshold = v;
        self
    }

    /// Contact solver target stiffness in Hertz.
    pub fn contact_hertz(mut self, v: f32) -> Self {
        self.def.0.contactHertz = v;
        self
    }

    /// Contact damping ratio (non-dimensional).
    pub fn contact_damping_ratio(mut self, v: f32) -> Self {
        self.def.0.contactDampingRatio = v;
        self
    }

    /// Velocity used by continuous collision detection.
    pub fn contact_speed(mut self, v: f32) -> Self {
        self.def.0.contactSpeed = v;
        self
    }

    /// Maximum linear speed clamp for bodies.
    pub fn maximum_linear_speed(mut self, v: f32) -> Self {
        self.def.0.maximumLinearSpeed = v;
        self
    }

    /// Enable/disable sleeping globally.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.0.enableSleep = flag;
        self
    }

    /// Enable/disable continuous collision detection globally.
    pub fn enable_continuous(mut self, flag: bool) -> Self {
        self.def.0.enableContinuous = flag;
        self
    }

    /// Enable/disable contact softening.
    pub fn enable_contact_softening(mut self, flag: bool) -> Self {
        self.def.0.enableContactSoftening = flag;
        self
    }

    /// Number of worker threads Box2D may use during stepping.
    ///
    /// Values above one select Box2D's built-in scheduler unless advanced users replace it through
    /// `unsafe WorldBuilder::task_system_raw(...)`, `WorldDef::set_task_system_raw(...)`, or an
    /// explicit raw `WorldDef` conversion path. The validated value rejects
    /// unsupported targets and counts outside Box2D's native range. This does
    /// not make `World` or owned handles `Send` / `Sync`.
    pub fn worker_count(mut self, count: WorkerCount) -> Self {
        self.def.0.workerCount = count.as_i32();
        self
    }

    /// Reserve initial world storage to avoid predictable run-time allocations.
    pub fn capacity(mut self, capacity: WorldCapacity) -> Self {
        self.def.0.capacity = capacity.into_raw();
        self
    }

    /// Install raw Box2D task-system callbacks on the builder.
    ///
    /// # Safety
    /// All exactly-once execution, finish synchronization, no-unwind, non-reentrancy, concurrency,
    /// and lifetime requirements documented by [`WorldDef::set_task_system_raw`] apply. In
    /// particular, the callback pointers and `user_task_context` must outlive every world created
    /// from the resulting definition and all tasks submitted by those worlds.
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn task_system_raw(
        mut self,
        worker_count: i32,
        enqueue_task: ffi::b2EnqueueTaskCallback,
        finish_task: ffi::b2FinishTaskCallback,
        user_task_context: *mut core::ffi::c_void,
    ) -> Self {
        unsafe {
            self.def.set_task_system_raw(
                worker_count,
                enqueue_task,
                finish_task,
                user_task_context,
            );
        }
        self
    }

    /// Remove any raw Box2D task-system callbacks from the builder.
    pub fn clear_task_system_raw(mut self) -> Self {
        self.def.clear_task_system_raw();
        self
    }

    #[must_use]
    pub fn build(self) -> WorldDef {
        self.def
    }
}
