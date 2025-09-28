use std::marker::PhantomData;

use crate::body::Body;
use crate::types::{BodyId, JointId};
use crate::world::World;
use boxdd_sys::ffi;

/// A joint owned by a world; drops by destroying the underlying joint.
pub struct Joint<'w> {
    pub(crate) id: ffi::b2JointId,
    pub(crate) _world: PhantomData<&'w World>,
}

impl<'w> Joint<'w> {
    pub fn id(&self) -> JointId {
        self.id
    }
    pub fn linear_separation(&self) -> f32 {
        unsafe { ffi::b2Joint_GetLinearSeparation(self.id) }
    }
    pub fn angular_separation(&self) -> f32 {
        unsafe { ffi::b2Joint_GetAngularSeparation(self.id) }
    }
}

impl<'w> Drop for Joint<'w> {
    fn drop(&mut self) {
        if unsafe { ffi::b2Joint_IsValid(self.id) } {
            unsafe { ffi::b2DestroyJoint(self.id, true) };
        }
    }
}

/// Base joint definition builder for common properties.
///
/// This configures `b2JointDef` fields shared by all joint types. Typically
/// you construct a specific joint def (e.g. `RevoluteJointDef`) with this as
/// its `base`.
#[derive(Clone, Debug)]
pub struct JointBase(pub(crate) ffi::b2JointDef);

impl Default for JointBase {
    fn default() -> Self {
        // No default constructor provided for b2JointDef; zero is OK for POD and we'll set fields explicitly.
        // Use identity frames by default.
        let mut base: ffi::b2JointDef = unsafe { core::mem::zeroed() };
        base.drawScale = 1.0;
        base.localFrameA = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        base.localFrameB = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        Self(base)
    }
}

#[derive(Clone, Debug)]
pub struct JointBaseBuilder {
    base: JointBase,
}

impl JointBaseBuilder {
    /// Create a new base with identity local frames.
    pub fn new() -> Self {
        Self {
            base: JointBase::default(),
        }
    }
    /// Attach two bodies using RAII wrappers.
    pub fn bodies<'w>(mut self, a: &Body<'w>, b: &Body<'w>) -> Self {
        self.base.0.bodyIdA = a.id;
        self.base.0.bodyIdB = b.id;
        self
    }
    /// Attach two bodies by raw ids.
    pub fn bodies_by_id(mut self, a: BodyId, b: BodyId) -> Self {
        self.base.0.bodyIdA = a;
        self.base.0.bodyIdB = b;
        self
    }
    /// Set local frames from positions and angles (radians).
    pub fn local_frames<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        pos_a: VA,
        angle_a: f32,
        pos_b: VB,
        angle_b: f32,
    ) -> Self {
        let (sa, ca) = angle_a.sin_cos();
        let (sb, cb) = angle_b.sin_cos();
        self.base.0.localFrameA = ffi::b2Transform {
            p: ffi::b2Vec2::from(pos_a.into()),
            q: ffi::b2Rot { c: ca, s: sa },
        };
        self.base.0.localFrameB = ffi::b2Transform {
            p: ffi::b2Vec2::from(pos_b.into()),
            q: ffi::b2Rot { c: cb, s: sb },
        };
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.base.0.collideConnected = flag;
        self
    }
    /// Force threshold for joint events.
    pub fn force_threshold(mut self, v: f32) -> Self {
        self.base.0.forceThreshold = v;
        self
    }
    /// Torque threshold for joint events.
    pub fn torque_threshold(mut self, v: f32) -> Self {
        self.base.0.torqueThreshold = v;
        self
    }
    /// Advanced constraint tuning frequency in Hertz.
    pub fn constraint_hertz(mut self, v: f32) -> Self {
        self.base.0.constraintHertz = v;
        self
    }
    /// Advanced constraint damping ratio.
    pub fn constraint_damping_ratio(mut self, v: f32) -> Self {
        self.base.0.constraintDampingRatio = v;
        self
    }
    pub fn draw_scale(mut self, v: f32) -> Self {
        self.base.0.drawScale = v;
        self
    }
    pub fn local_frames_raw(mut self, a: ffi::b2Transform, b: ffi::b2Transform) -> Self {
        self.base.0.localFrameA = a;
        self.base.0.localFrameB = b;
        self
    }
    /// Set local anchor positions from world points (rotation remains identity).
    pub fn local_points_from_world<'w, V: Into<crate::types::Vec2>>(
        mut self,
        body_a: &Body<'w>,
        world_a: V,
        body_b: &Body<'w>,
        world_b: V,
    ) -> Self {
        let ta = body_a.transform();
        let tb = body_b.transform();
        let wa: ffi::b2Vec2 = world_a.into().into();
        let wb: ffi::b2Vec2 = world_b.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ident = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        let mut fa = ident;
        let mut fb = ident;
        fa.p = la;
        fb.p = lb;
        self.base.0.localFrameA = fa;
        self.base.0.localFrameB = fb;
        self
    }
    pub fn build(self) -> JointBase {
        self.base
    }
    /// Set local frames using world anchors and a shared world axis (X-axis of joint frame).
    /// This computes localFrameA/B.rotation so that their X-axis aligns with the given world axis,
    /// and localFrameA/B.position to the given world anchor points.
    pub fn frames_from_world_with_axis<'w, VA, VB, AX>(
        mut self,
        body_a: &Body<'w>,
        anchor_a_world: VA,
        axis_world: AX,
        body_b: &Body<'w>,
        anchor_b_world: VB,
    ) -> Self
    where
        VA: Into<crate::types::Vec2>,
        VB: Into<crate::types::Vec2>,
        AX: Into<crate::types::Vec2>,
    {
        let ta = body_a.transform();
        let tb = body_b.transform();
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis_w: ffi::b2Vec2 = axis_world.into().into();
        // Local frames: positions from anchors, rotations from world axis
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis_w);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis_w);
        self.base.0.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.base.0.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self
    }
}

impl Default for JointBaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
