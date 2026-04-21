//! Unchecked, `unsafe` convenience APIs for hot paths.
//!
//! This module is gated behind the `unchecked` feature.
//! All methods here are `unsafe` and skip runtime id validity checks.
// Most APIs here are intentionally minimal; keep clippy's per-item safety doc requirement off.
#![allow(clippy::missing_safety_doc)]

use core::ffi::c_void;

use boxdd_sys::ffi;

use crate::body::{BodyType, OwnedBody};
use crate::joints::OwnedJoint;
use crate::shapes::chain::{Chain, OwnedChain};
use crate::shapes::{OwnedShape, ShapeType, SurfaceMaterial};
use crate::types::{BodyId, ChainId, JointId, ShapeId, Vec2};
use crate::{Body, Joint, Shape, Transform, World};

#[inline]
fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
fn raw_shape_id(id: ShapeId) -> ffi::b2ShapeId {
    id.into_raw()
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

#[inline]
fn raw_chain_id(id: ChainId) -> ffi::b2ChainId {
    id.into_raw()
}

#[inline]
unsafe fn body_transform_raw_unchecked_impl(id: BodyId) -> ffi::b2Transform {
    unsafe { ffi::b2Body_GetTransform(raw_body_id(id)) }
}

#[inline]
unsafe fn body_transform_unchecked_impl(id: BodyId) -> Transform {
    Transform::from_raw(unsafe { body_transform_raw_unchecked_impl(id) })
}

#[inline]
unsafe fn body_position_unchecked_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetPosition(raw_body_id(id)) })
}

#[inline]
unsafe fn body_linear_velocity_unchecked_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetLinearVelocity(raw_body_id(id)) })
}

#[inline]
unsafe fn body_angular_velocity_unchecked_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetAngularVelocity(raw_body_id(id)) }
}

#[inline]
unsafe fn set_body_linear_velocity_unchecked_impl(id: BodyId, v: Vec2) {
    let raw: ffi::b2Vec2 = v.into_raw();
    unsafe { ffi::b2Body_SetLinearVelocity(raw_body_id(id), raw) }
}

#[inline]
unsafe fn set_body_angular_velocity_unchecked_impl(id: BodyId, w: f32) {
    unsafe { ffi::b2Body_SetAngularVelocity(raw_body_id(id), w) }
}

#[inline]
unsafe fn body_type_unchecked_impl(id: BodyId) -> BodyType {
    BodyType::from_raw(unsafe { ffi::b2Body_GetType(raw_body_id(id)) })
}

#[inline]
unsafe fn set_body_type_unchecked_impl(id: BodyId, body_type: BodyType) {
    unsafe { ffi::b2Body_SetType(raw_body_id(id), body_type.into_raw()) }
}

#[inline]
unsafe fn set_body_gravity_scale_unchecked_impl(id: BodyId, value: f32) {
    unsafe { ffi::b2Body_SetGravityScale(raw_body_id(id), value) }
}

#[inline]
unsafe fn body_gravity_scale_unchecked_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetGravityScale(raw_body_id(id)) }
}

#[inline]
unsafe fn shape_body_unchecked_impl(id: ShapeId) -> BodyId {
    BodyId::from_raw(unsafe { ffi::b2Shape_GetBody(raw_shape_id(id)) })
}

#[inline]
unsafe fn shape_type_unchecked_impl(id: ShapeId) -> ShapeType {
    ShapeType::from_raw(unsafe { ffi::b2Shape_GetType(raw_shape_id(id)) })
        .expect("Box2D returned an unknown shape type")
}

#[inline]
unsafe fn shape_density_unchecked_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetDensity(raw_shape_id(id)) }
}

#[inline]
unsafe fn set_shape_density_unchecked_impl(id: ShapeId, density: f32, update_body_mass: bool) {
    unsafe { ffi::b2Shape_SetDensity(raw_shape_id(id), density, update_body_mass) }
}

#[inline]
unsafe fn set_shape_surface_material_unchecked_impl(id: ShapeId, material: &SurfaceMaterial) {
    unsafe { ffi::b2Shape_SetSurfaceMaterial(raw_shape_id(id), &material.0) }
}

#[inline]
unsafe fn joint_force_threshold_unchecked_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetForceThreshold(raw_joint_id(id)) }
}

#[inline]
unsafe fn set_joint_force_threshold_unchecked_impl(id: JointId, threshold: f32) {
    unsafe { ffi::b2Joint_SetForceThreshold(raw_joint_id(id), threshold) }
}

#[inline]
unsafe fn joint_user_data_ptr_unchecked_impl(id: JointId) -> *mut c_void {
    unsafe { ffi::b2Joint_GetUserData(raw_joint_id(id)) }
}

#[inline]
unsafe fn set_joint_user_data_ptr_unchecked_impl(id: JointId, ptr: *mut c_void) {
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), ptr) }
}

#[inline]
unsafe fn chain_segment_count_unchecked_impl(id: ChainId) -> i32 {
    unsafe { ffi::b2Chain_GetSegmentCount(raw_chain_id(id)) }
}

#[inline]
unsafe fn chain_segments_unchecked_impl(id: ChainId) -> Vec<ShapeId> {
    let count = unsafe { chain_segment_count_unchecked_impl(id) }.max(0) as usize;
    let id = raw_chain_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(count, |ptr: *mut ShapeId, count| {
            ffi::b2Chain_GetSegments(id, ptr.cast(), count)
        })
    }
}

#[inline]
unsafe fn chain_surface_material_unchecked_impl(id: ChainId, index: i32) -> SurfaceMaterial {
    SurfaceMaterial::from_raw(unsafe {
        ffi::b2Chain_GetRuntimeSurfaceMaterial(raw_chain_id(id), index)
    })
}

#[inline]
unsafe fn set_chain_surface_material_unchecked_impl(
    id: ChainId,
    index: i32,
    material: &SurfaceMaterial,
) {
    unsafe { ffi::b2Chain_SetRuntimeSurfaceMaterial(raw_chain_id(id), &material.0, index) }
}

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
        unsafe { body_transform_unchecked_impl(body) }
    }

    unsafe fn body_position_unchecked(&self, body: BodyId) -> Vec2 {
        unsafe { body_position_unchecked_impl(body) }
    }

    unsafe fn set_body_linear_velocity_unchecked(&mut self, body: BodyId, v: Vec2) {
        unsafe { set_body_linear_velocity_unchecked_impl(body, v) }
    }

    unsafe fn set_body_angular_velocity_unchecked(&mut self, body: BodyId, w: f32) {
        unsafe { set_body_angular_velocity_unchecked_impl(body, w) }
    }

    unsafe fn set_body_type_unchecked(&mut self, body: BodyId, t: BodyType) {
        unsafe { set_body_type_unchecked_impl(body, t) }
    }

    unsafe fn shape_body_unchecked(&self, shape: ShapeId) -> BodyId {
        unsafe { shape_body_unchecked_impl(shape) }
    }

    unsafe fn shape_type_unchecked(&self, shape: ShapeId) -> ShapeType {
        unsafe { shape_type_unchecked_impl(shape) }
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

impl BodyUncheckedExt for Body<'_> {
    unsafe fn position_unchecked(&self) -> Vec2 {
        unsafe { body_position_unchecked_impl(self.id) }
    }

    unsafe fn linear_velocity_unchecked(&self) -> Vec2 {
        unsafe { body_linear_velocity_unchecked_impl(self.id) }
    }

    unsafe fn angular_velocity_unchecked(&self) -> f32 {
        unsafe { body_angular_velocity_unchecked_impl(self.id) }
    }

    unsafe fn transform_unchecked(&self) -> ffi::b2Transform {
        unsafe { body_transform_raw_unchecked_impl(self.id) }
    }

    unsafe fn set_linear_velocity_unchecked(&mut self, v: Vec2) {
        unsafe { set_body_linear_velocity_unchecked_impl(self.id, v) }
    }

    unsafe fn set_angular_velocity_unchecked(&mut self, w: f32) {
        unsafe { set_body_angular_velocity_unchecked_impl(self.id, w) }
    }

    unsafe fn body_type_unchecked(&self) -> BodyType {
        unsafe { body_type_unchecked_impl(self.id) }
    }

    unsafe fn set_body_type_unchecked(&mut self, t: BodyType) {
        unsafe { set_body_type_unchecked_impl(self.id, t) }
    }

    unsafe fn set_gravity_scale_unchecked(&mut self, v: f32) {
        unsafe { set_body_gravity_scale_unchecked_impl(self.id, v) }
    }

    unsafe fn gravity_scale_unchecked(&self) -> f32 {
        unsafe { body_gravity_scale_unchecked_impl(self.id) }
    }
}

impl BodyUncheckedExt for OwnedBody {
    unsafe fn position_unchecked(&self) -> Vec2 {
        unsafe { body_position_unchecked_impl(self.id()) }
    }

    unsafe fn linear_velocity_unchecked(&self) -> Vec2 {
        unsafe { body_linear_velocity_unchecked_impl(self.id()) }
    }

    unsafe fn angular_velocity_unchecked(&self) -> f32 {
        unsafe { body_angular_velocity_unchecked_impl(self.id()) }
    }

    unsafe fn transform_unchecked(&self) -> ffi::b2Transform {
        unsafe { body_transform_raw_unchecked_impl(self.id()) }
    }

    unsafe fn set_linear_velocity_unchecked(&mut self, v: Vec2) {
        unsafe { set_body_linear_velocity_unchecked_impl(self.id(), v) }
    }

    unsafe fn set_angular_velocity_unchecked(&mut self, w: f32) {
        unsafe { set_body_angular_velocity_unchecked_impl(self.id(), w) }
    }

    unsafe fn body_type_unchecked(&self) -> BodyType {
        unsafe { body_type_unchecked_impl(self.id()) }
    }

    unsafe fn set_body_type_unchecked(&mut self, t: BodyType) {
        unsafe { set_body_type_unchecked_impl(self.id(), t) }
    }

    unsafe fn set_gravity_scale_unchecked(&mut self, v: f32) {
        unsafe { set_body_gravity_scale_unchecked_impl(self.id(), v) }
    }

    unsafe fn gravity_scale_unchecked(&self) -> f32 {
        unsafe { body_gravity_scale_unchecked_impl(self.id()) }
    }
}

pub trait ShapeUncheckedExt {
    unsafe fn shape_type_unchecked(&self) -> ShapeType;
    unsafe fn body_id_unchecked(&self) -> BodyId;
    unsafe fn density_unchecked(&self) -> f32;
    unsafe fn set_density_unchecked(&mut self, density: f32, update_body_mass: bool);
    unsafe fn set_surface_material_unchecked(&mut self, material: &SurfaceMaterial);
}

impl ShapeUncheckedExt for Shape<'_> {
    unsafe fn shape_type_unchecked(&self) -> ShapeType {
        unsafe { shape_type_unchecked_impl(self.id) }
    }

    unsafe fn body_id_unchecked(&self) -> BodyId {
        unsafe { shape_body_unchecked_impl(self.id) }
    }

    unsafe fn density_unchecked(&self) -> f32 {
        unsafe { shape_density_unchecked_impl(self.id) }
    }

    unsafe fn set_density_unchecked(&mut self, density: f32, update_body_mass: bool) {
        unsafe { set_shape_density_unchecked_impl(self.id, density, update_body_mass) }
    }

    unsafe fn set_surface_material_unchecked(&mut self, material: &SurfaceMaterial) {
        unsafe { set_shape_surface_material_unchecked_impl(self.id, material) }
    }
}

impl ShapeUncheckedExt for OwnedShape {
    unsafe fn shape_type_unchecked(&self) -> ShapeType {
        unsafe { shape_type_unchecked_impl(self.id()) }
    }

    unsafe fn body_id_unchecked(&self) -> BodyId {
        unsafe { shape_body_unchecked_impl(self.id()) }
    }

    unsafe fn density_unchecked(&self) -> f32 {
        unsafe { shape_density_unchecked_impl(self.id()) }
    }

    unsafe fn set_density_unchecked(&mut self, density: f32, update_body_mass: bool) {
        unsafe { set_shape_density_unchecked_impl(self.id(), density, update_body_mass) }
    }

    unsafe fn set_surface_material_unchecked(&mut self, material: &SurfaceMaterial) {
        unsafe { set_shape_surface_material_unchecked_impl(self.id(), material) }
    }
}

pub trait JointUncheckedExt {
    unsafe fn force_threshold_unchecked(&self) -> f32;
    unsafe fn set_force_threshold_unchecked(&mut self, threshold: f32);
    unsafe fn user_data_ptr_unchecked(&self) -> *mut c_void;
    unsafe fn set_user_data_ptr_unchecked(&mut self, p: *mut c_void);
}

impl JointUncheckedExt for Joint<'_> {
    unsafe fn force_threshold_unchecked(&self) -> f32 {
        unsafe { joint_force_threshold_unchecked_impl(self.id) }
    }

    unsafe fn set_force_threshold_unchecked(&mut self, threshold: f32) {
        unsafe { set_joint_force_threshold_unchecked_impl(self.id, threshold) }
    }

    unsafe fn user_data_ptr_unchecked(&self) -> *mut c_void {
        unsafe { joint_user_data_ptr_unchecked_impl(self.id) }
    }

    unsafe fn set_user_data_ptr_unchecked(&mut self, p: *mut c_void) {
        unsafe { set_joint_user_data_ptr_unchecked_impl(self.id, p) }
    }
}

impl JointUncheckedExt for OwnedJoint {
    unsafe fn force_threshold_unchecked(&self) -> f32 {
        unsafe { joint_force_threshold_unchecked_impl(self.id()) }
    }

    unsafe fn set_force_threshold_unchecked(&mut self, threshold: f32) {
        unsafe { set_joint_force_threshold_unchecked_impl(self.id(), threshold) }
    }

    unsafe fn user_data_ptr_unchecked(&self) -> *mut c_void {
        unsafe { joint_user_data_ptr_unchecked_impl(self.id()) }
    }

    unsafe fn set_user_data_ptr_unchecked(&mut self, p: *mut c_void) {
        unsafe { set_joint_user_data_ptr_unchecked_impl(self.id(), p) }
    }
}

pub trait ChainUncheckedExt {
    unsafe fn segment_count_unchecked(&self) -> i32;
    unsafe fn segments_unchecked(&self) -> Vec<ShapeId>;
    unsafe fn surface_material_unchecked(&self, index: i32) -> SurfaceMaterial;
    unsafe fn set_surface_material_unchecked(&mut self, index: i32, material: &SurfaceMaterial);
}

impl ChainUncheckedExt for OwnedChain {
    unsafe fn segment_count_unchecked(&self) -> i32 {
        unsafe { chain_segment_count_unchecked_impl(self.id()) }
    }

    unsafe fn segments_unchecked(&self) -> Vec<ShapeId> {
        unsafe { chain_segments_unchecked_impl(self.id()) }
    }

    unsafe fn surface_material_unchecked(&self, index: i32) -> SurfaceMaterial {
        unsafe { chain_surface_material_unchecked_impl(self.id(), index) }
    }

    unsafe fn set_surface_material_unchecked(&mut self, index: i32, material: &SurfaceMaterial) {
        unsafe { set_chain_surface_material_unchecked_impl(self.id(), index, material) }
    }
}

impl ChainUncheckedExt for Chain<'_> {
    unsafe fn segment_count_unchecked(&self) -> i32 {
        unsafe { chain_segment_count_unchecked_impl(self.id) }
    }

    unsafe fn segments_unchecked(&self) -> Vec<ShapeId> {
        unsafe { chain_segments_unchecked_impl(self.id) }
    }

    unsafe fn surface_material_unchecked(&self, index: i32) -> SurfaceMaterial {
        unsafe { chain_surface_material_unchecked_impl(self.id, index) }
    }

    unsafe fn set_surface_material_unchecked(&mut self, index: i32, material: &SurfaceMaterial) {
        unsafe { set_chain_surface_material_unchecked_impl(self.id, index, material) }
    }
}

// Re-export some ids to make `unchecked` imports feel self-contained.
pub type UncheckedBodyId = BodyId;
pub type UncheckedShapeId = ShapeId;
pub type UncheckedJointId = JointId;
pub type UncheckedChainId = ChainId;
