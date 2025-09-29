use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, JointBaseBuilder};

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
    pub(crate) anchor_world: Option<ffi::b2Vec2>,
    pub(crate) def: WeldJointDef,
}

impl<'w> WeldJointBuilder<'w> {
    /// Set world-space anchor (defaults to body A position).
    pub fn anchor_world<V: Into<crate::types::Vec2>>(mut self, a: V) -> Self {
        self.anchor_world = Some(ffi::b2Vec2::from(a.into()));
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

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_world.unwrap_or(ta.p);
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
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
        self.world.create_weld_joint(&self.def)
    }
}
