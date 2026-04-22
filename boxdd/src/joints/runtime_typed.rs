use super::*;

#[inline]
fn distance_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetLength)
}

#[inline]
fn distance_set_length_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetLength)
}

#[inline]
fn distance_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsSpringEnabled)
}

#[inline]
fn distance_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_EnableSpring)
}

#[inline]
fn distance_spring_force_range_impl(id: JointId) -> (f32, f32) {
    let mut lower_force = 0.0f32;
    let mut upper_force = 0.0f32;
    unsafe {
        ffi::b2DistanceJoint_GetSpringForceRange(
            raw_joint_id(id),
            &mut lower_force,
            &mut upper_force,
        )
    };
    (lower_force, upper_force)
}
#[inline]
fn distance_lower_spring_force_impl(id: JointId) -> f32 {
    distance_spring_force_range_impl(id).0
}
#[inline]
fn distance_upper_spring_force_impl(id: JointId) -> f32 {
    distance_spring_force_range_impl(id).1
}
#[inline]
fn distance_set_spring_force_range_impl(id: JointId, lower_force: f32, upper_force: f32) {
    unsafe { ffi::b2DistanceJoint_SetSpringForceRange(raw_joint_id(id), lower_force, upper_force) }
}

#[inline]
fn distance_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetSpringHertz)
}

#[inline]
fn distance_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetSpringHertz)
}

#[inline]
fn distance_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetSpringDampingRatio)
}

#[inline]
fn distance_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetSpringDampingRatio)
}

#[inline]
fn distance_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsLimitEnabled)
}

#[inline]
fn distance_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_EnableLimit)
}

#[inline]
fn distance_min_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMinLength)
}

#[inline]
fn distance_max_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMaxLength)
}

#[inline]
fn distance_current_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetCurrentLength)
}

#[inline]
fn distance_set_length_range_impl(id: JointId, min_length: f32, max_length: f32) {
    unsafe { ffi::b2DistanceJoint_SetLengthRange(raw_joint_id(id), min_length, max_length) }
}

#[inline]
fn distance_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsMotorEnabled)
}

#[inline]
fn distance_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_EnableMotor)
}

#[inline]
fn distance_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMotorSpeed)
}

#[inline]
fn distance_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetMotorSpeed)
}

#[inline]
fn distance_max_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMaxMotorForce)
}

#[inline]
fn distance_set_max_motor_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetMaxMotorForce)
}

#[inline]
fn distance_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMotorForce)
}

#[inline]
fn prismatic_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsSpringEnabled)
}

#[inline]
fn prismatic_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_EnableSpring)
}

#[inline]
fn prismatic_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpringHertz)
}

#[inline]
fn prismatic_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetSpringHertz)
}

#[inline]
fn prismatic_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpringDampingRatio)
}

#[inline]
fn prismatic_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetSpringDampingRatio)
}

#[inline]
fn prismatic_target_translation_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetTargetTranslation)
}

#[inline]
fn prismatic_set_target_translation_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetTargetTranslation)
}

#[inline]
fn prismatic_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsLimitEnabled)
}

#[inline]
fn prismatic_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_EnableLimit)
}

#[inline]
fn prismatic_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetLowerLimit)
}

#[inline]
fn prismatic_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetUpperLimit)
}

#[inline]
fn prismatic_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2PrismaticJoint_SetLimits(raw_joint_id(id), lower, upper) }
}

#[inline]
fn prismatic_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsMotorEnabled)
}

#[inline]
fn prismatic_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_EnableMotor)
}

#[inline]
fn prismatic_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMotorSpeed)
}

#[inline]
fn prismatic_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetMotorSpeed)
}

#[inline]
fn prismatic_max_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMaxMotorForce)
}

#[inline]
fn prismatic_set_max_motor_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetMaxMotorForce)
}

#[inline]
fn prismatic_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMotorForce)
}

#[inline]
fn prismatic_translation_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetTranslation)
}

#[inline]
fn prismatic_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpeed)
}

#[inline]
fn revolute_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsSpringEnabled)
}

#[inline]
fn revolute_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_EnableSpring)
}

#[inline]
fn revolute_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetSpringHertz)
}

#[inline]
fn revolute_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetSpringHertz)
}

#[inline]
fn revolute_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetSpringDampingRatio)
}

#[inline]
fn revolute_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetSpringDampingRatio)
}

#[inline]
fn revolute_target_angle_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetTargetAngle)
}

#[inline]
fn revolute_set_target_angle_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetTargetAngle)
}

#[inline]
fn revolute_angle_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetAngle)
}

#[inline]
fn revolute_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsLimitEnabled)
}

#[inline]
fn revolute_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_EnableLimit)
}

#[inline]
fn revolute_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetLowerLimit)
}

#[inline]
fn revolute_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetUpperLimit)
}

#[inline]
fn revolute_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2RevoluteJoint_SetLimits(raw_joint_id(id), lower, upper) }
}

#[inline]
fn revolute_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsMotorEnabled)
}

#[inline]
fn revolute_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_EnableMotor)
}

#[inline]
fn revolute_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMotorSpeed)
}

#[inline]
fn revolute_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetMotorSpeed)
}

#[inline]
fn revolute_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMotorTorque)
}

#[inline]
fn revolute_max_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMaxMotorTorque)
}

#[inline]
fn revolute_set_max_motor_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetMaxMotorTorque)
}

#[inline]
fn weld_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearHertz)
}

#[inline]
fn weld_set_linear_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetLinearHertz)
}

#[inline]
fn weld_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearDampingRatio)
}

#[inline]
fn weld_set_linear_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetLinearDampingRatio)
}

#[inline]
fn weld_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularHertz)
}

#[inline]
fn weld_set_angular_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetAngularHertz)
}

#[inline]
fn weld_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularDampingRatio)
}

#[inline]
fn weld_set_angular_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetAngularDampingRatio)
}

#[inline]
fn wheel_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsSpringEnabled)
}

#[inline]
fn wheel_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_EnableSpring)
}

#[inline]
fn wheel_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetSpringHertz)
}

#[inline]
fn wheel_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetSpringHertz)
}

#[inline]
fn wheel_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetSpringDampingRatio)
}

#[inline]
fn wheel_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetSpringDampingRatio)
}

#[inline]
fn wheel_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsLimitEnabled)
}

#[inline]
fn wheel_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_EnableLimit)
}

#[inline]
fn wheel_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetLowerLimit)
}

#[inline]
fn wheel_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetUpperLimit)
}

#[inline]
fn wheel_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2WheelJoint_SetLimits(raw_joint_id(id), lower, upper) }
}

#[inline]
fn wheel_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsMotorEnabled)
}

#[inline]
fn wheel_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_EnableMotor)
}

#[inline]
fn wheel_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMotorSpeed)
}

#[inline]
fn wheel_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetMotorSpeed)
}

#[inline]
fn wheel_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMotorTorque)
}

#[inline]
fn wheel_max_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMaxMotorTorque)
}

#[inline]
fn wheel_set_max_motor_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetMaxMotorTorque)
}

#[inline]
fn motor_linear_velocity_impl(id: JointId) -> Vec2 {
    joint_vec2_read_impl(id, ffi::b2MotorJoint_GetLinearVelocity)
}

#[inline]
fn motor_set_linear_velocity_impl(id: JointId, value: Vec2) {
    let raw: ffi::b2Vec2 = value.into_raw();
    unsafe { ffi::b2MotorJoint_SetLinearVelocity(raw_joint_id(id), raw) }
}

#[inline]
fn motor_angular_velocity_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularVelocity)
}

#[inline]
fn motor_set_angular_velocity_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetAngularVelocity)
}

#[inline]
fn motor_max_velocity_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxVelocityForce)
}

#[inline]
fn motor_set_max_velocity_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxVelocityForce)
}

#[inline]
fn motor_max_velocity_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxVelocityTorque)
}

#[inline]
fn motor_set_max_velocity_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxVelocityTorque)
}

#[inline]
fn motor_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetLinearHertz)
}

#[inline]
fn motor_set_linear_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetLinearHertz)
}

#[inline]
fn motor_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetLinearDampingRatio)
}

#[inline]
fn motor_set_linear_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetLinearDampingRatio)
}

#[inline]
fn motor_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularHertz)
}

#[inline]
fn motor_set_angular_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetAngularHertz)
}

#[inline]
fn motor_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularDampingRatio)
}

#[inline]
fn motor_set_angular_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetAngularDampingRatio)
}

#[inline]
fn motor_max_spring_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxSpringForce)
}

#[inline]
fn motor_set_max_spring_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxSpringForce)
}

#[inline]
fn motor_max_spring_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxSpringTorque)
}

#[inline]
fn motor_set_max_spring_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxSpringTorque)
}

impl World {
    pub fn distance_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn try_distance_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn distance_set_length(&mut self, id: JointId, length: f32) {
        joint_kind_set_checked_impl(id, JointType::Distance, length, distance_set_length_impl)
    }

    pub fn try_distance_set_length(&mut self, id: JointId, length: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Distance, length, distance_set_length_impl)
    }

    pub fn distance_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn try_distance_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn distance_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_spring_impl)
    }

    pub fn try_distance_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            enable,
            distance_enable_spring_impl,
        )
    }

    pub fn distance_lower_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn try_distance_lower_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn distance_upper_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn try_distance_upper_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn distance_set_spring_force_range(
        &mut self,
        id: JointId,
        lower_force: f32,
        upper_force: f32,
    ) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Distance,
            lower_force,
            upper_force,
            assert_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        )
    }

    pub fn try_distance_set_spring_force_range(
        &mut self,
        id: JointId,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Distance,
            lower_force,
            upper_force,
            check_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        )
    }

    pub fn distance_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn try_distance_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        )
    }

    pub fn try_distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        )
    }

    pub fn distance_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn try_distance_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn distance_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_distance_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        )
    }

    pub fn distance_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn try_distance_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn distance_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_limit_impl)
    }

    pub fn try_distance_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_limit_impl)
    }

    pub fn distance_min_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn try_distance_min_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn distance_max_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn try_distance_max_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn distance_current_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn try_distance_current_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn distance_set_length_range(&mut self, id: JointId, min_length: f32, max_length: f32) {
        joint_kind_set2_checked_impl(
            id,
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        )
    }

    pub fn try_distance_set_length_range(
        &mut self,
        id: JointId,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_impl(
            id,
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        )
    }

    pub fn distance_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn try_distance_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn distance_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_motor_impl)
    }

    pub fn try_distance_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_motor_impl)
    }

    pub fn distance_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn try_distance_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn distance_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        )
    }

    pub fn try_distance_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        )
    }

    pub fn distance_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn try_distance_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn distance_set_max_motor_force(&mut self, id: JointId, force: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        )
    }

    pub fn try_distance_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        )
    }

    pub fn distance_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }

    pub fn try_distance_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }
}

impl WorldHandle {
    pub fn distance_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn try_distance_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn distance_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn try_distance_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn distance_lower_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn try_distance_lower_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn distance_upper_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn try_distance_upper_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn distance_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn try_distance_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn distance_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn try_distance_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn distance_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn try_distance_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn distance_min_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn try_distance_min_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn distance_max_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn try_distance_max_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn distance_current_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn try_distance_current_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn distance_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn try_distance_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn distance_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn try_distance_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn distance_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn try_distance_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn distance_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }

    pub fn try_distance_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }
}

trait DistanceJointRuntimeHandle {
    fn distance_joint_id(&self) -> JointId;

    fn distance_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_length_impl,
        )
    }

    fn try_distance_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_length_impl,
        )
    }

    fn distance_set_length(&mut self, length: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            length,
            distance_set_length_impl,
        );
    }

    fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            length,
            distance_set_length_impl,
        )
    }

    fn distance_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    fn distance_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_spring_impl,
        );
    }

    fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_spring_impl,
        )
    }

    fn distance_lower_spring_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    fn distance_upper_spring_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        joint_kind_set2_checked_validated_impl(
            self.distance_joint_id(),
            JointType::Distance,
            lower_force,
            upper_force,
            assert_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        );
    }

    fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.distance_joint_id(),
            JointType::Distance,
            lower_force,
            upper_force,
            check_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        )
    }

    fn distance_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    fn distance_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        );
    }

    fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        )
    }

    fn distance_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        );
    }

    fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        )
    }

    fn distance_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    fn distance_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_limit_impl,
        );
    }

    fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_limit_impl,
        )
    }

    fn distance_min_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    fn try_distance_min_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    fn distance_max_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    fn try_distance_max_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    fn distance_current_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    fn try_distance_current_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        joint_kind_set2_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        );
    }

    fn try_distance_set_length_range(&mut self, min_length: f32, max_length: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        )
    }

    fn distance_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    fn distance_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_motor_impl,
        );
    }

    fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_motor_impl,
        )
    }

    fn distance_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    fn distance_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        );
    }

    fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        )
    }

    fn distance_max_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    fn distance_set_max_motor_force(&mut self, force: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        );
    }

    fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        )
    }

    fn distance_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_force_impl,
        )
    }

    fn try_distance_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_force_impl,
        )
    }
}

impl DistanceJointRuntimeHandle for OwnedJoint {
    fn distance_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> DistanceJointRuntimeHandle for Joint<'w> {
    fn distance_joint_id(&self) -> JointId {
        self.id()
    }
}

impl OwnedJoint {
    pub fn distance_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_length(self)
    }
    pub fn try_distance_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_length(self)
    }
    pub fn distance_set_length(&mut self, length: f32) {
        DistanceJointRuntimeHandle::distance_set_length(self, length)
    }
    pub fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length(self, length)
    }
    pub fn distance_spring_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_spring_enabled(self)
    }
    pub fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_spring_enabled(self)
    }
    pub fn distance_enable_spring(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_spring(self, enable)
    }
    pub fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_spring(self, enable)
    }
    pub fn distance_lower_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_lower_spring_force(self)
    }
    pub fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_lower_spring_force(self)
    }
    pub fn distance_upper_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_upper_spring_force(self)
    }
    pub fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_upper_spring_force(self)
    }
    pub fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_force_range(self, lower_force, upper_force)
    }
    pub fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_force_range(
            self,
            lower_force,
            upper_force,
        )
    }
    pub fn distance_spring_hertz(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_hertz(self)
    }
    pub fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_hertz(self)
    }
    pub fn distance_set_spring_hertz(&mut self, hertz: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_hertz(self, hertz)
    }
    pub fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_hertz(self, hertz)
    }
    pub fn distance_spring_damping_ratio(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_damping_ratio(self)
    }
    pub fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_damping_ratio(self)
    }
    pub fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn distance_limit_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_limit_enabled(self)
    }
    pub fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_limit_enabled(self)
    }
    pub fn distance_enable_limit(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_limit(self, enable)
    }
    pub fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_limit(self, enable)
    }
    pub fn distance_min_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_min_length(self)
    }
    pub fn try_distance_min_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_min_length(self)
    }
    pub fn distance_max_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_length(self)
    }
    pub fn try_distance_max_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_length(self)
    }
    pub fn distance_current_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_current_length(self)
    }
    pub fn try_distance_current_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_current_length(self)
    }
    pub fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        DistanceJointRuntimeHandle::distance_set_length_range(self, min_length, max_length)
    }
    pub fn try_distance_set_length_range(
        &mut self,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length_range(self, min_length, max_length)
    }
    pub fn distance_motor_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_motor_enabled(self)
    }
    pub fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_motor_enabled(self)
    }
    pub fn distance_enable_motor(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_motor(self, enable)
    }
    pub fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_motor(self, enable)
    }
    pub fn distance_motor_speed(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_speed(self)
    }
    pub fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_speed(self)
    }
    pub fn distance_set_motor_speed(&mut self, speed: f32) {
        DistanceJointRuntimeHandle::distance_set_motor_speed(self, speed)
    }
    pub fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_motor_speed(self, speed)
    }
    pub fn distance_max_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_motor_force(self)
    }
    pub fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_motor_force(self)
    }
    pub fn distance_set_max_motor_force(&mut self, force: f32) {
        DistanceJointRuntimeHandle::distance_set_max_motor_force(self, force)
    }
    pub fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_max_motor_force(self, force)
    }
    pub fn distance_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_force(self)
    }
    pub fn try_distance_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_force(self)
    }
}

impl<'w> Joint<'w> {
    pub fn distance_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_length(self)
    }
    pub fn try_distance_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_length(self)
    }
    pub fn distance_set_length(&mut self, length: f32) {
        DistanceJointRuntimeHandle::distance_set_length(self, length)
    }
    pub fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length(self, length)
    }
    pub fn distance_spring_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_spring_enabled(self)
    }
    pub fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_spring_enabled(self)
    }
    pub fn distance_enable_spring(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_spring(self, enable)
    }
    pub fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_spring(self, enable)
    }
    pub fn distance_lower_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_lower_spring_force(self)
    }
    pub fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_lower_spring_force(self)
    }
    pub fn distance_upper_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_upper_spring_force(self)
    }
    pub fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_upper_spring_force(self)
    }
    pub fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_force_range(self, lower_force, upper_force)
    }
    pub fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_force_range(
            self,
            lower_force,
            upper_force,
        )
    }
    pub fn distance_spring_hertz(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_hertz(self)
    }
    pub fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_hertz(self)
    }
    pub fn distance_set_spring_hertz(&mut self, hertz: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_hertz(self, hertz)
    }
    pub fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_hertz(self, hertz)
    }
    pub fn distance_spring_damping_ratio(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_damping_ratio(self)
    }
    pub fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_damping_ratio(self)
    }
    pub fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn distance_limit_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_limit_enabled(self)
    }
    pub fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_limit_enabled(self)
    }
    pub fn distance_enable_limit(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_limit(self, enable)
    }
    pub fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_limit(self, enable)
    }
    pub fn distance_min_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_min_length(self)
    }
    pub fn try_distance_min_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_min_length(self)
    }
    pub fn distance_max_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_length(self)
    }
    pub fn try_distance_max_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_length(self)
    }
    pub fn distance_current_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_current_length(self)
    }
    pub fn try_distance_current_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_current_length(self)
    }
    pub fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        DistanceJointRuntimeHandle::distance_set_length_range(self, min_length, max_length)
    }
    pub fn try_distance_set_length_range(
        &mut self,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length_range(self, min_length, max_length)
    }
    pub fn distance_motor_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_motor_enabled(self)
    }
    pub fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_motor_enabled(self)
    }
    pub fn distance_enable_motor(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_motor(self, enable)
    }
    pub fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_motor(self, enable)
    }
    pub fn distance_motor_speed(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_speed(self)
    }
    pub fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_speed(self)
    }
    pub fn distance_set_motor_speed(&mut self, speed: f32) {
        DistanceJointRuntimeHandle::distance_set_motor_speed(self, speed)
    }
    pub fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_motor_speed(self, speed)
    }
    pub fn distance_max_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_motor_force(self)
    }
    pub fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_motor_force(self)
    }
    pub fn distance_set_max_motor_force(&mut self, force: f32) {
        DistanceJointRuntimeHandle::distance_set_max_motor_force(self, force)
    }
    pub fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_max_motor_force(self, force)
    }
    pub fn distance_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_force(self)
    }
    pub fn try_distance_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_force(self)
    }
}

impl World {
    pub fn prismatic_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn try_prismatic_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn prismatic_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        )
    }

    pub fn try_prismatic_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        )
    }

    pub fn prismatic_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn try_prismatic_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        )
    }

    pub fn try_prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        )
    }

    pub fn prismatic_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn try_prismatic_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn prismatic_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_prismatic_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        )
    }

    pub fn prismatic_target_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn try_prismatic_target_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn prismatic_set_target_translation(&mut self, id: JointId, translation: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        )
    }

    pub fn try_prismatic_set_target_translation(
        &mut self,
        id: JointId,
        translation: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        )
    }

    pub fn prismatic_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn try_prismatic_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn prismatic_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        )
    }

    pub fn try_prismatic_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        )
    }

    pub fn prismatic_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn try_prismatic_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn prismatic_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn try_prismatic_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn prismatic_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Prismatic,
            lower,
            upper,
            assert_prismatic_limits_valid,
            prismatic_set_limits_impl,
        )
    }

    pub fn try_prismatic_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Prismatic,
            lower,
            upper,
            check_prismatic_limits_valid,
            prismatic_set_limits_impl,
        )
    }

    pub fn prismatic_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn try_prismatic_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn prismatic_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        )
    }

    pub fn try_prismatic_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        )
    }

    pub fn prismatic_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn try_prismatic_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        )
    }

    pub fn try_prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        )
    }

    pub fn prismatic_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn try_prismatic_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        )
    }

    pub fn try_prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        )
    }

    pub fn prismatic_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn try_prismatic_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn prismatic_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn try_prismatic_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn prismatic_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn try_prismatic_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn revolute_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn try_revolute_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn revolute_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_spring_impl)
    }

    pub fn try_revolute_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            enable,
            revolute_enable_spring_impl,
        )
    }

    pub fn revolute_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn try_revolute_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        )
    }

    pub fn try_revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        )
    }

    pub fn revolute_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn try_revolute_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn revolute_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_revolute_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        )
    }

    pub fn revolute_target_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn try_revolute_target_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn revolute_set_target_angle(&mut self, id: JointId, angle: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        )
    }

    pub fn try_revolute_set_target_angle(&mut self, id: JointId, angle: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        )
    }

    pub fn revolute_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn try_revolute_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn revolute_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn try_revolute_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn revolute_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_limit_impl)
    }

    pub fn try_revolute_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_limit_impl)
    }

    pub fn revolute_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn try_revolute_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn revolute_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn try_revolute_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn revolute_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Revolute,
            lower,
            upper,
            assert_revolute_limits_valid,
            revolute_set_limits_impl,
        )
    }

    pub fn try_revolute_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Revolute,
            lower,
            upper,
            check_revolute_limits_valid,
            revolute_set_limits_impl,
        )
    }

    pub fn revolute_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn try_revolute_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn revolute_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_motor_impl)
    }

    pub fn try_revolute_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_motor_impl)
    }

    pub fn revolute_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn try_revolute_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn revolute_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        )
    }

    pub fn try_revolute_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        )
    }

    pub fn revolute_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn try_revolute_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn revolute_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }

    pub fn try_revolute_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }

    pub fn revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        )
    }

    pub fn try_revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        )
    }
}

impl WorldHandle {
    pub fn prismatic_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn try_prismatic_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn prismatic_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn try_prismatic_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn prismatic_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn try_prismatic_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn prismatic_target_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn try_prismatic_target_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn prismatic_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn try_prismatic_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn prismatic_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn try_prismatic_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn prismatic_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn try_prismatic_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn prismatic_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn try_prismatic_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn prismatic_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn try_prismatic_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn prismatic_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn try_prismatic_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn prismatic_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn try_prismatic_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn prismatic_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn try_prismatic_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn prismatic_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn try_prismatic_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn revolute_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn try_revolute_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn revolute_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn try_revolute_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn revolute_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn try_revolute_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn revolute_target_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn try_revolute_target_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn revolute_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn try_revolute_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn revolute_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn try_revolute_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn revolute_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn try_revolute_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn revolute_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn try_revolute_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn revolute_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn try_revolute_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn revolute_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn try_revolute_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn revolute_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn try_revolute_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn revolute_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }

    pub fn try_revolute_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }
}

trait PrismaticJointRuntimeHandle {
    fn prismatic_joint_id(&self) -> JointId;

    fn prismatic_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    fn prismatic_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        );
    }

    fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        )
    }

    fn prismatic_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        );
    }

    fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        )
    }

    fn prismatic_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        );
    }

    fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        )
    }

    fn prismatic_target_translation(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    fn prismatic_set_target_translation(&mut self, translation: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        );
    }

    fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        )
    }

    fn prismatic_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    fn prismatic_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        );
    }

    fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        )
    }

    fn prismatic_lower_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    fn prismatic_upper_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            lower,
            upper,
            assert_prismatic_limits_valid,
            prismatic_set_limits_impl,
        );
    }

    fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            lower,
            upper,
            check_prismatic_limits_valid,
            prismatic_set_limits_impl,
        )
    }

    fn prismatic_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    fn prismatic_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        );
    }

    fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        )
    }

    fn prismatic_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    fn prismatic_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        );
    }

    fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        )
    }

    fn prismatic_max_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    fn prismatic_set_max_motor_force(&mut self, force: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        );
    }

    fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        )
    }

    fn prismatic_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    fn prismatic_translation(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    fn try_prismatic_translation(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    fn prismatic_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }

    fn try_prismatic_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }
}

impl PrismaticJointRuntimeHandle for OwnedJoint {
    fn prismatic_joint_id(&self) -> JointId {
        self.id()
    }
}

impl OwnedJoint {
    pub fn prismatic_spring_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_spring_enabled(self)
    }
    pub fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_enabled(self)
    }
    pub fn prismatic_enable_spring(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_spring(self, enable)
    }
    pub fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_spring(self, enable)
    }
    pub fn prismatic_spring_hertz(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_hertz(self)
    }
    pub fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_hertz(self)
    }
    pub fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_hertz(self, hertz)
    }
    pub fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_hertz(self, hertz)
    }
    pub fn prismatic_spring_damping_ratio(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_damping_ratio(self)
    }
    pub fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_damping_ratio(self)
    }
    pub fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn prismatic_target_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_target_translation(self)
    }
    pub fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_target_translation(self)
    }
    pub fn prismatic_set_target_translation(&mut self, translation: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_target_translation(self, translation)
    }
    pub fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_target_translation(self, translation)
    }
    pub fn prismatic_limit_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_limit_enabled(self)
    }
    pub fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_limit_enabled(self)
    }
    pub fn prismatic_enable_limit(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_limit(self, enable)
    }
    pub fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_limit(self, enable)
    }
    pub fn prismatic_lower_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_lower_limit(self)
    }
    pub fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_lower_limit(self)
    }
    pub fn prismatic_upper_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_upper_limit(self)
    }
    pub fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_upper_limit(self)
    }
    pub fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_limits(self, lower, upper)
    }
    pub fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_limits(self, lower, upper)
    }
    pub fn prismatic_motor_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_motor_enabled(self)
    }
    pub fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_enabled(self)
    }
    pub fn prismatic_enable_motor(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_motor(self, enable)
    }
    pub fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_motor(self, enable)
    }
    pub fn prismatic_motor_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_speed(self)
    }
    pub fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_speed(self)
    }
    pub fn prismatic_set_motor_speed(&mut self, speed: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_motor_speed(self, speed)
    }
    pub fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_motor_speed(self, speed)
    }
    pub fn prismatic_max_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_max_motor_force(self)
    }
    pub fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_max_motor_force(self)
    }
    pub fn prismatic_set_max_motor_force(&mut self, force: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_max_motor_force(self, force)
    }
    pub fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_max_motor_force(self, force)
    }
    pub fn prismatic_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_force(self)
    }
    pub fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_force(self)
    }
    pub fn prismatic_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_translation(self)
    }
    pub fn try_prismatic_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_translation(self)
    }
    pub fn prismatic_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_speed(self)
    }
    pub fn try_prismatic_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_speed(self)
    }

    pub fn revolute_spring_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_spring_enabled(self)
    }
    pub fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_spring_enabled(self)
    }
    pub fn revolute_enable_spring(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_spring(self, enable)
    }
    pub fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_spring(self, enable)
    }
    pub fn revolute_spring_hertz(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_hertz(self)
    }
    pub fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_hertz(self)
    }
    pub fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_hertz(self, hertz)
    }
    pub fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_hertz(self, hertz)
    }
    pub fn revolute_spring_damping_ratio(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_damping_ratio(self)
    }
    pub fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_damping_ratio(self)
    }
    pub fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn revolute_target_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_target_angle(self)
    }
    pub fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_target_angle(self)
    }
    pub fn revolute_set_target_angle(&mut self, angle: f32) {
        RevoluteJointRuntimeHandle::revolute_set_target_angle(self, angle)
    }
    pub fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_target_angle(self, angle)
    }
    pub fn revolute_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_angle(self)
    }
    pub fn try_revolute_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_angle(self)
    }
    pub fn revolute_limit_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_limit_enabled(self)
    }
    pub fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_limit_enabled(self)
    }
    pub fn revolute_enable_limit(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_limit(self, enable)
    }
    pub fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_limit(self, enable)
    }
    pub fn revolute_lower_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_lower_limit(self)
    }
    pub fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_lower_limit(self)
    }
    pub fn revolute_upper_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_upper_limit(self)
    }
    pub fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_upper_limit(self)
    }
    pub fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        RevoluteJointRuntimeHandle::revolute_set_limits(self, lower, upper)
    }
    pub fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_limits(self, lower, upper)
    }
    pub fn revolute_motor_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_motor_enabled(self)
    }
    pub fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_motor_enabled(self)
    }
    pub fn revolute_enable_motor(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_motor(self, enable)
    }
    pub fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_motor(self, enable)
    }
    pub fn revolute_motor_speed(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_speed(self)
    }
    pub fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_speed(self)
    }
    pub fn revolute_set_motor_speed(&mut self, speed: f32) {
        RevoluteJointRuntimeHandle::revolute_set_motor_speed(self, speed)
    }
    pub fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_motor_speed(self, speed)
    }
    pub fn revolute_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_torque(self)
    }
    pub fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_torque(self)
    }
    pub fn revolute_max_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_max_motor_torque(self)
    }
    pub fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_max_motor_torque(self)
    }
    pub fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        RevoluteJointRuntimeHandle::revolute_set_max_motor_torque(self, torque)
    }
    pub fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_max_motor_torque(self, torque)
    }
}

impl<'w> PrismaticJointRuntimeHandle for Joint<'w> {
    fn prismatic_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> Joint<'w> {
    pub fn prismatic_spring_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_spring_enabled(self)
    }
    pub fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_enabled(self)
    }
    pub fn prismatic_enable_spring(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_spring(self, enable)
    }
    pub fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_spring(self, enable)
    }
    pub fn prismatic_spring_hertz(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_hertz(self)
    }
    pub fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_hertz(self)
    }
    pub fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_hertz(self, hertz)
    }
    pub fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_hertz(self, hertz)
    }
    pub fn prismatic_spring_damping_ratio(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_damping_ratio(self)
    }
    pub fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_damping_ratio(self)
    }
    pub fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn prismatic_target_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_target_translation(self)
    }
    pub fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_target_translation(self)
    }
    pub fn prismatic_set_target_translation(&mut self, translation: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_target_translation(self, translation)
    }
    pub fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_target_translation(self, translation)
    }
    pub fn prismatic_limit_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_limit_enabled(self)
    }
    pub fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_limit_enabled(self)
    }
    pub fn prismatic_enable_limit(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_limit(self, enable)
    }
    pub fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_limit(self, enable)
    }
    pub fn prismatic_lower_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_lower_limit(self)
    }
    pub fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_lower_limit(self)
    }
    pub fn prismatic_upper_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_upper_limit(self)
    }
    pub fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_upper_limit(self)
    }
    pub fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_limits(self, lower, upper)
    }
    pub fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_limits(self, lower, upper)
    }
    pub fn prismatic_motor_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_motor_enabled(self)
    }
    pub fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_enabled(self)
    }
    pub fn prismatic_enable_motor(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_motor(self, enable)
    }
    pub fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_motor(self, enable)
    }
    pub fn prismatic_motor_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_speed(self)
    }
    pub fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_speed(self)
    }
    pub fn prismatic_set_motor_speed(&mut self, speed: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_motor_speed(self, speed)
    }
    pub fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_motor_speed(self, speed)
    }
    pub fn prismatic_max_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_max_motor_force(self)
    }
    pub fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_max_motor_force(self)
    }
    pub fn prismatic_set_max_motor_force(&mut self, force: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_max_motor_force(self, force)
    }
    pub fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_max_motor_force(self, force)
    }
    pub fn prismatic_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_force(self)
    }
    pub fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_force(self)
    }
    pub fn prismatic_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_translation(self)
    }
    pub fn try_prismatic_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_translation(self)
    }
    pub fn prismatic_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_speed(self)
    }
    pub fn try_prismatic_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_speed(self)
    }

    pub fn revolute_spring_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_spring_enabled(self)
    }
    pub fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_spring_enabled(self)
    }
    pub fn revolute_enable_spring(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_spring(self, enable)
    }
    pub fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_spring(self, enable)
    }
    pub fn revolute_spring_hertz(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_hertz(self)
    }
    pub fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_hertz(self)
    }
    pub fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_hertz(self, hertz)
    }
    pub fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_hertz(self, hertz)
    }
    pub fn revolute_spring_damping_ratio(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_damping_ratio(self)
    }
    pub fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_damping_ratio(self)
    }
    pub fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn revolute_target_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_target_angle(self)
    }
    pub fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_target_angle(self)
    }
    pub fn revolute_set_target_angle(&mut self, angle: f32) {
        RevoluteJointRuntimeHandle::revolute_set_target_angle(self, angle)
    }
    pub fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_target_angle(self, angle)
    }
    pub fn revolute_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_angle(self)
    }
    pub fn try_revolute_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_angle(self)
    }
    pub fn revolute_limit_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_limit_enabled(self)
    }
    pub fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_limit_enabled(self)
    }
    pub fn revolute_enable_limit(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_limit(self, enable)
    }
    pub fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_limit(self, enable)
    }
    pub fn revolute_lower_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_lower_limit(self)
    }
    pub fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_lower_limit(self)
    }
    pub fn revolute_upper_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_upper_limit(self)
    }
    pub fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_upper_limit(self)
    }
    pub fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        RevoluteJointRuntimeHandle::revolute_set_limits(self, lower, upper)
    }
    pub fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_limits(self, lower, upper)
    }
    pub fn revolute_motor_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_motor_enabled(self)
    }
    pub fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_motor_enabled(self)
    }
    pub fn revolute_enable_motor(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_motor(self, enable)
    }
    pub fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_motor(self, enable)
    }
    pub fn revolute_motor_speed(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_speed(self)
    }
    pub fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_speed(self)
    }
    pub fn revolute_set_motor_speed(&mut self, speed: f32) {
        RevoluteJointRuntimeHandle::revolute_set_motor_speed(self, speed)
    }
    pub fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_motor_speed(self, speed)
    }
    pub fn revolute_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_torque(self)
    }
    pub fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_torque(self)
    }
    pub fn revolute_max_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_max_motor_torque(self)
    }
    pub fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_max_motor_torque(self)
    }
    pub fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        RevoluteJointRuntimeHandle::revolute_set_max_motor_torque(self, torque)
    }
    pub fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_max_motor_torque(self, torque)
    }
}

trait RevoluteJointRuntimeHandle {
    fn revolute_joint_id(&self) -> JointId;

    fn revolute_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    fn revolute_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_spring_impl,
        );
    }

    fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_spring_impl,
        )
    }

    fn revolute_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        );
    }

    fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        )
    }

    fn revolute_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        );
    }

    fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        )
    }

    fn revolute_target_angle(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    fn revolute_set_target_angle(&mut self, angle: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        );
    }

    fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        )
    }

    fn revolute_angle(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    fn try_revolute_angle(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    fn revolute_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    fn revolute_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_limit_impl,
        );
    }

    fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_limit_impl,
        )
    }

    fn revolute_lower_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    fn revolute_upper_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            lower,
            upper,
            assert_revolute_limits_valid,
            revolute_set_limits_impl,
        );
    }

    fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            lower,
            upper,
            check_revolute_limits_valid,
            revolute_set_limits_impl,
        )
    }

    fn revolute_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    fn revolute_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_motor_impl,
        );
    }

    fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_motor_impl,
        )
    }

    fn revolute_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    fn revolute_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        );
    }

    fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        )
    }

    fn revolute_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    fn revolute_max_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        );
    }

    fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        )
    }
}

impl RevoluteJointRuntimeHandle for OwnedJoint {
    fn revolute_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> RevoluteJointRuntimeHandle for Joint<'w> {
    fn revolute_joint_id(&self) -> JointId {
        self.id()
    }
}

trait WeldJointRuntimeHandle {
    fn weld_joint_id(&self) -> JointId;

    fn weld_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn weld_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_linear_hertz_impl,
        );
    }

    fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_linear_hertz_impl,
        )
    }

    fn weld_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        );
    }

    fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    fn weld_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn weld_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_angular_hertz_impl,
        );
    }

    fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_angular_hertz_impl,
        )
    }

    fn weld_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        );
    }

    fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }
}

impl WeldJointRuntimeHandle for OwnedJoint {
    fn weld_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> WeldJointRuntimeHandle for Joint<'w> {
    fn weld_joint_id(&self) -> JointId {
        self.id()
    }
}

trait WheelJointRuntimeHandle {
    fn wheel_joint_id(&self) -> JointId;

    fn wheel_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    fn wheel_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_spring_impl,
        );
    }

    fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_spring_impl,
        )
    }

    fn wheel_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            hertz,
            wheel_set_spring_hertz_impl,
        );
    }

    fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            hertz,
            wheel_set_spring_hertz_impl,
        )
    }

    fn wheel_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        );
    }

    fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        )
    }

    fn wheel_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    fn wheel_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_limit_impl,
        );
    }

    fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_limit_impl,
        )
    }

    fn wheel_lower_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    fn wheel_upper_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            lower,
            upper,
            assert_wheel_limits_valid,
            wheel_set_limits_impl,
        );
    }

    fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            lower,
            upper,
            check_wheel_limits_valid,
            wheel_set_limits_impl,
        )
    }

    fn wheel_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    fn wheel_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_motor_impl,
        );
    }

    fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_motor_impl,
        )
    }

    fn wheel_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    fn wheel_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            speed,
            wheel_set_motor_speed_impl,
        );
    }

    fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            speed,
            wheel_set_motor_speed_impl,
        )
    }

    fn wheel_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    fn wheel_max_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        );
    }

    fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        )
    }
}

impl WheelJointRuntimeHandle for OwnedJoint {
    fn wheel_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> WheelJointRuntimeHandle for Joint<'w> {
    fn wheel_joint_id(&self) -> JointId {
        self.id()
    }
}

trait MotorJointRuntimeHandle {
    fn motor_joint_id(&self) -> JointId;

    fn motor_linear_velocity(&self) -> Vec2 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        );
    }

    fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        )
    }

    fn motor_angular_velocity(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    fn motor_set_angular_velocity(&mut self, w: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            w,
            motor_set_angular_velocity_impl,
        );
    }

    fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            w,
            motor_set_angular_velocity_impl,
        )
    }

    fn motor_max_velocity_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    fn motor_set_max_velocity_force(&mut self, f: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_velocity_force_impl,
        );
    }

    fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_velocity_force_impl,
        )
    }

    fn motor_max_velocity_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    fn motor_set_max_velocity_torque(&mut self, t: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_velocity_torque_impl,
        );
    }

    fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_velocity_torque_impl,
        )
    }

    fn motor_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    fn motor_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_linear_hertz_impl,
        );
    }

    fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_linear_hertz_impl,
        )
    }

    fn motor_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        );
    }

    fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        )
    }

    fn motor_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    fn motor_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_angular_hertz_impl,
        );
    }

    fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_angular_hertz_impl,
        )
    }

    fn motor_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        );
    }

    fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        )
    }

    fn motor_max_spring_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    fn motor_set_max_spring_force(&mut self, f: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_spring_force_impl,
        );
    }

    fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_spring_force_impl,
        )
    }

    fn motor_max_spring_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    fn motor_set_max_spring_torque(&mut self, t: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_spring_torque_impl,
        );
    }

    fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_spring_torque_impl,
        )
    }
}

impl MotorJointRuntimeHandle for OwnedJoint {
    fn motor_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> MotorJointRuntimeHandle for Joint<'w> {
    fn motor_joint_id(&self) -> JointId {
        self.id()
    }
}

impl World {
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_linear_hertz_impl)
    }

    pub fn try_weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_linear_hertz_impl)
    }

    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    pub fn try_weld_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_angular_hertz_impl)
    }

    pub fn try_weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_angular_hertz_impl)
    }

    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }

    pub fn try_weld_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }

    pub fn wheel_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn try_wheel_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn wheel_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_spring_impl)
    }

    pub fn try_wheel_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_spring_impl)
    }

    pub fn wheel_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn try_wheel_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Wheel, hertz, wheel_set_spring_hertz_impl)
    }

    pub fn try_wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, hertz, wheel_set_spring_hertz_impl)
    }

    pub fn wheel_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn try_wheel_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn wheel_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_wheel_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        )
    }

    pub fn wheel_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn try_wheel_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn wheel_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_limit_impl)
    }

    pub fn try_wheel_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_limit_impl)
    }

    pub fn wheel_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn try_wheel_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn wheel_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn try_wheel_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Wheel,
            lower,
            upper,
            assert_wheel_limits_valid,
            wheel_set_limits_impl,
        )
    }

    pub fn try_wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Wheel,
            lower,
            upper,
            check_wheel_limits_valid,
            wheel_set_limits_impl,
        )
    }

    pub fn wheel_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn try_wheel_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn wheel_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_motor_impl)
    }

    pub fn try_wheel_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_motor_impl)
    }

    pub fn wheel_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn try_wheel_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn wheel_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(id, JointType::Wheel, speed, wheel_set_motor_speed_impl)
    }

    pub fn try_wheel_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, speed, wheel_set_motor_speed_impl)
    }

    pub fn wheel_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn try_wheel_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn wheel_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn try_wheel_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        )
    }

    pub fn try_wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        )
    }

    pub fn motor_linear_velocity(&self, id: JointId) -> Vec2 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn try_motor_linear_velocity(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, id: JointId, v: V) {
        joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        )
    }

    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(
        &mut self,
        id: JointId,
        v: V,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        )
    }

    pub fn motor_angular_velocity(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn try_motor_angular_velocity(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn motor_set_angular_velocity(&mut self, id: JointId, w: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, w, motor_set_angular_velocity_impl)
    }

    pub fn try_motor_set_angular_velocity(&mut self, id: JointId, w: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, w, motor_set_angular_velocity_impl)
    }

    pub fn motor_max_velocity_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn try_motor_max_velocity_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn motor_set_max_velocity_force(&mut self, id: JointId, f: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_velocity_force_impl)
    }

    pub fn try_motor_set_max_velocity_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_velocity_force_impl)
    }

    pub fn motor_max_velocity_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn try_motor_max_velocity_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_velocity_torque_impl)
    }

    pub fn try_motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_velocity_torque_impl)
    }

    pub fn motor_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn try_motor_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_linear_hertz_impl)
    }

    pub fn try_motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_linear_hertz_impl)
    }

    pub fn motor_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn try_motor_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn motor_set_linear_damping_ratio(&mut self, id: JointId, damping: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        )
    }

    pub fn try_motor_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        )
    }

    pub fn motor_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn try_motor_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_angular_hertz_impl)
    }

    pub fn try_motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_angular_hertz_impl)
    }

    pub fn motor_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn try_motor_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn motor_set_angular_damping_ratio(&mut self, id: JointId, damping: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        )
    }

    pub fn try_motor_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        )
    }

    pub fn motor_max_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn try_motor_max_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn motor_set_max_spring_force(&mut self, id: JointId, f: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_spring_force_impl)
    }

    pub fn try_motor_set_max_spring_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_spring_force_impl)
    }

    pub fn motor_max_spring_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }

    pub fn try_motor_max_spring_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }

    pub fn motor_set_max_spring_torque(&mut self, id: JointId, t: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_spring_torque_impl)
    }

    pub fn try_motor_set_max_spring_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_spring_torque_impl)
    }
}

impl WorldHandle {
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn wheel_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn try_wheel_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn wheel_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn try_wheel_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn wheel_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn try_wheel_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn wheel_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn try_wheel_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn wheel_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn try_wheel_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn wheel_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn try_wheel_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn wheel_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn try_wheel_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn wheel_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn try_wheel_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn wheel_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn try_wheel_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn wheel_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn try_wheel_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn motor_linear_velocity(&self, id: JointId) -> Vec2 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn try_motor_linear_velocity(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn motor_angular_velocity(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn try_motor_angular_velocity(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn motor_max_velocity_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn try_motor_max_velocity_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn motor_max_velocity_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn try_motor_max_velocity_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn motor_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn try_motor_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn motor_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn try_motor_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn motor_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn try_motor_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn motor_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn try_motor_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn motor_max_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn try_motor_max_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn motor_max_spring_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }

    pub fn try_motor_max_spring_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }
}

impl OwnedJoint {
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_spring_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_spring_enabled(self)
    }
    pub fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_spring_enabled(self)
    }
    pub fn wheel_enable_spring(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_spring(self, enable)
    }
    pub fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_spring(self, enable)
    }
    pub fn wheel_spring_hertz(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_hertz(self)
    }
    pub fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_hertz(self)
    }
    pub fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_hertz(self, hertz)
    }
    pub fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_hertz(self, hertz)
    }
    pub fn wheel_spring_damping_ratio(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_damping_ratio(self)
    }
    pub fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_damping_ratio(self)
    }
    pub fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_limit_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_limit_enabled(self)
    }
    pub fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_limit_enabled(self)
    }
    pub fn wheel_enable_limit(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_limit(self, enable)
    }
    pub fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_limit(self, enable)
    }
    pub fn wheel_lower_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_lower_limit(self)
    }
    pub fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_lower_limit(self)
    }
    pub fn wheel_upper_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_upper_limit(self)
    }
    pub fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_upper_limit(self)
    }
    pub fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        WheelJointRuntimeHandle::wheel_set_limits(self, lower, upper)
    }
    pub fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_limits(self, lower, upper)
    }
    pub fn wheel_motor_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_motor_enabled(self)
    }
    pub fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_motor_enabled(self)
    }
    pub fn wheel_enable_motor(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_motor(self, enable)
    }
    pub fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_motor(self, enable)
    }
    pub fn wheel_motor_speed(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_speed(self)
    }
    pub fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_speed(self)
    }
    pub fn wheel_set_motor_speed(&mut self, speed: f32) {
        WheelJointRuntimeHandle::wheel_set_motor_speed(self, speed)
    }
    pub fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_motor_speed(self, speed)
    }
    pub fn wheel_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_torque(self)
    }
    pub fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_torque(self)
    }
    pub fn wheel_max_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_max_motor_torque(self)
    }
    pub fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_max_motor_torque(self)
    }
    pub fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        WheelJointRuntimeHandle::wheel_set_max_motor_torque(self, torque)
    }
    pub fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_max_motor_torque(self, torque)
    }
    pub fn motor_linear_velocity(&self) -> Vec2 {
        MotorJointRuntimeHandle::motor_linear_velocity(self)
    }
    pub fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        MotorJointRuntimeHandle::try_motor_linear_velocity(self)
    }
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        MotorJointRuntimeHandle::motor_set_linear_velocity(self, v)
    }
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_velocity(self, v)
    }
    pub fn motor_angular_velocity(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_velocity(self)
    }
    pub fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_velocity(self)
    }
    pub fn motor_set_angular_velocity(&mut self, w: f32) {
        MotorJointRuntimeHandle::motor_set_angular_velocity(self, w)
    }
    pub fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_velocity(self, w)
    }
    pub fn motor_max_velocity_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_force(self)
    }
    pub fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_force(self)
    }
    pub fn motor_set_max_velocity_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_force(self, f)
    }
    pub fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_force(self, f)
    }
    pub fn motor_max_velocity_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_torque(self)
    }
    pub fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_torque(self)
    }
    pub fn motor_set_max_velocity_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_torque(self, t)
    }
    pub fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_torque(self, t)
    }
    pub fn motor_linear_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_hertz(self)
    }
    pub fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_hertz(self)
    }
    pub fn motor_set_linear_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_linear_hertz(self, hertz)
    }
    pub fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_hertz(self, hertz)
    }
    pub fn motor_linear_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_damping_ratio(self)
    }
    pub fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_damping_ratio(self)
    }
    pub fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_linear_damping_ratio(self, damping)
    }
    pub fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_damping_ratio(self, damping)
    }
    pub fn motor_angular_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_hertz(self)
    }
    pub fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_hertz(self)
    }
    pub fn motor_set_angular_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_angular_hertz(self, hertz)
    }
    pub fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_hertz(self, hertz)
    }
    pub fn motor_angular_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_damping_ratio(self)
    }
    pub fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_damping_ratio(self)
    }
    pub fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_angular_damping_ratio(self, damping)
    }
    pub fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_damping_ratio(self, damping)
    }
    pub fn motor_max_spring_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_force(self)
    }
    pub fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_force(self)
    }
    pub fn motor_set_max_spring_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_force(self, f)
    }
    pub fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_force(self, f)
    }
    pub fn motor_max_spring_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_torque(self)
    }
    pub fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_torque(self)
    }
    pub fn motor_set_max_spring_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_torque(self, t)
    }
    pub fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_torque(self, t)
    }
}

impl<'w> Joint<'w> {
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_spring_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_spring_enabled(self)
    }
    pub fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_spring_enabled(self)
    }
    pub fn wheel_enable_spring(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_spring(self, enable)
    }
    pub fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_spring(self, enable)
    }
    pub fn wheel_spring_hertz(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_hertz(self)
    }
    pub fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_hertz(self)
    }
    pub fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_hertz(self, hertz)
    }
    pub fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_hertz(self, hertz)
    }
    pub fn wheel_spring_damping_ratio(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_damping_ratio(self)
    }
    pub fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_damping_ratio(self)
    }
    pub fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_limit_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_limit_enabled(self)
    }
    pub fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_limit_enabled(self)
    }
    pub fn wheel_enable_limit(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_limit(self, enable)
    }
    pub fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_limit(self, enable)
    }
    pub fn wheel_lower_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_lower_limit(self)
    }
    pub fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_lower_limit(self)
    }
    pub fn wheel_upper_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_upper_limit(self)
    }
    pub fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_upper_limit(self)
    }
    pub fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        WheelJointRuntimeHandle::wheel_set_limits(self, lower, upper)
    }
    pub fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_limits(self, lower, upper)
    }
    pub fn wheel_motor_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_motor_enabled(self)
    }
    pub fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_motor_enabled(self)
    }
    pub fn wheel_enable_motor(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_motor(self, enable)
    }
    pub fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_motor(self, enable)
    }
    pub fn wheel_motor_speed(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_speed(self)
    }
    pub fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_speed(self)
    }
    pub fn wheel_set_motor_speed(&mut self, speed: f32) {
        WheelJointRuntimeHandle::wheel_set_motor_speed(self, speed)
    }
    pub fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_motor_speed(self, speed)
    }
    pub fn wheel_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_torque(self)
    }
    pub fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_torque(self)
    }
    pub fn wheel_max_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_max_motor_torque(self)
    }
    pub fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_max_motor_torque(self)
    }
    pub fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        WheelJointRuntimeHandle::wheel_set_max_motor_torque(self, torque)
    }
    pub fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_max_motor_torque(self, torque)
    }
    pub fn motor_linear_velocity(&self) -> Vec2 {
        MotorJointRuntimeHandle::motor_linear_velocity(self)
    }
    pub fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        MotorJointRuntimeHandle::try_motor_linear_velocity(self)
    }
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        MotorJointRuntimeHandle::motor_set_linear_velocity(self, v)
    }
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_velocity(self, v)
    }
    pub fn motor_angular_velocity(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_velocity(self)
    }
    pub fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_velocity(self)
    }
    pub fn motor_set_angular_velocity(&mut self, w: f32) {
        MotorJointRuntimeHandle::motor_set_angular_velocity(self, w)
    }
    pub fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_velocity(self, w)
    }
    pub fn motor_max_velocity_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_force(self)
    }
    pub fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_force(self)
    }
    pub fn motor_set_max_velocity_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_force(self, f)
    }
    pub fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_force(self, f)
    }
    pub fn motor_max_velocity_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_torque(self)
    }
    pub fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_torque(self)
    }
    pub fn motor_set_max_velocity_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_torque(self, t)
    }
    pub fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_torque(self, t)
    }
    pub fn motor_linear_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_hertz(self)
    }
    pub fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_hertz(self)
    }
    pub fn motor_set_linear_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_linear_hertz(self, hertz)
    }
    pub fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_hertz(self, hertz)
    }
    pub fn motor_linear_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_damping_ratio(self)
    }
    pub fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_damping_ratio(self)
    }
    pub fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_linear_damping_ratio(self, damping)
    }
    pub fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_damping_ratio(self, damping)
    }
    pub fn motor_angular_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_hertz(self)
    }
    pub fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_hertz(self)
    }
    pub fn motor_set_angular_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_angular_hertz(self, hertz)
    }
    pub fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_hertz(self, hertz)
    }
    pub fn motor_angular_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_damping_ratio(self)
    }
    pub fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_damping_ratio(self)
    }
    pub fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_angular_damping_ratio(self, damping)
    }
    pub fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_damping_ratio(self, damping)
    }
    pub fn motor_max_spring_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_force(self)
    }
    pub fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_force(self)
    }
    pub fn motor_set_max_spring_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_force(self, f)
    }
    pub fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_force(self, f)
    }
    pub fn motor_max_spring_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_torque(self)
    }
    pub fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_torque(self)
    }
    pub fn motor_set_max_spring_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_torque(self, t)
    }
    pub fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_torque(self, t)
    }
}
