#![allow(rustdoc::broken_intra_doc_links)]
use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, JointBaseBuilder};

// Revolute joint
#[derive(Clone, Debug)]
/// Revolute (hinge) joint definition (maps to `b2RevoluteJointDef`).
///
/// Allows rotation around an anchor with optional angular limits, motor, and
/// spring (stiffness/damping). Use with `World::create_revolute_joint(_id)` or
/// `World::revolute(...).build()`.
pub struct RevoluteJointDef(pub(crate) ffi::b2RevoluteJointDef);

impl RevoluteJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2RevoluteJointDef = unsafe { ffi::b2DefaultRevoluteJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn target_angle(mut self, v: f32) -> Self {
        self.0.targetAngle = v;
        self
    }
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    pub fn lower_angle(mut self, v: f32) -> Self {
        self.0.lowerAngle = v;
        self
    }
    pub fn upper_angle(mut self, v: f32) -> Self {
        self.0.upperAngle = v;
        self
    }
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    pub fn max_motor_torque(mut self, v: f32) -> Self {
        self.0.maxMotorTorque = v;
        self
    }
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }

    /// Convenience: set angular limits in degrees.
    pub fn limit_deg(mut self, lower_deg: f32, upper_deg: f32) -> Self {
        let to_rad = core::f32::consts::PI / 180.0;
        self.0.lowerAngle = lower_deg * to_rad;
        self.0.upperAngle = upper_deg * to_rad;
        self.0.enableLimit = true;
        self
    }
    /// Convenience: motor speed in degrees/sec.
    pub fn motor_speed_deg(mut self, speed_deg_per_s: f32) -> Self {
        self.0.motorSpeed = speed_deg_per_s * (core::f32::consts::PI / 180.0);
        self
    }
}

/// Builder for a revolute (hinge) joint in world space.
/// Fluent builder for revolute joints using a world anchor.
pub struct RevoluteJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) anchor_world: Option<ffi::b2Vec2>,
    pub(crate) def: RevoluteJointDef,
}

impl<'w> RevoluteJointBuilder<'w> {
    /// Set world anchor (defaults to body A position).
    pub fn anchor_world<V: Into<crate::types::Vec2>>(mut self, a: V) -> Self {
        self.anchor_world = Some(ffi::b2Vec2::from(a.into()));
        self
    }
    /// Limit angles in radians.
    pub fn limit(mut self, lower: f32, upper: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .lower_angle(lower)
            .upper_angle(upper);
        self
    }
    /// Limit angles in degrees.
    pub fn limit_deg(mut self, lower_deg: f32, upper_deg: f32) -> Self {
        self.def = self.def.limit_deg(lower_deg, upper_deg);
        self
    }
    /// Enable motor with maximum torque (N·m) and speed (rad/s).
    pub fn motor(mut self, max_torque: f32, speed: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed(speed);
        self
    }
    /// Enable motor with maximum torque (N·m) and speed (deg/s).
    pub fn motor_deg(mut self, max_torque: f32, speed_deg: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed_deg(speed_deg);
        self
    }
    /// Spring (Hz, damping ratio).
    pub fn spring(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    /// Convenience: enable limit and motor together.
    /// - lower/upper: radians; -pi..pi typical
    /// - max_torque: N·m; speed: rad/s
    pub fn with_limit_and_motor(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor(max_torque, speed);
        self
    }
    /// Convenience: enable limit and motor together (motor speed in degrees/sec).
    /// - lower/upper: radians; -pi..pi typical
    /// - max_torque: N·m; speed_deg: deg/s
    pub fn with_limit_and_motor_deg(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed_deg: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor_deg(max_torque, speed_deg);
        self
    }
    /// Convenience: enable limit and spring together.
    /// - lower/upper: radians; -pi..pi typical
    /// - hertz: stiffness (Hz), typical 4–20; damping_ratio: [0,1], typical 0.1–0.7
    pub fn with_limit_and_spring(
        mut self,
        lower: f32,
        upper: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable motor and spring together.
    /// - max_torque: N·m; speed: rad/s; hertz: Hz; damping_ratio: [0,1]
    pub fn with_motor_and_spring(
        mut self,
        max_torque: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor(max_torque, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable motor and spring together (motor speed in degrees/sec).
    /// - max_torque: N·m; speed_deg: deg/s; hertz: Hz; damping_ratio: [0,1]
    pub fn with_motor_and_spring_deg(
        mut self,
        max_torque: f32,
        speed_deg: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor_deg(max_torque, speed_deg);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable limit, motor, and spring together.
    /// - lower/upper: radians; -pi..pi typical
    /// - max_torque: N·m; speed: rad/s; hertz: Hz; damping_ratio: [0,1]
    pub fn with_limit_motor_spring(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor(max_torque, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable limit, motor (deg/s), and spring together.
    /// - lower/upper: radians; -pi..pi typical
    /// - max_torque: N·m; speed_deg: deg/s; hertz: Hz; damping_ratio: [0,1]
    pub fn with_limit_motor_spring_deg(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed_deg: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor_deg(max_torque, speed_deg);
        self = self.spring(hertz, damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_world.unwrap_or(ta.p);
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
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
        self.def.0.base = base.0;
        self.world.create_revolute_joint(&self.def)
    }
}
