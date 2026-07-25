#![allow(rustdoc::broken_intra_doc_links)]
use crate::types::{Position, Vec2, WorldTransform};
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
pub struct WheelJointDef {
    base: JointBase,
    enable_spring: bool,
    hertz: f32,
    damping_ratio: f32,
    enable_limit: bool,
    lower_translation: f32,
    upper_translation: f32,
    enable_motor: bool,
    max_motor_torque: f32,
    motor_speed: f32,
}

impl WheelJointDef {
    pub fn new(base: JointBase) -> Self {
        let _lease = crate::core::foundation::assert_transient_native_lease();
        let raw = unsafe { ffi::b2DefaultWheelJointDef() };
        Self {
            base,
            enable_spring: raw.enableSpring,
            hertz: raw.hertz,
            damping_ratio: raw.dampingRatio,
            enable_limit: raw.enableLimit,
            lower_translation: raw.lowerTranslation,
            upper_translation: raw.upperTranslation,
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
    pub fn base_mut(&mut self) -> &mut JointBase {
        &mut self.base
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
    pub fn minimum_translation(&self) -> f32 {
        self.lower_translation
    }

    #[inline]
    pub fn maximum_translation(&self) -> f32 {
        self.upper_translation
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

    pub(crate) fn to_raw(&self) -> ffi::b2WheelJointDef {
        let mut raw = unsafe { ffi::b2DefaultWheelJointDef() };
        raw.base = self.base.to_raw();
        raw.enableSpring = self.enable_spring;
        raw.hertz = self.hertz;
        raw.dampingRatio = self.damping_ratio;
        raw.enableLimit = self.enable_limit;
        raw.lowerTranslation = self.lower_translation;
        raw.upperTranslation = self.upper_translation;
        raw.enableMotor = self.enable_motor;
        raw.maxMotorTorque = self.max_motor_torque;
        raw.motorSpeed = self.motor_speed;
        raw
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_wheel_joint_def_valid(self)
    }

    /// Enable/disable suspension spring.
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.enable_spring = flag;
        self
    }
    /// Spring stiffness in Hertz.
    pub fn hertz(mut self, v: f32) -> Self {
        self.hertz = v;
        self
    }
    /// Spring damping ratio \[0,1].
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.damping_ratio = v;
        self
    }
    /// Enable/disable translation limits.
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.enable_limit = flag;
        self
    }
    /// Lower translation limit (meters).
    pub fn lower_translation(mut self, v: f32) -> Self {
        self.lower_translation = v;
        self
    }
    /// Upper translation limit (meters).
    pub fn upper_translation(mut self, v: f32) -> Self {
        self.upper_translation = v;
        self
    }
    /// Enable/disable wheel motor.
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.enable_motor = flag;
        self
    }
    /// Maximum motor torque (N·m).
    pub fn max_motor_torque(mut self, v: f32) -> Self {
        self.max_motor_torque = v;
        self
    }
    /// Motor speed (rad/s).
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.motor_speed = v;
        self
    }
    /// Convenience: motor speed in degrees/sec.
    pub fn motor_speed_deg(mut self, speed_deg_per_s: f32) -> Self {
        self.motor_speed = speed_deg_per_s * (core::f32::consts::PI / 180.0);
        self
    }
}

/// Fluent builder for wheel joints using world anchors and axis.
pub struct WheelJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) anchor_a_world: Option<Position>,
    pub(crate) anchor_b_world: Option<Position>,
    pub(crate) axis_world: Option<Vec2>,
    pub(crate) def: WheelJointDef,
}

impl<'w> WheelJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<Position>, VB: Into<Position>>(mut self, a: VA, b: VB) -> Self {
        self.anchor_a_world = Some(a.into());
        self.anchor_b_world = Some(b.into());
        self
    }
    /// Set wheel axis in world space.
    pub fn axis_world<V: Into<Vec2>>(mut self, axis: V) -> Self {
        self.axis_world = Some(axis.into());
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
        self.def.base = self.def.base.with_collide_connected(flag);
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

    fn configure_local_frames(&mut self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        super::creation::check_joint_target_identity(self.world, self.def.base())?;
        self.def.validate()?;
        if let Some(anchor) = self.anchor_a_world {
            super::validation::check_joint_position(anchor)?;
        }
        if let Some(anchor) = self.anchor_b_world {
            super::validation::check_joint_position(anchor)?;
        }
        let axis = self.axis_world.unwrap_or(Vec2::new(1.0, 0.0));
        super::validation::check_joint_axis(axis)?;
        super::creation::check_joint_target_native(self.world, self.def.base())?;

        let body_a = self.def.base().body_a_id();
        let body_b = self.def.base().body_b_id();
        let ta = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_a)) });
        let tb = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_b)) });
        let aw = self.anchor_a_world.unwrap_or_else(|| ta.position());
        let bw = self.anchor_b_world.unwrap_or_else(|| tb.position());
        let la = super::base_def::checked_world_to_local_point(ta, aw)?;
        let lb = super::base_def::checked_world_to_local_point(tb, bw)?;
        let ra = super::base_def::checked_world_axis_to_local_rotation(ta, axis)?;
        let rb = super::base_def::checked_world_axis_to_local_rotation(tb, axis)?;
        self.def.base_mut().set_local_frames(
            crate::Transform::from_pos_angle(la, ra.angle()),
            crate::Transform::from_pos_angle(lb, rb.angle()),
        );
        Ok(())
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        self.configure_local_frames()
            .expect("wheel-joint bodies and world-space frame must be valid for this world");
        self.world.create_wheel_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        self.configure_local_frames()?;
        self.world.try_create_wheel_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        self.configure_local_frames()
            .expect("wheel-joint bodies and world-space frame must be valid for this world");
        self.world.create_wheel_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        self.configure_local_frames()?;
        self.world.try_create_wheel_joint_owned(&self.def)
    }
}
