use crate::body::{Body, BodyDef};
use crate::core::world_core::WorldCore;
use crate::id::{IdBrand, WorldToken};
use crate::query::Aabb;
use crate::types::{BodyId, ChainId, JointId, Position, ShapeId, Vec2};
use boxdd_sys::ffi;
use std::ops::Deref;
use std::pin::Pin;

mod borrow;
mod capability;
mod creation;
mod definition;
mod metrics;
mod runtime;

pub(crate) use capability::{
    BodyCall, BodyProof, ChainCall, ChainProof, JointCall, JointProof, OwnerAdapter, OwnerCreation,
    QueryCall, QueryCallGuard, QueryProof, ShapeCall, ShapeProof, WorldCall, joint_base_for_owner,
    run_owner_call,
};

pub(crate) use creation::create_body_id;

pub use definition::{WorldBuilder, WorldDef};
pub(crate) use definition::{
    check_non_negative_finite_world_scalar, check_positive_finite_world_linear_speed,
    check_world_gravity_valid,
};
pub use metrics::{B2_MAX_WORKERS, Counters, Profile, WorkerCount, WorldCapacity};
pub use runtime::MaterialMixInput;
pub(crate) use runtime::{
    world_enable_continuous, world_enable_sleeping, world_enable_warm_starting,
    world_set_contact_recycle_distance, world_set_contact_tuning, world_set_gravity,
    world_set_hit_event_threshold, world_set_maximum_linear_speed, world_set_restitution_threshold,
};

#[inline]
pub(crate) fn check_world_available(world: &World) -> crate::error::Result<()> {
    crate::core::callback_state::check_not_in_callback()?;
    world.retire_completed_step();
    world.core.check_available()
}

#[inline]
pub(crate) fn check_recording_world_available(world: &World) -> crate::error::Result<()> {
    crate::core::callback_state::check_not_in_callback()?;
    world.retire_completed_step();
    world.core.check_recording_available()
}

/// A simulation world.
///
/// Dropping `World` ends the underlying Box2D world's lifetime. If it is dropped from a native
/// callback, teardown is transferred to that call's outermost owner boundary so Box2D is never
/// re-entered while the world is locked. A callback with no such Rust boundary retains the native
/// owner instead of risking use-after-free.
pub struct World {
    core: WorldOwner,
    completed_step: crate::events::CompletedStepState,
}

struct WorldOwner(Option<Pin<Box<WorldCore>>>);

impl WorldOwner {
    fn new(core: Pin<Box<WorldCore>>) -> Self {
        Self(Some(core))
    }

    fn take(&mut self) -> Pin<Box<WorldCore>> {
        self.0
            .take()
            .expect("a live World owns exactly one WorldCore")
    }
}

impl Deref for WorldOwner {
    type Target = WorldCore;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_deref()
            .expect("WorldCore remains present until World::drop")
    }
}

impl World {
    pub(crate) fn create(
        foundation: &'static crate::Foundation,
        def: WorldDef,
    ) -> crate::error::Result<Self> {
        crate::core::callback_state::check_not_in_callback()?;
        def.validate()?;
        let length_scale = foundation.length_scale();
        length_scale.check_definition("Foundation::create_world", def.length_scale())?;
        let token = WorldToken::allocate().map_err(|_| crate::Error::WorldIdentityExhausted)?;
        let foundation_lease = foundation.acquire_ordinary_world_lease()?;
        let world_slot_guard = crate::core::foundation::lock_world_slot_mutation();
        let raw = def.into_raw();
        // SAFETY: the definition was validated and the process-global world slot is serialized.
        let world_id = unsafe { ffi::b2CreateWorld(&raw) };
        let ok = unsafe { ffi::b2World_IsValid(world_id) };
        if ok {
            let brand = IdBrand::new(world_id, token).map_err(|_| {
                unsafe { ffi::b2DestroyWorld(world_id) };
                crate::Error::WorldCreationFailed
            })?;
            drop(world_slot_guard);
            Ok(Self {
                core: WorldOwner::new(WorldCore::new(
                    world_id,
                    brand,
                    length_scale,
                    foundation_lease,
                )),
                completed_step: crate::events::CompletedStepState::default(),
            })
        } else {
            drop(world_slot_guard);
            Err(crate::Error::WorldCreationFailed)
        }
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

    /// Construct body defaults carrying this world's length-scale provenance.
    #[must_use]
    pub fn body_def(&self) -> BodyDef {
        BodyDef::with_length_scale(self.core.length_scale())
    }

    /// Start a body builder carrying this world's length-scale provenance.
    #[must_use]
    pub fn body_builder(&self) -> crate::BodyBuilder {
        self.body_def().into()
    }

    /// Construct a joint base after proving that both body ids belong to this live world.
    pub fn joint_base(
        &self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> crate::error::Result<crate::JointBase> {
        joint_base_for_owner(self, body_a, body_b)
    }

    #[cfg(test)]
    pub(crate) fn event_storage(&self) -> &crate::events::EventStorage {
        self.completed_step.storage()
    }

    pub(crate) fn completed_step_state(&self) -> &crate::events::CompletedStepState {
        &self.completed_step
    }

    pub(crate) fn retire_completed_step(&self) {
        self.completed_step.retire(&self.core);
    }

    pub(crate) fn invalidate_completed_step(&self) {
        self.completed_step.invalidate(&self.core);
    }

    #[cfg(test)]
    pub(crate) fn completed_step_active_for_test(&self) -> bool {
        self.completed_step.is_active_for_test()
    }

    // --- Typed user data ---------------------------------------------------------
    /// Set typed user data on this world.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the world is dropped.
    pub fn set_user_data<T: 'static>(&mut self, value: T) -> crate::error::Result<()> {
        let value = crate::core::callback_state::PendingUserValue::new(value);
        check_world_available(self)?;
        let update = self.core.set_world_user_data(value)?;
        let (pointer, retired) = update.into_parts();
        unsafe { ffi::b2World_SetUserData(self.raw(), pointer) };
        retired.resume_drop_panic();
        Ok(())
    }

    /// Return whether the world currently has a native user-data pointer.
    pub fn has_user_data(&self) -> crate::error::Result<bool> {
        check_world_available(self)?;
        Ok(unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() })
    }

    /// Clear typed user data on this world. Returns whether any data was present.
    pub fn clear_user_data(&mut self) -> crate::error::Result<bool> {
        check_world_available(self)?;
        let retired = self.core.clear_world_user_data()?;
        let had = retired.is_some() || unsafe { !ffi::b2World_GetUserData(self.raw()).is_null() };
        if had {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        retired.resume_drop_panic();
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        check_world_available(self)?;
        self.core.try_with_world_user_data(f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> crate::error::Result<Option<T>> {
        check_world_available(self)?;
        let v = self.core.take_world_user_data::<T>()?;
        if v.is_some() {
            unsafe { ffi::b2World_SetUserData(self.raw(), core::ptr::null_mut()) };
        }
        Ok(v)
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let core = self.core.take();
        // Box2D detaches an active recording as part of b2DestroyWorld. Only a callback stack makes
        // immediate native teardown unsafe; an active owner frame then performs every explicit
        // detach cleanup before destroying this transferred owner.
        if crate::core::callback_state::in_callback() {
            crate::core::callback_state::defer_world_owner_or_forget(core);
        } else {
            core.shutdown_native();
            drop(core);
        }
    }
}

#[cfg(test)]
mod availability_tests;

#[cfg(test)]
mod tests;
