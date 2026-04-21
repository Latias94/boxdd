use crate::Transform;
use crate::body::{Body, BodyDef, BodyType};
use crate::collision::CastOutput;
use crate::core::world_core::{CustomFilterCtx, MaterialMixCtx, PreSolveCtx, WorldCore};
use crate::query::Aabb;
use crate::shapes::{ShapeDef, SurfaceMaterial};
use crate::types::{BodyId, ChainId, JointId, MassData, MotionLocks, ShapeId, Vec2};
use boxdd_sys::ffi;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Arc;

type ShapeFilterFn = fn(crate::types::ShapeId, crate::types::ShapeId) -> bool;
type PreSolveFn = fn(
    crate::types::ShapeId,
    crate::types::ShapeId,
    crate::types::Vec2,
    crate::types::Vec2,
) -> bool;

/// Input passed to world-level friction and restitution mixing callbacks.
///
/// `coefficient` is the shape's friction or restitution coefficient, depending on the callback.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MaterialMixInput {
    pub coefficient: f32,
    pub user_material_id: u64,
}

impl MaterialMixInput {
    #[inline]
    pub const fn new(coefficient: f32, user_material_id: u64) -> Self {
        Self {
            coefficient,
            user_material_id,
        }
    }
}

macro_rules! impl_world_shape_create_methods {
    ($(
        $create:ident,
        $owned:ident,
        $arg:ident,
        $geom_ty:path,
        $ffi_create:path;
    )+) => {
        $(
            pub fn $create(
                &mut self,
                body: BodyId,
                def: &ShapeDef,
                $arg: &$geom_ty,
            ) -> ShapeId {
                crate::core::debug_checks::assert_body_valid(body);
                let raw = $arg.into_raw();
                let sid = unsafe { $ffi_create(body, &def.0, &raw) };
                #[cfg(feature = "serialize")]
                self.record_shape_flags(sid, &def.0);
                sid
            }

            pub fn $owned(
                &mut self,
                body: BodyId,
                def: &ShapeDef,
                $arg: &$geom_ty,
            ) -> crate::shapes::OwnedShape {
                let sid = self.$create(body, def, $arg);
                crate::shapes::OwnedShape::new(self.core_arc(), sid)
            }
        )+
    };
}

macro_rules! impl_world_shape_set_methods {
    ($(
        $set:ident,
        $try_set:ident,
        $arg:ident,
        $geom_ty:path,
        $ffi_set:path;
    )+) => {
        $(
            pub fn $set(&mut self, shape: ShapeId, $arg: &$geom_ty) {
                crate::core::debug_checks::assert_shape_valid(shape);
                let raw = $arg.into_raw();
                unsafe { $ffi_set(shape, &raw) }
            }

            pub fn $try_set(
                &mut self,
                shape: ShapeId,
                $arg: &$geom_ty,
            ) -> crate::error::ApiResult<()> {
                crate::core::debug_checks::check_shape_valid(shape)?;
                let raw = $arg.into_raw();
                unsafe { $ffi_set(shape, &raw) }
                Ok(())
            }
        )+
    };
}

macro_rules! impl_world_gravity_methods {
    () => {
        /// Get current gravity vector.
        pub fn gravity(&self) -> Vec2 {
            crate::core::callback_state::assert_not_in_callback();
            world_gravity_impl(self.raw())
        }

        pub fn try_gravity(&self) -> crate::error::ApiResult<Vec2> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_gravity_impl(self.raw()))
        }
    };
}

macro_rules! impl_world_runtime_snapshot_methods {
    () => {
        /// World counters snapshot (sizes, tree heights, etc.).
        pub fn counters(&self) -> Counters {
            crate::core::callback_state::assert_not_in_callback();
            world_counters_impl(self.raw())
        }

        pub fn try_counters(&self) -> crate::error::ApiResult<Counters> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_counters_impl(self.raw()))
        }

        /// World profile snapshot with per-stage timing in milliseconds from the last completed step.
        pub fn profile(&self) -> Profile {
            crate::core::callback_state::assert_not_in_callback();
            world_profile_impl(self.raw())
        }

        pub fn try_profile(&self) -> crate::error::ApiResult<Profile> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_profile_impl(self.raw()))
        }

        /// Get number of awake bodies.
        pub fn awake_body_count(&self) -> i32 {
            crate::core::callback_state::assert_not_in_callback();
            world_awake_body_count_impl(self.raw())
        }

        pub fn try_awake_body_count(&self) -> crate::error::ApiResult<i32> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_awake_body_count_impl(self.raw()))
        }
    };
}

macro_rules! impl_world_runtime_tuning_getter_methods {
    () => {
        pub fn is_sleeping_enabled(&self) -> bool {
            crate::core::callback_state::assert_not_in_callback();
            world_is_sleeping_enabled_impl(self.raw())
        }

        pub fn try_is_sleeping_enabled(&self) -> crate::error::ApiResult<bool> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_is_sleeping_enabled_impl(self.raw()))
        }

        pub fn is_continuous_enabled(&self) -> bool {
            crate::core::callback_state::assert_not_in_callback();
            world_is_continuous_enabled_impl(self.raw())
        }

        pub fn try_is_continuous_enabled(&self) -> crate::error::ApiResult<bool> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_is_continuous_enabled_impl(self.raw()))
        }

        /// Returns true if constraint warm starting is enabled.
        pub fn is_warm_starting_enabled(&self) -> bool {
            crate::core::callback_state::assert_not_in_callback();
            world_is_warm_starting_enabled_impl(self.raw())
        }

        pub fn try_is_warm_starting_enabled(&self) -> crate::error::ApiResult<bool> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_is_warm_starting_enabled_impl(self.raw()))
        }

        pub fn restitution_threshold(&self) -> f32 {
            crate::core::callback_state::assert_not_in_callback();
            world_restitution_threshold_impl(self.raw())
        }

        pub fn try_restitution_threshold(&self) -> crate::error::ApiResult<f32> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_restitution_threshold_impl(self.raw()))
        }

        pub fn hit_event_threshold(&self) -> f32 {
            crate::core::callback_state::assert_not_in_callback();
            world_hit_event_threshold_impl(self.raw())
        }

        pub fn try_hit_event_threshold(&self) -> crate::error::ApiResult<f32> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_hit_event_threshold_impl(self.raw()))
        }

        pub fn maximum_linear_speed(&self) -> f32 {
            crate::core::callback_state::assert_not_in_callback();
            world_maximum_linear_speed_impl(self.raw())
        }

        pub fn try_maximum_linear_speed(&self) -> crate::error::ApiResult<f32> {
            crate::core::callback_state::check_not_in_callback()?;
            Ok(world_maximum_linear_speed_impl(self.raw()))
        }
    };
}

#[inline]
fn world_gravity_impl(world: ffi::b2WorldId) -> Vec2 {
    Vec2::from(unsafe { ffi::b2World_GetGravity(world) })
}

#[inline]
fn world_counters_impl(world: ffi::b2WorldId) -> Counters {
    Counters::from_raw(unsafe { ffi::b2World_GetCounters(world) })
}

#[inline]
fn world_profile_impl(world: ffi::b2WorldId) -> Profile {
    Profile::from_raw(unsafe { ffi::b2World_GetProfile(world) })
}

#[inline]
fn world_awake_body_count_impl(world: ffi::b2WorldId) -> i32 {
    unsafe { ffi::b2World_GetAwakeBodyCount(world) }
}

#[inline]
fn world_is_sleeping_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsSleepingEnabled(world) }
}

#[inline]
fn world_is_continuous_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsContinuousEnabled(world) }
}

#[inline]
fn world_is_warm_starting_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsWarmStartingEnabled(world) }
}

#[inline]
fn world_restitution_threshold_impl(world: ffi::b2WorldId) -> f32 {
    unsafe { ffi::b2World_GetRestitutionThreshold(world) }
}

#[inline]
fn world_hit_event_threshold_impl(world: ffi::b2WorldId) -> f32 {
    unsafe { ffi::b2World_GetHitEventThreshold(world) }
}

#[inline]
fn world_maximum_linear_speed_impl(world: ffi::b2WorldId) -> f32 {
    unsafe { ffi::b2World_GetMaximumLinearSpeed(world) }
}

unsafe extern "C" fn custom_filter_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: context is provided by the custom-filter registration helpers and points to
    // `CustomFilterCtx` for the lifetime of the registered callback.
    let ctx = unsafe { &*(context as *const CustomFilterCtx) };
    let core = match ctx.core.upgrade() {
        Some(c) => c,
        None => return true,
    };
    if core
        .callback_panicked
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return true;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        let cw = CallbackWorld::new(Arc::clone(&core));
        (ctx.cb)(&cw, a, b)
    })) {
        Ok(v) => v,
        Err(payload) => {
            if !core
                .callback_panicked
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                *core
                    .callback_panic
                    .lock()
                    .expect("callback_panic mutex poisoned") = Some(payload);
            }
            true
        }
    }
}

unsafe extern "C" fn pre_solve_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    point: ffi::b2Vec2,
    normal: ffi::b2Vec2,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: context is provided by the pre-solve registration helpers and points to
    // `PreSolveCtx` for the lifetime of the registered callback.
    let ctx = unsafe { &*(context as *const PreSolveCtx) };
    let core = match ctx.core.upgrade() {
        Some(c) => c,
        None => return true,
    };
    if core
        .callback_panicked
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return true;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        let cw = CallbackWorld::new(Arc::clone(&core));
        (ctx.cb)(
            &cw,
            a,
            b,
            crate::types::Vec2::from(point),
            crate::types::Vec2::from(normal),
        )
    })) {
        Ok(v) => v,
        Err(payload) => {
            if !core
                .callback_panicked
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                *core
                    .callback_panic
                    .lock()
                    .expect("callback_panic mutex poisoned") = Some(payload);
            }
            true
        }
    }
}

/// Error type for world creation and operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create Box2D world")]
    CreateFailed,
}

/// World definition builder for constructing a simulation world.
#[doc(alias = "world_def")]
#[doc(alias = "worlddef")]
#[derive(Clone, Debug)]
pub struct WorldDef(ffi::b2WorldDef);

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

    pub fn from_raw(raw: ffi::b2WorldDef) -> Self {
        Self(raw)
    }

    pub fn gravity(&self) -> crate::types::Vec2 {
        crate::types::Vec2::from(self.0.gravity)
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

    pub fn into_raw(self) -> ffi::b2WorldDef {
        self.0
    }
}

// serde for WorldDef via config representation
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
            gravity: crate::types::Vec2::from(self.0.gravity),
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
            b.0.gravity = ffi::b2Vec2::from(g);
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
        self.def.0.gravity = ffi::b2Vec2::from(g.into());
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
    /// This controls Box2D's internal parallelism only; it does not make `World` or owned handles
    /// `Send` / `Sync`.
    pub fn worker_count(mut self, n: i32) -> Self {
        self.def.0.workerCount = n;
        self
    }

    #[must_use]
    pub fn build(self) -> WorldDef {
        self.def
    }
}

/// A simulation world.
///
/// Note: the underlying Box2D world is owned by an internal reference-counted core, so it will
/// be destroyed when the last owned handle (`OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain`)
/// is dropped.
pub struct World {
    core: Arc<WorldCore>,
    // Box2D's external API is not thread-safe; prevent `World: Send/Sync`.
    _not_send_sync: core::marker::PhantomData<Rc<()>>,
}

/// A cheap, cloneable handle to a world that keeps it alive via the internal reference-counted core.
///
/// Unlike `&World`, this does not borrow the world, which makes it convenient to store inside other
/// objects (e.g. debug draw implementations). It is still `!Send`/`!Sync` to match Box2D's thread
/// safety guarantees.
///
/// `WorldHandle` intentionally focuses on stored read-only queries/diagnostics. Step-local event
/// snapshot/view APIs remain on [`World`] because they are tied to Box2D's completed-step event
/// buffers plus deferred-destroy flushing behavior.
#[derive(Clone)]
pub struct WorldHandle {
    core: Arc<WorldCore>,
    _not_send_sync: core::marker::PhantomData<Rc<()>>,
}

/// A lightweight, thread-safe context passed to Box2D callbacks.
///
/// This type intentionally exposes only APIs that do not call into Box2D while the world is locked.
#[derive(Clone)]
pub struct CallbackWorld {
    core: Arc<WorldCore>,
}

impl CallbackWorld {
    pub(crate) fn new(core: Arc<WorldCore>) -> Self {
        Self { core }
    }

    pub fn with_body_user_data<T: 'static + Sync, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.core
            .try_with_body_user_data(id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_body_user_data<T: 'static + Sync, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_body_user_data(id, f)
    }

    pub fn with_shape_user_data<T: 'static + Sync, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.core
            .try_with_shape_user_data(id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_shape_user_data<T: 'static + Sync, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_shape_user_data(id, f)
    }

    pub fn with_joint_user_data<T: 'static + Sync, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.core
            .try_with_joint_user_data(id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_joint_user_data<T: 'static + Sync, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_joint_user_data(id, f)
    }

    pub fn with_world_user_data<T: 'static + Sync, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.core
            .try_with_world_user_data(f)
            .expect("user data type mismatch")
    }

    pub fn try_with_world_user_data<T: 'static + Sync, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_world_user_data(f)
    }
}

impl WorldHandle {
    /// Expose raw world id for advanced use-cases.
    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.core.id
    }

    pub(crate) fn raw(&self) -> ffi::b2WorldId {
        self.world_id_raw()
    }

    impl_world_gravity_methods!();
    impl_world_runtime_snapshot_methods!();
    impl_world_runtime_tuning_getter_methods!();
}

#[cfg(feature = "serialize")]
pub use crate::core::serialize_registry::{
    ChainCreateRecord, ChainMaterialsRecord, ShapeFlagsRecord,
};

impl World {
    fn ensure_material_mix_slot(&self) -> crate::error::ApiResult<usize> {
        let mut slot = self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned");
        if let Some(slot) = *slot {
            return Ok(slot);
        }

        let Some(new_slot) = crate::core::material_mix_registry::acquire_slot() else {
            return Err(crate::error::ApiError::CallbackSlotsExhausted);
        };
        *slot = Some(new_slot);
        Ok(new_slot)
    }

    fn maybe_release_material_mix_slot(&self) {
        let mut slot = self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned");
        if let Some(slot_index) = *slot
            && !crate::core::material_mix_registry::has_any_callback(slot_index)
        {
            crate::core::material_mix_registry::release_slot(slot_index);
            *slot = None;
        }
    }

    fn set_custom_filter_with_ctx_impl<F>(&mut self, f: F)
    where
        F: Fn(&CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
            + Send
            + Sync
            + 'static,
    {
        let ctx = Box::new(CustomFilterCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        self.install_custom_filter_ctx(ctx);
    }

    fn install_custom_filter_ctx(&mut self, ctx: Box<CustomFilterCtx>) {
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const CustomFilterCtx as *mut _;
        unsafe {
            ffi::b2World_SetCustomFilterCallback(self.raw(), Some(custom_filter_callback), ctx_ptr)
        };
        *self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned") = Some(ctx);
    }

    fn clear_custom_filter_impl(&mut self) {
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), None, core::ptr::null_mut()) };
        *self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned") = None;
    }

    fn set_pre_solve_with_ctx_impl<F>(&mut self, f: F)
    where
        F: Fn(
                &CallbackWorld,
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        let ctx = Box::new(PreSolveCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        self.install_pre_solve_ctx(ctx);
    }

    fn install_pre_solve_ctx(&mut self, ctx: Box<PreSolveCtx>) {
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const PreSolveCtx as *mut _;
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), Some(pre_solve_callback), ctx_ptr) };
        *self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned") = Some(ctx);
    }

    fn clear_pre_solve_impl(&mut self) {
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), None, core::ptr::null_mut()) };
        *self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned") = None;
    }

    /// Create a world from a definition.
    pub fn new(def: WorldDef) -> Result<Self, Error> {
        let _guard = crate::core::box2d_lock::lock();
        let raw = def.into_raw();
        // SAFETY: FFI call to create a world; returns an id handle
        let world_id = unsafe { ffi::b2CreateWorld(&raw) };
        let ok = unsafe { ffi::b2World_IsValid(world_id) };
        if ok {
            Ok(Self {
                core: WorldCore::new(world_id),
                _not_send_sync: core::marker::PhantomData,
            })
        } else {
            Err(Error::CreateFailed)
        }
    }

    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    pub fn step(&mut self, time_step: f32, sub_steps: i32) {
        crate::core::callback_state::assert_not_in_callback();
        // Prepare panic forwarding for callbacks invoked during the FFI call.
        self.core
            .callback_panicked
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self
            .core
            .callback_panic
            .lock()
            .expect("callback_panic mutex poisoned") = None;
        // SAFETY: valid world id managed by RAII
        unsafe { ffi::b2World_Step(self.raw(), time_step, sub_steps) };

        // Flush deferred destroys scheduled from callbacks.
        self.core.process_deferred_destroys();

        if self
            .core
            .callback_panicked
            .load(std::sync::atomic::Ordering::Relaxed)
            && let Some(payload) = self
                .core
                .callback_panic
                .lock()
                .expect("callback_panic mutex poisoned")
                .take()
        {
            std::panic::resume_unwind(payload);
        }
    }

    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_step(&mut self, time_step: f32, sub_steps: i32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.step(time_step, sub_steps);
        Ok(())
    }

    /// Flush deferred destroys scheduled from Box2D callbacks.
    ///
    /// Most users don't need to call this because `World::step`, event view helpers
    /// (`with_*_events_view`), and debug draw helpers flush automatically. This is useful if you
    /// drop `Owned*` handles during callbacks but want to reclaim resources without stepping the
    /// simulation again.
    pub fn flush_deferred_destroys(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.core.process_deferred_destroys();
    }

    /// Flush deferred destroys scheduled from Box2D callbacks.
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_flush_deferred_destroys(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.flush_deferred_destroys();
        Ok(())
    }

    /// Set gravity vector.
    pub fn set_gravity<V: Into<Vec2>>(&mut self, g: V) {
        crate::core::callback_state::assert_not_in_callback();
        let gv: ffi::b2Vec2 = g.into().into();
        unsafe { ffi::b2World_SetGravity(self.raw(), gv) };
    }

    pub fn try_set_gravity<V: Into<Vec2>>(&mut self, g: V) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        let gv: ffi::b2Vec2 = g.into().into();
        unsafe { ffi::b2World_SetGravity(self.raw(), gv) };
        Ok(())
    }

    impl_world_gravity_methods!();

    /// Expose the raw Box2D world id for advanced use-cases.
    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.core.id
    }

    pub(crate) fn raw(&self) -> ffi::b2WorldId {
        self.world_id_raw()
    }

    pub(crate) fn core_arc(&self) -> Arc<WorldCore> {
        Arc::clone(&self.core)
    }

    pub(crate) fn with_borrowed_event_buffers<T>(&self, f: impl FnOnce() -> T) -> T {
        crate::core::callback_state::assert_not_in_callback();
        let core = self.core_arc();
        let out = {
            let _borrow = core.borrow_event_buffers();
            f()
        };
        // Nested raw/view event borrows are allowed. Deferred destroys must wait until the
        // outermost borrow ends so previously returned event slices cannot be invalidated early.
        core.process_deferred_destroys();
        out
    }

    pub(crate) fn try_with_borrowed_event_buffers<T>(
        &self,
        f: impl FnOnce() -> T,
    ) -> crate::error::ApiResult<T> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(self.with_borrowed_event_buffers(f))
    }

    // --- Typed user data ---------------------------------------------------------
    /// Set typed user data on this world.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the world is dropped.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        crate::core::callback_state::assert_not_in_callback();
        let p = self.core.set_world_user_data(value);
        unsafe { ffi::b2World_SetUserData(self.raw(), p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        let p = self.core.set_world_user_data(value);
        unsafe { ffi::b2World_SetUserData(self.raw(), p) };
        Ok(())
    }

    /// Clear typed user data on this world. Returns whether any data was present.
    pub fn clear_user_data(&mut self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        let had = unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() };
        unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        self.core.clear_world_user_data();
        had
    }

    pub fn try_clear_user_data(&mut self) -> crate::error::ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        let had = unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() };
        unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        self.core.clear_world_user_data();
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .try_with_world_user_data(f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.try_with_world_user_data(f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        crate::core::callback_state::assert_not_in_callback();
        let v = self
            .core
            .take_world_user_data::<T>()
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> crate::error::ApiResult<Option<T>> {
        crate::core::callback_state::check_not_in_callback()?;
        let v = self.core.take_world_user_data::<T>()?;
        if v.is_some() {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        Ok(v)
    }

    /// Create a cheap, cloneable handle to this world.
    pub fn handle(&self) -> WorldHandle {
        WorldHandle {
            core: Arc::clone(&self.core),
            _not_send_sync: core::marker::PhantomData,
        }
    }

    /// Number of outstanding owned handles (`OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain`).
    pub fn owned_handle_count(&self) -> usize {
        Arc::strong_count(&self.core).saturating_sub(1)
    }

    pub fn owned_handle_counts(&self) -> OwnedHandleCounts {
        let (bodies, shapes, joints, chains) = self.core.owned_counts();
        OwnedHandleCounts {
            bodies,
            shapes,
            joints,
            chains,
        }
    }

    /// Attempt to destroy the world by consuming `self`.
    ///
    /// This returns an error if there are still owned handles alive, because they keep the world
    /// core reference-counted and prevent destruction.
    pub fn shutdown(self) -> Result<(), (Self, OutstandingOwnedHandles)> {
        let strong = Arc::strong_count(&self.core);
        if strong == 1 {
            Ok(())
        } else {
            let (bodies, shapes, joints, chains) = self.core.owned_counts();
            Err((
                self,
                OutstandingOwnedHandles {
                    strong_count: strong,
                    counts: OwnedHandleCounts {
                        bodies,
                        shapes,
                        joints,
                        chains,
                    },
                },
            ))
        }
    }

    /// Enumerate known body ids created via this wrapper. Invalid/destroyed ids are filtered out.
    #[cfg(feature = "serialize")]
    pub fn body_ids(&self) -> Vec<BodyId> {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .body_ids()
    }

    /// Enumerate known body ids created via this wrapper into a caller-owned buffer.
    #[cfg(feature = "serialize")]
    pub fn body_ids_into(&self, out: &mut Vec<BodyId>) {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .body_ids_into(out);
    }

    /// Enumerate known body ids created via this wrapper. Invalid/destroyed ids are filtered out.
    #[cfg(feature = "serialize")]
    pub fn try_body_ids(&self) -> crate::error::ApiResult<Vec<BodyId>> {
        crate::core::callback_state::check_not_in_callback()?;
        let mut out = Vec::new();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .body_ids_into(&mut out);
        Ok(out)
    }

    /// Enumerate known body ids created via this wrapper into a caller-owned buffer.
    #[cfg(feature = "serialize")]
    pub fn try_body_ids_into(&self, out: &mut Vec<BodyId>) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .body_ids_into(out);
        Ok(())
    }

    /// Return chain creation records captured at creation time using crate-owned value types.
    #[cfg(feature = "serialize")]
    pub fn chain_records(&self) -> Vec<ChainCreateRecord> {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .chain_records()
    }

    /// Return chain creation records captured at creation time into a caller-owned buffer.
    #[cfg(feature = "serialize")]
    pub fn chain_records_into(&self, out: &mut Vec<ChainCreateRecord>) {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .chain_records_into(out);
    }

    /// Return chain creation records captured at creation time using crate-owned value types.
    #[cfg(feature = "serialize")]
    pub fn try_chain_records(&self) -> crate::error::ApiResult<Vec<ChainCreateRecord>> {
        crate::core::callback_state::check_not_in_callback()?;
        let mut out = Vec::new();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .chain_records_into(&mut out);
        Ok(out)
    }

    /// Return chain creation records captured at creation time into a caller-owned buffer.
    #[cfg(feature = "serialize")]
    pub fn try_chain_records_into(
        &self,
        out: &mut Vec<ChainCreateRecord>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .chain_records_into(out);
        Ok(())
    }

    /// Return recorded shape flags for shapes created via this wrapper.
    #[cfg(feature = "serialize")]
    pub fn shape_flags(&self, sid: ShapeId) -> Option<ShapeFlagsRecord> {
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .shape_flags(sid)
    }

    impl_world_runtime_snapshot_methods!();

    /// Get a body's transform safely from its id.
    pub fn body_transform(&self, body: BodyId) -> Transform {
        crate::core::debug_checks::assert_body_valid(body);
        Transform::from(unsafe { ffi::b2Body_GetTransform(body) })
    }

    pub fn try_body_transform(&self, body: BodyId) -> crate::error::ApiResult<Transform> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Transform::from(unsafe { ffi::b2Body_GetTransform(body) }))
    }

    /// Get a body's world position.
    pub fn body_position(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        Vec2::from(unsafe { ffi::b2Body_GetPosition(body) })
    }

    pub fn try_body_position(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Vec2::from(unsafe { ffi::b2Body_GetPosition(body) }))
    }

    pub fn body_rotation(&self, body: BodyId) -> crate::Rot {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_rotation_impl(body)
    }

    pub fn try_body_rotation(&self, body: BodyId) -> crate::error::ApiResult<crate::Rot> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_rotation_impl(body))
    }

    pub fn body_aabb(&self, body: BodyId) -> Aabb {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_aabb_impl(body)
    }

    pub fn try_body_aabb(&self, body: BodyId) -> crate::error::ApiResult<Aabb> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_aabb_impl(body))
    }

    pub fn body_local_point<V: Into<Vec2>>(&self, body: BodyId, world_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(body, p) })
    }

    pub fn try_body_local_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(body, p) }))
    }

    pub fn body_world_point<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(body, p) })
    }

    pub fn try_body_world_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(body, p) }))
    }

    pub fn body_local_vector<V: Into<Vec2>>(&self, body: BodyId, world_vector: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let v: ffi::b2Vec2 = world_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalVector(body, v) })
    }

    pub fn try_body_local_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let v: ffi::b2Vec2 = world_vector.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetLocalVector(body, v) }))
    }

    pub fn body_world_vector<V: Into<Vec2>>(&self, body: BodyId, local_vector: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let v: ffi::b2Vec2 = local_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldVector(body, v) })
    }

    pub fn try_body_world_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let v: ffi::b2Vec2 = local_vector.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetWorldVector(body, v) }))
    }

    pub fn body_local_point_velocity<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPointVelocity(body, p) })
    }

    pub fn try_body_local_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalPointVelocity(body, p)
        }))
    }

    pub fn body_world_point_velocity<V: Into<Vec2>>(&self, body: BodyId, world_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPointVelocity(body, p) })
    }

    pub fn try_body_world_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldPointVelocity(body, p)
        }))
    }

    pub fn body_mass(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_GetMass(body) }
    }

    pub fn try_body_mass(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(unsafe { ffi::b2Body_GetMass(body) })
    }

    pub fn body_rotational_inertia(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_GetRotationalInertia(body) }
    }

    pub fn try_body_rotational_inertia(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(unsafe { ffi::b2Body_GetRotationalInertia(body) })
    }

    pub fn body_local_center_of_mass(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        Vec2::from(unsafe { ffi::b2Body_GetLocalCenterOfMass(body) })
    }

    pub fn try_body_local_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalCenterOfMass(body)
        }))
    }

    pub fn body_world_center_of_mass(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        Vec2::from(unsafe { ffi::b2Body_GetWorldCenterOfMass(body) })
    }

    pub fn try_body_world_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldCenterOfMass(body)
        }))
    }

    pub fn body_mass_data(&self, body: BodyId) -> MassData {
        crate::core::debug_checks::assert_body_valid(body);
        MassData::from_raw(unsafe { ffi::b2Body_GetMassData(body) })
    }

    pub fn try_body_mass_data(&self, body: BodyId) -> crate::error::ApiResult<MassData> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(MassData::from_raw(unsafe { ffi::b2Body_GetMassData(body) }))
    }

    pub fn set_body_mass_data(&mut self, body: BodyId, mass_data: MassData) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetMassData(body, mass_data.into_raw()) };
    }

    pub fn try_set_body_mass_data(
        &mut self,
        body: BodyId,
        mass_data: MassData,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetMassData(body, mass_data.into_raw()) };
        Ok(())
    }

    pub fn body_apply_mass_from_shapes(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_ApplyMassFromShapes(body) };
    }

    pub fn try_body_apply_mass_from_shapes(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_ApplyMassFromShapes(body) };
        Ok(())
    }

    pub fn body_shape_count(&self, body: BodyId) -> i32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_shape_count_impl(body)
    }

    pub fn try_body_shape_count(&self, body: BodyId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_shape_count_impl(body))
    }

    pub fn body_shapes(&self, body: BodyId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_shapes_impl(body)
    }

    pub fn body_shapes_into(&self, body: BodyId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_shapes_into_impl(body, out);
    }

    pub fn try_body_shapes(&self, body: BodyId) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_shapes_impl(body))
    }

    pub fn try_body_shapes_into(
        &self,
        body: BodyId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_shapes_into_impl(body, out);
        Ok(())
    }

    pub fn body_joint_count(&self, body: BodyId) -> i32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_joint_count_impl(body)
    }

    pub fn try_body_joint_count(&self, body: BodyId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_joint_count_impl(body))
    }

    pub fn body_joints(&self, body: BodyId) -> Vec<JointId> {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_joints_impl(body)
    }

    pub fn body_joints_into(&self, body: BodyId, out: &mut Vec<JointId>) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_joints_into_impl(body, out);
    }

    pub fn try_body_joints(&self, body: BodyId) -> crate::error::ApiResult<Vec<JointId>> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_joints_impl(body))
    }

    pub fn try_body_joints_into(
        &self,
        body: BodyId,
        out: &mut Vec<JointId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_joints_into_impl(body, out);
        Ok(())
    }

    pub fn set_body_target_transform(
        &mut self,
        body: BodyId,
        target: Transform,
        time_step: f32,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetTargetTransform(body, target.into(), time_step, wake) };
    }

    pub fn try_set_body_target_transform(
        &mut self,
        body: BodyId,
        target: Transform,
        time_step: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetTargetTransform(body, target.into(), time_step, wake) };
        Ok(())
    }

    /// Set a body's world position and rotation (angle in radians) by id.
    pub fn set_body_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        p: V,
        angle_radians: f32,
    ) {
        crate::core::debug_checks::assert_body_valid(body);
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(body, pos, rot) };
    }

    pub fn try_set_body_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        p: V,
        angle_radians: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(body, pos, rot) };
        Ok(())
    }

    /// Set a body's linear velocity by id.
    pub fn set_body_linear_velocity<V: Into<Vec2>>(&mut self, body: BodyId, v: V) {
        crate::core::debug_checks::assert_body_valid(body);
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(body, vv) }
    }

    pub fn try_set_body_linear_velocity<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        v: V,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(body, vv) }
        Ok(())
    }

    /// Set a body's angular velocity by id.
    pub fn set_body_angular_velocity(&mut self, body: BodyId, w: f32) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetAngularVelocity(body, w) }
    }

    pub fn try_set_body_angular_velocity(
        &mut self,
        body: BodyId,
        w: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetAngularVelocity(body, w) }
        Ok(())
    }

    pub fn body_enable_sleep(&mut self, body: BodyId, flag: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_enable_sleep_impl(body, flag)
    }

    pub fn try_body_enable_sleep(
        &mut self,
        body: BodyId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_enable_sleep_impl(body, flag);
        Ok(())
    }

    pub fn body_is_sleep_enabled(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_sleep_enabled_impl(body)
    }

    pub fn try_body_is_sleep_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_sleep_enabled_impl(body))
    }

    pub fn body_sleep_threshold(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_sleep_threshold_impl(body)
    }

    pub fn try_body_sleep_threshold(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_sleep_threshold_impl(body))
    }

    pub fn set_body_sleep_threshold(&mut self, body: BodyId, sleep_threshold: f32) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_set_sleep_threshold_impl(body, sleep_threshold)
    }

    pub fn try_set_body_sleep_threshold(
        &mut self,
        body: BodyId,
        sleep_threshold: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_set_sleep_threshold_impl(body, sleep_threshold);
        Ok(())
    }

    pub fn body_is_awake(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_awake_impl(body)
    }

    pub fn try_body_is_awake(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_awake_impl(body))
    }

    pub fn set_body_awake(&mut self, body: BodyId, awake: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_set_awake_impl(body, awake)
    }

    pub fn try_set_body_awake(&mut self, body: BodyId, awake: bool) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_set_awake_impl(body, awake);
        Ok(())
    }

    pub fn body_is_enabled(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_enabled_impl(body)
    }

    pub fn try_body_is_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_enabled_impl(body))
    }

    pub fn body_enable_contact_events(&mut self, body: BodyId, flag: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_enable_contact_events_impl(body, flag)
    }

    pub fn try_body_enable_contact_events(
        &mut self,
        body: BodyId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_enable_contact_events_impl(body, flag);
        Ok(())
    }

    pub fn body_enable_hit_events(&mut self, body: BodyId, flag: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_enable_hit_events_impl(body, flag)
    }

    pub fn try_body_enable_hit_events(
        &mut self,
        body: BodyId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_enable_hit_events_impl(body, flag);
        Ok(())
    }

    /// Get the current motion locks for a body.
    pub fn body_motion_locks(&self, body: BodyId) -> MotionLocks {
        crate::core::debug_checks::assert_body_valid(body);
        MotionLocks::from_raw(unsafe { ffi::b2Body_GetMotionLocks(body) })
    }

    pub fn try_body_motion_locks(&self, body: BodyId) -> crate::error::ApiResult<MotionLocks> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(MotionLocks::from_raw(unsafe {
            ffi::b2Body_GetMotionLocks(body)
        }))
    }

    /// Set motion locks (translation/rotation constraints) for a body.
    pub fn set_body_motion_locks(&mut self, body: BodyId, locks: MotionLocks) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetMotionLocks(body, locks.into_raw()) }
    }

    pub fn try_set_body_motion_locks(
        &mut self,
        body: BodyId,
        locks: MotionLocks,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetMotionLocks(body, locks.into_raw()) }
        Ok(())
    }

    /// Apply a linear impulse to the center of mass of a body.
    pub fn body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_body_valid(body);
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(body, i, wake) };
    }

    pub fn try_body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(body, i, wake) };
        Ok(())
    }

    /// Apply an angular impulse to a body.
    pub fn body_apply_angular_impulse(&mut self, body: BodyId, impulse: f32, wake: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_ApplyAngularImpulse(body, impulse, wake) };
    }

    pub fn try_body_apply_angular_impulse(
        &mut self,
        body: BodyId,
        impulse: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_ApplyAngularImpulse(body, impulse, wake) };
        Ok(())
    }

    /// Clear accumulated forces and torque on a body (usually only needed before stepping).
    pub fn body_clear_forces(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_ClearForces(body) };
    }

    pub fn try_body_clear_forces(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_ClearForces(body) };
        Ok(())
    }

    /// Wake all touching bodies.
    pub fn body_wake_touching(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_WakeTouching(body) };
    }

    pub fn try_body_wake_touching(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_WakeTouching(body) };
        Ok(())
    }

    /// Set a body's type by id.
    pub fn set_body_type(&mut self, body: BodyId, t: BodyType) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetType(body, t.into_raw()) }
    }

    pub fn try_set_body_type(&mut self, body: BodyId, t: BodyType) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetType(body, t.into_raw()) }
        Ok(())
    }

    /// Enable a body by id.
    pub fn enable_body(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_enable_impl(body)
    }

    pub fn try_enable_body(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_enable_impl(body);
        Ok(())
    }

    /// Disable a body by id.
    pub fn disable_body(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_disable_impl(body)
    }

    pub fn try_disable_body(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_disable_impl(body);
        Ok(())
    }

    pub fn body_is_bullet(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_bullet_impl(body)
    }

    pub fn try_body_is_bullet(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_bullet_impl(body))
    }

    pub fn set_body_bullet(&mut self, body: BodyId, bullet: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_set_bullet_impl(body, bullet)
    }

    pub fn try_set_body_bullet(
        &mut self,
        body: BodyId,
        bullet: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_set_bullet_impl(body, bullet);
        Ok(())
    }

    /// Set a body's name by id.
    pub fn set_body_name(&mut self, body: BodyId, name: &str) {
        crate::core::debug_checks::assert_body_valid(body);
        let cs = CString::new(name).expect("body name contains an interior NUL byte");
        crate::body::body_set_name_impl(body, cs.as_c_str())
    }

    pub fn try_set_body_name(&mut self, body: BodyId, name: &str) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let cs = CString::new(name).map_err(|_| crate::error::ApiError::NulByteInString)?;
        crate::body::body_set_name_impl(body, cs.as_c_str());
        Ok(())
    }

    pub fn body_name(&self, body: BodyId) -> Option<String> {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_name_impl(body)
    }

    pub fn try_body_name(&self, body: BodyId) -> crate::error::ApiResult<Option<String>> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_name_impl(body))
    }

    /// Create a body owned by this world.
    pub fn create_body<'w>(&'w mut self, def: BodyDef) -> Body<'w> {
        crate::core::callback_state::assert_not_in_callback();
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Body::new(self.core_arc(), id)
    }

    pub fn try_create_body<'w>(&'w mut self, def: BodyDef) -> crate::error::ApiResult<Body<'w>> {
        crate::core::callback_state::check_not_in_callback()?;
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Ok(Body::new(self.core_arc(), id))
    }

    /// Create a RAII-owned body. Dropping the returned handle destroys the body.
    pub fn create_body_owned(&mut self, def: BodyDef) -> crate::body::OwnedBody {
        crate::core::callback_state::assert_not_in_callback();
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        crate::body::OwnedBody::new(self.core_arc(), id)
    }

    pub fn try_create_body_owned(
        &mut self,
        def: BodyDef,
    ) -> crate::error::ApiResult<crate::body::OwnedBody> {
        crate::core::callback_state::check_not_in_callback()?;
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Ok(crate::body::OwnedBody::new(self.core_arc(), id))
    }

    /// ID-style body creation. Prefer when you want to store/pass ids without borrowing the world.
    pub fn create_body_id(&mut self, def: BodyDef) -> BodyId {
        crate::core::callback_state::assert_not_in_callback();
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        id
    }

    pub fn try_create_body_id(&mut self, def: BodyDef) -> crate::error::ApiResult<BodyId> {
        crate::core::callback_state::check_not_in_callback()?;
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Ok(id)
    }

    /// Destroy a body by id.
    pub fn destroy_body_id(&mut self, id: BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Body_IsValid(id) } {
            #[cfg(feature = "serialize")]
            self.core.cleanup_before_destroy_body(id);
            unsafe { ffi::b2DestroyBody(id) };
            let _ = self.core.clear_body_user_data(id);
        }
    }

    pub fn try_destroy_body_id(&mut self, id: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(id)?;
        #[cfg(feature = "serialize")]
        self.core.cleanup_before_destroy_body(id);
        unsafe { ffi::b2DestroyBody(id) };
        let _ = self.core.clear_body_user_data(id);
        Ok(())
    }

    /// Borrow a scoped body handle by id (returns `None` if the id is invalid).
    pub fn body<'w>(&'w mut self, id: BodyId) -> Option<Body<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Body_IsValid(id) } {
            Some(Body::new(self.core_arc(), id))
        } else {
            None
        }
    }

    /// Borrow a scoped joint handle by id (returns `None` if the id is invalid).
    pub fn joint<'w>(&'w mut self, id: JointId) -> Option<crate::joints::Joint<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(id) } {
            Some(crate::joints::Joint::new(self.core_arc(), id))
        } else {
            None
        }
    }

    /// Borrow a scoped shape handle by id (returns `None` if the id is invalid).
    pub fn shape<'w>(&'w mut self, id: ShapeId) -> Option<crate::shapes::Shape<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Shape_IsValid(id) } {
            Some(crate::shapes::Shape::new(self.core_arc(), id))
        } else {
            None
        }
    }

    /// Borrow a scoped chain handle by id (returns `None` if the id is invalid).
    pub fn chain<'w>(&'w mut self, id: ChainId) -> Option<crate::shapes::chain::Chain<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Chain_IsValid(id) } {
            Some(crate::shapes::chain::Chain::new(self.core_arc(), id))
        } else {
            None
        }
    }

    // Runtime configuration helpers mirroring WorldDef fields
    pub fn enable_sleeping(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableSleeping(self.raw(), flag) }
    }
    pub fn try_enable_sleeping(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableSleeping(self.raw(), flag) }
        Ok(())
    }
    pub fn enable_continuous(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableContinuous(self.raw(), flag) }
    }
    pub fn try_enable_continuous(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableContinuous(self.raw(), flag) }
        Ok(())
    }
    /// Enable or disable constraint warm starting at runtime.
    ///
    /// Warm starting seeds the solver with accumulated impulses from the previous
    /// step to improve stability and convergence. Disabling this is only useful
    /// for experiments and will significantly reduce stability in most scenes.
    pub fn enable_warm_starting(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableWarmStarting(self.raw(), flag) }
    }
    pub fn try_enable_warm_starting(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableWarmStarting(self.raw(), flag) }
        Ok(())
    }
    pub fn set_restitution_threshold(&mut self, value: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetRestitutionThreshold(self.raw(), value) }
    }
    pub fn try_set_restitution_threshold(&mut self, value: f32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_SetRestitutionThreshold(self.raw(), value) }
        Ok(())
    }
    pub fn set_hit_event_threshold(&mut self, value: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetHitEventThreshold(self.raw(), value) }
    }
    pub fn try_set_hit_event_threshold(&mut self, value: f32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_SetHitEventThreshold(self.raw(), value) }
        Ok(())
    }
    pub fn set_contact_tuning(&mut self, hertz: f32, damping_ratio: f32, push_speed: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetContactTuning(self.raw(), hertz, damping_ratio, push_speed) }
    }
    pub fn try_set_contact_tuning(
        &mut self,
        hertz: f32,
        damping_ratio: f32,
        push_speed: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_SetContactTuning(self.raw(), hertz, damping_ratio, push_speed) }
        Ok(())
    }
    /// Enable or disable speculative collision handling at runtime.
    pub fn enable_speculative(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableSpeculative(self.raw(), flag) }
    }
    pub fn try_enable_speculative(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableSpeculative(self.raw(), flag) }
        Ok(())
    }
    pub fn set_maximum_linear_speed(&mut self, v: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetMaximumLinearSpeed(self.raw(), v) }
    }
    pub fn try_set_maximum_linear_speed(&mut self, v: f32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_SetMaximumLinearSpeed(self.raw(), v) }
        Ok(())
    }
    impl_world_runtime_tuning_getter_methods!();

    // --- Collision/solve callbacks ---------------------------------------------------------
    /// Register a thread-safe custom filter closure. This is called when a contact pair is
    /// considered for collision if either shape has custom filtering enabled.
    /// Return false to disable the collision.
    ///
    /// Note: Box2D runs this callback while the world is locked. Use the provided `CallbackWorld`
    /// context for operations that must be safe under this constraint (e.g. typed user data).
    pub fn set_custom_filter_with_ctx<F>(&mut self, f: F)
    where
        F: Fn(&CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_custom_filter_with_ctx_impl(f);
    }

    pub fn try_set_custom_filter_with_ctx<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(&CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_custom_filter_with_ctx_impl(f);
        Ok(())
    }

    /// Backwards-compatible custom filter API without a callback context.
    pub fn set_custom_filter<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_custom_filter_with_ctx_impl(move |_, a, b| f(a, b))
    }

    pub fn try_set_custom_filter<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_custom_filter_with_ctx_impl(move |_, a, b| f(a, b));
        Ok(())
    }

    /// Clear the custom filter callback and release associated resources.
    pub fn clear_custom_filter(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.clear_custom_filter_impl();
    }

    pub fn try_clear_custom_filter(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_custom_filter_impl();
        Ok(())
    }

    /// Register a thread-safe pre-solve closure. This is called after contact update (when enabled
    /// on shapes) and before the solver. Return false to disable the contact this step.
    ///
    /// Note: Box2D runs this callback while the world is locked. Use the provided `CallbackWorld`
    /// context for operations that must be safe under this constraint (e.g. typed user data).
    pub fn set_pre_solve_with_ctx<F>(&mut self, f: F)
    where
        F: Fn(
                &CallbackWorld,
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_pre_solve_with_ctx_impl(f);
    }

    pub fn try_set_pre_solve_with_ctx<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(
                &CallbackWorld,
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_pre_solve_with_ctx_impl(f);
        Ok(())
    }

    /// Backwards-compatible pre-solve API without a callback context.
    pub fn set_pre_solve<F>(&mut self, f: F)
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| f(a, b, p, n))
    }

    pub fn try_set_pre_solve<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| f(a, b, p, n));
        Ok(())
    }

    /// Clear the pre-solve callback and release associated resources.
    pub fn clear_pre_solve(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.clear_pre_solve_impl();
    }

    pub fn try_clear_pre_solve(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_pre_solve_impl();
        Ok(())
    }

    /// Compatibility helper: set or clear the custom filter using a plain function pointer.
    pub fn set_custom_filter_callback(&mut self, cb: Option<ShapeFilterFn>) {
        crate::core::callback_state::assert_not_in_callback();
        match cb {
            Some(func) => self.set_custom_filter_with_ctx_impl(move |_, a, b| func(a, b)),
            None => self.clear_custom_filter_impl(),
        }
    }

    pub fn try_set_custom_filter_callback(
        &mut self,
        cb: Option<ShapeFilterFn>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        match cb {
            Some(func) => self.set_custom_filter_with_ctx_impl(move |_, a, b| func(a, b)),
            None => self.clear_custom_filter_impl(),
        }
        Ok(())
    }

    /// Compatibility helper: set or clear the pre-solve using a plain function pointer.
    pub fn set_pre_solve_callback(&mut self, cb: Option<PreSolveFn>) {
        crate::core::callback_state::assert_not_in_callback();
        match cb {
            Some(func) => self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| func(a, b, p, n)),
            None => self.clear_pre_solve_impl(),
        }
    }

    pub fn try_set_pre_solve_callback(
        &mut self,
        cb: Option<PreSolveFn>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        match cb {
            Some(func) => self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| func(a, b, p, n)),
            None => self.clear_pre_solve_impl(),
        }
        Ok(())
    }

    /// Register a thread-safe friction mixing callback.
    ///
    /// This callback may run on Box2D worker threads and intentionally receives no world context.
    /// Use `user_material_id` to implement table-driven material behavior.
    ///
    /// The callback must not attempt to modify Box2D state or unsafely mutate shared application
    /// state.
    pub fn set_friction_callback<F>(&mut self, f: F)
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        self.try_set_friction_callback(f)
            .expect("no free callback slot is available for material mixing callbacks");
    }

    pub fn try_set_friction_callback<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        let slot = self.ensure_material_mix_slot()?;
        let ctx = Box::new(MaterialMixCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        let ptr = (&*ctx) as *const MaterialMixCtx as *mut MaterialMixCtx;
        crate::core::material_mix_registry::set_friction_ptr(slot, ptr);
        *self
            .core
            .friction_mix
            .lock()
            .expect("friction_mix mutex poisoned") = Some(ctx);
        unsafe {
            ffi::b2World_SetFrictionCallback(
                self.raw(),
                crate::core::material_mix_registry::friction_callback(slot),
            );
        }
        Ok(())
    }

    /// Clear the friction mixing callback and restore Box2D's default mixing rule.
    pub fn clear_friction_callback(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        if let Some(slot) = *self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned")
        {
            unsafe { ffi::b2World_SetFrictionCallback(self.raw(), None) };
            crate::core::material_mix_registry::set_friction_ptr(slot, core::ptr::null_mut());
            *self
                .core
                .friction_mix
                .lock()
                .expect("friction_mix mutex poisoned") = None;
            self.maybe_release_material_mix_slot();
        }
    }

    pub fn try_clear_friction_callback(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_friction_callback();
        Ok(())
    }

    /// Register a thread-safe restitution mixing callback.
    ///
    /// This callback may run on Box2D worker threads and intentionally receives no world context.
    /// Use `user_material_id` to implement table-driven material behavior.
    ///
    /// The callback must not attempt to modify Box2D state or unsafely mutate shared application
    /// state.
    pub fn set_restitution_callback<F>(&mut self, f: F)
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        self.try_set_restitution_callback(f)
            .expect("no free callback slot is available for material mixing callbacks");
    }

    pub fn try_set_restitution_callback<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        let slot = self.ensure_material_mix_slot()?;
        let ctx = Box::new(MaterialMixCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        let ptr = (&*ctx) as *const MaterialMixCtx as *mut MaterialMixCtx;
        crate::core::material_mix_registry::set_restitution_ptr(slot, ptr);
        *self
            .core
            .restitution_mix
            .lock()
            .expect("restitution_mix mutex poisoned") = Some(ctx);
        unsafe {
            ffi::b2World_SetRestitutionCallback(
                self.raw(),
                crate::core::material_mix_registry::restitution_callback(slot),
            );
        }
        Ok(())
    }

    /// Clear the restitution mixing callback and restore Box2D's default mixing rule.
    pub fn clear_restitution_callback(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        if let Some(slot) = *self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned")
        {
            unsafe { ffi::b2World_SetRestitutionCallback(self.raw(), None) };
            crate::core::material_mix_registry::set_restitution_ptr(slot, core::ptr::null_mut());
            *self
                .core
                .restitution_mix
                .lock()
                .expect("restitution_mix mutex poisoned") = None;
            self.maybe_release_material_mix_slot();
        }
    }

    pub fn try_clear_restitution_callback(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_restitution_callback();
        Ok(())
    }

    // Convenience joints built from world anchors and axis using body ids
    pub fn create_revolute_joint_world<VA: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> crate::joints::Joint<'_> {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        let aw: ffi::b2Vec2 = anchor_world.into().into();
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
            )
            .build();
        let def = crate::joints::RevoluteJointDef::new(base);
        self.create_revolute_joint(&def)
    }

    pub fn create_revolute_joint_world_id<VA: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> JointId {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        let aw: ffi::b2Vec2 = anchor_world.into().into();
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
            )
            .build();
        let def = crate::joints::RevoluteJointDef::new(base);
        self.create_revolute_joint_id(&def)
    }

    pub fn create_prismatic_joint_world<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::PrismaticJointDef::new(base);
        self.create_prismatic_joint(&def)
    }

    pub fn create_prismatic_joint_world_id<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: ffi::b2BodyId,
        body_b: ffi::b2BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::PrismaticJointDef::new(base);
        self.create_prismatic_joint_id(&def)
    }

    pub fn create_wheel_joint_world<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::WheelJointDef::new(base);
        self.create_wheel_joint(&def)
    }

    pub fn create_wheel_joint_world_id<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: ffi::b2BodyId,
        body_b: ffi::b2BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::WheelJointDef::new(base);
        self.create_wheel_joint_id(&def)
    }

    /// Helper: build a joint base from two world anchor points.
    /// Build `JointBase` from two world anchor points.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let a = world.create_body_id(BodyBuilder::new().position([-1.0,2.0]).build());
    /// let b = world.create_body_id(BodyBuilder::new().position([ 1.0,2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
    /// let base = world.joint_base_from_world_points(a, b, world.body_position(a), world.body_position(b));
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_points<VA: Into<Vec2>, VB: Into<Vec2>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
    ) -> crate::joints::JointBase {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        // Build JointBase from two world anchor points.
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
            )
            .build()
    }

    /// Helper: build a joint base from two world anchors and a shared world axis (X-axis of joint frames).
    /// Build `JointBase` from world anchors and a shared world axis (X-axis of local frames).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let a = world.create_body_id(BodyBuilder::new().position([0.0,2.0]).build());
    /// let b = world.create_body_id(BodyBuilder::new().position([1.0,2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
    /// let axis = Vec2::new(1.0, 0.0);
    /// let base = world.joint_base_from_world_with_axis(a, b, world.body_position(a), world.body_position(b), axis);
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_with_axis<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::JointBase {
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
        // Build JointBase from world anchors and a shared world axis (X-axis of local frames).
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build()
    }

    // ID-based shape helpers (world-anchored)
    impl_world_shape_create_methods! {
        create_circle_shape_for,
        create_circle_shape_for_owned,
        c,
        crate::shapes::Circle,
        ffi::b2CreateCircleShape;
        create_segment_shape_for,
        create_segment_shape_for_owned,
        s,
        crate::shapes::Segment,
        ffi::b2CreateSegmentShape;
        create_capsule_shape_for,
        create_capsule_shape_for_owned,
        c,
        crate::shapes::Capsule,
        ffi::b2CreateCapsuleShape;
        create_polygon_shape_for,
        create_polygon_shape_for_owned,
        p,
        crate::shapes::Polygon,
        ffi::b2CreatePolygonShape;
    }
    pub fn destroy_shape_id(&mut self, shape: ShapeId, update_body_mass: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Shape_IsValid(shape) } {
            unsafe { ffi::b2DestroyShape(shape, update_body_mass) };
            let _ = self.core.clear_shape_user_data(shape);
        }
        #[cfg(feature = "serialize")]
        {
            self.core.remove_shape_flags(shape);
        }
    }

    // Chain API (ID-style)
    pub fn create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> ChainId {
        crate::core::debug_checks::assert_body_valid(body);
        crate::shapes::chain::assert_chain_def_valid(def);
        let cid = unsafe { ffi::b2CreateChain(body, &def.def) };
        #[cfg(feature = "serialize")]
        {
            let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(body, def);
            self.core.record_chain(cid, meta);
        }
        cid
    }

    pub fn try_create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<ChainId> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::shapes::chain::check_chain_def_valid(def)?;
        let cid = unsafe { ffi::b2CreateChain(body, &def.def) };
        #[cfg(feature = "serialize")]
        {
            let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(body, def);
            self.core.record_chain(cid, meta);
        }
        Ok(cid)
    }

    pub fn create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::shapes::chain::OwnedChain {
        let cid = self.create_chain_for_id(body, def);
        crate::shapes::chain::OwnedChain::new(self.core_arc(), cid)
    }

    pub fn try_create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<crate::shapes::chain::OwnedChain> {
        let cid = self.try_create_chain_for_id(body, def)?;
        Ok(crate::shapes::chain::OwnedChain::new(self.core_arc(), cid))
    }

    pub fn destroy_chain_id(&mut self, chain: ChainId) {
        crate::core::debug_checks::assert_chain_valid(chain);
        if unsafe { ffi::b2Chain_IsValid(chain) } {
            unsafe { ffi::b2DestroyChain(chain) };
        }
        #[cfg(feature = "serialize")]
        {
            self.core.remove_chain(chain);
        }
    }

    pub fn try_destroy_chain_id(&mut self, chain: ChainId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_chain_valid(chain)?;
        unsafe { ffi::b2DestroyChain(chain) };
        #[cfg(feature = "serialize")]
        {
            self.core.remove_chain(chain);
        }
        Ok(())
    }

    // Shape helpers (ID-style)
    impl_world_shape_set_methods! {
        shape_set_circle,
        try_shape_set_circle,
        c,
        crate::shapes::Circle,
        ffi::b2Shape_SetCircle;
        shape_set_segment,
        try_shape_set_segment,
        s,
        crate::shapes::Segment,
        ffi::b2Shape_SetSegment;
        shape_set_capsule,
        try_shape_set_capsule,
        c,
        crate::shapes::Capsule,
        ffi::b2Shape_SetCapsule;
        shape_set_polygon,
        try_shape_set_polygon,
        p,
        crate::shapes::Polygon,
        ffi::b2Shape_SetPolygon;
    }

    pub fn shape_surface_material(&self, shape: ShapeId) -> SurfaceMaterial {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_surface_material_impl(shape)
    }

    pub fn try_shape_surface_material(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<SurfaceMaterial> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_surface_material_impl(shape))
    }

    pub fn shape_set_surface_material(&mut self, shape: ShapeId, material: &SurfaceMaterial) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_set_surface_material_impl(shape, material)
    }

    pub fn try_shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &SurfaceMaterial,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_set_surface_material_impl(shape, material);
        Ok(())
    }

    pub fn shape_body_id(&self, shape: ShapeId) -> BodyId {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_body_id_impl(shape)
    }

    pub fn try_shape_body_id(&self, shape: ShapeId) -> crate::error::ApiResult<BodyId> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_body_id_impl(shape))
    }

    pub fn shape_aabb(&self, shape: ShapeId) -> Aabb {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_aabb_impl(shape)
    }

    pub fn try_shape_aabb(&self, shape: ShapeId) -> crate::error::ApiResult<Aabb> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_aabb_impl(shape))
    }

    pub fn shape_test_point<V: Into<Vec2>>(&self, shape: ShapeId, point: V) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_test_point_impl(shape, point)
    }

    pub fn try_shape_test_point<V: Into<Vec2>>(
        &self,
        shape: ShapeId,
        point: V,
    ) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_test_point_impl(shape, point))
    }

    pub fn shape_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_ray_cast_impl(shape, origin, translation)
    }

    pub fn try_shape_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: VO,
        translation: VT,
    ) -> crate::error::ApiResult<CastOutput> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_ray_cast_impl(
            shape,
            origin,
            translation,
        ))
    }

    /// Return the closest point on a shape to `target` (in world coordinates).
    pub fn shape_closest_point<V: Into<Vec2>>(&self, shape: ShapeId, target: V) -> Vec2 {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_closest_point_impl(shape, target)
    }

    pub fn try_shape_closest_point<V: Into<Vec2>>(
        &self,
        shape: ShapeId,
        target: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_closest_point_impl(shape, target))
    }

    /// Apply wind force/torque approximation to a shape.
    pub fn shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_apply_wind_impl(shape, wind, drag, lift, wake)
    }

    pub fn try_shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_apply_wind_impl(shape, wind, drag, lift, wake);
        Ok(())
    }

    pub fn shape_mass_data(&self, shape: ShapeId) -> MassData {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_mass_data_impl(shape)
    }

    pub fn try_shape_mass_data(&self, shape: ShapeId) -> crate::error::ApiResult<MassData> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_mass_data_impl(shape))
    }

    pub fn shape_enable_sensor_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_sensor_events_impl(shape, flag)
    }

    pub fn try_shape_enable_sensor_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_sensor_events_impl(shape, flag);
        Ok(())
    }

    pub fn shape_sensor_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_events_enabled_impl(shape)
    }

    pub fn try_shape_sensor_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_events_enabled_impl(shape))
    }

    pub fn shape_enable_contact_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_contact_events_impl(shape, flag)
    }

    pub fn try_shape_enable_contact_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_contact_events_impl(shape, flag);
        Ok(())
    }

    pub fn shape_contact_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_contact_events_enabled_impl(shape)
    }

    pub fn try_shape_contact_events_enabled(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_contact_events_enabled_impl(shape))
    }

    pub fn shape_enable_pre_solve_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_pre_solve_events_impl(shape, flag)
    }

    pub fn try_shape_enable_pre_solve_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_pre_solve_events_impl(shape, flag);
        Ok(())
    }

    pub fn shape_pre_solve_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_pre_solve_events_enabled_impl(shape)
    }

    pub fn try_shape_pre_solve_events_enabled(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_pre_solve_events_enabled_impl(shape))
    }

    pub fn shape_enable_hit_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_hit_events_impl(shape, flag)
    }

    pub fn try_shape_enable_hit_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_hit_events_impl(shape, flag);
        Ok(())
    }

    pub fn shape_hit_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_hit_events_enabled_impl(shape)
    }

    pub fn try_shape_hit_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_hit_events_enabled_impl(shape))
    }

    // Sensor helpers (ID-style)
    /// Get the maximum capacity required to retrieve sensor overlaps for a shape id.
    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_capacity_impl(shape)
    }

    pub fn try_shape_sensor_capacity(&self, shape: ShapeId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_capacity_impl(shape))
    }

    /// Get overlapped shapes for a sensor shape id. Returns empty if not a sensor.
    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_impl(shape)
    }

    pub fn shape_sensor_overlaps_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_into_impl(shape, out);
    }

    pub fn try_shape_sensor_overlaps(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_overlaps_impl(shape))
    }

    pub fn try_shape_sensor_overlaps_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_sensor_overlaps_into_impl(shape, out);
        Ok(())
    }

    /// Get overlapped shapes for a sensor shape id, filtered to valid (non-destroyed) ids.
    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_valid_impl(shape)
    }

    pub fn try_shape_sensor_overlaps_valid(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_overlaps_valid_impl(shape))
    }

    pub fn shape_sensor_overlaps_valid_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_valid_into_impl(shape, out);
    }

    pub fn try_shape_sensor_overlaps_valid_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_sensor_overlaps_valid_into_impl(shape, out);
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedHandleCounts {
    pub bodies: usize,
    pub shapes: usize,
    pub joints: usize,
    pub chains: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct OutstandingOwnedHandles {
    /// `Arc` strong count of the internal world core, including the `World` itself.
    pub strong_count: usize,
    pub counts: OwnedHandleCounts,
}

impl OutstandingOwnedHandles {
    pub fn total(&self) -> usize {
        self.counts.bodies + self.counts.shapes + self.counts.joints + self.counts.chains
    }
}

/// Simulation counters providing size and internal stats.
#[derive(Clone, Debug)]
pub struct Counters {
    pub body_count: i32,
    pub shape_count: i32,
    pub contact_count: i32,
    pub joint_count: i32,
    pub island_count: i32,
    pub stack_used: i32,
    pub static_tree_height: i32,
    pub tree_height: i32,
    pub byte_count: i32,
    pub task_count: i32,
    pub color_counts: [i32; 24],
}

impl Counters {
    #[inline]
    pub fn from_raw(raw: ffi::b2Counters) -> Self {
        Self {
            body_count: raw.bodyCount,
            shape_count: raw.shapeCount,
            contact_count: raw.contactCount,
            joint_count: raw.jointCount,
            island_count: raw.islandCount,
            stack_used: raw.stackUsed,
            static_tree_height: raw.staticTreeHeight,
            tree_height: raw.treeHeight,
            byte_count: raw.byteCount,
            task_count: raw.taskCount,
            color_counts: raw.colorCounts,
        }
    }
}

/// Simulation profile timings in milliseconds for the last completed world step.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Profile {
    pub step: f32,
    pub pairs: f32,
    pub collide: f32,
    pub solve: f32,
    pub prepare_stages: f32,
    pub solve_constraints: f32,
    pub prepare_constraints: f32,
    pub integrate_velocities: f32,
    pub warm_start: f32,
    pub solve_impulses: f32,
    pub integrate_positions: f32,
    pub relax_impulses: f32,
    pub apply_restitution: f32,
    pub store_impulses: f32,
    pub split_islands: f32,
    pub transforms: f32,
    pub sensor_hits: f32,
    pub joint_events: f32,
    pub hit_events: f32,
    pub refit: f32,
    pub bullets: f32,
    pub sleep_islands: f32,
    pub sensors: f32,
}

impl Profile {
    #[inline]
    pub fn from_raw(raw: ffi::b2Profile) -> Self {
        Self {
            step: raw.step,
            pairs: raw.pairs,
            collide: raw.collide,
            solve: raw.solve,
            prepare_stages: raw.prepareStages,
            solve_constraints: raw.solveConstraints,
            prepare_constraints: raw.prepareConstraints,
            integrate_velocities: raw.integrateVelocities,
            warm_start: raw.warmStart,
            solve_impulses: raw.solveImpulses,
            integrate_positions: raw.integratePositions,
            relax_impulses: raw.relaxImpulses,
            apply_restitution: raw.applyRestitution,
            store_impulses: raw.storeImpulses,
            split_islands: raw.splitIslands,
            transforms: raw.transforms,
            sensor_hits: raw.sensorHits,
            joint_events: raw.jointEvents,
            hit_events: raw.hitEvents,
            refit: raw.refit,
            bullets: raw.bullets,
            sleep_islands: raw.sleepIslands,
            sensors: raw.sensors,
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2Profile {
        ffi::b2Profile {
            step: self.step,
            pairs: self.pairs,
            collide: self.collide,
            solve: self.solve,
            prepareStages: self.prepare_stages,
            solveConstraints: self.solve_constraints,
            prepareConstraints: self.prepare_constraints,
            integrateVelocities: self.integrate_velocities,
            warmStart: self.warm_start,
            solveImpulses: self.solve_impulses,
            integratePositions: self.integrate_positions,
            relaxImpulses: self.relax_impulses,
            applyRestitution: self.apply_restitution,
            storeImpulses: self.store_impulses,
            splitIslands: self.split_islands,
            transforms: self.transforms,
            sensorHits: self.sensor_hits,
            jointEvents: self.joint_events,
            hitEvents: self.hit_events,
            refit: self.refit,
            bullets: self.bullets,
            sleepIslands: self.sleep_islands,
            sensors: self.sensors,
        }
    }
}
#[cfg(feature = "serialize")]
impl World {
    fn record_shape_flags(&mut self, sid: ffi::b2ShapeId, def: &ffi::b2ShapeDef) {
        self.core.record_shape_flags(sid, def);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_world_runtime_extras_return_in_callback() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let handle = world.handle();
        let explosion = crate::ExplosionDef::new()
            .position([0.0_f32, 0.0])
            .radius(1.0)
            .falloff(0.5)
            .impulse_per_length(2.0);

        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world.try_enable_sleeping(false).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_is_sleeping_enabled().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_gravity().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_counters().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_profile().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_awake_body_count().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_is_sleeping_enabled().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_is_continuous_enabled().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_is_warm_starting_enabled().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_restitution_threshold().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_hit_event_threshold().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            handle.try_maximum_linear_speed().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_profile().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_enable_speculative(true).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_explode(&explosion).unwrap_err(),
            crate::ApiError::InCallback
        );
    }

    #[test]
    fn try_world_callback_sensitive_entrypoints_return_in_callback() {
        struct NoopDrawer;

        impl crate::DebugDraw for NoopDrawer {}

        impl crate::debug_draw::RawDebugDraw for NoopDrawer {
            fn draw_polygon(
                &mut self,
                _vertices: &[boxdd_sys::ffi::b2Vec2],
                _color: crate::HexColor,
            ) {
            }
        }

        let mut world = World::new(WorldDef::default()).unwrap();
        let mut cmds = Vec::new();
        let mut drawer = NoopDrawer;
        let mut raw_drawer = NoopDrawer;
        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world.try_step(1.0 / 60.0, 1).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_flush_deferred_destroys().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .try_debug_draw_collect(crate::DebugDrawOptions::default())
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .try_debug_draw_collect_into(&mut cmds, crate::DebugDrawOptions::default())
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .try_debug_draw(&mut drawer, crate::DebugDrawOptions::default())
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .try_debug_draw_raw(&mut raw_drawer, crate::DebugDrawOptions::default())
                .unwrap_err(),
            crate::ApiError::InCallback
        );
    }

    #[test]
    fn try_world_callback_registration_returns_in_callback() {
        fn always_true_filter(_a: ShapeId, _b: ShapeId) -> bool {
            true
        }

        fn always_true_pre(
            _a: ShapeId,
            _b: ShapeId,
            _p: crate::types::Vec2,
            _n: crate::types::Vec2,
        ) -> bool {
            true
        }

        let mut world = World::new(WorldDef::default()).unwrap();
        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world
                .try_set_custom_filter_with_ctx(|_, _, _| true)
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_set_custom_filter(always_true_filter).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_clear_custom_filter().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .try_set_custom_filter_callback(Some(always_true_filter))
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_set_custom_filter_callback(None).unwrap_err(),
            crate::ApiError::InCallback
        );

        assert_eq!(
            world
                .try_set_pre_solve_with_ctx(|_, _, _, _, _| true)
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_set_pre_solve(always_true_pre).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_clear_pre_solve().unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .try_set_pre_solve_callback(Some(always_true_pre))
                .unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world.try_set_pre_solve_callback(None).unwrap_err(),
            crate::ApiError::InCallback
        );
    }
}
