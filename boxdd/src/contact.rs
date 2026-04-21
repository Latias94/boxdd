use crate::error::ApiResult;
use crate::types::{ContactData, ContactId};
use boxdd_sys::ffi;

#[inline]
fn contact_is_valid_impl(id: ContactId) -> bool {
    unsafe { ffi::b2Contact_IsValid(id) }
}

#[inline]
fn contact_data_raw_impl(id: ContactId) -> ffi::b2ContactData {
    unsafe { ffi::b2Contact_GetData(id) }
}

/// Safe helpers for standalone [`ContactId`] values.
///
/// `ContactId` values commonly come from contact events or contact-data snapshots. This trait
/// mirrors the crate's panic-by-default plus `try_*` split for direct contact inspection without
/// forcing callers back to raw FFI.
pub trait ContactIdExt {
    /// Returns whether this contact id currently references a live Box2D contact.
    fn is_valid(self) -> bool;

    /// Recoverable version of [`Self::is_valid`].
    fn try_is_valid(self) -> ApiResult<bool>;

    /// Fetch the crate-owned contact snapshot for this contact.
    fn data(self) -> ContactData;

    /// Recoverable version of [`Self::data`].
    fn try_data(self) -> ApiResult<ContactData>;

    /// Fetch the raw Box2D contact snapshot for this contact.
    fn data_raw(self) -> ffi::b2ContactData;

    /// Recoverable version of [`Self::data_raw`].
    fn try_data_raw(self) -> ApiResult<ffi::b2ContactData>;
}

impl ContactIdExt for ContactId {
    #[inline]
    fn is_valid(self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        contact_is_valid_impl(self)
    }

    #[inline]
    fn try_is_valid(self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(contact_is_valid_impl(self))
    }

    #[inline]
    fn data(self) -> ContactData {
        crate::core::debug_checks::assert_contact_valid(self);
        ContactData::from_raw(contact_data_raw_impl(self))
    }

    #[inline]
    fn try_data(self) -> ApiResult<ContactData> {
        crate::core::debug_checks::check_contact_valid(self)?;
        Ok(ContactData::from_raw(contact_data_raw_impl(self)))
    }

    #[inline]
    fn data_raw(self) -> ffi::b2ContactData {
        crate::core::debug_checks::assert_contact_valid(self);
        contact_data_raw_impl(self)
    }

    #[inline]
    fn try_data_raw(self) -> ApiResult<ffi::b2ContactData> {
        crate::core::debug_checks::check_contact_valid(self)?;
        Ok(contact_data_raw_impl(self))
    }
}

#[cfg(test)]
mod tests {
    use super::ContactIdExt;
    use crate::ApiError;
    use boxdd_sys::ffi;

    fn invalid_contact_id() -> crate::ContactId {
        ffi::b2ContactId {
            index1: 0,
            world0: 0,
            padding: 0,
            generation: 0,
        }
    }

    #[test]
    fn try_contact_id_helpers_return_in_callback() {
        let contact = invalid_contact_id();
        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(contact.try_is_valid().unwrap_err(), ApiError::InCallback);
        assert_eq!(contact.try_data().unwrap_err(), ApiError::InCallback);
        assert_eq!(contact.try_data_raw().unwrap_err(), ApiError::InCallback);
    }
}
