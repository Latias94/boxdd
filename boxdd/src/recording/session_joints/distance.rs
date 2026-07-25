use super::RecordingSession;
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{ApiResult, JointId, JointType};

impl RecordingSession<'_> {
    /// Set a distance joint's rest length and record the mutation.
    pub fn distance_joint_set_length(&mut self, joint: JointId, length: f32) {
        self.try_distance_joint_set_length(joint, length)
            .expect("recording session received an invalid distance-joint length")
    }

    pub fn try_distance_joint_set_length(&mut self, joint: JointId, length: f32) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetLength,
            JointWriteValue::Scalar(length),
        )
    }

    /// Enable or disable the distance spring and record the mutation.
    pub fn distance_joint_enable_spring(&mut self, joint: JointId, enable: bool) {
        self.try_distance_joint_enable_spring(joint, enable)
            .expect("recording session received an invalid distance joint")
    }

    pub fn try_distance_joint_enable_spring(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceEnableSpring,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set the distance spring force range and record the mutation.
    pub fn distance_joint_set_spring_force_range(
        &mut self,
        joint: JointId,
        lower: f32,
        upper: f32,
    ) {
        self.try_distance_joint_set_spring_force_range(joint, lower, upper)
            .expect("recording session received an invalid distance-joint spring force range")
    }

    pub fn try_distance_joint_set_spring_force_range(
        &mut self,
        joint: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetSpringForceRange,
            JointWriteValue::ScalarPair(lower, upper),
        )
    }

    /// Set distance spring frequency and record the mutation.
    pub fn distance_joint_set_spring_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_distance_joint_set_spring_hertz(joint, hertz)
            .expect("recording session received an invalid distance-joint spring frequency")
    }

    pub fn try_distance_joint_set_spring_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetSpringHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set distance spring damping and record the mutation.
    pub fn distance_joint_set_spring_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_distance_joint_set_spring_damping_ratio(joint, ratio)
            .expect("recording session received invalid distance-joint spring damping")
    }

    pub fn try_distance_joint_set_spring_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetSpringDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Enable or disable the distance limit and record the mutation.
    pub fn distance_joint_enable_limit(&mut self, joint: JointId, enable: bool) {
        self.try_distance_joint_enable_limit(joint, enable)
            .expect("recording session received an invalid distance joint")
    }

    pub fn try_distance_joint_enable_limit(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceEnableLimit,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set the distance limit range and record the mutation.
    pub fn distance_joint_set_length_range(&mut self, joint: JointId, minimum: f32, maximum: f32) {
        self.try_distance_joint_set_length_range(joint, minimum, maximum)
            .expect("recording session received an invalid distance-joint length range")
    }

    pub fn try_distance_joint_set_length_range(
        &mut self,
        joint: JointId,
        minimum: f32,
        maximum: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetLengthRange,
            JointWriteValue::ScalarPair(minimum, maximum),
        )
    }

    /// Enable or disable the distance motor and record the mutation.
    pub fn distance_joint_enable_motor(&mut self, joint: JointId, enable: bool) {
        self.try_distance_joint_enable_motor(joint, enable)
            .expect("recording session received an invalid distance joint")
    }

    pub fn try_distance_joint_enable_motor(
        &mut self,
        joint: JointId,
        enable: bool,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceEnableMotor,
            JointWriteValue::Bool(enable),
        )
    }

    /// Set distance motor speed and record the mutation.
    pub fn distance_joint_set_motor_speed(&mut self, joint: JointId, speed: f32) {
        self.try_distance_joint_set_motor_speed(joint, speed)
            .expect("recording session received an invalid distance-joint motor speed")
    }

    pub fn try_distance_joint_set_motor_speed(
        &mut self,
        joint: JointId,
        speed: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetMotorSpeed,
            JointWriteValue::Scalar(speed),
        )
    }

    /// Set maximum distance motor force and record the mutation.
    pub fn distance_joint_set_max_motor_force(&mut self, joint: JointId, force: f32) {
        self.try_distance_joint_set_max_motor_force(joint, force)
            .expect("recording session received an invalid distance-joint motor force")
    }

    pub fn try_distance_joint_set_max_motor_force(
        &mut self,
        joint: JointId,
        force: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Distance),
            JointWriteKind::DistanceSetMaxMotorForce,
            JointWriteValue::Scalar(force),
        )
    }
}
