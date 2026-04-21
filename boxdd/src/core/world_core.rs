use boxdd_sys::ffi;
use std::any::Any;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use crate::types::{BodyId, ChainId, JointId, ShapeId};

pub(crate) type CustomFilterCb = dyn Fn(&crate::world::CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
    + Send
    + Sync
    + 'static;

pub(crate) type PreSolveCb = dyn Fn(
        &crate::world::CallbackWorld,
        crate::types::ShapeId,
        crate::types::ShapeId,
        crate::types::Vec2,
        crate::types::Vec2,
    ) -> bool
    + Send
    + Sync
    + 'static;

pub(crate) type MaterialMixCb = dyn Fn(crate::world::MaterialMixInput, crate::world::MaterialMixInput) -> f32
    + Send
    + Sync
    + 'static;

pub(crate) struct CustomFilterCtx {
    pub(crate) core: Weak<WorldCore>,
    pub(crate) cb: Box<CustomFilterCb>,
}

pub(crate) struct PreSolveCtx {
    pub(crate) core: Weak<WorldCore>,
    pub(crate) cb: Box<PreSolveCb>,
}

pub(crate) struct MaterialMixCtx {
    pub(crate) core: Weak<WorldCore>,
    pub(crate) cb: Box<MaterialMixCb>,
}

pub(crate) struct WorldCore {
    pub(crate) id: ffi::b2WorldId,
    pub(crate) custom_filter: Mutex<Option<Box<CustomFilterCtx>>>,
    pub(crate) pre_solve: Mutex<Option<Box<PreSolveCtx>>>,
    pub(crate) material_mix_slot: Mutex<Option<usize>>,
    pub(crate) friction_mix: Mutex<Option<Box<MaterialMixCtx>>>,
    pub(crate) restitution_mix: Mutex<Option<Box<MaterialMixCtx>>>,
    pub(crate) callback_panicked: AtomicBool,
    pub(crate) callback_panic: Mutex<Option<Box<dyn Any + Send + 'static>>>,
    pub(crate) deferred_destroys: Mutex<Vec<DeferredDestroy>>,
    pub(crate) user_data: Mutex<crate::core::user_data::UserDataStore>,
    pub(crate) borrowed_event_buffers: AtomicUsize,
    #[cfg(feature = "serialize")]
    pub(crate) registries: Mutex<crate::core::serialize_registry::Registries>,
    pub(crate) owned_bodies: AtomicUsize,
    pub(crate) owned_shapes: AtomicUsize,
    pub(crate) owned_joints: AtomicUsize,
    pub(crate) owned_chains: AtomicUsize,
}

// SAFETY: `WorldCore` contains only thread-safe primitives (atomics, mutexes) and is used as a
// ref-counted lifetime anchor. Box2D itself is not thread-safe; the public API prevents sending
// `World` / owned handles across threads. `CallbackWorld` exposes only operations that do not call
// into Box2D while the world is locked.
unsafe impl Send for WorldCore {}
unsafe impl Sync for WorldCore {}

#[derive(Clone, Debug)]
pub(crate) enum DeferredDestroy {
    Body(BodyId),
    Shape { id: ShapeId, update_body_mass: bool },
    Joint { id: JointId, wake_bodies: bool },
    Chain(ChainId),
}

impl WorldCore {
    pub(crate) fn new(id: ffi::b2WorldId) -> Arc<Self> {
        Arc::new(Self {
            id,
            custom_filter: Mutex::new(None),
            pre_solve: Mutex::new(None),
            material_mix_slot: Mutex::new(None),
            friction_mix: Mutex::new(None),
            restitution_mix: Mutex::new(None),
            callback_panicked: AtomicBool::new(false),
            callback_panic: Mutex::new(None),
            deferred_destroys: Mutex::new(Vec::new()),
            user_data: Mutex::new(crate::core::user_data::UserDataStore::default()),
            borrowed_event_buffers: AtomicUsize::new(0),
            #[cfg(feature = "serialize")]
            registries: Mutex::new(crate::core::serialize_registry::Registries::default()),
            owned_bodies: AtomicUsize::new(0),
            owned_shapes: AtomicUsize::new(0),
            owned_joints: AtomicUsize::new(0),
            owned_chains: AtomicUsize::new(0),
        })
    }

    pub(crate) fn owned_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.owned_bodies.load(Ordering::Relaxed),
            self.owned_shapes.load(Ordering::Relaxed),
            self.owned_joints.load(Ordering::Relaxed),
            self.owned_chains.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn defer_destroy(&self, d: DeferredDestroy) {
        self.deferred_destroys
            .lock()
            .expect("deferred_destroys mutex poisoned")
            .push(d);
    }

    pub(crate) fn events_buffers_are_borrowed(&self) -> bool {
        self.borrowed_event_buffers.load(Ordering::Relaxed) > 0
    }

    pub(crate) fn borrow_event_buffers(self: &Arc<Self>) -> BorrowedEventBuffersGuard {
        self.borrowed_event_buffers.fetch_add(1, Ordering::Relaxed);
        BorrowedEventBuffersGuard {
            core: Arc::clone(self),
        }
    }

    pub(crate) fn process_deferred_destroys(&self) {
        crate::core::callback_state::assert_not_in_callback();
        if self.events_buffers_are_borrowed() {
            return;
        }
        let mut pending = self
            .deferred_destroys
            .lock()
            .expect("deferred_destroys mutex poisoned");
        if pending.is_empty() {
            return;
        }
        let items = core::mem::take(&mut *pending);
        drop(pending);

        for item in items {
            match item {
                DeferredDestroy::Body(id) => {
                    if unsafe { ffi::b2Body_IsValid(id.into_raw()) } {
                        #[cfg(feature = "serialize")]
                        {
                            let mut r = self.registries.lock().expect("registries mutex poisoned");
                            r.remove_shape_flags_for_body(id);
                            r.remove_chains_for_body(id);
                            r.remove_body(id);
                        }
                        unsafe { ffi::b2DestroyBody(id.into_raw()) };
                    }
                    let old = self
                        .user_data
                        .lock()
                        .expect("user_data mutex poisoned")
                        .bodies
                        .remove(&crate::core::user_data::IdKey::from(id));
                    drop(old);
                }
                DeferredDestroy::Shape {
                    id,
                    update_body_mass,
                } => {
                    if unsafe { ffi::b2Shape_IsValid(id.into_raw()) } {
                        unsafe { ffi::b2DestroyShape(id.into_raw(), update_body_mass) };
                        #[cfg(feature = "serialize")]
                        {
                            self.registries
                                .lock()
                                .expect("registries mutex poisoned")
                                .remove_shape_flags(id);
                        }
                    }
                    let old = self
                        .user_data
                        .lock()
                        .expect("user_data mutex poisoned")
                        .shapes
                        .remove(&crate::core::user_data::IdKey::from(id));
                    drop(old);
                }
                DeferredDestroy::Joint { id, wake_bodies } => {
                    if unsafe { ffi::b2Joint_IsValid(id.into_raw()) } {
                        unsafe { ffi::b2DestroyJoint(id.into_raw(), wake_bodies) };
                    }
                    let old = self
                        .user_data
                        .lock()
                        .expect("user_data mutex poisoned")
                        .joints
                        .remove(&crate::core::user_data::IdKey::from(id));
                    drop(old);
                }
                DeferredDestroy::Chain(id) => {
                    if unsafe { ffi::b2Chain_IsValid(id.into_raw()) } {
                        unsafe { ffi::b2DestroyChain(id.into_raw()) };
                        #[cfg(feature = "serialize")]
                        {
                            self.registries
                                .lock()
                                .expect("registries mutex poisoned")
                                .remove_chain(id);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn clear_world_user_data(&self) -> bool {
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .world
            .take();
        let had = old.is_some();
        drop(old);
        had
    }

    pub(crate) fn clear_body_user_data(&self, id: BodyId) -> bool {
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .bodies
            .remove(&crate::core::user_data::IdKey::from(id));
        let had = old.is_some();
        drop(old);
        had
    }

    pub(crate) fn clear_shape_user_data(&self, id: ShapeId) -> bool {
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .shapes
            .remove(&crate::core::user_data::IdKey::from(id));
        let had = old.is_some();
        drop(old);
        had
    }

    pub(crate) fn clear_joint_user_data(&self, id: JointId) -> bool {
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .joints
            .remove(&crate::core::user_data::IdKey::from(id));
        let had = old.is_some();
        drop(old);
        had
    }

    pub(crate) fn set_world_user_data<T: 'static>(&self, value: T) -> *mut core::ffi::c_void {
        let new = crate::core::user_data::ErasedUserData::new(value);
        let ptr = new.as_ptr();
        let old = {
            let mut s = self.user_data.lock().expect("user_data mutex poisoned");
            s.world.replace(new)
        };
        drop(old);
        ptr
    }

    pub(crate) fn set_body_user_data<T: 'static>(
        &self,
        id: BodyId,
        value: T,
    ) -> *mut core::ffi::c_void {
        let key = crate::core::user_data::IdKey::from(id);
        let new = crate::core::user_data::ErasedUserData::new(value);
        let ptr = new.as_ptr();
        let old = {
            let mut s = self.user_data.lock().expect("user_data mutex poisoned");
            s.bodies.insert(key, new)
        };
        drop(old);
        ptr
    }

    pub(crate) fn set_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
        value: T,
    ) -> *mut core::ffi::c_void {
        let key = crate::core::user_data::IdKey::from(id);
        let new = crate::core::user_data::ErasedUserData::new(value);
        let ptr = new.as_ptr();
        let old = {
            let mut s = self.user_data.lock().expect("user_data mutex poisoned");
            s.shapes.insert(key, new)
        };
        drop(old);
        ptr
    }

    pub(crate) fn set_joint_user_data<T: 'static>(
        &self,
        id: JointId,
        value: T,
    ) -> *mut core::ffi::c_void {
        let key = crate::core::user_data::IdKey::from(id);
        let new = crate::core::user_data::ErasedUserData::new(value);
        let ptr = new.as_ptr();
        let old = {
            let mut s = self.user_data.lock().expect("user_data mutex poisoned");
            s.joints.insert(key, new)
        };
        drop(old);
        ptr
    }

    pub(crate) fn try_with_world_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.world.as_ref() else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_ref(f).expect("type checked")))
    }

    pub(crate) fn try_with_body_user_data<T: 'static, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.bodies.get(&key) else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_ref(f).expect("type checked")))
    }

    pub(crate) fn try_with_shape_user_data<T: 'static, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.shapes.get(&key) else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_ref(f).expect("type checked")))
    }

    pub(crate) fn try_with_joint_user_data<T: 'static, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.joints.get(&key) else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_ref(f).expect("type checked")))
    }

    pub(crate) fn try_with_body_user_data_mut<T: 'static, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&mut T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.bodies.get_mut(&key) else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_mut(f).expect("type checked")))
    }

    pub(crate) fn try_with_shape_user_data_mut<T: 'static, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&mut T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.shapes.get_mut(&key) else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_mut(f).expect("type checked")))
    }

    pub(crate) fn try_with_joint_user_data_mut<T: 'static, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&mut T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut s = self.user_data.lock().expect("user_data mutex poisoned");
        let Some(e) = s.joints.get_mut(&key) else {
            return Ok(None);
        };
        if !e.matches::<T>() {
            return Err(crate::error::ApiError::UserDataTypeMismatch);
        }
        Ok(Some(e.with_mut(f).expect("type checked")))
    }

    pub(crate) fn take_world_user_data<T: 'static>(&self) -> crate::error::ApiResult<Option<T>> {
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .world
            .take();
        let Some(old) = old else { return Ok(None) };
        match old.try_into_value::<T>() {
            Ok(v) => Ok(Some(v)),
            Err(old) => {
                self.user_data
                    .lock()
                    .expect("user_data mutex poisoned")
                    .world = Some(old);
                Err(crate::error::ApiError::UserDataTypeMismatch)
            }
        }
    }

    pub(crate) fn take_body_user_data<T: 'static>(
        &self,
        id: BodyId,
    ) -> crate::error::ApiResult<Option<T>> {
        let key = crate::core::user_data::IdKey::from(id);
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .bodies
            .remove(&key);
        let Some(old) = old else { return Ok(None) };
        match old.try_into_value::<T>() {
            Ok(v) => Ok(Some(v)),
            Err(old) => {
                self.user_data
                    .lock()
                    .expect("user_data mutex poisoned")
                    .bodies
                    .insert(key, old);
                Err(crate::error::ApiError::UserDataTypeMismatch)
            }
        }
    }

    pub(crate) fn take_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
    ) -> crate::error::ApiResult<Option<T>> {
        let key = crate::core::user_data::IdKey::from(id);
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .shapes
            .remove(&key);
        let Some(old) = old else { return Ok(None) };
        match old.try_into_value::<T>() {
            Ok(v) => Ok(Some(v)),
            Err(old) => {
                self.user_data
                    .lock()
                    .expect("user_data mutex poisoned")
                    .shapes
                    .insert(key, old);
                Err(crate::error::ApiError::UserDataTypeMismatch)
            }
        }
    }

    pub(crate) fn take_joint_user_data<T: 'static>(
        &self,
        id: JointId,
    ) -> crate::error::ApiResult<Option<T>> {
        let key = crate::core::user_data::IdKey::from(id);
        let old = self
            .user_data
            .lock()
            .expect("user_data mutex poisoned")
            .joints
            .remove(&key);
        let Some(old) = old else { return Ok(None) };
        match old.try_into_value::<T>() {
            Ok(v) => Ok(Some(v)),
            Err(old) => {
                self.user_data
                    .lock()
                    .expect("user_data mutex poisoned")
                    .joints
                    .insert(key, old);
                Err(crate::error::ApiError::UserDataTypeMismatch)
            }
        }
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn record_body(&self, id: BodyId) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_body(id);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn record_chain(
        &self,
        id: crate::types::ChainId,
        meta: crate::core::serialize_registry::ChainCreateMeta,
    ) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_chain(id, meta);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn record_shape_flags(&self, sid: ShapeId, def: &ffi::b2ShapeDef) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_shape_flags(sid, def);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn remove_chain(&self, id: crate::types::ChainId) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .remove_chain(id);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn remove_shape_flags(&self, sid: ShapeId) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .remove_shape_flags(sid);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn cleanup_before_destroy_body(&self, id: BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        let mut r = self.registries.lock().expect("registries mutex poisoned");
        r.remove_shape_flags_for_body(id);
        r.remove_chains_for_body(id);
        r.remove_body(id);
    }
}

pub(crate) struct BorrowedEventBuffersGuard {
    core: Arc<WorldCore>,
}

impl Drop for BorrowedEventBuffersGuard {
    fn drop(&mut self) {
        let prev = self
            .core
            .borrowed_event_buffers
            .fetch_sub(1, Ordering::Relaxed);
        debug_assert!(prev > 0, "borrowed_event_buffers counter underflow");
    }
}

impl Drop for WorldCore {
    fn drop(&mut self) {
        self.clear_world_user_data();
        if let Some(slot) = self
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned")
            .take()
        {
            crate::core::material_mix_registry::set_friction_ptr(slot, core::ptr::null_mut());
            crate::core::material_mix_registry::set_restitution_ptr(slot, core::ptr::null_mut());
            crate::core::material_mix_registry::release_slot(slot);
        }
        let _guard = crate::core::box2d_lock::lock();
        // SAFETY: `WorldCore` owns the Box2D world id; only the last Arc drops it.
        unsafe { ffi::b2DestroyWorld(self.id) };
    }
}
