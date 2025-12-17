//! Joint builders and creation helpers (modularized).
//!
//! Two creation styles are available:
//! - Scoped handles: `World::create_*_joint(&def) -> Joint` returning a scoped handle for immediate
//!   configuration/queries. Dropping the handle does **not** destroy the joint.
//! - Owned handles: `World::create_*_joint_owned(&def) -> OwnedJoint` or `World::*().build_owned() -> OwnedJoint`
//!   returning a RAII handle that destroys the joint on drop.
//! - ID style: `World::create_*_joint_id(&def) -> b2JointId` returning the raw id for storage.
//!
//! The `World` convenience builders (`revolute`, `prismatic`, `wheel`, `distance`, `weld`,
//! `motor_joint`, `filter_joint`) help compose joints in world space and build local frames
//! from world anchors/axes.

mod base;
mod distance;
mod filter;
mod motor;
mod prismatic;
mod revolute;
mod weld;
mod wheel;

pub use base::{Joint, JointBase, JointBaseBuilder, OwnedJoint};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use std::marker::PhantomData;

use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

#[inline]
fn assert_joint_valid(id: JointId) {
    crate::core::debug_checks::assert_joint_valid(id);
}

#[inline]
fn check_joint_valid(id: JointId) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)
}

#[inline]
fn assert_joint_def_bodies_valid(base: &ffi::b2JointDef) {
    crate::core::debug_checks::assert_body_valid(base.bodyIdA);
    crate::core::debug_checks::assert_body_valid(base.bodyIdB);
}

#[inline]
fn check_joint_def_bodies_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(base.bodyIdA)?;
    crate::core::debug_checks::check_body_valid(base.bodyIdB)?;
    Ok(())
}

// Convenience builder entry points on World
impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        RevoluteJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: RevoluteJointDef::new(JointBase::default()),
        }
    }
    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        PrismaticJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(JointBase::default()),
        }
    }
    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        WheelJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(JointBase::default()),
        }
    }
    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        DistanceJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(JointBase::default()),
        }
    }
    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        WeldJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: WeldJointDef::new(JointBase::default()),
        }
    }
    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        MotorJointBuilder {
            world: self,
            body_a,
            body_b,
            def: MotorJointDef::new(JointBase::default()),
        }
    }
    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        FilterJointBuilder {
            world: self,
            body_a,
            body_b,
            def: FilterJointDef::new(JointBase::default()),
        }
    }
}

// Creation/destroy: scoped handles and ID style
impl World {
    pub fn create_distance_joint<'w>(&'w mut self, def: &DistanceJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_distance_joint_id(&mut self, def: &DistanceJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) }
    }
    pub fn create_distance_joint_owned(&mut self, def: &DistanceJointDef) -> OwnedJoint {
        let id = self.create_distance_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_distance_joint<'w>(
        &'w mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_distance_joint_id(&mut self, def: &DistanceJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) })
    }
    pub fn try_create_distance_joint_owned(
        &mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<OwnedJoint> {
        let id = self.try_create_distance_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn create_revolute_joint<'w>(&'w mut self, def: &RevoluteJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) }
    }
    pub fn create_revolute_joint_owned(&mut self, def: &RevoluteJointDef) -> OwnedJoint {
        let id = self.create_revolute_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_revolute_joint<'w>(
        &'w mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) })
    }
    pub fn try_create_revolute_joint_owned(
        &mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<OwnedJoint> {
        let id = self.try_create_revolute_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn create_prismatic_joint<'w>(&'w mut self, def: &PrismaticJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) }
    }
    pub fn create_prismatic_joint_owned(&mut self, def: &PrismaticJointDef) -> OwnedJoint {
        let id = self.create_prismatic_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_prismatic_joint<'w>(
        &'w mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) })
    }
    pub fn try_create_prismatic_joint_owned(
        &mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<OwnedJoint> {
        let id = self.try_create_prismatic_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_wheel_joint_id(&mut self, def: &WheelJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) }
    }
    pub fn create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> OwnedJoint {
        let id = self.create_wheel_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_wheel_joint_id(&mut self, def: &WheelJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) })
    }
    pub fn try_create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_wheel_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_weld_joint_id(&mut self, def: &WeldJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) }
    }
    pub fn create_weld_joint_owned(&mut self, def: &WeldJointDef) -> OwnedJoint {
        let id = self.create_weld_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_weld_joint_id(&mut self, def: &WeldJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) })
    }
    pub fn try_create_weld_joint_owned(&mut self, def: &WeldJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_weld_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_motor_joint_id(&mut self, def: &MotorJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) }
    }
    pub fn create_motor_joint_owned(&mut self, def: &MotorJointDef) -> OwnedJoint {
        let id = self.create_motor_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_motor_joint_id(&mut self, def: &MotorJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) })
    }
    pub fn try_create_motor_joint_owned(&mut self, def: &MotorJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_motor_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> Joint<'w> {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        let id = unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_filter_joint_id(&mut self, def: &FilterJointDef) -> JointId {
        crate::core::callback_state::assert_not_in_callback();
        assert_joint_def_bodies_valid(&def.0.base);
        unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) }
    }
    pub fn create_filter_joint_owned(&mut self, def: &FilterJointDef) -> OwnedJoint {
        let id = self.create_filter_joint_id(def);
        OwnedJoint::new(self.core_arc(), id)
    }

    pub fn try_create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> ApiResult<Joint<'w>> {
        check_joint_def_bodies_valid(&def.0.base)?;
        let id = unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) };
        Ok(Joint {
            id,
            _world: PhantomData,
        })
    }
    pub fn try_create_filter_joint_id(&mut self, def: &FilterJointDef) -> ApiResult<JointId> {
        check_joint_def_bodies_valid(&def.0.base)?;
        Ok(unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) })
    }
    pub fn try_create_filter_joint_owned(&mut self, def: &FilterJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_filter_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_arc(), id))
    }

    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(id) } {
            unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
        }
    }

    pub fn try_destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
        Ok(())
    }
}

// Runtime joint control APIs (by joint type)
impl World {
    pub fn joint_linear_separation(&self, id: JointId) -> f32 {
        assert_joint_valid(id);
        unsafe { ffi::b2Joint_GetLinearSeparation(id) }
    }

    pub fn try_joint_linear_separation(&self, id: JointId) -> ApiResult<f32> {
        check_joint_valid(id)?;
        Ok(unsafe { ffi::b2Joint_GetLinearSeparation(id) })
    }

    pub fn joint_angular_separation(&self, id: JointId) -> f32 {
        assert_joint_valid(id);
        unsafe { ffi::b2Joint_GetAngularSeparation(id) }
    }

    pub fn try_joint_angular_separation(&self, id: JointId) -> ApiResult<f32> {
        check_joint_valid(id)?;
        Ok(unsafe { ffi::b2Joint_GetAngularSeparation(id) })
    }

    pub fn joint_constraint_force(&self, id: JointId) -> Vec2 {
        assert_joint_valid(id);
        Vec2::from(unsafe { ffi::b2Joint_GetConstraintForce(id) })
    }

    pub fn try_joint_constraint_force(&self, id: JointId) -> ApiResult<Vec2> {
        check_joint_valid(id)?;
        Ok(Vec2::from(unsafe { ffi::b2Joint_GetConstraintForce(id) }))
    }

    pub fn joint_constraint_torque(&self, id: JointId) -> f32 {
        assert_joint_valid(id);
        unsafe { ffi::b2Joint_GetConstraintTorque(id) }
    }

    pub fn try_joint_constraint_torque(&self, id: JointId) -> ApiResult<f32> {
        check_joint_valid(id)?;
        Ok(unsafe { ffi::b2Joint_GetConstraintTorque(id) })
    }

    // Distance joint
    #[inline]
    pub fn distance_set_length(&mut self, id: JointId, length: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_SetLength(id, length) }
    }
    #[inline]
    pub fn try_distance_set_length(&mut self, id: JointId, length: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_SetLength(id, length) }
        Ok(())
    }
    #[inline]
    pub fn distance_enable_spring(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn try_distance_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_EnableSpring(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn try_distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_SetSpringHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn distance_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn try_distance_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_SetSpringDampingRatio(id, damping_ratio) }
        Ok(())
    }
    #[inline]
    pub fn distance_enable_limit(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn try_distance_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_EnableLimit(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn distance_set_length_range(&mut self, id: JointId, min_length: f32, max_length: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_SetLengthRange(id, min_length, max_length) }
    }
    #[inline]
    pub fn try_distance_set_length_range(
        &mut self,
        id: JointId,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_SetLengthRange(id, min_length, max_length) }
        Ok(())
    }
    #[inline]
    pub fn distance_enable_motor(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn try_distance_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_EnableMotor(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn distance_set_motor_speed(&mut self, id: JointId, speed: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn try_distance_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_SetMotorSpeed(id, speed) }
        Ok(())
    }
    #[inline]
    pub fn distance_set_max_motor_force(&mut self, id: JointId, force: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2DistanceJoint_SetMaxMotorForce(id, force) }
    }
    #[inline]
    pub fn try_distance_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DistanceJoint_SetMaxMotorForce(id, force) }
        Ok(())
    }

    // Prismatic joint
    #[inline]
    pub fn prismatic_enable_spring(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn try_prismatic_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_EnableSpring(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn try_prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_SetSpringHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn try_prismatic_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_SetSpringDampingRatio(id, damping_ratio) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_set_target_translation(&mut self, id: JointId, translation: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_SetTargetTranslation(id, translation) }
    }
    #[inline]
    pub fn try_prismatic_set_target_translation(
        &mut self,
        id: JointId,
        translation: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_SetTargetTranslation(id, translation) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_enable_limit(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn try_prismatic_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_EnableLimit(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn try_prismatic_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_SetLimits(id, lower, upper) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_enable_motor(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn try_prismatic_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_EnableMotor(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn try_prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_SetMotorSpeed(id, speed) }
        Ok(())
    }
    #[inline]
    pub fn prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2PrismaticJoint_SetMaxMotorForce(id, force) }
    }
    #[inline]
    pub fn try_prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2PrismaticJoint_SetMaxMotorForce(id, force) }
        Ok(())
    }

    // Revolute joint
    #[inline]
    pub fn revolute_enable_spring(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn try_revolute_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_EnableSpring(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn try_revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_SetSpringHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn revolute_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn try_revolute_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_SetSpringDampingRatio(id, damping_ratio) }
        Ok(())
    }
    #[inline]
    pub fn revolute_set_target_angle(&mut self, id: JointId, angle: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_SetTargetAngle(id, angle) }
    }
    #[inline]
    pub fn try_revolute_set_target_angle(&mut self, id: JointId, angle: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_SetTargetAngle(id, angle) }
        Ok(())
    }
    #[inline]
    pub fn revolute_enable_limit(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn try_revolute_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_EnableLimit(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn revolute_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn try_revolute_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_SetLimits(id, lower, upper) }
        Ok(())
    }
    #[inline]
    pub fn revolute_enable_motor(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn try_revolute_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_EnableMotor(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn revolute_set_motor_speed(&mut self, id: JointId, speed: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn try_revolute_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_SetMotorSpeed(id, speed) }
        Ok(())
    }
    #[inline]
    pub fn revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2RevoluteJoint_SetMaxMotorTorque(id, torque) }
    }
    #[inline]
    pub fn try_revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2RevoluteJoint_SetMaxMotorTorque(id, torque) }
        Ok(())
    }

    // Weld joint
    #[inline]
    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WeldJoint_SetLinearHertz(id, hertz) }
    }
    #[inline]
    pub fn try_weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WeldJoint_SetLinearHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WeldJoint_SetLinearDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn try_weld_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WeldJoint_SetLinearDampingRatio(id, damping_ratio) }
        Ok(())
    }
    #[inline]
    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WeldJoint_SetAngularHertz(id, hertz) }
    }
    #[inline]
    pub fn try_weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WeldJoint_SetAngularHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WeldJoint_SetAngularDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn try_weld_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WeldJoint_SetAngularDampingRatio(id, damping_ratio) }
        Ok(())
    }

    // Wheel joint
    #[inline]
    pub fn wheel_enable_spring(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn try_wheel_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_EnableSpring(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn try_wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_SetSpringHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn wheel_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn try_wheel_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_SetSpringDampingRatio(id, damping_ratio) }
        Ok(())
    }
    #[inline]
    pub fn wheel_enable_limit(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn try_wheel_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_EnableLimit(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn try_wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_SetLimits(id, lower, upper) }
        Ok(())
    }
    #[inline]
    pub fn wheel_enable_motor(&mut self, id: JointId, enable: bool) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn try_wheel_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_EnableMotor(id, enable) }
        Ok(())
    }
    #[inline]
    pub fn wheel_set_motor_speed(&mut self, id: JointId, speed: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn try_wheel_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_SetMotorSpeed(id, speed) }
        Ok(())
    }
    #[inline]
    pub fn wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2WheelJoint_SetMaxMotorTorque(id, torque) }
    }
    #[inline]
    pub fn try_wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2WheelJoint_SetMaxMotorTorque(id, torque) }
        Ok(())
    }

    // Motor joint
    #[inline]
    pub fn motor_set_linear_velocity<V: Into<crate::types::Vec2>>(&mut self, id: JointId, v: V) {
        assert_joint_valid(id);
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2MotorJoint_SetLinearVelocity(id, vv) }
    }
    #[inline]
    pub fn try_motor_set_linear_velocity<V: Into<crate::types::Vec2>>(
        &mut self,
        id: JointId,
        v: V,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2MotorJoint_SetLinearVelocity(id, vv) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_angular_velocity(&mut self, id: JointId, w: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetAngularVelocity(id, w) }
    }
    #[inline]
    pub fn try_motor_set_angular_velocity(&mut self, id: JointId, w: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetAngularVelocity(id, w) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_max_velocity_force(&mut self, id: JointId, f: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetMaxVelocityForce(id, f) }
    }
    #[inline]
    pub fn try_motor_set_max_velocity_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetMaxVelocityForce(id, f) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetMaxVelocityTorque(id, t) }
    }
    #[inline]
    pub fn try_motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetMaxVelocityTorque(id, t) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetLinearHertz(id, hertz) }
    }
    #[inline]
    pub fn try_motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetLinearHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_linear_damping_ratio(&mut self, id: JointId, damping: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetLinearDampingRatio(id, damping) }
    }
    #[inline]
    pub fn try_motor_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetLinearDampingRatio(id, damping) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetAngularHertz(id, hertz) }
    }
    #[inline]
    pub fn try_motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetAngularHertz(id, hertz) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_angular_damping_ratio(&mut self, id: JointId, damping: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetAngularDampingRatio(id, damping) }
    }
    #[inline]
    pub fn try_motor_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetAngularDampingRatio(id, damping) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_max_spring_force(&mut self, id: JointId, f: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetMaxSpringForce(id, f) }
    }
    #[inline]
    pub fn try_motor_set_max_spring_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetMaxSpringForce(id, f) }
        Ok(())
    }
    #[inline]
    pub fn motor_set_max_spring_torque(&mut self, id: JointId, t: f32) {
        assert_joint_valid(id);
        unsafe { ffi::b2MotorJoint_SetMaxSpringTorque(id, t) }
    }
    #[inline]
    pub fn try_motor_set_max_spring_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2MotorJoint_SetMaxSpringTorque(id, t) }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn try_joint_apis_return_in_callback() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let a = world.create_body_id(crate::BodyBuilder::new().build());
        let b = world.create_body_id(crate::BodyBuilder::new().build());

        let def = crate::DistanceJointDef::new(
            crate::JointBaseBuilder::new()
                .bodies_by_id(a, b)
                .collide_connected(false)
                .build(),
        );

        let _g = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            world.try_create_distance_joint_id(&def).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .revolute(a, b)
                .anchor_world([0.0, 0.0])
                .try_build()
                .unwrap_err(),
            crate::ApiError::InCallback
        );
    }
}
