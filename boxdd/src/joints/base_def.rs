use crate::core::math::Rot;
use crate::error::{ApiError, ApiResult};
use crate::types::{BodyId, Position, Vec2, WorldTransform};
use boxdd_sys::ffi;

use super::base::ConstraintTuning;

/// Convert an absolute world point into a body's local `f32` coordinates.
pub(crate) fn checked_world_to_local_point(
    body_transform: WorldTransform,
    world_point: Position,
) -> ApiResult<Vec2> {
    let rotation = body_transform.rotation();
    if !body_transform.position().is_valid() || !rotation.is_valid() || !world_point.is_valid() {
        return Err(ApiError::InvalidArgument);
    }

    let relative = world_point
        .checked_relative_to(body_transform.position())
        .map_err(|_| ApiError::InvalidArgument)?;
    let local = rotation.inv_rotate_vec(relative);
    if !local.x.is_finite() || !local.y.is_finite() {
        return Err(ApiError::InvalidArgument);
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
        return Err(ApiError::InvalidArgument);
    }

    let local_axis = rotation.inv_rotate_vec(world_axis);
    if !local_axis.x.is_finite() || !local_axis.y.is_finite() {
        return Err(ApiError::InvalidArgument);
    }

    Ok(Rot::from_radians(local_axis.y.atan2(local_axis.x)))
}

/// Common joint configuration that retains the provenance of both attached bodies.
///
/// The native `b2JointDef` is encoded only after a target [`crate::World`] validates both body
/// ids. This prevents a definition from laundering a body id through its FFI representation.
#[derive(Copy, Clone, Debug)]
pub struct JointBase {
    body_a: BodyId,
    body_b: BodyId,
    local_frame_a: crate::Transform,
    local_frame_b: crate::Transform,
    collide_connected: bool,
    force_threshold: f32,
    torque_threshold: f32,
    constraint_tuning: ConstraintTuning,
    draw_scale: f32,
}

impl JointBase {
    /// Create a joint base for two branded body ids.
    pub fn new(body_a: BodyId, body_b: BodyId) -> Self {
        Self {
            body_a,
            body_b,
            local_frame_a: crate::Transform::IDENTITY,
            local_frame_b: crate::Transform::IDENTITY,
            collide_connected: false,
            force_threshold: f32::MAX,
            torque_threshold: f32::MAX,
            constraint_tuning: ConstraintTuning::new(60.0, 2.0),
            draw_scale: crate::length_units_per_meter(),
        }
    }

    #[inline]
    pub fn body_a_id(&self) -> BodyId {
        self.body_a
    }

    #[inline]
    pub fn body_b_id(&self) -> BodyId {
        self.body_b
    }

    #[inline]
    pub fn local_frame_a(&self) -> crate::Transform {
        self.local_frame_a
    }

    #[inline]
    pub fn local_frame_b(&self) -> crate::Transform {
        self.local_frame_b
    }

    #[inline]
    pub fn collide_connected(&self) -> bool {
        self.collide_connected
    }

    #[inline]
    pub fn force_threshold(&self) -> f32 {
        self.force_threshold
    }

    #[inline]
    pub fn torque_threshold(&self) -> f32 {
        self.torque_threshold
    }

    #[inline]
    pub fn constraint_tuning(&self) -> ConstraintTuning {
        self.constraint_tuning
    }

    #[inline]
    pub fn draw_scale(&self) -> f32 {
        self.draw_scale
    }

    #[inline]
    pub fn with_local_frames(
        mut self,
        local_frame_a: crate::Transform,
        local_frame_b: crate::Transform,
    ) -> Self {
        self.local_frame_a = local_frame_a;
        self.local_frame_b = local_frame_b;
        self
    }

    /// Set local frames from positions and angles in radians.
    pub fn with_local_frame_components<VA: Into<Vec2>, VB: Into<Vec2>>(
        self,
        position_a: VA,
        angle_a: f32,
        position_b: VB,
        angle_b: f32,
    ) -> Self {
        self.with_local_frames(
            crate::Transform::from_pos_angle(position_a.into(), angle_a),
            crate::Transform::from_pos_angle(position_b.into(), angle_b),
        )
    }

    #[inline]
    pub fn with_collide_connected(mut self, collide_connected: bool) -> Self {
        self.collide_connected = collide_connected;
        self
    }

    #[inline]
    pub fn with_force_threshold(mut self, force_threshold: f32) -> Self {
        self.force_threshold = force_threshold;
        self
    }

    #[inline]
    pub fn with_torque_threshold(mut self, torque_threshold: f32) -> Self {
        self.torque_threshold = torque_threshold;
        self
    }

    #[inline]
    pub fn with_constraint_tuning(mut self, constraint_tuning: ConstraintTuning) -> Self {
        self.constraint_tuning = constraint_tuning;
        self
    }

    #[inline]
    pub fn with_draw_scale(mut self, draw_scale: f32) -> Self {
        self.draw_scale = draw_scale;
        self
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        super::check_joint_base_valid(self)
    }

    pub(crate) fn set_local_frames(
        &mut self,
        local_frame_a: crate::Transform,
        local_frame_b: crate::Transform,
    ) {
        self.local_frame_a = local_frame_a;
        self.local_frame_b = local_frame_b;
    }

    pub(crate) fn to_raw(self) -> ffi::b2JointDef {
        ffi::b2JointDef {
            userData: core::ptr::null_mut(),
            bodyIdA: self.body_a.into_raw(),
            bodyIdB: self.body_b.into_raw(),
            localFrameA: self.local_frame_a.into_raw(),
            localFrameB: self.local_frame_b.into_raw(),
            forceThreshold: self.force_threshold,
            torqueThreshold: self.torque_threshold,
            constraintHertz: self.constraint_tuning.hertz,
            constraintDampingRatio: self.constraint_tuning.damping_ratio,
            drawScale: self.draw_scale,
            collideConnected: self.collide_connected,
        }
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
            Err(ApiError::InvalidArgument)
        );
    }

    #[test]
    fn checked_world_axis_rejects_zero_direction() {
        assert!(matches!(
            checked_world_axis_to_local_rotation(WorldTransform::IDENTITY, Vec2::ZERO),
            Err(ApiError::InvalidArgument)
        ));
    }
}
