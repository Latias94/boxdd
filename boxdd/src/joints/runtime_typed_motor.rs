use super::*;

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
