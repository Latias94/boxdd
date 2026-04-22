use super::*;
use crate::error::ApiResult;
use std::os::raw::c_void;

unsafe fn joint_set_user_data_ptr_impl(
    world_core: &WorldCore,
    id: JointId,
    user_data: *mut c_void,
) {
    let _ = world_core.clear_joint_user_data(id);
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), user_data) }
}

#[inline]
fn joint_user_data_ptr_impl(id: JointId) -> *mut c_void {
    unsafe { ffi::b2Joint_GetUserData(raw_joint_id(id)) }
}

fn joint_set_user_data_impl<T: 'static>(world_core: &WorldCore, id: JointId, value: T) {
    let user_data = world_core.set_joint_user_data(id, value);
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), user_data) };
}

fn joint_clear_user_data_impl(world_core: &WorldCore, id: JointId) -> bool {
    let had = world_core.clear_joint_user_data(id);
    if had {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), core::ptr::null_mut()) };
    }
    had
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
    crate::core::debug_checks::assert_joint_valid(id);
    unsafe { joint_set_user_data_ptr_impl(world_core, id, p) }
}

pub(crate) unsafe fn try_joint_set_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: JointId,
    p: *mut c_void,
) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)?;
    unsafe { joint_set_user_data_ptr_impl(world_core, id, p) }
    Ok(())
}

pub(crate) fn joint_user_data_ptr_raw_checked_impl(id: JointId) -> *mut c_void {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_user_data_ptr_impl(id)
}

pub(crate) fn try_joint_user_data_ptr_raw_impl(id: JointId) -> ApiResult<*mut c_void> {
    crate::core::debug_checks::check_joint_valid(id)?;
    Ok(joint_user_data_ptr_impl(id))
}

pub(crate) fn joint_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
    value: T,
) {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_set_user_data_impl(world_core, id, value);
}

pub(crate) fn try_joint_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
    value: T,
) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_set_user_data_impl(world_core, id, value);
    Ok(())
}

pub(crate) fn joint_clear_user_data_checked_impl(world_core: &WorldCore, id: JointId) -> bool {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_clear_user_data_impl(world_core, id)
}

pub(crate) fn try_joint_clear_user_data_checked_impl(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<bool> {
    crate::core::debug_checks::check_joint_valid(id)?;
    Ok(joint_clear_user_data_impl(world_core, id))
}

pub(crate) fn joint_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_with_user_data_impl(world_core, id, f).expect("user data type mismatch")
}

pub(crate) fn try_joint_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_with_user_data_impl(world_core, id, f)
}

pub(crate) fn joint_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_with_user_data_mut_impl(world_core, id, f).expect("user data type mismatch")
}

pub(crate) fn try_joint_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_with_user_data_mut_impl(world_core, id, f)
}

pub(crate) fn joint_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> Option<T> {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_take_user_data_impl(world_core, id).expect("user data type mismatch")
}

pub(crate) fn try_joint_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<Option<T>> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_take_user_data_impl(world_core, id)
}
