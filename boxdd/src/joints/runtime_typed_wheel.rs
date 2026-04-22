use super::*;

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

impl World {
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
}

impl WorldHandle {
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
}

impl OwnedJoint {
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
}

impl<'w> Joint<'w> {
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
}
