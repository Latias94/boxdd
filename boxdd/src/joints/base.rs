use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::core::world_core::WorldCore;
use crate::error::{ApiError, ApiResult};
use crate::types::{BodyId, JointId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

mod owned;
mod runtime_handle;
mod scoped;
mod user_data;

/// A scoped joint handle tied to a mutable borrow of the world.
pub struct Joint<'w> {
    pub(crate) id: JointId,
    pub(crate) core: Arc<crate::core::world_core::WorldCore>,
    pub(crate) _world: PhantomData<&'w World>,
}

/// A RAII-owned joint that is destroyed on drop.
pub struct OwnedJoint {
    id: JointId,
    core: Arc<WorldCore>,
    destroy_on_drop: bool,
    wake_bodies_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

/// Joint kinds reported by Box2D.
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
    fn try_from(value: ffi::b2JointType) -> Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}

#[inline]
fn joint_type_from_ffi(raw: ffi::b2JointType) -> JointType {
    JointType::from_raw(raw).expect("Box2D returned an unknown joint type")
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

/// Shared constraint tuning (Hertz + damping ratio) used by Box2D joints.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ConstraintTuning {
    pub hertz: f32,
    pub damping_ratio: f32,
}

impl ConstraintTuning {
    #[inline]
    pub const fn new(hertz: f32, damping_ratio: f32) -> Self {
        Self {
            hertz,
            damping_ratio,
        }
    }
}

#[inline]
pub(crate) fn joint_is_valid_impl(id: JointId) -> bool {
    unsafe { ffi::b2Joint_IsValid(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_type_raw_impl(id: JointId) -> ffi::b2JointType {
    unsafe { ffi::b2Joint_GetType(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_type_impl(id: JointId) -> JointType {
    joint_type_from_ffi(joint_type_raw_impl(id))
}

#[inline]
pub(crate) fn joint_body_a_id_impl(id: JointId) -> BodyId {
    BodyId::from_raw(unsafe { ffi::b2Joint_GetBodyA(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_body_b_id_impl(id: JointId) -> BodyId {
    BodyId::from_raw(unsafe { ffi::b2Joint_GetBodyB(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_world_id_raw_impl(id: JointId) -> ffi::b2WorldId {
    unsafe { ffi::b2Joint_GetWorld(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_linear_separation_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetLinearSeparation(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_angular_separation_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetAngularSeparation(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_constraint_force_impl(id: JointId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Joint_GetConstraintForce(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_constraint_torque_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetConstraintTorque(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_collide_connected_impl(id: JointId) -> bool {
    unsafe { ffi::b2Joint_GetCollideConnected(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_set_collide_connected_impl(id: JointId, flag: bool) {
    unsafe { ffi::b2Joint_SetCollideConnected(raw_joint_id(id), flag) }
}

#[inline]
pub(crate) fn joint_constraint_tuning_impl(id: JointId) -> ConstraintTuning {
    let mut hertz = 0.0f32;
    let mut damping_ratio = 0.0f32;
    unsafe { ffi::b2Joint_GetConstraintTuning(raw_joint_id(id), &mut hertz, &mut damping_ratio) };
    ConstraintTuning::new(hertz, damping_ratio)
}

#[inline]
pub(crate) fn joint_set_constraint_tuning_impl(id: JointId, tuning: ConstraintTuning) {
    unsafe {
        ffi::b2Joint_SetConstraintTuning(raw_joint_id(id), tuning.hertz, tuning.damping_ratio)
    }
}

#[inline]
pub(crate) fn joint_local_frame_a_impl(id: JointId) -> crate::Transform {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameA(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_local_frame_b_impl(id: JointId) -> crate::Transform {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameB(raw_joint_id(id)) })
}

#[track_caller]
pub(crate) fn assert_joint_local_frame_valid(frame: crate::Transform) {
    assert!(
        frame.is_valid(),
        "joint local frame must be a valid Box2D transform, got {:?}",
        frame
    );
}

#[inline]
pub(crate) fn check_joint_local_frame_valid(frame: crate::Transform) -> ApiResult<()> {
    if frame.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn joint_set_local_frame_a_impl(id: JointId, frame: crate::Transform) {
    unsafe { ffi::b2Joint_SetLocalFrameA(raw_joint_id(id), frame.into_raw()) }
}

#[inline]
pub(crate) fn joint_set_local_frame_b_impl(id: JointId, frame: crate::Transform) {
    unsafe { ffi::b2Joint_SetLocalFrameB(raw_joint_id(id), frame.into_raw()) }
}

#[inline]
pub(crate) fn joint_wake_bodies_impl(id: JointId) {
    unsafe { ffi::b2Joint_WakeBodies(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_force_threshold_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetForceThreshold(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_set_force_threshold_impl(id: JointId, threshold: f32) {
    unsafe { ffi::b2Joint_SetForceThreshold(raw_joint_id(id), threshold) }
}

#[inline]
pub(crate) fn joint_torque_threshold_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetTorqueThreshold(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_set_torque_threshold_impl(id: JointId, threshold: f32) {
    unsafe { ffi::b2Joint_SetTorqueThreshold(raw_joint_id(id), threshold) }
}
