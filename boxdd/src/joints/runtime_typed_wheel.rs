use super::*;

#[inline]
fn wheel_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsSpringEnabled)
}

const WHEEL_ENABLE_SPRING: JointSetOp<bool> = JointSetOp::new(JointWriteKind::WheelEnableSpring);

#[inline]
fn wheel_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetSpringHertz)
}

const WHEEL_SET_SPRING_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::WheelSetSpringHertz);

#[inline]
fn wheel_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetSpringDampingRatio)
}

const WHEEL_SET_SPRING_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::WheelSetSpringDampingRatio);

#[inline]
fn wheel_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsLimitEnabled)
}

const WHEEL_ENABLE_LIMIT: JointSetOp<bool> = JointSetOp::new(JointWriteKind::WheelEnableLimit);

#[inline]
fn wheel_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetLowerLimit)
}

#[inline]
fn wheel_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetUpperLimit)
}

const WHEEL_SET_LIMITS: JointSet2Op<f32, f32> = JointSet2Op::new(JointWriteKind::WheelSetLimits);

#[inline]
fn wheel_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsMotorEnabled)
}

const WHEEL_ENABLE_MOTOR: JointSetOp<bool> = JointSetOp::new(JointWriteKind::WheelEnableMotor);

#[inline]
fn wheel_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMotorSpeed)
}

const WHEEL_SET_MOTOR_SPEED: JointSetOp<f32> = JointSetOp::new(JointWriteKind::WheelSetMotorSpeed);

#[inline]
fn wheel_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMotorTorque)
}

#[inline]
fn wheel_max_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMaxMotorTorque)
}

const WHEEL_SET_MAX_MOTOR_TORQUE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::WheelSetMaxMotorTorque);

trait WheelJointRuntimeHandle: TypedJointRuntimeHandle {
    fn wheel_spring_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    fn wheel_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_SPRING,
        );
    }

    fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_SPRING,
        )
    }

    fn wheel_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            hertz,
            WHEEL_SET_SPRING_HERTZ,
        );
    }

    fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            hertz,
            WHEEL_SET_SPRING_HERTZ,
        )
    }

    fn wheel_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            damping_ratio,
            WHEEL_SET_SPRING_DAMPING_RATIO,
        );
    }

    fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            damping_ratio,
            WHEEL_SET_SPRING_DAMPING_RATIO,
        )
    }

    fn wheel_limit_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    fn wheel_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_LIMIT,
        );
    }

    fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_LIMIT,
        )
    }

    fn wheel_lower_limit(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    fn wheel_upper_limit(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            lower,
            upper,
            WHEEL_SET_LIMITS,
        );
    }

    fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            lower,
            upper,
            WHEEL_SET_LIMITS,
        )
    }

    fn wheel_motor_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    fn wheel_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_MOTOR,
        );
    }

    fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_MOTOR,
        )
    }

    fn wheel_motor_speed(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    fn wheel_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            speed,
            WHEEL_SET_MOTOR_SPEED,
        );
    }

    fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            speed,
            WHEEL_SET_MOTOR_SPEED,
        )
    }

    fn wheel_motor_torque(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    fn wheel_max_motor_torque(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            torque,
            WHEEL_SET_MAX_MOTOR_TORQUE,
        );
    }

    fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Wheel,
            torque,
            WHEEL_SET_MAX_MOTOR_TORQUE,
        )
    }
}

impl WheelJointRuntimeHandle for OwnedJoint {}

impl WheelJointRuntimeHandle for Joint<'_> {}

impl World {
    /// Returns whether the selected wheel joint's spring is enabled.
    pub fn wheel_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    /// Fallible variant of wheel_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    /// Enables or disables the selected wheel joint's spring.
    pub fn wheel_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_SPRING,
        )
    }

    /// Fallible variant of wheel_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_SPRING,
        )
    }

    /// Returns the selected wheel joint's spring frequency in hertz.
    pub fn wheel_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    /// Fallible variant of wheel_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    /// Sets the selected wheel joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            hertz,
            WHEEL_SET_SPRING_HERTZ,
        )
    }

    /// Fallible variant of wheel_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            hertz,
            WHEEL_SET_SPRING_HERTZ,
        )
    }

    /// Returns the selected wheel joint's spring damping ratio.
    pub fn wheel_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of wheel_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    /// Sets the selected wheel joint's spring damping ratio; the value must be finite and non-negative.
    pub fn wheel_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            damping_ratio,
            WHEEL_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Fallible variant of wheel_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            damping_ratio,
            WHEEL_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Returns whether the selected wheel joint's limit is enabled.
    pub fn wheel_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    /// Fallible variant of wheel_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    /// Enables or disables the selected wheel joint's limit.
    pub fn wheel_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_LIMIT,
        )
    }

    /// Fallible variant of wheel_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_LIMIT,
        )
    }

    /// Returns the selected wheel joint's lower translation limit in meters.
    pub fn wheel_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_lower_limit_impl)
    }

    /// Fallible variant of wheel_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    /// Returns the selected wheel joint's upper translation limit in meters.
    pub fn wheel_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_upper_limit_impl)
    }

    /// Fallible variant of wheel_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    /// Sets the selected wheel joint's lower and upper translation limits in meters; the bounds must be finite and ordered.
    pub fn wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            lower,
            upper,
            WHEEL_SET_LIMITS,
        )
    }

    /// Fallible variant of wheel_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            lower,
            upper,
            WHEEL_SET_LIMITS,
        )
    }

    /// Returns whether the selected wheel joint's motor is enabled.
    pub fn wheel_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    /// Fallible variant of wheel_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    /// Enables or disables the selected wheel joint's motor.
    pub fn wheel_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_MOTOR,
        )
    }

    /// Fallible variant of wheel_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            enable,
            WHEEL_ENABLE_MOTOR,
        )
    }

    /// Returns the selected wheel joint's target motor speed in radians per second.
    pub fn wheel_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_motor_speed_impl)
    }

    /// Fallible variant of wheel_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    /// Sets the selected wheel joint's target motor speed in radians per second; the value must be finite.
    pub fn wheel_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            speed,
            WHEEL_SET_MOTOR_SPEED,
        )
    }

    /// Fallible variant of wheel_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            speed,
            WHEEL_SET_MOTOR_SPEED,
        )
    }

    /// Returns the selected wheel joint's current motor torque.
    pub fn wheel_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_motor_torque_impl)
    }

    /// Fallible variant of wheel_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    /// Returns the selected wheel joint's maximum motor torque.
    pub fn wheel_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    /// Fallible variant of wheel_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    /// Sets the selected wheel joint's maximum motor torque; the value must be finite and non-negative.
    pub fn wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            torque,
            WHEEL_SET_MAX_MOTOR_TORQUE,
        )
    }

    /// Fallible variant of wheel_set_max_motor_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            torque,
            WHEEL_SET_MAX_MOTOR_TORQUE,
        )
    }
}

impl WorldHandle {
    /// Returns whether the selected wheel joint's spring is enabled.
    pub fn wheel_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    /// Fallible variant of wheel_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    /// Returns the selected wheel joint's spring frequency in hertz.
    pub fn wheel_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    /// Fallible variant of wheel_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    /// Returns the selected wheel joint's spring damping ratio.
    pub fn wheel_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of wheel_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    /// Returns whether the selected wheel joint's limit is enabled.
    pub fn wheel_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    /// Fallible variant of wheel_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    /// Returns the selected wheel joint's lower translation limit in meters.
    pub fn wheel_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_lower_limit_impl)
    }

    /// Fallible variant of wheel_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    /// Returns the selected wheel joint's upper translation limit in meters.
    pub fn wheel_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_upper_limit_impl)
    }

    /// Fallible variant of wheel_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    /// Returns whether the selected wheel joint's motor is enabled.
    pub fn wheel_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    /// Fallible variant of wheel_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    /// Returns the selected wheel joint's target motor speed in radians per second.
    pub fn wheel_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_motor_speed_impl)
    }

    /// Fallible variant of wheel_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    /// Returns the selected wheel joint's current motor torque.
    pub fn wheel_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Wheel, wheel_motor_torque_impl)
    }

    /// Fallible variant of wheel_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    /// Returns the selected wheel joint's maximum motor torque.
    pub fn wheel_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    /// Fallible variant of wheel_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }
}

impl OwnedJoint {
    /// Returns whether the selected wheel joint's spring is enabled.
    pub fn wheel_spring_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_spring_enabled(self)
    }
    /// Fallible variant of wheel_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_spring_enabled(self)
    }
    /// Enables or disables the selected wheel joint's spring.
    pub fn wheel_enable_spring(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_spring(self, enable)
    }
    /// Fallible variant of wheel_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_spring(self, enable)
    }
    /// Returns the selected wheel joint's spring frequency in hertz.
    pub fn wheel_spring_hertz(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_hertz(self)
    }
    /// Fallible variant of wheel_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_hertz(self)
    }
    /// Sets the selected wheel joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of wheel_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_hertz(self, hertz)
    }
    /// Returns the selected wheel joint's spring damping ratio.
    pub fn wheel_spring_damping_ratio(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_damping_ratio(self)
    }
    /// Fallible variant of wheel_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_damping_ratio(self)
    }
    /// Sets the selected wheel joint's spring damping ratio; the value must be finite and non-negative.
    pub fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of wheel_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns whether the selected wheel joint's limit is enabled.
    pub fn wheel_limit_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_limit_enabled(self)
    }
    /// Fallible variant of wheel_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_limit_enabled(self)
    }
    /// Enables or disables the selected wheel joint's limit.
    pub fn wheel_enable_limit(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_limit(self, enable)
    }
    /// Fallible variant of wheel_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_limit(self, enable)
    }
    /// Returns the selected wheel joint's lower translation limit in meters.
    pub fn wheel_lower_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_lower_limit(self)
    }
    /// Fallible variant of wheel_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_lower_limit(self)
    }
    /// Returns the selected wheel joint's upper translation limit in meters.
    pub fn wheel_upper_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_upper_limit(self)
    }
    /// Fallible variant of wheel_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_upper_limit(self)
    }
    /// Sets the selected wheel joint's lower and upper translation limits in meters; the bounds must be finite and ordered.
    pub fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        WheelJointRuntimeHandle::wheel_set_limits(self, lower, upper)
    }
    /// Fallible variant of wheel_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_limits(self, lower, upper)
    }
    /// Returns whether the selected wheel joint's motor is enabled.
    pub fn wheel_motor_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_motor_enabled(self)
    }
    /// Fallible variant of wheel_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_motor_enabled(self)
    }
    /// Enables or disables the selected wheel joint's motor.
    pub fn wheel_enable_motor(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_motor(self, enable)
    }
    /// Fallible variant of wheel_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_motor(self, enable)
    }
    /// Returns the selected wheel joint's target motor speed in radians per second.
    pub fn wheel_motor_speed(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_speed(self)
    }
    /// Fallible variant of wheel_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_speed(self)
    }
    /// Sets the selected wheel joint's target motor speed in radians per second; the value must be finite.
    pub fn wheel_set_motor_speed(&mut self, speed: f32) {
        WheelJointRuntimeHandle::wheel_set_motor_speed(self, speed)
    }
    /// Fallible variant of wheel_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_motor_speed(self, speed)
    }
    /// Returns the selected wheel joint's current motor torque.
    pub fn wheel_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_torque(self)
    }
    /// Fallible variant of wheel_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_torque(self)
    }
    /// Returns the selected wheel joint's maximum motor torque.
    pub fn wheel_max_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_max_motor_torque(self)
    }
    /// Fallible variant of wheel_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_max_motor_torque(self)
    }
    /// Sets the selected wheel joint's maximum motor torque; the value must be finite and non-negative.
    pub fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        WheelJointRuntimeHandle::wheel_set_max_motor_torque(self, torque)
    }
    /// Fallible variant of wheel_set_max_motor_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_max_motor_torque(self, torque)
    }
}

impl<'w> Joint<'w> {
    /// Returns whether the selected wheel joint's spring is enabled.
    pub fn wheel_spring_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_spring_enabled(self)
    }
    /// Fallible variant of wheel_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_spring_enabled(self)
    }
    /// Enables or disables the selected wheel joint's spring.
    pub fn wheel_enable_spring(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_spring(self, enable)
    }
    /// Fallible variant of wheel_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_spring(self, enable)
    }
    /// Returns the selected wheel joint's spring frequency in hertz.
    pub fn wheel_spring_hertz(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_hertz(self)
    }
    /// Fallible variant of wheel_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_hertz(self)
    }
    /// Sets the selected wheel joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of wheel_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_hertz(self, hertz)
    }
    /// Returns the selected wheel joint's spring damping ratio.
    pub fn wheel_spring_damping_ratio(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_damping_ratio(self)
    }
    /// Fallible variant of wheel_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_damping_ratio(self)
    }
    /// Sets the selected wheel joint's spring damping ratio; the value must be finite and non-negative.
    pub fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of wheel_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns whether the selected wheel joint's limit is enabled.
    pub fn wheel_limit_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_limit_enabled(self)
    }
    /// Fallible variant of wheel_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_limit_enabled(self)
    }
    /// Enables or disables the selected wheel joint's limit.
    pub fn wheel_enable_limit(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_limit(self, enable)
    }
    /// Fallible variant of wheel_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_limit(self, enable)
    }
    /// Returns the selected wheel joint's lower translation limit in meters.
    pub fn wheel_lower_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_lower_limit(self)
    }
    /// Fallible variant of wheel_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_lower_limit(self)
    }
    /// Returns the selected wheel joint's upper translation limit in meters.
    pub fn wheel_upper_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_upper_limit(self)
    }
    /// Fallible variant of wheel_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_upper_limit(self)
    }
    /// Sets the selected wheel joint's lower and upper translation limits in meters; the bounds must be finite and ordered.
    pub fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        WheelJointRuntimeHandle::wheel_set_limits(self, lower, upper)
    }
    /// Fallible variant of wheel_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_limits(self, lower, upper)
    }
    /// Returns whether the selected wheel joint's motor is enabled.
    pub fn wheel_motor_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_motor_enabled(self)
    }
    /// Fallible variant of wheel_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_motor_enabled(self)
    }
    /// Enables or disables the selected wheel joint's motor.
    pub fn wheel_enable_motor(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_motor(self, enable)
    }
    /// Fallible variant of wheel_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_motor(self, enable)
    }
    /// Returns the selected wheel joint's target motor speed in radians per second.
    pub fn wheel_motor_speed(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_speed(self)
    }
    /// Fallible variant of wheel_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_speed(self)
    }
    /// Sets the selected wheel joint's target motor speed in radians per second; the value must be finite.
    pub fn wheel_set_motor_speed(&mut self, speed: f32) {
        WheelJointRuntimeHandle::wheel_set_motor_speed(self, speed)
    }
    /// Fallible variant of wheel_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_motor_speed(self, speed)
    }
    /// Returns the selected wheel joint's current motor torque.
    pub fn wheel_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_torque(self)
    }
    /// Fallible variant of wheel_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_torque(self)
    }
    /// Returns the selected wheel joint's maximum motor torque.
    pub fn wheel_max_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_max_motor_torque(self)
    }
    /// Fallible variant of wheel_max_motor_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_max_motor_torque(self)
    }
    /// Sets the selected wheel joint's maximum motor torque; the value must be finite and non-negative.
    pub fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        WheelJointRuntimeHandle::wheel_set_max_motor_torque(self, torque)
    }
    /// Fallible variant of wheel_set_max_motor_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_max_motor_torque(self, torque)
    }
}
