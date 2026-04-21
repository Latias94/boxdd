use crate::error::ApiResult;
use crate::types::{ContactData, ContactId};
use boxdd_sys::ffi;

#[inline]
fn contact_is_valid_impl(id: ContactId) -> bool {
    unsafe { ffi::b2Contact_IsValid(id.into_raw()) }
}

#[inline]
fn contact_data_raw_impl(id: ContactId) -> ffi::b2ContactData {
    unsafe { ffi::b2Contact_GetData(id.into_raw()) }
}

impl ContactId {
    /// Returns whether this contact id currently references a live Box2D contact.
    #[inline]
    pub fn is_valid(self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        contact_is_valid_impl(self)
    }

    /// Recoverable version of [`Self::is_valid`].
    #[inline]
    pub fn try_is_valid(self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(contact_is_valid_impl(self))
    }

    /// Fetch the crate-owned contact snapshot for this contact.
    #[inline]
    pub fn data(self) -> ContactData {
        crate::core::debug_checks::assert_contact_valid(self);
        ContactData::from_raw(contact_data_raw_impl(self))
    }

    /// Recoverable version of [`Self::data`].
    #[inline]
    pub fn try_data(self) -> ApiResult<ContactData> {
        crate::core::debug_checks::check_contact_valid(self)?;
        Ok(ContactData::from_raw(contact_data_raw_impl(self)))
    }

    /// Fetch the raw Box2D contact snapshot for this contact.
    #[inline]
    pub fn data_raw(self) -> ffi::b2ContactData {
        crate::core::debug_checks::assert_contact_valid(self);
        contact_data_raw_impl(self)
    }

    /// Recoverable version of [`Self::data_raw`].
    #[inline]
    pub fn try_data_raw(self) -> ApiResult<ffi::b2ContactData> {
        crate::core::debug_checks::check_contact_valid(self)?;
        Ok(contact_data_raw_impl(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::ApiError;

    fn invalid_contact_id() -> crate::ContactId {
        crate::ContactId {
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
