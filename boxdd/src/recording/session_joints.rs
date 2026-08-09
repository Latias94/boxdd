use super::RecordingSession;
use crate::{
    DistanceJointDef, FilterJointDef, JointId, MotorJointDef, PrismaticJointDef, Result,
    RevoluteJointDef, WeldJointDef, WheelJointDef,
};

impl RecordingSession<'_> {
    /// Create a distance joint and record the mutation.
    pub fn create_distance_joint(&mut self, def: &DistanceJointDef) -> Result<JointId> {
        crate::joints::create_distance_joint_id(self, def)
    }

    /// Create a motor joint and record the mutation.
    pub fn create_motor_joint(&mut self, def: &MotorJointDef) -> Result<JointId> {
        crate::joints::create_motor_joint_id(self, def)
    }

    /// Create a filter joint and record the mutation.
    pub fn create_filter_joint(&mut self, def: &FilterJointDef) -> Result<JointId> {
        crate::joints::create_filter_joint_id(self, def)
    }

    /// Create a prismatic joint and record the mutation.
    pub fn create_prismatic_joint(&mut self, def: &PrismaticJointDef) -> Result<JointId> {
        crate::joints::create_prismatic_joint_id(self, def)
    }

    /// Create a revolute joint and record the mutation.
    pub fn create_revolute_joint(&mut self, def: &RevoluteJointDef) -> Result<JointId> {
        crate::joints::create_revolute_joint_id(self, def)
    }

    /// Create a weld joint and record the mutation.
    pub fn create_weld_joint(&mut self, def: &WeldJointDef) -> Result<JointId> {
        crate::joints::create_weld_joint_id(self, def)
    }

    /// Create a wheel joint and record the mutation.
    pub fn create_wheel_joint(&mut self, def: &WheelJointDef) -> Result<JointId> {
        crate::joints::create_wheel_joint_id(self, def)
    }
}
