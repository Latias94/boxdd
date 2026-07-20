#![allow(rustdoc::broken_intra_doc_links)]
use crate::types::{Position, WorldTransform};
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint, raw_body_id};
use crate::error::ApiResult;

// Revolute joint
#[derive(Clone, Debug)]
/// Revolute (hinge) joint definition (maps to `b2RevoluteJointDef`).
///
/// Allows rotation around an anchor with optional angular limits, motor, and
/// spring (stiffness/damping). Use with `World::create_revolute_joint(_id)` or
/// `World::revolute(...).build()`.
pub struct RevoluteJointDef {
    base: JointBase,
    target_angle: f32,
    enable_spring: bool,
    hertz: f32,
    damping_ratio: f32,
    enable_limit: bool,
    lower_angle: f32,
    upper_angle: f32,
    enable_motor: bool,
    max_motor_torque: f32,
    motor_speed: f32,
}

impl RevoluteJointDef {
    pub fn new(base: JointBase) -> Self {
        let raw = unsafe { ffi::b2DefaultRevoluteJointDef() };
        Self {
            base,
            target_angle: raw.targetAngle,
            enable_spring: raw.enableSpring,
            hertz: raw.hertz,
            damping_ratio: raw.dampingRatio,
            enable_limit: raw.enableLimit,
            lower_angle: raw.lowerAngle,
            upper_angle: raw.upperAngle,
            enable_motor: raw.enableMotor,
            max_motor_torque: raw.maxMotorTorque,
            motor_speed: raw.motorSpeed,
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
    pub fn target_angle_value(&self) -> f32 {
        self.target_angle
    }

    #[inline]
    pub fn spring_enabled(&self) -> bool {
        self.enable_spring
    }

    #[inline]
    pub fn spring_hertz(&self) -> f32 {
        self.hertz
    }

    #[inline]
    pub fn spring_damping_ratio(&self) -> f32 {
        self.damping_ratio
    }

    #[inline]
    pub fn limit_enabled(&self) -> bool {
        self.enable_limit
    }

    #[inline]
    pub fn minimum_angle(&self) -> f32 {
        self.lower_angle
    }

    #[inline]
    pub fn maximum_angle(&self) -> f32 {
        self.upper_angle
    }

    #[inline]
    pub fn motor_enabled(&self) -> bool {
        self.enable_motor
    }

    #[inline]
    pub fn maximum_motor_torque(&self) -> f32 {
        self.max_motor_torque
    }

    #[inline]
    pub fn target_motor_speed(&self) -> f32 {
        self.motor_speed
    }

    pub(crate) fn to_raw(&self) -> ffi::b2RevoluteJointDef {
        let mut raw = unsafe { ffi::b2DefaultRevoluteJointDef() };
        raw.base = self.base.to_raw();
        raw.targetAngle = self.target_angle;
        raw.enableSpring = self.enable_spring;
        raw.hertz = self.hertz;
        raw.dampingRatio = self.damping_ratio;
        raw.enableLimit = self.enable_limit;
        raw.lowerAngle = self.lower_angle;
        raw.upperAngle = self.upper_angle;
        raw.enableMotor = self.enable_motor;
        raw.maxMotorTorque = self.max_motor_torque;
        raw.motorSpeed = self.motor_speed;
        raw
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_revolute_joint_def_valid(self)
    }

    pub fn target_angle(mut self, v: f32) -> Self {
        self.target_angle = v;
        self
    }
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.enable_spring = flag;
        self
    }
    pub fn hertz(mut self, v: f32) -> Self {
        self.hertz = v;
        self
    }
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.damping_ratio = v;
        self
    }
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.enable_limit = flag;
        self
    }
    pub fn lower_angle(mut self, v: f32) -> Self {
        self.lower_angle = v;
        self
    }
    pub fn upper_angle(mut self, v: f32) -> Self {
        self.upper_angle = v;
        self
    }
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.enable_motor = flag;
        self
    }
    pub fn max_motor_torque(mut self, v: f32) -> Self {
        self.max_motor_torque = v;
        self
    }
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.motor_speed = v;
        self
    }

    /// Convenience: set angular limits in degrees.
    pub fn limit_deg(mut self, lower_deg: f32, upper_deg: f32) -> Self {
        let to_rad = core::f32::consts::PI / 180.0;
        self.lower_angle = lower_deg * to_rad;
        self.upper_angle = upper_deg * to_rad;
        self.enable_limit = true;
        self
    }
    /// Convenience: motor speed in degrees/sec.
    pub fn motor_speed_deg(mut self, speed_deg_per_s: f32) -> Self {
        self.motor_speed = speed_deg_per_s * (core::f32::consts::PI / 180.0);
        self
    }
}

/// Builder for a revolute (hinge) joint in world space.
/// Fluent builder for revolute joints using a world anchor.
pub struct RevoluteJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) anchor_world: Option<Position>,
    pub(crate) def: RevoluteJointDef,
}

impl<'w> RevoluteJointBuilder<'w> {
    /// Set world anchor (defaults to body A position).
    pub fn anchor_world<V: Into<Position>>(mut self, a: V) -> Self {
        self.anchor_world = Some(a.into());
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
        self.def.base = self.def.base.with_collide_connected(flag);
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
    /// - lower/upper: radians; -pi..pi typical
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
    /// - lower/upper: radians; -pi..pi typical
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

    fn configure_local_frames(&mut self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        let body_a = self.def.base().body_a_id();
        let body_b = self.def.base().body_b_id();
        self.world.core().check_body(body_a)?;
        self.world.core().check_body(body_b)?;

        let ta = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_a)) });
        let tb = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_b)) });
        let anchor = self.anchor_world.unwrap_or_else(|| ta.position());
        let la = super::base_def::checked_world_to_local_point(ta, anchor)?;
        let lb = super::base_def::checked_world_to_local_point(tb, anchor)?;
        self.def.base_mut().set_local_frames(
            crate::Transform::from_pos_angle(la, 0.0),
            crate::Transform::from_pos_angle(lb, 0.0),
        );
        Ok(())
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        self.configure_local_frames()
            .expect("revolute-joint bodies and world anchor must be valid for this world");
        self.world.create_revolute_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        self.configure_local_frames()?;
        self.world.try_create_revolute_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        self.configure_local_frames()
            .expect("revolute-joint bodies and world anchor must be valid for this world");
        self.world.create_revolute_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        self.configure_local_frames()?;
        self.world.try_create_revolute_joint_owned(&self.def)
    }
}
