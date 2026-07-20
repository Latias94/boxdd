use super::*;
use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use boxdd_sys::ffi;
use std::os::raw::c_void;

fn check_body(world_core: &WorldCore, id: BodyId) -> ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    world_core.check_body(id)
}

unsafe fn body_set_user_data_ptr_impl(
    world_core: &WorldCore,
    id: BodyId,
    user_data: *mut c_void,
) -> ApiResult<()> {
    let retired = world_core.clear_body_user_data(id)?;
    unsafe { ffi::b2Body_SetUserData(raw_body_id(id), user_data) };
    drop(retired);
    Ok(())
}

#[inline]
fn body_user_data_ptr_impl(id: BodyId) -> *mut c_void {
    unsafe { ffi::b2Body_GetUserData(raw_body_id(id)) }
}

fn body_set_user_data_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
    value: T,
) -> ApiResult<()> {
    let update = world_core.set_body_user_data(id, value)?;
    unsafe { ffi::b2Body_SetUserData(raw_body_id(id), update.pointer()) };
    drop(update);
    Ok(())
}

fn body_clear_user_data_impl(world_core: &WorldCore, id: BodyId) -> ApiResult<bool> {
    let retired = world_core.clear_body_user_data(id)?;
    let had = retired.is_some();
    if had {
        unsafe { ffi::b2Body_SetUserData(raw_body_id(id), core::ptr::null_mut()) };
    }
    drop(retired);
    Ok(had)
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
    check_body(world_core, id).expect("invalid or foreign BodyId");
    unsafe { body_set_user_data_ptr_impl(world_core, id, p) }
        .expect("body user data is already borrowed");
}

pub(crate) unsafe fn try_body_set_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: BodyId,
    p: *mut c_void,
) -> ApiResult<()> {
    check_body(world_core, id)?;
    unsafe { body_set_user_data_ptr_impl(world_core, id, p) }
}

pub(crate) fn body_user_data_ptr_raw_checked_impl(
    world_core: &WorldCore,
    id: BodyId,
) -> *mut c_void {
    check_body(world_core, id).expect("invalid or foreign BodyId");
    body_user_data_ptr_impl(id)
}

pub(crate) fn try_body_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<*mut c_void> {
    check_body(world_core, id)?;
    Ok(body_user_data_ptr_impl(id))
}

pub(crate) fn body_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
    value: T,
) {
    check_body(world_core, id).expect("invalid or foreign BodyId");
    body_set_user_data_impl(world_core, id, value).expect("body user data is already borrowed");
}

pub(crate) fn try_body_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
    value: T,
) -> ApiResult<()> {
    check_body(world_core, id)?;
    body_set_user_data_impl(world_core, id, value)
}

pub(crate) fn body_clear_user_data_checked_impl(world_core: &WorldCore, id: BodyId) -> bool {
    check_body(world_core, id).expect("invalid or foreign BodyId");
    body_clear_user_data_impl(world_core, id).expect("body user data is already borrowed")
}

pub(crate) fn try_body_clear_user_data_checked_impl(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<bool> {
    check_body(world_core, id)?;
    body_clear_user_data_impl(world_core, id)
}

pub(crate) fn body_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    check_body(world_core, id).expect("invalid or foreign BodyId");
    body_with_user_data_impl(world_core, id, f).expect("body user data access failed")
}

pub(crate) fn try_body_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    check_body(world_core, id)?;
    body_with_user_data_impl(world_core, id, f)
}

pub(crate) fn body_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    check_body(world_core, id).expect("invalid or foreign BodyId");
    body_with_user_data_mut_impl(world_core, id, f).expect("body user data access failed")
}

pub(crate) fn try_body_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: BodyId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    check_body(world_core, id)?;
    body_with_user_data_mut_impl(world_core, id, f)
}

pub(crate) fn body_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
) -> Option<T> {
    check_body(world_core, id).expect("invalid or foreign BodyId");
    body_take_user_data_impl(world_core, id).expect("body user data access failed")
}

pub(crate) fn try_body_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: BodyId,
) -> ApiResult<Option<T>> {
    check_body(world_core, id)?;
    body_take_user_data_impl(world_core, id)
}
