use super::*;

fn check_shape(world_core: &crate::core::world_core::WorldCore, id: ShapeId) -> ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    world_core.check_shape(id)
}

pub(crate) unsafe fn shape_set_user_data_ptr_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    user_data: *mut c_void,
) -> ApiResult<()> {
    let retired = world_core.clear_shape_user_data(id)?;
    unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), user_data) };
    drop(retired);
    Ok(())
}

#[inline]
pub(crate) fn shape_user_data_ptr_impl(id: ShapeId) -> *mut c_void {
    unsafe { ffi::b2Shape_GetUserData(raw_shape_id(id)) }
}

pub(crate) fn shape_set_user_data_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    value: T,
) -> ApiResult<()> {
    let update = world_core.set_shape_user_data(id, value)?;
    unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), update.pointer()) };
    drop(update);
    Ok(())
}

pub(crate) fn shape_clear_user_data_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<bool> {
    let retired = world_core.clear_shape_user_data(id)?;
    let had = retired.is_some();
    if had {
        unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), core::ptr::null_mut()) };
    }
    drop(retired);
    Ok(had)
}

pub(crate) fn shape_with_user_data_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_shape_user_data(id, f)
}

pub(crate) fn shape_with_user_data_mut_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_shape_user_data_mut(id, f)
}

pub(crate) fn shape_take_user_data_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<Option<T>> {
    let value = world_core.take_shape_user_data::<T>(id)?;
    if value.is_some() {
        unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), core::ptr::null_mut()) };
    }
    Ok(value)
}

pub(crate) unsafe fn shape_set_user_data_ptr_raw_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    p: *mut c_void,
) {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    unsafe { shape_set_user_data_ptr_impl(world_core, id, p) }
        .expect("shape user data is already borrowed");
}

pub(crate) unsafe fn try_shape_set_user_data_ptr_raw_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    p: *mut c_void,
) -> ApiResult<()> {
    check_shape(world_core, id)?;
    unsafe { shape_set_user_data_ptr_impl(world_core, id, p) }
}

pub(crate) fn shape_user_data_ptr_raw_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> *mut c_void {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    shape_user_data_ptr_impl(id)
}

pub(crate) fn try_shape_user_data_ptr_raw_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<*mut c_void> {
    check_shape(world_core, id)?;
    Ok(shape_user_data_ptr_impl(id))
}

pub(crate) fn shape_set_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    value: T,
) {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    shape_set_user_data_impl(world_core, id, value).expect("shape user data is already borrowed");
}

pub(crate) fn try_shape_set_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    value: T,
) -> ApiResult<()> {
    check_shape(world_core, id)?;
    shape_set_user_data_impl(world_core, id, value)
}

pub(crate) fn shape_clear_user_data_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> bool {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    shape_clear_user_data_impl(world_core, id).expect("shape user data is already borrowed")
}

pub(crate) fn try_shape_clear_user_data_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<bool> {
    check_shape(world_core, id)?;
    shape_clear_user_data_impl(world_core, id)
}

pub(crate) fn shape_with_user_data_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    shape_with_user_data_impl(world_core, id, f).expect("shape user data access failed")
}

pub(crate) fn try_shape_with_user_data_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    check_shape(world_core, id)?;
    shape_with_user_data_impl(world_core, id, f)
}

pub(crate) fn shape_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    shape_with_user_data_mut_impl(world_core, id, f).expect("shape user data access failed")
}

pub(crate) fn try_shape_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    check_shape(world_core, id)?;
    shape_with_user_data_mut_impl(world_core, id, f)
}

pub(crate) fn shape_take_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> Option<T> {
    check_shape(world_core, id).expect("invalid or foreign ShapeId");
    shape_take_user_data_impl(world_core, id).expect("shape user data access failed")
}

pub(crate) fn try_shape_take_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<Option<T>> {
    check_shape(world_core, id)?;
    shape_take_user_data_impl(world_core, id)
}
