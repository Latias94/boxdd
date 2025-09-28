use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, JointBaseBuilder};

// Motor joint
#[derive(Clone, Debug)]
pub struct MotorJointDef(pub(crate) ffi::b2MotorJointDef);

impl MotorJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2MotorJointDef = unsafe { ffi::b2DefaultMotorJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.0.linearVelocity = ffi::b2Vec2::from(v.into());
        self
    }
    pub fn max_velocity_force(mut self, v: f32) -> Self {
        self.0.maxVelocityForce = v;
        self
    }
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.0.angularVelocity = v;
        self
    }
    pub fn max_velocity_torque(mut self, v: f32) -> Self {
        self.0.maxVelocityTorque = v;
        self
    }
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.0.linearHertz = v;
        self
    }
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.0.linearDampingRatio = v;
        self
    }
    pub fn max_spring_force(mut self, v: f32) -> Self {
        self.0.maxSpringForce = v;
        self
    }
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.0.angularHertz = v;
        self
    }
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.0.angularDampingRatio = v;
        self
    }
    pub fn max_spring_torque(mut self, v: f32) -> Self {
        self.0.maxSpringTorque = v;
        self
    }
}

// Motor joint convenience builder
pub struct MotorJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) def: MotorJointDef,
}

impl<'w> MotorJointBuilder<'w> {
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.def = self.def.linear_velocity(ffi::b2Vec2::from(v.into()));
        self
    }
    pub fn angular_velocity(mut self, w: f32) -> Self {
        self.def = self.def.angular_velocity(w);
        self
    }
    pub fn max_velocity_force(mut self, f: f32) -> Self {
        self.def = self.def.max_velocity_force(f);
        self
    }
    pub fn max_velocity_torque(mut self, t: f32) -> Self {
        self.def = self.def.max_velocity_torque(t);
        self
    }
    pub fn linear_spring(mut self, hz: f32, dr: f32) -> Self {
        self.def = self.def.linear_hertz(hz).linear_damping_ratio(dr);
        self
    }
    pub fn angular_spring(mut self, hz: f32, dr: f32) -> Self {
        self.def = self.def.angular_hertz(hz).angular_damping_ratio(dr);
        self
    }
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
