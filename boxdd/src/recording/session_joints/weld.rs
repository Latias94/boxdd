use super::RecordingSession;
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{ApiResult, JointId, JointType};

impl RecordingSession<'_> {
    /// Set weld linear spring frequency and record the mutation.
    pub fn weld_joint_set_linear_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_weld_joint_set_linear_hertz(joint, hertz)
            .expect("recording session received an invalid weld-joint linear frequency")
    }

    pub fn try_weld_joint_set_linear_hertz(&mut self, joint: JointId, hertz: f32) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Weld),
            JointWriteKind::WeldSetLinearHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set weld linear damping and record the mutation.
    pub fn weld_joint_set_linear_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_weld_joint_set_linear_damping_ratio(joint, ratio)
            .expect("recording session received invalid weld-joint linear damping")
    }

    pub fn try_weld_joint_set_linear_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Weld),
            JointWriteKind::WeldSetLinearDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }

    /// Set weld angular spring frequency and record the mutation.
    pub fn weld_joint_set_angular_hertz(&mut self, joint: JointId, hertz: f32) {
        self.try_weld_joint_set_angular_hertz(joint, hertz)
            .expect("recording session received an invalid weld-joint angular frequency")
    }

    pub fn try_weld_joint_set_angular_hertz(
        &mut self,
        joint: JointId,
        hertz: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Weld),
            JointWriteKind::WeldSetAngularHertz,
            JointWriteValue::Scalar(hertz),
        )
    }

    /// Set weld angular damping and record the mutation.
    pub fn weld_joint_set_angular_damping_ratio(&mut self, joint: JointId, ratio: f32) {
        self.try_weld_joint_set_angular_damping_ratio(joint, ratio)
            .expect("recording session received invalid weld-joint angular damping")
    }

    pub fn try_weld_joint_set_angular_damping_ratio(
        &mut self,
        joint: JointId,
        ratio: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            Some(JointType::Weld),
            JointWriteKind::WeldSetAngularDampingRatio,
            JointWriteValue::Scalar(ratio),
        )
    }
}
