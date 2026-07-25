use super::RecordingSession;
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{ApiResult, JointId, JointType};

impl RecordingSession<'_> {
    /// Enable or disable the revolute spring and record the mutation.
    pub fn revolute_joint_enable_spring(&mut self, joint: JointId, enable: bool) {
        self.try_revolute_joint_enable_spring(joint, enable)
            .expect("recording session received an invalid revolute joint")
    }

    pub fn try_revolute_joint_enable_spring(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteEnableSpring,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set revolute spring frequency and record the mutation.
    pub fn revolute_joint_set_spring_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_revolute_joint_set_spring_hertz(joint, hertz)
            .expect("recording session received an invalid revolute-joint spring frequency")
    }

    pub fn try_revolute_joint_set_spring_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteSetSpringHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set revolute spring damping and record the mutation.
    pub fn revolute_joint_set_spring_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_revolute_joint_set_spring_damping_ratio(joint, ratio)
            .expect("recording session received invalid revolute-joint spring damping")
    }

    pub fn try_revolute_joint_set_spring_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteSetSpringDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Set the revolute target angle and record the mutation.
    pub fn revolute_joint_set_target_angle(&mut self, joint: JointId, angle: f32) {
        self.try_revolute_joint_set_target_angle(joint, angle)
            .expect("recording session received an invalid revolute-joint target angle")
    }

    pub fn try_revolute_joint_set_target_angle(
        &mut self,
        joint: JointId,
        angle: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteSetTargetAngle,
            JointWriteValue::Scalar(angle),
        )
    }

    /// Enable or disable revolute limits and record the mutation.
    pub fn revolute_joint_enable_limit(&mut self, joint: JointId, enable: bool) {
        self.try_revolute_joint_enable_limit(joint, enable)
            .expect("recording session received an invalid revolute joint")
    }

    pub fn try_revolute_joint_enable_limit(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteEnableLimit,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set revolute angular limits and record the mutation.
    pub fn revolute_joint_set_limits(&mut self, joint: JointId, lower: f32, upper: f32) {
        self.try_revolute_joint_set_limits(joint, lower, upper)
            .expect("recording session received invalid revolute-joint limits")
    }

    pub fn try_revolute_joint_set_limits(
        &mut self,
        joint: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteSetLimits,
            JointWriteValue::ScalarPair(lower, upper),
        )
    }

    /// Enable or disable the revolute motor and record the mutation.
    pub fn revolute_joint_enable_motor(&mut self, joint: JointId, enable: bool) {
        self.try_revolute_joint_enable_motor(joint, enable)
            .expect("recording session received an invalid revolute joint")
    }

    pub fn try_revolute_joint_enable_motor(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteEnableMotor,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set revolute motor speed and record the mutation.
    pub fn revolute_joint_set_motor_speed(&mut self, joint: JointId, speed: f32) {
        self.try_revolute_joint_set_motor_speed(joint, speed)
            .expect("recording session received an invalid revolute-joint motor speed")
    }

    pub fn try_revolute_joint_set_motor_speed(
        &mut self,
        joint: JointId,
        speed: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteSetMotorSpeed,
            JointWriteValue::Scalar(speed),
        )
    }

    /// Set maximum revolute motor torque and record the mutation.
    pub fn revolute_joint_set_max_motor_torque(&mut self, joint: JointId, torque: f32) {
        self.try_revolute_joint_set_max_motor_torque(joint, torque)
            .expect("recording session received an invalid revolute-joint motor torque")
    }

    pub fn try_revolute_joint_set_max_motor_torque(
        &mut self,
        joint: JointId,
        torque: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Revolute),
            JointWriteKind::RevoluteSetMaxMotorTorque,
            JointWriteValue::Scalar(torque),
        )
    }
}
