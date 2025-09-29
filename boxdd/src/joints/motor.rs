use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, JointBaseBuilder};

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
    /// Target linear velocity of body B relative to A (m/s).
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.0.linearVelocity = ffi::b2Vec2::from(v.into());
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
        self.def = self.def.linear_velocity(ffi::b2Vec2::from(v.into()));
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
        // Default frames: identity (base only needs bodies)
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .build();
        self.def.0.base = base.0;
        self.world.create_motor_joint(&self.def)
    }
}
