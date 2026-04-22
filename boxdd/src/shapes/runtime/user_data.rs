use super::*;

pub(crate) unsafe fn shape_set_user_data_ptr_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    user_data: *mut c_void,
) {
    let _ = world_core.clear_shape_user_data(id);
    unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), user_data) }
}

#[inline]
pub(crate) fn shape_user_data_ptr_impl(id: ShapeId) -> *mut c_void {
    unsafe { ffi::b2Shape_GetUserData(raw_shape_id(id)) }
}

pub(crate) fn shape_set_user_data_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    value: T,
) {
    let user_data = world_core.set_shape_user_data(id, value);
    unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), user_data) };
}

pub(crate) fn shape_clear_user_data_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> bool {
    let had = world_core.clear_shape_user_data(id);
    if had {
        unsafe { ffi::b2Shape_SetUserData(raw_shape_id(id), core::ptr::null_mut()) };
    }
    had
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

pub(crate) fn shape_world_id_checked_impl(id: ShapeId) -> ffi::b2WorldId {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_world_id_impl(id)
}

pub(crate) fn try_shape_world_id_raw_impl(id: ShapeId) -> ApiResult<ffi::b2WorldId> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_world_id_impl(id))
}

pub(crate) fn shape_parent_chain_id_checked_impl(id: ShapeId) -> Option<ChainId> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_parent_chain_id_impl(id)
}

pub(crate) fn try_shape_parent_chain_id_impl(id: ShapeId) -> ApiResult<Option<ChainId>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_parent_chain_id_impl(id))
}

pub(crate) fn shape_is_valid_checked_impl(id: ShapeId) -> bool {
    crate::core::callback_state::assert_not_in_callback();
    shape_is_valid_impl(id)
}

pub(crate) fn try_shape_is_valid_impl(id: ShapeId) -> ApiResult<bool> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(shape_is_valid_impl(id))
}

pub(crate) unsafe fn shape_set_user_data_ptr_raw_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    p: *mut c_void,
) {
    crate::core::debug_checks::assert_shape_valid(id);
    unsafe { shape_set_user_data_ptr_impl(world_core, id, p) }
}

pub(crate) unsafe fn try_shape_set_user_data_ptr_raw_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    p: *mut c_void,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    unsafe { shape_set_user_data_ptr_impl(world_core, id, p) }
    Ok(())
}

pub(crate) fn shape_user_data_ptr_raw_checked_impl(id: ShapeId) -> *mut c_void {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_user_data_ptr_impl(id)
}

pub(crate) fn try_shape_user_data_ptr_raw_impl(id: ShapeId) -> ApiResult<*mut c_void> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_user_data_ptr_impl(id))
}

pub(crate) fn shape_set_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    value: T,
) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_set_user_data_impl(world_core, id, value);
}

pub(crate) fn try_shape_set_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    value: T,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_set_user_data_impl(world_core, id, value);
    Ok(())
}

pub(crate) fn shape_clear_user_data_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> bool {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_clear_user_data_impl(world_core, id)
}

pub(crate) fn try_shape_clear_user_data_checked_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<bool> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_clear_user_data_impl(world_core, id))
}

pub(crate) fn shape_with_user_data_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_with_user_data_impl(world_core, id, f).expect("user data type mismatch")
}

pub(crate) fn try_shape_with_user_data_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_with_user_data_impl(world_core, id, f)
}

pub(crate) fn shape_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_with_user_data_mut_impl(world_core, id, f).expect("user data type mismatch")
}

pub(crate) fn try_shape_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_with_user_data_mut_impl(world_core, id, f)
}

pub(crate) fn shape_take_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> Option<T> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_take_user_data_impl(world_core, id).expect("user data type mismatch")
}

pub(crate) fn try_shape_take_user_data_checked_impl<T: 'static>(
    world_core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<Option<T>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_take_user_data_impl(world_core, id)
}
