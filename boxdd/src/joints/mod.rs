//! Joint definitions, creation helpers, and borrow-scoped runtime capabilities.
//!
//! Creation methods such as [`World::create_distance_joint`] return a world-bound [`JointId`]
//! suitable for application storage. Acquire a [`Joint`] with [`World::joint`] when reading,
//! mutating, converting to a typed joint capability, or explicitly destroying it. Dropping a
//! capability releases its world borrow and does not destroy the native joint.
//!
//! The `World` convenience builders (`revolute`, `prismatic`, `wheel`, `distance`, `weld`,
//! `motor_joint`, `filter_joint`) help compose joints in world space and build local frames
//! from absolute anchors and local axes. All public creation and runtime operations are fallible;
//! definitions are validated before Box2D assertions can be reached.

mod base;
mod base_def;
mod creation;
mod distance;
mod filter;
mod motor;
mod prismatic;
mod revolute;
mod runtime;
mod typed;
mod validation;
mod weld;
mod wheel;

pub use base::{ConstraintTuning, Joint, JointType};
pub use base_def::JointBase;
pub(crate) use base_def::{checked_world_axis_to_local_rotation, checked_world_to_local_point};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use typed::{
    DistanceJoint, FilterJoint, MotorJoint, PrismaticJoint, RevoluteJoint, WeldJoint, WheelJoint,
};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use crate::error::Result;
use crate::types::{BodyId, JointId};
use crate::world::World;
use boxdd_sys::ffi;
pub(crate) use validation::*;

pub(crate) use creation::{
    check_distance_joint_def_valid, check_filter_joint_def_valid, check_joint_base_valid,
    check_motor_joint_def_valid, check_prismatic_joint_def_valid, check_revolute_joint_def_valid,
    check_weld_joint_def_valid, check_wheel_joint_def_valid, create_distance_joint_id,
    create_filter_joint_id, create_motor_joint_id, create_prismatic_joint_id,
    create_revolute_joint_id, create_weld_joint_id, create_wheel_joint_id,
};

#[inline]
pub(crate) fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

#[cfg(test)]
mod tests {
    #[test]
    fn joint_apis_return_in_callback() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let def = crate::DistanceJointDef::new(
            world
                .joint_base(a, b)
                .unwrap()
                .with_collide_connected(false),
        );

        {
            let _guard = crate::core::callback_state::CallbackGuard::enter();
            assert_eq!(
                world.create_distance_joint(&def).unwrap_err(),
                crate::Error::InCallback
            );
        }

        let builder = world.revolute(a, b).anchor_world([0.0, 0.0]);
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(builder.build().unwrap_err(), crate::Error::InCallback);
    }

    #[test]
    fn cached_joint_defaults_are_callback_safe() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let base = world.joint_base(a, b).unwrap();

        let _guard = crate::core::callback_state::CallbackGuard::enter();
        let _ = crate::DistanceJointDef::new(base);
        let _ = crate::FilterJointDef::new(base);
        let _ = crate::MotorJointDef::new(base);
        let _ = crate::PrismaticJointDef::new(base);
        let _ = crate::RevoluteJointDef::new(base);
        let _ = crate::WeldJointDef::new(base);
        let _ = crate::WheelJointDef::new(base);
    }
}
