use super::*;

fn retain_valid_shape_ids(ids: &mut Vec<ShapeId>) {
    ids.retain(|sid| unsafe { ffi::b2Shape_IsValid(raw_shape_id(*sid)) });
}

pub(crate) fn shape_sensor_capacity_checked_impl(id: ShapeId) -> i32 {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_capacity_impl(id)
}

pub(crate) fn try_shape_sensor_capacity_impl(id: ShapeId) -> ApiResult<i32> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_sensor_capacity_impl(id))
}

pub(crate) fn shape_sensor_overlaps_checked_impl(id: ShapeId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_impl(id)
}

pub(crate) fn shape_sensor_overlaps_into_checked_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_into_impl(id, out);
}

pub(crate) fn try_shape_sensor_overlaps_impl(id: ShapeId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_sensor_overlaps_impl(id))
}

pub(crate) fn try_shape_sensor_overlaps_into_impl(
    id: ShapeId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_sensor_overlaps_into_impl(id, out);
    Ok(())
}

pub(crate) fn shape_sensor_overlaps_valid_checked_impl(id: ShapeId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_valid_impl(id)
}

pub(crate) fn try_shape_sensor_overlaps_valid_impl(id: ShapeId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_sensor_overlaps_valid_impl(id))
}

pub(crate) fn shape_sensor_overlaps_valid_into_checked_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_valid_into_impl(id, out);
}

pub(crate) fn try_shape_sensor_overlaps_valid_into_impl(
    id: ShapeId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_sensor_overlaps_valid_into_impl(id, out);
    Ok(())
}

pub(crate) fn shape_sensor_overlaps_into_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    let id = raw_shape_id(id);
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr.cast(), cap)
        });
    }
}

pub(crate) fn shape_sensor_overlaps_impl(id: ShapeId) -> Vec<ShapeId> {
    let id = raw_shape_id(id);
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr: *mut ShapeId, cap| {
            ffi::b2Shape_GetSensorData(id, ptr.cast(), cap)
        })
    }
}

pub(crate) fn shape_sensor_overlaps_valid_into_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    shape_sensor_overlaps_into_impl(id, out);
    retain_valid_shape_ids(out);
}

pub(crate) fn shape_sensor_overlaps_valid_impl(id: ShapeId) -> Vec<ShapeId> {
    let mut ids = shape_sensor_overlaps_impl(id);
    retain_valid_shape_ids(&mut ids);
    ids
}

#[inline]
pub(crate) fn shape_sensor_capacity_impl(id: ShapeId) -> i32 {
    unsafe { ffi::b2Shape_GetSensorCapacity(raw_shape_id(id)) }
}
