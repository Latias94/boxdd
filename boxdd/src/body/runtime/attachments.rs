use super::*;
use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use crate::types::ContactData;
use boxdd_sys::ffi;

fn body_contact_capacity(id: BodyId) -> i32 {
    unsafe { ffi::b2Body_GetContactCapacity(raw_body_id(id)) }
}

pub(crate) fn body_contact_data_into_in_impl(
    core: &WorldCore,
    id: BodyId,
    out: &mut Vec<ContactData>,
) -> ApiResult<()> {
    let brand = core.brand();
    let contact_epoch = core.contact_epoch();
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::try_fill_mapped_from_ffi(
            out,
            cap,
            |ptr, cap| ffi::b2Body_GetContactData(id, ptr, cap),
            |raw| ContactData::try_from_raw_in(brand, contact_epoch, raw),
        )
    }
}

pub(crate) fn body_contact_data_in_impl(
    core: &WorldCore,
    id: BodyId,
) -> ApiResult<Vec<ContactData>> {
    let brand = core.brand();
    let contact_epoch = core.contact_epoch();
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(
            cap,
            |ptr, cap| ffi::b2Body_GetContactData(id, ptr, cap),
            |raw| ContactData::try_from_raw_in(brand, contact_epoch, raw),
        )
    }
}

pub(crate) fn body_contact_data_raw_into_impl(
    id: BodyId,
    out: &mut Vec<ffi::b2ContactData>,
) -> ApiResult<()> {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        })
    }
}

pub(crate) fn body_contact_data_raw_impl(id: BodyId) -> ApiResult<Vec<ffi::b2ContactData>> {
    let cap = body_contact_capacity(id);
    let id = raw_body_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        })
    }
}
