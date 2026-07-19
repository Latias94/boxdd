use crate::types::{BodyId, Position, WorldTransform};
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint, raw_body_id};
use crate::error::ApiResult;

// Weld joint
#[derive(Clone, Debug)]
/// Weld joint definition (maps to `b2WeldJointDef`). Rigidly attaches two
/// bodies at an anchor with optional soft-constraint tuning.
pub struct WeldJointDef(pub(crate) ffi::b2WeldJointDef);

impl WeldJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2WeldJointDef = unsafe { ffi::b2DefaultWeldJointDef() };
        def.base = base.0;
        Self(def)
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2WeldJointDef) -> Self {
        Self(raw)
    }

    #[inline]
    pub fn base(&self) -> JointBase {
        JointBase(self.0.base)
    }

    #[inline]
    pub fn configured_linear_hertz(&self) -> f32 {
        self.0.linearHertz
    }

    #[inline]
    pub fn configured_angular_hertz(&self) -> f32 {
        self.0.angularHertz
    }

    #[inline]
    pub fn configured_linear_damping_ratio(&self) -> f32 {
        self.0.linearDampingRatio
    }

    #[inline]
    pub fn configured_angular_damping_ratio(&self) -> f32 {
        self.0.angularDampingRatio
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2WeldJointDef {
        self.0
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_weld_joint_def_valid(self)
    }

    /// Linear stiffness (Hz) for weld constraint.
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.0.linearHertz = v;
        self
    }
    /// Angular stiffness (Hz) for weld constraint.
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.0.angularHertz = v;
        self
    }
    /// Linear damping ratio \[0,1].
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.0.linearDampingRatio = v;
        self
    }
    /// Angular damping ratio \[0,1].
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.0.angularDampingRatio = v;
        self
    }
}

// Weld joint convenience builder
/// Fluent builder for weld joints using a world anchor.
pub struct WeldJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) anchor_world: Option<Position>,
    pub(crate) def: WeldJointDef,
}

impl<'w> WeldJointBuilder<'w> {
    /// Set world-space anchor (defaults to body A position).
    pub fn anchor_world<V: Into<Position>>(mut self, a: V) -> Self {
        self.anchor_world = Some(a.into());
        self
    }
    pub fn linear_stiffness(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .linear_hertz(hertz)
            .linear_damping_ratio(damping_ratio);
        self
    }
    pub fn angular_stiffness(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .angular_hertz(hertz)
            .angular_damping_ratio(damping_ratio);
        self
    }
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
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    fn configure_local_frames(&mut self) -> ApiResult<()> {
        let ta =
            WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_a)) });
        let tb =
            WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(self.body_b)) });
        let anchor = self.anchor_world.unwrap_or_else(|| ta.position());
        let la = super::base_def::checked_world_to_local_point(ta, anchor)?;
        let lb = super::base_def::checked_world_to_local_point(tb, anchor)?;
        self.def.0.base.bodyIdA = raw_body_id(self.body_a);
        self.def.0.base.bodyIdB = raw_body_id(self.body_b);
        self.def.0.base.localFrameA = ffi::b2Transform {
            p: la.into_raw(),
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        self.def.0.base.localFrameB = ffi::b2Transform {
            p: lb.into_raw(),
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        Ok(())
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        self.configure_local_frames()
            .expect("weld-joint world anchor must fit in both local f32 frames");
        self.world.create_weld_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        self.configure_local_frames()?;
        self.world.try_create_weld_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        crate::core::debug_checks::assert_body_valid(self.body_a);
        crate::core::debug_checks::assert_body_valid(self.body_b);
        self.configure_local_frames()
            .expect("weld-joint world anchor must fit in both local f32 frames");
        self.world.create_weld_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        crate::core::debug_checks::check_body_valid(self.body_a)?;
        crate::core::debug_checks::check_body_valid(self.body_b)?;
        self.configure_local_frames()?;
        self.world.try_create_weld_joint_owned(&self.def)
    }
}
