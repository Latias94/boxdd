use super::*;
use crate::error::ApiResult;
use crate::types::ContactData;
use boxdd_sys::ffi;

fn body_contact_capacity(id: BodyId) -> usize {
    unsafe { ffi::b2Body_GetContactCapacity(raw_body_id(id)) }.max(0) as usize
}

fn body_contact_data_into_impl(id: BodyId, out: &mut Vec<ContactData>) {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        });
    }
}

fn body_contact_data_impl(id: BodyId) -> Vec<ContactData> {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi::<ContactData>(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        })
    }
}

fn body_contact_data_raw_into_impl(id: BodyId, out: &mut Vec<ffi::b2ContactData>) {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        });
    }
}

fn body_contact_data_raw_impl(id: BodyId) -> Vec<ffi::b2ContactData> {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        })
    }
}

pub(crate) fn body_contact_data_checked_impl(id: BodyId) -> Vec<ContactData> {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_impl(id)
}

pub(crate) fn body_contact_data_into_checked_impl(id: BodyId, out: &mut Vec<ContactData>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_into_impl(id, out);
}

pub(crate) fn try_body_contact_data_impl(id: BodyId) -> ApiResult<Vec<ContactData>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_contact_data_impl(id))
}

pub(crate) fn try_body_contact_data_into_impl(
    id: BodyId,
    out: &mut Vec<ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_contact_data_into_impl(id, out);
    Ok(())
}

pub(crate) fn body_contact_data_raw_checked_impl(id: BodyId) -> Vec<ffi::b2ContactData> {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_raw_impl(id)
}

pub(crate) fn body_contact_data_raw_into_checked_impl(
    id: BodyId,
    out: &mut Vec<ffi::b2ContactData>,
) {
    crate::core::debug_checks::assert_body_valid(id);
    body_contact_data_raw_into_impl(id, out);
}

pub(crate) fn try_body_contact_data_raw_impl(id: BodyId) -> ApiResult<Vec<ffi::b2ContactData>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_contact_data_raw_impl(id))
}

pub(crate) fn try_body_contact_data_raw_into_impl(
    id: BodyId,
    out: &mut Vec<ffi::b2ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_contact_data_raw_into_impl(id, out);
    Ok(())
}

pub(crate) fn body_shape_count_checked_impl(id: BodyId) -> i32 {
    crate::core::debug_checks::assert_body_valid(id);
    body_shape_count_impl(id)
}

pub(crate) fn try_body_shape_count_impl(id: BodyId) -> ApiResult<i32> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_shape_count_impl(id))
}

pub(crate) fn body_shapes_checked_impl(id: BodyId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_body_valid(id);
    body_shapes_impl(id)
}

pub(crate) fn body_shapes_into_checked_impl(id: BodyId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_shapes_into_impl(id, out);
}

pub(crate) fn try_body_shapes_impl(id: BodyId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_shapes_impl(id))
}

pub(crate) fn try_body_shapes_into_impl(id: BodyId, out: &mut Vec<ShapeId>) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_shapes_into_impl(id, out);
    Ok(())
}

pub(crate) fn body_joint_count_checked_impl(id: BodyId) -> i32 {
    crate::core::debug_checks::assert_body_valid(id);
    body_joint_count_impl(id)
}

pub(crate) fn try_body_joint_count_impl(id: BodyId) -> ApiResult<i32> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_joint_count_impl(id))
}

pub(crate) fn body_joints_checked_impl(id: BodyId) -> Vec<JointId> {
    crate::core::debug_checks::assert_body_valid(id);
    body_joints_impl(id)
}

pub(crate) fn body_joints_into_checked_impl(id: BodyId, out: &mut Vec<JointId>) {
    crate::core::debug_checks::assert_body_valid(id);
    body_joints_into_impl(id, out);
}

pub(crate) fn try_body_joints_impl(id: BodyId) -> ApiResult<Vec<JointId>> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_joints_impl(id))
}

pub(crate) fn try_body_joints_into_impl(id: BodyId, out: &mut Vec<JointId>) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(id)?;
    body_joints_into_impl(id, out);
    Ok(())
}
