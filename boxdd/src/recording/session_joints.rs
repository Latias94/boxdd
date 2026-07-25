use super::RecordingSession;
use crate::core::world_core::{WorldAccess, WorldCore};
use crate::joints::{JointWriteKind, JointWriteValue};
use crate::{
    ApiResult, ConstraintTuning, DistanceJointDef, FilterJointDef, JointId, JointType,
    MotorJointDef, PrismaticJointDef, RevoluteJointDef, Transform, WeldJointDef, WheelJointDef,
};

mod distance;
mod motor;
mod prismatic;
mod revolute;
mod weld;
mod wheel;

const RECORDING: WorldAccess = WorldAccess::Recording;

impl RecordingSession<'_> {
    fn try_recording_joint_write(
        &self,
        joint: JointId,
        expected: Option<JointType>,
        kind: JointWriteKind,
        value: JointWriteValue,
    ) -> ApiResult<()> {
        crate::joints::try_joint_write_with_access(
            self.world.core(),
            joint,
            expected,
            kind,
            value,
            RECORDING,
        )
    }

    /// Create a distance joint and record the mutation.
    pub fn create_distance_joint(&mut self, def: &DistanceJointDef) -> JointId {
        self.try_create_distance_joint(def)
            .expect("recording session could not create a distance joint")
    }

    pub fn try_create_distance_joint(&mut self, def: &DistanceJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_distance_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Create a motor joint and record the mutation.
    pub fn create_motor_joint(&mut self, def: &MotorJointDef) -> JointId {
        self.try_create_motor_joint(def)
            .expect("recording session could not create a motor joint")
    }

    pub fn try_create_motor_joint(&mut self, def: &MotorJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_motor_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Create a filter joint and record the mutation.
    pub fn create_filter_joint(&mut self, def: &FilterJointDef) -> JointId {
        self.try_create_filter_joint(def)
            .expect("recording session could not create a filter joint")
    }

    pub fn try_create_filter_joint(&mut self, def: &FilterJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_filter_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Create a prismatic joint and record the mutation.
    pub fn create_prismatic_joint(&mut self, def: &PrismaticJointDef) -> JointId {
        self.try_create_prismatic_joint(def)
            .expect("recording session could not create a prismatic joint")
    }

    pub fn try_create_prismatic_joint(&mut self, def: &PrismaticJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_prismatic_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Create a revolute joint and record the mutation.
    pub fn create_revolute_joint(&mut self, def: &RevoluteJointDef) -> JointId {
        self.try_create_revolute_joint(def)
            .expect("recording session could not create a revolute joint")
    }

    pub fn try_create_revolute_joint(&mut self, def: &RevoluteJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_revolute_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Create a weld joint and record the mutation.
    pub fn create_weld_joint(&mut self, def: &WeldJointDef) -> JointId {
        self.try_create_weld_joint(def)
            .expect("recording session could not create a weld joint")
    }

    pub fn try_create_weld_joint(&mut self, def: &WeldJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_weld_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Create a wheel joint and record the mutation.
    pub fn create_wheel_joint(&mut self, def: &WheelJointDef) -> JointId {
        self.try_create_wheel_joint(def)
            .expect("recording session could not create a wheel joint")
    }

    pub fn try_create_wheel_joint(&mut self, def: &WheelJointDef) -> ApiResult<JointId> {
        crate::joints::try_create_wheel_joint_id_with_access(self.world, def, RECORDING)
    }

    /// Destroy a joint and record the mutation.
    pub fn destroy_joint(&mut self, joint: JointId, wake_bodies: bool) {
        self.try_destroy_joint(joint, wake_bodies)
            .expect("recording session received an invalid JointId")
    }

    pub fn try_destroy_joint(&mut self, joint: JointId, wake_bodies: bool) -> ApiResult<()> {
        WorldCore::destroy_joint_now_with_access(self.world.core(), joint, wake_bodies, RECORDING)
    }

    /// Set a joint's local frame on body A and record the mutation.
    pub fn set_joint_local_frame_a(&mut self, joint: JointId, frame: Transform) {
        self.try_set_joint_local_frame_a(joint, frame)
            .expect("recording session received an invalid joint frame")
    }

    pub fn try_set_joint_local_frame_a(
        &mut self,
        joint: JointId,
        frame: Transform,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointSetLocalFrameA,
            JointWriteValue::Transform(frame),
        )
    }

    /// Set a joint's local frame on body B and record the mutation.
    pub fn set_joint_local_frame_b(&mut self, joint: JointId, frame: Transform) {
        self.try_set_joint_local_frame_b(joint, frame)
            .expect("recording session received an invalid joint frame")
    }

    pub fn try_set_joint_local_frame_b(
        &mut self,
        joint: JointId,
        frame: Transform,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointSetLocalFrameB,
            JointWriteValue::Transform(frame),
        )
    }

    /// Set the connected-body collision flag and record the mutation.
    pub fn set_joint_collide_connected(&mut self, joint: JointId, flag: bool) {
        self.try_set_joint_collide_connected(joint, flag)
            .expect("recording session received an invalid JointId")
    }

    pub fn try_set_joint_collide_connected(&mut self, joint: JointId, flag: bool) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointSetCollideConnected,
            JointWriteValue::Bool(flag),
        )
    }

    /// Wake both connected bodies and record the mutation.
    pub fn joint_wake_bodies(&mut self, joint: JointId) {
        self.try_joint_wake_bodies(joint)
            .expect("recording session received an invalid JointId")
    }

    pub fn try_joint_wake_bodies(&mut self, joint: JointId) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointWakeBodies,
            JointWriteValue::Unit,
        )
    }

    /// Set generic constraint tuning and record the mutation.
    pub fn set_joint_constraint_tuning(&mut self, joint: JointId, tuning: ConstraintTuning) {
        self.try_set_joint_constraint_tuning(joint, tuning)
            .expect("recording session received invalid joint tuning")
    }

    pub fn try_set_joint_constraint_tuning(
        &mut self,
        joint: JointId,
        tuning: ConstraintTuning,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointSetConstraintTuning,
            JointWriteValue::Tuning(tuning),
        )
    }

    /// Set the joint force threshold and record the mutation.
    pub fn set_joint_force_threshold(&mut self, joint: JointId, threshold: f32) {
        self.try_set_joint_force_threshold(joint, threshold)
            .expect("recording session received an invalid joint force threshold")
    }

    pub fn try_set_joint_force_threshold(
        &mut self,
        joint: JointId,
        threshold: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointSetForceThreshold,
            JointWriteValue::Scalar(threshold),
        )
    }

    /// Set the joint torque threshold and record the mutation.
    pub fn set_joint_torque_threshold(&mut self, joint: JointId, threshold: f32) {
        self.try_set_joint_torque_threshold(joint, threshold)
            .expect("recording session received an invalid joint torque threshold")
    }

    pub fn try_set_joint_torque_threshold(
        &mut self,
        joint: JointId,
        threshold: f32,
    ) -> ApiResult<()> {
        self.try_recording_joint_write(
            joint,
            None,
            JointWriteKind::JointSetTorqueThreshold,
            JointWriteValue::Scalar(threshold),
        )
    }
}
