use std::{ffi::CStr, os::raw::c_void};

use boxdd_sys::ffi;

use crate::error::Result;
use crate::types::BodyId;

use super::super::{scoped::Body, validation::check_valid_body_name};
use super::{
    raw_body_id,
    user_data::{
        body_clear_user_data_impl, body_set_user_data_impl, body_set_user_data_ptr_impl,
        body_take_user_data_impl, body_user_data_ptr_impl, body_with_user_data_impl,
        body_with_user_data_mut_impl,
    },
};

#[inline]
pub(crate) fn body_set_name_impl(id: BodyId, name: &CStr) {
    unsafe { ffi::b2Body_SetName(raw_body_id(id), name.as_ptr()) }
}

#[inline]
pub(crate) fn body_name_impl(id: BodyId) -> Option<String> {
    let name_ptr = unsafe { ffi::b2Body_GetName(raw_body_id(id)) };
    if name_ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

impl Body<'_> {
    pub fn set_name(&mut self, name: &str) -> Result<()> {
        self.body_access().call(|_| {
            let cstr = check_valid_body_name("Body::set_name", name)?;
            body_set_name_impl(self.body_id(), &cstr);
            Ok(())
        })
    }

    pub fn name(&self) -> Result<Option<String>> {
        self.body_access()
            .call(|_| Ok(body_name_impl(self.body_id())))
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// Box2D and `boxdd` store but never dereference this pointer. If typed user data was
    /// previously set via [`Self::set_user_data`], it is cleared and dropped.
    pub fn set_user_data_ptr_raw(&mut self, user_data: *mut c_void) -> Result<()> {
        self.body_access()
            .call(|body| body_set_user_data_ptr_impl(body, user_data))
    }

    pub fn user_data_ptr_raw(&self) -> Result<*mut c_void> {
        let id = self.body_id();
        self.body_access().call(|_| Ok(body_user_data_ptr_impl(id)))
    }

    pub fn set_user_data<T: 'static>(&mut self, value: T) -> Result<()> {
        let value = crate::core::callback_state::PendingUserValue::new(value);
        self.body_access()
            .call(move |body| body_set_user_data_impl(body, value))
    }

    pub fn clear_user_data(&mut self) -> Result<bool> {
        self.body_access().call(body_clear_user_data_impl)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        self.body_access()
            .call(move |body| body_with_user_data_impl(body, f))
    }

    pub fn with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> Result<Option<R>> {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        self.body_access()
            .call(move |body| body_with_user_data_mut_impl(body, f))
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Result<Option<T>> {
        self.body_access().call(body_take_user_data_impl::<T>)
    }
}
