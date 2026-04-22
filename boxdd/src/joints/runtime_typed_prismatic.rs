use super::*;

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
}
