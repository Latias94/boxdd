use super::*;

pub(crate) fn check_joint_base_valid(base: &JointBase) -> ApiResult<()> {
    if base.body_a_id().brand() != base.body_b_id().brand() {
        return Err(crate::error::ApiError::WrongWorld);
    }

    check_joint_condition(base.body_a_id() != base.body_b_id())?;
    check_joint_transform(base.local_frame_a())?;
    check_joint_transform(base.local_frame_b())?;
    check_joint_non_negative(base.force_threshold())?;
    check_joint_non_negative(base.torque_threshold())?;
    check_joint_tuning(base.constraint_tuning())?;
    check_joint_non_negative(base.draw_scale())
}

pub(crate) fn check_distance_joint_def_valid(def: &DistanceJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check_joint_positive(def.target_length())?;
    check_joint_ordered_range(def.minimum_spring_force(), def.maximum_spring_force())?;
    check_joint_non_negative(def.spring_hertz())?;
    check_joint_non_negative(def.spring_damping_ratio())?;
    check_joint_non_negative_range(def.minimum_length(), def.maximum_length())?;
    check_joint_non_negative(def.maximum_motor_force())?;
    check_joint_finite(def.target_motor_speed())
}

pub(crate) fn check_motor_joint_def_valid(def: &MotorJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check_joint_vec2(def.target_linear_velocity())?;
    check_joint_finite(def.target_angular_velocity())?;
    check_joint_non_negative(def.maximum_velocity_force())?;
    check_joint_non_negative(def.maximum_velocity_torque())?;
    check_joint_non_negative(def.linear_spring_hertz())?;
    check_joint_non_negative(def.linear_spring_damping_ratio())?;
    check_joint_non_negative(def.maximum_spring_force())?;
    check_joint_non_negative(def.angular_spring_hertz())?;
    check_joint_non_negative(def.angular_spring_damping_ratio())?;
    check_joint_non_negative(def.maximum_spring_torque())
}

pub(crate) fn check_filter_joint_def_valid(def: &FilterJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())
}

pub(crate) fn check_prismatic_joint_def_valid(def: &PrismaticJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check_joint_non_negative(def.spring_hertz())?;
    check_joint_non_negative(def.spring_damping_ratio())?;
    check_joint_finite(def.target_translation())?;
    check_joint_ordered_range(def.minimum_translation(), def.maximum_translation())?;
    check_joint_non_negative(def.maximum_motor_force())?;
    check_joint_finite(def.target_motor_speed())
}

pub(crate) fn check_revolute_joint_def_valid(def: &RevoluteJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check_joint_finite(def.target_angle_value())?;
    check_joint_non_negative(def.spring_hertz())?;
    check_joint_non_negative(def.spring_damping_ratio())?;
    check_revolute_joint_range(def.minimum_angle(), def.maximum_angle())?;
    check_joint_non_negative(def.maximum_motor_torque())?;
    check_joint_finite(def.target_motor_speed())
}

pub(crate) fn check_weld_joint_def_valid(def: &WeldJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check_joint_non_negative(def.configured_linear_hertz())?;
    check_joint_non_negative(def.configured_angular_hertz())?;
    check_joint_non_negative(def.configured_linear_damping_ratio())?;
    check_joint_non_negative(def.configured_angular_damping_ratio())
}

pub(crate) fn check_wheel_joint_def_valid(def: &WheelJointDef) -> ApiResult<()> {
    check_joint_base_valid(def.base())?;
    check_joint_non_negative(def.spring_hertz())?;
    check_joint_non_negative(def.spring_damping_ratio())?;
    check_joint_ordered_range(def.minimum_translation(), def.maximum_translation())?;
    check_joint_non_negative(def.maximum_motor_torque())?;
    check_joint_finite(def.target_motor_speed())
}
