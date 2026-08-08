use crate::types::{JointId, Position};
use crate::world::World;
use boxdd_sys::ffi;

use super::JointBase;
use crate::error::{Error, Result};

fn checked_world_distance(a: Position, b: Position) -> Result<f32> {
    let delta = b.checked_relative_to(a).map_err(|_| {
        Error::invalid_argument(
            "DistanceJointDef::length_from_world_points",
            "a/b",
            "points whose delta fits in f32 coordinates",
        )
    })?;
    let length = delta.x.hypot(delta.y);
    if length.is_finite() {
        Ok(length)
    } else {
        Err(Error::invalid_argument(
            "DistanceJointDef::length_from_world_points",
            "a/b",
            "points that produce a finite distance",
        ))
    }
}

// Distance joint
#[derive(Clone, Debug)]
/// Distance joint definition (maps to `b2DistanceJointDef`).
///
/// Controls distance limits, optional spring (stiffness/damping), and optional motor.
/// Use with [`World::create_distance_joint`] or the [`World::distance`] convenience builder.
pub struct DistanceJointDef {
    base: JointBase,
    length: f32,
    enable_spring: bool,
    lower_spring_force: f32,
    upper_spring_force: f32,
    hertz: f32,
    damping_ratio: f32,
    enable_limit: bool,
    min_length: f32,
    max_length: f32,
    enable_motor: bool,
    max_motor_force: f32,
    motor_speed: f32,
}

impl DistanceJointDef {
    pub fn new(base: JointBase) -> Self {
        let default: ffi::b2DistanceJointDef = crate::core::native_defaults::distance_joint_def(
            base.to_raw(),
            base.length_scale().units_per_meter(),
        );
        Self {
            base,
            length: default.length,
            enable_spring: default.enableSpring,
            lower_spring_force: default.lowerSpringForce,
            upper_spring_force: default.upperSpringForce,
            hertz: default.hertz,
            damping_ratio: default.dampingRatio,
            enable_limit: default.enableLimit,
            min_length: default.minLength,
            max_length: default.maxLength,
            enable_motor: default.enableMotor,
            max_motor_force: default.maxMotorForce,
            motor_speed: default.motorSpeed,
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
    pub fn target_length(&self) -> f32 {
        self.length
    }

    #[inline]
    pub fn spring_enabled(&self) -> bool {
        self.enable_spring
    }

    #[inline]
    pub fn minimum_spring_force(&self) -> f32 {
        self.lower_spring_force
    }

    #[inline]
    pub fn maximum_spring_force(&self) -> f32 {
        self.upper_spring_force
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
    pub fn minimum_length(&self) -> f32 {
        self.min_length
    }

    #[inline]
    pub fn maximum_length(&self) -> f32 {
        self.max_length
    }

    #[inline]
    pub fn motor_enabled(&self) -> bool {
        self.enable_motor
    }

    #[inline]
    pub fn maximum_motor_force(&self) -> f32 {
        self.max_motor_force
    }

    #[inline]
    pub fn target_motor_speed(&self) -> f32 {
        self.motor_speed
    }

    pub(crate) fn to_raw(&self) -> ffi::b2DistanceJointDef {
        let mut raw: ffi::b2DistanceJointDef = crate::core::native_defaults::distance_joint_def(
            self.base.to_raw(),
            self.base.length_scale().units_per_meter(),
        );
        raw.length = self.length;
        raw.enableSpring = self.enable_spring;
        raw.lowerSpringForce = self.lower_spring_force;
        raw.upperSpringForce = self.upper_spring_force;
        raw.hertz = self.hertz;
        raw.dampingRatio = self.damping_ratio;
        raw.enableLimit = self.enable_limit;
        raw.minLength = self.min_length;
        raw.maxLength = self.max_length;
        raw.enableMotor = self.enable_motor;
        raw.maxMotorForce = self.max_motor_force;
        raw.motorSpeed = self.motor_speed;
        raw
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        super::check_distance_joint_def_valid(self)
    }

    /// Target distance between anchors (meters).
    pub fn length(mut self, v: f32) -> Self {
        self.length = v;
        self
    }
    /// Enable/disable spring behavior.
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.enable_spring = flag;
        self
    }
    /// Lower bound on spring force.
    pub fn lower_spring_force(mut self, v: f32) -> Self {
        self.lower_spring_force = v;
        self
    }
    /// Upper bound on spring force.
    pub fn upper_spring_force(mut self, v: f32) -> Self {
        self.upper_spring_force = v;
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
    /// Enable/disable distance limits.
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.enable_limit = flag;
        self
    }
    /// Minimum distance when limits are enabled.
    pub fn min_length(mut self, v: f32) -> Self {
        self.min_length = v;
        self
    }
    /// Maximum distance when limits are enabled.
    pub fn max_length(mut self, v: f32) -> Self {
        self.max_length = v;
        self
    }
    /// Enable/disable motor along the line.
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.enable_motor = flag;
        self
    }
    /// Motor maximum force (N).
    pub fn max_motor_force(mut self, v: f32) -> Self {
        self.max_motor_force = v;
        self
    }
    /// Motor speed (m/s) along the line.
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.motor_speed = v;
        self
    }

    /// Compute length from two absolute world points.
    pub fn length_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Result<Self> {
        self.length = checked_world_distance(a.into(), b.into())?;
        Ok(self)
    }
}

// Distance joint convenience builder
/// Fluent builder for distance joints working in world space.
///
/// Use `anchors_world` and `length_from_world_points` to configure anchors and
/// target length without manually computing local frames.
pub struct DistanceJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) anchor_a_world: Option<Position>,
    pub(crate) anchor_b_world: Option<Position>,
    pub(crate) def: DistanceJointDef,
}

impl<'w> DistanceJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<Position>, VB: Into<Position>>(mut self, a: VA, b: VB) -> Self {
        self.anchor_a_world = Some(a.into());
        self.anchor_b_world = Some(b.into());
        self
    }
    /// Set desired distance (meters).
    pub fn length(mut self, len: f32) -> Self {
        self.def = self.def.length(len);
        self
    }
    /// Compute desired distance from two world points.
    pub fn length_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Result<Self> {
        self.def = self.def.length_from_world_points(a, b)?;
        Ok(self)
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
        let base = *self.def.base();
        *self.def.base_mut() = base.with_collide_connected(flag);
        self
    }

    fn configure_local_frames(&mut self) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        super::creation::check_joint_target_identity(self.world, self.def.base())?;
        self.def.validate()?;
        if let Some(anchor) = self.anchor_a_world {
            super::validation::check_joint_position(
                anchor,
                "DistanceJointBuilder::build",
                "anchor_a_world",
            )?;
        }
        if let Some(anchor) = self.anchor_b_world {
            super::validation::check_joint_position(
                anchor,
                "DistanceJointBuilder::build",
                "anchor_b_world",
            )?;
        }
        super::creation::check_joint_target_native(self.world, self.def.base())?;

        let body_a = self.def.base().body_a_id();
        let body_b = self.def.base().body_b_id();

        let ta = super::read_native_body_world_transform(
            "DistanceJointBuilder::build",
            "body_a_transform",
            body_a,
        )?;
        let tb = super::read_native_body_world_transform(
            "DistanceJointBuilder::build",
            "body_b_transform",
            body_b,
        )?;
        let aw = self.anchor_a_world.unwrap_or_else(|| ta.position());
        let bw = self.anchor_b_world.unwrap_or_else(|| tb.position());
        let la = super::base_def::checked_world_to_local_point(
            "DistanceJointBuilder::build",
            "anchor_a_world",
            ta,
            aw,
        )?;
        let lb = super::base_def::checked_world_to_local_point(
            "DistanceJointBuilder::build",
            "anchor_b_world",
            tb,
            bw,
        )?;
        self.def.base_mut().set_local_frames(
            crate::Transform::from_pos_angle(la, 0.0)?,
            crate::Transform::from_pos_angle(lb, 0.0)?,
        );
        Ok(())
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
    /// - damping_ratio: \[0, 1], typical 0.1–0.7
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
    /// - damping_ratio: \[0, 1], typical 0.1–0.7
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
    /// - damping_ratio: \[0, 1], typical 0.1–0.7
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

    pub fn build(mut self) -> Result<JointId> {
        self.configure_local_frames()?;
        self.world.create_distance_joint(&self.def)
    }
}
