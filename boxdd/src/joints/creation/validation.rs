use super::*;

pub(crate) fn check_joint_base_valid_for(base: &JointBase, operation: &'static str) -> Result<()> {
    if base.body_a_id().brand() != base.body_b_id().brand() {
        return Err(crate::error::Error::WrongWorld);
    }

    check_joint_condition(
        base.body_a_id() != base.body_b_id(),
        operation,
        "body_a/body_b",
        "two distinct body identifiers",
    )?;
    check_joint_transform(base.local_frame_a(), operation, "local_frame_a")?;
    check_joint_transform(base.local_frame_b(), operation, "local_frame_b")?;
    check_joint_non_negative(base.force_threshold(), operation, "force_threshold")?;
    check_joint_non_negative(base.torque_threshold(), operation, "torque_threshold")?;
    check_joint_tuning(base.constraint_tuning(), operation, "constraint_tuning")?;
    check_joint_non_negative(base.draw_scale(), operation, "draw_scale")
}

pub(crate) fn check_joint_base_valid(base: &JointBase) -> Result<()> {
    check_joint_base_valid_for(base, "JointBase::validate")
}

pub(crate) fn check_distance_joint_def_valid(def: &DistanceJointDef) -> Result<()> {
    const OP: &str = "DistanceJointDef::validate";
    check_joint_base_valid_for(def.base(), OP)?;
    check_joint_positive(def.target_length(), OP, "length")?;
    check_joint_ordered_range(
        def.minimum_spring_force(),
        def.maximum_spring_force(),
        OP,
        "spring_force_range",
    )?;
    check_joint_non_negative(def.spring_hertz(), OP, "hertz")?;
    check_joint_non_negative(def.spring_damping_ratio(), OP, "damping_ratio")?;
    check_joint_non_negative_range(
        def.minimum_length(),
        def.maximum_length(),
        OP,
        "length_range",
    )?;
    check_joint_non_negative(def.maximum_motor_force(), OP, "max_motor_force")?;
    check_joint_finite(def.target_motor_speed(), OP, "motor_speed")
}

pub(crate) fn check_motor_joint_def_valid(def: &MotorJointDef) -> Result<()> {
    const OP: &str = "MotorJointDef::validate";
    check_joint_base_valid_for(def.base(), OP)?;
    check_joint_vec2(def.target_linear_velocity(), OP, "linear_velocity")?;
    check_joint_finite(def.target_angular_velocity(), OP, "angular_velocity")?;
    check_joint_non_negative(def.maximum_velocity_force(), OP, "max_velocity_force")?;
    check_joint_non_negative(def.maximum_velocity_torque(), OP, "max_velocity_torque")?;
    check_joint_non_negative(def.linear_spring_hertz(), OP, "linear_hertz")?;
    check_joint_non_negative(
        def.linear_spring_damping_ratio(),
        OP,
        "linear_damping_ratio",
    )?;
    check_joint_non_negative(def.maximum_spring_force(), OP, "max_spring_force")?;
    check_joint_non_negative(def.angular_spring_hertz(), OP, "angular_hertz")?;
    check_joint_non_negative(
        def.angular_spring_damping_ratio(),
        OP,
        "angular_damping_ratio",
    )?;
    check_joint_non_negative(def.maximum_spring_torque(), OP, "max_spring_torque")
}

pub(crate) fn check_filter_joint_def_valid(def: &FilterJointDef) -> Result<()> {
    check_joint_base_valid_for(def.base(), "FilterJointDef::validate")
}

pub(crate) fn check_prismatic_joint_def_valid(def: &PrismaticJointDef) -> Result<()> {
    const OP: &str = "PrismaticJointDef::validate";
    check_joint_base_valid_for(def.base(), OP)?;
    check_joint_non_negative(def.spring_hertz(), OP, "hertz")?;
    check_joint_non_negative(def.spring_damping_ratio(), OP, "damping_ratio")?;
    check_joint_finite(def.target_translation(), OP, "target_translation")?;
    check_joint_ordered_range(
        def.minimum_translation(),
        def.maximum_translation(),
        OP,
        "translation_range",
    )?;
    check_joint_non_negative(def.maximum_motor_force(), OP, "max_motor_force")?;
    check_joint_finite(def.target_motor_speed(), OP, "motor_speed")
}

pub(crate) fn check_revolute_joint_def_valid(def: &RevoluteJointDef) -> Result<()> {
    const OP: &str = "RevoluteJointDef::validate";
    check_joint_base_valid_for(def.base(), OP)?;
    check_joint_finite(def.target_angle_value(), OP, "target_angle")?;
    check_joint_non_negative(def.spring_hertz(), OP, "hertz")?;
    check_joint_non_negative(def.spring_damping_ratio(), OP, "damping_ratio")?;
    check_revolute_joint_range(def.minimum_angle(), def.maximum_angle(), OP, "angle_range")?;
    check_joint_non_negative(def.maximum_motor_torque(), OP, "max_motor_torque")?;
    check_joint_finite(def.target_motor_speed(), OP, "motor_speed")
}

pub(crate) fn check_weld_joint_def_valid(def: &WeldJointDef) -> Result<()> {
    const OP: &str = "WeldJointDef::validate";
    check_joint_base_valid_for(def.base(), OP)?;
    check_joint_non_negative(def.configured_linear_hertz(), OP, "linear_hertz")?;
    check_joint_non_negative(def.configured_angular_hertz(), OP, "angular_hertz")?;
    check_joint_non_negative(
        def.configured_linear_damping_ratio(),
        OP,
        "linear_damping_ratio",
    )?;
    check_joint_non_negative(
        def.configured_angular_damping_ratio(),
        OP,
        "angular_damping_ratio",
    )
}

pub(crate) fn check_wheel_joint_def_valid(def: &WheelJointDef) -> Result<()> {
    const OP: &str = "WheelJointDef::validate";
    check_joint_base_valid_for(def.base(), OP)?;
    check_joint_non_negative(def.spring_hertz(), OP, "hertz")?;
    check_joint_non_negative(def.spring_damping_ratio(), OP, "damping_ratio")?;
    check_joint_ordered_range(
        def.minimum_translation(),
        def.maximum_translation(),
        OP,
        "translation_range",
    )?;
    check_joint_non_negative(def.maximum_motor_torque(), OP, "max_motor_torque")?;
    check_joint_finite(def.target_motor_speed(), OP, "motor_speed")
}
