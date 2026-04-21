use std::any::TypeId;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;

use boxdd_sys::ffi;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct IdKey {
    pub(crate) index1: i32,
    pub(crate) world0: u16,
    pub(crate) generation: u16,
}

impl From<ffi::b2BodyId> for IdKey {
    #[inline]
    fn from(id: ffi::b2BodyId) -> Self {
        Self {
            index1: id.index1,
            world0: id.world0,
            generation: id.generation,
        }
    }
}

impl From<crate::types::BodyId> for IdKey {
    #[inline]
    fn from(id: crate::types::BodyId) -> Self {
        Self {
            index1: id.index1,
            world0: id.world0,
            generation: id.generation,
        }
    }
}

impl From<ffi::b2ShapeId> for IdKey {
    #[inline]
    fn from(id: ffi::b2ShapeId) -> Self {
        Self {
            index1: id.index1,
            world0: id.world0,
            generation: id.generation,
        }
    }
}

impl From<crate::types::ShapeId> for IdKey {
    #[inline]
    fn from(id: crate::types::ShapeId) -> Self {
        Self {
            index1: id.index1,
            world0: id.world0,
            generation: id.generation,
        }
    }
}

impl From<ffi::b2JointId> for IdKey {
    #[inline]
    fn from(id: ffi::b2JointId) -> Self {
        Self {
            index1: id.index1,
            world0: id.world0,
            generation: id.generation,
        }
    }
}

impl From<crate::types::JointId> for IdKey {
    #[inline]
    fn from(id: crate::types::JointId) -> Self {
        Self {
            index1: id.index1,
            world0: id.world0,
            generation: id.generation,
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

#[derive(Default)]
pub(crate) struct UserDataStore {
    pub(crate) world: Option<ErasedUserData>,
    pub(crate) bodies: HashMap<IdKey, ErasedUserData>,
    pub(crate) shapes: HashMap<IdKey, ErasedUserData>,
    pub(crate) joints: HashMap<IdKey, ErasedUserData>,
}
