use super::*;

#[inline]
fn prismatic_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsSpringEnabled)
}

const PRISMATIC_ENABLE_SPRING: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::PrismaticEnableSpring);

#[inline]
fn prismatic_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpringHertz)
}

const PRISMATIC_SET_SPRING_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::PrismaticSetSpringHertz);

#[inline]
fn prismatic_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpringDampingRatio)
}

const PRISMATIC_SET_SPRING_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::PrismaticSetSpringDampingRatio);

#[inline]
fn prismatic_target_translation_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetTargetTranslation)
}

const PRISMATIC_SET_TARGET_TRANSLATION: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::PrismaticSetTargetTranslation);

#[inline]
fn prismatic_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsLimitEnabled)
}

const PRISMATIC_ENABLE_LIMIT: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::PrismaticEnableLimit);

#[inline]
fn prismatic_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetLowerLimit)
}

#[inline]
fn prismatic_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetUpperLimit)
}

const PRISMATIC_SET_LIMITS: JointSet2Op<f32, f32> =
    JointSet2Op::new(JointWriteKind::PrismaticSetLimits);

#[inline]
fn prismatic_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsMotorEnabled)
}

const PRISMATIC_ENABLE_MOTOR: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::PrismaticEnableMotor);

#[inline]
fn prismatic_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMotorSpeed)
}

const PRISMATIC_SET_MOTOR_SPEED: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::PrismaticSetMotorSpeed);

#[inline]
fn prismatic_max_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMaxMotorForce)
}

const PRISMATIC_SET_MAX_MOTOR_FORCE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::PrismaticSetMaxMotorForce);

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
    /// Returns whether the selected prismatic joint's spring is enabled.
    pub fn prismatic_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    /// Fallible variant of prismatic_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    /// Enables or disables the selected prismatic joint's spring.
    pub fn prismatic_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_SPRING,
        )
    }

    /// Fallible variant of prismatic_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_SPRING,
        )
    }

    /// Returns the selected prismatic joint's spring frequency in hertz.
    pub fn prismatic_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    /// Fallible variant of prismatic_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    /// Sets the selected prismatic joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            hertz,
            PRISMATIC_SET_SPRING_HERTZ,
        )
    }

    /// Fallible variant of prismatic_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            hertz,
            PRISMATIC_SET_SPRING_HERTZ,
        )
    }

    /// Returns the selected prismatic joint's spring damping ratio.
    pub fn prismatic_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of prismatic_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    /// Sets the selected prismatic joint's spring damping ratio; the value must be finite and non-negative.
    pub fn prismatic_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            damping_ratio,
            PRISMATIC_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Fallible variant of prismatic_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            damping_ratio,
            PRISMATIC_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Returns the selected prismatic joint's target translation in meters.
    pub fn prismatic_target_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    /// Fallible variant of prismatic_target_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_target_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    /// Sets the selected prismatic joint's target translation in meters; the value must be finite.
    pub fn prismatic_set_target_translation(&mut self, id: JointId, translation: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            translation,
            PRISMATIC_SET_TARGET_TRANSLATION,
        )
    }

    /// Fallible variant of prismatic_set_target_translation; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_target_translation(
        &mut self,
        id: JointId,
        translation: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            translation,
            PRISMATIC_SET_TARGET_TRANSLATION,
        )
    }

    /// Returns whether the selected prismatic joint's limit is enabled.
    pub fn prismatic_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    /// Fallible variant of prismatic_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    /// Enables or disables the selected prismatic joint's limit.
    pub fn prismatic_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_LIMIT,
        )
    }

    /// Fallible variant of prismatic_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_LIMIT,
        )
    }

    /// Returns the selected prismatic joint's lower translation limit in meters.
    pub fn prismatic_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    /// Fallible variant of prismatic_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    /// Returns the selected prismatic joint's upper translation limit in meters.
    pub fn prismatic_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    /// Fallible variant of prismatic_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    /// Sets the selected prismatic joint's lower and upper translation limits in meters; the bounds must be finite and ordered.
    pub fn prismatic_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            lower,
            upper,
            PRISMATIC_SET_LIMITS,
        )
    }

    /// Fallible variant of prismatic_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            lower,
            upper,
            PRISMATIC_SET_LIMITS,
        )
    }

    /// Returns whether the selected prismatic joint's motor is enabled.
    pub fn prismatic_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    /// Fallible variant of prismatic_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    /// Enables or disables the selected prismatic joint's motor.
    pub fn prismatic_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_MOTOR,
        )
    }

    /// Fallible variant of prismatic_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_MOTOR,
        )
    }

    /// Returns the selected prismatic joint's target motor speed in meters per second.
    pub fn prismatic_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    /// Fallible variant of prismatic_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    /// Sets the selected prismatic joint's target motor speed in meters per second; the value must be finite.
    pub fn prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            speed,
            PRISMATIC_SET_MOTOR_SPEED,
        )
    }

    /// Fallible variant of prismatic_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            speed,
            PRISMATIC_SET_MOTOR_SPEED,
        )
    }

    /// Returns the selected prismatic joint's maximum motor force in newtons.
    pub fn prismatic_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    /// Fallible variant of prismatic_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    /// Sets the selected prismatic joint's maximum motor force in newtons; the value must be finite and non-negative.
    pub fn prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            force,
            PRISMATIC_SET_MAX_MOTOR_FORCE,
        )
    }

    /// Fallible variant of prismatic_set_max_motor_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            force,
            PRISMATIC_SET_MAX_MOTOR_FORCE,
        )
    }

    /// Returns the selected prismatic joint's current motor force in newtons.
    pub fn prismatic_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    /// Fallible variant of prismatic_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    /// Returns the selected prismatic joint's current translation in meters.
    pub fn prismatic_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    /// Fallible variant of prismatic_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    /// Returns the selected prismatic joint's current translation speed.
    pub fn prismatic_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Prismatic, prismatic_speed_impl)
    }

    /// Fallible variant of prismatic_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }
}

impl WorldHandle {
    /// Returns whether the selected prismatic joint's spring is enabled.
    pub fn prismatic_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    /// Fallible variant of prismatic_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    /// Returns the selected prismatic joint's spring frequency in hertz.
    pub fn prismatic_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    /// Fallible variant of prismatic_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    /// Returns the selected prismatic joint's spring damping ratio.
    pub fn prismatic_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of prismatic_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    /// Returns the selected prismatic joint's target translation in meters.
    pub fn prismatic_target_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    /// Fallible variant of prismatic_target_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_target_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    /// Returns whether the selected prismatic joint's limit is enabled.
    pub fn prismatic_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    /// Fallible variant of prismatic_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    /// Returns the selected prismatic joint's lower translation limit in meters.
    pub fn prismatic_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    /// Fallible variant of prismatic_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    /// Returns the selected prismatic joint's upper translation limit in meters.
    pub fn prismatic_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    /// Fallible variant of prismatic_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    /// Returns whether the selected prismatic joint's motor is enabled.
    pub fn prismatic_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    /// Fallible variant of prismatic_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    /// Returns the selected prismatic joint's target motor speed in meters per second.
    pub fn prismatic_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    /// Fallible variant of prismatic_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    /// Returns the selected prismatic joint's maximum motor force in newtons.
    pub fn prismatic_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    /// Fallible variant of prismatic_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    /// Returns the selected prismatic joint's current motor force in newtons.
    pub fn prismatic_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    /// Fallible variant of prismatic_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    /// Returns the selected prismatic joint's current translation in meters.
    pub fn prismatic_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    /// Fallible variant of prismatic_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    /// Returns the selected prismatic joint's current translation speed.
    pub fn prismatic_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Prismatic, prismatic_speed_impl)
    }

    /// Fallible variant of prismatic_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }
}

trait PrismaticJointRuntimeHandle: TypedJointRuntimeHandle {
    fn prismatic_spring_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    fn prismatic_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_SPRING,
        );
    }

    fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_SPRING,
        )
    }

    fn prismatic_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            hertz,
            PRISMATIC_SET_SPRING_HERTZ,
        );
    }

    fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            hertz,
            PRISMATIC_SET_SPRING_HERTZ,
        )
    }

    fn prismatic_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            damping_ratio,
            PRISMATIC_SET_SPRING_DAMPING_RATIO,
        );
    }

    fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            damping_ratio,
            PRISMATIC_SET_SPRING_DAMPING_RATIO,
        )
    }

    fn prismatic_target_translation(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    fn prismatic_set_target_translation(&mut self, translation: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            translation,
            PRISMATIC_SET_TARGET_TRANSLATION,
        );
    }

    fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            translation,
            PRISMATIC_SET_TARGET_TRANSLATION,
        )
    }

    fn prismatic_limit_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    fn prismatic_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_LIMIT,
        );
    }

    fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_LIMIT,
        )
    }

    fn prismatic_lower_limit(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    fn prismatic_upper_limit(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            lower,
            upper,
            PRISMATIC_SET_LIMITS,
        );
    }

    fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            lower,
            upper,
            PRISMATIC_SET_LIMITS,
        )
    }

    fn prismatic_motor_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    fn prismatic_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_MOTOR,
        );
    }

    fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            enable,
            PRISMATIC_ENABLE_MOTOR,
        )
    }

    fn prismatic_motor_speed(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    fn prismatic_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            speed,
            PRISMATIC_SET_MOTOR_SPEED,
        );
    }

    fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            speed,
            PRISMATIC_SET_MOTOR_SPEED,
        )
    }

    fn prismatic_max_motor_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    fn prismatic_set_max_motor_force(&mut self, force: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            force,
            PRISMATIC_SET_MAX_MOTOR_FORCE,
        );
    }

    fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            force,
            PRISMATIC_SET_MAX_MOTOR_FORCE,
        )
    }

    fn prismatic_motor_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    fn prismatic_translation(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    fn try_prismatic_translation(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    fn prismatic_speed(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }

    fn try_prismatic_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }
}

impl PrismaticJointRuntimeHandle for OwnedJoint {}

impl OwnedJoint {
    /// Returns whether the selected prismatic joint's spring is enabled.
    pub fn prismatic_spring_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_spring_enabled(self)
    }
    /// Fallible variant of prismatic_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_enabled(self)
    }
    /// Enables or disables the selected prismatic joint's spring.
    pub fn prismatic_enable_spring(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_spring(self, enable)
    }
    /// Fallible variant of prismatic_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_spring(self, enable)
    }
    /// Returns the selected prismatic joint's spring frequency in hertz.
    pub fn prismatic_spring_hertz(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_hertz(self)
    }
    /// Fallible variant of prismatic_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_hertz(self)
    }
    /// Sets the selected prismatic joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of prismatic_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_hertz(self, hertz)
    }
    /// Returns the selected prismatic joint's spring damping ratio.
    pub fn prismatic_spring_damping_ratio(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_damping_ratio(self)
    }
    /// Fallible variant of prismatic_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_damping_ratio(self)
    }
    /// Sets the selected prismatic joint's spring damping ratio; the value must be finite and non-negative.
    pub fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of prismatic_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns the selected prismatic joint's target translation in meters.
    pub fn prismatic_target_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_target_translation(self)
    }
    /// Fallible variant of prismatic_target_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_target_translation(self)
    }
    /// Sets the selected prismatic joint's target translation in meters; the value must be finite.
    pub fn prismatic_set_target_translation(&mut self, translation: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_target_translation(self, translation)
    }
    /// Fallible variant of prismatic_set_target_translation; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_target_translation(self, translation)
    }
    /// Returns whether the selected prismatic joint's limit is enabled.
    pub fn prismatic_limit_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_limit_enabled(self)
    }
    /// Fallible variant of prismatic_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_limit_enabled(self)
    }
    /// Enables or disables the selected prismatic joint's limit.
    pub fn prismatic_enable_limit(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_limit(self, enable)
    }
    /// Fallible variant of prismatic_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_limit(self, enable)
    }
    /// Returns the selected prismatic joint's lower translation limit in meters.
    pub fn prismatic_lower_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_lower_limit(self)
    }
    /// Fallible variant of prismatic_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_lower_limit(self)
    }
    /// Returns the selected prismatic joint's upper translation limit in meters.
    pub fn prismatic_upper_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_upper_limit(self)
    }
    /// Fallible variant of prismatic_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_upper_limit(self)
    }
    /// Sets the selected prismatic joint's lower and upper translation limits in meters; the bounds must be finite and ordered.
    pub fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_limits(self, lower, upper)
    }
    /// Fallible variant of prismatic_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_limits(self, lower, upper)
    }
    /// Returns whether the selected prismatic joint's motor is enabled.
    pub fn prismatic_motor_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_motor_enabled(self)
    }
    /// Fallible variant of prismatic_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_enabled(self)
    }
    /// Enables or disables the selected prismatic joint's motor.
    pub fn prismatic_enable_motor(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_motor(self, enable)
    }
    /// Fallible variant of prismatic_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_motor(self, enable)
    }
    /// Returns the selected prismatic joint's target motor speed in meters per second.
    pub fn prismatic_motor_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_speed(self)
    }
    /// Fallible variant of prismatic_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_speed(self)
    }
    /// Sets the selected prismatic joint's target motor speed in meters per second; the value must be finite.
    pub fn prismatic_set_motor_speed(&mut self, speed: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_motor_speed(self, speed)
    }
    /// Fallible variant of prismatic_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_motor_speed(self, speed)
    }
    /// Returns the selected prismatic joint's maximum motor force in newtons.
    pub fn prismatic_max_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_max_motor_force(self)
    }
    /// Fallible variant of prismatic_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_max_motor_force(self)
    }
    /// Sets the selected prismatic joint's maximum motor force in newtons; the value must be finite and non-negative.
    pub fn prismatic_set_max_motor_force(&mut self, force: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_max_motor_force(self, force)
    }
    /// Fallible variant of prismatic_set_max_motor_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_max_motor_force(self, force)
    }
    /// Returns the selected prismatic joint's current motor force in newtons.
    pub fn prismatic_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_force(self)
    }
    /// Fallible variant of prismatic_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_force(self)
    }
    /// Returns the selected prismatic joint's current translation in meters.
    pub fn prismatic_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_translation(self)
    }
    /// Fallible variant of prismatic_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_translation(self)
    }
    /// Returns the selected prismatic joint's current translation speed.
    pub fn prismatic_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_speed(self)
    }
    /// Fallible variant of prismatic_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_speed(self)
    }
}

impl PrismaticJointRuntimeHandle for Joint<'_> {}

impl<'w> Joint<'w> {
    /// Returns whether the selected prismatic joint's spring is enabled.
    pub fn prismatic_spring_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_spring_enabled(self)
    }
    /// Fallible variant of prismatic_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_enabled(self)
    }
    /// Enables or disables the selected prismatic joint's spring.
    pub fn prismatic_enable_spring(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_spring(self, enable)
    }
    /// Fallible variant of prismatic_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_spring(self, enable)
    }
    /// Returns the selected prismatic joint's spring frequency in hertz.
    pub fn prismatic_spring_hertz(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_hertz(self)
    }
    /// Fallible variant of prismatic_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_hertz(self)
    }
    /// Sets the selected prismatic joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of prismatic_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_hertz(self, hertz)
    }
    /// Returns the selected prismatic joint's spring damping ratio.
    pub fn prismatic_spring_damping_ratio(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_damping_ratio(self)
    }
    /// Fallible variant of prismatic_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_damping_ratio(self)
    }
    /// Sets the selected prismatic joint's spring damping ratio; the value must be finite and non-negative.
    pub fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of prismatic_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns the selected prismatic joint's target translation in meters.
    pub fn prismatic_target_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_target_translation(self)
    }
    /// Fallible variant of prismatic_target_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_target_translation(self)
    }
    /// Sets the selected prismatic joint's target translation in meters; the value must be finite.
    pub fn prismatic_set_target_translation(&mut self, translation: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_target_translation(self, translation)
    }
    /// Fallible variant of prismatic_set_target_translation; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_target_translation(self, translation)
    }
    /// Returns whether the selected prismatic joint's limit is enabled.
    pub fn prismatic_limit_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_limit_enabled(self)
    }
    /// Fallible variant of prismatic_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_limit_enabled(self)
    }
    /// Enables or disables the selected prismatic joint's limit.
    pub fn prismatic_enable_limit(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_limit(self, enable)
    }
    /// Fallible variant of prismatic_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_limit(self, enable)
    }
    /// Returns the selected prismatic joint's lower translation limit in meters.
    pub fn prismatic_lower_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_lower_limit(self)
    }
    /// Fallible variant of prismatic_lower_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_lower_limit(self)
    }
    /// Returns the selected prismatic joint's upper translation limit in meters.
    pub fn prismatic_upper_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_upper_limit(self)
    }
    /// Fallible variant of prismatic_upper_limit; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_upper_limit(self)
    }
    /// Sets the selected prismatic joint's lower and upper translation limits in meters; the bounds must be finite and ordered.
    pub fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_limits(self, lower, upper)
    }
    /// Fallible variant of prismatic_set_limits; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_limits(self, lower, upper)
    }
    /// Returns whether the selected prismatic joint's motor is enabled.
    pub fn prismatic_motor_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_motor_enabled(self)
    }
    /// Fallible variant of prismatic_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_enabled(self)
    }
    /// Enables or disables the selected prismatic joint's motor.
    pub fn prismatic_enable_motor(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_motor(self, enable)
    }
    /// Fallible variant of prismatic_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_motor(self, enable)
    }
    /// Returns the selected prismatic joint's target motor speed in meters per second.
    pub fn prismatic_motor_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_speed(self)
    }
    /// Fallible variant of prismatic_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_speed(self)
    }
    /// Sets the selected prismatic joint's target motor speed in meters per second; the value must be finite.
    pub fn prismatic_set_motor_speed(&mut self, speed: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_motor_speed(self, speed)
    }
    /// Fallible variant of prismatic_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_motor_speed(self, speed)
    }
    /// Returns the selected prismatic joint's maximum motor force in newtons.
    pub fn prismatic_max_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_max_motor_force(self)
    }
    /// Fallible variant of prismatic_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_max_motor_force(self)
    }
    /// Sets the selected prismatic joint's maximum motor force in newtons; the value must be finite and non-negative.
    pub fn prismatic_set_max_motor_force(&mut self, force: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_max_motor_force(self, force)
    }
    /// Fallible variant of prismatic_set_max_motor_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_max_motor_force(self, force)
    }
    /// Returns the selected prismatic joint's current motor force in newtons.
    pub fn prismatic_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_force(self)
    }
    /// Fallible variant of prismatic_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_force(self)
    }
    /// Returns the selected prismatic joint's current translation in meters.
    pub fn prismatic_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_translation(self)
    }
    /// Fallible variant of prismatic_translation; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_translation(self)
    }
    /// Returns the selected prismatic joint's current translation speed.
    pub fn prismatic_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_speed(self)
    }
    /// Fallible variant of prismatic_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_prismatic_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_speed(self)
    }
}
