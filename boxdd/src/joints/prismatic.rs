#![allow(rustdoc::broken_intra_doc_links)]
use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint};
use crate::error::ApiResult;

// Prismatic joint
#[derive(Clone, Debug)]
/// Prismatic (slider) joint definition (maps to `b2PrismaticJointDef`).
///
/// Constrains two bodies to slide along an axis with optional limits, motor,
/// and spring (stiffness/damping). Use with `World::create_prismatic_joint(_id)`
/// or `World::prismatic(...).build()`.
pub struct PrismaticJointDef(pub(crate) ffi::b2PrismaticJointDef);

impl PrismaticJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2PrismaticJointDef = unsafe { ffi::b2DefaultPrismaticJointDef() };
        def.base = base.0;
        Self(def)
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
    pub fn minimum_translation(&self) -> f32 {
        self.0.lowerTranslation
    }

    #[inline]
    pub fn maximum_translation(&self) -> f32 {
        self.0.upperTranslation
    }

    #[inline]
    pub fn limit_enabled(&self) -> bool {
        self.0.enableLimit
    }

    #[inline]
    pub fn motor_enabled(&self) -> bool {
        self.0.enableMotor
    }

    #[inline]
    pub fn maximum_motor_force(&self) -> f32 {
        self.0.maxMotorForce
    }

    #[inline]
    pub fn target_motor_speed(&self) -> f32 {
        self.0.motorSpeed
    }

    /// Enable/disable spring along the prismatic axis.
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
    /// Enable/disable translation limits.
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    /// Enable/disable motor along the axis.
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    /// Maximum motor force (N) along the axis.
    pub fn max_motor_force(mut self, v: f32) -> Self {
        self.0.maxMotorForce = v;
        self
    }
    /// Motor speed (m/s) along the axis.
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

/// Builder for a prismatic joint in world space.
/// Fluent builder for prismatic joints using world anchors and axis.
pub struct PrismaticJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) anchor_a_world: Option<ffi::b2Vec2>,
    pub(crate) anchor_b_world: Option<ffi::b2Vec2>,
    pub(crate) axis_world: Option<ffi::b2Vec2>,
    pub(crate) def: PrismaticJointDef,
}

impl<'w> PrismaticJointBuilder<'w> {
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
    /// Set prismatic axis in world space.
    pub fn axis_world<V: Into<crate::types::Vec2>>(mut self, axis: V) -> Self {
        self.axis_world = Some(ffi::b2Vec2::from(axis.into()));
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
    /// Enable motor with maximum force (N) and speed (deg/s).
    pub fn motor_deg(mut self, max_force: f32, speed_deg: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_force(max_force)
            .motor_speed_deg(speed_deg);
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
    /// Stiffness on both linear and angular degrees of freedom.
    ///
    /// - linear_hz/angular_hz: stiffness (Hz), typical 4–20
    /// - linear_dr/angular_dr: damping ratio \[0, 1], typical 0.1–0.7
    pub fn with_stiffness(
        mut self,
        linear_hz: f32,
        linear_dr: f32,
        angular_hz: f32,
        angular_dr: f32,
    ) -> Self {
        self = self.linear_stiffness(linear_hz, linear_dr);
        self = self.angular_stiffness(angular_hz, angular_dr);
        self
    }
    pub fn linear_stiffness(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self.def.hertz(hertz).damping_ratio(damping_ratio);
        self
    }
    pub fn angular_stiffness(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        // Defaults: anchors = body positions, axis = x
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = self.body_a;
        self.def.0.base.bodyIdB = self.body_b;
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.create_prismatic_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = self.body_a;
        self.def.0.base.bodyIdB = self.body_b;
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.try_create_prismatic_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        // Defaults: anchors = body positions, axis = x
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = self.body_a;
        self.def.0.base.bodyIdB = self.body_b;
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.create_prismatic_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        self.def.0.base.bodyIdA = self.body_a;
        self.def.0.base.bodyIdB = self.body_b;
        self.def.0.base.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.def.0.base.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self.world.try_create_prismatic_joint_owned(&self.def)
    }
}
