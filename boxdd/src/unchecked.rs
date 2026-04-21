//! Unchecked, `unsafe` convenience APIs for hot paths.
//!
//! This module is gated behind the `unchecked` feature.
//! All methods here are `unsafe` and skip runtime id validity checks.
// Most APIs here are intentionally minimal; keep clippy's per-item safety doc requirement off.
#![allow(clippy::missing_safety_doc)]

use boxdd_sys::ffi;

use crate::body::{BodyType, OwnedBody};
use crate::joints::OwnedJoint;
use crate::shapes::chain::OwnedChain;
use crate::shapes::{OwnedShape, ShapeType, SurfaceMaterial};
use crate::types::{BodyId, ChainId, JointId, ShapeId, Vec2};
use crate::{Body, Joint, Shape, Transform, World};

pub trait WorldUncheckedExt {
    unsafe fn body_transform_unchecked(&self, body: BodyId) -> Transform;
    unsafe fn body_position_unchecked(&self, body: BodyId) -> Vec2;
    unsafe fn set_body_linear_velocity_unchecked(&mut self, body: BodyId, v: Vec2);
    unsafe fn set_body_angular_velocity_unchecked(&mut self, body: BodyId, w: f32);
    unsafe fn set_body_type_unchecked(&mut self, body: BodyId, t: BodyType);

    unsafe fn shape_body_unchecked(&self, shape: ShapeId) -> BodyId;
    unsafe fn shape_type_unchecked(&self, shape: ShapeId) -> ShapeType;
}

impl WorldUncheckedExt for World {
    unsafe fn body_transform_unchecked(&self, body: BodyId) -> Transform {
        Transform::from(unsafe { ffi::b2Body_GetTransform(body) })
    }
    unsafe fn body_position_unchecked(&self, body: BodyId) -> Vec2 {
        Vec2::from(unsafe { ffi::b2Body_GetPosition(body) })
    }
    unsafe fn set_body_linear_velocity_unchecked(&mut self, body: BodyId, v: Vec2) {
        let vv: ffi::b2Vec2 = v.into();
        unsafe { ffi::b2Body_SetLinearVelocity(body, vv) }
    }
    unsafe fn set_body_angular_velocity_unchecked(&mut self, body: BodyId, w: f32) {
        unsafe { ffi::b2Body_SetAngularVelocity(body, w) }
    }
    unsafe fn set_body_type_unchecked(&mut self, body: BodyId, t: BodyType) {
        unsafe { ffi::b2Body_SetType(body, t.into_raw()) }
    }

    unsafe fn shape_body_unchecked(&self, shape: ShapeId) -> BodyId {
        unsafe { ffi::b2Shape_GetBody(shape) }
    }
    unsafe fn shape_type_unchecked(&self, shape: ShapeId) -> ShapeType {
        ShapeType::from_raw(unsafe { ffi::b2Shape_GetType(shape) })
            .expect("Box2D returned an unknown shape type")
    }
}

pub trait BodyUncheckedExt {
    unsafe fn position_unchecked(&self) -> Vec2;
    unsafe fn linear_velocity_unchecked(&self) -> Vec2;
    unsafe fn angular_velocity_unchecked(&self) -> f32;
    unsafe fn transform_unchecked(&self) -> ffi::b2Transform;
    unsafe fn set_linear_velocity_unchecked(&mut self, v: Vec2);
    unsafe fn set_angular_velocity_unchecked(&mut self, w: f32);
    unsafe fn body_type_unchecked(&self) -> BodyType;
    unsafe fn set_body_type_unchecked(&mut self, t: BodyType);
    unsafe fn set_gravity_scale_unchecked(&mut self, v: f32);
    unsafe fn gravity_scale_unchecked(&self) -> f32;
}

macro_rules! impl_body_unchecked_ext {
    ($ty:ty, field $id:ident) => {
        impl BodyUncheckedExt for $ty {
            unsafe fn position_unchecked(&self) -> Vec2 {
                Vec2::from(unsafe { ffi::b2Body_GetPosition(self.$id) })
            }
            unsafe fn linear_velocity_unchecked(&self) -> Vec2 {
                Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(self.$id) })
            }
            unsafe fn angular_velocity_unchecked(&self) -> f32 {
                unsafe { ffi::b2Body_GetAngularVelocity(self.$id) }
            }
            unsafe fn transform_unchecked(&self) -> ffi::b2Transform {
                unsafe { ffi::b2Body_GetTransform(self.$id) }
            }
            unsafe fn set_linear_velocity_unchecked(&mut self, v: Vec2) {
                let vv: ffi::b2Vec2 = v.into();
                unsafe { ffi::b2Body_SetLinearVelocity(self.$id, vv) }
            }
            unsafe fn set_angular_velocity_unchecked(&mut self, w: f32) {
                unsafe { ffi::b2Body_SetAngularVelocity(self.$id, w) }
            }
            unsafe fn body_type_unchecked(&self) -> BodyType {
                BodyType::from_raw(unsafe { ffi::b2Body_GetType(self.$id) })
            }
            unsafe fn set_body_type_unchecked(&mut self, t: BodyType) {
                unsafe { ffi::b2Body_SetType(self.$id, t.into_raw()) }
            }
            unsafe fn set_gravity_scale_unchecked(&mut self, v: f32) {
                unsafe { ffi::b2Body_SetGravityScale(self.$id, v) }
            }
            unsafe fn gravity_scale_unchecked(&self) -> f32 {
                unsafe { ffi::b2Body_GetGravityScale(self.$id) }
            }
        }
    };
    ($ty:ty, method $id:ident) => {
        impl BodyUncheckedExt for $ty {
            unsafe fn position_unchecked(&self) -> Vec2 {
                Vec2::from(unsafe { ffi::b2Body_GetPosition(self.$id()) })
            }
            unsafe fn linear_velocity_unchecked(&self) -> Vec2 {
                Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(self.$id()) })
            }
            unsafe fn angular_velocity_unchecked(&self) -> f32 {
                unsafe { ffi::b2Body_GetAngularVelocity(self.$id()) }
            }
            unsafe fn transform_unchecked(&self) -> ffi::b2Transform {
                unsafe { ffi::b2Body_GetTransform(self.$id()) }
            }
            unsafe fn set_linear_velocity_unchecked(&mut self, v: Vec2) {
                let vv: ffi::b2Vec2 = v.into();
                unsafe { ffi::b2Body_SetLinearVelocity(self.$id(), vv) }
            }
            unsafe fn set_angular_velocity_unchecked(&mut self, w: f32) {
                unsafe { ffi::b2Body_SetAngularVelocity(self.$id(), w) }
            }
            unsafe fn body_type_unchecked(&self) -> BodyType {
                BodyType::from_raw(unsafe { ffi::b2Body_GetType(self.$id()) })
            }
            unsafe fn set_body_type_unchecked(&mut self, t: BodyType) {
                unsafe { ffi::b2Body_SetType(self.$id(), t.into_raw()) }
            }
            unsafe fn set_gravity_scale_unchecked(&mut self, v: f32) {
                unsafe { ffi::b2Body_SetGravityScale(self.$id(), v) }
            }
            unsafe fn gravity_scale_unchecked(&self) -> f32 {
                unsafe { ffi::b2Body_GetGravityScale(self.$id()) }
            }
        }
    };
}

impl_body_unchecked_ext!(Body<'_>, field id);
impl_body_unchecked_ext!(OwnedBody, method id);

pub trait ShapeUncheckedExt {
    unsafe fn shape_type_unchecked(&self) -> ShapeType;
    unsafe fn body_id_unchecked(&self) -> BodyId;
    unsafe fn density_unchecked(&self) -> f32;
    unsafe fn set_density_unchecked(&mut self, density: f32, update_body_mass: bool);
    unsafe fn set_surface_material_unchecked(&mut self, material: &SurfaceMaterial);
}

macro_rules! impl_shape_unchecked_ext {
    ($ty:ty, field $id:ident) => {
        impl ShapeUncheckedExt for $ty {
            unsafe fn shape_type_unchecked(&self) -> ShapeType {
                ShapeType::from_raw(unsafe { ffi::b2Shape_GetType(self.$id) })
                    .expect("Box2D returned an unknown shape type")
            }
            unsafe fn body_id_unchecked(&self) -> BodyId {
                unsafe { ffi::b2Shape_GetBody(self.$id) }
            }
            unsafe fn density_unchecked(&self) -> f32 {
                unsafe { ffi::b2Shape_GetDensity(self.$id) }
            }
            unsafe fn set_density_unchecked(&mut self, density: f32, update_body_mass: bool) {
                unsafe { ffi::b2Shape_SetDensity(self.$id, density, update_body_mass) }
            }
            unsafe fn set_surface_material_unchecked(&mut self, material: &SurfaceMaterial) {
                unsafe { ffi::b2Shape_SetSurfaceMaterial(self.$id, &material.0) }
            }
        }
    };
    ($ty:ty, method $id:ident) => {
        impl ShapeUncheckedExt for $ty {
            unsafe fn shape_type_unchecked(&self) -> ShapeType {
                ShapeType::from_raw(unsafe { ffi::b2Shape_GetType(self.$id()) })
                    .expect("Box2D returned an unknown shape type")
            }
            unsafe fn body_id_unchecked(&self) -> BodyId {
                unsafe { ffi::b2Shape_GetBody(self.$id()) }
            }
            unsafe fn density_unchecked(&self) -> f32 {
                unsafe { ffi::b2Shape_GetDensity(self.$id()) }
            }
            unsafe fn set_density_unchecked(&mut self, density: f32, update_body_mass: bool) {
                unsafe { ffi::b2Shape_SetDensity(self.$id(), density, update_body_mass) }
            }
            unsafe fn set_surface_material_unchecked(&mut self, material: &SurfaceMaterial) {
                unsafe { ffi::b2Shape_SetSurfaceMaterial(self.$id(), &material.0) }
            }
        }
    };
}

impl_shape_unchecked_ext!(Shape<'_>, field id);
impl_shape_unchecked_ext!(OwnedShape, method id);

pub trait JointUncheckedExt {
    unsafe fn force_threshold_unchecked(&self) -> f32;
    unsafe fn set_force_threshold_unchecked(&mut self, threshold: f32);
    unsafe fn user_data_ptr_unchecked(&self) -> *mut core::ffi::c_void;
    unsafe fn set_user_data_ptr_unchecked(&mut self, p: *mut core::ffi::c_void);
}

macro_rules! impl_joint_unchecked_ext {
    ($ty:ty, field $id:ident) => {
        impl JointUncheckedExt for $ty {
            unsafe fn force_threshold_unchecked(&self) -> f32 {
                unsafe { ffi::b2Joint_GetForceThreshold(self.$id) }
            }
            unsafe fn set_force_threshold_unchecked(&mut self, threshold: f32) {
                unsafe { ffi::b2Joint_SetForceThreshold(self.$id, threshold) }
            }
            unsafe fn user_data_ptr_unchecked(&self) -> *mut core::ffi::c_void {
                unsafe { ffi::b2Joint_GetUserData(self.$id) }
            }
            unsafe fn set_user_data_ptr_unchecked(&mut self, p: *mut core::ffi::c_void) {
                unsafe { ffi::b2Joint_SetUserData(self.$id, p) }
            }
        }
    };
    ($ty:ty, method $id:ident) => {
        impl JointUncheckedExt for $ty {
            unsafe fn force_threshold_unchecked(&self) -> f32 {
                unsafe { ffi::b2Joint_GetForceThreshold(self.$id()) }
            }
            unsafe fn set_force_threshold_unchecked(&mut self, threshold: f32) {
                unsafe { ffi::b2Joint_SetForceThreshold(self.$id(), threshold) }
            }
            unsafe fn user_data_ptr_unchecked(&self) -> *mut core::ffi::c_void {
                unsafe { ffi::b2Joint_GetUserData(self.$id()) }
            }
            unsafe fn set_user_data_ptr_unchecked(&mut self, p: *mut core::ffi::c_void) {
                unsafe { ffi::b2Joint_SetUserData(self.$id(), p) }
            }
        }
    };
}

impl_joint_unchecked_ext!(Joint<'_>, field id);
impl_joint_unchecked_ext!(OwnedJoint, method id);

pub trait ChainUncheckedExt {
    unsafe fn segment_count_unchecked(&self) -> i32;
    unsafe fn segments_unchecked(&self) -> Vec<ShapeId>;
    unsafe fn surface_material_unchecked(&self, index: i32) -> SurfaceMaterial;
    unsafe fn set_surface_material_unchecked(&mut self, index: i32, material: &SurfaceMaterial);
}

macro_rules! impl_chain_unchecked_ext {
    ($ty:ty, field $id:ident) => {
        impl ChainUncheckedExt for $ty {
            unsafe fn segment_count_unchecked(&self) -> i32 {
                unsafe { ffi::b2Chain_GetSegmentCount(self.$id) }
            }
            unsafe fn segments_unchecked(&self) -> Vec<ShapeId> {
                let count = unsafe { ffi::b2Chain_GetSegmentCount(self.$id) }.max(0) as usize;
                unsafe {
                    crate::core::ffi_vec::read_from_ffi(count, |ptr, count| {
                        ffi::b2Chain_GetSegments(self.$id, ptr, count)
                    })
                }
            }
            unsafe fn surface_material_unchecked(&self, index: i32) -> SurfaceMaterial {
                SurfaceMaterial::from_raw(unsafe {
                    ffi::b2Chain_GetSurfaceMaterial(self.$id, index)
                })
            }
            unsafe fn set_surface_material_unchecked(
                &mut self,
                index: i32,
                material: &SurfaceMaterial,
            ) {
                unsafe { ffi::b2Chain_SetSurfaceMaterial(self.$id, &material.0, index) }
            }
        }
    };
    ($ty:ty, method $id:ident) => {
        impl ChainUncheckedExt for $ty {
            unsafe fn segment_count_unchecked(&self) -> i32 {
                unsafe { ffi::b2Chain_GetSegmentCount(self.$id()) }
            }
            unsafe fn segments_unchecked(&self) -> Vec<ShapeId> {
                let count = unsafe { ffi::b2Chain_GetSegmentCount(self.$id()) }.max(0) as usize;
                unsafe {
                    crate::core::ffi_vec::read_from_ffi(count, |ptr, count| {
                        ffi::b2Chain_GetSegments(self.$id(), ptr, count)
                    })
                }
            }
            unsafe fn surface_material_unchecked(&self, index: i32) -> SurfaceMaterial {
                SurfaceMaterial::from_raw(unsafe {
                    ffi::b2Chain_GetSurfaceMaterial(self.$id(), index)
                })
            }
            unsafe fn set_surface_material_unchecked(
                &mut self,
                index: i32,
                material: &SurfaceMaterial,
            ) {
                unsafe { ffi::b2Chain_SetSurfaceMaterial(self.$id(), &material.0, index) }
            }
        }
    };
}

impl_chain_unchecked_ext!(OwnedChain, method id);
impl_chain_unchecked_ext!(crate::shapes::chain::Chain<'_>, field id);

// Re-export some ids to make `unchecked` imports feel self-contained.
pub type UncheckedBodyId = BodyId;
pub type UncheckedShapeId = ShapeId;
pub type UncheckedJointId = JointId;
pub type UncheckedChainId = ChainId;
