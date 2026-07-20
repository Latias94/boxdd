use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::error::{ApiError, ApiResult};
use crate::id::WorldToken;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct IdKey {
    pub(crate) index1: i32,
    pub(crate) world0: u16,
    pub(crate) generation: u16,
    pub(crate) world_generation: u16,
    pub(crate) token: WorldToken,
}

impl From<crate::types::BodyId> for IdKey {
    #[inline]
    fn from(id: crate::types::BodyId) -> Self {
        let raw = id.into_raw();
        let brand = id.brand();
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
            world_generation: brand.world_generation(),
            token: brand.token(),
        }
    }
}

impl From<crate::types::ShapeId> for IdKey {
    #[inline]
    fn from(id: crate::types::ShapeId) -> Self {
        let raw = id.into_raw();
        let brand = id.brand();
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
            world_generation: brand.world_generation(),
            token: brand.token(),
        }
    }
}

impl From<crate::types::JointId> for IdKey {
    #[inline]
    fn from(id: crate::types::JointId) -> Self {
        let raw = id.into_raw();
        let brand = id.brand();
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
            world_generation: brand.world_generation(),
            token: brand.token(),
        }
    }
}

pub(crate) struct ErasedUserData {
    type_id: TypeId,
    ptr: NonNull<u8>,
    drop_fn: unsafe fn(*mut c_void),
}

impl ErasedUserData {
    #[inline]
    pub(crate) fn new<T: 'static>(value: T) -> Self {
        unsafe fn drop_boxed<T: 'static>(p: *mut c_void) {
            drop(unsafe { Box::from_raw(p as *mut T) });
        }

        let boxed = Box::new(value);
        let ptr =
            NonNull::new(Box::into_raw(boxed) as *mut u8).expect("Box::into_raw returned null");
        Self {
            type_id: TypeId::of::<T>(),
            ptr,
            drop_fn: drop_boxed::<T>,
        }
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr() as *mut c_void
    }

    #[inline]
    pub(crate) fn with_ref<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        if self.type_id != TypeId::of::<T>() {
            return None;
        }
        // SAFETY: `type_id` guarantees the allocation is a `T`.
        let r = unsafe { &*(self.ptr.as_ptr() as *const T) };
        Some(f(r))
    }

    #[inline]
    pub(crate) fn matches<T: 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    #[inline]
    pub(crate) fn with_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        if self.type_id != TypeId::of::<T>() {
            return None;
        }
        // SAFETY: `type_id` guarantees the allocation is a `T`.
        let r = unsafe { &mut *(self.ptr.as_ptr() as *mut T) };
        Some(f(r))
    }

    pub(crate) fn try_into_value<T: 'static>(self) -> Result<T, Self> {
        if self.type_id != TypeId::of::<T>() {
            return Err(self);
        }
        let p = self.ptr;
        core::mem::forget(self);
        // SAFETY: `type_id` guarantees the allocation is a `T`.
        Ok(*unsafe { Box::from_raw(p.as_ptr() as *mut T) })
    }
}

impl Drop for ErasedUserData {
    fn drop(&mut self) {
        unsafe { (self.drop_fn)(self.ptr.as_ptr() as *mut c_void) }
    }
}

pub(crate) struct UserDataUpdate {
    pointer: *mut c_void,
    _retired: Option<ErasedUserData>,
}

impl UserDataUpdate {
    #[inline]
    pub(crate) fn inserted(pointer: *mut c_void) -> Self {
        Self {
            pointer,
            _retired: None,
        }
    }

    #[inline]
    pub(crate) fn pointer(&self) -> *mut c_void {
        self.pointer
    }
}

pub(crate) struct UserDataEntry {
    value: RefCell<Option<ErasedUserData>>,
}

pub(crate) type UserDataEntryRef = Rc<UserDataEntry>;

impl UserDataEntry {
    pub(crate) fn new<T: 'static>(value: T) -> (UserDataEntryRef, *mut c_void) {
        let value = ErasedUserData::new(value);
        let pointer = value.as_ptr();
        (
            Rc::new(Self {
                value: RefCell::new(Some(value)),
            }),
            pointer,
        )
    }

    pub(crate) fn replace<T: 'static>(&self, value: T) -> ApiResult<UserDataUpdate> {
        let value = ErasedUserData::new(value);
        let pointer = value.as_ptr();
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let retired = slot.replace(value);
        drop(slot);
        Ok(UserDataUpdate {
            pointer,
            _retired: retired,
        })
    }

    pub(crate) fn check_mutable(&self) -> ApiResult<()> {
        let borrow = self
            .value
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        drop(borrow);
        Ok(())
    }

    pub(crate) fn take_erased(&self) -> ApiResult<Option<ErasedUserData>> {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        Ok(slot.take())
    }

    pub(crate) fn try_with<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> ApiResult<Option<R>> {
        let slot = self
            .value
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(value) = slot.as_ref() else {
            return Ok(None);
        };
        if !value.matches::<T>() {
            return Err(ApiError::UserDataTypeMismatch);
        }
        Ok(Some(value.with_ref(f).expect("type checked")))
    }

    pub(crate) fn try_with_mut<T: 'static, R>(
        &self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(value) = slot.as_mut() else {
            return Ok(None);
        };
        if !value.matches::<T>() {
            return Err(ApiError::UserDataTypeMismatch);
        }
        Ok(Some(value.with_mut(f).expect("type checked")))
    }

    pub(crate) fn take<T: 'static>(&self) -> ApiResult<Option<T>> {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(value) = slot.as_ref() else {
            return Ok(None);
        };
        if !value.matches::<T>() {
            return Err(ApiError::UserDataTypeMismatch);
        }
        let value = slot.take().expect("value checked");
        drop(slot);
        match value.try_into_value::<T>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => unreachable!("type checked"),
        }
    }
}

#[derive(Default)]
pub(crate) struct UserDataStore {
    pub(crate) world: Option<UserDataEntryRef>,
    pub(crate) bodies: HashMap<IdKey, UserDataEntryRef>,
    pub(crate) shapes: HashMap<IdKey, UserDataEntryRef>,
    pub(crate) joints: HashMap<IdKey, UserDataEntryRef>,
}
