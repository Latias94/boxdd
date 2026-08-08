use super::*;
use crate::world::ShapeCall;

pub(crate) fn shape_set_user_data_ptr_impl(
    shape: ShapeCall<'_>,
    user_data: *mut c_void,
) -> Result<()> {
    let retired = shape.clear_user_data()?;
    unsafe { ffi::b2Shape_SetUserData(raw_shape_id(shape.id()), user_data) };
    retired.resume_drop_panic();
    Ok(())
}

#[inline]
pub(crate) fn shape_user_data_ptr_impl(id: ShapeId) -> *mut c_void {
    unsafe { ffi::b2Shape_GetUserData(raw_shape_id(id)) }
}

pub(crate) fn shape_set_user_data_impl<T: 'static>(
    shape: ShapeCall<'_>,
    value: crate::core::callback_state::PendingUserValue<T>,
) -> Result<()> {
    let update = shape.set_user_data(value)?;
    let (pointer, retired) = update.into_parts();
    unsafe { ffi::b2Shape_SetUserData(raw_shape_id(shape.id()), pointer) };
    retired.resume_drop_panic();
    Ok(())
}

pub(crate) fn shape_clear_user_data_impl(shape: ShapeCall<'_>) -> Result<bool> {
    let retired = shape.clear_user_data()?;
    let had = retired.is_some() || !shape_user_data_ptr_impl(shape.id()).is_null();
    if had {
        unsafe { ffi::b2Shape_SetUserData(raw_shape_id(shape.id()), core::ptr::null_mut()) };
    }
    retired.resume_drop_panic();
    Ok(had)
}

pub(crate) fn shape_with_user_data_impl<T: 'static, R, F>(
    shape: ShapeCall<'_>,
    f: crate::core::callback_state::PendingUserValue<F>,
) -> Result<Option<R>>
where
    F: FnOnce(&T) -> R,
{
    shape.with_user_data(f)
}

pub(crate) fn shape_with_user_data_mut_impl<T: 'static, R, F>(
    shape: ShapeCall<'_>,
    f: crate::core::callback_state::PendingUserValue<F>,
) -> Result<Option<R>>
where
    F: FnOnce(&mut T) -> R,
{
    shape.with_user_data_mut(f)
}

pub(crate) fn shape_take_user_data_impl<T: 'static>(shape: ShapeCall<'_>) -> Result<Option<T>> {
    let value = shape.take_user_data::<T>()?;
    if value.is_some() {
        unsafe { ffi::b2Shape_SetUserData(raw_shape_id(shape.id()), core::ptr::null_mut()) };
    }
    Ok(value)
}
