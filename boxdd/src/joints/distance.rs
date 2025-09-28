use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, JointBaseBuilder};

// Distance joint
#[derive(Clone, Debug)]
/// Distance joint definition (maps to `b2DistanceJointDef`).
///
/// Controls distance limits, optional spring (stiffness/damping), and optional motor.
/// Use with `World::create_distance_joint(_id)` or the world convenience
/// builder `World::distance(...).build()`.
pub struct DistanceJointDef(pub(crate) ffi::b2DistanceJointDef);

impl DistanceJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2DistanceJointDef = unsafe { ffi::b2DefaultDistanceJointDef() };
        def.base = base.0;
        Self(def)
    }
    /// Target distance between anchors (meters).
    pub fn length(mut self, v: f32) -> Self {
        self.0.length = v;
        self
    }
    /// Enable/disable spring behavior.
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    /// Lower bound on spring force.
    pub fn lower_spring_force(mut self, v: f32) -> Self {
        self.0.lowerSpringForce = v;
        self
    }
    /// Upper bound on spring force.
    pub fn upper_spring_force(mut self, v: f32) -> Self {
        self.0.upperSpringForce = v;
        self
    }
    /// Spring stiffness in Hertz.
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    /// Spring damping ratio [0,1].
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    /// Enable/disable distance limits.
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    /// Minimum distance when limits are enabled.
    pub fn min_length(mut self, v: f32) -> Self {
        self.0.minLength = v;
        self
    }
    /// Maximum distance when limits are enabled.
    pub fn max_length(mut self, v: f32) -> Self {
        self.0.maxLength = v;
        self
    }
    /// Enable/disable motor along the line.
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    /// Motor maximum force (N).
    pub fn max_motor_force(mut self, v: f32) -> Self {
        self.0.maxMotorForce = v;
        self
    }
    /// Motor speed (m/s) along the line.
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }

    /// Convenience: compute length from two world points.
    pub fn length_from_world_points<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        let a: ffi::b2Vec2 = a.into().into();
        let b: ffi::b2Vec2 = b.into().into();
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        self.0.length = (dx * dx + dy * dy).sqrt();
        self
    }
}

// Distance joint convenience builder
/// Fluent builder for distance joints working in world space.
///
/// Use `anchors_world` and `length_from_world_points` to configure anchors and
/// target length without manually computing local frames.
pub struct DistanceJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) anchor_a_world: Option<ffi::b2Vec2>,
    pub(crate) anchor_b_world: Option<ffi::b2Vec2>,
    pub(crate) def: DistanceJointDef,
}

impl<'w> DistanceJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.anchor_a_world = Some(ffi::b2Vec2::from(a.into()));
        self.anchor_b_world = Some(ffi::b2Vec2::from(b.into()));
        self
    }
    /// Set desired distance (meters).
    pub fn length(mut self, len: f32) -> Self {
        self.def = self.def.length(len);
        self
    }
    /// Compute desired distance from two world points.
    pub fn length_from_world_points<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.def = self
            .def
            .length_from_world_points(ffi::b2Vec2::from(a.into()), ffi::b2Vec2::from(b.into()));
        self
    }
    /// Enable limits with minimum/maximum length (meters).
    pub fn limit(mut self, min_len: f32, max_len: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .min_length(min_len)
            .max_length(max_len);
        self
    }
    /// Enable motor with maximum force (N) and speed (m/s).
    pub fn motor(mut self, max_force: f32, speed: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_force(max_force)
            .motor_speed(speed);
        self
    }
    /// Enable spring with stiffness (Hz) and damping ratio.
    pub fn spring(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }
    /// Allow bodies to collide while connected.
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    /// Enable limits and motor together.
    ///
    /// - min_len/max_len: meters
    /// - max_force: Newtons
    /// - speed: meters/second
    pub fn with_limit_and_motor(
        mut self,
        min_len: f32,
        max_len: f32,
        max_force: f32,
        speed: f32,
    ) -> Self {
        self = self.limit(min_len, max_len);
        self = self.motor(max_force, speed);
        self
    }
    /// Enable limits and spring together.
    ///
    /// - min_len/max_len: meters
    /// - hertz: stiffness (Hz), typical 4–20
    /// - damping_ratio: [0, 1], typical 0.1–0.7
    pub fn with_limit_and_spring(
        mut self,
        min_len: f32,
        max_len: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(min_len, max_len);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Enable motor and spring together.
    ///
    /// - max_force: Newtons
    /// - speed: meters/second
    /// - hertz: stiffness (Hz), typical 4–20
    /// - damping_ratio: [0, 1], typical 0.1–0.7
    pub fn with_motor_and_spring(
        mut self,
        max_force: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor(max_force, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Enable limit, motor, and spring together.
    ///
    /// - min_len/max_len: meters
    /// - max_force: Newtons
    /// - speed: meters/second
    /// - hertz: stiffness (Hz), typical 4–20
    /// - damping_ratio: [0, 1], typical 0.1–0.7
    pub fn with_limit_motor_spring(
        mut self,
        min_len: f32,
        max_len: f32,
        max_force: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(min_len, max_len);
        self = self.motor(max_force, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        // Compute frames from anchors; default to body positions
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
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
        self.world.create_distance_joint(&self.def)
    }
}
