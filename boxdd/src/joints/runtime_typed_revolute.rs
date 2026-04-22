use super::*;

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

impl World {
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

impl OwnedJoint {
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

impl<'w> Joint<'w> {
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
