use crate::types::{JointId, Position, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

use super::JointBase;
use crate::error::Result;

// Prismatic joint
#[derive(Clone, Debug)]
/// Prismatic (slider) joint definition (maps to `b2PrismaticJointDef`).
///
/// Constrains two bodies to slide along an axis with optional limits, motor,
/// and spring (stiffness/damping). Use with [`World::create_prismatic_joint`]
/// or [`World::prismatic`].
pub struct PrismaticJointDef {
    base: JointBase,
    enable_spring: bool,
    hertz: f32,
    damping_ratio: f32,
    target_translation: f32,
    enable_limit: bool,
    lower_translation: f32,
    upper_translation: f32,
    enable_motor: bool,
    max_motor_force: f32,
    motor_speed: f32,
}

impl PrismaticJointDef {
    pub fn new(base: JointBase) -> Self {
        let raw: ffi::b2PrismaticJointDef =
            crate::core::native_defaults::prismatic_joint_def(base.to_raw());
        Self {
            base,
            enable_spring: raw.enableSpring,
            hertz: raw.hertz,
            damping_ratio: raw.dampingRatio,
            target_translation: raw.targetTranslation,
            enable_limit: raw.enableLimit,
            lower_translation: raw.lowerTranslation,
            upper_translation: raw.upperTranslation,
            enable_motor: raw.enableMotor,
            max_motor_force: raw.maxMotorForce,
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
    pub fn target_translation(&self) -> f32 {
        self.target_translation
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
    pub fn limit_enabled(&self) -> bool {
        self.enable_limit
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

    pub(crate) fn to_raw(&self) -> ffi::b2PrismaticJointDef {
        let mut raw: ffi::b2PrismaticJointDef =
            crate::core::native_defaults::prismatic_joint_def(self.base.to_raw());
        raw.enableSpring = self.enable_spring;
        raw.hertz = self.hertz;
        raw.dampingRatio = self.damping_ratio;
        raw.targetTranslation = self.target_translation;
        raw.enableLimit = self.enable_limit;
        raw.lowerTranslation = self.lower_translation;
        raw.upperTranslation = self.upper_translation;
        raw.enableMotor = self.enable_motor;
        raw.maxMotorForce = self.max_motor_force;
        raw.motorSpeed = self.motor_speed;
        raw
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        super::check_prismatic_joint_def_valid(self)
    }

    /// Enable/disable spring along the prismatic axis.
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
    /// Target spring translation along the prismatic axis (meters).
    pub fn translation(mut self, v: f32) -> Self {
        self.target_translation = v;
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
    /// Enable/disable translation limits.
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.enable_limit = flag;
        self
    }
    /// Enable/disable motor along the axis.
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.enable_motor = flag;
        self
    }
    /// Maximum motor force (N) along the axis.
    pub fn max_motor_force(mut self, v: f32) -> Self {
        self.max_motor_force = v;
        self
    }
    /// Motor speed (m/s) along the axis.
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.motor_speed = v;
        self
    }
}

/// Builder for a prismatic joint in world space.
/// Fluent builder for prismatic joints using world anchors and axis.
pub struct PrismaticJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) anchor_a_world: Option<Position>,
    pub(crate) anchor_b_world: Option<Position>,
    pub(crate) axis_world: Option<Vec2>,
    pub(crate) def: PrismaticJointDef,
}

impl<'w> PrismaticJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<Position>, VB: Into<Position>>(mut self, a: VA, b: VB) -> Self {
        self.anchor_a_world = Some(a.into());
        self.anchor_b_world = Some(b.into());
        self
    }
    /// Set prismatic axis in world space.
    pub fn axis_world<V: Into<Vec2>>(mut self, axis: V) -> Self {
        self.axis_world = Some(axis.into());
        self
    }
    /// Enable limits with lower/upper translation (meters).
    pub fn limit(mut self, lower: f32, upper: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .lower_translation(lower)
            .upper_translation(upper);
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
        self.def.base = self.def.base.with_collide_connected(flag);
        self
    }
    fn configure_local_frames(&mut self) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        super::creation::check_joint_target_identity(self.world, self.def.base())?;
        self.def.validate()?;
        if let Some(anchor) = self.anchor_a_world {
            super::validation::check_joint_position(
                anchor,
                "PrismaticJointBuilder::build",
                "anchor_a_world",
            )?;
        }
        if let Some(anchor) = self.anchor_b_world {
            super::validation::check_joint_position(
                anchor,
                "PrismaticJointBuilder::build",
                "anchor_b_world",
            )?;
        }
        let axis = self.axis_world.unwrap_or(Vec2::new(1.0, 0.0));
        super::validation::check_joint_axis(axis, "PrismaticJointBuilder::build", "axis_world")?;
        super::creation::check_joint_target_native(self.world, self.def.base())?;

        let body_a = self.def.base().body_a_id();
        let body_b = self.def.base().body_b_id();

        let ta = super::read_native_body_world_transform(
            "PrismaticJointBuilder::build",
            "body_a_transform",
            body_a,
        )?;
        let tb = super::read_native_body_world_transform(
            "PrismaticJointBuilder::build",
            "body_b_transform",
            body_b,
        )?;
        let aw = self.anchor_a_world.unwrap_or_else(|| ta.position());
        let bw = self.anchor_b_world.unwrap_or_else(|| tb.position());
        let la = super::base_def::checked_world_to_local_point(
            "PrismaticJointBuilder::build",
            "anchor_a_world",
            ta,
            aw,
        )?;
        let lb = super::base_def::checked_world_to_local_point(
            "PrismaticJointBuilder::build",
            "anchor_b_world",
            tb,
            bw,
        )?;
        let ra = super::base_def::checked_world_axis_to_local_rotation(
            "PrismaticJointBuilder::build",
            "axis_world",
            ta,
            axis,
        )?;
        let rb = super::base_def::checked_world_axis_to_local_rotation(
            "PrismaticJointBuilder::build",
            "axis_world",
            tb,
            axis,
        )?;
        self.def.base_mut().set_local_frames(
            crate::Transform::from_pos_angle(la, ra.angle())?,
            crate::Transform::from_pos_angle(lb, rb.angle())?,
        );
        Ok(())
    }

    pub fn build(mut self) -> Result<JointId> {
        self.configure_local_frames()?;
        self.world.create_prismatic_joint(&self.def)
    }
}
