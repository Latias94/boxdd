use std::marker::PhantomData;
use std::rc::Rc;

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
    pub(crate) core: Rc<crate::core::world_core::WorldCore>,
    pub(crate) _world: PhantomData<&'w World>,
}

/// A RAII-owned joint that is destroyed on drop.
pub struct OwnedJoint {
    id: JointId,
    core: Rc<WorldCore>,
    destroy_on_drop: bool,
    wake_bodies_on_drop: bool,
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

    #[inline]
    pub(crate) fn decode_native(raw: ffi::b2JointType) -> ApiResult<Self> {
        Self::from_raw(raw).ok_or(ApiError::InvalidNativeJointType { raw })
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
pub(crate) fn joint_type_raw_impl(id: JointId) -> ffi::b2JointType {
    #[cfg(test)]
    {
        JOINT_GET_TYPE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        if let Some(raw) = JOINT_GET_TYPE_OVERRIDE.with(core::cell::Cell::get) {
            return raw;
        }
    }
    unsafe { ffi::b2Joint_GetType(raw_joint_id(id)) }
}

#[cfg(test)]
thread_local! {
    static JOINT_GET_TYPE_OVERRIDE: core::cell::Cell<Option<ffi::b2JointType>> = const {
        core::cell::Cell::new(None)
    };
    static JOINT_GET_TYPE_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

#[inline]
pub(crate) fn resolve_joint_type_output(
    core: &WorldCore,
    raw: ffi::b2JointType,
) -> ApiResult<JointType> {
    JointType::decode_native(raw).inspect_err(|_| core.poison())
}

#[inline]
pub(crate) fn try_joint_type_impl(core: &WorldCore, id: JointId) -> ApiResult<JointType> {
    resolve_joint_type_output(core, joint_type_raw_impl(id))
}

#[inline]
pub(crate) fn joint_body_a_id_in_impl(brand: crate::id::IdBrand, id: JointId) -> BodyId {
    brand
        .try_body(unsafe { ffi::b2Joint_GetBodyA(raw_joint_id(id)) })
        .expect("Box2D returned an invalid body id for a validated joint")
}

#[inline]
pub(crate) fn joint_body_a_id_impl(id: JointId) -> BodyId {
    joint_body_a_id_in_impl(id.brand(), id)
}

#[inline]
pub(crate) fn joint_body_b_id_in_impl(brand: crate::id::IdBrand, id: JointId) -> BodyId {
    brand
        .try_body(unsafe { ffi::b2Joint_GetBodyB(raw_joint_id(id)) })
        .expect("Box2D returned an invalid body id for a validated joint")
}

#[inline]
pub(crate) fn joint_body_b_id_impl(id: JointId) -> BodyId {
    joint_body_b_id_in_impl(id.brand(), id)
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

pub(crate) const JOINT_SET_COLLIDE_CONNECTED: super::runtime::JointSetOp<bool> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointSetCollideConnected);

#[inline]
pub(crate) fn joint_constraint_tuning_impl(id: JointId) -> ConstraintTuning {
    let mut hertz = 0.0f32;
    let mut damping_ratio = 0.0f32;
    unsafe { ffi::b2Joint_GetConstraintTuning(raw_joint_id(id), &mut hertz, &mut damping_ratio) };
    ConstraintTuning::new(hertz, damping_ratio)
}

pub(crate) const JOINT_SET_CONSTRAINT_TUNING: super::runtime::JointSetOp<ConstraintTuning> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointSetConstraintTuning);

#[inline]
pub(crate) fn joint_local_frame_a_impl(id: JointId) -> crate::Transform {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameA(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_local_frame_b_impl(id: JointId) -> crate::Transform {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameB(raw_joint_id(id)) })
}

pub(crate) const JOINT_SET_LOCAL_FRAME_A: super::runtime::JointSetOp<crate::Transform> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointSetLocalFrameA);

pub(crate) const JOINT_SET_LOCAL_FRAME_B: super::runtime::JointSetOp<crate::Transform> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointSetLocalFrameB);

pub(crate) const JOINT_WAKE_BODIES: super::runtime::JointSetOp<()> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointWakeBodies);

#[inline]
pub(crate) fn joint_force_threshold_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetForceThreshold(raw_joint_id(id)) }
}

pub(crate) const JOINT_SET_FORCE_THRESHOLD: super::runtime::JointSetOp<f32> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointSetForceThreshold);

#[inline]
pub(crate) fn joint_torque_threshold_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetTorqueThreshold(raw_joint_id(id)) }
}

pub(crate) const JOINT_SET_TORQUE_THRESHOLD: super::runtime::JointSetOp<f32> =
    super::runtime::JointSetOp::new(super::runtime::JointWriteKind::JointSetTorqueThreshold);

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct JointGetTypeOverride;

    impl JointGetTypeOverride {
        fn install(raw: ffi::b2JointType) -> Self {
            JOINT_GET_TYPE_OVERRIDE.with(|current| {
                assert_eq!(current.replace(Some(raw)), None);
            });
            JOINT_GET_TYPE_CALLS.with(|calls| calls.set(0));
            Self
        }

        fn calls(&self) -> usize {
            JOINT_GET_TYPE_CALLS.with(core::cell::Cell::get)
        }
    }

    impl Drop for JointGetTypeOverride {
        fn drop(&mut self) {
            JOINT_GET_TYPE_OVERRIDE.with(|current| current.set(None));
            JOINT_GET_TYPE_CALLS.with(|calls| calls.set(0));
        }
    }

    #[test]
    fn joint_type_native_decoder_preserves_known_values_and_reports_the_raw_unknown() {
        for expected in [
            JointType::Distance,
            JointType::Filter,
            JointType::Motor,
            JointType::Prismatic,
            JointType::Revolute,
            JointType::Weld,
            JointType::Wheel,
        ] {
            assert_eq!(JointType::decode_native(expected.into_raw()), Ok(expected));
        }

        let raw = u32::MAX;
        assert_eq!(
            JointType::decode_native(raw),
            Err(ApiError::InvalidNativeJointType { raw })
        );
    }

    #[test]
    fn all_public_joint_type_getters_report_unknown_once_then_stop_before_get_type() {
        let raw = u32::MAX;

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let body_a = world.create_body_id(crate::BodyBuilder::new().build());
            let body_b = world.create_body_id(crate::BodyBuilder::new().build());
            let joint = world.create_distance_joint_owned(&crate::DistanceJointDef::new(
                crate::JointBase::new(body_a, body_b),
            ));
            let get_type = JointGetTypeOverride::install(raw);

            assert_eq!(
                joint.try_joint_type(),
                Err(ApiError::InvalidNativeJointType { raw })
            );
            assert_eq!(get_type.calls(), 1);
            assert_eq!(joint.try_joint_type(), Err(ApiError::WorldPoisoned));
            assert_eq!(joint.try_joint_type_raw(), Err(ApiError::WorldPoisoned));
            assert_eq!(get_type.calls(), 1);
        }

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let body_a = world.create_body_id(crate::BodyBuilder::new().build());
            let body_b = world.create_body_id(crate::BodyBuilder::new().build());
            let joint = world.create_distance_joint_id(&crate::DistanceJointDef::new(
                crate::JointBase::new(body_a, body_b),
            ));
            let get_type = JointGetTypeOverride::install(raw);

            assert_eq!(
                world.try_joint_type(joint),
                Err(ApiError::InvalidNativeJointType { raw })
            );
            assert_eq!(get_type.calls(), 1);
            assert_eq!(world.try_joint_type(joint), Err(ApiError::WorldPoisoned));
            assert_eq!(
                world.try_joint_type_raw(joint),
                Err(ApiError::WorldPoisoned)
            );
            assert_eq!(get_type.calls(), 1);
        }

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let body_a = world.create_body_id(crate::BodyBuilder::new().build());
            let body_b = world.create_body_id(crate::BodyBuilder::new().build());
            let joint = world.create_distance_joint_id(&crate::DistanceJointDef::new(
                crate::JointBase::new(body_a, body_b),
            ));
            let handle = world.handle();
            let get_type = JointGetTypeOverride::install(raw);

            assert_eq!(
                handle.try_joint_type(joint),
                Err(ApiError::InvalidNativeJointType { raw })
            );
            assert_eq!(get_type.calls(), 1);
            assert_eq!(handle.try_joint_type(joint), Err(ApiError::WorldPoisoned));
            assert_eq!(
                handle.try_joint_type_raw(joint),
                Err(ApiError::WorldPoisoned)
            );
            assert_eq!(get_type.calls(), 1);
        }

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let body_a = world.create_body_id(crate::BodyBuilder::new().build());
            let body_b = world.create_body_id(crate::BodyBuilder::new().build());
            let joint = world.create_distance_joint_id(&crate::DistanceJointDef::new(
                crate::JointBase::new(body_a, body_b),
            ));
            let joint = world.joint(joint).unwrap();
            let get_type = JointGetTypeOverride::install(raw);

            assert_eq!(
                joint.try_joint_type(),
                Err(ApiError::InvalidNativeJointType { raw })
            );
            assert_eq!(get_type.calls(), 1);
            assert_eq!(joint.try_joint_type(), Err(ApiError::WorldPoisoned));
            assert_eq!(joint.try_joint_type_raw(), Err(ApiError::WorldPoisoned));
            assert_eq!(get_type.calls(), 1);
        }
    }

    #[test]
    fn infallible_joint_type_poisoning_precedes_its_unknown_native_panic() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let body_a = world.create_body_id(crate::BodyBuilder::new().build());
        let body_b = world.create_body_id(crate::BodyBuilder::new().build());
        let joint = world.create_distance_joint_owned(&crate::DistanceJointDef::new(
            crate::JointBase::new(body_a, body_b),
        ));
        let raw = u32::MAX;
        let get_type = JointGetTypeOverride::install(raw);

        assert!(catch_unwind(AssertUnwindSafe(|| joint.joint_type())).is_err());
        assert_eq!(get_type.calls(), 1);
        assert_eq!(joint.try_joint_type(), Err(ApiError::WorldPoisoned));
        assert_eq!(get_type.calls(), 1);
    }
}
