//! Joint builders and creation helpers (modularized).
//!
//! Two creation styles are available:
//! - Scoped handles: `World::create_*_joint(&def) -> Joint` returning a scoped handle for immediate
//!   configuration/queries. Dropping the handle does **not** destroy the joint.
//! - Owned handles: `World::create_*_joint_owned(&def) -> OwnedJoint` or `World::*().build_owned() -> OwnedJoint`
//!   returning a RAII handle that destroys the joint on drop.
//! - ID style: `World::create_*_joint_id(&def) -> b2JointId` returning the raw id for storage.
//!
//! The `World` convenience builders (`revolute`, `prismatic`, `wheel`, `distance`, `weld`,
//! `motor_joint`, `filter_joint`) help compose joints in world space and build local frames
//! from world anchors/axes.

mod base;
mod base_def;
mod creation;
mod distance;
mod filter;
mod motor;
mod prismatic;
mod revolute;
mod runtime;
mod runtime_typed_distance;
mod runtime_typed_motor;
mod runtime_typed_prismatic;
mod runtime_typed_revolute;
mod runtime_typed_weld;
mod runtime_typed_wheel;
mod weld;
mod wheel;

pub use base::{ConstraintTuning, Joint, JointType, OwnedJoint};
pub use base_def::{JointBase, JointBaseBuilder};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;
use runtime::*;

pub(crate) use creation::{
    check_distance_joint_def_valid, check_filter_joint_def_valid, check_joint_base_valid,
    check_motor_joint_def_valid, check_prismatic_joint_def_valid, check_revolute_joint_def_valid,
    check_weld_joint_def_valid, check_wheel_joint_def_valid,
};

#[inline]
pub(crate) fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

#[inline]
fn assert_joint_valid(id: JointId) {
    crate::core::debug_checks::assert_joint_valid(id);
}

#[inline]
fn check_joint_valid(id: JointId) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)
}

#[inline]
fn joint_read_checked_impl<R>(id: JointId, f: impl FnOnce(JointId) -> R) -> R {
    assert_joint_valid(id);
    f(id)
}

#[inline]
fn try_joint_read_checked_impl<R>(id: JointId, f: impl FnOnce(JointId) -> R) -> ApiResult<R> {
    check_joint_valid(id)?;
    Ok(f(id))
}

#[inline]
pub(crate) fn joint_is_valid_impl(id: JointId) -> bool {
    base::joint_is_valid_impl(id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn try_joint_apis_return_in_callback() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let a = world.create_body_id(crate::BodyBuilder::new().build());
        let b = world.create_body_id(crate::BodyBuilder::new().build());

        let def = crate::DistanceJointDef::new(
            crate::JointBaseBuilder::new()
                .bodies_by_id(a, b)
                .collide_connected(false)
                .build(),
        );

        let _g = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            world.try_create_distance_joint_id(&def).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .revolute(a, b)
                .anchor_world([0.0, 0.0])
                .try_build()
                .unwrap_err(),
            crate::ApiError::InCallback
        );
    }
}
