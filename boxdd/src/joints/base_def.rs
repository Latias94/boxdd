use crate::core::math::Rot;
use crate::error::{Error, Result};
use crate::types::{BodyId, Position, Vec2, WorldTransform};
use boxdd_sys::ffi;

use super::base::ConstraintTuning;

/// Convert an absolute world point into a body's local `f32` coordinates.
pub(crate) fn checked_world_to_local_point(
    operation: &'static str,
    argument: &'static str,
    body_transform: WorldTransform,
    world_point: Position,
) -> Result<Vec2> {
    let rotation = body_transform.rotation();
    if !body_transform.position().is_valid() || !rotation.is_valid() {
        return Err(Error::InvalidNativeOutput {
            operation,
            output: "body_transform",
            constraint: "a valid finite transform",
        });
    }
    super::validation::check_joint_position(world_point, operation, argument)?;

    let relative = world_point
        .checked_relative_to(body_transform.position())
        .map_err(|_| {
            Error::invalid_argument(
                operation,
                argument,
                "a point whose delta from the body origin fits in f32 coordinates",
            )
        })?;
    let local = rotation.inv_rotate_vec(relative);
    if !local.x.is_finite() || !local.y.is_finite() {
        return Err(Error::invalid_argument(
            operation,
            argument,
            "a point that produces finite local coordinates",
        ));
    }

    Ok(local)
}

/// Convert a world-space direction into the local rotation for a joint frame.
pub(crate) fn checked_world_axis_to_local_rotation(
    operation: &'static str,
    argument: &'static str,
    body_transform: WorldTransform,
    world_axis: Vec2,
) -> Result<Rot> {
    let rotation = body_transform.rotation();
    super::validation::check_joint_axis(world_axis, operation, argument)?;
    if !body_transform.position().is_valid() || !rotation.is_valid() {
        return Err(Error::InvalidNativeOutput {
            operation,
            output: "body_transform",
            constraint: "a valid finite transform",
        });
    }

    let local_axis = rotation.inv_rotate_vec(world_axis);
    if !local_axis.x.is_finite() || !local_axis.y.is_finite() {
        return Err(Error::invalid_argument(
            operation,
            argument,
            "a direction that produces finite local coordinates",
        ));
    }

    Rot::from_radians(local_axis.y.atan2(local_axis.x))
}

/// Common joint configuration that retains the provenance of both attached bodies.
///
/// The native `b2JointDef` is encoded only after a target [`crate::World`] validates both body
/// ids. Construct it through [`crate::World::joint_base`] or
/// [`crate::RecordingSession::joint_base`], which authenticate both ids under the same owner
/// before returning it. This prevents a definition from laundering a body id through its FFI
/// representation.
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
    length_scale: crate::core::length_scale::LengthScale,
}

impl JointBase {
    pub(crate) fn with_length_scale(
        body_a: BodyId,
        body_b: BodyId,
        length_scale: crate::core::length_scale::LengthScale,
    ) -> Self {
        Self {
            body_a,
            body_b,
            local_frame_a: crate::Transform::IDENTITY,
            local_frame_b: crate::Transform::IDENTITY,
            collide_connected: false,
            force_threshold: f32::MAX,
            torque_threshold: f32::MAX,
            constraint_tuning: ConstraintTuning::new(60.0, 2.0)
                .expect("Box2D's default joint tuning is valid"),
            draw_scale: length_scale.units_per_meter(),
            length_scale,
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
    pub(crate) fn length_scale(&self) -> crate::core::length_scale::LengthScale {
        self.length_scale
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
    ) -> Result<Self> {
        Ok(self.with_local_frames(
            crate::Transform::from_pos_angle(position_a.into(), angle_a)?,
            crate::Transform::from_pos_angle(position_b.into(), angle_b)?,
        ))
    }

    #[inline]
    pub fn with_collide_connected(mut self, collide_connected: bool) -> Self {
        self.collide_connected = collide_connected;
        self
    }

    #[inline]
    pub fn with_force_threshold(mut self, force_threshold: f32) -> Result<Self> {
        super::check_joint_non_negative(
            force_threshold,
            "JointBase::with_force_threshold",
            "force_threshold",
        )?;
        self.force_threshold = force_threshold;
        Ok(self)
    }

    #[inline]
    pub fn with_torque_threshold(mut self, torque_threshold: f32) -> Result<Self> {
        super::check_joint_non_negative(
            torque_threshold,
            "JointBase::with_torque_threshold",
            "torque_threshold",
        )?;
        self.torque_threshold = torque_threshold;
        Ok(self)
    }

    #[inline]
    pub fn with_constraint_tuning(mut self, constraint_tuning: ConstraintTuning) -> Self {
        self.constraint_tuning = constraint_tuning;
        self
    }

    #[inline]
    pub fn with_draw_scale(mut self, draw_scale: f32) -> Result<Self> {
        super::check_joint_non_negative(draw_scale, "JointBase::with_draw_scale", "draw_scale")?;
        self.draw_scale = draw_scale;
        Ok(self)
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
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
            constraintHertz: self.constraint_tuning.hertz(),
            constraintDampingRatio: self.constraint_tuning.damping_ratio(),
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
        let transform =
            WorldTransform::from_pos_angle(Position::new(1000.0, -2000.0), 0.5).unwrap();
        let world_point = transform.position().offset(Vec2::new(3.0, -2.0));

        let local = checked_world_to_local_point(
            "test_checked_world_to_local_point",
            "world_point",
            transform,
            world_point,
        )
        .unwrap();
        let expected = Rot::from_radians(0.5)
            .unwrap()
            .inv_rotate_vec(Vec2::new(3.0, -2.0));
        assert!((local.x - expected.x).abs() < 1.0e-5);
        assert!((local.y - expected.y).abs() < 1.0e-5);
    }

    #[cfg(feature = "double-precision")]
    #[test]
    fn checked_world_to_local_point_rejects_out_of_range_double_delta() {
        let transform = WorldTransform::IDENTITY;
        let world_point = Position::new(f64::from(f32::MAX) * 2.0, 0.0);

        assert_eq!(
            checked_world_to_local_point(
                "test_checked_world_to_local_point",
                "world_point",
                transform,
                world_point,
            ),
            Err(Error::invalid_argument(
                "test_checked_world_to_local_point",
                "world_point",
                "a point whose delta from the body origin fits in f32 coordinates",
            ))
        );
    }

    #[test]
    fn checked_world_axis_rejects_zero_direction() {
        assert!(matches!(
            checked_world_axis_to_local_rotation(
                "test_checked_world_axis_to_local_rotation",
                "world_axis",
                WorldTransform::IDENTITY,
                Vec2::ZERO,
            ),
            Err(Error::InvalidArgument { .. })
        ));
    }
}
