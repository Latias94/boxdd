use crate::error::{ApiError, ApiResult};
use crate::types::{Position, Vec2};

use super::ConstraintTuning;

pub(crate) const REVOLUTE_LIMIT_ABS_MAX: f32 = 0.99 * core::f32::consts::PI;

#[inline]
pub(crate) fn check_joint_condition(condition: bool) -> ApiResult<()> {
    if condition {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn check_joint_finite(value: f32) -> ApiResult<()> {
    check_joint_condition(value.is_finite())
}

#[inline]
pub(crate) fn check_joint_non_negative(value: f32) -> ApiResult<()> {
    check_joint_condition(value.is_finite() && value >= 0.0)
}

#[inline]
pub(crate) fn check_joint_positive(value: f32) -> ApiResult<()> {
    check_joint_condition(value.is_finite() && value > 0.0)
}

#[inline]
pub(crate) fn check_joint_vec2(value: Vec2) -> ApiResult<()> {
    check_joint_condition(value.is_valid())
}

#[inline]
pub(crate) fn check_joint_position(value: Position) -> ApiResult<()> {
    check_joint_condition(value.is_valid())
}

#[inline]
pub(crate) fn check_joint_axis(value: Vec2) -> ApiResult<()> {
    check_joint_vec2(value)?;
    check_joint_condition(value != Vec2::ZERO)
}

#[inline]
pub(crate) fn check_joint_tuning(value: ConstraintTuning) -> ApiResult<()> {
    check_joint_non_negative(value.hertz)?;
    check_joint_non_negative(value.damping_ratio)
}

#[inline]
pub(crate) fn check_joint_transform(value: crate::Transform) -> ApiResult<()> {
    check_joint_condition(value.is_valid())
}

#[inline]
pub(crate) fn check_joint_ordered_range(lower: f32, upper: f32) -> ApiResult<()> {
    check_joint_finite(lower)?;
    check_joint_finite(upper)?;
    check_joint_condition(lower <= upper)
}

#[inline]
pub(crate) fn check_joint_non_negative_range(lower: f32, upper: f32) -> ApiResult<()> {
    check_joint_non_negative(lower)?;
    check_joint_non_negative(upper)?;
    check_joint_condition(lower <= upper)
}

#[inline]
pub(crate) fn check_revolute_joint_range(lower: f32, upper: f32) -> ApiResult<()> {
    check_joint_ordered_range(lower, upper)?;
    check_joint_condition(lower >= -REVOLUTE_LIMIT_ABS_MAX && upper <= REVOLUTE_LIMIT_ABS_MAX)
}
