use super::*;
use crate::error::Result;
use crate::world::BodyCall;
use boxdd_sys::ffi;
use std::os::raw::c_void;

pub(crate) fn body_set_user_data_ptr_impl(
    body: BodyCall<'_>,
    user_data: *mut c_void,
) -> Result<()> {
    let retired = body.clear_user_data()?;
    unsafe { ffi::b2Body_SetUserData(raw_body_id(body.id()), user_data) };
    retired.resume_drop_panic();
    Ok(())
}

#[inline]
pub(crate) fn body_user_data_ptr_impl(id: BodyId) -> *mut c_void {
    unsafe { ffi::b2Body_GetUserData(raw_body_id(id)) }
}

pub(crate) fn body_set_user_data_impl<T: 'static>(
    body: BodyCall<'_>,
    value: crate::core::callback_state::PendingUserValue<T>,
) -> Result<()> {
    let update = body.set_user_data(value)?;
    let (pointer, retired) = update.into_parts();
    unsafe { ffi::b2Body_SetUserData(raw_body_id(body.id()), pointer) };
    retired.resume_drop_panic();
    Ok(())
}

pub(crate) fn body_clear_user_data_impl(body: BodyCall<'_>) -> Result<bool> {
    let retired = body.clear_user_data()?;
    let had = retired.is_some() || !body_user_data_ptr_impl(body.id()).is_null();
    if had {
        unsafe { ffi::b2Body_SetUserData(raw_body_id(body.id()), core::ptr::null_mut()) };
    }
    retired.resume_drop_panic();
    Ok(had)
}

pub(crate) fn body_with_user_data_impl<T: 'static, R, F>(
    body: BodyCall<'_>,
    f: crate::core::callback_state::PendingUserValue<F>,
) -> Result<Option<R>>
where
    F: FnOnce(&T) -> R,
{
    body.with_user_data(f)
}

pub(crate) fn body_with_user_data_mut_impl<T: 'static, R, F>(
    body: BodyCall<'_>,
    f: crate::core::callback_state::PendingUserValue<F>,
) -> Result<Option<R>>
where
    F: FnOnce(&mut T) -> R,
{
    body.with_user_data_mut(f)
}

pub(crate) fn body_take_user_data_impl<T: 'static>(body: BodyCall<'_>) -> Result<Option<T>> {
    let value = body.take_user_data::<T>()?;
    if value.is_some() {
        unsafe { ffi::b2Body_SetUserData(raw_body_id(body.id()), core::ptr::null_mut()) };
    }
    Ok(value)
}
