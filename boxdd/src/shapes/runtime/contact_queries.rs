use super::*;
use crate::world::ShapeCall;

fn shape_contact_capacity(id: ShapeId) -> Result<i32> {
    check_native_shape_count("Shape::contact_data", "contact_capacity", unsafe {
        ffi::b2Shape_GetContactCapacity(raw_shape_id(id))
    })
}

pub(crate) fn shape_contact_data_in_impl(shape: ShapeCall<'_>) -> Result<Vec<ContactData>> {
    let contact_epoch = shape.contact_epoch();
    let cap = shape_contact_capacity(shape.id())?;
    let id = raw_shape_id(shape.id());
    let raw_contacts = unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        })
    }?;
    shape.with_output_identity_resolver(|resolver| {
        let mut contacts = Vec::new();
        contacts
            .try_reserve_exact(raw_contacts.len())
            .map_err(|_| Error::FfiOutputAllocationFailed)?;
        for raw in raw_contacts {
            contacts.push(ContactData::from_raw_in(resolver, contact_epoch, raw)?);
        }
        Ok(contacts)
    })
}
