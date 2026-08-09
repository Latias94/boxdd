use super::*;
use crate::error::Result;
use crate::world::JointCall;
use std::os::raw::c_void;

pub(crate) fn joint_set_user_data_ptr_impl(
    joint: JointCall<'_>,
    user_data: *mut c_void,
) -> Result<()> {
    let retired = joint.clear_user_data()?;
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(joint.id()), user_data) };
    retired.resume_drop_panic();
    Ok(())
}

#[inline]
pub(crate) fn joint_user_data_ptr_impl(id: JointId) -> *mut c_void {
    unsafe { ffi::b2Joint_GetUserData(raw_joint_id(id)) }
}

pub(crate) fn joint_set_user_data_impl<T: 'static>(
    joint: JointCall<'_>,
    value: crate::core::callback_state::PendingUserValue<T>,
) -> Result<()> {
    let update = joint.set_user_data(value)?;
    let (pointer, retired) = update.into_parts();
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(joint.id()), pointer) };
    retired.resume_drop_panic();
    Ok(())
}

pub(crate) fn joint_clear_user_data_impl(joint: JointCall<'_>) -> Result<bool> {
    let retired = joint.clear_user_data()?;
    let had = retired.is_some() || !joint_user_data_ptr_impl(joint.id()).is_null();
    if had {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(joint.id()), core::ptr::null_mut()) };
    }
    retired.resume_drop_panic();
    Ok(had)
}

pub(crate) fn joint_with_user_data_impl<T: 'static, R, F>(
    joint: JointCall<'_>,
    f: crate::core::callback_state::PendingUserValue<F>,
) -> Result<Option<R>>
where
    F: FnOnce(&T) -> R,
{
    joint.with_user_data(f)
}

pub(crate) fn joint_with_user_data_mut_impl<T: 'static, R, F>(
    joint: JointCall<'_>,
    f: crate::core::callback_state::PendingUserValue<F>,
) -> Result<Option<R>>
where
    F: FnOnce(&mut T) -> R,
{
    joint.with_user_data_mut(f)
}

pub(crate) fn joint_take_user_data_impl<T: 'static>(joint: JointCall<'_>) -> Result<Option<T>> {
    let value = joint.take_user_data::<T>()?;
    if value.is_some() {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(joint.id()), core::ptr::null_mut()) };
    }
    Ok(value)
}
