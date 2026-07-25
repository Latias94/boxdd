use crate::body::{Body, BodyDef, BodyType};
#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{CustomFilterCtx, MaterialMixCtx, PreSolveCtx};
use crate::core::world_core::WorldCore;
use crate::id::{IdBrand, RawBodyId, RawChainId, RawContactId, RawJointId, RawShapeId, WorldToken};
use crate::query::Aabb;
use crate::shapes::{ShapeDef, SurfaceMaterial};
use crate::types::{
    BodyId, ChainId, JointId, MassData, MotionLocks, Position, ShapeId, Vec2, WorldCastOutput,
    WorldTransform,
};
use boxdd_sys::ffi;
use std::rc::Rc;

mod body_api;
mod borrow;
mod creation;
mod definition;
mod handle;
mod metrics;
mod runtime;
mod shape_api;

pub(crate) use body_api::{
    try_body_apply_angular_impulse_with_access, try_body_apply_force_to_center_with_access,
    try_body_apply_force_with_access, try_body_apply_linear_impulse_to_center_with_access,
    try_body_apply_linear_impulse_with_access, try_body_apply_mass_from_shapes_with_access,
    try_body_apply_torque_with_access, try_body_clear_forces_with_access,
    try_body_disable_with_access, try_body_enable_contact_events_with_access,
    try_body_enable_contact_recycling_with_access, try_body_enable_hit_events_with_access,
    try_body_enable_sleep_with_access, try_body_enable_with_access,
    try_body_set_angular_damping_with_access, try_body_set_awake_with_access,
    try_body_set_bullet_with_access, try_body_set_gravity_scale_with_access,
    try_body_set_linear_damping_with_access, try_body_set_mass_data_with_access,
    try_body_set_motion_locks_with_access, try_body_set_name_with_access,
    try_body_set_sleep_threshold_with_access, try_body_wake_touching_with_access,
    try_set_body_angular_velocity_with_access, try_set_body_linear_velocity_with_access,
    try_set_body_position_and_rotation_with_access, try_set_body_target_transform_with_access,
    try_set_body_type_with_access,
};
pub(crate) use creation::try_create_body_id_with_access;
pub(crate) use shape_api::{
    try_world_shape_set_capsule_with_access, try_world_shape_set_circle_with_access,
    try_world_shape_set_polygon_with_access, try_world_shape_set_segment_with_access,
    try_world_shape_set_surface_material_with_access,
};

pub use definition::{Error, WorldBuilder, WorldDef};
pub(crate) use definition::{
    assert_non_negative_finite_world_scalar, assert_positive_finite_world_scalar,
    assert_world_gravity_valid, check_non_negative_finite_world_scalar,
    check_positive_finite_world_scalar, check_world_gravity_valid,
};
pub use handle::WorldHandle;
pub use metrics::{
    B2_MAX_WORKERS, Counters, OwnedHandleCounts, Profile, WorkerCount, WorldCapacity,
};
pub use runtime::MaterialMixInput;
pub(crate) use runtime::{
    try_world_awake_body_count_impl, try_world_counters_impl,
    try_world_enable_continuous_with_access, try_world_enable_sleeping_with_access,
    try_world_enable_warm_starting_with_access, try_world_gravity_impl,
    try_world_hit_event_threshold_impl, try_world_is_continuous_enabled_impl,
    try_world_is_sleeping_enabled_impl, try_world_is_warm_starting_enabled_impl,
    try_world_maximum_linear_speed_impl, try_world_profile_impl,
    try_world_restitution_threshold_impl, try_world_set_contact_recycle_distance_with_access,
    try_world_set_contact_tuning_with_access, try_world_set_gravity_with_access,
    try_world_set_hit_event_threshold_with_access, try_world_set_maximum_linear_speed_with_access,
    try_world_set_restitution_threshold_with_access, world_awake_body_count_checked_impl,
    world_counters_checked_impl, world_gravity_checked_impl,
    world_hit_event_threshold_checked_impl, world_is_continuous_enabled_checked_impl,
    world_is_sleeping_enabled_checked_impl, world_is_warm_starting_enabled_checked_impl,
    world_maximum_linear_speed_checked_impl, world_profile_checked_impl,
    world_restitution_threshold_checked_impl,
};

#[inline]
fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
fn raw_shape_id(id: ShapeId) -> ffi::b2ShapeId {
    id.into_raw()
}

#[inline]
#[track_caller]
pub(crate) fn assert_world_available(core: &WorldCore) {
    crate::core::callback_state::assert_not_in_callback();
    core.check_available()
        .expect("world must be idle, live, and not poisoned");
}

#[inline]
pub(crate) fn check_world_available(core: &WorldCore) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_available()
}

#[inline]
pub(crate) fn check_recording_world_available(core: &WorldCore) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_recording_available()
}

/// A simulation world.
///
/// Dropping `World` ends the underlying Box2D world's lifetime. Handles may retain an inert Rust
/// shell afterward so their fallible methods can report [`crate::ApiError::WorldDestroyed`].
pub struct World {
    core: Rc<WorldCore>,
    events: Rc<crate::events::EventCache>,
    uses_raw_task_system: bool,
}
impl World {
    /// Create a world from a definition.
    pub fn new(def: WorldDef) -> Result<Self, Error> {
        crate::core::callback_state::assert_not_in_callback();
        def.validate()?;
        let uses_raw_task_system = def.has_task_system_raw();
        let friction_mixer_present = def.0.frictionCallback.is_some();
        let restitution_mixer_present = def.0.restitutionCallback.is_some();
        let token = WorldToken::allocate().map_err(|_| Error::IdentityExhausted)?;
        let foundation_lease = crate::core::foundation::acquire_ordinary_world_lease()?;
        let world_slot_guard = crate::core::foundation::lock_world_slot_mutation();
        let raw = def.into_raw();
        // SAFETY: FFI call to create a world; returns an id handle
        let world_id = unsafe { ffi::b2CreateWorld(&raw) };
        let ok = unsafe { ffi::b2World_IsValid(world_id) };
        if ok {
            let brand = IdBrand::new(world_id, token).map_err(|_| {
                unsafe { ffi::b2DestroyWorld(world_id) };
                Error::CreateFailed
            })?;
            drop(world_slot_guard);
            Ok(Self {
                core: WorldCore::new(
                    world_id,
                    brand,
                    foundation_lease,
                    friction_mixer_present,
                    restitution_mixer_present,
                ),
                events: Rc::new(crate::events::EventCache::default()),
                uses_raw_task_system,
            })
        } else {
            drop(world_slot_guard);
            Err(Error::CreateFailed)
        }
    }

    /// Expose the raw Box2D world id for advanced use-cases.
    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        assert_world_available(&self.core);
        self.core.id
    }

    pub(crate) fn raw(&self) -> ffi::b2WorldId {
        self.core.id
    }

    pub(crate) fn brand(&self) -> IdBrand {
        WorldCore::brand(&self.core)
    }

    pub(crate) fn core(&self) -> &WorldCore {
        &self.core
    }

    pub(crate) fn event_cache(&self) -> &crate::events::EventCache {
        &self.events
    }

    pub(crate) const fn uses_raw_task_system(&self) -> bool {
        self.uses_raw_task_system
    }

    pub(crate) fn query_target(&self) -> crate::query::QueryTarget {
        crate::query::QueryTarget::new(self.core_rc())
    }

    /// Validate and bind an unbound body identifier to this world.
    pub fn bind_body_id(&self, raw: RawBodyId) -> crate::error::ApiResult<BodyId> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.bind_body(raw)
    }

    /// Validate and bind an unbound shape identifier to this world.
    pub fn bind_shape_id(&self, raw: RawShapeId) -> crate::error::ApiResult<ShapeId> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.bind_shape(raw)
    }

    /// Validate and bind an unbound joint identifier to this world.
    pub fn bind_joint_id(&self, raw: RawJointId) -> crate::error::ApiResult<JointId> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.bind_joint(raw)
    }

    /// Validate and bind an unbound chain identifier to this world.
    pub fn bind_chain_id(&self, raw: RawChainId) -> crate::error::ApiResult<ChainId> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.bind_chain(raw)
    }

    /// Validate and bind an unbound contact identifier to this world.
    pub fn bind_contact_id(&self, raw: RawContactId) -> crate::error::ApiResult<crate::ContactId> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.bind_contact(raw)
    }

    /// Return whether Box2D still recognizes this world's native id.
    pub fn is_valid(&self) -> bool {
        assert_world_available(&self.core);
        unsafe { ffi::b2World_IsValid(self.raw()) }
    }

    /// Return whether Box2D still recognizes this world's native id.
    pub fn try_is_valid(&self) -> crate::error::ApiResult<bool> {
        check_world_available(&self.core)?;
        Ok(unsafe { ffi::b2World_IsValid(self.raw()) })
    }

    pub(crate) fn core_rc(&self) -> Rc<WorldCore> {
        Rc::clone(&self.core)
    }

    pub(crate) fn from_restored_core(core: Rc<WorldCore>) -> Self {
        Self {
            core,
            events: Rc::new(crate::events::EventCache::default()),
            uses_raw_task_system: false,
        }
    }

    pub(crate) fn with_borrowed_event_buffers<T>(&self, f: impl FnOnce() -> T) -> T {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .check_available()
            .expect("world is not available for event access");
        let core = self.core_rc();
        let owner_scope = crate::core::callback_state::OwnerCallScope::enter();
        let result = {
            let _borrow = core.borrow_event_buffers();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        };
        // Nested raw/view event borrows are allowed. Deferred destroys must wait until the
        // outermost borrow ends so previously returned event slices cannot be invalidated early.
        owner_scope.finish(result, [core])
    }

    pub(crate) fn try_with_borrowed_event_buffers<T>(
        &self,
        f: impl FnOnce() -> T,
    ) -> crate::error::ApiResult<T> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.check_available()?;
        Ok(self.with_borrowed_event_buffers(f))
    }

    // --- Typed user data ---------------------------------------------------------
    /// Set typed user data on this world.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the world is dropped.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        assert_world_available(&self.core);
        let update = self
            .core
            .set_world_user_data(value)
            .expect("world user data is already borrowed");
        unsafe { ffi::b2World_SetUserData(self.raw(), update.pointer()) };
        drop(update);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        let update = self.core.set_world_user_data(value)?;
        unsafe { ffi::b2World_SetUserData(self.raw(), update.pointer()) };
        drop(update);
        Ok(())
    }

    /// Return whether the world currently has a native user-data pointer.
    pub fn has_user_data(&self) -> bool {
        assert_world_available(&self.core);
        unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() }
    }

    /// Return whether the world currently has a native user-data pointer.
    pub fn try_has_user_data(&self) -> crate::error::ApiResult<bool> {
        check_world_available(&self.core)?;
        Ok(unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() })
    }

    /// Clear typed user data on this world. Returns whether any data was present.
    pub fn clear_user_data(&mut self) -> bool {
        assert_world_available(&self.core);
        let retired = self
            .core
            .clear_world_user_data()
            .expect("world user data is already borrowed");
        let had = retired.is_some() || unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() };
        if had {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        drop(retired);
        had
    }

    pub fn try_clear_user_data(&mut self) -> crate::error::ApiResult<bool> {
        check_world_available(&self.core)?;
        let retired = self.core.clear_world_user_data()?;
        let had = retired.is_some() || unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() };
        if had {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        drop(retired);
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        assert_world_available(&self.core);
        self.core
            .try_with_world_user_data(f)
            .expect("world user data access failed")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        check_world_available(&self.core)?;
        self.core.try_with_world_user_data(f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        assert_world_available(&self.core);
        let v = self
            .core
            .take_world_user_data::<T>()
            .expect("world user data access failed");
        if v.is_some() {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> crate::error::ApiResult<Option<T>> {
        check_world_available(&self.core)?;
        let v = self.core.take_world_user_data::<T>()?;
        if v.is_some() {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        Ok(v)
    }

    /// Create a cheap, cloneable handle to this world.
    pub fn handle(&self) -> WorldHandle {
        assert_world_available(&self.core);
        WorldHandle::new(Rc::clone(&self.core), Rc::clone(&self.events))
    }

    pub fn owned_handle_counts(&self) -> OwnedHandleCounts {
        assert_world_available(&self.core);
        let (bodies, shapes, joints, chains) = self.core.owned_counts();
        OwnedHandleCounts {
            bodies,
            shapes,
            joints,
            chains,
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        self.core.shutdown_native();
    }
}

#[cfg(test)]
mod availability_tests;

#[cfg(test)]
mod tests;
