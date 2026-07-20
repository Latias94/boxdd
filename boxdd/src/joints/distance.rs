#![allow(rustdoc::broken_intra_doc_links)]
use crate::types::{Position, WorldTransform};
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint, raw_body_id};
use crate::error::{ApiError, ApiResult};

fn checked_world_distance(a: Position, b: Position) -> ApiResult<f32> {
    let delta = b
        .checked_relative_to(a)
        .map_err(|_| ApiError::InvalidArgument)?;
    let length = delta.x.hypot(delta.y);
    if length.is_finite() {
        Ok(length)
    } else {
        Err(ApiError::InvalidArgument)
    }
}

// Distance joint
#[derive(Clone, Debug)]
/// Distance joint definition (maps to `b2DistanceJointDef`).
///
/// Controls distance limits, optional spring (stiffness/damping), and optional motor.
/// Use with `World::create_distance_joint(_id)` or the world convenience
/// builder `World::distance(...).build()`.
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
        let default = unsafe { ffi::b2DefaultDistanceJointDef() };
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
        let mut raw = unsafe { ffi::b2DefaultDistanceJointDef() };
        raw.base = self.base.to_raw();
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
    pub fn validate(&self) -> ApiResult<()> {
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

    /// Convenience: compute length from two absolute world points.
    ///
    /// # Panics
    ///
    /// Panics when the world-space delta cannot be represented as a local `f32` length. Use
    /// [`Self::try_length_from_world_points`] for a recoverable error.
    pub fn length_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        self,
        a: VA,
        b: VB,
    ) -> Self {
        self.try_length_from_world_points(a, b)
            .expect("distance-joint world-point delta must fit in local f32 coordinates")
    }

    /// Fallible variant of [`Self::length_from_world_points`].
    pub fn try_length_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        mut self,
        a: VA,
        b: VB,
    ) -> ApiResult<Self> {
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
    ///
    /// # Panics
    ///
    /// Panics when the world-space delta cannot be represented as a local `f32` length. Use
    /// [`Self::try_length_from_world_points`] for a recoverable error.
    pub fn length_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        self,
        a: VA,
        b: VB,
    ) -> Self {
        self.try_length_from_world_points(a, b)
            .expect("distance-joint world-point delta must fit in local f32 coordinates")
    }

    /// Fallible variant of [`Self::length_from_world_points`].
    pub fn try_length_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        mut self,
        a: VA,
        b: VB,
    ) -> ApiResult<Self> {
        self.def = self.def.try_length_from_world_points(a, b)?;
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

    fn configure_local_frames(&mut self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        let body_a = self.def.base().body_a_id();
        let body_b = self.def.base().body_b_id();
        self.world.core().check_body(body_a)?;
        self.world.core().check_body(body_b)?;

        let ta = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_a)) });
        let tb = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_b)) });
        let aw = self.anchor_a_world.unwrap_or_else(|| ta.position());
        let bw = self.anchor_b_world.unwrap_or_else(|| tb.position());
        let la = super::base_def::checked_world_to_local_point(ta, aw)?;
        let lb = super::base_def::checked_world_to_local_point(tb, bw)?;
        self.def.base_mut().set_local_frames(
            crate::Transform::from_pos_angle(la, 0.0),
            crate::Transform::from_pos_angle(lb, 0.0),
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

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        self.configure_local_frames()
            .expect("distance-joint world anchors must fit in local f32 frames");
        self.world.create_distance_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        self.configure_local_frames()?;
        self.world.try_create_distance_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        self.configure_local_frames()
            .expect("distance-joint world anchors must fit in local f32 frames");
        self.world.create_distance_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        self.configure_local_frames()?;
        self.world.try_create_distance_joint_owned(&self.def)
    }
}
