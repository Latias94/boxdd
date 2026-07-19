use crate::body::Body;
use crate::core::math::Rot;
use crate::error::ApiResult;
use crate::types::{BodyId, Position, Vec2, WorldTransform};
use boxdd_sys::ffi;

use super::base::ConstraintTuning;

/// Convert an absolute world point into a body's local `f32` coordinates.
///
/// Box2D keeps joint frames in single precision even when absolute world positions use double
/// precision. Keep the narrowing at this boundary explicit and reject values that cannot be
/// represented by the local coordinate domain.
pub(crate) fn checked_world_to_local_point(
    body_transform: WorldTransform,
    world_point: Position,
) -> ApiResult<Vec2> {
    let rotation = body_transform.rotation();
    if !body_transform.position().is_valid() || !rotation.is_valid() || !world_point.is_valid() {
        return Err(crate::error::ApiError::InvalidArgument);
    }

    let relative = world_point
        .checked_relative_to(body_transform.position())
        .map_err(|_| crate::error::ApiError::InvalidArgument)?;
    let local = rotation.inv_rotate_vec(relative);
    if !local.x.is_finite() || !local.y.is_finite() {
        return Err(crate::error::ApiError::InvalidArgument);
    }

    Ok(local)
}

/// Convert a world-space direction into the local rotation for a joint frame.
pub(crate) fn checked_world_axis_to_local_rotation(
    body_transform: WorldTransform,
    world_axis: Vec2,
) -> ApiResult<Rot> {
    let rotation = body_transform.rotation();
    if !rotation.is_valid()
        || !world_axis.x.is_finite()
        || !world_axis.y.is_finite()
        || (world_axis.x == 0.0 && world_axis.y == 0.0)
    {
        return Err(crate::error::ApiError::InvalidArgument);
    }

    let local_axis = rotation.inv_rotate_vec(world_axis);
    if !local_axis.x.is_finite() || !local_axis.y.is_finite() {
        return Err(crate::error::ApiError::InvalidArgument);
    }

    Ok(Rot::from_radians(local_axis.y.atan2(local_axis.x)))
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
        // Box2D does not export a b2DefaultJointDef helper, so mirror the upstream defaults here.
        let mut base: ffi::b2JointDef = unsafe { core::mem::zeroed() };
        base.forceThreshold = f32::MAX;
        base.torqueThreshold = f32::MAX;
        base.constraintHertz = 60.0;
        base.constraintDampingRatio = 2.0;
        base.drawScale = crate::length_units_per_meter();
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

impl JointBase {
    /// Start building a new `JointBase` from defaults.
    pub fn builder() -> JointBaseBuilder {
        JointBaseBuilder::new()
    }

    /// Construct from the raw Box2D joint base definition value.
    #[inline]
    pub fn from_raw(raw: ffi::b2JointDef) -> Self {
        Self(raw)
    }

    /// Attached body A id.
    #[inline]
    pub fn body_a_id(&self) -> BodyId {
        BodyId::from_raw(self.0.bodyIdA)
    }

    /// Attached body B id.
    #[inline]
    pub fn body_b_id(&self) -> BodyId {
        BodyId::from_raw(self.0.bodyIdB)
    }

    /// Local frame on body A.
    #[inline]
    pub fn local_frame_a(&self) -> crate::Transform {
        crate::Transform::from_raw(self.0.localFrameA)
    }

    /// Local frame on body B.
    #[inline]
    pub fn local_frame_b(&self) -> crate::Transform {
        crate::Transform::from_raw(self.0.localFrameB)
    }

    /// Whether the connected bodies should collide with each other.
    #[inline]
    pub fn collide_connected(&self) -> bool {
        self.0.collideConnected
    }

    /// Force threshold used for joint events.
    #[inline]
    pub fn force_threshold(&self) -> f32 {
        self.0.forceThreshold
    }

    /// Torque threshold used for joint events.
    #[inline]
    pub fn torque_threshold(&self) -> f32 {
        self.0.torqueThreshold
    }

    /// Shared constraint tuning on the base definition.
    #[inline]
    pub fn constraint_tuning(&self) -> ConstraintTuning {
        ConstraintTuning::new(self.0.constraintHertz, self.0.constraintDampingRatio)
    }

    /// Debug draw scale.
    #[inline]
    pub fn draw_scale(&self) -> f32 {
        self.0.drawScale
    }

    /// Convert into the raw Box2D joint base definition value.
    #[inline]
    pub fn into_raw(self) -> ffi::b2JointDef {
        self.0
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_joint_base_valid(self)
    }
}

#[derive(Clone, Debug)]
pub struct JointBaseBuilder {
    base: JointBase,
}

impl JointBaseBuilder {
    pub(crate) fn checked_world_to_local_point(
        body_transform: WorldTransform,
        world_point: Position,
    ) -> ApiResult<Vec2> {
        checked_world_to_local_point(body_transform, world_point)
    }

    pub(crate) fn checked_world_axis_to_local_rotation(
        body_transform: WorldTransform,
        world_axis: Vec2,
    ) -> ApiResult<Rot> {
        checked_world_axis_to_local_rotation(body_transform, world_axis)
    }

    /// Create a new base with identity local frames.
    pub fn new() -> Self {
        Self {
            base: JointBase::default(),
        }
    }
    /// Attach two bodies using scoped body handles.
    pub fn bodies<'w>(mut self, a: &Body<'w>, b: &Body<'w>) -> Self {
        self.base.0.bodyIdA = a.id.into_raw();
        self.base.0.bodyIdB = b.id.into_raw();
        self
    }
    /// Attach two bodies by raw ids.
    pub fn bodies_by_id(mut self, a: BodyId, b: BodyId) -> Self {
        self.base.0.bodyIdA = a.into_raw();
        self.base.0.bodyIdB = b.into_raw();
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
            p: pos_a.into().into_raw(),
            q: ffi::b2Rot { c: ca, s: sa },
        };
        self.base.0.localFrameB = ffi::b2Transform {
            p: pos_b.into().into_raw(),
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
    /// Set local anchor positions from absolute world points (rotation remains identity).
    ///
    /// # Panics
    ///
    /// Panics if either world point cannot be represented in the corresponding local `f32`
    /// frame. Use [`Self::try_local_points_from_world`] for a recoverable error.
    pub fn local_points_from_world<'w, VA: Into<Position>, VB: Into<Position>>(
        self,
        body_a: &Body<'w>,
        world_a: VA,
        body_b: &Body<'w>,
        world_b: VB,
    ) -> Self {
        self.try_local_points_from_world(body_a, world_a, body_b, world_b)
            .expect("joint world anchors must be representable in local f32 frames")
    }

    /// Fallible variant of [`Self::local_points_from_world`].
    pub fn try_local_points_from_world<'w, VA: Into<Position>, VB: Into<Position>>(
        mut self,
        body_a: &Body<'w>,
        world_a: VA,
        body_b: &Body<'w>,
        world_b: VB,
    ) -> ApiResult<Self> {
        let ta = body_a.transform();
        let tb = body_b.transform();
        let la = checked_world_to_local_point(ta, world_a.into())?;
        let lb = checked_world_to_local_point(tb, world_b.into())?;
        let ident = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        let mut fa = ident;
        let mut fb = ident;
        fa.p = la.into_raw();
        fb.p = lb.into_raw();
        self.base.0.localFrameA = fa;
        self.base.0.localFrameB = fb;
        Ok(self)
    }
    pub fn build(self) -> JointBase {
        self.base
    }
    /// Set local frames using world anchors and a shared world axis (X-axis of joint frame).
    /// This computes localFrameA/B.rotation so that their X-axis aligns with the given world axis,
    /// and localFrameA/B.position to the given world anchor points.
    ///
    /// # Panics
    ///
    /// Panics if an anchor cannot be represented in a local `f32` frame or the axis is invalid.
    /// Use [`Self::try_frames_from_world_with_axis`] for a recoverable error.
    pub fn frames_from_world_with_axis<'w, VA, VB, AX>(
        self,
        body_a: &Body<'w>,
        anchor_a_world: VA,
        axis_world: AX,
        body_b: &Body<'w>,
        anchor_b_world: VB,
    ) -> Self
    where
        VA: Into<Position>,
        VB: Into<Position>,
        AX: Into<Vec2>,
    {
        self.try_frames_from_world_with_axis(
            body_a,
            anchor_a_world,
            axis_world,
            body_b,
            anchor_b_world,
        )
        .expect("joint world anchors and axis must define representable local f32 frames")
    }

    /// Fallible variant of [`Self::frames_from_world_with_axis`].
    pub fn try_frames_from_world_with_axis<'w, VA, VB, AX>(
        mut self,
        body_a: &Body<'w>,
        anchor_a_world: VA,
        axis_world: AX,
        body_b: &Body<'w>,
        anchor_b_world: VB,
    ) -> ApiResult<Self>
    where
        VA: Into<Position>,
        VB: Into<Position>,
        AX: Into<Vec2>,
    {
        let ta = body_a.transform();
        let tb = body_b.transform();
        let axis = axis_world.into();
        let la = checked_world_to_local_point(ta, anchor_a_world.into())?;
        let lb = checked_world_to_local_point(tb, anchor_b_world.into())?;
        let ra = checked_world_axis_to_local_rotation(ta, axis)?;
        let rb = checked_world_axis_to_local_rotation(tb, axis)?;
        self.base.0.localFrameA = ffi::b2Transform {
            p: la.into_raw(),
            q: ra.into_raw(),
        };
        self.base.0.localFrameB = ffi::b2Transform {
            p: lb.into_raw(),
            q: rb.into_raw(),
        };
        Ok(self)
    }
}

impl Default for JointBaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<JointBase> for JointBaseBuilder {
    fn from(base: JointBase) -> Self {
        Self { base }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_world_to_local_point_applies_inverse_body_rotation() {
        let transform = WorldTransform::from_pos_angle(Position::new(1000.0, -2000.0), 0.5);
        let world_point = transform.position().offset(Vec2::new(3.0, -2.0));

        let local = checked_world_to_local_point(transform, world_point).unwrap();
        let expected = Rot::from_radians(0.5).inv_rotate_vec(Vec2::new(3.0, -2.0));
        assert!((local.x - expected.x).abs() < 1.0e-5);
        assert!((local.y - expected.y).abs() < 1.0e-5);
    }

    #[cfg(feature = "double-precision")]
    #[test]
    fn checked_world_to_local_point_rejects_out_of_range_double_delta() {
        let transform = WorldTransform::IDENTITY;
        let world_point = Position::new(f64::from(f32::MAX) * 2.0, 0.0);

        assert_eq!(
            checked_world_to_local_point(transform, world_point),
            Err(crate::error::ApiError::InvalidArgument)
        );
    }

    #[test]
    fn checked_world_axis_rejects_zero_direction() {
        assert!(matches!(
            checked_world_axis_to_local_rotation(WorldTransform::IDENTITY, Vec2::ZERO),
            Err(crate::error::ApiError::InvalidArgument)
        ));
    }
}
