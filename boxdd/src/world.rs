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

mod body_api;
mod borrow;
mod creation;
mod definition;
mod handle;
mod metrics;
mod runtime;
mod shape_api;

pub use definition::{Error, WorldBuilder, WorldDef};
pub(crate) use definition::{
    assert_non_negative_finite_world_scalar, assert_positive_finite_world_scalar,
    assert_world_gravity_valid, check_non_negative_finite_world_scalar,
    check_positive_finite_world_scalar, check_world_gravity_valid,
};
pub use handle::{CallbackWorld, WorldHandle};
pub use metrics::{Counters, OutstandingOwnedHandles, OwnedHandleCounts, Profile};
pub use runtime::MaterialMixInput;
pub(crate) use runtime::{
    try_world_awake_body_count_impl, try_world_counters_impl, try_world_gravity_impl,
    try_world_hit_event_threshold_impl, try_world_is_continuous_enabled_impl,
    try_world_is_sleeping_enabled_impl, try_world_is_warm_starting_enabled_impl,
    try_world_maximum_linear_speed_impl, try_world_profile_impl,
    try_world_restitution_threshold_impl, world_awake_body_count_checked_impl,
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
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

#[inline]
fn raw_chain_id(id: ChainId) -> ffi::b2ChainId {
    id.into_raw()
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
#[cfg(feature = "serialize")]
pub use crate::core::serialize_registry::{
    ChainCreateRecord, ChainMaterialsRecord, ShapeFlagsRecord,
};

impl World {
    /// Create a world from a definition.
    pub fn new(def: WorldDef) -> Result<Self, Error> {
        def.validate()?;
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
        WorldHandle::new(Arc::clone(&self.core))
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
}

#[cfg(test)]
mod tests;
