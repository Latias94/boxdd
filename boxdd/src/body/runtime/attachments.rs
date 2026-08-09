use super::*;
use crate::error::Result;
use crate::types::ContactData;
use crate::world::BodyCall;
use boxdd_sys::ffi;

fn body_contact_capacity(id: BodyId) -> i32 {
    unsafe { ffi::b2Body_GetContactCapacity(raw_body_id(id)) }
}

pub(crate) fn body_contact_data_in_impl(body: BodyCall<'_>) -> Result<Vec<ContactData>> {
    let contact_epoch = body.contact_epoch();
    let cap = check_native_body_count(
        "Body::contact_data",
        "contact_capacity",
        body_contact_capacity(body.id()),
    )?;
    let id = raw_body_id(body.id());
    let raw_contacts = unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        })
    }?;
    body.with_output_identity_resolver(|resolver| {
        let mut contacts = Vec::new();
        contacts
            .try_reserve_exact(raw_contacts.len())
            .map_err(|_| crate::error::Error::FfiOutputAllocationFailed)?;
        for raw in raw_contacts {
            contacts.push(ContactData::from_raw_in(resolver, contact_epoch, raw)?);
        }
        Ok(contacts)
    })
}
