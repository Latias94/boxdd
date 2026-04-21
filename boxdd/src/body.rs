use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::core::world_core::WorldCore;
use crate::error::{ApiError, ApiResult};
use crate::query::Aabb;
use crate::types::{BodyId, ContactData, JointId, MassData, MotionLocks, ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;

/// A RAII-owned body that is destroyed on drop.
///
/// This handle is not `Send` so it cannot be dropped on another thread. It keeps the underlying
/// world alive via an internal reference-counted core.
pub struct OwnedBody {
    id: BodyId,
    core: Arc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

#[inline]
fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

fn body_contact_capacity(id: BodyId) -> usize {
    unsafe { ffi::b2Body_GetContactCapacity(raw_body_id(id)) }.max(0) as usize
}

fn body_contact_data_into_impl(id: BodyId, out: &mut Vec<ContactData>) {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        });
    }
}

fn body_contact_data_impl(id: BodyId) -> Vec<ContactData> {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi::<ContactData>(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        })
    }
}

fn body_contact_data_raw_into_impl(id: BodyId, out: &mut Vec<ffi::b2ContactData>) {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        });
    }
}

fn body_contact_data_raw_impl(id: BodyId) -> Vec<ffi::b2ContactData> {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        })
    }
}

fn body_contact_data_checked_impl(id: BodyId) -> Vec<ContactData> {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_impl(id)
}

fn body_contact_data_into_checked_impl(id: BodyId, out: &mut Vec<ContactData>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_into_impl(id, out);
}

fn try_body_contact_data_impl(id: BodyId) -> ApiResult<Vec<ContactData>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_contact_data_impl(id))
}

fn try_body_contact_data_into_impl(id: BodyId, out: &mut Vec<ContactData>) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_contact_data_into_impl(id, out);
    Ok(())
}

fn body_contact_data_raw_checked_impl(id: BodyId) -> Vec<ffi::b2ContactData> {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_raw_impl(id)
}

fn body_contact_data_raw_into_checked_impl(id: BodyId, out: &mut Vec<ffi::b2ContactData>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_raw_into_impl(id, out);
}

fn try_body_contact_data_raw_impl(id: BodyId) -> ApiResult<Vec<ffi::b2ContactData>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_contact_data_raw_impl(id))
}

fn try_body_contact_data_raw_into_impl(
    id: BodyId,
    out: &mut Vec<ffi::b2ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_contact_data_raw_into_impl(id, out);
    Ok(())
}

fn body_shape_count_checked_impl(id: BodyId) -> i32 {
    crate::core::debug_checks::assert_body_valid(id);
    body_shape_count_impl(id)
}

fn try_body_shape_count_impl(id: BodyId) -> ApiResult<i32> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_shape_count_impl(id))
}

fn body_shapes_checked_impl(id: BodyId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_body_valid(id);
    body_shapes_impl(id)
}

fn body_shapes_into_checked_impl(id: BodyId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_shapes_into_impl(id, out);
}

fn try_body_shapes_impl(id: BodyId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_shapes_impl(id))
}

fn try_body_shapes_into_impl(id: BodyId, out: &mut Vec<ShapeId>) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_shapes_into_impl(id, out);
    Ok(())
}

fn body_joint_count_checked_impl(id: BodyId) -> i32 {
    crate::core::debug_checks::assert_body_valid(id);
    body_joint_count_impl(id)
}

fn try_body_joint_count_impl(id: BodyId) -> ApiResult<i32> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_joint_count_impl(id))
}

fn body_joints_checked_impl(id: BodyId) -> Vec<JointId> {
    crate::core::debug_checks::assert_body_valid(id);
    body_joints_impl(id)
}

fn body_joints_into_checked_impl(id: BodyId, out: &mut Vec<JointId>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_joints_into_impl(id, out);
}

fn try_body_joints_impl(id: BodyId) -> ApiResult<Vec<JointId>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_joints_impl(id))
}

fn try_body_joints_into_impl(id: BodyId, out: &mut Vec<JointId>) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_joints_into_impl(id, out);
    Ok(())
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

unsafe fn body_set_user_data_ptr_impl(world_core: &WorldCore, id: BodyId, user_data: *mut c_void) {
    let _ = world_core.clear_body_user_data(id);
    unsafe { ffi::b2Body_SetUserData(raw_body_id(id), user_data) }
}

#[inline]
fn body_user_data_ptr_impl(id: BodyId) -> *mut c_void {
    unsafe { ffi::b2Body_GetUserData(raw_body_id(id)) }
}

fn body_set_user_data_impl<T: 'static>(world_core: &WorldCore, id: BodyId, value: T) {
    let user_data = world_core.set_body_user_data(id, value);
    unsafe { ffi::b2Body_SetUserData(raw_body_id(id), user_data) };
}

fn body_clear_user_data_impl(world_core: &WorldCore, id: BodyId) -> bool {
    let had = world_core.clear_body_user_data(id);
    if had {
        unsafe { ffi::b2Body_SetUserData(raw_body_id(id), core::ptr::null_mut()) };
    }
    had
}

fn body_with_user_data_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_body_user_data(id, f)
}

fn body_with_user_data_mut_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_body_user_data_mut(id, f)
}

fn body_take_user_data_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<Option<T>> {
    let value = world_core.take_body_user_data::<T>(id)?;
    if value.is_some() {
        unsafe { ffi::b2Body_SetUserData(raw_body_id(id), core::ptr::null_mut()) };
    }
    Ok(value)
}

fn body_world_id_checked_impl(id: BodyId) -> ffi::b2WorldId {
    crate::core::debug_checks::assert_body_valid(id);
    body_world_id_impl(id)
}

fn try_body_world_id_raw_impl(id: BodyId) -> ApiResult<ffi::b2WorldId> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_world_id_impl(id))
}

fn body_is_valid_checked_impl(id: BodyId) -> bool {
    crate::core::callback_state::assert_not_in_callback();
    body_is_valid_impl(id)
}

fn try_body_is_valid_impl(id: BodyId) -> ApiResult<bool> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(body_is_valid_impl(id))
}

unsafe fn body_set_user_data_ptr_raw_checked_impl(
    world_core: &WorldCore,
    id: BodyId,
    p: *mut c_void,
) {
    crate::core::debug_checks::assert_body_valid(id);
    unsafe { body_set_user_data_ptr_impl(world_core, id, p) }
}

unsafe fn try_body_set_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: BodyId,
    p: *mut c_void,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    unsafe { body_set_user_data_ptr_impl(world_core, id, p) }
    Ok(())
}

fn body_user_data_ptr_raw_checked_impl(id: BodyId) -> *mut c_void {
    crate::core::debug_checks::assert_body_valid(id);
    body_user_data_ptr_impl(id)
}

fn try_body_user_data_ptr_raw_impl(id: BodyId) -> ApiResult<*mut c_void> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_user_data_ptr_impl(id))
}

fn body_set_user_data_checked_impl<T: 'static>(world_core: &WorldCore, id: BodyId, value: T) {
    crate::core::debug_checks::assert_body_valid(id);
    body_set_user_data_impl(world_core, id, value);
}

fn try_body_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
    value: T,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_set_user_data_impl(world_core, id, value);
    Ok(())
}

fn body_clear_user_data_checked_impl(world_core: &WorldCore, id: BodyId) -> bool {
    crate::core::debug_checks::assert_body_valid(id);
    body_clear_user_data_impl(world_core, id)
}

fn try_body_clear_user_data_checked_impl(world_core: &WorldCore, id: BodyId) -> ApiResult<bool> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_clear_user_data_impl(world_core, id))
}

fn body_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_body_valid(id);
    body_with_user_data_impl(world_core, id, f).expect("user data type mismatch")
}

fn try_body_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_with_user_data_impl(world_core, id, f)
}

fn body_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_body_valid(id);
    body_with_user_data_mut_impl(world_core, id, f).expect("user data type mismatch")
}

fn try_body_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_with_user_data_mut_impl(world_core, id, f)
}

fn body_take_user_data_checked_impl<T: 'static>(world_core: &WorldCore, id: BodyId) -> Option<T> {
    crate::core::debug_checks::assert_body_valid(id);
    body_take_user_data_impl(world_core, id).expect("user data type mismatch")
}

fn try_body_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<Option<T>> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_take_user_data_impl(world_core, id)
}

impl OwnedBody {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: BodyId) -> Self {
        core.owned_bodies
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            core,
            destroy_on_drop: true,
            _not_send: PhantomData,
        }
    }

    pub fn id(&self) -> BodyId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        body_world_id_checked_impl(self.id)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        try_body_world_id_raw_impl(self.id)
    }

    pub fn is_valid(&self) -> bool {
        body_is_valid_checked_impl(self.id)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        try_body_is_valid_impl(self.id)
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_body_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_body_valid(self.id)
    }

    pub fn position(&self) -> Vec2 {
        self.assert_valid();
        body_position_impl(self.id)
    }

    pub fn try_position(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_position_impl(self.id))
    }

    pub fn linear_velocity(&self) -> Vec2 {
        self.assert_valid();
        body_linear_velocity_impl(self.id)
    }

    pub fn try_linear_velocity(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_linear_velocity_impl(self.id))
    }

    pub fn angular_velocity(&self) -> f32 {
        self.assert_valid();
        body_angular_velocity_impl(self.id)
    }

    pub fn try_angular_velocity(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_angular_velocity_impl(self.id))
    }

    pub fn rotation(&self) -> crate::Rot {
        self.assert_valid();
        body_rotation_impl(self.id)
    }

    pub fn try_rotation(&self) -> ApiResult<crate::Rot> {
        self.check_valid()?;
        Ok(body_rotation_impl(self.id))
    }

    pub fn rotation_raw(&self) -> ffi::b2Rot {
        self.assert_valid();
        body_rotation_raw_impl(self.id)
    }

    pub fn try_rotation_raw(&self) -> ApiResult<ffi::b2Rot> {
        self.check_valid()?;
        Ok(body_rotation_raw_impl(self.id))
    }

    pub fn transform(&self) -> crate::Transform {
        self.assert_valid();
        body_transform_impl(self.id)
    }

    pub fn try_transform(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(body_transform_impl(self.id))
    }

    pub fn transform_raw(&self) -> ffi::b2Transform {
        self.assert_valid();
        body_transform_raw_impl(self.id)
    }

    pub fn try_transform_raw(&self) -> ApiResult<ffi::b2Transform> {
        self.check_valid()?;
        Ok(body_transform_raw_impl(self.id))
    }

    pub fn aabb(&self) -> Aabb {
        self.assert_valid();
        body_aabb_impl(self.id)
    }

    pub fn try_aabb(&self) -> ApiResult<Aabb> {
        self.check_valid()?;
        Ok(body_aabb_impl(self.id))
    }

    pub fn local_point<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        body_local_point_impl(self.id, world_point)
    }

    pub fn try_local_point<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_point_impl(self.id, world_point))
    }

    pub fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        body_world_point_impl(self.id, local_point)
    }

    pub fn try_world_point<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_point_impl(self.id, local_point))
    }

    pub fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Vec2 {
        self.assert_valid();
        body_local_vector_impl(self.id, world_vector)
    }

    pub fn try_local_vector<V: Into<Vec2>>(&self, world_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_vector_impl(self.id, world_vector))
    }

    pub fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Vec2 {
        self.assert_valid();
        body_world_vector_impl(self.id, local_vector)
    }

    pub fn try_world_vector<V: Into<Vec2>>(&self, local_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_vector_impl(self.id, local_vector))
    }

    pub fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        body_local_point_velocity_impl(self.id, local_point)
    }

    pub fn try_local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_point_velocity_impl(self.id, local_point))
    }

    pub fn world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        body_world_point_velocity_impl(self.id, world_point)
    }

    pub fn try_world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_point_velocity_impl(self.id, world_point))
    }

    pub fn set_position_and_rotation<V: Into<Vec2>>(&mut self, p: V, angle_radians: f32) {
        self.assert_valid();
        body_set_position_and_rotation_impl(self.id, p, angle_radians);
    }

    pub fn try_set_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        p: V,
        angle_radians: f32,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_set_position_and_rotation_impl(self.id, p, angle_radians);
        Ok(())
    }

    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        self.assert_valid();
        body_set_linear_velocity_impl(self.id, v)
    }

    pub fn try_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        self.check_valid()?;
        body_set_linear_velocity_impl(self.id, v);
        Ok(())
    }

    pub fn set_angular_velocity(&mut self, w: f32) {
        self.assert_valid();
        body_set_angular_velocity_impl(self.id, w)
    }

    pub fn try_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        self.check_valid()?;
        body_set_angular_velocity_impl(self.id, w);
        Ok(())
    }

    pub fn set_target_transform(&mut self, target: crate::Transform, time_step: f32, wake: bool) {
        self.assert_valid();
        body_set_target_transform_impl(self.id, target, time_step, wake);
    }

    pub fn try_set_target_transform(
        &mut self,
        target: crate::Transform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_set_target_transform_impl(self.id, target, time_step, wake);
        Ok(())
    }

    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        self.assert_valid();
        body_apply_force_to_center_impl(self.id, force, wake);
    }

    pub fn try_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        force: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_force_to_center_impl(self.id, force, wake);
        Ok(())
    }

    pub fn apply_force<F: Into<Vec2>, P: Into<Vec2>>(&mut self, force: F, point: P, wake: bool) {
        self.assert_valid();
        body_apply_force_impl(self.id, force, point, wake);
    }

    pub fn try_apply_force<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_force_impl(self.id, force, point, wake);
        Ok(())
    }
    pub fn apply_torque(&mut self, torque: f32, wake: bool) {
        self.assert_valid();
        body_apply_torque_impl(self.id, torque, wake)
    }

    pub fn try_apply_torque(&mut self, torque: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_torque_impl(self.id, torque, wake);
        Ok(())
    }

    pub fn clear_forces(&mut self) {
        self.assert_valid();
        body_clear_forces_impl(self.id);
    }

    pub fn try_clear_forces(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_clear_forces_impl(self.id);
        Ok(())
    }
    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        self.assert_valid();
        body_apply_linear_impulse_to_center_impl(self.id, impulse, wake);
    }

    pub fn try_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_linear_impulse_to_center_impl(self.id, impulse, wake);
        Ok(())
    }

    pub fn apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        self.assert_valid();
        body_apply_linear_impulse_impl(self.id, impulse, point, wake);
    }
    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        self.assert_valid();
        body_apply_angular_impulse_impl(self.id, impulse, wake)
    }

    pub fn try_apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_linear_impulse_impl(self.id, impulse, point, wake);
        Ok(())
    }

    pub fn try_apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_angular_impulse_impl(self.id, impulse, wake);
        Ok(())
    }

    pub fn mass(&self) -> f32 {
        self.assert_valid();
        body_mass_impl(self.id)
    }

    pub fn try_mass(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_mass_impl(self.id))
    }

    pub fn rotational_inertia(&self) -> f32 {
        self.assert_valid();
        body_rotational_inertia_impl(self.id)
    }

    pub fn try_rotational_inertia(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_rotational_inertia_impl(self.id))
    }

    pub fn local_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        body_local_center_of_mass_impl(self.id)
    }

    pub fn try_local_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_center_of_mass_impl(self.id))
    }

    pub fn world_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        body_world_center_of_mass_impl(self.id)
    }

    pub fn try_world_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_center_of_mass_impl(self.id))
    }

    pub fn mass_data(&self) -> MassData {
        self.assert_valid();
        body_mass_data_impl(self.id)
    }

    pub fn try_mass_data(&self) -> ApiResult<MassData> {
        self.check_valid()?;
        Ok(body_mass_data_impl(self.id))
    }

    pub fn set_mass_data(&mut self, mass_data: MassData) {
        self.assert_valid();
        assert_mass_data_valid(mass_data);
        body_set_mass_data_impl(self.id, mass_data);
    }

    pub fn try_set_mass_data(&mut self, mass_data: MassData) -> ApiResult<()> {
        self.check_valid()?;
        check_mass_data_valid(mass_data)?;
        body_set_mass_data_impl(self.id, mass_data);
        Ok(())
    }

    pub fn apply_mass_from_shapes(&mut self) {
        self.assert_valid();
        body_apply_mass_from_shapes_impl(self.id);
    }

    pub fn try_apply_mass_from_shapes(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_mass_from_shapes_impl(self.id);
        Ok(())
    }

    pub fn shape_count(&self) -> i32 {
        body_shape_count_checked_impl(self.id)
    }

    pub fn try_shape_count(&self) -> ApiResult<i32> {
        try_body_shape_count_impl(self.id)
    }

    pub fn shapes(&self) -> Vec<ShapeId> {
        body_shapes_checked_impl(self.id)
    }

    pub fn shapes_into(&self, out: &mut Vec<ShapeId>) {
        body_shapes_into_checked_impl(self.id, out);
    }

    pub fn try_shapes(&self) -> ApiResult<Vec<ShapeId>> {
        try_body_shapes_impl(self.id)
    }

    pub fn try_shapes_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_body_shapes_into_impl(self.id, out)
    }

    pub fn joint_count(&self) -> i32 {
        body_joint_count_checked_impl(self.id)
    }

    pub fn try_joint_count(&self) -> ApiResult<i32> {
        try_body_joint_count_impl(self.id)
    }

    pub fn joints(&self) -> Vec<JointId> {
        body_joints_checked_impl(self.id)
    }

    pub fn joints_into(&self, out: &mut Vec<JointId>) {
        body_joints_into_checked_impl(self.id, out);
    }

    pub fn try_joints(&self) -> ApiResult<Vec<JointId>> {
        try_body_joints_impl(self.id)
    }

    pub fn try_joints_into(&self, out: &mut Vec<JointId>) -> ApiResult<()> {
        try_body_joints_into_impl(self.id, out)
    }

    pub fn body_type(&self) -> BodyType {
        self.assert_valid();
        body_type_impl(self.id)
    }

    pub fn try_body_type(&self) -> ApiResult<BodyType> {
        self.check_valid()?;
        Ok(body_type_impl(self.id))
    }
    pub fn set_body_type(&mut self, t: BodyType) {
        self.assert_valid();
        body_set_type_impl(self.id, t)
    }

    pub fn try_set_body_type(&mut self, t: BodyType) -> ApiResult<()> {
        self.check_valid()?;
        body_set_type_impl(self.id, t);
        Ok(())
    }

    pub fn gravity_scale(&self) -> f32 {
        self.assert_valid();
        body_gravity_scale_impl(self.id)
    }
    pub fn try_gravity_scale(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_gravity_scale_impl(self.id))
    }
    pub fn set_gravity_scale(&mut self, v: f32) {
        self.assert_valid();
        assert!(
            crate::is_valid_float(v),
            "gravity_scale must be finite, got {v}"
        );
        body_set_gravity_scale_impl(self.id, v)
    }

    pub fn try_set_gravity_scale(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        if !crate::is_valid_float(v) {
            return Err(ApiError::InvalidArgument);
        }
        body_set_gravity_scale_impl(self.id, v);
        Ok(())
    }

    pub fn linear_damping(&self) -> f32 {
        self.assert_valid();
        body_linear_damping_impl(self.id)
    }
    pub fn try_linear_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_linear_damping_impl(self.id))
    }
    pub fn set_linear_damping(&mut self, v: f32) {
        self.assert_valid();
        assert_non_negative_finite_body_scalar("linear_damping", v);
        body_set_linear_damping_impl(self.id, v)
    }
    pub fn try_set_linear_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_body_scalar(v)?;
        body_set_linear_damping_impl(self.id, v);
        Ok(())
    }
    pub fn angular_damping(&self) -> f32 {
        self.assert_valid();
        body_angular_damping_impl(self.id)
    }
    pub fn try_angular_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_angular_damping_impl(self.id))
    }
    pub fn set_angular_damping(&mut self, v: f32) {
        self.assert_valid();
        assert_non_negative_finite_body_scalar("angular_damping", v);
        body_set_angular_damping_impl(self.id, v)
    }
    pub fn try_set_angular_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_body_scalar(v)?;
        body_set_angular_damping_impl(self.id, v);
        Ok(())
    }

    pub fn enable_sleep(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_sleep_impl(self.id, flag)
    }

    pub fn try_enable_sleep(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_sleep_impl(self.id, flag);
        Ok(())
    }

    pub fn is_sleep_enabled(&self) -> bool {
        self.assert_valid();
        body_is_sleep_enabled_impl(self.id)
    }

    pub fn try_is_sleep_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_sleep_enabled_impl(self.id))
    }

    pub fn set_sleep_threshold(&mut self, sleep_threshold: f32) {
        self.assert_valid();
        body_set_sleep_threshold_impl(self.id, sleep_threshold)
    }

    pub fn try_set_sleep_threshold(&mut self, sleep_threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        body_set_sleep_threshold_impl(self.id, sleep_threshold);
        Ok(())
    }

    pub fn sleep_threshold(&self) -> f32 {
        self.assert_valid();
        body_sleep_threshold_impl(self.id)
    }

    pub fn try_sleep_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_sleep_threshold_impl(self.id))
    }

    pub fn is_awake(&self) -> bool {
        self.assert_valid();
        body_is_awake_impl(self.id)
    }
    pub fn try_is_awake(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_awake_impl(self.id))
    }
    pub fn set_awake(&mut self, awake: bool) {
        self.assert_valid();
        body_set_awake_impl(self.id, awake)
    }
    pub fn try_set_awake(&mut self, awake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_set_awake_impl(self.id, awake);
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.assert_valid();
        body_is_enabled_impl(self.id)
    }
    pub fn try_is_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_enabled_impl(self.id))
    }
    pub fn enable(&mut self) {
        self.assert_valid();
        body_enable_impl(self.id)
    }
    pub fn try_enable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_impl(self.id);
        Ok(())
    }
    pub fn disable(&mut self) {
        self.assert_valid();
        body_disable_impl(self.id)
    }
    pub fn try_disable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_disable_impl(self.id);
        Ok(())
    }

    pub fn is_bullet(&self) -> bool {
        self.assert_valid();
        body_is_bullet_impl(self.id)
    }
    pub fn try_is_bullet(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_bullet_impl(self.id))
    }
    pub fn set_bullet(&mut self, flag: bool) {
        self.assert_valid();
        body_set_bullet_impl(self.id, flag)
    }

    pub fn try_set_bullet(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_set_bullet_impl(self.id, flag);
        Ok(())
    }

    pub fn enable_contact_events(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_contact_events_impl(self.id, flag)
    }

    pub fn try_enable_contact_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_contact_events_impl(self.id, flag);
        Ok(())
    }

    pub fn enable_hit_events(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_hit_events_impl(self.id, flag)
    }

    pub fn try_enable_hit_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_hit_events_impl(self.id, flag);
        Ok(())
    }

    pub fn set_name(&mut self, name: &str) {
        self.assert_valid();
        let cs = CString::new(name).expect("body name contains an interior NUL byte");
        body_set_name_impl(self.id, &cs)
    }

    pub fn try_set_name(&mut self, name: &str) -> ApiResult<()> {
        self.check_valid()?;
        let cs = CString::new(name).map_err(|_| ApiError::NulByteInString)?;
        body_set_name_impl(self.id, &cs);
        Ok(())
    }

    pub fn name(&self) -> Option<String> {
        self.assert_valid();
        body_name_impl(self.id)
    }

    pub fn try_name(&self) -> ApiResult<Option<String>> {
        self.check_valid()?;
        Ok(body_name_impl(self.id))
    }

    pub fn contact_data(&self) -> Vec<ContactData> {
        body_contact_data_checked_impl(self.id)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        body_contact_data_into_checked_impl(self.id, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        try_body_contact_data_impl(self.id)
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        try_body_contact_data_into_impl(self.id, out)
    }

    pub fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        body_contact_data_raw_checked_impl(self.id)
    }

    pub fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        body_contact_data_raw_into_checked_impl(self.id, out);
    }

    pub fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        try_body_contact_data_raw_impl(self.id)
    }

    pub fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        try_body_contact_data_raw_into_impl(self.id, out)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> BodyId {
        self.id
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// The caller must ensure that `p` is either null or points to a valid object
    /// for the entire time the body may access it, and that any lifetimes/aliasing rules
    /// are upheld. Box2D treats this as an opaque pointer and may store/use it across steps.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { body_set_user_data_ptr_raw_checked_impl(self.core.as_ref(), self.id, p) }
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { try_body_set_user_data_ptr_raw_impl(self.core.as_ref(), self.id, p) }
    }
    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        body_user_data_ptr_raw_checked_impl(self.id)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_body_user_data_ptr_raw_impl(self.id)
    }

    /// Set typed user data on this body.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the body is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        body_set_user_data_checked_impl(self.core.as_ref(), self.id, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_body_set_user_data_checked_impl(self.core.as_ref(), self.id, value)
    }

    /// Clear typed user data on this body. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        body_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_body_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        body_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        try_body_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        body_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_body_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        body_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_body_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    /// Disarm RAII and return the raw id for manual lifetime management.
    pub fn into_id(mut self) -> BodyId {
        self.destroy_on_drop = false;
        self.id
    }

    /// Destroy the body immediately and disarm drop.
    pub fn destroy(mut self) {
        if self.destroy_on_drop && unsafe { ffi::b2Body_IsValid(raw_body_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Body(self.id));
            } else {
                #[cfg(feature = "serialize")]
                self.core.cleanup_before_destroy_body(self.id);
                unsafe { ffi::b2DestroyBody(raw_body_id(self.id)) };
                let _ = self.core.clear_body_user_data(self.id);
            }
        }
        self.destroy_on_drop = false;
    }
}

impl Drop for OwnedBody {
    fn drop(&mut self) {
        let _ = self.core.id;
        let prev = self
            .core
            .owned_bodies
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug_assert!(prev > 0, "owned body counter underflow");
        if self.destroy_on_drop && unsafe { ffi::b2Body_IsValid(raw_body_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Body(self.id));
            } else {
                #[cfg(feature = "serialize")]
                self.core.cleanup_before_destroy_body(self.id);
                unsafe { ffi::b2DestroyBody(raw_body_id(self.id)) };
                let _ = self.core.clear_body_user_data(self.id);
            }
        }
    }
}

/// Body types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

impl BodyType {
    #[inline]
    pub const fn into_raw(self) -> ffi::b2BodyType {
        match self {
            BodyType::Static => ffi::b2BodyType_b2_staticBody,
            BodyType::Kinematic => ffi::b2BodyType_b2_kinematicBody,
            BodyType::Dynamic => ffi::b2BodyType_b2_dynamicBody,
        }
    }

    #[inline]
    pub const fn from_raw(raw: ffi::b2BodyType) -> Self {
        match raw {
            x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
            x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
            _ => BodyType::Dynamic,
        }
    }
}

#[inline]
fn body_type_is_known(raw: ffi::b2BodyType) -> bool {
    raw == ffi::b2BodyType_b2_staticBody
        || raw == ffi::b2BodyType_b2_kinematicBody
        || raw == ffi::b2BodyType_b2_dynamicBody
}

#[inline]
fn body_def_cookie_is_valid(def: &BodyDef) -> bool {
    def.0.internalValue == unsafe { ffi::b2DefaultBodyDef() }.internalValue
}

#[inline]
fn assert_non_negative_finite_body_scalar(name: &str, value: f32) {
    assert!(
        value.is_finite() && value >= 0.0,
        "{name} must be finite and >= 0.0, got {value}"
    );
}

#[inline]
fn check_non_negative_finite_body_scalar(value: f32) -> ApiResult<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn assert_mass_data_valid(mass_data: MassData) {
    assert_non_negative_finite_body_scalar("mass", mass_data.mass);
    assert_non_negative_finite_body_scalar("rotational_inertia", mass_data.rotational_inertia);
    assert!(
        mass_data.center.is_valid(),
        "mass_data.center must be a valid Box2D vector, got {:?}",
        mass_data.center
    );
}

#[inline]
pub(crate) fn check_mass_data_valid(mass_data: MassData) -> ApiResult<()> {
    check_non_negative_finite_body_scalar(mass_data.mass)?;
    check_non_negative_finite_body_scalar(mass_data.rotational_inertia)?;
    if mass_data.center.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

pub(crate) fn assert_body_def_valid(def: &BodyDef) {
    assert!(
        body_def_cookie_is_valid(def),
        "invalid BodyDef: not initialized from b2DefaultBodyDef"
    );
    assert!(
        body_type_is_known(def.0.type_),
        "invalid BodyDef: unknown body type value {}",
        def.0.type_
    );
    assert!(
        Vec2::from_raw(def.0.position).is_valid(),
        "invalid BodyDef: position must be a valid Box2D vector"
    );
    assert!(
        crate::Rot::from_raw(def.0.rotation).is_valid(),
        "invalid BodyDef: rotation must be a valid Box2D rotation"
    );
    assert!(
        Vec2::from_raw(def.0.linearVelocity).is_valid(),
        "invalid BodyDef: linearVelocity must be a valid Box2D vector"
    );
    assert!(
        crate::is_valid_float(def.0.angularVelocity),
        "invalid BodyDef: angularVelocity must be finite"
    );
    assert_non_negative_finite_body_scalar("linearDamping", def.0.linearDamping);
    assert_non_negative_finite_body_scalar("angularDamping", def.0.angularDamping);
    assert_non_negative_finite_body_scalar("sleepThreshold", def.0.sleepThreshold);
    assert!(
        crate::is_valid_float(def.0.gravityScale),
        "invalid BodyDef: gravityScale must be finite"
    );
}

pub(crate) fn check_body_def_valid(def: &BodyDef) -> ApiResult<()> {
    if !body_def_cookie_is_valid(def)
        || !body_type_is_known(def.0.type_)
        || !Vec2::from_raw(def.0.position).is_valid()
        || !crate::Rot::from_raw(def.0.rotation).is_valid()
        || !Vec2::from_raw(def.0.linearVelocity).is_valid()
        || !crate::is_valid_float(def.0.angularVelocity)
        || check_non_negative_finite_body_scalar(def.0.linearDamping).is_err()
        || check_non_negative_finite_body_scalar(def.0.angularDamping).is_err()
        || check_non_negative_finite_body_scalar(def.0.sleepThreshold).is_err()
        || !crate::is_valid_float(def.0.gravityScale)
    {
        Err(ApiError::InvalidArgument)
    } else {
        Ok(())
    }
}

/// Body definition wrapper with builder API.
#[derive(Clone, Debug)]
pub struct BodyDef(pub(crate) ffi::b2BodyDef);

impl Default for BodyDef {
    fn default() -> Self {
        let def = unsafe { ffi::b2DefaultBodyDef() };
        Self(def)
    }
}

impl BodyDef {
    /// Start building a new `BodyDef` from defaults.
    pub fn builder() -> BodyBuilder {
        BodyBuilder::new()
    }

    /// Construct from the raw Box2D body definition value.
    #[inline]
    pub fn from_raw(raw: ffi::b2BodyDef) -> Self {
        Self(raw)
    }

    /// Body type used when the body is created.
    #[inline]
    pub fn body_type(&self) -> BodyType {
        BodyType::from_raw(self.0.type_)
    }

    /// Initial world-space position.
    #[inline]
    pub fn position(&self) -> Vec2 {
        Vec2::from_raw(self.0.position)
    }

    /// Initial rotation value.
    #[inline]
    pub fn rotation(&self) -> crate::Rot {
        crate::Rot::from_raw(self.0.rotation)
    }

    /// Initial angle in radians.
    #[inline]
    pub fn angle(&self) -> f32 {
        self.rotation().angle()
    }

    /// Initial linear velocity in m/s.
    #[inline]
    pub fn linear_velocity(&self) -> Vec2 {
        Vec2::from_raw(self.0.linearVelocity)
    }

    /// Initial angular velocity in rad/s.
    #[inline]
    pub fn angular_velocity(&self) -> f32 {
        self.0.angularVelocity
    }

    /// Linear damping.
    #[inline]
    pub fn linear_damping(&self) -> f32 {
        self.0.linearDamping
    }

    /// Angular damping.
    #[inline]
    pub fn angular_damping(&self) -> f32 {
        self.0.angularDamping
    }

    /// Per-body gravity scale.
    #[inline]
    pub fn gravity_scale(&self) -> f32 {
        self.0.gravityScale
    }

    /// Whether sleeping is enabled at creation.
    #[inline]
    pub fn is_sleep_enabled(&self) -> bool {
        self.0.enableSleep
    }

    /// Whether the body starts awake.
    #[inline]
    pub fn is_awake(&self) -> bool {
        self.0.isAwake
    }

    /// Whether the body starts as a bullet.
    #[inline]
    pub fn is_bullet(&self) -> bool {
        self.0.isBullet
    }

    /// Whether the body allows fast rotation without Box2D's default clamp.
    #[inline]
    pub fn is_fast_rotation_allowed(&self) -> bool {
        self.0.allowFastRotation
    }

    /// Whether the body starts enabled for simulation.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.0.isEnabled
    }

    /// Convert into the raw Box2D body definition value.
    #[inline]
    pub fn into_raw(self) -> ffi::b2BodyDef {
        self.0
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        check_body_def_valid(self)
    }
}

/// Fluent builder for `BodyDef`.
#[doc(alias = "body_builder")]
#[doc(alias = "bodybuilder")]
///
/// Chain methods to configure a body and finish with `build()`. This maps
/// to the upstream `b2BodyDef` fields.
#[derive(Clone, Debug)]
pub struct BodyBuilder {
    def: BodyDef,
}

impl BodyBuilder {
    /// Start a new builder with default `BodyDef`.
    pub fn new() -> Self {
        Self {
            def: BodyDef::default(),
        }
    }
    /// Set the body type (static, kinematic, dynamic).
    pub fn body_type(mut self, t: BodyType) -> Self {
        self.def.0.type_ = t.into_raw();
        self
    }
    /// Initial world-space position.
    pub fn position<V: Into<Vec2>>(mut self, p: V) -> Self {
        self.def.0.position = p.into().into_raw();
        self
    }
    /// Initial rotation in radians.
    pub fn angle(mut self, radians: f32) -> Self {
        // Build a rotation from angle
        let (s, c) = radians.sin_cos();
        self.def.0.rotation = ffi::b2Rot { c, s };
        self
    }
    /// Initial linear velocity (m/s).
    pub fn linear_velocity<V: Into<Vec2>>(mut self, v: V) -> Self {
        self.def.0.linearVelocity = v.into().into_raw();
        self
    }
    /// Initial angular velocity (rad/s).
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.def.0.angularVelocity = v;
        self
    }
    /// Linear damping (drag-like term).
    pub fn linear_damping(mut self, v: f32) -> Self {
        self.def.0.linearDamping = v;
        self
    }
    /// Angular damping.
    pub fn angular_damping(mut self, v: f32) -> Self {
        self.def.0.angularDamping = v;
        self
    }
    /// Per-body gravity scale (1 = normal gravity).
    pub fn gravity_scale(mut self, v: f32) -> Self {
        self.def.0.gravityScale = v;
        self
    }
    /// Allow body to go to sleep.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.0.enableSleep = flag;
        self
    }
    /// Awake/asleep flag at creation.
    pub fn awake(mut self, flag: bool) -> Self {
        self.def.0.isAwake = flag;
        self
    }
    /// Treat as bullet (CCD).
    pub fn bullet(mut self, flag: bool) -> Self {
        self.def.0.isBullet = flag;
        self
    }
    /// Allow high angular speed without Box2D's default clamp.
    pub fn allow_fast_rotation(mut self, flag: bool) -> Self {
        self.def.0.allowFastRotation = flag;
        self
    }
    /// Enable/disable simulation for this body.
    pub fn enabled(mut self, flag: bool) -> Self {
        self.def.0.isEnabled = flag;
        self
    }

    #[must_use]
    pub fn build(self) -> BodyDef {
        self.def
    }
}

impl From<BodyDef> for BodyBuilder {
    fn from(def: BodyDef) -> Self {
        Self { def }
    }
}

// serde support for BodyDef via a transparent config struct
#[cfg(feature = "serde")]
impl serde::Serialize for BodyDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            body_type: super::body::BodyType,
            position: crate::types::Vec2,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            allow_fast_rotation: bool,
            enabled: bool,
        }
        let angle = self.0.rotation.s.atan2(self.0.rotation.c);
        let r = Repr {
            body_type: match self.0.type_ {
                x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
                x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
                _ => BodyType::Dynamic,
            },
            position: crate::types::Vec2::from_raw(self.0.position),
            angle,
            linear_velocity: crate::types::Vec2::from_raw(self.0.linearVelocity),
            angular_velocity: self.0.angularVelocity,
            linear_damping: self.0.linearDamping,
            angular_damping: self.0.angularDamping,
            gravity_scale: self.0.gravityScale,
            enable_sleep: self.0.enableSleep,
            awake: self.0.isAwake,
            bullet: self.0.isBullet,
            allow_fast_rotation: self.0.allowFastRotation,
            enabled: self.0.isEnabled,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BodyDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            body_type: super::body::BodyType,
            position: crate::types::Vec2,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            allow_fast_rotation: bool,
            enabled: bool,
        }
        let r = Repr::deserialize(deserializer)?;
        let b = BodyBuilder::new()
            .body_type(r.body_type)
            .position(r.position)
            .angle(r.angle)
            .linear_velocity(r.linear_velocity)
            .angular_velocity(r.angular_velocity)
            .linear_damping(r.linear_damping)
            .angular_damping(r.angular_damping)
            .gravity_scale(r.gravity_scale)
            .enable_sleep(r.enable_sleep)
            .awake(r.awake)
            .bullet(r.bullet)
            .allow_fast_rotation(r.allow_fast_rotation)
            .enabled(r.enabled);
        Ok(b.build())
    }
}

impl Default for BodyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::BodyBuilder;

    #[test]
    fn body_builder_allow_fast_rotation_sets_raw_field() {
        assert!(!BodyBuilder::new().build().0.allowFastRotation);
        assert!(
            BodyBuilder::new()
                .allow_fast_rotation(true)
                .build()
                .0
                .allowFastRotation
        );
    }
}

/// A body handle with lifetime tied to the owning world.
pub struct Body<'w> {
    pub(crate) id: BodyId,
    pub(crate) core: Arc<WorldCore>,
    _world: PhantomData<&'w World>,
}

impl<'w> Body<'w> {
    pub(crate) fn new(core: Arc<WorldCore>, id: BodyId) -> Self {
        Self {
            id,
            core,
            _world: PhantomData,
        }
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_body_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_body_valid(self.id)
    }

    pub fn id(&self) -> BodyId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        body_world_id_checked_impl(self.id)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        try_body_world_id_raw_impl(self.id)
    }

    pub fn is_valid(&self) -> bool {
        body_is_valid_checked_impl(self.id)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        try_body_is_valid_impl(self.id)
    }

    // Queries
    pub fn position(&self) -> Vec2 {
        self.assert_valid();
        body_position_impl(self.id)
    }

    pub fn try_position(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_position_impl(self.id))
    }

    pub fn linear_velocity(&self) -> Vec2 {
        self.assert_valid();
        body_linear_velocity_impl(self.id)
    }

    pub fn try_linear_velocity(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_linear_velocity_impl(self.id))
    }

    pub fn angular_velocity(&self) -> f32 {
        self.assert_valid();
        body_angular_velocity_impl(self.id)
    }

    pub fn try_angular_velocity(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_angular_velocity_impl(self.id))
    }

    pub fn rotation(&self) -> crate::Rot {
        self.assert_valid();
        body_rotation_impl(self.id)
    }

    pub fn try_rotation(&self) -> ApiResult<crate::Rot> {
        self.check_valid()?;
        Ok(body_rotation_impl(self.id))
    }

    pub fn rotation_raw(&self) -> ffi::b2Rot {
        self.assert_valid();
        body_rotation_raw_impl(self.id)
    }

    pub fn try_rotation_raw(&self) -> ApiResult<ffi::b2Rot> {
        self.check_valid()?;
        Ok(body_rotation_raw_impl(self.id))
    }

    pub fn transform(&self) -> crate::Transform {
        self.assert_valid();
        body_transform_impl(self.id)
    }

    pub fn try_transform(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(body_transform_impl(self.id))
    }

    pub fn transform_raw(&self) -> ffi::b2Transform {
        self.assert_valid();
        body_transform_raw_impl(self.id)
    }

    pub fn try_transform_raw(&self) -> ApiResult<ffi::b2Transform> {
        self.check_valid()?;
        Ok(body_transform_raw_impl(self.id))
    }

    pub fn aabb(&self) -> Aabb {
        self.assert_valid();
        body_aabb_impl(self.id)
    }

    pub fn try_aabb(&self) -> ApiResult<Aabb> {
        self.check_valid()?;
        Ok(body_aabb_impl(self.id))
    }

    pub fn local_point<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        body_local_point_impl(self.id, world_point)
    }

    pub fn try_local_point<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_point_impl(self.id, world_point))
    }

    pub fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        body_world_point_impl(self.id, local_point)
    }

    pub fn try_world_point<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_point_impl(self.id, local_point))
    }

    pub fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Vec2 {
        self.assert_valid();
        body_local_vector_impl(self.id, world_vector)
    }

    pub fn try_local_vector<V: Into<Vec2>>(&self, world_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_vector_impl(self.id, world_vector))
    }

    pub fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Vec2 {
        self.assert_valid();
        body_world_vector_impl(self.id, local_vector)
    }

    pub fn try_world_vector<V: Into<Vec2>>(&self, local_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_vector_impl(self.id, local_vector))
    }

    pub fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        body_local_point_velocity_impl(self.id, local_point)
    }

    pub fn try_local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_point_velocity_impl(self.id, local_point))
    }

    pub fn world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        body_world_point_velocity_impl(self.id, world_point)
    }

    pub fn try_world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_point_velocity_impl(self.id, world_point))
    }

    // Mutations
    pub fn set_position_and_rotation<V: Into<Vec2>>(&mut self, p: V, angle_radians: f32) {
        self.assert_valid();
        body_set_position_and_rotation_impl(self.id, p, angle_radians);
    }
    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        self.assert_valid();
        body_set_linear_velocity_impl(self.id, v)
    }

    pub fn try_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        self.check_valid()?;
        body_set_linear_velocity_impl(self.id, v);
        Ok(())
    }

    pub fn set_angular_velocity(&mut self, w: f32) {
        self.assert_valid();
        body_set_angular_velocity_impl(self.id, w)
    }

    pub fn try_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        self.check_valid()?;
        body_set_angular_velocity_impl(self.id, w);
        Ok(())
    }

    pub fn set_target_transform(&mut self, target: crate::Transform, time_step: f32, wake: bool) {
        self.assert_valid();
        body_set_target_transform_impl(self.id, target, time_step, wake);
    }

    pub fn try_set_target_transform(
        &mut self,
        target: crate::Transform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_set_target_transform_impl(self.id, target, time_step, wake);
        Ok(())
    }

    pub fn contact_data(&self) -> Vec<ContactData> {
        body_contact_data_checked_impl(self.id)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        body_contact_data_into_checked_impl(self.id, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        try_body_contact_data_impl(self.id)
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        try_body_contact_data_into_impl(self.id, out)
    }

    pub fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        body_contact_data_raw_checked_impl(self.id)
    }

    pub fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        body_contact_data_raw_into_checked_impl(self.id, out);
    }

    pub fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        try_body_contact_data_raw_impl(self.id)
    }

    pub fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        try_body_contact_data_raw_into_impl(self.id, out)
    }

    // Forces/impulses
    pub fn apply_force<F: Into<Vec2>, P: Into<Vec2>>(&mut self, force: F, point: P, wake: bool) {
        self.assert_valid();
        body_apply_force_impl(self.id, force, point, wake);
    }

    pub fn try_apply_force<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_force_impl(self.id, force, point, wake);
        Ok(())
    }
    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        self.assert_valid();
        body_apply_force_to_center_impl(self.id, force, wake);
    }

    pub fn try_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        force: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_force_to_center_impl(self.id, force, wake);
        Ok(())
    }
    pub fn apply_torque(&mut self, torque: f32, wake: bool) {
        self.assert_valid();
        body_apply_torque_impl(self.id, torque, wake)
    }

    pub fn try_apply_torque(&mut self, torque: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_torque_impl(self.id, torque, wake);
        Ok(())
    }

    pub fn clear_forces(&mut self) {
        self.assert_valid();
        body_clear_forces_impl(self.id);
    }

    pub fn try_clear_forces(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_clear_forces_impl(self.id);
        Ok(())
    }

    pub fn apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        self.assert_valid();
        body_apply_linear_impulse_impl(self.id, impulse, point, wake);
    }

    pub fn try_apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_linear_impulse_impl(self.id, impulse, point, wake);
        Ok(())
    }
    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        self.assert_valid();
        body_apply_linear_impulse_to_center_impl(self.id, impulse, wake);
    }

    pub fn try_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_linear_impulse_to_center_impl(self.id, impulse, wake);
        Ok(())
    }
    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        self.assert_valid();
        body_apply_angular_impulse_impl(self.id, impulse, wake)
    }

    pub fn try_apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_angular_impulse_impl(self.id, impulse, wake);
        Ok(())
    }

    pub fn mass(&self) -> f32 {
        self.assert_valid();
        body_mass_impl(self.id)
    }

    pub fn try_mass(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_mass_impl(self.id))
    }

    pub fn rotational_inertia(&self) -> f32 {
        self.assert_valid();
        body_rotational_inertia_impl(self.id)
    }

    pub fn try_rotational_inertia(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_rotational_inertia_impl(self.id))
    }

    pub fn local_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        body_local_center_of_mass_impl(self.id)
    }

    pub fn try_local_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_center_of_mass_impl(self.id))
    }

    pub fn world_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        body_world_center_of_mass_impl(self.id)
    }

    pub fn try_world_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_center_of_mass_impl(self.id))
    }

    pub fn mass_data(&self) -> MassData {
        self.assert_valid();
        body_mass_data_impl(self.id)
    }

    pub fn try_mass_data(&self) -> ApiResult<MassData> {
        self.check_valid()?;
        Ok(body_mass_data_impl(self.id))
    }

    pub fn set_mass_data(&mut self, mass_data: MassData) {
        self.assert_valid();
        assert_mass_data_valid(mass_data);
        body_set_mass_data_impl(self.id, mass_data);
    }

    pub fn try_set_mass_data(&mut self, mass_data: MassData) -> ApiResult<()> {
        self.check_valid()?;
        check_mass_data_valid(mass_data)?;
        body_set_mass_data_impl(self.id, mass_data);
        Ok(())
    }

    pub fn apply_mass_from_shapes(&mut self) {
        self.assert_valid();
        body_apply_mass_from_shapes_impl(self.id);
    }

    pub fn try_apply_mass_from_shapes(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_mass_from_shapes_impl(self.id);
        Ok(())
    }

    pub fn shape_count(&self) -> i32 {
        body_shape_count_checked_impl(self.id)
    }

    pub fn try_shape_count(&self) -> ApiResult<i32> {
        try_body_shape_count_impl(self.id)
    }

    pub fn shapes(&self) -> Vec<ShapeId> {
        body_shapes_checked_impl(self.id)
    }

    pub fn shapes_into(&self, out: &mut Vec<ShapeId>) {
        body_shapes_into_checked_impl(self.id, out);
    }

    pub fn try_shapes(&self) -> ApiResult<Vec<ShapeId>> {
        try_body_shapes_impl(self.id)
    }

    pub fn try_shapes_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_body_shapes_into_impl(self.id, out)
    }

    pub fn joint_count(&self) -> i32 {
        body_joint_count_checked_impl(self.id)
    }

    pub fn try_joint_count(&self) -> ApiResult<i32> {
        try_body_joint_count_impl(self.id)
    }

    pub fn joints(&self) -> Vec<JointId> {
        body_joints_checked_impl(self.id)
    }

    pub fn joints_into(&self, out: &mut Vec<JointId>) {
        body_joints_into_checked_impl(self.id, out);
    }

    pub fn try_joints(&self) -> ApiResult<Vec<JointId>> {
        try_body_joints_impl(self.id)
    }

    pub fn try_joints_into(&self, out: &mut Vec<JointId>) -> ApiResult<()> {
        try_body_joints_into_impl(self.id, out)
    }

    pub fn body_type(&self) -> BodyType {
        self.assert_valid();
        body_type_impl(self.id)
    }

    pub fn try_body_type(&self) -> ApiResult<BodyType> {
        self.check_valid()?;
        Ok(body_type_impl(self.id))
    }
    pub fn set_body_type(&mut self, t: BodyType) {
        self.assert_valid();
        body_set_type_impl(self.id, t)
    }

    pub fn try_set_body_type(&mut self, t: BodyType) -> ApiResult<()> {
        self.check_valid()?;
        body_set_type_impl(self.id, t);
        Ok(())
    }

    pub fn gravity_scale(&self) -> f32 {
        self.assert_valid();
        body_gravity_scale_impl(self.id)
    }
    pub fn try_gravity_scale(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_gravity_scale_impl(self.id))
    }
    pub fn set_gravity_scale(&mut self, v: f32) {
        self.assert_valid();
        assert!(
            crate::is_valid_float(v),
            "gravity_scale must be finite, got {v}"
        );
        body_set_gravity_scale_impl(self.id, v)
    }

    pub fn try_set_gravity_scale(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        if !crate::is_valid_float(v) {
            return Err(ApiError::InvalidArgument);
        }
        body_set_gravity_scale_impl(self.id, v);
        Ok(())
    }

    pub fn linear_damping(&self) -> f32 {
        self.assert_valid();
        body_linear_damping_impl(self.id)
    }
    pub fn try_linear_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_linear_damping_impl(self.id))
    }
    pub fn set_linear_damping(&mut self, v: f32) {
        self.assert_valid();
        assert_non_negative_finite_body_scalar("linear_damping", v);
        body_set_linear_damping_impl(self.id, v)
    }
    pub fn try_set_linear_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_body_scalar(v)?;
        body_set_linear_damping_impl(self.id, v);
        Ok(())
    }
    pub fn angular_damping(&self) -> f32 {
        self.assert_valid();
        body_angular_damping_impl(self.id)
    }
    pub fn try_angular_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_angular_damping_impl(self.id))
    }
    pub fn set_angular_damping(&mut self, v: f32) {
        self.assert_valid();
        assert_non_negative_finite_body_scalar("angular_damping", v);
        body_set_angular_damping_impl(self.id, v)
    }

    pub fn try_set_angular_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_body_scalar(v)?;
        body_set_angular_damping_impl(self.id, v);
        Ok(())
    }

    pub fn enable_sleep(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_sleep_impl(self.id, flag)
    }

    pub fn try_enable_sleep(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_sleep_impl(self.id, flag);
        Ok(())
    }

    pub fn is_sleep_enabled(&self) -> bool {
        self.assert_valid();
        body_is_sleep_enabled_impl(self.id)
    }

    pub fn try_is_sleep_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_sleep_enabled_impl(self.id))
    }

    pub fn set_sleep_threshold(&mut self, sleep_threshold: f32) {
        self.assert_valid();
        body_set_sleep_threshold_impl(self.id, sleep_threshold)
    }

    pub fn try_set_sleep_threshold(&mut self, sleep_threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        body_set_sleep_threshold_impl(self.id, sleep_threshold);
        Ok(())
    }

    pub fn sleep_threshold(&self) -> f32 {
        self.assert_valid();
        body_sleep_threshold_impl(self.id)
    }

    pub fn try_sleep_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_sleep_threshold_impl(self.id))
    }

    pub fn is_awake(&self) -> bool {
        self.assert_valid();
        body_is_awake_impl(self.id)
    }
    pub fn set_awake(&mut self, awake: bool) {
        self.assert_valid();
        body_set_awake_impl(self.id, awake)
    }

    pub fn try_is_awake(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_awake_impl(self.id))
    }

    pub fn try_set_awake(&mut self, awake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_set_awake_impl(self.id, awake);
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.assert_valid();
        body_is_enabled_impl(self.id)
    }
    pub fn enable(&mut self) {
        self.assert_valid();
        body_enable_impl(self.id)
    }
    pub fn disable(&mut self) {
        self.assert_valid();
        body_disable_impl(self.id)
    }

    pub fn try_is_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_enabled_impl(self.id))
    }

    pub fn try_enable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_impl(self.id);
        Ok(())
    }

    pub fn try_disable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_disable_impl(self.id);
        Ok(())
    }

    pub fn is_bullet(&self) -> bool {
        self.assert_valid();
        body_is_bullet_impl(self.id)
    }
    pub fn set_bullet(&mut self, flag: bool) {
        self.assert_valid();
        body_set_bullet_impl(self.id, flag)
    }

    pub fn try_is_bullet(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_bullet_impl(self.id))
    }

    pub fn try_set_bullet(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_set_bullet_impl(self.id, flag);
        Ok(())
    }

    pub fn enable_contact_events(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_contact_events_impl(self.id, flag)
    }

    pub fn try_enable_contact_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_contact_events_impl(self.id, flag);
        Ok(())
    }

    pub fn enable_hit_events(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_hit_events_impl(self.id, flag)
    }

    pub fn try_enable_hit_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_hit_events_impl(self.id, flag);
        Ok(())
    }

    // Names and user data (raw pointer)
    pub fn set_name(&mut self, name: &str) {
        self.assert_valid();
        let cs = CString::new(name).expect("body name contains an interior NUL byte");
        body_set_name_impl(self.id, &cs)
    }

    pub fn try_set_name(&mut self, name: &str) -> ApiResult<()> {
        self.check_valid()?;
        let cs = CString::new(name).map_err(|_| ApiError::NulByteInString)?;
        body_set_name_impl(self.id, &cs);
        Ok(())
    }

    pub fn name(&self) -> Option<String> {
        self.assert_valid();
        body_name_impl(self.id)
    }

    pub fn try_name(&self) -> ApiResult<Option<String>> {
        self.check_valid()?;
        Ok(body_name_impl(self.id))
    }
    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// The caller must ensure that `p` is either null or points to a valid object
    /// for the entire time the body may access it, and that any lifetimes/aliasing rules
    /// are upheld. Box2D treats this as an opaque pointer and may store/use it across steps.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { body_set_user_data_ptr_raw_checked_impl(self.core.as_ref(), self.id, p) }
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { try_body_set_user_data_ptr_raw_impl(self.core.as_ref(), self.id, p) }
    }
    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        body_user_data_ptr_raw_checked_impl(self.id)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_body_user_data_ptr_raw_impl(self.id)
    }

    /// Set typed user data on this body.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the body is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        body_set_user_data_checked_impl(self.core.as_ref(), self.id, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_body_set_user_data_checked_impl(self.core.as_ref(), self.id, value)
    }

    /// Clear typed user data on this body. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        body_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_body_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        body_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        try_body_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        body_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_body_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        body_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_body_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> BodyId {
        self.id
    }
}
