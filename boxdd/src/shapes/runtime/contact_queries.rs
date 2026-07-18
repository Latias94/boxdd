use super::*;

fn shape_contact_capacity(id: ShapeId) -> i32 {
    unsafe { ffi::b2Shape_GetContactCapacity(raw_shape_id(id)) }
}

pub(crate) fn shape_contact_data_into_impl(
    id: ShapeId,
    out: &mut Vec<ContactData>,
) -> ApiResult<()> {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::fill_mapped_from_ffi(
            out,
            cap,
            |ptr, cap| ffi::b2Shape_GetContactData(id, ptr, cap),
            ContactData::from_raw,
        )
    }
}

pub(crate) fn shape_contact_data_impl(id: ShapeId) -> ApiResult<Vec<ContactData>> {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::read_mapped_from_ffi(
            cap,
            |ptr, cap| ffi::b2Shape_GetContactData(id, ptr, cap),
            ContactData::from_raw,
        )
    }
}

pub(crate) fn shape_contact_data_raw_into_impl(
    id: ShapeId,
    out: &mut Vec<ffi::b2ContactData>,
) -> ApiResult<()> {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        })
    }
}

pub(crate) fn shape_contact_data_raw_impl(id: ShapeId) -> ApiResult<Vec<ffi::b2ContactData>> {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        })
    }
}

pub(crate) fn shape_contact_data_checked_impl(id: ShapeId) -> Vec<ContactData> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_impl(id).expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
}

pub(crate) fn shape_contact_data_into_checked_impl(id: ShapeId, out: &mut Vec<ContactData>) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_into_impl(id, out).expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
}

pub(crate) fn try_shape_contact_data_impl(id: ShapeId) -> ApiResult<Vec<ContactData>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_contact_data_impl(id)
}

pub(crate) fn try_shape_contact_data_into_impl(
    id: ShapeId,
    out: &mut Vec<ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_contact_data_into_impl(id, out)
}

pub(crate) fn shape_contact_data_raw_checked_impl(id: ShapeId) -> Vec<ffi::b2ContactData> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_raw_impl(id).expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
}

pub(crate) fn shape_contact_data_raw_into_checked_impl(
    id: ShapeId,
    out: &mut Vec<ffi::b2ContactData>,
) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_raw_into_impl(id, out).expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
}

pub(crate) fn try_shape_contact_data_raw_impl(id: ShapeId) -> ApiResult<Vec<ffi::b2ContactData>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_contact_data_raw_impl(id)
}

pub(crate) fn try_shape_contact_data_raw_into_impl(
    id: ShapeId,
    out: &mut Vec<ffi::b2ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_contact_data_raw_into_impl(id, out)
}
