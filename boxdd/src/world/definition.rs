use super::*;

/// Error type for world creation and operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid world definition: {0}")]
    InvalidDefinition(#[from] crate::error::ApiError),

    #[error("failed to create Box2D world")]
    CreateFailed,
}

#[inline]
fn world_def_cookie_is_valid(def: &WorldDef) -> bool {
    def.0.internalValue == unsafe { ffi::b2DefaultWorldDef() }.internalValue
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
fn check_world_worker_count_valid(worker_count: i32) -> crate::error::ApiResult<()> {
    if worker_count >= 0 {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn check_world_def_valid(def: &WorldDef) -> crate::error::ApiResult<()> {
    if !world_def_cookie_is_valid(def) {
        return Err(crate::error::ApiError::InvalidArgument);
    }
    check_world_gravity_valid(def.gravity())?;
    check_non_negative_finite_world_scalar(def.restitution_threshold())?;
    check_non_negative_finite_world_scalar(def.hit_event_threshold())?;
    check_non_negative_finite_world_scalar(def.contact_hertz())?;
    check_non_negative_finite_world_scalar(def.contact_damping_ratio())?;
    check_non_negative_finite_world_scalar(def.contact_speed())?;
    check_positive_finite_world_scalar(def.maximum_linear_speed())?;
    check_world_worker_count_valid(def.worker_count())
}

/// World definition builder for constructing a simulation world.
#[doc(alias = "world_def")]
#[doc(alias = "worlddef")]
#[derive(Clone, Debug)]
pub struct WorldDef(pub(crate) ffi::b2WorldDef);

impl Default for WorldDef {
    fn default() -> Self {
        // SAFETY: FFI call to obtain a plain value struct
        let def = unsafe { ffi::b2DefaultWorldDef() };
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
    /// Any raw callback pointers stored in `raw` (`frictionCallback`, `restitutionCallback`,
    /// `enqueueTask`, and `finishTask`) must remain valid whenever the resulting `WorldDef` is
    /// later used to create or step a world. This constructor does not validate callback
    /// pointers, task contexts, or other raw pointer fields.
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

    pub fn worker_count(&self) -> i32 {
        self.0.workerCount
    }

    /// Returns whether raw task-system callbacks are installed on this definition.
    pub fn has_task_system_raw(&self) -> bool {
        self.0.enqueueTask.is_some() || self.0.finishTask.is_some()
    }

    /// Install raw Box2D task-system callbacks on this definition.
    ///
    /// # Safety
    /// `enqueue_task`, `finish_task`, and `user_task_context` must satisfy Box2D's task-system
    /// contract and remain valid for the lifetime of any world created from this definition.
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
        self.0.workerCount = 0;
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
            worker_count: i32,
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
            worker_count: self.0.workerCount,
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
            worker_count: Option<i32>,
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
            b.0.workerCount = v;
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

    /// Number of worker threads Box2D may use during stepping when a task system is installed.
    ///
    /// This does not make `World` or owned handles `Send` / `Sync`. Non-zero values only become
    /// active when advanced users also supply raw task callbacks through
    /// `unsafe WorldBuilder::task_system_raw(...)`, `WorldDef::set_task_system_raw(...)`, or an
    /// explicit raw `WorldDef` conversion path.
    pub fn worker_count(mut self, n: i32) -> Self {
        self.def.0.workerCount = n;
        self
    }

    /// Install raw Box2D task-system callbacks on the builder.
    ///
    /// # Safety
    /// `enqueue_task`, `finish_task`, and `user_task_context` must satisfy Box2D's task-system
    /// contract and remain valid for the lifetime of any world created from the resulting
    /// definition.
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
