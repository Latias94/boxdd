use boxdd_sys::ffi;

use super::validation::{
    check_joint_finite, check_joint_non_negative, check_joint_non_negative_range,
    check_joint_ordered_range, check_joint_positive, check_joint_transform, check_joint_tuning,
    check_joint_vec2, check_revolute_joint_range,
};
use super::{ConstraintTuning, raw_joint_id};
use crate::Transform;
use crate::error::Result;
use crate::types::{JointId, Vec2};

pub(in crate::joints) enum JointWrite {
    SetCollideConnected(bool),
    SetConstraintTuning(ConstraintTuning),
    SetLocalFrameA(Transform),
    SetLocalFrameB(Transform),
    WakeBodies,
    SetForceThreshold(f32),
    SetTorqueThreshold(f32),
    DistanceSetLength(f32),
    DistanceEnableSpring(bool),
    DistanceSetSpringForceRange(f32, f32),
    DistanceSetSpringHertz(f32),
    DistanceSetSpringDampingRatio(f32),
    DistanceEnableLimit(bool),
    DistanceSetLengthRange(f32, f32),
    DistanceEnableMotor(bool),
    DistanceSetMotorSpeed(f32),
    DistanceSetMaxMotorForce(f32),
    MotorSetLinearVelocity(Vec2),
    MotorSetAngularVelocity(f32),
    MotorSetMaxVelocityForce(f32),
    MotorSetMaxVelocityTorque(f32),
    MotorSetLinearHertz(f32),
    MotorSetLinearDampingRatio(f32),
    MotorSetAngularHertz(f32),
    MotorSetAngularDampingRatio(f32),
    MotorSetMaxSpringForce(f32),
    MotorSetMaxSpringTorque(f32),
    PrismaticEnableSpring(bool),
    PrismaticSetSpringHertz(f32),
    PrismaticSetSpringDampingRatio(f32),
    PrismaticSetTargetTranslation(f32),
    PrismaticEnableLimit(bool),
    PrismaticSetLimits(f32, f32),
    PrismaticEnableMotor(bool),
    PrismaticSetMotorSpeed(f32),
    PrismaticSetMaxMotorForce(f32),
    RevoluteEnableSpring(bool),
    RevoluteSetSpringHertz(f32),
    RevoluteSetSpringDampingRatio(f32),
    RevoluteSetTargetAngle(f32),
    RevoluteEnableLimit(bool),
    RevoluteSetLimits(f32, f32),
    RevoluteEnableMotor(bool),
    RevoluteSetMotorSpeed(f32),
    RevoluteSetMaxMotorTorque(f32),
    WeldSetLinearHertz(f32),
    WeldSetLinearDampingRatio(f32),
    WeldSetAngularHertz(f32),
    WeldSetAngularDampingRatio(f32),
    WheelEnableSpring(bool),
    WheelSetSpringHertz(f32),
    WheelSetSpringDampingRatio(f32),
    WheelEnableLimit(bool),
    WheelSetLimits(f32, f32),
    WheelEnableMotor(bool),
    WheelSetMotorSpeed(f32),
    WheelSetMaxMotorTorque(f32),
}

impl JointWrite {
    pub(in crate::joints) fn apply(self, id: JointId) -> Result<()> {
        match self {
            Self::SetCollideConnected(value) => unsafe {
                ffi::b2Joint_SetCollideConnected(raw_joint_id(id), value)
            },
            Self::SetConstraintTuning(value) => {
                check_joint_tuning(value, "Joint::set_constraint_tuning", "tuning")?;
                unsafe {
                    ffi::b2Joint_SetConstraintTuning(
                        raw_joint_id(id),
                        value.hertz(),
                        value.damping_ratio(),
                    )
                }
            }
            Self::SetLocalFrameA(value) => {
                check_joint_transform(value, "Joint::set_local_frame_a", "frame")?;
                unsafe { ffi::b2Joint_SetLocalFrameA(raw_joint_id(id), value.into_raw()) }
            }
            Self::SetLocalFrameB(value) => {
                check_joint_transform(value, "Joint::set_local_frame_b", "frame")?;
                unsafe { ffi::b2Joint_SetLocalFrameB(raw_joint_id(id), value.into_raw()) }
            }
            Self::WakeBodies => unsafe { ffi::b2Joint_WakeBodies(raw_joint_id(id)) },
            Self::SetForceThreshold(value) => {
                check_joint_non_negative(value, "Joint::set_force_threshold", "threshold")?;
                unsafe { ffi::b2Joint_SetForceThreshold(raw_joint_id(id), value) }
            }
            Self::SetTorqueThreshold(value) => {
                check_joint_non_negative(value, "Joint::set_torque_threshold", "threshold")?;
                unsafe { ffi::b2Joint_SetTorqueThreshold(raw_joint_id(id), value) }
            }
            Self::DistanceSetLength(value) => {
                check_joint_positive(value, "DistanceJoint::set_length", "length")?;
                unsafe { ffi::b2DistanceJoint_SetLength(raw_joint_id(id), value) }
            }
            Self::DistanceEnableSpring(value) => unsafe {
                ffi::b2DistanceJoint_EnableSpring(raw_joint_id(id), value)
            },
            Self::DistanceSetSpringForceRange(lower, upper) => {
                check_joint_ordered_range(
                    lower,
                    upper,
                    "DistanceJoint::set_spring_force_range",
                    "lower/upper",
                )?;
                unsafe { ffi::b2DistanceJoint_SetSpringForceRange(raw_joint_id(id), lower, upper) }
            }
            Self::DistanceSetSpringHertz(value) => {
                check_joint_non_negative(value, "DistanceJoint::set_spring_hertz", "hertz")?;
                unsafe { ffi::b2DistanceJoint_SetSpringHertz(raw_joint_id(id), value) }
            }
            Self::DistanceSetSpringDampingRatio(value) => {
                check_joint_non_negative(
                    value,
                    "DistanceJoint::set_spring_damping_ratio",
                    "ratio",
                )?;
                unsafe { ffi::b2DistanceJoint_SetSpringDampingRatio(raw_joint_id(id), value) }
            }
            Self::DistanceEnableLimit(value) => unsafe {
                ffi::b2DistanceJoint_EnableLimit(raw_joint_id(id), value)
            },
            Self::DistanceSetLengthRange(lower, upper) => {
                check_joint_non_negative_range(
                    lower,
                    upper,
                    "DistanceJoint::set_length_range",
                    "min/max",
                )?;
                unsafe { ffi::b2DistanceJoint_SetLengthRange(raw_joint_id(id), lower, upper) }
            }
            Self::DistanceEnableMotor(value) => unsafe {
                ffi::b2DistanceJoint_EnableMotor(raw_joint_id(id), value)
            },
            Self::DistanceSetMotorSpeed(value) => {
                check_joint_finite(value, "DistanceJoint::set_motor_speed", "speed")?;
                unsafe { ffi::b2DistanceJoint_SetMotorSpeed(raw_joint_id(id), value) }
            }
            Self::DistanceSetMaxMotorForce(value) => {
                check_joint_non_negative(value, "DistanceJoint::set_max_motor_force", "force")?;
                unsafe { ffi::b2DistanceJoint_SetMaxMotorForce(raw_joint_id(id), value) }
            }
            Self::MotorSetLinearVelocity(value) => {
                check_joint_vec2(value, "MotorJoint::set_linear_velocity", "velocity")?;
                unsafe { ffi::b2MotorJoint_SetLinearVelocity(raw_joint_id(id), value.into_raw()) }
            }
            Self::MotorSetAngularVelocity(value) => {
                check_joint_finite(value, "MotorJoint::set_angular_velocity", "velocity")?;
                unsafe { ffi::b2MotorJoint_SetAngularVelocity(raw_joint_id(id), value) }
            }
            Self::MotorSetMaxVelocityForce(value) => {
                check_joint_non_negative(value, "MotorJoint::set_max_velocity_force", "force")?;
                unsafe { ffi::b2MotorJoint_SetMaxVelocityForce(raw_joint_id(id), value) }
            }
            Self::MotorSetMaxVelocityTorque(value) => {
                check_joint_non_negative(value, "MotorJoint::set_max_velocity_torque", "torque")?;
                unsafe { ffi::b2MotorJoint_SetMaxVelocityTorque(raw_joint_id(id), value) }
            }
            Self::MotorSetLinearHertz(value) => {
                check_joint_non_negative(value, "MotorJoint::set_linear_hertz", "hertz")?;
                unsafe { ffi::b2MotorJoint_SetLinearHertz(raw_joint_id(id), value) }
            }
            Self::MotorSetLinearDampingRatio(value) => {
                check_joint_non_negative(value, "MotorJoint::set_linear_damping_ratio", "ratio")?;
                unsafe { ffi::b2MotorJoint_SetLinearDampingRatio(raw_joint_id(id), value) }
            }
            Self::MotorSetAngularHertz(value) => {
                check_joint_non_negative(value, "MotorJoint::set_angular_hertz", "hertz")?;
                unsafe { ffi::b2MotorJoint_SetAngularHertz(raw_joint_id(id), value) }
            }
            Self::MotorSetAngularDampingRatio(value) => {
                check_joint_non_negative(value, "MotorJoint::set_angular_damping_ratio", "ratio")?;
                unsafe { ffi::b2MotorJoint_SetAngularDampingRatio(raw_joint_id(id), value) }
            }
            Self::MotorSetMaxSpringForce(value) => {
                check_joint_non_negative(value, "MotorJoint::set_max_spring_force", "force")?;
                unsafe { ffi::b2MotorJoint_SetMaxSpringForce(raw_joint_id(id), value) }
            }
            Self::MotorSetMaxSpringTorque(value) => {
                check_joint_non_negative(value, "MotorJoint::set_max_spring_torque", "torque")?;
                unsafe { ffi::b2MotorJoint_SetMaxSpringTorque(raw_joint_id(id), value) }
            }
            Self::PrismaticEnableSpring(value) => unsafe {
                ffi::b2PrismaticJoint_EnableSpring(raw_joint_id(id), value)
            },
            Self::PrismaticSetSpringHertz(value) => {
                check_joint_non_negative(value, "PrismaticJoint::set_spring_hertz", "hertz")?;
                unsafe { ffi::b2PrismaticJoint_SetSpringHertz(raw_joint_id(id), value) }
            }
            Self::PrismaticSetSpringDampingRatio(value) => {
                check_joint_non_negative(
                    value,
                    "PrismaticJoint::set_spring_damping_ratio",
                    "ratio",
                )?;
                unsafe { ffi::b2PrismaticJoint_SetSpringDampingRatio(raw_joint_id(id), value) }
            }
            Self::PrismaticSetTargetTranslation(value) => {
                check_joint_finite(
                    value,
                    "PrismaticJoint::set_target_translation",
                    "translation",
                )?;
                unsafe { ffi::b2PrismaticJoint_SetTargetTranslation(raw_joint_id(id), value) }
            }
            Self::PrismaticEnableLimit(value) => unsafe {
                ffi::b2PrismaticJoint_EnableLimit(raw_joint_id(id), value)
            },
            Self::PrismaticSetLimits(lower, upper) => {
                check_joint_ordered_range(
                    lower,
                    upper,
                    "PrismaticJoint::set_limits",
                    "lower/upper",
                )?;
                unsafe { ffi::b2PrismaticJoint_SetLimits(raw_joint_id(id), lower, upper) }
            }
            Self::PrismaticEnableMotor(value) => unsafe {
                ffi::b2PrismaticJoint_EnableMotor(raw_joint_id(id), value)
            },
            Self::PrismaticSetMotorSpeed(value) => {
                check_joint_finite(value, "PrismaticJoint::set_motor_speed", "speed")?;
                unsafe { ffi::b2PrismaticJoint_SetMotorSpeed(raw_joint_id(id), value) }
            }
            Self::PrismaticSetMaxMotorForce(value) => {
                check_joint_non_negative(value, "PrismaticJoint::set_max_motor_force", "force")?;
                unsafe { ffi::b2PrismaticJoint_SetMaxMotorForce(raw_joint_id(id), value) }
            }
            Self::RevoluteEnableSpring(value) => unsafe {
                ffi::b2RevoluteJoint_EnableSpring(raw_joint_id(id), value)
            },
            Self::RevoluteSetSpringHertz(value) => {
                check_joint_non_negative(value, "RevoluteJoint::set_spring_hertz", "hertz")?;
                unsafe { ffi::b2RevoluteJoint_SetSpringHertz(raw_joint_id(id), value) }
            }
            Self::RevoluteSetSpringDampingRatio(value) => {
                check_joint_non_negative(
                    value,
                    "RevoluteJoint::set_spring_damping_ratio",
                    "ratio",
                )?;
                unsafe { ffi::b2RevoluteJoint_SetSpringDampingRatio(raw_joint_id(id), value) }
            }
            Self::RevoluteSetTargetAngle(value) => {
                check_joint_finite(value, "RevoluteJoint::set_target_angle", "angle")?;
                unsafe { ffi::b2RevoluteJoint_SetTargetAngle(raw_joint_id(id), value) }
            }
            Self::RevoluteEnableLimit(value) => unsafe {
                ffi::b2RevoluteJoint_EnableLimit(raw_joint_id(id), value)
            },
            Self::RevoluteSetLimits(lower, upper) => {
                check_revolute_joint_range(
                    lower,
                    upper,
                    "RevoluteJoint::set_limits",
                    "lower/upper",
                )?;
                unsafe { ffi::b2RevoluteJoint_SetLimits(raw_joint_id(id), lower, upper) }
            }
            Self::RevoluteEnableMotor(value) => unsafe {
                ffi::b2RevoluteJoint_EnableMotor(raw_joint_id(id), value)
            },
            Self::RevoluteSetMotorSpeed(value) => {
                check_joint_finite(value, "RevoluteJoint::set_motor_speed", "speed")?;
                unsafe { ffi::b2RevoluteJoint_SetMotorSpeed(raw_joint_id(id), value) }
            }
            Self::RevoluteSetMaxMotorTorque(value) => {
                check_joint_non_negative(value, "RevoluteJoint::set_max_motor_torque", "torque")?;
                unsafe { ffi::b2RevoluteJoint_SetMaxMotorTorque(raw_joint_id(id), value) }
            }
            Self::WeldSetLinearHertz(value) => {
                check_joint_non_negative(value, "WeldJoint::set_linear_hertz", "hertz")?;
                unsafe { ffi::b2WeldJoint_SetLinearHertz(raw_joint_id(id), value) }
            }
            Self::WeldSetLinearDampingRatio(value) => {
                check_joint_non_negative(value, "WeldJoint::set_linear_damping_ratio", "ratio")?;
                unsafe { ffi::b2WeldJoint_SetLinearDampingRatio(raw_joint_id(id), value) }
            }
            Self::WeldSetAngularHertz(value) => {
                check_joint_non_negative(value, "WeldJoint::set_angular_hertz", "hertz")?;
                unsafe { ffi::b2WeldJoint_SetAngularHertz(raw_joint_id(id), value) }
            }
            Self::WeldSetAngularDampingRatio(value) => {
                check_joint_non_negative(value, "WeldJoint::set_angular_damping_ratio", "ratio")?;
                unsafe { ffi::b2WeldJoint_SetAngularDampingRatio(raw_joint_id(id), value) }
            }
            Self::WheelEnableSpring(value) => unsafe {
                ffi::b2WheelJoint_EnableSpring(raw_joint_id(id), value)
            },
            Self::WheelSetSpringHertz(value) => {
                check_joint_non_negative(value, "WheelJoint::set_spring_hertz", "hertz")?;
                unsafe { ffi::b2WheelJoint_SetSpringHertz(raw_joint_id(id), value) }
            }
            Self::WheelSetSpringDampingRatio(value) => {
                check_joint_non_negative(value, "WheelJoint::set_spring_damping_ratio", "ratio")?;
                unsafe { ffi::b2WheelJoint_SetSpringDampingRatio(raw_joint_id(id), value) }
            }
            Self::WheelEnableLimit(value) => unsafe {
                ffi::b2WheelJoint_EnableLimit(raw_joint_id(id), value)
            },
            Self::WheelSetLimits(lower, upper) => {
                check_joint_ordered_range(lower, upper, "WheelJoint::set_limits", "lower/upper")?;
                unsafe { ffi::b2WheelJoint_SetLimits(raw_joint_id(id), lower, upper) }
            }
            Self::WheelEnableMotor(value) => unsafe {
                ffi::b2WheelJoint_EnableMotor(raw_joint_id(id), value)
            },
            Self::WheelSetMotorSpeed(value) => {
                check_joint_finite(value, "WheelJoint::set_motor_speed", "speed")?;
                unsafe { ffi::b2WheelJoint_SetMotorSpeed(raw_joint_id(id), value) }
            }
            Self::WheelSetMaxMotorTorque(value) => {
                check_joint_non_negative(value, "WheelJoint::set_max_motor_torque", "torque")?;
                unsafe { ffi::b2WheelJoint_SetMaxMotorTorque(raw_joint_id(id), value) }
            }
        }
        Ok(())
    }
}
