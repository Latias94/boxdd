use super::*;

#[inline]
fn motor_linear_velocity_impl(id: JointId) -> Vec2 {
    joint_vec2_read_impl(id, ffi::b2MotorJoint_GetLinearVelocity)
}

const MOTOR_SET_LINEAR_VELOCITY: JointSetOp<Vec2> =
    JointSetOp::new(JointWriteKind::MotorSetLinearVelocity);

#[inline]
fn motor_angular_velocity_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularVelocity)
}

const MOTOR_SET_ANGULAR_VELOCITY: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetAngularVelocity);

#[inline]
fn motor_max_velocity_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxVelocityForce)
}

const MOTOR_SET_MAX_VELOCITY_FORCE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetMaxVelocityForce);

#[inline]
fn motor_max_velocity_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxVelocityTorque)
}

const MOTOR_SET_MAX_VELOCITY_TORQUE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetMaxVelocityTorque);

#[inline]
fn motor_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetLinearHertz)
}

const MOTOR_SET_LINEAR_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetLinearHertz);

#[inline]
fn motor_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetLinearDampingRatio)
}

const MOTOR_SET_LINEAR_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetLinearDampingRatio);

#[inline]
fn motor_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularHertz)
}

const MOTOR_SET_ANGULAR_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetAngularHertz);

#[inline]
fn motor_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularDampingRatio)
}

const MOTOR_SET_ANGULAR_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetAngularDampingRatio);

#[inline]
fn motor_max_spring_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxSpringForce)
}

const MOTOR_SET_MAX_SPRING_FORCE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetMaxSpringForce);

#[inline]
fn motor_max_spring_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxSpringTorque)
}

const MOTOR_SET_MAX_SPRING_TORQUE: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::MotorSetMaxSpringTorque);

trait MotorJointRuntimeHandle: TypedJointRuntimeHandle {
    fn motor_linear_velocity(&self) -> Vec2 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            v.into(),
            MOTOR_SET_LINEAR_VELOCITY,
        );
    }

    fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            v.into(),
            MOTOR_SET_LINEAR_VELOCITY,
        )
    }

    fn motor_angular_velocity(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    fn motor_set_angular_velocity(&mut self, w: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            w,
            MOTOR_SET_ANGULAR_VELOCITY,
        );
    }

    fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            w,
            MOTOR_SET_ANGULAR_VELOCITY,
        )
    }

    fn motor_max_velocity_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    fn motor_set_max_velocity_force(&mut self, f: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            f,
            MOTOR_SET_MAX_VELOCITY_FORCE,
        );
    }

    fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            f,
            MOTOR_SET_MAX_VELOCITY_FORCE,
        )
    }

    fn motor_max_velocity_torque(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    fn motor_set_max_velocity_torque(&mut self, t: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            t,
            MOTOR_SET_MAX_VELOCITY_TORQUE,
        );
    }

    fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            t,
            MOTOR_SET_MAX_VELOCITY_TORQUE,
        )
    }

    fn motor_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    fn motor_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            hertz,
            MOTOR_SET_LINEAR_HERTZ,
        );
    }

    fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            hertz,
            MOTOR_SET_LINEAR_HERTZ,
        )
    }

    fn motor_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            damping,
            MOTOR_SET_LINEAR_DAMPING_RATIO,
        );
    }

    fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            damping,
            MOTOR_SET_LINEAR_DAMPING_RATIO,
        )
    }

    fn motor_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    fn motor_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            hertz,
            MOTOR_SET_ANGULAR_HERTZ,
        );
    }

    fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            hertz,
            MOTOR_SET_ANGULAR_HERTZ,
        )
    }

    fn motor_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            damping,
            MOTOR_SET_ANGULAR_DAMPING_RATIO,
        );
    }

    fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            damping,
            MOTOR_SET_ANGULAR_DAMPING_RATIO,
        )
    }

    fn motor_max_spring_force(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    fn motor_set_max_spring_force(&mut self, f: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            f,
            MOTOR_SET_MAX_SPRING_FORCE,
        );
    }

    fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            f,
            MOTOR_SET_MAX_SPRING_FORCE,
        )
    }

    fn motor_max_spring_torque(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    fn motor_set_max_spring_torque(&mut self, t: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            t,
            MOTOR_SET_MAX_SPRING_TORQUE,
        );
    }

    fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Motor,
            t,
            MOTOR_SET_MAX_SPRING_TORQUE,
        )
    }
}

impl MotorJointRuntimeHandle for OwnedJoint {}

impl MotorJointRuntimeHandle for Joint<'_> {}

impl World {
    /// Returns the selected motor joint's target linear velocity.
    pub fn motor_linear_velocity(&self, id: JointId) -> Vec2 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    /// Fallible variant of motor_linear_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_velocity(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    /// Sets the selected motor joint's target linear velocity; both components must be finite.
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, id: JointId, v: V) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            v.into(),
            MOTOR_SET_LINEAR_VELOCITY,
        )
    }

    /// Fallible variant of motor_set_linear_velocity; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(
        &mut self,
        id: JointId,
        v: V,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            v.into(),
            MOTOR_SET_LINEAR_VELOCITY,
        )
    }

    /// Returns the selected motor joint's target angular velocity.
    pub fn motor_angular_velocity(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    /// Fallible variant of motor_angular_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_velocity(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    /// Sets the selected motor joint's target angular velocity; the value must be finite.
    pub fn motor_set_angular_velocity(&mut self, id: JointId, w: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            w,
            MOTOR_SET_ANGULAR_VELOCITY,
        )
    }

    /// Fallible variant of motor_set_angular_velocity; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_velocity(&mut self, id: JointId, w: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            w,
            MOTOR_SET_ANGULAR_VELOCITY,
        )
    }

    /// Returns the selected motor joint's maximum velocity-control force.
    pub fn motor_max_velocity_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    /// Fallible variant of motor_max_velocity_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    /// Sets the selected motor joint's maximum velocity-control force; the value must be finite and non-negative.
    pub fn motor_set_max_velocity_force(&mut self, id: JointId, f: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            f,
            MOTOR_SET_MAX_VELOCITY_FORCE,
        )
    }

    /// Fallible variant of motor_set_max_velocity_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_velocity_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            f,
            MOTOR_SET_MAX_VELOCITY_FORCE,
        )
    }

    /// Returns the selected motor joint's maximum velocity-control torque.
    pub fn motor_max_velocity_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    /// Fallible variant of motor_max_velocity_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    /// Sets the selected motor joint's maximum velocity-control torque; the value must be finite and non-negative.
    pub fn motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            t,
            MOTOR_SET_MAX_VELOCITY_TORQUE,
        )
    }

    /// Fallible variant of motor_set_max_velocity_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            t,
            MOTOR_SET_MAX_VELOCITY_TORQUE,
        )
    }

    /// Returns the selected motor joint's linear spring frequency in hertz.
    pub fn motor_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Motor, motor_linear_hertz_impl)
    }

    /// Fallible variant of motor_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    /// Sets the selected motor joint's linear spring frequency in hertz; the value must be finite and non-negative.
    pub fn motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            hertz,
            MOTOR_SET_LINEAR_HERTZ,
        )
    }

    /// Fallible variant of motor_set_linear_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            hertz,
            MOTOR_SET_LINEAR_HERTZ,
        )
    }

    /// Returns the selected motor joint's linear spring damping ratio.
    pub fn motor_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    /// Fallible variant of motor_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    /// Sets the selected motor joint's linear spring damping ratio; the value must be finite and non-negative.
    pub fn motor_set_linear_damping_ratio(&mut self, id: JointId, damping: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            damping,
            MOTOR_SET_LINEAR_DAMPING_RATIO,
        )
    }

    /// Fallible variant of motor_set_linear_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            damping,
            MOTOR_SET_LINEAR_DAMPING_RATIO,
        )
    }

    /// Returns the selected motor joint's angular spring frequency in hertz.
    pub fn motor_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Motor, motor_angular_hertz_impl)
    }

    /// Fallible variant of motor_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    /// Sets the selected motor joint's angular spring frequency in hertz; the value must be finite and non-negative.
    pub fn motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            hertz,
            MOTOR_SET_ANGULAR_HERTZ,
        )
    }

    /// Fallible variant of motor_set_angular_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            hertz,
            MOTOR_SET_ANGULAR_HERTZ,
        )
    }

    /// Returns the selected motor joint's angular spring damping ratio.
    pub fn motor_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    /// Fallible variant of motor_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    /// Sets the selected motor joint's angular spring damping ratio; the value must be finite and non-negative.
    pub fn motor_set_angular_damping_ratio(&mut self, id: JointId, damping: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            damping,
            MOTOR_SET_ANGULAR_DAMPING_RATIO,
        )
    }

    /// Fallible variant of motor_set_angular_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            damping,
            MOTOR_SET_ANGULAR_DAMPING_RATIO,
        )
    }

    /// Returns the selected motor joint's maximum spring force.
    pub fn motor_max_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    /// Fallible variant of motor_max_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    /// Sets the selected motor joint's maximum spring force; the value must be finite and non-negative.
    pub fn motor_set_max_spring_force(&mut self, id: JointId, f: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            f,
            MOTOR_SET_MAX_SPRING_FORCE,
        )
    }

    /// Fallible variant of motor_set_max_spring_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_spring_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            f,
            MOTOR_SET_MAX_SPRING_FORCE,
        )
    }

    /// Returns the selected motor joint's maximum spring torque.
    pub fn motor_max_spring_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    /// Fallible variant of motor_max_spring_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    /// Sets the selected motor joint's maximum spring torque; the value must be finite and non-negative.
    pub fn motor_set_max_spring_torque(&mut self, id: JointId, t: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            t,
            MOTOR_SET_MAX_SPRING_TORQUE,
        )
    }

    /// Fallible variant of motor_set_max_spring_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_spring_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            t,
            MOTOR_SET_MAX_SPRING_TORQUE,
        )
    }
}

impl WorldHandle {
    /// Returns the selected motor joint's target linear velocity.
    pub fn motor_linear_velocity(&self, id: JointId) -> Vec2 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    /// Fallible variant of motor_linear_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_velocity(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    /// Returns the selected motor joint's target angular velocity.
    pub fn motor_angular_velocity(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    /// Fallible variant of motor_angular_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_velocity(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    /// Returns the selected motor joint's maximum velocity-control force.
    pub fn motor_max_velocity_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    /// Fallible variant of motor_max_velocity_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    /// Returns the selected motor joint's maximum velocity-control torque.
    pub fn motor_max_velocity_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    /// Fallible variant of motor_max_velocity_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    /// Returns the selected motor joint's linear spring frequency in hertz.
    pub fn motor_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Motor, motor_linear_hertz_impl)
    }

    /// Fallible variant of motor_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    /// Returns the selected motor joint's linear spring damping ratio.
    pub fn motor_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    /// Fallible variant of motor_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    /// Returns the selected motor joint's angular spring frequency in hertz.
    pub fn motor_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Motor, motor_angular_hertz_impl)
    }

    /// Fallible variant of motor_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    /// Returns the selected motor joint's angular spring damping ratio.
    pub fn motor_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    /// Fallible variant of motor_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    /// Returns the selected motor joint's maximum spring force.
    pub fn motor_max_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    /// Fallible variant of motor_max_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    /// Returns the selected motor joint's maximum spring torque.
    pub fn motor_max_spring_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    /// Fallible variant of motor_max_spring_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }
}

impl OwnedJoint {
    /// Returns the selected motor joint's target linear velocity.
    pub fn motor_linear_velocity(&self) -> Vec2 {
        MotorJointRuntimeHandle::motor_linear_velocity(self)
    }
    /// Fallible variant of motor_linear_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        MotorJointRuntimeHandle::try_motor_linear_velocity(self)
    }
    /// Sets the selected motor joint's target linear velocity; both components must be finite.
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        MotorJointRuntimeHandle::motor_set_linear_velocity(self, v)
    }
    /// Fallible variant of motor_set_linear_velocity; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_velocity(self, v)
    }
    /// Returns the selected motor joint's target angular velocity.
    pub fn motor_angular_velocity(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_velocity(self)
    }
    /// Fallible variant of motor_angular_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_velocity(self)
    }
    /// Sets the selected motor joint's target angular velocity; the value must be finite.
    pub fn motor_set_angular_velocity(&mut self, w: f32) {
        MotorJointRuntimeHandle::motor_set_angular_velocity(self, w)
    }
    /// Fallible variant of motor_set_angular_velocity; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_velocity(self, w)
    }
    /// Returns the selected motor joint's maximum velocity-control force.
    pub fn motor_max_velocity_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_force(self)
    }
    /// Fallible variant of motor_max_velocity_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_force(self)
    }
    /// Sets the selected motor joint's maximum velocity-control force; the value must be finite and non-negative.
    pub fn motor_set_max_velocity_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_force(self, f)
    }
    /// Fallible variant of motor_set_max_velocity_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_force(self, f)
    }
    /// Returns the selected motor joint's maximum velocity-control torque.
    pub fn motor_max_velocity_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_torque(self)
    }
    /// Fallible variant of motor_max_velocity_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_torque(self)
    }
    /// Sets the selected motor joint's maximum velocity-control torque; the value must be finite and non-negative.
    pub fn motor_set_max_velocity_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_torque(self, t)
    }
    /// Fallible variant of motor_set_max_velocity_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_torque(self, t)
    }
    /// Returns the selected motor joint's linear spring frequency in hertz.
    pub fn motor_linear_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_hertz(self)
    }
    /// Fallible variant of motor_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_hertz(self)
    }
    /// Sets the selected motor joint's linear spring frequency in hertz; the value must be finite and non-negative.
    pub fn motor_set_linear_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_linear_hertz(self, hertz)
    }
    /// Fallible variant of motor_set_linear_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_hertz(self, hertz)
    }
    /// Returns the selected motor joint's linear spring damping ratio.
    pub fn motor_linear_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_damping_ratio(self)
    }
    /// Fallible variant of motor_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_damping_ratio(self)
    }
    /// Sets the selected motor joint's linear spring damping ratio; the value must be finite and non-negative.
    pub fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_linear_damping_ratio(self, damping)
    }
    /// Fallible variant of motor_set_linear_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_damping_ratio(self, damping)
    }
    /// Returns the selected motor joint's angular spring frequency in hertz.
    pub fn motor_angular_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_hertz(self)
    }
    /// Fallible variant of motor_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_hertz(self)
    }
    /// Sets the selected motor joint's angular spring frequency in hertz; the value must be finite and non-negative.
    pub fn motor_set_angular_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_angular_hertz(self, hertz)
    }
    /// Fallible variant of motor_set_angular_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_hertz(self, hertz)
    }
    /// Returns the selected motor joint's angular spring damping ratio.
    pub fn motor_angular_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_damping_ratio(self)
    }
    /// Fallible variant of motor_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_damping_ratio(self)
    }
    /// Sets the selected motor joint's angular spring damping ratio; the value must be finite and non-negative.
    pub fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_angular_damping_ratio(self, damping)
    }
    /// Fallible variant of motor_set_angular_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_damping_ratio(self, damping)
    }
    /// Returns the selected motor joint's maximum spring force.
    pub fn motor_max_spring_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_force(self)
    }
    /// Fallible variant of motor_max_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_force(self)
    }
    /// Sets the selected motor joint's maximum spring force; the value must be finite and non-negative.
    pub fn motor_set_max_spring_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_force(self, f)
    }
    /// Fallible variant of motor_set_max_spring_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_force(self, f)
    }
    /// Returns the selected motor joint's maximum spring torque.
    pub fn motor_max_spring_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_torque(self)
    }
    /// Fallible variant of motor_max_spring_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_torque(self)
    }
    /// Sets the selected motor joint's maximum spring torque; the value must be finite and non-negative.
    pub fn motor_set_max_spring_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_torque(self, t)
    }
    /// Fallible variant of motor_set_max_spring_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_torque(self, t)
    }
}

impl<'w> Joint<'w> {
    /// Returns the selected motor joint's target linear velocity.
    pub fn motor_linear_velocity(&self) -> Vec2 {
        MotorJointRuntimeHandle::motor_linear_velocity(self)
    }
    /// Fallible variant of motor_linear_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        MotorJointRuntimeHandle::try_motor_linear_velocity(self)
    }
    /// Sets the selected motor joint's target linear velocity; both components must be finite.
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        MotorJointRuntimeHandle::motor_set_linear_velocity(self, v)
    }
    /// Fallible variant of motor_set_linear_velocity; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_velocity(self, v)
    }
    /// Returns the selected motor joint's target angular velocity.
    pub fn motor_angular_velocity(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_velocity(self)
    }
    /// Fallible variant of motor_angular_velocity; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_velocity(self)
    }
    /// Sets the selected motor joint's target angular velocity; the value must be finite.
    pub fn motor_set_angular_velocity(&mut self, w: f32) {
        MotorJointRuntimeHandle::motor_set_angular_velocity(self, w)
    }
    /// Fallible variant of motor_set_angular_velocity; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_velocity(self, w)
    }
    /// Returns the selected motor joint's maximum velocity-control force.
    pub fn motor_max_velocity_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_force(self)
    }
    /// Fallible variant of motor_max_velocity_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_force(self)
    }
    /// Sets the selected motor joint's maximum velocity-control force; the value must be finite and non-negative.
    pub fn motor_set_max_velocity_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_force(self, f)
    }
    /// Fallible variant of motor_set_max_velocity_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_force(self, f)
    }
    /// Returns the selected motor joint's maximum velocity-control torque.
    pub fn motor_max_velocity_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_torque(self)
    }
    /// Fallible variant of motor_max_velocity_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_torque(self)
    }
    /// Sets the selected motor joint's maximum velocity-control torque; the value must be finite and non-negative.
    pub fn motor_set_max_velocity_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_torque(self, t)
    }
    /// Fallible variant of motor_set_max_velocity_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_torque(self, t)
    }
    /// Returns the selected motor joint's linear spring frequency in hertz.
    pub fn motor_linear_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_hertz(self)
    }
    /// Fallible variant of motor_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_hertz(self)
    }
    /// Sets the selected motor joint's linear spring frequency in hertz; the value must be finite and non-negative.
    pub fn motor_set_linear_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_linear_hertz(self, hertz)
    }
    /// Fallible variant of motor_set_linear_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_hertz(self, hertz)
    }
    /// Returns the selected motor joint's linear spring damping ratio.
    pub fn motor_linear_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_damping_ratio(self)
    }
    /// Fallible variant of motor_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_damping_ratio(self)
    }
    /// Sets the selected motor joint's linear spring damping ratio; the value must be finite and non-negative.
    pub fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_linear_damping_ratio(self, damping)
    }
    /// Fallible variant of motor_set_linear_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_damping_ratio(self, damping)
    }
    /// Returns the selected motor joint's angular spring frequency in hertz.
    pub fn motor_angular_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_hertz(self)
    }
    /// Fallible variant of motor_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_hertz(self)
    }
    /// Sets the selected motor joint's angular spring frequency in hertz; the value must be finite and non-negative.
    pub fn motor_set_angular_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_angular_hertz(self, hertz)
    }
    /// Fallible variant of motor_set_angular_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_hertz(self, hertz)
    }
    /// Returns the selected motor joint's angular spring damping ratio.
    pub fn motor_angular_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_damping_ratio(self)
    }
    /// Fallible variant of motor_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_damping_ratio(self)
    }
    /// Sets the selected motor joint's angular spring damping ratio; the value must be finite and non-negative.
    pub fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_angular_damping_ratio(self, damping)
    }
    /// Fallible variant of motor_set_angular_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_damping_ratio(self, damping)
    }
    /// Returns the selected motor joint's maximum spring force.
    pub fn motor_max_spring_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_force(self)
    }
    /// Fallible variant of motor_max_spring_force; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_force(self)
    }
    /// Sets the selected motor joint's maximum spring force; the value must be finite and non-negative.
    pub fn motor_set_max_spring_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_force(self, f)
    }
    /// Fallible variant of motor_set_max_spring_force; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_force(self, f)
    }
    /// Returns the selected motor joint's maximum spring torque.
    pub fn motor_max_spring_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_torque(self)
    }
    /// Fallible variant of motor_max_spring_torque; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_torque(self)
    }
    /// Sets the selected motor joint's maximum spring torque; the value must be finite and non-negative.
    pub fn motor_set_max_spring_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_torque(self, t)
    }
    /// Fallible variant of motor_set_max_spring_torque; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_torque(self, t)
    }
}
