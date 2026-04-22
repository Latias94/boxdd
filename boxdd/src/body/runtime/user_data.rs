use super::*;
use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use boxdd_sys::ffi;
use std::os::raw::c_void;

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

pub(crate) unsafe fn body_set_user_data_ptr_raw_checked_impl(
    world_core: &WorldCore,
    id: BodyId,
    p: *mut c_void,
) {
    crate::core::debug_checks::assert_body_valid(id);
    unsafe { body_set_user_data_ptr_impl(world_core, id, p) }
}

pub(crate) unsafe fn try_body_set_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: BodyId,
    p: *mut c_void,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    unsafe { body_set_user_data_ptr_impl(world_core, id, p) }
    Ok(())
}

pub(crate) fn body_user_data_ptr_raw_checked_impl(id: BodyId) -> *mut c_void {
    crate::core::debug_checks::assert_body_valid(id);
    body_user_data_ptr_impl(id)
}

pub(crate) fn try_body_user_data_ptr_raw_impl(id: BodyId) -> ApiResult<*mut c_void> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_user_data_ptr_impl(id))
}

pub(crate) fn body_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
    value: T,
) {
    crate::core::debug_checks::assert_body_valid(id);
    body_set_user_data_impl(world_core, id, value);
}

pub(crate) fn try_body_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
    value: T,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_set_user_data_impl(world_core, id, value);
    Ok(())
}

pub(crate) fn body_clear_user_data_checked_impl(world_core: &WorldCore, id: BodyId) -> bool {
    crate::core::debug_checks::assert_body_valid(id);
    body_clear_user_data_impl(world_core, id)
}

pub(crate) fn try_body_clear_user_data_checked_impl(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<bool> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_clear_user_data_impl(world_core, id))
}

pub(crate) fn body_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_body_valid(id);
    body_with_user_data_impl(world_core, id, f).expect("user data type mismatch")
}

pub(crate) fn try_body_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_with_user_data_impl(world_core, id, f)
}

pub(crate) fn body_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_body_valid(id);
    body_with_user_data_mut_impl(world_core, id, f).expect("user data type mismatch")
}

pub(crate) fn try_body_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_with_user_data_mut_impl(world_core, id, f)
}

pub(crate) fn body_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
) -> Option<T> {
    crate::core::debug_checks::assert_body_valid(id);
    body_take_user_data_impl(world_core, id).expect("user data type mismatch")
}

pub(crate) fn try_body_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<Option<T>> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_take_user_data_impl(world_core, id)
}
