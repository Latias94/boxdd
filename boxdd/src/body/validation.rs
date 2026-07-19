use crate::Rot;
use crate::error::{ApiError, ApiResult};
use crate::types::{Position, Vec2, WorldTransform};
use std::ffi::CString;

#[inline]
#[track_caller]
pub(crate) fn assert_valid_body_name(name: &str) -> CString {
    let name = CString::new(name).expect("body name contains an interior NUL byte");
    assert!(
        name.as_bytes().len() <= super::MAX_BODY_NAME_BYTES,
        "body name must contain at most {} UTF-8 bytes",
        super::MAX_BODY_NAME_BYTES
    );
    name
}

#[inline]
pub(crate) fn check_valid_body_name(name: &str) -> ApiResult<CString> {
    let name = CString::new(name).map_err(|_| ApiError::NulByteInString)?;
    if name.as_bytes().len() <= super::MAX_BODY_NAME_BYTES {
        Ok(name)
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_valid_body_vec2(name: &str, value: Vec2) -> Vec2 {
    assert!(
        value.is_valid(),
        "{name} must be a finite Box2D vector, got {value:?}"
    );
    value
}

#[inline]
pub(crate) fn check_valid_body_vec2(value: Vec2) -> ApiResult<Vec2> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_valid_body_position(name: &str, value: Position) -> Position {
    assert!(
        value.is_valid(),
        "{name} must be a finite Box2D world position, got {value:?}"
    );
    value
}

#[inline]
pub(crate) fn check_valid_body_position(value: Position) -> ApiResult<Position> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_valid_body_float(name: &str, value: f32) -> f32 {
    assert!(value.is_finite(), "{name} must be finite, got {value}");
    value
}

#[inline]
pub(crate) fn check_valid_body_float(value: f32) -> ApiResult<f32> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_body_world_point_in_local_range(
    name: &str,
    point: Position,
    origin: Position,
) -> Position {
    assert!(
        point.checked_relative_to(origin).is_ok(),
        "{name} must be finite and its offset from the body must fit in a local f32 vector, got {point:?}"
    );
    point
}

#[inline]
pub(crate) fn check_body_world_point_in_local_range(
    point: Position,
    origin: Position,
) -> ApiResult<Position> {
    if point.checked_relative_to(origin).is_ok() {
        Ok(point)
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn body_target_motion_is_valid(
    target: WorldTransform,
    time_step: f32,
    current_center: Position,
    current_rotation: Rot,
    local_center: Vec2,
) -> bool {
    if !target.is_valid()
        || !current_center.is_valid()
        || !current_rotation.is_valid()
        || !local_center.is_valid()
        || !time_step.is_finite()
        || time_step <= 0.0
    {
        return false;
    }

    let Ok(delta) = target
        .transform_point(local_center)
        .checked_relative_to(current_center)
    else {
        return false;
    };
    let inverse_time_step = 1.0 / time_step;
    if !inverse_time_step.is_finite()
        || !(delta.x * inverse_time_step).is_finite()
        || !(delta.y * inverse_time_step).is_finite()
    {
        return false;
    }

    let target_rotation = target.rotation();
    let sin_delta = current_rotation.cosine() * target_rotation.sine()
        - current_rotation.sine() * target_rotation.cosine();
    let cos_delta = current_rotation.cosine() * target_rotation.cosine()
        + current_rotation.sine() * target_rotation.sine();
    (sin_delta.atan2(cos_delta) * inverse_time_step).is_finite()
}

#[inline]
#[track_caller]
pub(crate) fn assert_valid_body_target_motion(
    target: WorldTransform,
    time_step: f32,
    current_center: Position,
    current_rotation: Rot,
    local_center: Vec2,
) -> (WorldTransform, f32) {
    assert!(
        body_target_motion_is_valid(
            target,
            time_step,
            current_center,
            current_rotation,
            local_center,
        ),
        "target and time_step must produce finite linear and angular velocities"
    );
    (target, time_step)
}

#[inline]
pub(crate) fn check_valid_body_target_motion(
    target: WorldTransform,
    time_step: f32,
    current_center: Position,
    current_rotation: Rot,
    local_center: Vec2,
) -> ApiResult<(WorldTransform, f32)> {
    if body_target_motion_is_valid(
        target,
        time_step,
        current_center,
        current_rotation,
        local_center,
    ) {
        Ok((target, time_step))
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[cfg(test)]
mod tests {
    use super::{assert_valid_body_name, check_valid_body_name};
    use crate::error::ApiError;

    #[test]
    fn body_names_are_bounded_by_utf8_bytes() {
        assert!(check_valid_body_name("1234567890").is_ok());
        assert_eq!(
            check_valid_body_name("12345678901").unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            check_valid_body_name("eeeee")
                .expect("ASCII name")
                .as_bytes()
                .len(),
            5
        );
        assert_eq!(
            check_valid_body_name("\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}")
                .expect("five two-byte characters fit exactly")
                .as_bytes()
                .len(),
            10
        );
        assert_eq!(
            check_valid_body_name("\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}").unwrap_err(),
            ApiError::InvalidArgument
        );
        assert!(std::panic::catch_unwind(|| assert_valid_body_name("12345678901")).is_err());
    }
}
