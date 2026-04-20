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
mod distance;
mod filter;
mod motor;
mod prismatic;
mod revolute;
mod weld;
mod wheel;

pub use base::{ConstraintTuning, Joint, JointBase, JointBaseBuilder, JointType, OwnedJoint};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

#[inline]
fn assert_joint_valid(id: JointId) {
    crate::core::debug_checks::assert_joint_valid(id);
}

#[inline]
fn check_joint_valid(id: JointId) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)
}

#[inline]
fn assert_joint_def_bodies_valid(base: &ffi::b2JointDef) {
    crate::core::debug_checks::assert_body_valid(base.bodyIdA);
    crate::core::debug_checks::assert_body_valid(base.bodyIdB);
}

#[inline]
fn check_joint_def_bodies_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(base.bodyIdA)?;
    crate::core::debug_checks::check_body_valid(base.bodyIdB)?;
    Ok(())
}

macro_rules! impl_world_joint_create_methods {
    ($(
        $scoped:ident,
        $id:ident,
        $owned:ident,
        $try_scoped:ident,
        $try_id:ident,
        $try_owned:ident,
        $def_ty:ty,
        $create:path;
    )+) => {
        $(
            pub fn $scoped<'w>(&'w mut self, def: &$def_ty) -> Joint<'w> {
                crate::core::callback_state::assert_not_in_callback();
                assert_joint_def_bodies_valid(&def.0.base);
                let id = unsafe { $create(self.raw(), &def.0) };
                Joint::new(self.core_arc(), id)
            }

            pub fn $id(&mut self, def: &$def_ty) -> JointId {
                crate::core::callback_state::assert_not_in_callback();
                assert_joint_def_bodies_valid(&def.0.base);
                unsafe { $create(self.raw(), &def.0) }
            }

            pub fn $owned(&mut self, def: &$def_ty) -> OwnedJoint {
                let id = self.$id(def);
                OwnedJoint::new(self.core_arc(), id)
            }

            pub fn $try_scoped<'w>(&'w mut self, def: &$def_ty) -> ApiResult<Joint<'w>> {
                crate::core::callback_state::check_not_in_callback()?;
                check_joint_def_bodies_valid(&def.0.base)?;
                let id = unsafe { $create(self.raw(), &def.0) };
                Ok(Joint::new(self.core_arc(), id))
            }

            pub fn $try_id(&mut self, def: &$def_ty) -> ApiResult<JointId> {
                crate::core::callback_state::check_not_in_callback()?;
                check_joint_def_bodies_valid(&def.0.base)?;
                Ok(unsafe { $create(self.raw(), &def.0) })
            }

            pub fn $try_owned(&mut self, def: &$def_ty) -> ApiResult<OwnedJoint> {
                let id = self.$try_id(def)?;
                Ok(OwnedJoint::new(self.core_arc(), id))
            }
        )+
    };
}

// Convenience builder entry points on World
impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        RevoluteJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: RevoluteJointDef::new(JointBase::default()),
        }
    }
    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        PrismaticJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(JointBase::default()),
        }
    }
    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        WheelJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(JointBase::default()),
        }
    }
    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        DistanceJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(JointBase::default()),
        }
    }
    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        WeldJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: WeldJointDef::new(JointBase::default()),
        }
    }
    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        MotorJointBuilder {
            world: self,
            body_a,
            body_b,
            def: MotorJointDef::new(JointBase::default()),
        }
    }
    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        FilterJointBuilder {
            world: self,
            body_a,
            body_b,
            def: FilterJointDef::new(JointBase::default()),
        }
    }
}

// Creation/destroy: scoped handles and ID style
impl World {
    impl_world_joint_create_methods! {
        create_distance_joint,
        create_distance_joint_id,
        create_distance_joint_owned,
        try_create_distance_joint,
        try_create_distance_joint_id,
        try_create_distance_joint_owned,
        DistanceJointDef,
        ffi::b2CreateDistanceJoint;
        create_revolute_joint,
        create_revolute_joint_id,
        create_revolute_joint_owned,
        try_create_revolute_joint,
        try_create_revolute_joint_id,
        try_create_revolute_joint_owned,
        RevoluteJointDef,
        ffi::b2CreateRevoluteJoint;
        create_prismatic_joint,
        create_prismatic_joint_id,
        create_prismatic_joint_owned,
        try_create_prismatic_joint,
        try_create_prismatic_joint_id,
        try_create_prismatic_joint_owned,
        PrismaticJointDef,
        ffi::b2CreatePrismaticJoint;
        create_wheel_joint,
        create_wheel_joint_id,
        create_wheel_joint_owned,
        try_create_wheel_joint,
        try_create_wheel_joint_id,
        try_create_wheel_joint_owned,
        WheelJointDef,
        ffi::b2CreateWheelJoint;
        create_weld_joint,
        create_weld_joint_id,
        create_weld_joint_owned,
        try_create_weld_joint,
        try_create_weld_joint_id,
        try_create_weld_joint_owned,
        WeldJointDef,
        ffi::b2CreateWeldJoint;
        create_motor_joint,
        create_motor_joint_id,
        create_motor_joint_owned,
        try_create_motor_joint,
        try_create_motor_joint_id,
        try_create_motor_joint_owned,
        MotorJointDef,
        ffi::b2CreateMotorJoint;
        create_filter_joint,
        create_filter_joint_id,
        create_filter_joint_owned,
        try_create_filter_joint,
        try_create_filter_joint_id,
        try_create_filter_joint_owned,
        FilterJointDef,
        ffi::b2CreateFilterJoint;
    }

    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(id) } {
            unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
            let _ = self.core_arc().clear_joint_user_data(id);
        }
    }

    pub fn try_destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
        let _ = self.core_arc().clear_joint_user_data(id);
        Ok(())
    }
}

// Runtime joint control APIs (by joint type)
impl World {
    pub fn joint_type(&self, id: JointId) -> JointType {
        assert_joint_valid(id);
        base::joint_type_impl(id)
    }

    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        check_joint_valid(id)?;
        Ok(base::joint_type_impl(id))
    }

    pub fn joint_type_raw(&self, id: JointId) -> ffi::b2JointType {
        assert_joint_valid(id);
        base::joint_type_raw_impl(id)
    }

    pub fn try_joint_type_raw(&self, id: JointId) -> ApiResult<ffi::b2JointType> {
        check_joint_valid(id)?;
        Ok(base::joint_type_raw_impl(id))
    }

    pub fn joint_body_a_id(&self, id: JointId) -> BodyId {
        assert_joint_valid(id);
        base::joint_body_a_id_impl(id)
    }

    pub fn try_joint_body_a_id(&self, id: JointId) -> ApiResult<BodyId> {
        check_joint_valid(id)?;
        Ok(base::joint_body_a_id_impl(id))
    }

    pub fn joint_body_b_id(&self, id: JointId) -> BodyId {
        assert_joint_valid(id);
        base::joint_body_b_id_impl(id)
    }

    pub fn try_joint_body_b_id(&self, id: JointId) -> ApiResult<BodyId> {
        check_joint_valid(id)?;
        Ok(base::joint_body_b_id_impl(id))
    }

    pub fn joint_collide_connected(&self, id: JointId) -> bool {
        assert_joint_valid(id);
        base::joint_collide_connected_impl(id)
    }

    pub fn try_joint_collide_connected(&self, id: JointId) -> ApiResult<bool> {
        check_joint_valid(id)?;
        Ok(base::joint_collide_connected_impl(id))
    }

    pub fn set_joint_collide_connected(&mut self, id: JointId, flag: bool) {
        assert_joint_valid(id);
        base::joint_set_collide_connected_impl(id, flag)
    }

    pub fn try_set_joint_collide_connected(&mut self, id: JointId, flag: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_set_collide_connected_impl(id, flag);
        Ok(())
    }

    pub fn joint_constraint_tuning(&self, id: JointId) -> ConstraintTuning {
        assert_joint_valid(id);
        base::joint_constraint_tuning_impl(id)
    }

    pub fn try_joint_constraint_tuning(&self, id: JointId) -> ApiResult<ConstraintTuning> {
        check_joint_valid(id)?;
        Ok(base::joint_constraint_tuning_impl(id))
    }

    pub fn set_joint_constraint_tuning(&mut self, id: JointId, tuning: ConstraintTuning) {
        assert_joint_valid(id);
        base::joint_set_constraint_tuning_impl(id, tuning)
    }

    pub fn try_set_joint_constraint_tuning(
        &mut self,
        id: JointId,
        tuning: ConstraintTuning,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_set_constraint_tuning_impl(id, tuning);
        Ok(())
    }

    pub fn joint_local_frame_a(&self, id: JointId) -> crate::Transform {
        assert_joint_valid(id);
        base::joint_local_frame_a_impl(id)
    }

    pub fn try_joint_local_frame_a(&self, id: JointId) -> ApiResult<crate::Transform> {
        check_joint_valid(id)?;
        Ok(base::joint_local_frame_a_impl(id))
    }

    pub fn joint_local_frame_b(&self, id: JointId) -> crate::Transform {
        assert_joint_valid(id);
        base::joint_local_frame_b_impl(id)
    }

    pub fn try_joint_local_frame_b(&self, id: JointId) -> ApiResult<crate::Transform> {
        check_joint_valid(id)?;
        Ok(base::joint_local_frame_b_impl(id))
    }

    pub fn joint_wake_bodies(&mut self, id: JointId) {
        assert_joint_valid(id);
        base::joint_wake_bodies_impl(id)
    }

    pub fn try_joint_wake_bodies(&mut self, id: JointId) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_wake_bodies_impl(id);
        Ok(())
    }

    pub fn joint_linear_separation(&self, id: JointId) -> f32 {
        assert_joint_valid(id);
        base::joint_linear_separation_impl(id)
    }

    pub fn try_joint_linear_separation(&self, id: JointId) -> ApiResult<f32> {
        check_joint_valid(id)?;
        Ok(base::joint_linear_separation_impl(id))
    }

    pub fn joint_angular_separation(&self, id: JointId) -> f32 {
        assert_joint_valid(id);
        base::joint_angular_separation_impl(id)
    }

    pub fn try_joint_angular_separation(&self, id: JointId) -> ApiResult<f32> {
        check_joint_valid(id)?;
        Ok(base::joint_angular_separation_impl(id))
    }

    pub fn joint_constraint_force(&self, id: JointId) -> Vec2 {
        assert_joint_valid(id);
        base::joint_constraint_force_impl(id)
    }

    pub fn try_joint_constraint_force(&self, id: JointId) -> ApiResult<Vec2> {
        check_joint_valid(id)?;
        Ok(base::joint_constraint_force_impl(id))
    }

    pub fn joint_constraint_torque(&self, id: JointId) -> f32 {
        assert_joint_valid(id);
        base::joint_constraint_torque_impl(id)
    }

    pub fn try_joint_constraint_torque(&self, id: JointId) -> ApiResult<f32> {
        check_joint_valid(id)?;
        Ok(base::joint_constraint_torque_impl(id))
    }
}

#[inline]
fn assert_joint_kind(id: JointId, expected: JointType) {
    assert_joint_valid(id);
    let actual = base::joint_type_impl(id);
    assert!(
        actual == expected,
        "joint type mismatch: expected {:?}, got {:?}",
        expected,
        actual
    );
}

#[inline]
fn check_joint_kind(id: JointId, expected: JointType) -> ApiResult<()> {
    check_joint_valid(id)?;
    if base::joint_type_impl(id) != expected {
        return Err(crate::error::ApiError::InvalidJointType);
    }
    Ok(())
}

macro_rules! joint_scalar_get_impl {
    ($name:ident, $ffi_get:path, $ty:ty) => {
        #[inline]
        fn $name(id: JointId) -> $ty {
            unsafe { $ffi_get(id) }
        }
    };
}

macro_rules! joint_scalar_set_impl {
    ($name:ident, $ffi_set:path, $ty:ty) => {
        #[inline]
        fn $name(id: JointId, value: $ty) {
            unsafe { $ffi_set(id, value) }
        }
    };
}

macro_rules! joint_scalar_rw_impl {
    ($get_name:ident, $set_name:ident, $ffi_get:path, $ffi_set:path, $ty:ty) => {
        joint_scalar_get_impl!($get_name, $ffi_get, $ty);
        joint_scalar_set_impl!($set_name, $ffi_set, $ty);
    };
}

macro_rules! joint_vec2_get_impl {
    ($name:ident, $ffi_get:path) => {
        #[inline]
        fn $name(id: JointId) -> Vec2 {
            Vec2::from(unsafe { $ffi_get(id) })
        }
    };
}

macro_rules! joint_scalar_getter_triple {
    ($name:ident, $try_name:ident, $kind:expr, $impl_fn:ident, $ty:ty) => {
        impl World {
            pub fn $name(&self, id: JointId) -> $ty {
                assert_joint_kind(id, $kind);
                $impl_fn(id)
            }

            pub fn $try_name(&self, id: JointId) -> ApiResult<$ty> {
                check_joint_kind(id, $kind)?;
                Ok($impl_fn(id))
            }
        }

        impl OwnedJoint {
            pub fn $name(&self) -> $ty {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id)
            }

            pub fn $try_name(&self) -> ApiResult<$ty> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                Ok($impl_fn(id))
            }
        }

        impl<'w> Joint<'w> {
            pub fn $name(&self) -> $ty {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id)
            }

            pub fn $try_name(&self) -> ApiResult<$ty> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                Ok($impl_fn(id))
            }
        }
    };
}

macro_rules! joint_scalar_setter_triple {
    ($name:ident, $try_name:ident, $kind:expr, $impl_fn:ident, $arg:ident : $ty:ty) => {
        impl World {
            pub fn $name(&mut self, id: JointId, $arg: $ty) {
                assert_joint_kind(id, $kind);
                $impl_fn(id, $arg)
            }

            pub fn $try_name(&mut self, id: JointId, $arg: $ty) -> ApiResult<()> {
                check_joint_kind(id, $kind)?;
                $impl_fn(id, $arg);
                Ok(())
            }
        }

        impl OwnedJoint {
            pub fn $name(&mut self, $arg: $ty) {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id, $arg)
            }

            pub fn $try_name(&mut self, $arg: $ty) -> ApiResult<()> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                $impl_fn(id, $arg);
                Ok(())
            }
        }

        impl<'w> Joint<'w> {
            pub fn $name(&mut self, $arg: $ty) {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id, $arg)
            }

            pub fn $try_name(&mut self, $arg: $ty) -> ApiResult<()> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                $impl_fn(id, $arg);
                Ok(())
            }
        }
    };
}

macro_rules! joint_two_arg_setter_triple {
    (
        $name:ident,
        $try_name:ident,
        $kind:expr,
        $impl_fn:ident,
        $arg_a:ident : $ty_a:ty,
        $arg_b:ident : $ty_b:ty
    ) => {
        impl World {
            pub fn $name(&mut self, id: JointId, $arg_a: $ty_a, $arg_b: $ty_b) {
                assert_joint_kind(id, $kind);
                $impl_fn(id, $arg_a, $arg_b)
            }

            pub fn $try_name(
                &mut self,
                id: JointId,
                $arg_a: $ty_a,
                $arg_b: $ty_b,
            ) -> ApiResult<()> {
                check_joint_kind(id, $kind)?;
                $impl_fn(id, $arg_a, $arg_b);
                Ok(())
            }
        }

        impl OwnedJoint {
            pub fn $name(&mut self, $arg_a: $ty_a, $arg_b: $ty_b) {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id, $arg_a, $arg_b)
            }

            pub fn $try_name(&mut self, $arg_a: $ty_a, $arg_b: $ty_b) -> ApiResult<()> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                $impl_fn(id, $arg_a, $arg_b);
                Ok(())
            }
        }

        impl<'w> Joint<'w> {
            pub fn $name(&mut self, $arg_a: $ty_a, $arg_b: $ty_b) {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id, $arg_a, $arg_b)
            }

            pub fn $try_name(&mut self, $arg_a: $ty_a, $arg_b: $ty_b) -> ApiResult<()> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                $impl_fn(id, $arg_a, $arg_b);
                Ok(())
            }
        }
    };
}

macro_rules! joint_vec2_setter_triple {
    ($name:ident, $try_name:ident, $kind:expr, $impl_fn:ident) => {
        impl World {
            pub fn $name<V: Into<crate::types::Vec2>>(&mut self, id: JointId, v: V) {
                assert_joint_kind(id, $kind);
                $impl_fn(id, v.into())
            }

            pub fn $try_name<V: Into<crate::types::Vec2>>(
                &mut self,
                id: JointId,
                v: V,
            ) -> ApiResult<()> {
                check_joint_kind(id, $kind)?;
                $impl_fn(id, v.into());
                Ok(())
            }
        }

        impl OwnedJoint {
            pub fn $name<V: Into<crate::types::Vec2>>(&mut self, v: V) {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id, v.into())
            }

            pub fn $try_name<V: Into<crate::types::Vec2>>(&mut self, v: V) -> ApiResult<()> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                $impl_fn(id, v.into());
                Ok(())
            }
        }

        impl<'w> Joint<'w> {
            pub fn $name<V: Into<crate::types::Vec2>>(&mut self, v: V) {
                let id = self.id();
                assert_joint_kind(id, $kind);
                $impl_fn(id, v.into())
            }

            pub fn $try_name<V: Into<crate::types::Vec2>>(&mut self, v: V) -> ApiResult<()> {
                let id = self.id();
                check_joint_kind(id, $kind)?;
                $impl_fn(id, v.into());
                Ok(())
            }
        }
    };
}

joint_scalar_rw_impl!(
    distance_length_impl,
    distance_set_length_impl,
    ffi::b2DistanceJoint_GetLength,
    ffi::b2DistanceJoint_SetLength,
    f32
);
joint_scalar_rw_impl!(
    distance_spring_enabled_impl,
    distance_enable_spring_impl,
    ffi::b2DistanceJoint_IsSpringEnabled,
    ffi::b2DistanceJoint_EnableSpring,
    bool
);
#[inline]
fn distance_spring_force_range_impl(id: JointId) -> (f32, f32) {
    let mut lower_force = 0.0f32;
    let mut upper_force = 0.0f32;
    unsafe { ffi::b2DistanceJoint_GetSpringForceRange(id, &mut lower_force, &mut upper_force) };
    (lower_force, upper_force)
}
#[inline]
fn distance_lower_spring_force_impl(id: JointId) -> f32 {
    distance_spring_force_range_impl(id).0
}
#[inline]
fn distance_upper_spring_force_impl(id: JointId) -> f32 {
    distance_spring_force_range_impl(id).1
}
#[inline]
fn distance_set_spring_force_range_impl(id: JointId, lower_force: f32, upper_force: f32) {
    unsafe { ffi::b2DistanceJoint_SetSpringForceRange(id, lower_force, upper_force) }
}
joint_scalar_rw_impl!(
    distance_spring_hertz_impl,
    distance_set_spring_hertz_impl,
    ffi::b2DistanceJoint_GetSpringHertz,
    ffi::b2DistanceJoint_SetSpringHertz,
    f32
);
joint_scalar_rw_impl!(
    distance_spring_damping_ratio_impl,
    distance_set_spring_damping_ratio_impl,
    ffi::b2DistanceJoint_GetSpringDampingRatio,
    ffi::b2DistanceJoint_SetSpringDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    distance_limit_enabled_impl,
    distance_enable_limit_impl,
    ffi::b2DistanceJoint_IsLimitEnabled,
    ffi::b2DistanceJoint_EnableLimit,
    bool
);
joint_scalar_get_impl!(
    distance_min_length_impl,
    ffi::b2DistanceJoint_GetMinLength,
    f32
);
joint_scalar_get_impl!(
    distance_max_length_impl,
    ffi::b2DistanceJoint_GetMaxLength,
    f32
);
joint_scalar_get_impl!(
    distance_current_length_impl,
    ffi::b2DistanceJoint_GetCurrentLength,
    f32
);
#[inline]
fn distance_set_length_range_impl(id: JointId, min_length: f32, max_length: f32) {
    unsafe { ffi::b2DistanceJoint_SetLengthRange(id, min_length, max_length) }
}
joint_scalar_rw_impl!(
    distance_motor_enabled_impl,
    distance_enable_motor_impl,
    ffi::b2DistanceJoint_IsMotorEnabled,
    ffi::b2DistanceJoint_EnableMotor,
    bool
);
joint_scalar_rw_impl!(
    distance_motor_speed_impl,
    distance_set_motor_speed_impl,
    ffi::b2DistanceJoint_GetMotorSpeed,
    ffi::b2DistanceJoint_SetMotorSpeed,
    f32
);
joint_scalar_rw_impl!(
    distance_max_motor_force_impl,
    distance_set_max_motor_force_impl,
    ffi::b2DistanceJoint_GetMaxMotorForce,
    ffi::b2DistanceJoint_SetMaxMotorForce,
    f32
);
joint_scalar_get_impl!(
    distance_motor_force_impl,
    ffi::b2DistanceJoint_GetMotorForce,
    f32
);

joint_scalar_rw_impl!(
    prismatic_spring_enabled_impl,
    prismatic_enable_spring_impl,
    ffi::b2PrismaticJoint_IsSpringEnabled,
    ffi::b2PrismaticJoint_EnableSpring,
    bool
);
joint_scalar_rw_impl!(
    prismatic_spring_hertz_impl,
    prismatic_set_spring_hertz_impl,
    ffi::b2PrismaticJoint_GetSpringHertz,
    ffi::b2PrismaticJoint_SetSpringHertz,
    f32
);
joint_scalar_rw_impl!(
    prismatic_spring_damping_ratio_impl,
    prismatic_set_spring_damping_ratio_impl,
    ffi::b2PrismaticJoint_GetSpringDampingRatio,
    ffi::b2PrismaticJoint_SetSpringDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    prismatic_target_translation_impl,
    prismatic_set_target_translation_impl,
    ffi::b2PrismaticJoint_GetTargetTranslation,
    ffi::b2PrismaticJoint_SetTargetTranslation,
    f32
);
joint_scalar_rw_impl!(
    prismatic_limit_enabled_impl,
    prismatic_enable_limit_impl,
    ffi::b2PrismaticJoint_IsLimitEnabled,
    ffi::b2PrismaticJoint_EnableLimit,
    bool
);
joint_scalar_get_impl!(
    prismatic_lower_limit_impl,
    ffi::b2PrismaticJoint_GetLowerLimit,
    f32
);
joint_scalar_get_impl!(
    prismatic_upper_limit_impl,
    ffi::b2PrismaticJoint_GetUpperLimit,
    f32
);
#[inline]
fn prismatic_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2PrismaticJoint_SetLimits(id, lower, upper) }
}
joint_scalar_rw_impl!(
    prismatic_motor_enabled_impl,
    prismatic_enable_motor_impl,
    ffi::b2PrismaticJoint_IsMotorEnabled,
    ffi::b2PrismaticJoint_EnableMotor,
    bool
);
joint_scalar_rw_impl!(
    prismatic_motor_speed_impl,
    prismatic_set_motor_speed_impl,
    ffi::b2PrismaticJoint_GetMotorSpeed,
    ffi::b2PrismaticJoint_SetMotorSpeed,
    f32
);
joint_scalar_rw_impl!(
    prismatic_max_motor_force_impl,
    prismatic_set_max_motor_force_impl,
    ffi::b2PrismaticJoint_GetMaxMotorForce,
    ffi::b2PrismaticJoint_SetMaxMotorForce,
    f32
);
joint_scalar_get_impl!(
    prismatic_motor_force_impl,
    ffi::b2PrismaticJoint_GetMotorForce,
    f32
);
joint_scalar_get_impl!(
    prismatic_translation_impl,
    ffi::b2PrismaticJoint_GetTranslation,
    f32
);
joint_scalar_get_impl!(prismatic_speed_impl, ffi::b2PrismaticJoint_GetSpeed, f32);

joint_scalar_rw_impl!(
    revolute_spring_enabled_impl,
    revolute_enable_spring_impl,
    ffi::b2RevoluteJoint_IsSpringEnabled,
    ffi::b2RevoluteJoint_EnableSpring,
    bool
);
joint_scalar_rw_impl!(
    revolute_spring_hertz_impl,
    revolute_set_spring_hertz_impl,
    ffi::b2RevoluteJoint_GetSpringHertz,
    ffi::b2RevoluteJoint_SetSpringHertz,
    f32
);
joint_scalar_rw_impl!(
    revolute_spring_damping_ratio_impl,
    revolute_set_spring_damping_ratio_impl,
    ffi::b2RevoluteJoint_GetSpringDampingRatio,
    ffi::b2RevoluteJoint_SetSpringDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    revolute_target_angle_impl,
    revolute_set_target_angle_impl,
    ffi::b2RevoluteJoint_GetTargetAngle,
    ffi::b2RevoluteJoint_SetTargetAngle,
    f32
);
joint_scalar_get_impl!(revolute_angle_impl, ffi::b2RevoluteJoint_GetAngle, f32);
joint_scalar_rw_impl!(
    revolute_limit_enabled_impl,
    revolute_enable_limit_impl,
    ffi::b2RevoluteJoint_IsLimitEnabled,
    ffi::b2RevoluteJoint_EnableLimit,
    bool
);
joint_scalar_get_impl!(
    revolute_lower_limit_impl,
    ffi::b2RevoluteJoint_GetLowerLimit,
    f32
);
joint_scalar_get_impl!(
    revolute_upper_limit_impl,
    ffi::b2RevoluteJoint_GetUpperLimit,
    f32
);
#[inline]
fn revolute_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2RevoluteJoint_SetLimits(id, lower, upper) }
}
joint_scalar_rw_impl!(
    revolute_motor_enabled_impl,
    revolute_enable_motor_impl,
    ffi::b2RevoluteJoint_IsMotorEnabled,
    ffi::b2RevoluteJoint_EnableMotor,
    bool
);
joint_scalar_rw_impl!(
    revolute_motor_speed_impl,
    revolute_set_motor_speed_impl,
    ffi::b2RevoluteJoint_GetMotorSpeed,
    ffi::b2RevoluteJoint_SetMotorSpeed,
    f32
);
joint_scalar_get_impl!(
    revolute_motor_torque_impl,
    ffi::b2RevoluteJoint_GetMotorTorque,
    f32
);
joint_scalar_rw_impl!(
    revolute_max_motor_torque_impl,
    revolute_set_max_motor_torque_impl,
    ffi::b2RevoluteJoint_GetMaxMotorTorque,
    ffi::b2RevoluteJoint_SetMaxMotorTorque,
    f32
);

joint_scalar_rw_impl!(
    weld_linear_hertz_impl,
    weld_set_linear_hertz_impl,
    ffi::b2WeldJoint_GetLinearHertz,
    ffi::b2WeldJoint_SetLinearHertz,
    f32
);
joint_scalar_rw_impl!(
    weld_linear_damping_ratio_impl,
    weld_set_linear_damping_ratio_impl,
    ffi::b2WeldJoint_GetLinearDampingRatio,
    ffi::b2WeldJoint_SetLinearDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    weld_angular_hertz_impl,
    weld_set_angular_hertz_impl,
    ffi::b2WeldJoint_GetAngularHertz,
    ffi::b2WeldJoint_SetAngularHertz,
    f32
);
joint_scalar_rw_impl!(
    weld_angular_damping_ratio_impl,
    weld_set_angular_damping_ratio_impl,
    ffi::b2WeldJoint_GetAngularDampingRatio,
    ffi::b2WeldJoint_SetAngularDampingRatio,
    f32
);

joint_scalar_rw_impl!(
    wheel_spring_enabled_impl,
    wheel_enable_spring_impl,
    ffi::b2WheelJoint_IsSpringEnabled,
    ffi::b2WheelJoint_EnableSpring,
    bool
);
joint_scalar_rw_impl!(
    wheel_spring_hertz_impl,
    wheel_set_spring_hertz_impl,
    ffi::b2WheelJoint_GetSpringHertz,
    ffi::b2WheelJoint_SetSpringHertz,
    f32
);
joint_scalar_rw_impl!(
    wheel_spring_damping_ratio_impl,
    wheel_set_spring_damping_ratio_impl,
    ffi::b2WheelJoint_GetSpringDampingRatio,
    ffi::b2WheelJoint_SetSpringDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    wheel_limit_enabled_impl,
    wheel_enable_limit_impl,
    ffi::b2WheelJoint_IsLimitEnabled,
    ffi::b2WheelJoint_EnableLimit,
    bool
);
joint_scalar_get_impl!(wheel_lower_limit_impl, ffi::b2WheelJoint_GetLowerLimit, f32);
joint_scalar_get_impl!(wheel_upper_limit_impl, ffi::b2WheelJoint_GetUpperLimit, f32);
#[inline]
fn wheel_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2WheelJoint_SetLimits(id, lower, upper) }
}
joint_scalar_rw_impl!(
    wheel_motor_enabled_impl,
    wheel_enable_motor_impl,
    ffi::b2WheelJoint_IsMotorEnabled,
    ffi::b2WheelJoint_EnableMotor,
    bool
);
joint_scalar_rw_impl!(
    wheel_motor_speed_impl,
    wheel_set_motor_speed_impl,
    ffi::b2WheelJoint_GetMotorSpeed,
    ffi::b2WheelJoint_SetMotorSpeed,
    f32
);
joint_scalar_get_impl!(
    wheel_motor_torque_impl,
    ffi::b2WheelJoint_GetMotorTorque,
    f32
);
joint_scalar_rw_impl!(
    wheel_max_motor_torque_impl,
    wheel_set_max_motor_torque_impl,
    ffi::b2WheelJoint_GetMaxMotorTorque,
    ffi::b2WheelJoint_SetMaxMotorTorque,
    f32
);

joint_vec2_get_impl!(
    motor_linear_velocity_impl,
    ffi::b2MotorJoint_GetLinearVelocity
);
#[inline]
fn motor_set_linear_velocity_impl(id: JointId, value: Vec2) {
    let raw: ffi::b2Vec2 = value.into();
    unsafe { ffi::b2MotorJoint_SetLinearVelocity(id, raw) }
}
joint_scalar_rw_impl!(
    motor_angular_velocity_impl,
    motor_set_angular_velocity_impl,
    ffi::b2MotorJoint_GetAngularVelocity,
    ffi::b2MotorJoint_SetAngularVelocity,
    f32
);
joint_scalar_rw_impl!(
    motor_max_velocity_force_impl,
    motor_set_max_velocity_force_impl,
    ffi::b2MotorJoint_GetMaxVelocityForce,
    ffi::b2MotorJoint_SetMaxVelocityForce,
    f32
);
joint_scalar_rw_impl!(
    motor_max_velocity_torque_impl,
    motor_set_max_velocity_torque_impl,
    ffi::b2MotorJoint_GetMaxVelocityTorque,
    ffi::b2MotorJoint_SetMaxVelocityTorque,
    f32
);
joint_scalar_rw_impl!(
    motor_linear_hertz_impl,
    motor_set_linear_hertz_impl,
    ffi::b2MotorJoint_GetLinearHertz,
    ffi::b2MotorJoint_SetLinearHertz,
    f32
);
joint_scalar_rw_impl!(
    motor_linear_damping_ratio_impl,
    motor_set_linear_damping_ratio_impl,
    ffi::b2MotorJoint_GetLinearDampingRatio,
    ffi::b2MotorJoint_SetLinearDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    motor_angular_hertz_impl,
    motor_set_angular_hertz_impl,
    ffi::b2MotorJoint_GetAngularHertz,
    ffi::b2MotorJoint_SetAngularHertz,
    f32
);
joint_scalar_rw_impl!(
    motor_angular_damping_ratio_impl,
    motor_set_angular_damping_ratio_impl,
    ffi::b2MotorJoint_GetAngularDampingRatio,
    ffi::b2MotorJoint_SetAngularDampingRatio,
    f32
);
joint_scalar_rw_impl!(
    motor_max_spring_force_impl,
    motor_set_max_spring_force_impl,
    ffi::b2MotorJoint_GetMaxSpringForce,
    ffi::b2MotorJoint_SetMaxSpringForce,
    f32
);
joint_scalar_rw_impl!(
    motor_max_spring_torque_impl,
    motor_set_max_spring_torque_impl,
    ffi::b2MotorJoint_GetMaxSpringTorque,
    ffi::b2MotorJoint_SetMaxSpringTorque,
    f32
);

joint_scalar_getter_triple!(
    distance_length,
    try_distance_length,
    JointType::Distance,
    distance_length_impl,
    f32
);
joint_scalar_setter_triple!(
    distance_set_length,
    try_distance_set_length,
    JointType::Distance,
    distance_set_length_impl,
    length: f32
);
joint_scalar_getter_triple!(
    distance_spring_enabled,
    try_distance_spring_enabled,
    JointType::Distance,
    distance_spring_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    distance_enable_spring,
    try_distance_enable_spring,
    JointType::Distance,
    distance_enable_spring_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    distance_lower_spring_force,
    try_distance_lower_spring_force,
    JointType::Distance,
    distance_lower_spring_force_impl,
    f32
);
joint_scalar_getter_triple!(
    distance_upper_spring_force,
    try_distance_upper_spring_force,
    JointType::Distance,
    distance_upper_spring_force_impl,
    f32
);
joint_two_arg_setter_triple!(
    distance_set_spring_force_range,
    try_distance_set_spring_force_range,
    JointType::Distance,
    distance_set_spring_force_range_impl,
    lower_force: f32,
    upper_force: f32
);
joint_scalar_getter_triple!(
    distance_spring_hertz,
    try_distance_spring_hertz,
    JointType::Distance,
    distance_spring_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    distance_set_spring_hertz,
    try_distance_set_spring_hertz,
    JointType::Distance,
    distance_set_spring_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    distance_spring_damping_ratio,
    try_distance_spring_damping_ratio,
    JointType::Distance,
    distance_spring_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    distance_set_spring_damping_ratio,
    try_distance_set_spring_damping_ratio,
    JointType::Distance,
    distance_set_spring_damping_ratio_impl,
    damping_ratio: f32
);
joint_scalar_getter_triple!(
    distance_limit_enabled,
    try_distance_limit_enabled,
    JointType::Distance,
    distance_limit_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    distance_enable_limit,
    try_distance_enable_limit,
    JointType::Distance,
    distance_enable_limit_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    distance_min_length,
    try_distance_min_length,
    JointType::Distance,
    distance_min_length_impl,
    f32
);
joint_scalar_getter_triple!(
    distance_max_length,
    try_distance_max_length,
    JointType::Distance,
    distance_max_length_impl,
    f32
);
joint_scalar_getter_triple!(
    distance_current_length,
    try_distance_current_length,
    JointType::Distance,
    distance_current_length_impl,
    f32
);
joint_two_arg_setter_triple!(
    distance_set_length_range,
    try_distance_set_length_range,
    JointType::Distance,
    distance_set_length_range_impl,
    min_length: f32,
    max_length: f32
);
joint_scalar_getter_triple!(
    distance_motor_enabled,
    try_distance_motor_enabled,
    JointType::Distance,
    distance_motor_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    distance_enable_motor,
    try_distance_enable_motor,
    JointType::Distance,
    distance_enable_motor_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    distance_motor_speed,
    try_distance_motor_speed,
    JointType::Distance,
    distance_motor_speed_impl,
    f32
);
joint_scalar_setter_triple!(
    distance_set_motor_speed,
    try_distance_set_motor_speed,
    JointType::Distance,
    distance_set_motor_speed_impl,
    speed: f32
);
joint_scalar_getter_triple!(
    distance_max_motor_force,
    try_distance_max_motor_force,
    JointType::Distance,
    distance_max_motor_force_impl,
    f32
);
joint_scalar_setter_triple!(
    distance_set_max_motor_force,
    try_distance_set_max_motor_force,
    JointType::Distance,
    distance_set_max_motor_force_impl,
    force: f32
);
joint_scalar_getter_triple!(
    distance_motor_force,
    try_distance_motor_force,
    JointType::Distance,
    distance_motor_force_impl,
    f32
);

joint_scalar_getter_triple!(
    prismatic_spring_enabled,
    try_prismatic_spring_enabled,
    JointType::Prismatic,
    prismatic_spring_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    prismatic_enable_spring,
    try_prismatic_enable_spring,
    JointType::Prismatic,
    prismatic_enable_spring_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    prismatic_spring_hertz,
    try_prismatic_spring_hertz,
    JointType::Prismatic,
    prismatic_spring_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    prismatic_set_spring_hertz,
    try_prismatic_set_spring_hertz,
    JointType::Prismatic,
    prismatic_set_spring_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    prismatic_spring_damping_ratio,
    try_prismatic_spring_damping_ratio,
    JointType::Prismatic,
    prismatic_spring_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    prismatic_set_spring_damping_ratio,
    try_prismatic_set_spring_damping_ratio,
    JointType::Prismatic,
    prismatic_set_spring_damping_ratio_impl,
    damping_ratio: f32
);
joint_scalar_getter_triple!(
    prismatic_target_translation,
    try_prismatic_target_translation,
    JointType::Prismatic,
    prismatic_target_translation_impl,
    f32
);
joint_scalar_setter_triple!(
    prismatic_set_target_translation,
    try_prismatic_set_target_translation,
    JointType::Prismatic,
    prismatic_set_target_translation_impl,
    translation: f32
);
joint_scalar_getter_triple!(
    prismatic_limit_enabled,
    try_prismatic_limit_enabled,
    JointType::Prismatic,
    prismatic_limit_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    prismatic_enable_limit,
    try_prismatic_enable_limit,
    JointType::Prismatic,
    prismatic_enable_limit_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    prismatic_lower_limit,
    try_prismatic_lower_limit,
    JointType::Prismatic,
    prismatic_lower_limit_impl,
    f32
);
joint_scalar_getter_triple!(
    prismatic_upper_limit,
    try_prismatic_upper_limit,
    JointType::Prismatic,
    prismatic_upper_limit_impl,
    f32
);
joint_two_arg_setter_triple!(
    prismatic_set_limits,
    try_prismatic_set_limits,
    JointType::Prismatic,
    prismatic_set_limits_impl,
    lower: f32,
    upper: f32
);
joint_scalar_getter_triple!(
    prismatic_motor_enabled,
    try_prismatic_motor_enabled,
    JointType::Prismatic,
    prismatic_motor_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    prismatic_enable_motor,
    try_prismatic_enable_motor,
    JointType::Prismatic,
    prismatic_enable_motor_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    prismatic_motor_speed,
    try_prismatic_motor_speed,
    JointType::Prismatic,
    prismatic_motor_speed_impl,
    f32
);
joint_scalar_setter_triple!(
    prismatic_set_motor_speed,
    try_prismatic_set_motor_speed,
    JointType::Prismatic,
    prismatic_set_motor_speed_impl,
    speed: f32
);
joint_scalar_getter_triple!(
    prismatic_max_motor_force,
    try_prismatic_max_motor_force,
    JointType::Prismatic,
    prismatic_max_motor_force_impl,
    f32
);
joint_scalar_setter_triple!(
    prismatic_set_max_motor_force,
    try_prismatic_set_max_motor_force,
    JointType::Prismatic,
    prismatic_set_max_motor_force_impl,
    force: f32
);
joint_scalar_getter_triple!(
    prismatic_motor_force,
    try_prismatic_motor_force,
    JointType::Prismatic,
    prismatic_motor_force_impl,
    f32
);
joint_scalar_getter_triple!(
    prismatic_translation,
    try_prismatic_translation,
    JointType::Prismatic,
    prismatic_translation_impl,
    f32
);
joint_scalar_getter_triple!(
    prismatic_speed,
    try_prismatic_speed,
    JointType::Prismatic,
    prismatic_speed_impl,
    f32
);

joint_scalar_getter_triple!(
    revolute_spring_enabled,
    try_revolute_spring_enabled,
    JointType::Revolute,
    revolute_spring_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    revolute_enable_spring,
    try_revolute_enable_spring,
    JointType::Revolute,
    revolute_enable_spring_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    revolute_spring_hertz,
    try_revolute_spring_hertz,
    JointType::Revolute,
    revolute_spring_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    revolute_set_spring_hertz,
    try_revolute_set_spring_hertz,
    JointType::Revolute,
    revolute_set_spring_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    revolute_spring_damping_ratio,
    try_revolute_spring_damping_ratio,
    JointType::Revolute,
    revolute_spring_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    revolute_set_spring_damping_ratio,
    try_revolute_set_spring_damping_ratio,
    JointType::Revolute,
    revolute_set_spring_damping_ratio_impl,
    damping_ratio: f32
);
joint_scalar_getter_triple!(
    revolute_target_angle,
    try_revolute_target_angle,
    JointType::Revolute,
    revolute_target_angle_impl,
    f32
);
joint_scalar_setter_triple!(
    revolute_set_target_angle,
    try_revolute_set_target_angle,
    JointType::Revolute,
    revolute_set_target_angle_impl,
    angle: f32
);
joint_scalar_getter_triple!(
    revolute_angle,
    try_revolute_angle,
    JointType::Revolute,
    revolute_angle_impl,
    f32
);
joint_scalar_getter_triple!(
    revolute_limit_enabled,
    try_revolute_limit_enabled,
    JointType::Revolute,
    revolute_limit_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    revolute_enable_limit,
    try_revolute_enable_limit,
    JointType::Revolute,
    revolute_enable_limit_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    revolute_lower_limit,
    try_revolute_lower_limit,
    JointType::Revolute,
    revolute_lower_limit_impl,
    f32
);
joint_scalar_getter_triple!(
    revolute_upper_limit,
    try_revolute_upper_limit,
    JointType::Revolute,
    revolute_upper_limit_impl,
    f32
);
joint_two_arg_setter_triple!(
    revolute_set_limits,
    try_revolute_set_limits,
    JointType::Revolute,
    revolute_set_limits_impl,
    lower: f32,
    upper: f32
);
joint_scalar_getter_triple!(
    revolute_motor_enabled,
    try_revolute_motor_enabled,
    JointType::Revolute,
    revolute_motor_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    revolute_enable_motor,
    try_revolute_enable_motor,
    JointType::Revolute,
    revolute_enable_motor_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    revolute_motor_speed,
    try_revolute_motor_speed,
    JointType::Revolute,
    revolute_motor_speed_impl,
    f32
);
joint_scalar_setter_triple!(
    revolute_set_motor_speed,
    try_revolute_set_motor_speed,
    JointType::Revolute,
    revolute_set_motor_speed_impl,
    speed: f32
);
joint_scalar_getter_triple!(
    revolute_motor_torque,
    try_revolute_motor_torque,
    JointType::Revolute,
    revolute_motor_torque_impl,
    f32
);
joint_scalar_getter_triple!(
    revolute_max_motor_torque,
    try_revolute_max_motor_torque,
    JointType::Revolute,
    revolute_max_motor_torque_impl,
    f32
);
joint_scalar_setter_triple!(
    revolute_set_max_motor_torque,
    try_revolute_set_max_motor_torque,
    JointType::Revolute,
    revolute_set_max_motor_torque_impl,
    torque: f32
);

joint_scalar_getter_triple!(
    weld_linear_hertz,
    try_weld_linear_hertz,
    JointType::Weld,
    weld_linear_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    weld_set_linear_hertz,
    try_weld_set_linear_hertz,
    JointType::Weld,
    weld_set_linear_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    weld_linear_damping_ratio,
    try_weld_linear_damping_ratio,
    JointType::Weld,
    weld_linear_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    weld_set_linear_damping_ratio,
    try_weld_set_linear_damping_ratio,
    JointType::Weld,
    weld_set_linear_damping_ratio_impl,
    damping_ratio: f32
);
joint_scalar_getter_triple!(
    weld_angular_hertz,
    try_weld_angular_hertz,
    JointType::Weld,
    weld_angular_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    weld_set_angular_hertz,
    try_weld_set_angular_hertz,
    JointType::Weld,
    weld_set_angular_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    weld_angular_damping_ratio,
    try_weld_angular_damping_ratio,
    JointType::Weld,
    weld_angular_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    weld_set_angular_damping_ratio,
    try_weld_set_angular_damping_ratio,
    JointType::Weld,
    weld_set_angular_damping_ratio_impl,
    damping_ratio: f32
);

joint_scalar_getter_triple!(
    wheel_spring_enabled,
    try_wheel_spring_enabled,
    JointType::Wheel,
    wheel_spring_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    wheel_enable_spring,
    try_wheel_enable_spring,
    JointType::Wheel,
    wheel_enable_spring_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    wheel_spring_hertz,
    try_wheel_spring_hertz,
    JointType::Wheel,
    wheel_spring_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    wheel_set_spring_hertz,
    try_wheel_set_spring_hertz,
    JointType::Wheel,
    wheel_set_spring_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    wheel_spring_damping_ratio,
    try_wheel_spring_damping_ratio,
    JointType::Wheel,
    wheel_spring_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    wheel_set_spring_damping_ratio,
    try_wheel_set_spring_damping_ratio,
    JointType::Wheel,
    wheel_set_spring_damping_ratio_impl,
    damping_ratio: f32
);
joint_scalar_getter_triple!(
    wheel_limit_enabled,
    try_wheel_limit_enabled,
    JointType::Wheel,
    wheel_limit_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    wheel_enable_limit,
    try_wheel_enable_limit,
    JointType::Wheel,
    wheel_enable_limit_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    wheel_lower_limit,
    try_wheel_lower_limit,
    JointType::Wheel,
    wheel_lower_limit_impl,
    f32
);
joint_scalar_getter_triple!(
    wheel_upper_limit,
    try_wheel_upper_limit,
    JointType::Wheel,
    wheel_upper_limit_impl,
    f32
);
joint_two_arg_setter_triple!(
    wheel_set_limits,
    try_wheel_set_limits,
    JointType::Wheel,
    wheel_set_limits_impl,
    lower: f32,
    upper: f32
);
joint_scalar_getter_triple!(
    wheel_motor_enabled,
    try_wheel_motor_enabled,
    JointType::Wheel,
    wheel_motor_enabled_impl,
    bool
);
joint_scalar_setter_triple!(
    wheel_enable_motor,
    try_wheel_enable_motor,
    JointType::Wheel,
    wheel_enable_motor_impl,
    enable: bool
);
joint_scalar_getter_triple!(
    wheel_motor_speed,
    try_wheel_motor_speed,
    JointType::Wheel,
    wheel_motor_speed_impl,
    f32
);
joint_scalar_setter_triple!(
    wheel_set_motor_speed,
    try_wheel_set_motor_speed,
    JointType::Wheel,
    wheel_set_motor_speed_impl,
    speed: f32
);
joint_scalar_getter_triple!(
    wheel_motor_torque,
    try_wheel_motor_torque,
    JointType::Wheel,
    wheel_motor_torque_impl,
    f32
);
joint_scalar_getter_triple!(
    wheel_max_motor_torque,
    try_wheel_max_motor_torque,
    JointType::Wheel,
    wheel_max_motor_torque_impl,
    f32
);
joint_scalar_setter_triple!(
    wheel_set_max_motor_torque,
    try_wheel_set_max_motor_torque,
    JointType::Wheel,
    wheel_set_max_motor_torque_impl,
    torque: f32
);

joint_scalar_getter_triple!(
    motor_linear_velocity,
    try_motor_linear_velocity,
    JointType::Motor,
    motor_linear_velocity_impl,
    Vec2
);
joint_vec2_setter_triple!(
    motor_set_linear_velocity,
    try_motor_set_linear_velocity,
    JointType::Motor,
    motor_set_linear_velocity_impl
);
joint_scalar_getter_triple!(
    motor_angular_velocity,
    try_motor_angular_velocity,
    JointType::Motor,
    motor_angular_velocity_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_angular_velocity,
    try_motor_set_angular_velocity,
    JointType::Motor,
    motor_set_angular_velocity_impl,
    w: f32
);
joint_scalar_getter_triple!(
    motor_max_velocity_force,
    try_motor_max_velocity_force,
    JointType::Motor,
    motor_max_velocity_force_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_max_velocity_force,
    try_motor_set_max_velocity_force,
    JointType::Motor,
    motor_set_max_velocity_force_impl,
    f: f32
);
joint_scalar_getter_triple!(
    motor_max_velocity_torque,
    try_motor_max_velocity_torque,
    JointType::Motor,
    motor_max_velocity_torque_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_max_velocity_torque,
    try_motor_set_max_velocity_torque,
    JointType::Motor,
    motor_set_max_velocity_torque_impl,
    t: f32
);
joint_scalar_getter_triple!(
    motor_linear_hertz,
    try_motor_linear_hertz,
    JointType::Motor,
    motor_linear_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_linear_hertz,
    try_motor_set_linear_hertz,
    JointType::Motor,
    motor_set_linear_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    motor_linear_damping_ratio,
    try_motor_linear_damping_ratio,
    JointType::Motor,
    motor_linear_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_linear_damping_ratio,
    try_motor_set_linear_damping_ratio,
    JointType::Motor,
    motor_set_linear_damping_ratio_impl,
    damping: f32
);
joint_scalar_getter_triple!(
    motor_angular_hertz,
    try_motor_angular_hertz,
    JointType::Motor,
    motor_angular_hertz_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_angular_hertz,
    try_motor_set_angular_hertz,
    JointType::Motor,
    motor_set_angular_hertz_impl,
    hertz: f32
);
joint_scalar_getter_triple!(
    motor_angular_damping_ratio,
    try_motor_angular_damping_ratio,
    JointType::Motor,
    motor_angular_damping_ratio_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_angular_damping_ratio,
    try_motor_set_angular_damping_ratio,
    JointType::Motor,
    motor_set_angular_damping_ratio_impl,
    damping: f32
);
joint_scalar_getter_triple!(
    motor_max_spring_force,
    try_motor_max_spring_force,
    JointType::Motor,
    motor_max_spring_force_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_max_spring_force,
    try_motor_set_max_spring_force,
    JointType::Motor,
    motor_set_max_spring_force_impl,
    f: f32
);
joint_scalar_getter_triple!(
    motor_max_spring_torque,
    try_motor_max_spring_torque,
    JointType::Motor,
    motor_max_spring_torque_impl,
    f32
);
joint_scalar_setter_triple!(
    motor_set_max_spring_torque,
    try_motor_set_max_spring_torque,
    JointType::Motor,
    motor_set_max_spring_torque_impl,
    t: f32
);

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
