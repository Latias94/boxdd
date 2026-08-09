use crate::error::{Error, Result};
use crate::types::{BodyId, Position, Vec2, WorldTransform};
use boxdd_sys::ffi;

use super::ConstraintTuning;

pub(crate) const REVOLUTE_LIMIT_ABS_MAX: f32 = 0.99 * core::f32::consts::PI;

#[inline]
fn decode_native_body_world_transform(
    operation: &'static str,
    output: &'static str,
    raw: ffi::b2WorldTransform,
) -> Result<WorldTransform> {
    WorldTransform::from_raw(raw).map_err(|_| Error::InvalidNativeOutput {
        operation,
        output,
        constraint: "a finite rigid world transform",
    })
}

#[inline]
pub(crate) fn read_native_body_world_transform(
    operation: &'static str,
    output: &'static str,
    body: BodyId,
) -> Result<WorldTransform> {
    decode_native_body_world_transform(operation, output, unsafe {
        ffi::b2Body_GetTransform(super::raw_body_id(body))
    })
}

#[inline]
pub(crate) fn check_joint_condition(
    condition: bool,
    operation: &'static str,
    argument: &'static str,
    constraint: &'static str,
) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::invalid_argument(operation, argument, constraint))
    }
}

#[inline]
pub(crate) fn check_joint_finite(
    value: f32,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(value.is_finite(), operation, argument, "a finite value")
}

#[inline]
pub(crate) fn check_joint_non_negative(
    value: f32,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        value.is_finite() && value >= 0.0,
        operation,
        argument,
        "a finite non-negative value",
    )
}

#[inline]
pub(crate) fn check_joint_positive(
    value: f32,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        value.is_finite() && value > 0.0,
        operation,
        argument,
        "a finite positive value",
    )
}

#[inline]
pub(crate) fn check_joint_vec2(
    value: Vec2,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        value.is_valid(),
        operation,
        argument,
        "finite vector components",
    )
}

#[inline]
pub(crate) fn check_joint_position(
    value: Position,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(value.is_valid(), operation, argument, "finite coordinates")
}

#[inline]
pub(crate) fn check_joint_axis(
    value: Vec2,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        value.is_valid() && value != Vec2::ZERO,
        operation,
        argument,
        "a finite non-zero direction",
    )
}

#[inline]
pub(crate) fn check_joint_tuning(
    value: ConstraintTuning,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        value.hertz().is_finite()
            && value.hertz() >= 0.0
            && value.damping_ratio().is_finite()
            && value.damping_ratio() >= 0.0,
        operation,
        argument,
        "finite non-negative hertz and damping ratio values",
    )
}

#[inline]
pub(crate) fn check_joint_transform(
    value: crate::Transform,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        value.is_valid(),
        operation,
        argument,
        "a valid finite transform",
    )
}

#[inline]
pub(crate) fn check_joint_ordered_range(
    lower: f32,
    upper: f32,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        lower.is_finite() && upper.is_finite() && lower <= upper,
        operation,
        argument,
        "finite values ordered lower <= upper",
    )
}

#[inline]
pub(crate) fn check_joint_non_negative_range(
    lower: f32,
    upper: f32,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        lower.is_finite() && upper.is_finite() && lower >= 0.0 && lower <= upper,
        operation,
        argument,
        "finite values ordered 0 <= lower <= upper",
    )
}

#[inline]
pub(crate) fn check_revolute_joint_range(
    lower: f32,
    upper: f32,
    operation: &'static str,
    argument: &'static str,
) -> Result<()> {
    check_joint_condition(
        lower.is_finite()
            && upper.is_finite()
            && -REVOLUTE_LIMIT_ABS_MAX <= lower
            && lower <= upper
            && upper <= REVOLUTE_LIMIT_ABS_MAX,
        operation,
        argument,
        "finite ordered angles within the supported revolute limit",
    )
}

#[inline]
pub(crate) fn check_native_joint_finite(
    value: f32,
    operation: &'static str,
    output: &'static str,
) -> Result<f32> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite value",
        })
    }
}

#[inline]
pub(crate) fn check_native_joint_non_negative(
    value: f32,
    operation: &'static str,
    output: &'static str,
) -> Result<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite non-negative value",
        })
    }
}

#[inline]
pub(crate) fn check_native_joint_positive(
    value: f32,
    operation: &'static str,
    output: &'static str,
) -> Result<f32> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite positive value",
        })
    }
}

#[inline]
pub(crate) fn check_native_joint_vec2(
    value: Vec2,
    operation: &'static str,
    output: &'static str,
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
pub(crate) fn check_native_joint_ordered_range(
    lower: f32,
    upper: f32,
    operation: &'static str,
    output: &'static str,
) -> Result<(f32, f32)> {
    if lower.is_finite() && upper.is_finite() && lower <= upper {
        Ok((lower, upper))
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "finite values ordered lower <= upper",
        })
    }
}

#[inline]
pub(crate) fn check_native_joint_non_negative_range(
    lower: f32,
    upper: f32,
    operation: &'static str,
    output: &'static str,
) -> Result<(f32, f32)> {
    if lower.is_finite() && upper.is_finite() && lower >= 0.0 && lower <= upper {
        Ok((lower, upper))
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "finite values ordered 0 <= lower <= upper",
        })
    }
}

#[inline]
pub(crate) fn check_native_revolute_joint_range(
    lower: f32,
    upper: f32,
    operation: &'static str,
    output: &'static str,
) -> Result<(f32, f32)> {
    if lower.is_finite()
        && upper.is_finite()
        && -REVOLUTE_LIMIT_ABS_MAX <= lower
        && lower <= upper
        && upper <= REVOLUTE_LIMIT_ABS_MAX
    {
        Ok((lower, upper))
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "finite ordered angles within the supported revolute limit",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_joint_scalar_vector_and_range_checks_fail_closed() {
        assert_eq!(
            check_native_joint_finite(f32::NAN, "Joint::constraint_torque", "constraint_torque"),
            Err(Error::InvalidNativeOutput {
                operation: "Joint::constraint_torque",
                output: "constraint_torque",
                constraint: "a finite value",
            })
        );
        assert_eq!(
            check_native_joint_non_negative(-1.0, "DistanceJoint::spring_hertz", "spring_hertz",),
            Err(Error::InvalidNativeOutput {
                operation: "DistanceJoint::spring_hertz",
                output: "spring_hertz",
                constraint: "a finite non-negative value",
            })
        );
        assert_eq!(
            check_native_joint_positive(0.0, "DistanceJoint::length", "length"),
            Err(Error::InvalidNativeOutput {
                operation: "DistanceJoint::length",
                output: "length",
                constraint: "a finite positive value",
            })
        );
        assert_eq!(
            check_native_joint_vec2(
                Vec2::new(0.0, f32::INFINITY),
                "MotorJoint::linear_velocity",
                "linear_velocity",
            ),
            Err(Error::InvalidNativeOutput {
                operation: "MotorJoint::linear_velocity",
                output: "linear_velocity",
                constraint: "a finite vector",
            })
        );
        assert_eq!(
            check_native_joint_ordered_range(
                2.0,
                1.0,
                "DistanceJoint::spring_force_range",
                "spring_force_range",
            ),
            Err(Error::InvalidNativeOutput {
                operation: "DistanceJoint::spring_force_range",
                output: "spring_force_range",
                constraint: "finite values ordered lower <= upper",
            })
        );
        assert_eq!(
            check_native_joint_non_negative_range(
                -1.0,
                2.0,
                "DistanceJoint::length_range",
                "length_range",
            ),
            Err(Error::InvalidNativeOutput {
                operation: "DistanceJoint::length_range",
                output: "length_range",
                constraint: "finite values ordered 0 <= lower <= upper",
            })
        );
        assert_eq!(
            check_native_revolute_joint_range(
                -REVOLUTE_LIMIT_ABS_MAX,
                core::f32::consts::PI,
                "RevoluteJoint::limit_range",
                "limit_range",
            ),
            Err(Error::InvalidNativeOutput {
                operation: "RevoluteJoint::limit_range",
                output: "limit_range",
                constraint: "finite ordered angles within the supported revolute limit",
            })
        );

        assert_eq!(
            check_native_joint_finite(-2.0, "DistanceJoint::motor_force", "motor_force"),
            Ok(-2.0)
        );

        let invalid_transform = ffi::b2WorldTransform {
            p: Position::ZERO.into_raw(),
            q: ffi::b2Rot {
                c: f32::NAN,
                s: 0.0,
            },
        };
        assert!(matches!(
            decode_native_body_world_transform(
                "DistanceJointBuilder::build",
                "body_a_transform",
                invalid_transform,
            ),
            Err(Error::InvalidNativeOutput {
                operation: "DistanceJointBuilder::build",
                output: "body_a_transform",
                constraint: "a finite rigid world transform",
            })
        ));
    }
}
