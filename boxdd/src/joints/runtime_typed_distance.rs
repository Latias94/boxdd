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
