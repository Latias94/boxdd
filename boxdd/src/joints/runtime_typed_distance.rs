use super::*;

#[inline]
fn distance_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetLength)
}

const DISTANCE_SET_LENGTH: JointSetOp<f32> = JointSetOp::new(JointWriteKind::DistanceSetLength);

#[inline]
fn distance_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsSpringEnabled)
}

const DISTANCE_ENABLE_SPRING: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::DistanceEnableSpring);

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
const DISTANCE_SET_SPRING_FORCE_RANGE: JointSet2Op<f32, f32> =
    JointSet2Op::new(JointWriteKind::DistanceSetSpringForceRange);

#[inline]
fn distance_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetSpringHertz)
}

const DISTANCE_SET_SPRING_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::DistanceSetSpringHertz);

#[inline]
fn distance_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetSpringDampingRatio)
}

const DISTANCE_SET_SPRING_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::DistanceSetSpringDampingRatio);

#[inline]
fn distance_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsLimitEnabled)
}

const DISTANCE_ENABLE_LIMIT: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::DistanceEnableLimit);

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

const DISTANCE_SET_LENGTH_RANGE: JointSet2Op<f32, f32> =
    JointSet2Op::new(JointWriteKind::DistanceSetLengthRange);

#[inline]
fn distance_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsMotorEnabled)
}

const DISTANCE_ENABLE_MOTOR: JointSetOp<bool> =
    JointSetOp::new(JointWriteKind::DistanceEnableMotor);

#[inline]
fn distance_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMotorSpeed)
}

const DISTANCE_SET_MOTOR_SPEED: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::DistanceSetMotorSpeed);

#[inline]
fn distance_max_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMaxMotorForce)
}

const DISTANCE_SET_MAX_MOTOR_FORCE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::DistanceSetMaxMotorForce);

#[inline]
fn distance_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMotorForce)
}

impl World {
    /// Returns the selected distance joint's target length in meters.
    pub fn distance_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Distance, distance_length_impl)
    }

    /// Fallible variant of distance_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_length_impl,
        )
    }

    /// Sets the selected distance joint's target length in meters; the value must be finite and positive.
    pub fn distance_set_length(&mut self, id: JointId, length: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            length,
            DISTANCE_SET_LENGTH,
        )
    }

    /// Fallible variant of distance_set_length; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_length(&mut self, id: JointId, length: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            length,
            DISTANCE_SET_LENGTH,
        )
    }

    /// Returns whether the selected distance joint's spring is enabled.
    pub fn distance_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    /// Fallible variant of distance_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    /// Enables or disables the selected distance joint's spring.
    pub fn distance_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_SPRING,
        )
    }

    /// Fallible variant of distance_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_SPRING,
        )
    }

    /// Returns the selected distance joint's lower and upper spring-force bounds in newtons.
    pub fn distance_spring_force_range(&self, id: JointId) -> (f32, f32) {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_force_range_impl,
        )
    }

    /// Fallible variant of distance_spring_force_range; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_force_range(&self, id: JointId) -> ApiResult<(f32, f32)> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_force_range_impl,
        )
    }

    /// Returns the selected distance joint's lower spring-force bound in newtons.
    pub fn distance_lower_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    /// Fallible variant of distance_lower_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_lower_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    /// Returns the selected distance joint's upper spring-force bound in newtons.
    pub fn distance_upper_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    /// Fallible variant of distance_upper_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_upper_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    /// Sets the selected distance joint's lower and upper spring-force bounds in newtons; the bounds must be finite and ordered.
    pub fn distance_set_spring_force_range(
        &mut self,
        id: JointId,
        lower_force: f32,
        upper_force: f32,
    ) {
        joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            lower_force,
            upper_force,
            DISTANCE_SET_SPRING_FORCE_RANGE,
        )
    }

    /// Fallible variant of distance_set_spring_force_range; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_force_range(
        &mut self,
        id: JointId,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            lower_force,
            upper_force,
            DISTANCE_SET_SPRING_FORCE_RANGE,
        )
    }

    /// Returns the selected distance joint's spring frequency in hertz.
    pub fn distance_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    /// Fallible variant of distance_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    /// Sets the selected distance joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            hertz,
            DISTANCE_SET_SPRING_HERTZ,
        )
    }

    /// Fallible variant of distance_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            hertz,
            DISTANCE_SET_SPRING_HERTZ,
        )
    }

    /// Returns the selected distance joint's spring damping ratio.
    pub fn distance_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of distance_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    /// Sets the selected distance joint's spring damping ratio; the value must be finite and non-negative.
    pub fn distance_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            damping_ratio,
            DISTANCE_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Fallible variant of distance_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            damping_ratio,
            DISTANCE_SET_SPRING_DAMPING_RATIO,
        )
    }

    /// Returns whether the selected distance joint's limit is enabled.
    pub fn distance_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    /// Fallible variant of distance_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    /// Enables or disables the selected distance joint's limit.
    pub fn distance_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_LIMIT,
        )
    }

    /// Fallible variant of distance_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_LIMIT,
        )
    }

    /// Returns the selected distance joint's minimum length limit in meters.
    pub fn distance_min_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    /// Fallible variant of distance_min_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_min_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    /// Returns the selected distance joint's maximum length limit in meters.
    pub fn distance_max_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    /// Fallible variant of distance_max_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    /// Returns the selected distance joint's current anchor separation in meters.
    pub fn distance_current_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    /// Fallible variant of distance_current_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_current_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    /// Sets the selected distance joint's minimum and maximum length limits in meters; the bounds must be finite, non-negative, and ordered.
    pub fn distance_set_length_range(&mut self, id: JointId, min_length: f32, max_length: f32) {
        joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            min_length,
            max_length,
            DISTANCE_SET_LENGTH_RANGE,
        )
    }

    /// Fallible variant of distance_set_length_range; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_length_range(
        &mut self,
        id: JointId,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            min_length,
            max_length,
            DISTANCE_SET_LENGTH_RANGE,
        )
    }

    /// Returns whether the selected distance joint's motor is enabled.
    pub fn distance_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    /// Fallible variant of distance_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    /// Enables or disables the selected distance joint's motor.
    pub fn distance_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_MOTOR,
        )
    }

    /// Fallible variant of distance_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_MOTOR,
        )
    }

    /// Returns the selected distance joint's target motor speed in meters per second.
    pub fn distance_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    /// Fallible variant of distance_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    /// Sets the selected distance joint's target motor speed in meters per second; the value must be finite.
    pub fn distance_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            speed,
            DISTANCE_SET_MOTOR_SPEED,
        )
    }

    /// Fallible variant of distance_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            speed,
            DISTANCE_SET_MOTOR_SPEED,
        )
    }

    /// Returns the selected distance joint's maximum motor force in newtons.
    pub fn distance_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    /// Fallible variant of distance_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    /// Sets the selected distance joint's maximum motor force in newtons; the value must be finite and non-negative.
    pub fn distance_set_max_motor_force(&mut self, id: JointId, force: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            force,
            DISTANCE_SET_MAX_MOTOR_FORCE,
        )
    }

    /// Fallible variant of distance_set_max_motor_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            force,
            DISTANCE_SET_MAX_MOTOR_FORCE,
        )
    }

    /// Returns the selected distance joint's current motor force in newtons.
    pub fn distance_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_force_impl,
        )
    }

    /// Fallible variant of distance_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_force_impl,
        )
    }
}

impl WorldHandle {
    /// Returns the selected distance joint's target length in meters.
    pub fn distance_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Distance, distance_length_impl)
    }

    /// Fallible variant of distance_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_length_impl,
        )
    }

    /// Returns whether the selected distance joint's spring is enabled.
    pub fn distance_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    /// Fallible variant of distance_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    /// Returns the selected distance joint's lower and upper spring-force bounds in newtons.
    pub fn distance_spring_force_range(&self, id: JointId) -> (f32, f32) {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_force_range_impl,
        )
    }

    /// Fallible variant of distance_spring_force_range; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_force_range(&self, id: JointId) -> ApiResult<(f32, f32)> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_force_range_impl,
        )
    }

    /// Returns the selected distance joint's lower spring-force bound in newtons.
    pub fn distance_lower_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    /// Fallible variant of distance_lower_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_lower_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    /// Returns the selected distance joint's upper spring-force bound in newtons.
    pub fn distance_upper_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    /// Fallible variant of distance_upper_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_upper_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    /// Returns the selected distance joint's spring frequency in hertz.
    pub fn distance_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    /// Fallible variant of distance_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    /// Returns the selected distance joint's spring damping ratio.
    pub fn distance_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    /// Fallible variant of distance_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    /// Returns whether the selected distance joint's limit is enabled.
    pub fn distance_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    /// Fallible variant of distance_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    /// Returns the selected distance joint's minimum length limit in meters.
    pub fn distance_min_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    /// Fallible variant of distance_min_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_min_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    /// Returns the selected distance joint's maximum length limit in meters.
    pub fn distance_max_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    /// Fallible variant of distance_max_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    /// Returns the selected distance joint's current anchor separation in meters.
    pub fn distance_current_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    /// Fallible variant of distance_current_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_current_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    /// Returns whether the selected distance joint's motor is enabled.
    pub fn distance_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    /// Fallible variant of distance_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    /// Returns the selected distance joint's target motor speed in meters per second.
    pub fn distance_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    /// Fallible variant of distance_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    /// Returns the selected distance joint's maximum motor force in newtons.
    pub fn distance_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    /// Fallible variant of distance_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    /// Returns the selected distance joint's current motor force in newtons.
    pub fn distance_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_force_impl,
        )
    }

    /// Fallible variant of distance_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Distance,
            distance_motor_force_impl,
        )
    }
}

trait DistanceJointRuntimeHandle: TypedJointRuntimeHandle {
    fn distance_length(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_length_impl,
        )
    }

    fn try_distance_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_length_impl,
        )
    }

    fn distance_set_length(&mut self, length: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            length,
            DISTANCE_SET_LENGTH,
        );
    }

    fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            length,
            DISTANCE_SET_LENGTH,
        )
    }

    fn distance_spring_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    fn distance_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_SPRING,
        );
    }

    fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_SPRING,
        )
    }

    fn distance_spring_force_range(&self) -> (f32, f32) {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_force_range_impl,
        )
    }

    fn try_distance_spring_force_range(&self) -> ApiResult<(f32, f32)> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_force_range_impl,
        )
    }

    fn distance_lower_spring_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    fn distance_upper_spring_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            lower_force,
            upper_force,
            DISTANCE_SET_SPRING_FORCE_RANGE,
        );
    }

    fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            lower_force,
            upper_force,
            DISTANCE_SET_SPRING_FORCE_RANGE,
        )
    }

    fn distance_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    fn distance_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            hertz,
            DISTANCE_SET_SPRING_HERTZ,
        );
    }

    fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            hertz,
            DISTANCE_SET_SPRING_HERTZ,
        )
    }

    fn distance_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            damping_ratio,
            DISTANCE_SET_SPRING_DAMPING_RATIO,
        );
    }

    fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            damping_ratio,
            DISTANCE_SET_SPRING_DAMPING_RATIO,
        )
    }

    fn distance_limit_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    fn distance_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_LIMIT,
        );
    }

    fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_LIMIT,
        )
    }

    fn distance_min_length(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    fn try_distance_min_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    fn distance_max_length(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    fn try_distance_max_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    fn distance_current_length(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    fn try_distance_current_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            min_length,
            max_length,
            DISTANCE_SET_LENGTH_RANGE,
        );
    }

    fn try_distance_set_length_range(&mut self, min_length: f32, max_length: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            min_length,
            max_length,
            DISTANCE_SET_LENGTH_RANGE,
        )
    }

    fn distance_motor_enabled(&self) -> bool {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    fn distance_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_MOTOR,
        );
    }

    fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            enable,
            DISTANCE_ENABLE_MOTOR,
        )
    }

    fn distance_motor_speed(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    fn distance_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            speed,
            DISTANCE_SET_MOTOR_SPEED,
        );
    }

    fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            speed,
            DISTANCE_SET_MOTOR_SPEED,
        )
    }

    fn distance_max_motor_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    fn distance_set_max_motor_force(&mut self, force: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            force,
            DISTANCE_SET_MAX_MOTOR_FORCE,
        );
    }

    fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            force,
            DISTANCE_SET_MAX_MOTOR_FORCE,
        )
    }

    fn distance_motor_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_motor_force_impl,
        )
    }

    fn try_distance_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Distance,
            distance_motor_force_impl,
        )
    }
}

impl DistanceJointRuntimeHandle for OwnedJoint {}

impl DistanceJointRuntimeHandle for Joint<'_> {}

impl OwnedJoint {
    /// Returns the selected distance joint's target length in meters.
    pub fn distance_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_length(self)
    }
    /// Fallible variant of distance_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_length(self)
    }
    /// Sets the selected distance joint's target length in meters; the value must be finite and positive.
    pub fn distance_set_length(&mut self, length: f32) {
        DistanceJointRuntimeHandle::distance_set_length(self, length)
    }
    /// Fallible variant of distance_set_length; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length(self, length)
    }
    /// Returns whether the selected distance joint's spring is enabled.
    pub fn distance_spring_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_spring_enabled(self)
    }
    /// Fallible variant of distance_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_spring_enabled(self)
    }
    /// Enables or disables the selected distance joint's spring.
    pub fn distance_enable_spring(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_spring(self, enable)
    }
    /// Fallible variant of distance_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_spring(self, enable)
    }
    /// Returns the selected distance joint's lower and upper spring-force bounds in newtons.
    pub fn distance_spring_force_range(&self) -> (f32, f32) {
        DistanceJointRuntimeHandle::distance_spring_force_range(self)
    }
    /// Fallible variant of distance_spring_force_range; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_force_range(&self) -> ApiResult<(f32, f32)> {
        DistanceJointRuntimeHandle::try_distance_spring_force_range(self)
    }
    /// Returns the selected distance joint's lower spring-force bound in newtons.
    pub fn distance_lower_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_lower_spring_force(self)
    }
    /// Fallible variant of distance_lower_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_lower_spring_force(self)
    }
    /// Returns the selected distance joint's upper spring-force bound in newtons.
    pub fn distance_upper_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_upper_spring_force(self)
    }
    /// Fallible variant of distance_upper_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_upper_spring_force(self)
    }
    /// Sets the selected distance joint's lower and upper spring-force bounds in newtons; the bounds must be finite and ordered.
    pub fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_force_range(self, lower_force, upper_force)
    }
    /// Fallible variant of distance_set_spring_force_range; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
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
    /// Returns the selected distance joint's spring frequency in hertz.
    pub fn distance_spring_hertz(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_hertz(self)
    }
    /// Fallible variant of distance_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_hertz(self)
    }
    /// Sets the selected distance joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn distance_set_spring_hertz(&mut self, hertz: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of distance_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_hertz(self, hertz)
    }
    /// Returns the selected distance joint's spring damping ratio.
    pub fn distance_spring_damping_ratio(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_damping_ratio(self)
    }
    /// Fallible variant of distance_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_damping_ratio(self)
    }
    /// Sets the selected distance joint's spring damping ratio; the value must be finite and non-negative.
    pub fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of distance_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns whether the selected distance joint's limit is enabled.
    pub fn distance_limit_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_limit_enabled(self)
    }
    /// Fallible variant of distance_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_limit_enabled(self)
    }
    /// Enables or disables the selected distance joint's limit.
    pub fn distance_enable_limit(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_limit(self, enable)
    }
    /// Fallible variant of distance_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_limit(self, enable)
    }
    /// Returns the selected distance joint's minimum length limit in meters.
    pub fn distance_min_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_min_length(self)
    }
    /// Fallible variant of distance_min_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_min_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_min_length(self)
    }
    /// Returns the selected distance joint's maximum length limit in meters.
    pub fn distance_max_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_length(self)
    }
    /// Fallible variant of distance_max_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_length(self)
    }
    /// Returns the selected distance joint's current anchor separation in meters.
    pub fn distance_current_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_current_length(self)
    }
    /// Fallible variant of distance_current_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_current_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_current_length(self)
    }
    /// Sets the selected distance joint's minimum and maximum length limits in meters; the bounds must be finite, non-negative, and ordered.
    pub fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        DistanceJointRuntimeHandle::distance_set_length_range(self, min_length, max_length)
    }
    /// Fallible variant of distance_set_length_range; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_length_range(
        &mut self,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length_range(self, min_length, max_length)
    }
    /// Returns whether the selected distance joint's motor is enabled.
    pub fn distance_motor_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_motor_enabled(self)
    }
    /// Fallible variant of distance_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_motor_enabled(self)
    }
    /// Enables or disables the selected distance joint's motor.
    pub fn distance_enable_motor(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_motor(self, enable)
    }
    /// Fallible variant of distance_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_motor(self, enable)
    }
    /// Returns the selected distance joint's target motor speed in meters per second.
    pub fn distance_motor_speed(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_speed(self)
    }
    /// Fallible variant of distance_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_speed(self)
    }
    /// Sets the selected distance joint's target motor speed in meters per second; the value must be finite.
    pub fn distance_set_motor_speed(&mut self, speed: f32) {
        DistanceJointRuntimeHandle::distance_set_motor_speed(self, speed)
    }
    /// Fallible variant of distance_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_motor_speed(self, speed)
    }
    /// Returns the selected distance joint's maximum motor force in newtons.
    pub fn distance_max_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_motor_force(self)
    }
    /// Fallible variant of distance_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_motor_force(self)
    }
    /// Sets the selected distance joint's maximum motor force in newtons; the value must be finite and non-negative.
    pub fn distance_set_max_motor_force(&mut self, force: f32) {
        DistanceJointRuntimeHandle::distance_set_max_motor_force(self, force)
    }
    /// Fallible variant of distance_set_max_motor_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_max_motor_force(self, force)
    }
    /// Returns the selected distance joint's current motor force in newtons.
    pub fn distance_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_force(self)
    }
    /// Fallible variant of distance_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_force(self)
    }
}

impl<'w> Joint<'w> {
    /// Returns the selected distance joint's target length in meters.
    pub fn distance_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_length(self)
    }
    /// Fallible variant of distance_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_length(self)
    }
    /// Sets the selected distance joint's target length in meters; the value must be finite and positive.
    pub fn distance_set_length(&mut self, length: f32) {
        DistanceJointRuntimeHandle::distance_set_length(self, length)
    }
    /// Fallible variant of distance_set_length; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length(self, length)
    }
    /// Returns whether the selected distance joint's spring is enabled.
    pub fn distance_spring_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_spring_enabled(self)
    }
    /// Fallible variant of distance_spring_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_spring_enabled(self)
    }
    /// Enables or disables the selected distance joint's spring.
    pub fn distance_enable_spring(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_spring(self, enable)
    }
    /// Fallible variant of distance_enable_spring; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_spring(self, enable)
    }
    /// Returns the selected distance joint's lower and upper spring-force bounds in newtons.
    pub fn distance_spring_force_range(&self) -> (f32, f32) {
        DistanceJointRuntimeHandle::distance_spring_force_range(self)
    }
    /// Fallible variant of distance_spring_force_range; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_force_range(&self) -> ApiResult<(f32, f32)> {
        DistanceJointRuntimeHandle::try_distance_spring_force_range(self)
    }
    /// Returns the selected distance joint's lower spring-force bound in newtons.
    pub fn distance_lower_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_lower_spring_force(self)
    }
    /// Fallible variant of distance_lower_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_lower_spring_force(self)
    }
    /// Returns the selected distance joint's upper spring-force bound in newtons.
    pub fn distance_upper_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_upper_spring_force(self)
    }
    /// Fallible variant of distance_upper_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_upper_spring_force(self)
    }
    /// Sets the selected distance joint's lower and upper spring-force bounds in newtons; the bounds must be finite and ordered.
    pub fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_force_range(self, lower_force, upper_force)
    }
    /// Fallible variant of distance_set_spring_force_range; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
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
    /// Returns the selected distance joint's spring frequency in hertz.
    pub fn distance_spring_hertz(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_hertz(self)
    }
    /// Fallible variant of distance_spring_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_hertz(self)
    }
    /// Sets the selected distance joint's spring frequency in hertz; the value must be finite and non-negative.
    pub fn distance_set_spring_hertz(&mut self, hertz: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_hertz(self, hertz)
    }
    /// Fallible variant of distance_set_spring_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_hertz(self, hertz)
    }
    /// Returns the selected distance joint's spring damping ratio.
    pub fn distance_spring_damping_ratio(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_damping_ratio(self)
    }
    /// Fallible variant of distance_spring_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_damping_ratio(self)
    }
    /// Sets the selected distance joint's spring damping ratio; the value must be finite and non-negative.
    pub fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of distance_set_spring_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_damping_ratio(self, damping_ratio)
    }
    /// Returns whether the selected distance joint's limit is enabled.
    pub fn distance_limit_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_limit_enabled(self)
    }
    /// Fallible variant of distance_limit_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_limit_enabled(self)
    }
    /// Enables or disables the selected distance joint's limit.
    pub fn distance_enable_limit(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_limit(self, enable)
    }
    /// Fallible variant of distance_enable_limit; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_limit(self, enable)
    }
    /// Returns the selected distance joint's minimum length limit in meters.
    pub fn distance_min_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_min_length(self)
    }
    /// Fallible variant of distance_min_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_min_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_min_length(self)
    }
    /// Returns the selected distance joint's maximum length limit in meters.
    pub fn distance_max_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_length(self)
    }
    /// Fallible variant of distance_max_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_length(self)
    }
    /// Returns the selected distance joint's current anchor separation in meters.
    pub fn distance_current_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_current_length(self)
    }
    /// Fallible variant of distance_current_length; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_current_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_current_length(self)
    }
    /// Sets the selected distance joint's minimum and maximum length limits in meters; the bounds must be finite, non-negative, and ordered.
    pub fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        DistanceJointRuntimeHandle::distance_set_length_range(self, min_length, max_length)
    }
    /// Fallible variant of distance_set_length_range; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_length_range(
        &mut self,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length_range(self, min_length, max_length)
    }
    /// Returns whether the selected distance joint's motor is enabled.
    pub fn distance_motor_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_motor_enabled(self)
    }
    /// Fallible variant of distance_motor_enabled; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_motor_enabled(self)
    }
    /// Enables or disables the selected distance joint's motor.
    pub fn distance_enable_motor(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_motor(self, enable)
    }
    /// Fallible variant of distance_enable_motor; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_motor(self, enable)
    }
    /// Returns the selected distance joint's target motor speed in meters per second.
    pub fn distance_motor_speed(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_speed(self)
    }
    /// Fallible variant of distance_motor_speed; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_speed(self)
    }
    /// Sets the selected distance joint's target motor speed in meters per second; the value must be finite.
    pub fn distance_set_motor_speed(&mut self, speed: f32) {
        DistanceJointRuntimeHandle::distance_set_motor_speed(self, speed)
    }
    /// Fallible variant of distance_set_motor_speed; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_motor_speed(self, speed)
    }
    /// Returns the selected distance joint's maximum motor force in newtons.
    pub fn distance_max_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_motor_force(self)
    }
    /// Fallible variant of distance_max_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_motor_force(self)
    }
    /// Sets the selected distance joint's maximum motor force in newtons; the value must be finite and non-negative.
    pub fn distance_set_max_motor_force(&mut self, force: f32) {
        DistanceJointRuntimeHandle::distance_set_max_motor_force(self, force)
    }
    /// Fallible variant of distance_set_max_motor_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_max_motor_force(self, force)
    }
    /// Returns the selected distance joint's current motor force in newtons.
    pub fn distance_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_force(self)
    }
    /// Fallible variant of distance_motor_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_distance_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_force(self)
    }
}
