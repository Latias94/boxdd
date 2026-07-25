use super::RecordingSession;
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{ApiResult, JointId, JointType};

impl RecordingSession<'_> {
    /// Enable or disable the prismatic spring and record the mutation.
    pub fn prismatic_joint_enable_spring(&mut self, joint: JointId, enable: bool) {
        self.try_prismatic_joint_enable_spring(joint, enable)
            .expect("recording session received an invalid prismatic joint")
    }

    pub fn try_prismatic_joint_enable_spring(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticEnableSpring,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set prismatic spring frequency and record the mutation.
    pub fn prismatic_joint_set_spring_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_prismatic_joint_set_spring_hertz(joint, hertz)
            .expect("recording session received an invalid prismatic-joint spring frequency")
    }

    pub fn try_prismatic_joint_set_spring_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticSetSpringHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set prismatic spring damping and record the mutation.
    pub fn prismatic_joint_set_spring_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_prismatic_joint_set_spring_damping_ratio(joint, ratio)
            .expect("recording session received invalid prismatic-joint spring damping")
    }

    pub fn try_prismatic_joint_set_spring_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticSetSpringDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Set the prismatic spring target translation and record the mutation.
    pub fn prismatic_joint_set_target_translation(&mut self, joint: JointId, translation: f32) {
        self.try_prismatic_joint_set_target_translation(joint, translation)
            .expect("recording session received an invalid prismatic-joint target translation")
    }

    pub fn try_prismatic_joint_set_target_translation(
        &mut self,
        joint: JointId,
        translation: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticSetTargetTranslation,
            JointWriteValue::Scalar(translation),
        )
    }

    /// Enable or disable prismatic limits and record the mutation.
    pub fn prismatic_joint_enable_limit(&mut self, joint: JointId, enable: bool) {
        self.try_prismatic_joint_enable_limit(joint, enable)
            .expect("recording session received an invalid prismatic joint")
    }

    pub fn try_prismatic_joint_enable_limit(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticEnableLimit,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set prismatic translation limits and record the mutation.
    pub fn prismatic_joint_set_limits(&mut self, joint: JointId, lower: f32, upper: f32) {
        self.try_prismatic_joint_set_limits(joint, lower, upper)
            .expect("recording session received invalid prismatic-joint limits")
    }

    pub fn try_prismatic_joint_set_limits(
        &mut self,
        joint: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticSetLimits,
            JointWriteValue::ScalarPair(lower, upper),
        )
    }

    /// Enable or disable the prismatic motor and record the mutation.
    pub fn prismatic_joint_enable_motor(&mut self, joint: JointId, enable: bool) {
        self.try_prismatic_joint_enable_motor(joint, enable)
            .expect("recording session received an invalid prismatic joint")
    }

    pub fn try_prismatic_joint_enable_motor(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticEnableMotor,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set prismatic motor speed and record the mutation.
    pub fn prismatic_joint_set_motor_speed(&mut self, joint: JointId, speed: f32) {
        self.try_prismatic_joint_set_motor_speed(joint, speed)
            .expect("recording session received an invalid prismatic-joint motor speed")
    }

    pub fn try_prismatic_joint_set_motor_speed(
        &mut self,
        joint: JointId,
        speed: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticSetMotorSpeed,
            JointWriteValue::Scalar(speed),
        )
    }

    /// Set maximum prismatic motor force and record the mutation.
    pub fn prismatic_joint_set_max_motor_force(&mut self, joint: JointId, force: f32) {
        self.try_prismatic_joint_set_max_motor_force(joint, force)
            .expect("recording session received an invalid prismatic-joint motor force")
    }

    pub fn try_prismatic_joint_set_max_motor_force(
        &mut self,
        joint: JointId,
        force: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Prismatic),
            JointWriteKind::PrismaticSetMaxMotorForce,
            JointWriteValue::Scalar(force),
        )
    }
}
