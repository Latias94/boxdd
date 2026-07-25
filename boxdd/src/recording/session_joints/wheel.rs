use super::RecordingSession;
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{ApiResult, JointId, JointType};

impl RecordingSession<'_> {
    /// Enable or disable the wheel spring and record the mutation.
    pub fn wheel_joint_enable_spring(&mut self, joint: JointId, enable: bool) {
        self.try_wheel_joint_enable_spring(joint, enable)
            .expect("recording session received an invalid wheel joint")
    }

    pub fn try_wheel_joint_enable_spring(&mut self, joint: JointId, enable: bool) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelEnableSpring,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set wheel spring frequency and record the mutation.
    pub fn wheel_joint_set_spring_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_wheel_joint_set_spring_hertz(joint, hertz)
            .expect("recording session received an invalid wheel-joint spring frequency")
    }

    pub fn try_wheel_joint_set_spring_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelSetSpringHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set wheel spring damping and record the mutation.
    pub fn wheel_joint_set_spring_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_wheel_joint_set_spring_damping_ratio(joint, ratio)
            .expect("recording session received invalid wheel-joint spring damping")
    }

    pub fn try_wheel_joint_set_spring_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelSetSpringDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Enable or disable wheel limits and record the mutation.
    pub fn wheel_joint_enable_limit(&mut self, joint: JointId, enable: bool) {
        self.try_wheel_joint_enable_limit(joint, enable)
            .expect("recording session received an invalid wheel joint")
    }

    pub fn try_wheel_joint_enable_limit(&mut self, joint: JointId, enable: bool) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelEnableLimit,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set wheel translation limits and record the mutation.
    pub fn wheel_joint_set_limits(&mut self, joint: JointId, lower: f32, upper: f32) {
        self.try_wheel_joint_set_limits(joint, lower, upper)
            .expect("recording session received invalid wheel-joint limits")
    }

    pub fn try_wheel_joint_set_limits(
        &mut self,
        joint: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelSetLimits,
            JointWriteValue::ScalarPair(lower, upper),
        )
    }

    /// Enable or disable the wheel motor and record the mutation.
    pub fn wheel_joint_enable_motor(&mut self, joint: JointId, enable: bool) {
        self.try_wheel_joint_enable_motor(joint, enable)
            .expect("recording session received an invalid wheel joint")
    }

    pub fn try_wheel_joint_enable_motor(&mut self, joint: JointId, enable: bool) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelEnableMotor,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set wheel motor speed and record the mutation.
    pub fn wheel_joint_set_motor_speed(&mut self, joint: JointId, speed: f32) {
        self.try_wheel_joint_set_motor_speed(joint, speed)
            .expect("recording session received an invalid wheel-joint motor speed")
    }

    pub fn try_wheel_joint_set_motor_speed(&mut self, joint: JointId, speed: f32) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelSetMotorSpeed,
            JointWriteValue::Scalar(speed),
        )
    }

    /// Set maximum wheel motor torque and record the mutation.
    pub fn wheel_joint_set_max_motor_torque(&mut self, joint: JointId, torque: f32) {
        self.try_wheel_joint_set_max_motor_torque(joint, torque)
            .expect("recording session received an invalid wheel-joint motor torque")
    }

    pub fn try_wheel_joint_set_max_motor_torque(
        &mut self,
        joint: JointId,
        torque: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Wheel),
            JointWriteKind::WheelSetMaxMotorTorque,
            JointWriteValue::Scalar(torque),
        )
    }
}
