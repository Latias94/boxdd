use super::*;

#[inline]
fn revolute_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsSpringEnabled)
}

const REVOLUTE_ENABLE_SPRING: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::RevoluteEnableSpring);

#[inline]
fn revolute_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetSpringHertz)
}

const REVOLUTE_SET_SPRING_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::RevoluteSetSpringHertz);

#[inline]
fn revolute_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetSpringDampingRatio)
}

const REVOLUTE_SET_SPRING_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::RevoluteSetSpringDampingRatio);

#[inline]
fn revolute_target_angle_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetTargetAngle)
}

const REVOLUTE_SET_TARGET_ANGLE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::RevoluteSetTargetAngle);

#[inline]
fn revolute_angle_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetAngle)
}

#[inline]
fn revolute_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsLimitEnabled)
}

const REVOLUTE_ENABLE_LIMIT: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::RevoluteEnableLimit);

#[inline]
fn revolute_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetLowerLimit)
}

#[inline]
fn revolute_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetUpperLimit)
}

const REVOLUTE_SET_LIMITS: JointSet2Op<f32, f32> =
    JointSet2Op::new(JointWriteKind::RevoluteSetLimits);

#[inline]
fn revolute_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsMotorEnabled)
}

const REVOLUTE_ENABLE_MOTOR: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::RevoluteEnableMotor);

#[inline]
fn revolute_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMotorSpeed)
}

const REVOLUTE_SET_MOTOR_SPEED: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::RevoluteSetMotorSpeed);

#[inline]
fn revolute_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMotorTorque)
}

#[inline]
fn revolute_max_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMaxMotorTorque)
}

const REVOLUTE_SET_MAX_MOTOR_TORQUE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::RevoluteSetMaxMotorTorque);

impl World {
    /// Returns whether the selected revolute joint's spring is enabled.
    pub fn revolute_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    /// Fallible variant of revolute_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    /// Enables or disables the selected revolute joint's spring.
    pub fn revolute_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_SPRING,
        )
    }

    /// Fallible variant of revolute_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_SPRING,
        )
    }

    /// Returns the selected revolute joint's spring frequency in hertz.
    pub fn revolute_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    /// Fallible variant of revolute_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    /// Sets the selected revolute joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            hertz,
            REVOLUTE_SET_SPRING_HERTZ,
        )
    }

    /// Fallible variant of revolute_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            hertz,
            REVOLUTE_SET_SPRING_HERTZ,
        )
    }

    /// Returns the selected revolute joint's spring damping ratio.
    pub fn revolute_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of revolute_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    /// Sets the selected revolute joint's spring damping ratio; the value must be finite and non-negative.
    pub fn revolute_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            damping_ratio,
            REVOLUTE_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Fallible variant of revolute_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            damping_ratio,
            REVOLUTE_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Returns the selected revolute joint's target angle in radians.
    pub fn revolute_target_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    /// Fallible variant of revolute_target_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_target_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    /// Sets the selected revolute joint's target angle in radians; the value must be finite.
    pub fn revolute_set_target_angle(&mut self, id: JointId, angle: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            angle,
            REVOLUTE_SET_TARGET_ANGLE,
        )
    }

    /// Fallible variant of revolute_set_target_angle; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_target_angle(&mut self, id: JointId, angle: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            angle,
            REVOLUTE_SET_TARGET_ANGLE,
        )
    }

    /// Returns the selected revolute joint's current angle in radians.
    pub fn revolute_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Revolute, revolute_angle_impl)
    }

    /// Fallible variant of revolute_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    /// Returns whether the selected revolute joint's limit is enabled.
    pub fn revolute_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    /// Fallible variant of revolute_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    /// Enables or disables the selected revolute joint's limit.
    pub fn revolute_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_LIMIT,
        )
    }

    /// Fallible variant of revolute_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_LIMIT,
        )
    }

    /// Returns the selected revolute joint's lower angular limit in radians.
    pub fn revolute_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    /// Fallible variant of revolute_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    /// Returns the selected revolute joint's upper angular limit in radians.
    pub fn revolute_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    /// Fallible variant of revolute_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    /// Sets the selected revolute joint's lower and upper angular limits in radians; the bounds must be finite and ordered.
    pub fn revolute_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            lower,
            upper,
            REVOLUTE_SET_LIMITS,
        )
    }

    /// Fallible variant of revolute_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            lower,
            upper,
            REVOLUTE_SET_LIMITS,
        )
    }

    /// Returns whether the selected revolute joint's motor is enabled.
    pub fn revolute_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    /// Fallible variant of revolute_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    /// Enables or disables the selected revolute joint's motor.
    pub fn revolute_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_MOTOR,
        )
    }

    /// Fallible variant of revolute_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_MOTOR,
        )
    }

    /// Returns the selected revolute joint's target motor speed in radians per second.
    pub fn revolute_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    /// Fallible variant of revolute_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    /// Sets the selected revolute joint's target motor speed in radians per second; the value must be finite.
    pub fn revolute_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            speed,
            REVOLUTE_SET_MOTOR_SPEED,
        )
    }

    /// Fallible variant of revolute_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            speed,
            REVOLUTE_SET_MOTOR_SPEED,
        )
    }

    /// Returns the selected revolute joint's current motor torque.
    pub fn revolute_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    /// Fallible variant of revolute_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    /// Returns the selected revolute joint's maximum motor torque.
    pub fn revolute_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    /// Fallible variant of revolute_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    /// Sets the selected revolute joint's maximum motor torque; the value must be finite and non-negative.
    pub fn revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            torque,
            REVOLUTE_SET_MAX_MOTOR_TORQUE,
        )
    }

    /// Fallible variant of revolute_set_max_motor_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            torque,
            REVOLUTE_SET_MAX_MOTOR_TORQUE,
        )
    }
}

impl WorldHandle {
    /// Returns whether the selected revolute joint's spring is enabled.
    pub fn revolute_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    /// Fallible variant of revolute_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    /// Returns the selected revolute joint's spring frequency in hertz.
    pub fn revolute_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    /// Fallible variant of revolute_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    /// Returns the selected revolute joint's spring damping ratio.
    pub fn revolute_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of revolute_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    /// Returns the selected revolute joint's target angle in radians.
    pub fn revolute_target_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    /// Fallible variant of revolute_target_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_target_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    /// Returns the selected revolute joint's current angle in radians.
    pub fn revolute_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Revolute, revolute_angle_impl)
    }

    /// Fallible variant of revolute_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    /// Returns whether the selected revolute joint's limit is enabled.
    pub fn revolute_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    /// Fallible variant of revolute_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    /// Returns the selected revolute joint's lower angular limit in radians.
    pub fn revolute_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    /// Fallible variant of revolute_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    /// Returns the selected revolute joint's upper angular limit in radians.
    pub fn revolute_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    /// Fallible variant of revolute_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    /// Returns whether the selected revolute joint's motor is enabled.
    pub fn revolute_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    /// Fallible variant of revolute_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    /// Returns the selected revolute joint's target motor speed in radians per second.
    pub fn revolute_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    /// Fallible variant of revolute_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    /// Returns the selected revolute joint's current motor torque.
    pub fn revolute_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    /// Fallible variant of revolute_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    /// Returns the selected revolute joint's maximum motor torque.
    pub fn revolute_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    /// Fallible variant of revolute_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }
}

impl OwnedJoint {
    /// Returns whether the selected revolute joint's spring is enabled.
    pub fn revolute_spring_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_spring_enabled(self)
    }
    /// Fallible variant of revolute_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_spring_enabled(self)
    }
    /// Enables or disables the selected revolute joint's spring.
    pub fn revolute_enable_spring(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_spring(self, enable)
    }
    /// Fallible variant of revolute_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_spring(self, enable)
    }
    /// Returns the selected revolute joint's spring frequency in hertz.
    pub fn revolute_spring_hertz(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_hertz(self)
    }
    /// Fallible variant of revolute_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_hertz(self)
    }
    /// Sets the selected revolute joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of revolute_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_hertz(self, hertz)
    }
    /// Returns the selected revolute joint's spring damping ratio.
    pub fn revolute_spring_damping_ratio(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_damping_ratio(self)
    }
    /// Fallible variant of revolute_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_damping_ratio(self)
    }
    /// Sets the selected revolute joint's spring damping ratio; the value must be finite and non-negative.
    pub fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of revolute_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns the selected revolute joint's target angle in radians.
    pub fn revolute_target_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_target_angle(self)
    }
    /// Fallible variant of revolute_target_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_target_angle(self)
    }
    /// Sets the selected revolute joint's target angle in radians; the value must be finite.
    pub fn revolute_set_target_angle(&mut self, angle: f32) {
        RevoluteJointRuntimeHandle::revolute_set_target_angle(self, angle)
    }
    /// Fallible variant of revolute_set_target_angle; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_target_angle(self, angle)
    }
    /// Returns the selected revolute joint's current angle in radians.
    pub fn revolute_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_angle(self)
    }
    /// Fallible variant of revolute_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_angle(self)
    }
    /// Returns whether the selected revolute joint's limit is enabled.
    pub fn revolute_limit_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_limit_enabled(self)
    }
    /// Fallible variant of revolute_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_limit_enabled(self)
    }
    /// Enables or disables the selected revolute joint's limit.
    pub fn revolute_enable_limit(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_limit(self, enable)
    }
    /// Fallible variant of revolute_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_limit(self, enable)
    }
    /// Returns the selected revolute joint's lower angular limit in radians.
    pub fn revolute_lower_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_lower_limit(self)
    }
    /// Fallible variant of revolute_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_lower_limit(self)
    }
    /// Returns the selected revolute joint's upper angular limit in radians.
    pub fn revolute_upper_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_upper_limit(self)
    }
    /// Fallible variant of revolute_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_upper_limit(self)
    }
    /// Sets the selected revolute joint's lower and upper angular limits in radians; the bounds must be finite and ordered.
    pub fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        RevoluteJointRuntimeHandle::revolute_set_limits(self, lower, upper)
    }
    /// Fallible variant of revolute_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_limits(self, lower, upper)
    }
    /// Returns whether the selected revolute joint's motor is enabled.
    pub fn revolute_motor_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_motor_enabled(self)
    }
    /// Fallible variant of revolute_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_motor_enabled(self)
    }
    /// Enables or disables the selected revolute joint's motor.
    pub fn revolute_enable_motor(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_motor(self, enable)
    }
    /// Fallible variant of revolute_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_motor(self, enable)
    }
    /// Returns the selected revolute joint's target motor speed in radians per second.
    pub fn revolute_motor_speed(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_speed(self)
    }
    /// Fallible variant of revolute_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_speed(self)
    }
    /// Sets the selected revolute joint's target motor speed in radians per second; the value must be finite.
    pub fn revolute_set_motor_speed(&mut self, speed: f32) {
        RevoluteJointRuntimeHandle::revolute_set_motor_speed(self, speed)
    }
    /// Fallible variant of revolute_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_motor_speed(self, speed)
    }
    /// Returns the selected revolute joint's current motor torque.
    pub fn revolute_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_torque(self)
    }
    /// Fallible variant of revolute_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_torque(self)
    }
    /// Returns the selected revolute joint's maximum motor torque.
    pub fn revolute_max_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_max_motor_torque(self)
    }
    /// Fallible variant of revolute_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_max_motor_torque(self)
    }
    /// Sets the selected revolute joint's maximum motor torque; the value must be finite and non-negative.
    pub fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        RevoluteJointRuntimeHandle::revolute_set_max_motor_torque(self, torque)
    }
    /// Fallible variant of revolute_set_max_motor_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_max_motor_torque(self, torque)
    }
}

impl<'w> Joint<'w> {
    /// Returns whether the selected revolute joint's spring is enabled.
    pub fn revolute_spring_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_spring_enabled(self)
    }
    /// Fallible variant of revolute_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_spring_enabled(self)
    }
    /// Enables or disables the selected revolute joint's spring.
    pub fn revolute_enable_spring(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_spring(self, enable)
    }
    /// Fallible variant of revolute_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_spring(self, enable)
    }
    /// Returns the selected revolute joint's spring frequency in hertz.
    pub fn revolute_spring_hertz(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_hertz(self)
    }
    /// Fallible variant of revolute_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_hertz(self)
    }
    /// Sets the selected revolute joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of revolute_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_hertz(self, hertz)
    }
    /// Returns the selected revolute joint's spring damping ratio.
    pub fn revolute_spring_damping_ratio(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_damping_ratio(self)
    }
    /// Fallible variant of revolute_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_damping_ratio(self)
    }
    /// Sets the selected revolute joint's spring damping ratio; the value must be finite and non-negative.
    pub fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of revolute_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns the selected revolute joint's target angle in radians.
    pub fn revolute_target_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_target_angle(self)
    }
    /// Fallible variant of revolute_target_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_target_angle(self)
    }
    /// Sets the selected revolute joint's target angle in radians; the value must be finite.
    pub fn revolute_set_target_angle(&mut self, angle: f32) {
        RevoluteJointRuntimeHandle::revolute_set_target_angle(self, angle)
    }
    /// Fallible variant of revolute_set_target_angle; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_target_angle(self, angle)
    }
    /// Returns the selected revolute joint's current angle in radians.
    pub fn revolute_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_angle(self)
    }
    /// Fallible variant of revolute_angle; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_angle(self)
    }
    /// Returns whether the selected revolute joint's limit is enabled.
    pub fn revolute_limit_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_limit_enabled(self)
    }
    /// Fallible variant of revolute_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_limit_enabled(self)
    }
    /// Enables or disables the selected revolute joint's limit.
    pub fn revolute_enable_limit(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_limit(self, enable)
    }
    /// Fallible variant of revolute_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_limit(self, enable)
    }
    /// Returns the selected revolute joint's lower angular limit in radians.
    pub fn revolute_lower_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_lower_limit(self)
    }
    /// Fallible variant of revolute_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_lower_limit(self)
    }
    /// Returns the selected revolute joint's upper angular limit in radians.
    pub fn revolute_upper_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_upper_limit(self)
    }
    /// Fallible variant of revolute_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_upper_limit(self)
    }
    /// Sets the selected revolute joint's lower and upper angular limits in radians; the bounds must be finite and ordered.
    pub fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        RevoluteJointRuntimeHandle::revolute_set_limits(self, lower, upper)
    }
    /// Fallible variant of revolute_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_limits(self, lower, upper)
    }
    /// Returns whether the selected revolute joint's motor is enabled.
    pub fn revolute_motor_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_motor_enabled(self)
    }
    /// Fallible variant of revolute_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_motor_enabled(self)
    }
    /// Enables or disables the selected revolute joint's motor.
    pub fn revolute_enable_motor(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_motor(self, enable)
    }
    /// Fallible variant of revolute_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_motor(self, enable)
    }
    /// Returns the selected revolute joint's target motor speed in radians per second.
    pub fn revolute_motor_speed(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_speed(self)
    }
    /// Fallible variant of revolute_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_speed(self)
    }
    /// Sets the selected revolute joint's target motor speed in radians per second; the value must be finite.
    pub fn revolute_set_motor_speed(&mut self, speed: f32) {
        RevoluteJointRuntimeHandle::revolute_set_motor_speed(self, speed)
    }
    /// Fallible variant of revolute_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_motor_speed(self, speed)
    }
    /// Returns the selected revolute joint's current motor torque.
    pub fn revolute_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_torque(self)
    }
    /// Fallible variant of revolute_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_torque(self)
    }
    /// Returns the selected revolute joint's maximum motor torque.
    pub fn revolute_max_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_max_motor_torque(self)
    }
    /// Fallible variant of revolute_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_max_motor_torque(self)
    }
    /// Sets the selected revolute joint's maximum motor torque; the value must be finite and non-negative.
    pub fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        RevoluteJointRuntimeHandle::revolute_set_max_motor_torque(self, torque)
    }
    /// Fallible variant of revolute_set_max_motor_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_max_motor_torque(self, torque)
    }
}

trait RevoluteJointRuntimeHandle: TypedJointRuntimeHandle {
    fn revolute_spring_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    fn revolute_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_SPRING,
        );
    }

    fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_SPRING,
        )
    }

    fn revolute_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            hertz,
            REVOLUTE_SET_SPRING_HERTZ,
        );
    }

    fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            hertz,
            REVOLUTE_SET_SPRING_HERTZ,
        )
    }

    fn revolute_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            damping_ratio,
            REVOLUTE_SET_SPRING_DAMPING_RATIO,
        );
    }

    fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            damping_ratio,
            REVOLUTE_SET_SPRING_DAMPING_RATIO,
        )
    }

    fn revolute_target_angle(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    fn revolute_set_target_angle(&mut self, angle: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            angle,
            REVOLUTE_SET_TARGET_ANGLE,
        );
    }

    fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            angle,
            REVOLUTE_SET_TARGET_ANGLE,
        )
    }

    fn revolute_angle(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    fn try_revolute_angle(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    fn revolute_limit_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    fn revolute_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_LIMIT,
        );
    }

    fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_LIMIT,
        )
    }

    fn revolute_lower_limit(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    fn revolute_upper_limit(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            lower,
            upper,
            REVOLUTE_SET_LIMITS,
        );
    }

    fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            lower,
            upper,
            REVOLUTE_SET_LIMITS,
        )
    }

    fn revolute_motor_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    fn revolute_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_MOTOR,
        );
    }

    fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            enable,
            REVOLUTE_ENABLE_MOTOR,
        )
    }

    fn revolute_motor_speed(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    fn revolute_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            speed,
            REVOLUTE_SET_MOTOR_SPEED,
        );
    }

    fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            speed,
            REVOLUTE_SET_MOTOR_SPEED,
        )
    }

    fn revolute_motor_torque(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    fn revolute_max_motor_torque(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            torque,
            REVOLUTE_SET_MAX_MOTOR_TORQUE,
        );
    }

    fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Revolute,
            torque,
            REVOLUTE_SET_MAX_MOTOR_TORQUE,
        )
    }
}

impl RevoluteJointRuntimeHandle for OwnedJoint {}

impl RevoluteJointRuntimeHandle for Joint<'_> {}
