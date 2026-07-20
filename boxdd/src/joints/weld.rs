use crate::types::{Position, WorldTransform};
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint, raw_body_id};
use crate::error::ApiResult;

// Weld joint
#[derive(Clone, Debug)]
/// Weld joint definition (maps to `b2WeldJointDef`). Rigidly attaches two
/// bodies at an anchor with optional soft-constraint tuning.
pub struct WeldJointDef {
    base: JointBase,
    linear_hertz: f32,
    angular_hertz: f32,
    linear_damping_ratio: f32,
    angular_damping_ratio: f32,
}

impl WeldJointDef {
    pub fn new(base: JointBase) -> Self {
        let raw = unsafe { ffi::b2DefaultWeldJointDef() };
        Self {
            base,
            linear_hertz: raw.linearHertz,
            angular_hertz: raw.angularHertz,
            linear_damping_ratio: raw.linearDampingRatio,
            angular_damping_ratio: raw.angularDampingRatio,
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
    pub(crate) fn to_raw(&self) -> ffi::b2WeldJointDef {
        let mut raw = unsafe { ffi::b2DefaultWeldJointDef() };
        raw.base = self.base.to_raw();
        raw.linearHertz = self.linear_hertz;
        raw.angularHertz = self.angular_hertz;
        raw.linearDampingRatio = self.linear_damping_ratio;
        raw.angularDampingRatio = self.angular_damping_ratio;
        raw
    }

    #[inline]
    pub fn configured_linear_hertz(&self) -> f32 {
        self.linear_hertz
    }

    #[inline]
    pub fn configured_angular_hertz(&self) -> f32 {
        self.angular_hertz
    }

    #[inline]
    pub fn configured_linear_damping_ratio(&self) -> f32 {
        self.linear_damping_ratio
    }

    #[inline]
    pub fn configured_angular_damping_ratio(&self) -> f32 {
        self.angular_damping_ratio
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_weld_joint_def_valid(self)
    }

    /// Linear stiffness (Hz) for weld constraint.
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.linear_hertz = v;
        self
    }
    /// Angular stiffness (Hz) for weld constraint.
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.angular_hertz = v;
        self
    }
    /// Linear damping ratio \[0,1].
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.linear_damping_ratio = v;
        self
    }
    /// Angular damping ratio \[0,1].
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.angular_damping_ratio = v;
        self
    }
}

// Weld joint convenience builder
/// Fluent builder for weld joints using a world anchor.
pub struct WeldJointBuilder<'w> {
    pub(crate) world: &'w mut World,
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
            .expect("weld-joint bodies must belong to the world and anchor must fit local frames");
        self.world.create_weld_joint(&self.def)
    }

    pub fn try_build(mut self) -> ApiResult<Joint<'w>> {
        self.configure_local_frames()?;
        self.world.try_create_weld_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(mut self) -> OwnedJoint {
        self.configure_local_frames()
            .expect("weld-joint bodies must belong to the world and anchor must fit local frames");
        self.world.create_weld_joint_owned(&self.def)
    }

    pub fn try_build_owned(mut self) -> ApiResult<OwnedJoint> {
        self.configure_local_frames()?;
        self.world.try_create_weld_joint_owned(&self.def)
    }
}
