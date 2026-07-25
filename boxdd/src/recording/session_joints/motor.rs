use super::RecordingSession;
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{ApiResult, JointId, JointType, Vec2};

impl RecordingSession<'_> {
    /// Set a motor joint's target linear velocity and record the mutation.
    pub fn motor_joint_set_linear_velocity<V: Into<Vec2>>(&mut self, joint: JointId, velocity: V) {
        self.try_motor_joint_set_linear_velocity(joint, velocity)
            .expect("recording session received an invalid motor-joint linear velocity")
    }

    pub fn try_motor_joint_set_linear_velocity<V: Into<Vec2>>(
        &mut self,
        joint: JointId,
        velocity: V,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetLinearVelocity,
            JointWriteValue::Vector(velocity.into()),
        )
    }

    /// Set a motor joint's target angular velocity and record the mutation.
    pub fn motor_joint_set_angular_velocity(&mut self, joint: JointId, velocity: f32) {
        self.try_motor_joint_set_angular_velocity(joint, velocity)
            .expect("recording session received an invalid motor-joint angular velocity")
    }

    pub fn try_motor_joint_set_angular_velocity(
        &mut self,
        joint: JointId,
        velocity: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetAngularVelocity,
            JointWriteValue::Scalar(velocity),
        )
    }

    /// Set maximum motor-joint velocity force and record the mutation.
    pub fn motor_joint_set_max_velocity_force(&mut self, joint: JointId, force: f32) {
        self.try_motor_joint_set_max_velocity_force(joint, force)
            .expect("recording session received an invalid motor-joint velocity force")
    }

    pub fn try_motor_joint_set_max_velocity_force(
        &mut self,
        joint: JointId,
        force: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetMaxVelocityForce,
            JointWriteValue::Scalar(force),
        )
    }

    /// Set maximum motor-joint velocity torque and record the mutation.
    pub fn motor_joint_set_max_velocity_torque(&mut self, joint: JointId, torque: f32) {
        self.try_motor_joint_set_max_velocity_torque(joint, torque)
            .expect("recording session received an invalid motor-joint velocity torque")
    }

    pub fn try_motor_joint_set_max_velocity_torque(
        &mut self,
        joint: JointId,
        torque: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetMaxVelocityTorque,
            JointWriteValue::Scalar(torque),
        )
    }

    /// Set motor-joint linear spring frequency and record the mutation.
    pub fn motor_joint_set_linear_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_motor_joint_set_linear_hertz(joint, hertz)
            .expect("recording session received an invalid motor-joint linear frequency")
    }

    pub fn try_motor_joint_set_linear_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetLinearHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set motor-joint linear damping and record the mutation.
    pub fn motor_joint_set_linear_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_motor_joint_set_linear_damping_ratio(joint, ratio)
            .expect("recording session received invalid motor-joint linear damping")
    }

    pub fn try_motor_joint_set_linear_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetLinearDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Set motor-joint angular spring frequency and record the mutation.
    pub fn motor_joint_set_angular_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_motor_joint_set_angular_hertz(joint, hertz)
            .expect("recording session received an invalid motor-joint angular frequency")
    }

    pub fn try_motor_joint_set_angular_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetAngularHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set motor-joint angular damping and record the mutation.
    pub fn motor_joint_set_angular_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_motor_joint_set_angular_damping_ratio(joint, ratio)
            .expect("recording session received invalid motor-joint angular damping")
    }

    pub fn try_motor_joint_set_angular_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetAngularDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Set maximum motor-joint spring force and record the mutation.
    pub fn motor_joint_set_max_spring_force(&mut self, joint: JointId, force: f32) {
        self.try_motor_joint_set_max_spring_force(joint, force)
            .expect("recording session received an invalid motor-joint spring force")
    }

    pub fn try_motor_joint_set_max_spring_force(
        &mut self,
        joint: JointId,
        force: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetMaxSpringForce,
            JointWriteValue::Scalar(force),
        )
    }

    /// Set maximum motor-joint spring torque and record the mutation.
    pub fn motor_joint_set_max_spring_torque(&mut self, joint: JointId, torque: f32) {
        self.try_motor_joint_set_max_spring_torque(joint, torque)
            .expect("recording session received an invalid motor-joint spring torque")
    }

    pub fn try_motor_joint_set_max_spring_torque(
        &mut self,
        joint: JointId,
        torque: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Motor),
            JointWriteKind::MotorSetMaxSpringTorque,
            JointWriteValue::Scalar(torque),
        )
    }
}
