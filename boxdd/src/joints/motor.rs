use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint};
use crate::error::ApiResult;

// Motor joint
#[derive(Clone, Debug)]
/// Motor joint definition (maps to `b2MotorJointDef`). Drives relative motion
/// between two bodies using linear and angular offsets with configurable
/// maximum forces.
pub struct MotorJointDef {
    base: JointBase,
    linear_velocity: crate::types::Vec2,
    max_velocity_force: f32,
    angular_velocity: f32,
    max_velocity_torque: f32,
    linear_hertz: f32,
    linear_damping_ratio: f32,
    max_spring_force: f32,
    angular_hertz: f32,
    angular_damping_ratio: f32,
    max_spring_torque: f32,
}

impl MotorJointDef {
    pub fn new(base: JointBase) -> Self {
        let _lease = crate::core::foundation::assert_transient_native_lease();
        let raw = unsafe { ffi::b2DefaultMotorJointDef() };
        Self {
            base,
            linear_velocity: crate::types::Vec2::from_raw(raw.linearVelocity),
            max_velocity_force: raw.maxVelocityForce,
            angular_velocity: raw.angularVelocity,
            max_velocity_torque: raw.maxVelocityTorque,
            linear_hertz: raw.linearHertz,
            linear_damping_ratio: raw.linearDampingRatio,
            max_spring_force: raw.maxSpringForce,
            angular_hertz: raw.angularHertz,
            angular_damping_ratio: raw.angularDampingRatio,
            max_spring_torque: raw.maxSpringTorque,
        }
    }

    #[inline]
    pub fn base(&self) -> &JointBase {
        &self.base
    }

    #[inline]
    pub(crate) fn base_mut(&mut self) -> &mut JointBase {
        &mut self.base
    }

    #[inline]
    pub(crate) fn to_raw(&self) -> ffi::b2MotorJointDef {
        let mut raw = unsafe { ffi::b2DefaultMotorJointDef() };
        raw.base = self.base.to_raw();
        raw.linearVelocity = self.linear_velocity.into_raw();
        raw.maxVelocityForce = self.max_velocity_force;
        raw.angularVelocity = self.angular_velocity;
        raw.maxVelocityTorque = self.max_velocity_torque;
        raw.linearHertz = self.linear_hertz;
        raw.linearDampingRatio = self.linear_damping_ratio;
        raw.maxSpringForce = self.max_spring_force;
        raw.angularHertz = self.angular_hertz;
        raw.angularDampingRatio = self.angular_damping_ratio;
        raw.maxSpringTorque = self.max_spring_torque;
        raw
    }

    #[inline]
    pub fn target_linear_velocity(&self) -> crate::types::Vec2 {
        self.linear_velocity
    }

    #[inline]
    pub fn maximum_velocity_force(&self) -> f32 {
        self.max_velocity_force
    }

    #[inline]
    pub fn target_angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    #[inline]
    pub fn maximum_velocity_torque(&self) -> f32 {
        self.max_velocity_torque
    }

    #[inline]
    pub fn linear_spring_hertz(&self) -> f32 {
        self.linear_hertz
    }

    #[inline]
    pub fn linear_spring_damping_ratio(&self) -> f32 {
        self.linear_damping_ratio
    }

    #[inline]
    pub fn maximum_spring_force(&self) -> f32 {
        self.max_spring_force
    }

    #[inline]
    pub fn angular_spring_hertz(&self) -> f32 {
        self.angular_hertz
    }

    #[inline]
    pub fn angular_spring_damping_ratio(&self) -> f32 {
        self.angular_damping_ratio
    }

    #[inline]
    pub fn maximum_spring_torque(&self) -> f32 {
        self.max_spring_torque
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_motor_joint_def_valid(self)
    }

    /// Target linear velocity of body B relative to A (m/s).
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.linear_velocity = v.into();
        self
    }
    /// Maximum force to achieve linear velocity (N).
    pub fn max_velocity_force(mut self, v: f32) -> Self {
        self.max_velocity_force = v;
        self
    }
    /// Target angular velocity of body B relative to A (rad/s).
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.angular_velocity = v;
        self
    }
    /// Maximum torque to achieve angular velocity (N·m).
    pub fn max_velocity_torque(mut self, v: f32) -> Self {
        self.max_velocity_torque = v;
        self
    }
    /// Linear spring stiffness (Hz).
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.linear_hertz = v;
        self
    }
    /// Linear damping ratio \[0,1].
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.linear_damping_ratio = v;
        self
    }
    /// Maximum linear spring force (N).
    pub fn max_spring_force(mut self, v: f32) -> Self {
        self.max_spring_force = v;
        self
    }
    /// Angular spring stiffness (Hz).
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.angular_hertz = v;
        self
    }
    /// Angular damping ratio \[0,1].
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.angular_damping_ratio = v;
        self
    }
    /// Maximum angular spring torque (N·m).
    pub fn max_spring_torque(mut self, v: f32) -> Self {
        self.max_spring_torque = v;
        self
    }
}

// Motor joint convenience builder
/// Fluent builder for motor joints.
pub struct MotorJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) def: MotorJointDef,
}

impl<'w> MotorJointBuilder<'w> {
    /// Target linear velocity (m/s).
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.def = self.def.linear_velocity(v.into());
        self
    }
    /// Target angular velocity (rad/s).
    pub fn angular_velocity(mut self, w: f32) -> Self {
        self.def = self.def.angular_velocity(w);
        self
    }
    /// Maximum force for achieving linear velocity (N).
    pub fn max_velocity_force(mut self, f: f32) -> Self {
        self.def = self.def.max_velocity_force(f);
        self
    }
    /// Maximum torque for achieving angular velocity (N·m).
    pub fn max_velocity_torque(mut self, t: f32) -> Self {
        self.def = self.def.max_velocity_torque(t);
        self
    }
    /// Linear spring (Hz, damping ratio).
    pub fn linear_spring(mut self, hz: f32, dr: f32) -> Self {
        self.def = self.def.linear_hertz(hz).linear_damping_ratio(dr);
        self
    }
    /// Angular spring (Hz, damping ratio).
    pub fn angular_spring(mut self, hz: f32, dr: f32) -> Self {
        self.def = self.def.angular_hertz(hz).angular_damping_ratio(dr);
        self
    }
    /// Allow bodies to collide while connected.
    pub fn collide_connected(mut self, flag: bool) -> Self {
        let base = *self.def.base();
        *self.def.base_mut() = base.with_collide_connected(flag);
        self
    }

    #[must_use]
    pub fn build(self) -> Joint<'w> {
        self.world.create_motor_joint(&self.def)
    }

    pub fn try_build(self) -> ApiResult<Joint<'w>> {
        self.world.try_create_motor_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(self) -> OwnedJoint {
        self.world.create_motor_joint_owned(&self.def)
    }

    pub fn try_build_owned(self) -> ApiResult<OwnedJoint> {
        self.world.try_create_motor_joint_owned(&self.def)
    }
}
