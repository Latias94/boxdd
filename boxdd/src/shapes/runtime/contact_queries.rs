use super::*;
use crate::core::world_core::WorldCore;

fn shape_contact_capacity(id: ShapeId) -> i32 {
    unsafe { ffi::b2Shape_GetContactCapacity(raw_shape_id(id)) }
}

pub(crate) fn shape_contact_data_into_in_impl(
    core: &WorldCore,
    id: ShapeId,
    out: &mut Vec<ContactData>,
) -> ApiResult<()> {
    let brand = core.brand();
    let contact_epoch = core.contact_epoch();
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::try_fill_mapped_from_ffi(
            out,
            cap,
            |ptr, cap| ffi::b2Shape_GetContactData(id, ptr, cap),
            |raw| ContactData::try_from_raw_in(brand, contact_epoch, raw),
        )
    }
}

pub(crate) fn shape_contact_data_in_impl(
    core: &WorldCore,
    id: ShapeId,
) -> ApiResult<Vec<ContactData>> {
    let brand = core.brand();
    let contact_epoch = core.contact_epoch();
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(
            cap,
            |ptr, cap| ffi::b2Shape_GetContactData(id, ptr, cap),
            |raw| ContactData::try_from_raw_in(brand, contact_epoch, raw),
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
