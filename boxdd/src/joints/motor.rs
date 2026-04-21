use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint, raw_body_id};
use crate::error::ApiResult;

// Motor joint
#[derive(Clone, Debug)]
/// Motor joint definition (maps to `b2MotorJointDef`). Drives relative motion
/// between two bodies using linear and angular offsets with configurable
/// maximum forces.
pub struct MotorJointDef(pub(crate) ffi::b2MotorJointDef);

impl MotorJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2MotorJointDef = unsafe { ffi::b2DefaultMotorJointDef() };
        def.base = base.0;
        Self(def)
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2MotorJointDef) -> Self {
        Self(raw)
    }

    #[inline]
    pub fn base(&self) -> JointBase {
        JointBase(self.0.base)
    }

    #[inline]
    pub fn target_linear_velocity(&self) -> crate::types::Vec2 {
        crate::types::Vec2::from_raw(self.0.linearVelocity)
    }

    #[inline]
    pub fn maximum_velocity_force(&self) -> f32 {
        self.0.maxVelocityForce
    }

    #[inline]
    pub fn target_angular_velocity(&self) -> f32 {
        self.0.angularVelocity
    }

    #[inline]
    pub fn maximum_velocity_torque(&self) -> f32 {
        self.0.maxVelocityTorque
    }

    #[inline]
    pub fn linear_spring_hertz(&self) -> f32 {
        self.0.linearHertz
    }

    #[inline]
    pub fn linear_spring_damping_ratio(&self) -> f32 {
        self.0.linearDampingRatio
    }

    #[inline]
    pub fn maximum_spring_force(&self) -> f32 {
        self.0.maxSpringForce
    }

    #[inline]
    pub fn angular_spring_hertz(&self) -> f32 {
        self.0.angularHertz
    }

    #[inline]
    pub fn angular_spring_damping_ratio(&self) -> f32 {
        self.0.angularDampingRatio
    }

    #[inline]
    pub fn maximum_spring_torque(&self) -> f32 {
        self.0.maxSpringTorque
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2MotorJointDef {
        self.0
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_motor_joint_def_valid(self)
    }

    /// Target linear velocity of body B relative to A (m/s).
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.0.linearVelocity = v.into().into_raw();
        self
    }
    /// Maximum force to achieve linear velocity (N).
    pub fn max_velocity_force(mut self, v: f32) -> Self {
        self.0.maxVelocityForce = v;
        self
    }
    /// Target angular velocity of body B relative to A (rad/s).
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.0.angularVelocity = v;
        self
    }
    /// Maximum torque to achieve angular velocity (N·m).
    pub fn max_velocity_torque(mut self, v: f32) -> Self {
        self.0.maxVelocityTorque = v;
        self
    }
    /// Linear spring stiffness (Hz).
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.0.linearHertz = v;
        self
    }
    /// Linear damping ratio \[0,1].
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.0.linearDampingRatio = v;
        self
    }
    /// Maximum linear spring force (N).
    pub fn max_spring_force(mut self, v: f32) -> Self {
        self.0.maxSpringForce = v;
        self
    }
    /// Angular spring stiffness (Hz).
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.0.angularHertz = v;
        self
    }
    /// Angular damping ratio \[0,1].
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.0.angularDampingRatio = v;
        self
    }
    /// Maximum angular spring torque (N·m).
    pub fn max_spring_torque(mut self, v: f32) -> Self {
        self.0.maxSpringTorque = v;
        self
    }
}

// Motor joint convenience builder
/// Fluent builder for motor joints.
pub struct MotorJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
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
        self.def.0.base.collideConnected = flag;
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        // Default frames: identity (base only needs bodies)
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.world.create_motor_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.world.try_create_motor_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.world.create_motor_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.world.try_create_motor_joint_owned(&self.def)
    }
}
