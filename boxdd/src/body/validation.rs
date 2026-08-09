use crate::Rot;
use crate::error::{Error, Result};
use crate::types::{Position, Vec2, WorldTransform};
use std::ffi::CString;

#[inline]
pub(crate) fn check_valid_body_name(operation: &'static str, name: &str) -> Result<CString> {
    let name = CString::new(name).map_err(|_| Error::NulByteInString)?;
    if name.as_bytes().len() <= super::MAX_BODY_NAME_BYTES {
        Ok(name)
    } else {
        Err(Error::invalid_argument(
            operation,
            "name",
            "at most 10 UTF-8 bytes",
        ))
    }
}

#[inline]
pub(crate) fn check_valid_body_vec2(
    operation: &'static str,
    argument: &'static str,
    value: Vec2,
) -> Result<Vec2> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite vector",
        ))
    }
}

#[inline]
pub(crate) fn check_valid_body_position(
    operation: &'static str,
    argument: &'static str,
    value: Position,
) -> Result<Position> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite world position",
        ))
    }
}

#[inline]
pub(crate) fn check_valid_body_float(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<f32> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value",
        ))
    }
}

#[inline]
pub(crate) fn check_body_world_point_in_local_range(
    operation: &'static str,
    argument: &'static str,
    point: Position,
    origin: Position,
) -> Result<Position> {
    if point.checked_relative_to(origin).is_ok() {
        Ok(point)
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "an offset from the body representable by a finite local vector",
        ))
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
pub(crate) fn check_valid_body_target_motion(
    target: WorldTransform,
    time_step: f32,
    current_center: Position,
    current_rotation: Rot,
    local_center: Vec2,
) -> Result<(WorldTransform, f32)> {
    if body_target_motion_is_valid(
        target,
        time_step,
        current_center,
        current_rotation,
        local_center,
    ) {
        Ok((target, time_step))
    } else {
        Err(Error::invalid_argument(
            "Body::set_target_transform",
            "target/time_step",
            "values that produce finite linear and angular velocities with time_step > 0",
        ))
    }
}

#[inline]
pub(crate) fn check_valid_native_body_vec2(
    operation: &'static str,
    output: &'static str,
    value: Vec2,
) -> Result<Vec2> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite vector",
        })
    }
}

#[inline]
pub(crate) fn check_valid_native_body_position(
    operation: &'static str,
    output: &'static str,
    value: Position,
) -> Result<Position> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite world position",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::check_valid_body_name;
    use crate::error::Error;

    #[test]
    fn body_names_are_bounded_by_utf8_bytes() {
        const OPERATION: &str = "BodyBuilder::name";
        let too_long = Error::invalid_argument(OPERATION, "name", "at most 10 UTF-8 bytes");
        assert!(check_valid_body_name(OPERATION, "1234567890").is_ok());
        assert_eq!(
            check_valid_body_name(OPERATION, "12345678901").unwrap_err(),
            too_long
        );
        assert_eq!(
            check_valid_body_name(OPERATION, "eeeee")
                .expect("ASCII name")
                .as_bytes()
                .len(),
            5
        );
        assert_eq!(
            check_valid_body_name(OPERATION, "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}")
                .expect("five two-byte characters fit exactly")
                .as_bytes()
                .len(),
            10
        );
        assert_eq!(
            check_valid_body_name(OPERATION, "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}").unwrap_err(),
            too_long
        );
    }
}
