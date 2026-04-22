use std::ffi::CStr;

use crate::query::Aabb;
use crate::types::{BodyId, JointId, MassData, MotionLocks, ShapeId, Vec2};
use boxdd_sys::ffi;

use super::definition::BodyType;

mod attachments;
mod handle;
mod user_data;

pub(crate) use handle::BodyRuntimeHandle;

#[inline]
pub(crate) fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
fn body_world_id_impl(id: BodyId) -> ffi::b2WorldId {
    unsafe { ffi::b2Body_GetWorld(raw_body_id(id)) }
}

#[inline]
fn body_is_valid_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsValid(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_position_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetPosition(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_linear_velocity_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetLinearVelocity(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_angular_velocity_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetAngularVelocity(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_rotation_raw_impl(id: BodyId) -> ffi::b2Rot {
    unsafe { ffi::b2Body_GetRotation(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_rotation_impl(id: BodyId) -> crate::Rot {
    crate::Rot::from_raw(body_rotation_raw_impl(id))
}

#[inline]
fn body_transform_raw_impl(id: BodyId) -> ffi::b2Transform {
    unsafe { ffi::b2Body_GetTransform(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_transform_impl(id: BodyId) -> crate::Transform {
    crate::Transform::from_raw(body_transform_raw_impl(id))
}

#[inline]
pub(crate) fn body_aabb_impl(id: BodyId) -> Aabb {
    Aabb::from_raw(unsafe { ffi::b2Body_ComputeAABB(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_local_point_impl<V: Into<Vec2>>(id: BodyId, world_point: V) -> Vec2 {
    let point: ffi::b2Vec2 = world_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalPoint(raw_body_id(id), point) })
}

#[inline]
pub(crate) fn body_world_point_impl<V: Into<Vec2>>(id: BodyId, local_point: V) -> Vec2 {
    let point: ffi::b2Vec2 = local_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetWorldPoint(raw_body_id(id), point) })
}

#[inline]
pub(crate) fn body_local_vector_impl<V: Into<Vec2>>(id: BodyId, world_vector: V) -> Vec2 {
    let vector: ffi::b2Vec2 = world_vector.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalVector(raw_body_id(id), vector) })
}

#[inline]
pub(crate) fn body_world_vector_impl<V: Into<Vec2>>(id: BodyId, local_vector: V) -> Vec2 {
    let vector: ffi::b2Vec2 = local_vector.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetWorldVector(raw_body_id(id), vector) })
}

#[inline]
pub(crate) fn body_local_point_velocity_impl<V: Into<Vec2>>(id: BodyId, local_point: V) -> Vec2 {
    let point: ffi::b2Vec2 = local_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalPointVelocity(raw_body_id(id), point) })
}

#[inline]
pub(crate) fn body_world_point_velocity_impl<V: Into<Vec2>>(id: BodyId, world_point: V) -> Vec2 {
    let point: ffi::b2Vec2 = world_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetWorldPointVelocity(raw_body_id(id), point) })
}

#[inline]
fn body_set_position_and_rotation_impl<V: Into<Vec2>>(id: BodyId, position: V, angle_radians: f32) {
    let (s, c) = angle_radians.sin_cos();
    let rotation = ffi::b2Rot { c, s };
    let position: ffi::b2Vec2 = position.into().into_raw();
    unsafe { ffi::b2Body_SetTransform(raw_body_id(id), position, rotation) };
}

#[inline]
fn body_set_linear_velocity_impl<V: Into<Vec2>>(id: BodyId, velocity: V) {
    let velocity: ffi::b2Vec2 = velocity.into().into_raw();
    unsafe { ffi::b2Body_SetLinearVelocity(raw_body_id(id), velocity) }
}

#[inline]
fn body_set_angular_velocity_impl(id: BodyId, angular_velocity: f32) {
    unsafe { ffi::b2Body_SetAngularVelocity(raw_body_id(id), angular_velocity) }
}

#[inline]
fn body_set_target_transform_impl(
    id: BodyId,
    target: crate::Transform,
    time_step: f32,
    wake: bool,
) {
    unsafe { ffi::b2Body_SetTargetTransform(raw_body_id(id), target.into_raw(), time_step, wake) };
}

#[inline]
fn body_apply_force_impl<F: Into<Vec2>, P: Into<Vec2>>(id: BodyId, force: F, point: P, wake: bool) {
    let force: ffi::b2Vec2 = force.into().into_raw();
    let point: ffi::b2Vec2 = point.into().into_raw();
    unsafe { ffi::b2Body_ApplyForce(raw_body_id(id), force, point, wake) };
}

#[inline]
fn body_apply_force_to_center_impl<V: Into<Vec2>>(id: BodyId, force: V, wake: bool) {
    let force: ffi::b2Vec2 = force.into().into_raw();
    unsafe { ffi::b2Body_ApplyForceToCenter(raw_body_id(id), force, wake) };
}

#[inline]
fn body_apply_torque_impl(id: BodyId, torque: f32, wake: bool) {
    unsafe { ffi::b2Body_ApplyTorque(raw_body_id(id), torque, wake) }
}

#[inline]
fn body_clear_forces_impl(id: BodyId) {
    unsafe { ffi::b2Body_ClearForces(raw_body_id(id)) };
}

#[inline]
fn body_apply_linear_impulse_impl<F: Into<Vec2>, P: Into<Vec2>>(
    id: BodyId,
    impulse: F,
    point: P,
    wake: bool,
) {
    let impulse: ffi::b2Vec2 = impulse.into().into_raw();
    let point: ffi::b2Vec2 = point.into().into_raw();
    unsafe { ffi::b2Body_ApplyLinearImpulse(raw_body_id(id), impulse, point, wake) };
}

#[inline]
fn body_apply_linear_impulse_to_center_impl<V: Into<Vec2>>(id: BodyId, impulse: V, wake: bool) {
    let impulse: ffi::b2Vec2 = impulse.into().into_raw();
    unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(raw_body_id(id), impulse, wake) };
}

#[inline]
fn body_apply_angular_impulse_impl(id: BodyId, impulse: f32, wake: bool) {
    unsafe { ffi::b2Body_ApplyAngularImpulse(raw_body_id(id), impulse, wake) }
}

#[inline]
pub(crate) fn body_mass_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetMass(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_rotational_inertia_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetRotationalInertia(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_local_center_of_mass_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalCenterOfMass(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_world_center_of_mass_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetWorldCenterOfMass(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_mass_data_impl(id: BodyId) -> MassData {
    MassData::from_raw(unsafe { ffi::b2Body_GetMassData(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_motion_locks_impl(id: BodyId) -> MotionLocks {
    MotionLocks::from_raw(unsafe { ffi::b2Body_GetMotionLocks(raw_body_id(id)) })
}

#[inline]
fn body_set_mass_data_impl(id: BodyId, mass_data: MassData) {
    unsafe { ffi::b2Body_SetMassData(raw_body_id(id), mass_data.into_raw()) };
}

#[inline]
fn body_apply_mass_from_shapes_impl(id: BodyId) {
    unsafe { ffi::b2Body_ApplyMassFromShapes(raw_body_id(id)) };
}

#[inline]
pub(crate) fn body_shape_count_impl(id: BodyId) -> i32 {
    unsafe { ffi::b2Body_GetShapeCount(raw_body_id(id)) }
}

#[inline]
fn body_shape_capacity(id: BodyId) -> usize {
    body_shape_count_impl(id).max(0) as usize
}

#[inline]
pub(crate) fn body_shapes_into_impl(id: BodyId, out: &mut Vec<ShapeId>) {
    let cap = body_shape_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetShapes(id, ptr.cast(), cap)
        });
    }
}

#[inline]
pub(crate) fn body_shapes_impl(id: BodyId) -> Vec<ShapeId> {
    let cap = body_shape_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr: *mut ShapeId, cap| {
            ffi::b2Body_GetShapes(id, ptr.cast(), cap)
        })
    }
}

#[inline]
pub(crate) fn body_joint_count_impl(id: BodyId) -> i32 {
    unsafe { ffi::b2Body_GetJointCount(raw_body_id(id)) }
}

#[inline]
fn body_joint_capacity(id: BodyId) -> usize {
    body_joint_count_impl(id).max(0) as usize
}

#[inline]
pub(crate) fn body_joints_into_impl(id: BodyId, out: &mut Vec<JointId>) {
    let cap = body_joint_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetJoints(id, ptr.cast(), cap)
        });
    }
}

#[inline]
pub(crate) fn body_joints_impl(id: BodyId) -> Vec<JointId> {
    let cap = body_joint_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr: *mut JointId, cap| {
            ffi::b2Body_GetJoints(id, ptr.cast(), cap)
        })
    }
}

#[inline]
pub(crate) fn body_type_impl(id: BodyId) -> BodyType {
    BodyType::from_raw(unsafe { ffi::b2Body_GetType(raw_body_id(id)) })
}

#[inline]
fn body_set_type_impl(id: BodyId, body_type: BodyType) {
    unsafe { ffi::b2Body_SetType(raw_body_id(id), body_type.into_raw()) }
}

#[inline]
pub(crate) fn body_gravity_scale_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetGravityScale(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_gravity_scale_impl(id: BodyId, gravity_scale: f32) {
    unsafe { ffi::b2Body_SetGravityScale(raw_body_id(id), gravity_scale) }
}

#[inline]
pub(crate) fn body_linear_damping_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetLinearDamping(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_linear_damping_impl(id: BodyId, linear_damping: f32) {
    unsafe { ffi::b2Body_SetLinearDamping(raw_body_id(id), linear_damping) }
}

#[inline]
pub(crate) fn body_angular_damping_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetAngularDamping(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_angular_damping_impl(id: BodyId, angular_damping: f32) {
    unsafe { ffi::b2Body_SetAngularDamping(raw_body_id(id), angular_damping) }
}

#[inline]
pub(crate) fn body_enable_sleep_impl(id: BodyId, enable_sleep: bool) {
    unsafe { ffi::b2Body_EnableSleep(raw_body_id(id), enable_sleep) }
}

#[inline]
pub(crate) fn body_is_sleep_enabled_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsSleepEnabled(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_sleep_threshold_impl(id: BodyId, sleep_threshold: f32) {
    unsafe { ffi::b2Body_SetSleepThreshold(raw_body_id(id), sleep_threshold) }
}

#[inline]
pub(crate) fn body_sleep_threshold_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetSleepThreshold(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_is_awake_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsAwake(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_awake_impl(id: BodyId, awake: bool) {
    unsafe { ffi::b2Body_SetAwake(raw_body_id(id), awake) }
}

#[inline]
pub(crate) fn body_is_enabled_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsEnabled(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_enable_impl(id: BodyId) {
    unsafe { ffi::b2Body_Enable(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_disable_impl(id: BodyId) {
    unsafe { ffi::b2Body_Disable(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_is_bullet_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsBullet(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_bullet_impl(id: BodyId, bullet: bool) {
    unsafe { ffi::b2Body_SetBullet(raw_body_id(id), bullet) }
}

#[inline]
pub(crate) fn body_enable_contact_events_impl(id: BodyId, flag: bool) {
    unsafe { ffi::b2Body_EnableContactEvents(raw_body_id(id), flag) }
}

#[inline]
pub(crate) fn body_enable_hit_events_impl(id: BodyId, flag: bool) {
    unsafe { ffi::b2Body_EnableHitEvents(raw_body_id(id), flag) }
}

#[inline]
pub(crate) fn body_set_name_impl(id: BodyId, name: &CStr) {
    unsafe { ffi::b2Body_SetName(raw_body_id(id), name.as_ptr()) }
}

#[inline]
pub(crate) fn body_name_impl(id: BodyId) -> Option<String> {
    let name_ptr = unsafe { ffi::b2Body_GetName(raw_body_id(id)) };
    if name_ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}
