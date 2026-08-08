use crate::error::{Error, Result};
use crate::joints::runtime::JointWrite;
use crate::types::{BodyId, JointId, Vec2};
use boxdd_sys::ffi;
use std::os::raw::c_void;

mod scoped;
mod user_data;

use self::user_data::*;

/// A scoped joint handle tied to a mutable borrow of the world.
pub struct Joint<'w> {
    pub(crate) proof: crate::world::JointProof<'w>,
}

/// Joint kinds tracked by the safe world's identity registry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JointType {
    Distance,
    Filter,
    Motor,
    Prismatic,
    Revolute,
    Weld,
    Wheel,
}

impl JointType {
    #[inline]
    pub const fn from_raw(raw: ffi::b2JointType) -> Option<Self> {
        match raw {
            ffi::b2JointType_b2_distanceJoint => Some(Self::Distance),
            ffi::b2JointType_b2_filterJoint => Some(Self::Filter),
            ffi::b2JointType_b2_motorJoint => Some(Self::Motor),
            ffi::b2JointType_b2_prismaticJoint => Some(Self::Prismatic),
            ffi::b2JointType_b2_revoluteJoint => Some(Self::Revolute),
            ffi::b2JointType_b2_weldJoint => Some(Self::Weld),
            ffi::b2JointType_b2_wheelJoint => Some(Self::Wheel),
            _ => None,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2JointType {
        match self {
            Self::Distance => ffi::b2JointType_b2_distanceJoint,
            Self::Filter => ffi::b2JointType_b2_filterJoint,
            Self::Motor => ffi::b2JointType_b2_motorJoint,
            Self::Prismatic => ffi::b2JointType_b2_prismaticJoint,
            Self::Revolute => ffi::b2JointType_b2_revoluteJoint,
            Self::Weld => ffi::b2JointType_b2_weldJoint,
            Self::Wheel => ffi::b2JointType_b2_wheelJoint,
        }
    }
}

impl TryFrom<ffi::b2JointType> for JointType {
    type Error = ffi::b2JointType;

    #[inline]
    fn try_from(value: ffi::b2JointType) -> std::result::Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

/// Shared constraint tuning (Hertz + damping ratio) used by Box2D joints.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ConstraintTuning {
    hertz: f32,
    damping_ratio: f32,
}

impl ConstraintTuning {
    #[inline]
    pub fn new(hertz: f32, damping_ratio: f32) -> Result<Self> {
        let tuning = Self {
            hertz,
            damping_ratio,
        };
        super::check_joint_tuning(tuning, "ConstraintTuning::new", "hertz/damping_ratio")?;
        Ok(tuning)
    }

    #[inline]
    pub const fn hertz(self) -> f32 {
        self.hertz
    }

    #[inline]
    pub const fn damping_ratio(self) -> f32 {
        self.damping_ratio
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ConstraintTuning {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            hertz: f32,
            damping_ratio: f32,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.hertz, repr.damping_ratio).map_err(serde::de::Error::custom)
    }
}

#[inline]
pub(crate) fn joint_body_a_id_in_impl(joint: crate::world::JointCall<'_>) -> Result<BodyId> {
    let raw = unsafe { ffi::b2Joint_GetBodyA(raw_joint_id(joint.id())) };
    joint.with_output_identity_resolver(|resolver| resolver.active_body(raw))
}

#[inline]
pub(crate) fn joint_body_b_id_in_impl(joint: crate::world::JointCall<'_>) -> Result<BodyId> {
    let raw = unsafe { ffi::b2Joint_GetBodyB(raw_joint_id(joint.id())) };
    joint.with_output_identity_resolver(|resolver| resolver.active_body(raw))
}

#[inline]
pub(crate) fn joint_linear_separation_impl(id: JointId) -> Result<f32> {
    super::check_native_joint_finite(
        unsafe { ffi::b2Joint_GetLinearSeparation(raw_joint_id(id)) },
        "Joint::linear_separation",
        "linear_separation",
    )
}

#[inline]
pub(crate) fn joint_angular_separation_impl(id: JointId) -> Result<f32> {
    super::check_native_joint_finite(
        unsafe { ffi::b2Joint_GetAngularSeparation(raw_joint_id(id)) },
        "Joint::angular_separation",
        "angular_separation",
    )
}

#[inline]
pub(crate) fn joint_constraint_force_impl(id: JointId) -> Result<Vec2> {
    super::check_native_joint_vec2(
        Vec2::from_raw(unsafe { ffi::b2Joint_GetConstraintForce(raw_joint_id(id)) }),
        "Joint::constraint_force",
        "constraint_force",
    )
}

#[inline]
pub(crate) fn joint_constraint_torque_impl(id: JointId) -> Result<f32> {
    super::check_native_joint_finite(
        unsafe { ffi::b2Joint_GetConstraintTorque(raw_joint_id(id)) },
        "Joint::constraint_torque",
        "constraint_torque",
    )
}

#[inline]
pub(crate) fn joint_collide_connected_impl(id: JointId) -> bool {
    unsafe { ffi::b2Joint_GetCollideConnected(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_constraint_tuning_impl(id: JointId) -> Result<ConstraintTuning> {
    let mut hertz = 0.0f32;
    let mut damping_ratio = 0.0f32;
    unsafe { ffi::b2Joint_GetConstraintTuning(raw_joint_id(id), &mut hertz, &mut damping_ratio) };
    ConstraintTuning::new(hertz, damping_ratio).map_err(|_| Error::InvalidNativeOutput {
        operation: "Joint::constraint_tuning",
        output: "constraint_tuning",
        constraint: "finite non-negative hertz and damping ratio values",
    })
}

#[inline]
pub(crate) fn joint_local_frame_a_impl(id: JointId) -> Result<crate::Transform> {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameA(raw_joint_id(id)) }).map_err(
        |_| Error::InvalidNativeOutput {
            operation: "Joint::local_frame_a",
            output: "local_frame_a",
            constraint: "a finite rigid transform",
        },
    )
}

#[inline]
pub(crate) fn joint_local_frame_b_impl(id: JointId) -> Result<crate::Transform> {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameB(raw_joint_id(id)) }).map_err(
        |_| Error::InvalidNativeOutput {
            operation: "Joint::local_frame_b",
            output: "local_frame_b",
            constraint: "a finite rigid transform",
        },
    )
}

#[inline]
pub(crate) fn joint_force_threshold_impl(id: JointId) -> Result<f32> {
    super::check_native_joint_non_negative(
        unsafe { ffi::b2Joint_GetForceThreshold(raw_joint_id(id)) },
        "Joint::force_threshold",
        "force_threshold",
    )
}

#[inline]
pub(crate) fn joint_torque_threshold_impl(id: JointId) -> Result<f32> {
    super::check_native_joint_non_negative(
        unsafe { ffi::b2Joint_GetTorqueThreshold(raw_joint_id(id)) },
        "Joint::torque_threshold",
        "torque_threshold",
    )
}

impl Joint<'_> {
    #[inline]
    pub(crate) const fn cached_kind(&self) -> JointType {
        self.proof.kind()
    }

    #[inline]
    fn joint_id(&self) -> JointId {
        self.proof.id()
    }

    #[inline]
    fn joint_access(&self) -> &crate::world::JointProof<'_> {
        &self.proof
    }

    /// Return the constraint type captured when this capability was acquired.
    pub fn joint_type(&self) -> Result<JointType> {
        self.joint_access().call(|joint| Ok(joint.kind()))
    }

    pub fn body_a_id(&self) -> Result<BodyId> {
        self.joint_access().call(joint_body_a_id_in_impl)
    }

    pub fn body_b_id(&self) -> Result<BodyId> {
        self.joint_access().call(joint_body_b_id_in_impl)
    }

    pub fn collide_connected(&self) -> Result<bool> {
        self.joint_access()
            .call(|_| Ok(joint_collide_connected_impl(self.joint_id())))
    }

    pub fn set_collide_connected(&mut self, flag: bool) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::SetCollideConnected(flag).apply(self.joint_id()))
    }

    pub fn constraint_tuning(&self) -> Result<ConstraintTuning> {
        self.joint_access()
            .call(|_| joint_constraint_tuning_impl(self.joint_id()))
    }

    pub fn set_constraint_tuning(&mut self, tuning: ConstraintTuning) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::SetConstraintTuning(tuning).apply(self.joint_id()))
    }

    pub fn local_frame_a(&self) -> Result<crate::Transform> {
        self.joint_access()
            .call(|_| joint_local_frame_a_impl(self.joint_id()))
    }

    pub fn local_frame_b(&self) -> Result<crate::Transform> {
        self.joint_access()
            .call(|_| joint_local_frame_b_impl(self.joint_id()))
    }

    pub fn set_local_frame_a(&mut self, frame: crate::Transform) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::SetLocalFrameA(frame).apply(self.joint_id()))
    }

    pub fn set_local_frame_b(&mut self, frame: crate::Transform) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::SetLocalFrameB(frame).apply(self.joint_id()))
    }

    pub fn wake_bodies(&mut self) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::WakeBodies.apply(self.joint_id()))
    }

    pub fn linear_separation(&self) -> Result<f32> {
        self.joint_access()
            .call(|_| joint_linear_separation_impl(self.joint_id()))
    }

    pub fn angular_separation(&self) -> Result<f32> {
        self.joint_access()
            .call(|_| joint_angular_separation_impl(self.joint_id()))
    }

    pub fn constraint_force(&self) -> Result<Vec2> {
        self.joint_access()
            .call(|_| joint_constraint_force_impl(self.joint_id()))
    }

    pub fn constraint_torque(&self) -> Result<f32> {
        self.joint_access()
            .call(|_| joint_constraint_torque_impl(self.joint_id()))
    }

    pub fn force_threshold(&self) -> Result<f32> {
        self.joint_access()
            .call(|_| joint_force_threshold_impl(self.joint_id()))
    }

    pub fn set_force_threshold(&mut self, threshold: f32) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::SetForceThreshold(threshold).apply(self.joint_id()))
    }

    pub fn torque_threshold(&self) -> Result<f32> {
        self.joint_access()
            .call(|_| joint_torque_threshold_impl(self.joint_id()))
    }

    pub fn set_torque_threshold(&mut self, threshold: f32) -> Result<()> {
        self.joint_access()
            .call(|_| JointWrite::SetTorqueThreshold(threshold).apply(self.joint_id()))
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// Box2D and `boxdd` store but never dereference this pointer. If typed user data was
    /// previously set via [`Self::set_user_data`], it is cleared and dropped.
    pub fn set_user_data_ptr_raw(&mut self, p: *mut c_void) -> Result<()> {
        self.joint_access()
            .call(|joint| joint_set_user_data_ptr_impl(joint, p))
    }

    pub fn user_data_ptr_raw(&self) -> Result<*mut c_void> {
        let id = self.joint_id();
        self.joint_access()
            .call(|_| Ok(joint_user_data_ptr_impl(id)))
    }

    pub fn set_user_data<T: 'static>(&mut self, value: T) -> Result<()> {
        let value = crate::core::callback_state::PendingUserValue::new(value);
        self.joint_access()
            .call(move |joint| joint_set_user_data_impl(joint, value))
    }

    pub fn clear_user_data(&mut self) -> Result<bool> {
        self.joint_access().call(joint_clear_user_data_impl)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        self.joint_access()
            .call(move |joint| joint_with_user_data_impl(joint, f))
    }

    pub fn with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        self.joint_access()
            .call(move |joint| joint_with_user_data_mut_impl(joint, f))
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Result<Option<T>> {
        self.joint_access().call(joint_take_user_data_impl::<T>)
    }
}
