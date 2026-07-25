//! Joint builders and creation helpers (modularized).
//!
//! Two creation styles are available:
//! - Scoped handles: `World::create_*_joint(&def) -> Joint` returning a scoped handle for immediate
//!   configuration/queries. Dropping the handle does **not** destroy the joint.
//! - Owned handles: `World::create_*_joint_owned(&def) -> OwnedJoint` or `World::*().build_owned() -> OwnedJoint`
//!   returning a RAII handle that destroys the joint on drop.
//! - ID style: `World::create_*_joint_id(&def) -> JointId` returning a world-bound branded ID for storage.
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
mod validation;
mod weld;
mod wheel;

pub use base::{ConstraintTuning, Joint, JointType, OwnedJoint};
pub use base_def::JointBase;
pub(crate) use base_def::{checked_world_axis_to_local_rotation, checked_world_to_local_point};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;
use runtime::*;
use validation::*;

pub(crate) use runtime::{JointWriteKind, JointWriteValue, try_joint_write_with_access};

pub(crate) use creation::{
    check_distance_joint_def_valid, check_filter_joint_def_valid, check_joint_base_valid,
    check_motor_joint_def_valid, check_prismatic_joint_def_valid, check_revolute_joint_def_valid,
    check_weld_joint_def_valid, check_wheel_joint_def_valid,
    try_create_distance_joint_id_with_access, try_create_filter_joint_id_with_access,
    try_create_motor_joint_id_with_access, try_create_prismatic_joint_id_with_access,
    try_create_revolute_joint_id_with_access, try_create_weld_joint_id_with_access,
    try_create_wheel_joint_id_with_access,
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
fn assert_joint_valid(core: &WorldCore, id: JointId) {
    assert_joint_access(core, id, None);
}

#[inline]
fn check_joint_valid(core: &WorldCore, id: JointId) -> ApiResult<()> {
    check_joint_access(core, id, None).map(|_| ())
}

#[inline]
fn joint_read_checked_impl<R>(core: &WorldCore, id: JointId, f: impl FnOnce(JointId) -> R) -> R {
    assert_joint_valid(core, id);
    f(id)
}

#[inline]
fn try_joint_read_checked_impl<R>(
    core: &WorldCore,
    id: JointId,
    f: impl FnOnce(JointId) -> R,
) -> ApiResult<R> {
    check_joint_valid(core, id)?;
    Ok(f(id))
}

#[cfg(test)]
mod tests {
    fn assert_panics(callback: impl FnOnce()) {
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback)).is_err(),
            "native joint default constructor accepted callback reentry"
        );
    }

    #[test]
    fn try_joint_apis_return_in_callback() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let a = world.create_body_id(crate::BodyBuilder::new().build());
        let b = world.create_body_id(crate::BodyBuilder::new().build());

        let def =
            crate::DistanceJointDef::new(crate::JointBase::new(a, b).with_collide_connected(false));

        {
            let _guard = crate::core::callback_state::CallbackGuard::enter();
            assert_eq!(
                world.try_create_distance_joint_id(&def).unwrap_err(),
                crate::ApiError::InCallback
            );
        }

        let builder = world.revolute(a, b).anchor_world([0.0, 0.0]);
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            builder.try_build().unwrap_err(),
            crate::ApiError::InCallback
        );
    }

    #[test]
    fn native_joint_defaults_reject_callback_reentry() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let a = world.create_body_id(crate::BodyBuilder::new().build());
        let b = world.create_body_id(crate::BodyBuilder::new().build());
        let base = crate::JointBase::new(a, b);

        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_panics(|| {
            let _ = crate::DistanceJointDef::new(base);
        });
        assert_panics(|| {
            let _ = crate::FilterJointDef::new(base);
        });
        assert_panics(|| {
            let _ = crate::MotorJointDef::new(base);
        });
        assert_panics(|| {
            let _ = crate::PrismaticJointDef::new(base);
        });
        assert_panics(|| {
            let _ = crate::RevoluteJointDef::new(base);
        });
        assert_panics(|| {
            let _ = crate::WeldJointDef::new(base);
        });
        assert_panics(|| {
            let _ = crate::WheelJointDef::new(base);
        });
    }
}
