use core::ops::{Deref, DerefMut};

use boxdd_sys::ffi;

use super::runtime::JointWrite;
use super::validation::{
    check_native_joint_finite, check_native_joint_non_negative,
    check_native_joint_non_negative_range, check_native_joint_ordered_range,
    check_native_joint_positive, check_native_joint_vec2, check_native_revolute_joint_range,
};
use super::{Joint, JointId, JointType, raw_joint_id};
use crate::error::{Error, Result};
use crate::types::Vec2;

trait TypedJointCapability {
    fn joint(&self) -> &Joint<'_>;

    fn read<T>(&self, read: impl FnOnce(JointId) -> T) -> Result<T> {
        let joint = self.joint();
        joint.proof.call(|call| Ok(read(call.id())))
    }

    fn read_finite(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> f32,
    ) -> Result<f32> {
        check_native_joint_finite(self.read(read)?, operation, output)
    }

    fn read_non_negative(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> f32,
    ) -> Result<f32> {
        check_native_joint_non_negative(self.read(read)?, operation, output)
    }

    fn read_positive(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> f32,
    ) -> Result<f32> {
        check_native_joint_positive(self.read(read)?, operation, output)
    }

    fn read_vec2(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> Vec2,
    ) -> Result<Vec2> {
        check_native_joint_vec2(self.read(read)?, operation, output)
    }

    fn read_ordered_range(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> (f32, f32),
    ) -> Result<(f32, f32)> {
        let (lower, upper) = self.read(read)?;
        check_native_joint_ordered_range(lower, upper, operation, output)
    }

    fn read_non_negative_range(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> (f32, f32),
    ) -> Result<(f32, f32)> {
        let (lower, upper) = self.read(read)?;
        check_native_joint_non_negative_range(lower, upper, operation, output)
    }

    fn read_revolute_range(
        &self,
        operation: &'static str,
        output: &'static str,
        read: impl FnOnce(JointId) -> (f32, f32),
    ) -> Result<(f32, f32)> {
        let (lower, upper) = self.read(read)?;
        check_native_revolute_joint_range(lower, upper, operation, output)
    }

    fn write(&mut self, write: JointWrite) -> Result<()> {
        let joint = self.joint();
        joint.proof.call(|call| write.apply(call.id()))
    }
}

macro_rules! typed_joint {
    ($name:ident, $kind:ident, $conversion:ident) => {
        #[derive(Debug)]
        pub struct $name<'world>(Joint<'world>);

        impl<'world> TryFrom<Joint<'world>> for $name<'world> {
            type Error = Error;

            fn try_from(joint: Joint<'world>) -> Result<Self> {
                let actual = joint.cached_kind();
                if actual == JointType::$kind {
                    Ok(Self(joint))
                } else {
                    Err(Error::WrongJointType {
                        expected: JointType::$kind,
                        actual,
                    })
                }
            }
        }

        impl<'world> TypedJointCapability for $name<'world> {
            fn joint(&self) -> &Joint<'_> {
                &self.0
            }
        }

        impl<'world> Deref for $name<'world> {
            type Target = Joint<'world>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $name<'_> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<'world> $name<'world> {
            pub fn into_joint(self) -> Joint<'world> {
                self.0
            }
        }

        impl<'world> Joint<'world> {
            pub fn $conversion(self) -> Result<$name<'world>> {
                self.try_into()
            }
        }
    };
}

typed_joint!(DistanceJoint, Distance, into_distance);
typed_joint!(FilterJoint, Filter, into_filter);
typed_joint!(MotorJoint, Motor, into_motor);
typed_joint!(PrismaticJoint, Prismatic, into_prismatic);
typed_joint!(RevoluteJoint, Revolute, into_revolute);
typed_joint!(WeldJoint, Weld, into_weld);
typed_joint!(WheelJoint, Wheel, into_wheel);

impl DistanceJoint<'_> {
    pub fn length(&self) -> Result<f32> {
        self.read_positive("DistanceJoint::length", "length", |id| unsafe {
            ffi::b2DistanceJoint_GetLength(raw_joint_id(id))
        })
    }

    pub fn set_length(&mut self, length: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetLength(length))
    }

    pub fn spring_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2DistanceJoint_IsSpringEnabled(raw_joint_id(id)) })
    }

    pub fn enable_spring(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::DistanceEnableSpring(enabled))
    }

    pub fn spring_force_range(&self) -> Result<(f32, f32)> {
        self.read_ordered_range(
            "DistanceJoint::spring_force_range",
            "spring_force_range",
            |id| {
                let mut lower = 0.0;
                let mut upper = 0.0;
                unsafe {
                    ffi::b2DistanceJoint_GetSpringForceRange(
                        raw_joint_id(id),
                        &mut lower,
                        &mut upper,
                    )
                };
                (lower, upper)
            },
        )
    }

    pub fn lower_spring_force(&self) -> Result<f32> {
        self.spring_force_range().map(|range| range.0)
    }

    pub fn upper_spring_force(&self) -> Result<f32> {
        self.spring_force_range().map(|range| range.1)
    }

    pub fn set_spring_force_range(&mut self, lower: f32, upper: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetSpringForceRange(lower, upper))
    }

    pub fn spring_hertz(&self) -> Result<f32> {
        self.read_non_negative("DistanceJoint::spring_hertz", "spring_hertz", |id| unsafe {
            ffi::b2DistanceJoint_GetSpringHertz(raw_joint_id(id))
        })
    }

    pub fn set_spring_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetSpringHertz(hertz))
    }

    pub fn spring_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "DistanceJoint::spring_damping_ratio",
            "spring_damping_ratio",
            |id| unsafe { ffi::b2DistanceJoint_GetSpringDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_spring_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetSpringDampingRatio(ratio))
    }

    pub fn limit_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2DistanceJoint_IsLimitEnabled(raw_joint_id(id)) })
    }

    pub fn enable_limit(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::DistanceEnableLimit(enabled))
    }

    pub fn min_length(&self) -> Result<f32> {
        self.length_range("DistanceJoint::min_length")
            .map(|range| range.0)
    }

    pub fn max_length(&self) -> Result<f32> {
        self.length_range("DistanceJoint::max_length")
            .map(|range| range.1)
    }

    pub fn current_length(&self) -> Result<f32> {
        self.read_non_negative(
            "DistanceJoint::current_length",
            "current_length",
            |id| unsafe { ffi::b2DistanceJoint_GetCurrentLength(raw_joint_id(id)) },
        )
    }

    fn length_range(&self, operation: &'static str) -> Result<(f32, f32)> {
        self.read_non_negative_range(operation, "length_range", |id| unsafe {
            (
                ffi::b2DistanceJoint_GetMinLength(raw_joint_id(id)),
                ffi::b2DistanceJoint_GetMaxLength(raw_joint_id(id)),
            )
        })
    }

    pub fn set_length_range(&mut self, min: f32, max: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetLengthRange(min, max))
    }

    pub fn motor_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2DistanceJoint_IsMotorEnabled(raw_joint_id(id)) })
    }

    pub fn enable_motor(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::DistanceEnableMotor(enabled))
    }

    pub fn motor_speed(&self) -> Result<f32> {
        self.read_finite("DistanceJoint::motor_speed", "motor_speed", |id| unsafe {
            ffi::b2DistanceJoint_GetMotorSpeed(raw_joint_id(id))
        })
    }

    pub fn set_motor_speed(&mut self, speed: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetMotorSpeed(speed))
    }

    pub fn max_motor_force(&self) -> Result<f32> {
        self.read_non_negative(
            "DistanceJoint::max_motor_force",
            "max_motor_force",
            |id| unsafe { ffi::b2DistanceJoint_GetMaxMotorForce(raw_joint_id(id)) },
        )
    }

    pub fn set_max_motor_force(&mut self, force: f32) -> Result<()> {
        self.write(JointWrite::DistanceSetMaxMotorForce(force))
    }

    pub fn motor_force(&self) -> Result<f32> {
        self.read_finite("DistanceJoint::motor_force", "motor_force", |id| unsafe {
            ffi::b2DistanceJoint_GetMotorForce(raw_joint_id(id))
        })
    }
}

impl PrismaticJoint<'_> {
    pub fn spring_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2PrismaticJoint_IsSpringEnabled(raw_joint_id(id)) })
    }

    pub fn enable_spring(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::PrismaticEnableSpring(enabled))
    }

    pub fn spring_hertz(&self) -> Result<f32> {
        self.read_non_negative(
            "PrismaticJoint::spring_hertz",
            "spring_hertz",
            |id| unsafe { ffi::b2PrismaticJoint_GetSpringHertz(raw_joint_id(id)) },
        )
    }

    pub fn set_spring_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::PrismaticSetSpringHertz(hertz))
    }

    pub fn spring_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "PrismaticJoint::spring_damping_ratio",
            "spring_damping_ratio",
            |id| unsafe { ffi::b2PrismaticJoint_GetSpringDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_spring_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::PrismaticSetSpringDampingRatio(ratio))
    }

    pub fn target_translation(&self) -> Result<f32> {
        self.read_finite(
            "PrismaticJoint::target_translation",
            "target_translation",
            |id| unsafe { ffi::b2PrismaticJoint_GetTargetTranslation(raw_joint_id(id)) },
        )
    }

    pub fn set_target_translation(&mut self, translation: f32) -> Result<()> {
        self.write(JointWrite::PrismaticSetTargetTranslation(translation))
    }

    pub fn limit_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2PrismaticJoint_IsLimitEnabled(raw_joint_id(id)) })
    }

    pub fn enable_limit(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::PrismaticEnableLimit(enabled))
    }

    pub fn lower_limit(&self) -> Result<f32> {
        self.limit_range("PrismaticJoint::lower_limit")
            .map(|range| range.0)
    }

    pub fn upper_limit(&self) -> Result<f32> {
        self.limit_range("PrismaticJoint::upper_limit")
            .map(|range| range.1)
    }

    fn limit_range(&self, operation: &'static str) -> Result<(f32, f32)> {
        self.read_ordered_range(operation, "limit_range", |id| unsafe {
            (
                ffi::b2PrismaticJoint_GetLowerLimit(raw_joint_id(id)),
                ffi::b2PrismaticJoint_GetUpperLimit(raw_joint_id(id)),
            )
        })
    }

    pub fn set_limits(&mut self, lower: f32, upper: f32) -> Result<()> {
        self.write(JointWrite::PrismaticSetLimits(lower, upper))
    }

    pub fn motor_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2PrismaticJoint_IsMotorEnabled(raw_joint_id(id)) })
    }

    pub fn enable_motor(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::PrismaticEnableMotor(enabled))
    }

    pub fn motor_speed(&self) -> Result<f32> {
        self.read_finite("PrismaticJoint::motor_speed", "motor_speed", |id| unsafe {
            ffi::b2PrismaticJoint_GetMotorSpeed(raw_joint_id(id))
        })
    }

    pub fn set_motor_speed(&mut self, speed: f32) -> Result<()> {
        self.write(JointWrite::PrismaticSetMotorSpeed(speed))
    }

    pub fn max_motor_force(&self) -> Result<f32> {
        self.read_non_negative(
            "PrismaticJoint::max_motor_force",
            "max_motor_force",
            |id| unsafe { ffi::b2PrismaticJoint_GetMaxMotorForce(raw_joint_id(id)) },
        )
    }

    pub fn set_max_motor_force(&mut self, force: f32) -> Result<()> {
        self.write(JointWrite::PrismaticSetMaxMotorForce(force))
    }

    pub fn motor_force(&self) -> Result<f32> {
        self.read_finite("PrismaticJoint::motor_force", "motor_force", |id| unsafe {
            ffi::b2PrismaticJoint_GetMotorForce(raw_joint_id(id))
        })
    }

    pub fn translation(&self) -> Result<f32> {
        self.read_finite("PrismaticJoint::translation", "translation", |id| unsafe {
            ffi::b2PrismaticJoint_GetTranslation(raw_joint_id(id))
        })
    }

    pub fn speed(&self) -> Result<f32> {
        self.read_finite("PrismaticJoint::speed", "speed", |id| unsafe {
            ffi::b2PrismaticJoint_GetSpeed(raw_joint_id(id))
        })
    }
}

impl RevoluteJoint<'_> {
    pub fn spring_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2RevoluteJoint_IsSpringEnabled(raw_joint_id(id)) })
    }

    pub fn enable_spring(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::RevoluteEnableSpring(enabled))
    }

    pub fn spring_hertz(&self) -> Result<f32> {
        self.read_non_negative("RevoluteJoint::spring_hertz", "spring_hertz", |id| unsafe {
            ffi::b2RevoluteJoint_GetSpringHertz(raw_joint_id(id))
        })
    }

    pub fn set_spring_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::RevoluteSetSpringHertz(hertz))
    }

    pub fn spring_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "RevoluteJoint::spring_damping_ratio",
            "spring_damping_ratio",
            |id| unsafe { ffi::b2RevoluteJoint_GetSpringDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_spring_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::RevoluteSetSpringDampingRatio(ratio))
    }

    pub fn target_angle(&self) -> Result<f32> {
        self.read_finite("RevoluteJoint::target_angle", "target_angle", |id| unsafe {
            ffi::b2RevoluteJoint_GetTargetAngle(raw_joint_id(id))
        })
    }

    pub fn set_target_angle(&mut self, angle: f32) -> Result<()> {
        self.write(JointWrite::RevoluteSetTargetAngle(angle))
    }

    pub fn angle(&self) -> Result<f32> {
        self.read_finite("RevoluteJoint::angle", "angle", |id| unsafe {
            ffi::b2RevoluteJoint_GetAngle(raw_joint_id(id))
        })
    }

    pub fn limit_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2RevoluteJoint_IsLimitEnabled(raw_joint_id(id)) })
    }

    pub fn enable_limit(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::RevoluteEnableLimit(enabled))
    }

    pub fn lower_limit(&self) -> Result<f32> {
        self.limit_range("RevoluteJoint::lower_limit")
            .map(|range| range.0)
    }

    pub fn upper_limit(&self) -> Result<f32> {
        self.limit_range("RevoluteJoint::upper_limit")
            .map(|range| range.1)
    }

    fn limit_range(&self, operation: &'static str) -> Result<(f32, f32)> {
        self.read_revolute_range(operation, "limit_range", |id| unsafe {
            (
                ffi::b2RevoluteJoint_GetLowerLimit(raw_joint_id(id)),
                ffi::b2RevoluteJoint_GetUpperLimit(raw_joint_id(id)),
            )
        })
    }

    pub fn set_limits(&mut self, lower: f32, upper: f32) -> Result<()> {
        self.write(JointWrite::RevoluteSetLimits(lower, upper))
    }

    pub fn motor_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2RevoluteJoint_IsMotorEnabled(raw_joint_id(id)) })
    }

    pub fn enable_motor(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::RevoluteEnableMotor(enabled))
    }

    pub fn motor_speed(&self) -> Result<f32> {
        self.read_finite("RevoluteJoint::motor_speed", "motor_speed", |id| unsafe {
            ffi::b2RevoluteJoint_GetMotorSpeed(raw_joint_id(id))
        })
    }

    pub fn set_motor_speed(&mut self, speed: f32) -> Result<()> {
        self.write(JointWrite::RevoluteSetMotorSpeed(speed))
    }

    pub fn motor_torque(&self) -> Result<f32> {
        self.read_finite("RevoluteJoint::motor_torque", "motor_torque", |id| unsafe {
            ffi::b2RevoluteJoint_GetMotorTorque(raw_joint_id(id))
        })
    }

    pub fn max_motor_torque(&self) -> Result<f32> {
        self.read_non_negative(
            "RevoluteJoint::max_motor_torque",
            "max_motor_torque",
            |id| unsafe { ffi::b2RevoluteJoint_GetMaxMotorTorque(raw_joint_id(id)) },
        )
    }

    pub fn set_max_motor_torque(&mut self, torque: f32) -> Result<()> {
        self.write(JointWrite::RevoluteSetMaxMotorTorque(torque))
    }
}

impl WheelJoint<'_> {
    pub fn spring_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2WheelJoint_IsSpringEnabled(raw_joint_id(id)) })
    }

    pub fn enable_spring(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::WheelEnableSpring(enabled))
    }

    pub fn spring_hertz(&self) -> Result<f32> {
        self.read_non_negative("WheelJoint::spring_hertz", "spring_hertz", |id| unsafe {
            ffi::b2WheelJoint_GetSpringHertz(raw_joint_id(id))
        })
    }

    pub fn set_spring_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::WheelSetSpringHertz(hertz))
    }

    pub fn spring_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "WheelJoint::spring_damping_ratio",
            "spring_damping_ratio",
            |id| unsafe { ffi::b2WheelJoint_GetSpringDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_spring_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::WheelSetSpringDampingRatio(ratio))
    }

    pub fn limit_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2WheelJoint_IsLimitEnabled(raw_joint_id(id)) })
    }

    pub fn enable_limit(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::WheelEnableLimit(enabled))
    }

    pub fn lower_limit(&self) -> Result<f32> {
        self.limit_range("WheelJoint::lower_limit")
            .map(|range| range.0)
    }

    pub fn upper_limit(&self) -> Result<f32> {
        self.limit_range("WheelJoint::upper_limit")
            .map(|range| range.1)
    }

    fn limit_range(&self, operation: &'static str) -> Result<(f32, f32)> {
        self.read_ordered_range(operation, "limit_range", |id| unsafe {
            (
                ffi::b2WheelJoint_GetLowerLimit(raw_joint_id(id)),
                ffi::b2WheelJoint_GetUpperLimit(raw_joint_id(id)),
            )
        })
    }

    pub fn set_limits(&mut self, lower: f32, upper: f32) -> Result<()> {
        self.write(JointWrite::WheelSetLimits(lower, upper))
    }

    pub fn motor_enabled(&self) -> Result<bool> {
        self.read(|id| unsafe { ffi::b2WheelJoint_IsMotorEnabled(raw_joint_id(id)) })
    }

    pub fn enable_motor(&mut self, enabled: bool) -> Result<()> {
        self.write(JointWrite::WheelEnableMotor(enabled))
    }

    pub fn motor_speed(&self) -> Result<f32> {
        self.read_finite("WheelJoint::motor_speed", "motor_speed", |id| unsafe {
            ffi::b2WheelJoint_GetMotorSpeed(raw_joint_id(id))
        })
    }

    pub fn set_motor_speed(&mut self, speed: f32) -> Result<()> {
        self.write(JointWrite::WheelSetMotorSpeed(speed))
    }

    pub fn motor_torque(&self) -> Result<f32> {
        self.read_finite("WheelJoint::motor_torque", "motor_torque", |id| unsafe {
            ffi::b2WheelJoint_GetMotorTorque(raw_joint_id(id))
        })
    }

    pub fn max_motor_torque(&self) -> Result<f32> {
        self.read_non_negative(
            "WheelJoint::max_motor_torque",
            "max_motor_torque",
            |id| unsafe { ffi::b2WheelJoint_GetMaxMotorTorque(raw_joint_id(id)) },
        )
    }

    pub fn set_max_motor_torque(&mut self, torque: f32) -> Result<()> {
        self.write(JointWrite::WheelSetMaxMotorTorque(torque))
    }
}

impl WeldJoint<'_> {
    pub fn linear_hertz(&self) -> Result<f32> {
        self.read_non_negative("WeldJoint::linear_hertz", "linear_hertz", |id| unsafe {
            ffi::b2WeldJoint_GetLinearHertz(raw_joint_id(id))
        })
    }

    pub fn set_linear_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::WeldSetLinearHertz(hertz))
    }

    pub fn linear_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "WeldJoint::linear_damping_ratio",
            "linear_damping_ratio",
            |id| unsafe { ffi::b2WeldJoint_GetLinearDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_linear_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::WeldSetLinearDampingRatio(ratio))
    }

    pub fn angular_hertz(&self) -> Result<f32> {
        self.read_non_negative("WeldJoint::angular_hertz", "angular_hertz", |id| unsafe {
            ffi::b2WeldJoint_GetAngularHertz(raw_joint_id(id))
        })
    }

    pub fn set_angular_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::WeldSetAngularHertz(hertz))
    }

    pub fn angular_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "WeldJoint::angular_damping_ratio",
            "angular_damping_ratio",
            |id| unsafe { ffi::b2WeldJoint_GetAngularDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_angular_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::WeldSetAngularDampingRatio(ratio))
    }
}

impl MotorJoint<'_> {
    pub fn linear_velocity(&self) -> Result<Vec2> {
        self.read_vec2("MotorJoint::linear_velocity", "linear_velocity", |id| {
            Vec2::from_raw(unsafe { ffi::b2MotorJoint_GetLinearVelocity(raw_joint_id(id)) })
        })
    }

    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, velocity: V) -> Result<()> {
        let joint = self.joint();
        joint
            .proof
            .call(|call| JointWrite::MotorSetLinearVelocity(velocity.into()).apply(call.id()))
    }

    pub fn angular_velocity(&self) -> Result<f32> {
        self.read_finite(
            "MotorJoint::angular_velocity",
            "angular_velocity",
            |id| unsafe { ffi::b2MotorJoint_GetAngularVelocity(raw_joint_id(id)) },
        )
    }

    pub fn set_angular_velocity(&mut self, velocity: f32) -> Result<()> {
        self.write(JointWrite::MotorSetAngularVelocity(velocity))
    }

    pub fn max_velocity_force(&self) -> Result<f32> {
        self.read_non_negative(
            "MotorJoint::max_velocity_force",
            "max_velocity_force",
            |id| unsafe { ffi::b2MotorJoint_GetMaxVelocityForce(raw_joint_id(id)) },
        )
    }

    pub fn set_max_velocity_force(&mut self, force: f32) -> Result<()> {
        self.write(JointWrite::MotorSetMaxVelocityForce(force))
    }

    pub fn max_velocity_torque(&self) -> Result<f32> {
        self.read_non_negative(
            "MotorJoint::max_velocity_torque",
            "max_velocity_torque",
            |id| unsafe { ffi::b2MotorJoint_GetMaxVelocityTorque(raw_joint_id(id)) },
        )
    }

    pub fn set_max_velocity_torque(&mut self, torque: f32) -> Result<()> {
        self.write(JointWrite::MotorSetMaxVelocityTorque(torque))
    }

    pub fn linear_hertz(&self) -> Result<f32> {
        self.read_non_negative("MotorJoint::linear_hertz", "linear_hertz", |id| unsafe {
            ffi::b2MotorJoint_GetLinearHertz(raw_joint_id(id))
        })
    }

    pub fn set_linear_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::MotorSetLinearHertz(hertz))
    }

    pub fn linear_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "MotorJoint::linear_damping_ratio",
            "linear_damping_ratio",
            |id| unsafe { ffi::b2MotorJoint_GetLinearDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_linear_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::MotorSetLinearDampingRatio(ratio))
    }

    pub fn angular_hertz(&self) -> Result<f32> {
        self.read_non_negative("MotorJoint::angular_hertz", "angular_hertz", |id| unsafe {
            ffi::b2MotorJoint_GetAngularHertz(raw_joint_id(id))
        })
    }

    pub fn set_angular_hertz(&mut self, hertz: f32) -> Result<()> {
        self.write(JointWrite::MotorSetAngularHertz(hertz))
    }

    pub fn angular_damping_ratio(&self) -> Result<f32> {
        self.read_non_negative(
            "MotorJoint::angular_damping_ratio",
            "angular_damping_ratio",
            |id| unsafe { ffi::b2MotorJoint_GetAngularDampingRatio(raw_joint_id(id)) },
        )
    }

    pub fn set_angular_damping_ratio(&mut self, ratio: f32) -> Result<()> {
        self.write(JointWrite::MotorSetAngularDampingRatio(ratio))
    }

    pub fn max_spring_force(&self) -> Result<f32> {
        self.read_non_negative(
            "MotorJoint::max_spring_force",
            "max_spring_force",
            |id| unsafe { ffi::b2MotorJoint_GetMaxSpringForce(raw_joint_id(id)) },
        )
    }

    pub fn set_max_spring_force(&mut self, force: f32) -> Result<()> {
        self.write(JointWrite::MotorSetMaxSpringForce(force))
    }

    pub fn max_spring_torque(&self) -> Result<f32> {
        self.read_non_negative(
            "MotorJoint::max_spring_torque",
            "max_spring_torque",
            |id| unsafe { ffi::b2MotorJoint_GetMaxSpringTorque(raw_joint_id(id)) },
        )
    }

    pub fn set_max_spring_torque(&mut self, torque: f32) -> Result<()> {
        self.write(JointWrite::MotorSetMaxSpringTorque(torque))
    }
}
