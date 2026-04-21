#![allow(rustdoc::broken_intra_doc_links)]
use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint, raw_body_id};
use crate::error::ApiResult;

// Wheel joint
#[derive(Clone, Debug)]
/// Wheel (suspension) joint definition (maps to `b2WheelJointDef`).
///
/// Constrains motion along an axis with suspension (spring + damping) and
/// optional motor around the wheel axis. Use with `World::create_wheel_joint(_id)`
/// or `World::wheel(...).build()`.
pub struct WheelJointDef(pub(crate) ffi::b2WheelJointDef);

impl WheelJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2WheelJointDef = unsafe { ffi::b2DefaultWheelJointDef() };
        def.base = base.0;
        Self(def)
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2WheelJointDef) -> Self {
        Self(raw)
    }

    #[inline]
    pub fn base(&self) -> JointBase {
        JointBase(self.0.base)
    }

    #[inline]
    pub fn spring_enabled(&self) -> bool {
        self.0.enableSpring
    }

    #[inline]
    pub fn spring_hertz(&self) -> f32 {
        self.0.hertz
    }

    #[inline]
    pub fn spring_damping_ratio(&self) -> f32 {
        self.0.dampingRatio
    }

    #[inline]
    pub fn limit_enabled(&self) -> bool {
        self.0.enableLimit
    }

    #[inline]
    pub fn minimum_translation(&self) -> f32 {
        self.0.lowerTranslation
    }

    #[inline]
    pub fn maximum_translation(&self) -> f32 {
        self.0.upperTranslation
    }

    #[inline]
    pub fn motor_enabled(&self) -> bool {
        self.0.enableMotor
    }

    #[inline]
    pub fn maximum_motor_torque(&self) -> f32 {
        self.0.maxMotorTorque
    }

    #[inline]
    pub fn target_motor_speed(&self) -> f32 {
        self.0.motorSpeed
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2WheelJointDef {
        self.0
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_wheel_joint_def_valid(self)
    }

    /// Enable/disable suspension spring.
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    /// Spring stiffness in Hertz.
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    /// Spring damping ratio \[0,1].
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    /// Enable/disable translation limits.
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    /// Lower translation limit (meters).
    pub fn lower_translation(mut self, v: f32) -> Self {
        self.0.lowerTranslation = v;
        self
    }
    /// Upper translation limit (meters).
    pub fn upper_translation(mut self, v: f32) -> Self {
        self.0.upperTranslation = v;
        self
    }
    /// Enable/disable wheel motor.
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    /// Maximum motor torque (N·m).
    pub fn max_motor_torque(mut self, v: f32) -> Self {
        self.0.maxMotorTorque = v;
        self
    }
    /// Motor speed (rad/s).
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }
    /// Convenience: motor speed in degrees/sec.
    pub fn motor_speed_deg(mut self, speed_deg_per_s: f32) -> Self {
        self.0.motorSpeed = speed_deg_per_s * (core::f32::consts::PI / 180.0);
        self
    }
}

/// Fluent builder for wheel joints using world anchors and axis.
pub struct WheelJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) anchor_a_world: Option<ffi::b2Vec2>,
    pub(crate) anchor_b_world: Option<ffi::b2Vec2>,
    pub(crate) axis_world: Option<ffi::b2Vec2>,
    pub(crate) def: WheelJointDef,
}

impl<'w> WheelJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.anchor_a_world = Some(a.into().into_raw());
        self.anchor_b_world = Some(b.into().into_raw());
        self
    }
    /// Set wheel axis in world space.
    pub fn axis_world<V: Into<crate::types::Vec2>>(mut self, axis: V) -> Self {
        self.axis_world = Some(axis.into().into_raw());
        self
    }
    pub fn limit(mut self, lower: f32, upper: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .lower_translation(lower)
            .upper_translation(upper);
        self
    }
    pub fn motor(mut self, max_torque: f32, speed: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed(speed);
        self
    }
    pub fn motor_deg(mut self, max_torque: f32, speed_deg: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed_deg(speed_deg);
        self
    }
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
    /// - lower/upper: meters
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
    /// - lower/upper: meters
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
    /// - lower/upper: meters
    /// - hertz: stiffness (Hz), typical 4–20; damping_ratio: \[0,1], typical 0.1–0.7
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
    /// - max_torque: N·m; speed: rad/s; hertz: Hz; damping_ratio: \[0,1]
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
    /// - max_torque: N·m; speed_deg: deg/s; hertz: Hz; damping_ratio: \[0,1]
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
    /// - lower/upper: meters
    /// - max_torque: N·m; speed: rad/s; hertz: Hz; damping_ratio: \[0,1]
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
    /// - lower/upper: meters
    /// - max_torque: N·m; speed_deg: deg/s; hertz: Hz; damping_ratio: \[0,1]
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
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        // Defaults: anchors = body positions, axis = x
        let ta = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_a)) };
        let tb = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_b)) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.create_wheel_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        let ta = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_a)) };
        let tb = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_b)) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.try_create_wheel_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        // Defaults: anchors = body positions, axis = x
        let ta = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_a)) };
        let tb = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_b)) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.create_wheel_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        let ta = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_a)) };
        let tb = unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_b)) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.try_create_wheel_joint_owned(&self.def)
    }
}
