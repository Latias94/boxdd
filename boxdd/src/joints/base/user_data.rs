use super::*;
use crate::error::ApiResult;
use std::os::raw::c_void;

fn check_joint(world_core: &WorldCore, id: JointId) -> ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    world_core.check_joint(id)
}

unsafe fn joint_set_user_data_ptr_impl(
    world_core: &WorldCore,
    id: JointId,
    user_data: *mut c_void,
) -> ApiResult<()> {
    let retired = world_core.clear_joint_user_data(id)?;
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), user_data) };
    drop(retired);
    Ok(())
}

#[inline]
fn joint_user_data_ptr_impl(id: JointId) -> *mut c_void {
    unsafe { ffi::b2Joint_GetUserData(raw_joint_id(id)) }
}

fn joint_set_user_data_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
    value: T,
) -> ApiResult<()> {
    let update = world_core.set_joint_user_data(id, value)?;
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), update.pointer()) };
    drop(update);
    Ok(())
}

fn joint_clear_user_data_impl(world_core: &WorldCore, id: JointId) -> ApiResult<bool> {
    let retired = world_core.clear_joint_user_data(id)?;
    let had = retired.is_some();
    if had {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), core::ptr::null_mut()) };
    }
    drop(retired);
    Ok(had)
}

fn joint_with_user_data_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_joint_user_data(id, f)
}

fn joint_with_user_data_mut_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_joint_user_data_mut(id, f)
}

fn joint_take_user_data_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<Option<T>> {
    let value = world_core.take_joint_user_data::<T>(id)?;
    if value.is_some() {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), core::ptr::null_mut()) };
    }
    Ok(value)
}

pub(crate) unsafe fn joint_set_user_data_ptr_raw_checked_impl(
    world_core: &WorldCore,
    id: JointId,
    p: *mut c_void,
) {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    unsafe { joint_set_user_data_ptr_impl(world_core, id, p) }
        .expect("joint user data is already borrowed");
}

pub(crate) unsafe fn try_joint_set_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: JointId,
    p: *mut c_void,
) -> ApiResult<()> {
    check_joint(world_core, id)?;
    unsafe { joint_set_user_data_ptr_impl(world_core, id, p) }
}

pub(crate) fn joint_user_data_ptr_raw_checked_impl(
    world_core: &WorldCore,
    id: JointId,
) -> *mut c_void {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    joint_user_data_ptr_impl(id)
}

pub(crate) fn try_joint_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<*mut c_void> {
    check_joint(world_core, id)?;
    Ok(joint_user_data_ptr_impl(id))
}

pub(crate) fn joint_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
    value: T,
) {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    joint_set_user_data_impl(world_core, id, value).expect("joint user data is already borrowed");
}

pub(crate) fn try_joint_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
    value: T,
) -> ApiResult<()> {
    check_joint(world_core, id)?;
    joint_set_user_data_impl(world_core, id, value)
}

pub(crate) fn joint_clear_user_data_checked_impl(world_core: &WorldCore, id: JointId) -> bool {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    joint_clear_user_data_impl(world_core, id).expect("joint user data is already borrowed")
}

pub(crate) fn try_joint_clear_user_data_checked_impl(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<bool> {
    check_joint(world_core, id)?;
    joint_clear_user_data_impl(world_core, id)
}

pub(crate) fn joint_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    joint_with_user_data_impl(world_core, id, f).expect("joint user data access failed")
}

pub(crate) fn try_joint_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    check_joint(world_core, id)?;
    joint_with_user_data_impl(world_core, id, f)
}

pub(crate) fn joint_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    joint_with_user_data_mut_impl(world_core, id, f).expect("joint user data access failed")
}

pub(crate) fn try_joint_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    check_joint(world_core, id)?;
    joint_with_user_data_mut_impl(world_core, id, f)
}

pub(crate) fn joint_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> Option<T> {
    check_joint(world_core, id).expect("invalid or foreign JointId");
    joint_take_user_data_impl(world_core, id).expect("joint user data access failed")
}

pub(crate) fn try_joint_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<Option<T>> {
    check_joint(world_core, id)?;
    joint_take_user_data_impl(world_core, id)
}
