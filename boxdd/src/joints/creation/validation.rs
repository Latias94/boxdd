use super::*;

#[inline]
fn finite(value: f32) -> bool {
    crate::is_valid_float(value)
}

#[inline]
fn finite_non_negative(value: f32) -> bool {
    finite(value) && value >= 0.0
}

#[inline]
fn check(condition: bool) -> ApiResult<()> {
    if condition {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_joint_base_valid(base: &JointBase) -> ApiResult<()> {
    if base.body_a_id().brand() != base.body_b_id().brand() {
        return Err(crate::error::ApiError::WrongWorld);
    }

    check(base.body_a_id() != base.body_b_id())?;
    check(base.local_frame_a().is_valid() && base.local_frame_b().is_valid())?;
    check(
        finite_non_negative(base.force_threshold())
            && finite_non_negative(base.torque_threshold())
            && finite_non_negative(base.constraint_tuning().hertz)
            && finite_non_negative(base.constraint_tuning().damping_ratio)
            && finite_non_negative(base.draw_scale()),
    )
}

pub(crate) fn check_distance_joint_def_valid(def: &DistanceJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check(finite(def.target_length()) && def.target_length() > 0.0)?;
    check(
        finite(def.minimum_spring_force())
            && finite(def.maximum_spring_force())
            && def.minimum_spring_force() <= def.maximum_spring_force()
            && finite_non_negative(def.spring_hertz())
            && finite_non_negative(def.spring_damping_ratio())
            && finite_non_negative(def.minimum_length())
            && finite_non_negative(def.maximum_length())
            && def.minimum_length() <= def.maximum_length()
            && finite_non_negative(def.maximum_motor_force())
            && finite(def.target_motor_speed()),
    )
}

pub(crate) fn check_motor_joint_def_valid(def: &MotorJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check(
        def.target_linear_velocity().is_valid()
            && finite(def.target_angular_velocity())
            && finite_non_negative(def.maximum_velocity_force())
            && finite_non_negative(def.maximum_velocity_torque())
            && finite_non_negative(def.linear_spring_hertz())
            && finite_non_negative(def.linear_spring_damping_ratio())
            && finite_non_negative(def.maximum_spring_force())
            && finite_non_negative(def.angular_spring_hertz())
            && finite_non_negative(def.angular_spring_damping_ratio())
            && finite_non_negative(def.maximum_spring_torque()),
    )
}

pub(crate) fn check_filter_joint_def_valid(def: &FilterJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())
}

pub(crate) fn check_prismatic_joint_def_valid(def: &PrismaticJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check(
        finite_non_negative(def.spring_hertz())
            && finite_non_negative(def.spring_damping_ratio())
            && finite(def.target_translation())
            && finite(def.minimum_translation())
            && finite(def.maximum_translation())
            && def.minimum_translation() <= def.maximum_translation()
            && finite_non_negative(def.maximum_motor_force())
            && finite(def.target_motor_speed()),
    )
}

pub(crate) fn check_revolute_joint_def_valid(def: &RevoluteJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    let limit = 0.99 * ffi::B2_PI as f32;
    check(
        finite(def.target_angle_value())
            && finite_non_negative(def.spring_hertz())
            && finite_non_negative(def.spring_damping_ratio())
            && finite(def.minimum_angle())
            && finite(def.maximum_angle())
            && def.minimum_angle() <= def.maximum_angle()
            && def.minimum_angle() >= -limit
            && def.maximum_angle() <= limit
            && finite_non_negative(def.maximum_motor_torque())
            && finite(def.target_motor_speed()),
    )
}

pub(crate) fn check_weld_joint_def_valid(def: &WeldJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check(
        finite_non_negative(def.configured_linear_hertz())
            && finite_non_negative(def.configured_angular_hertz())
            && finite_non_negative(def.configured_linear_damping_ratio())
            && finite_non_negative(def.configured_angular_damping_ratio()),
    )
}

pub(crate) fn check_wheel_joint_def_valid(def: &WheelJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check(
        finite_non_negative(def.spring_hertz())
            && finite_non_negative(def.spring_damping_ratio())
            && finite(def.minimum_translation())
            && finite(def.maximum_translation())
            && def.minimum_translation() <= def.maximum_translation()
            && finite_non_negative(def.maximum_motor_torque())
            && finite(def.target_motor_speed()),
    )
}
