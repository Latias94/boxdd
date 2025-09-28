//! Joint builders and creation helpers (modularized).
//!
//! Two creation styles are available:
//! - RAII wrappers: `World::create_*_joint(&def) -> Joint` returning a scoped wrapper that destroys
//!   the underlying joint on drop.
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

pub use base::{Joint, JointBase, JointBaseBuilder};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use std::marker::PhantomData;

use crate::types::{BodyId, JointId};
use crate::world::World;
use boxdd_sys::ffi;

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

// Creation/destroy: RAII and ID style
impl World {
    pub fn create_distance_joint<'w>(&'w mut self, def: &DistanceJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_distance_joint_id(&mut self, def: &DistanceJointDef) -> JointId {
        unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) }
    }

    pub fn create_revolute_joint<'w>(&'w mut self, def: &RevoluteJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> JointId {
        unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) }
    }

    pub fn create_prismatic_joint<'w>(&'w mut self, def: &PrismaticJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> JointId {
        unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) }
    }

    pub fn create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_wheel_joint_id(&mut self, def: &WheelJointDef) -> JointId {
        unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) }
    }

    pub fn create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_weld_joint_id(&mut self, def: &WeldJointDef) -> JointId {
        unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) }
    }

    pub fn create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_motor_joint_id(&mut self, def: &MotorJointDef) -> JointId {
        unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) }
    }

    pub fn create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_filter_joint_id(&mut self, def: &FilterJointDef) -> JointId {
        unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) }
    }

    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        if unsafe { ffi::b2Joint_IsValid(id) } {
            unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
        }
    }
}

// Runtime joint control APIs (by joint type)
impl World {
    // Distance joint
    #[inline]
    pub fn distance_set_length(&mut self, id: JointId, length: f32) {
        unsafe { ffi::b2DistanceJoint_SetLength(id, length) }
    }
    #[inline]
    pub fn distance_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2DistanceJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2DistanceJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn distance_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2DistanceJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn distance_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2DistanceJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn distance_set_length_range(&mut self, id: JointId, min_length: f32, max_length: f32) {
        unsafe { ffi::b2DistanceJoint_SetLengthRange(id, min_length, max_length) }
    }
    #[inline]
    pub fn distance_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2DistanceJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn distance_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2DistanceJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn distance_set_max_motor_force(&mut self, id: JointId, force: f32) {
        unsafe { ffi::b2DistanceJoint_SetMaxMotorForce(id, force) }
    }

    // Prismatic joint
    #[inline]
    pub fn prismatic_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2PrismaticJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2PrismaticJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn prismatic_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2PrismaticJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn prismatic_set_target_translation(&mut self, id: JointId, translation: f32) {
        unsafe { ffi::b2PrismaticJoint_SetTargetTranslation(id, translation) }
    }
    #[inline]
    pub fn prismatic_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2PrismaticJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn prismatic_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        unsafe { ffi::b2PrismaticJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn prismatic_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2PrismaticJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2PrismaticJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) {
        unsafe { ffi::b2PrismaticJoint_SetMaxMotorForce(id, force) }
    }

    // Revolute joint
    #[inline]
    pub fn revolute_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2RevoluteJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2RevoluteJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn revolute_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2RevoluteJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn revolute_set_target_angle(&mut self, id: JointId, angle: f32) {
        unsafe { ffi::b2RevoluteJoint_SetTargetAngle(id, angle) }
    }
    #[inline]
    pub fn revolute_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2RevoluteJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn revolute_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        unsafe { ffi::b2RevoluteJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn revolute_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2RevoluteJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn revolute_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2RevoluteJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        unsafe { ffi::b2RevoluteJoint_SetMaxMotorTorque(id, torque) }
    }

    // Weld joint
    #[inline]
    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2WeldJoint_SetLinearHertz(id, hertz) }
    }
    #[inline]
    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2WeldJoint_SetLinearDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2WeldJoint_SetAngularHertz(id, hertz) }
    }
    #[inline]
    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2WeldJoint_SetAngularDampingRatio(id, damping_ratio) }
    }

    // Wheel joint
    #[inline]
    pub fn wheel_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2WheelJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2WheelJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn wheel_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2WheelJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn wheel_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2WheelJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        unsafe { ffi::b2WheelJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn wheel_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2WheelJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn wheel_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2WheelJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        unsafe { ffi::b2WheelJoint_SetMaxMotorTorque(id, torque) }
    }

    // Motor joint
    #[inline]
    pub fn motor_set_linear_velocity<V: Into<crate::types::Vec2>>(&mut self, id: JointId, v: V) {
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2MotorJoint_SetLinearVelocity(id, vv) }
    }
    #[inline]
    pub fn motor_set_angular_velocity(&mut self, id: JointId, w: f32) {
        unsafe { ffi::b2MotorJoint_SetAngularVelocity(id, w) }
    }
    #[inline]
    pub fn motor_set_max_velocity_force(&mut self, id: JointId, f: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxVelocityForce(id, f) }
    }
    #[inline]
    pub fn motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxVelocityTorque(id, t) }
    }
    #[inline]
    pub fn motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2MotorJoint_SetLinearHertz(id, hertz) }
    }
    #[inline]
    pub fn motor_set_linear_damping_ratio(&mut self, id: JointId, damping: f32) {
        unsafe { ffi::b2MotorJoint_SetLinearDampingRatio(id, damping) }
    }
    #[inline]
    pub fn motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2MotorJoint_SetAngularHertz(id, hertz) }
    }
    #[inline]
    pub fn motor_set_angular_damping_ratio(&mut self, id: JointId, damping: f32) {
        unsafe { ffi::b2MotorJoint_SetAngularDampingRatio(id, damping) }
    }
    #[inline]
    pub fn motor_set_max_spring_force(&mut self, id: JointId, f: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxSpringForce(id, f) }
    }
    #[inline]
    pub fn motor_set_max_spring_torque(&mut self, id: JointId, t: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxSpringTorque(id, t) }
    }
}
