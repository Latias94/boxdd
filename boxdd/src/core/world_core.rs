use boxdd_sys::ffi;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::error::{ApiError, ApiResult};
use crate::id::{
    ContactEpoch, IdBrand, RawBodyId, RawChainId, RawContactId, RawJointId, RawShapeId,
};
use crate::types::{BodyId, ChainId, JointId, ShapeId};

pub(crate) type CustomFilterCb =
    dyn Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static;

pub(crate) type PreSolveCb = dyn Fn(
        crate::types::ShapeId,
        crate::types::ShapeId,
        crate::types::Position,
        crate::types::Vec2,
    ) -> bool
    + Send
    + Sync
    + 'static;

pub(crate) type MaterialMixCb = dyn Fn(crate::world::MaterialMixInput, crate::world::MaterialMixInput) -> f32
    + Send
    + Sync
    + 'static;

pub(crate) struct WorkerCallbackState {
    brand: IdBrand,
    panicked: AtomicBool,
    panic: Mutex<Option<Box<dyn Any + Send + 'static>>>,
}

impl WorkerCallbackState {
    fn new(brand: IdBrand) -> Arc<Self> {
        Arc::new(Self {
            brand,
            panicked: AtomicBool::new(false),
            panic: Mutex::new(None),
        })
    }

    #[inline]
    pub(crate) fn shape(&self, raw: ffi::b2ShapeId) -> ShapeId {
        self.brand
            .try_shape(raw)
            .unwrap_or_else(|error| panic!("Box2D callback returned an invalid shape id: {error}"))
    }

    #[inline]
    pub(crate) fn has_panicked(&self) -> bool {
        self.panicked.load(Ordering::Acquire)
    }

    pub(crate) fn record_panic(&self, payload: Box<dyn Any + Send + 'static>) {
        if self.panicked.swap(true, Ordering::AcqRel) {
            // A competing callback panic has no unique owner boundary. Forgetting this exceptional
            // payload prevents its destructor from panicking across the C callback boundary.
            std::mem::forget(payload);
            return;
        }

        let mut first = self
            .panic
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if first.is_none() {
            *first = Some(payload);
        } else {
            // Preserve the original payload even if the owner violates the step/clear protocol.
            std::mem::forget(payload);
        }
    }

    pub(crate) fn clear_panic(&self) {
        *self
            .panic
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        self.panicked.store(false, Ordering::Release);
    }

    pub(crate) fn take_panic(&self) -> Option<Box<dyn Any + Send + 'static>> {
        self.panic
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

pub(crate) struct CustomFilterCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<CustomFilterCb>,
}

pub(crate) struct PreSolveCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<PreSolveCb>,
}

pub(crate) struct MaterialMixCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<MaterialMixCb>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum LifecycleState {
    Live,
    Poisoned,
    Destroyed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "recording and restore owners are introduced in U8"
    )
)]
pub(crate) enum ActivityState {
    Idle,
    Recording,
    Restoring,
}

#[derive(Default)]
struct GenerationLedger {
    last_by_slot: Vec<Option<u16>>,
}

impl GenerationLedger {
    fn register(&mut self, index1: i32, generation: u16) -> ApiResult<()> {
        let slot = index1
            .checked_sub(1)
            .and_then(|slot| usize::try_from(slot).ok())
            .ok_or(ApiError::InvalidArgument)?;

        if slot >= self.last_by_slot.len() {
            let additional = slot
                .checked_add(1)
                .and_then(|required| required.checked_sub(self.last_by_slot.len()))
                .ok_or(ApiError::IdentityTrackingAllocationFailed)?;
            self.last_by_slot
                .try_reserve_exact(additional)
                .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
            self.last_by_slot.resize(slot + 1, None);
        }

        match self.last_by_slot[slot] {
            Some(previous) if generation <= previous => Err(ApiError::ObjectIdentityExhausted),
            _ => {
                self.last_by_slot[slot] = Some(generation);
                Ok(())
            }
        }
    }
}

#[derive(Default)]
struct ObjectGenerationLedgers {
    bodies: GenerationLedger,
    shapes: GenerationLedger,
    joints: GenerationLedger,
    chains: GenerationLedger,
}

pub(crate) struct WorldCore {
    pub(crate) id: ffi::b2WorldId,
    pub(crate) brand: IdBrand,
    lifecycle: Cell<LifecycleState>,
    activity: Cell<ActivityState>,
    native_calls: Cell<usize>,
    shutdown_requested: Cell<bool>,
    native_destroyed: Cell<bool>,
    contact_epoch: Cell<ContactEpoch>,
    object_generations: RefCell<ObjectGenerationLedgers>,
    pub(crate) custom_filter: Mutex<Option<Box<CustomFilterCtx>>>,
    pub(crate) pre_solve: Mutex<Option<Box<PreSolveCtx>>>,
    pub(crate) material_mix_slot: Mutex<Option<usize>>,
    pub(crate) friction_mix: Mutex<Option<Box<MaterialMixCtx>>>,
    pub(crate) restitution_mix: Mutex<Option<Box<MaterialMixCtx>>>,
    pub(crate) worker_callbacks: Arc<WorkerCallbackState>,
    pub(crate) deferred_destroys: Mutex<Vec<DeferredDestroy>>,
    pub(crate) user_data: RefCell<crate::core::user_data::UserDataStore>,
    pub(crate) borrowed_event_buffers: AtomicUsize,
    #[cfg(feature = "serialize")]
    pub(crate) registries: Mutex<crate::core::serialize_registry::Registries>,
    pub(crate) owned_bodies: AtomicUsize,
    pub(crate) owned_shapes: AtomicUsize,
    pub(crate) owned_joints: AtomicUsize,
    pub(crate) owned_chains: AtomicUsize,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum DeferredDestroy {
    Body(BodyId),
    Shape { id: ShapeId, update_body_mass: bool },
    Joint { id: JointId, wake_bodies: bool },
    Chain(ChainId),
}

impl DeferredDestroy {
    fn is_stale_error(self, error: ApiError) -> bool {
        matches!(
            (self, error),
            (Self::Body(_), ApiError::InvalidBodyId)
                | (Self::Shape { .. }, ApiError::InvalidShapeId)
                | (Self::Joint { .. }, ApiError::InvalidJointId)
                | (Self::Chain(_), ApiError::InvalidChainId)
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum OwnedDestroyGate {
    Ready,
    Defer,
    WorldTeardown,
}

impl WorldCore {
    pub(crate) fn new(id: ffi::b2WorldId, brand: IdBrand) -> Rc<Self> {
        Rc::new(Self {
            id,
            brand,
            lifecycle: Cell::new(LifecycleState::Live),
            activity: Cell::new(ActivityState::Idle),
            native_calls: Cell::new(0),
            shutdown_requested: Cell::new(false),
            native_destroyed: Cell::new(false),
            contact_epoch: Cell::new(ContactEpoch::INITIAL),
            object_generations: RefCell::new(ObjectGenerationLedgers::default()),
            custom_filter: Mutex::new(None),
            pre_solve: Mutex::new(None),
            material_mix_slot: Mutex::new(None),
            friction_mix: Mutex::new(None),
            restitution_mix: Mutex::new(None),
            worker_callbacks: WorkerCallbackState::new(brand),
            deferred_destroys: Mutex::new(Vec::new()),
            user_data: RefCell::new(crate::core::user_data::UserDataStore::default()),
            borrowed_event_buffers: AtomicUsize::new(0),
            #[cfg(feature = "serialize")]
            registries: Mutex::new(crate::core::serialize_registry::Registries::default()),
            owned_bodies: AtomicUsize::new(0),
            owned_shapes: AtomicUsize::new(0),
            owned_joints: AtomicUsize::new(0),
            owned_chains: AtomicUsize::new(0),
        })
    }

    #[inline]
    pub(crate) const fn brand(&self) -> IdBrand {
        self.brand
    }

    #[inline]
    pub(crate) fn contact_epoch(&self) -> ContactEpoch {
        self.contact_epoch.get()
    }

    pub(crate) fn advance_contact_epoch(&self) -> ApiResult<ContactEpoch> {
        let next = match self.contact_epoch.get().checked_next() {
            Ok(next) => next,
            Err(error) => {
                self.poison();
                return Err(error);
            }
        };
        self.contact_epoch.set(next);
        Ok(next)
    }

    #[inline]
    pub(crate) fn check_available(&self) -> ApiResult<()> {
        match self.lifecycle.get() {
            LifecycleState::Live => {}
            LifecycleState::Poisoned => return Err(ApiError::WorldPoisoned),
            LifecycleState::Destroyed => return Err(ApiError::WorldDestroyed),
        }
        match self.activity.get() {
            ActivityState::Idle => Ok(()),
            ActivityState::Recording | ActivityState::Restoring => Err(ApiError::WorldBusy),
        }
    }

    pub(crate) fn begin_native_call(self: &Rc<Self>) -> ApiResult<NativeCallGuard> {
        self.check_available()?;
        let depth = self
            .native_calls
            .get()
            .checked_add(1)
            .expect("native call depth overflow");
        self.native_calls.set(depth);
        Ok(NativeCallGuard {
            core: Rc::clone(self),
        })
    }

    #[inline]
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "lifecycle proofs are consumed by U8 owners")
    )]
    pub(crate) fn lifecycle(&self) -> LifecycleState {
        self.lifecycle.get()
    }

    #[inline]
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "activity proofs are consumed by U8 owners")
    )]
    pub(crate) fn activity(&self) -> ActivityState {
        self.activity.get()
    }

    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "recording and restore owners are introduced in U8"
        )
    )]
    pub(crate) fn set_activity(
        &self,
        expected: ActivityState,
        next: ActivityState,
    ) -> ApiResult<()> {
        self.check_live()?;
        if self.activity.get() != expected {
            return Err(ApiError::WorldBusy);
        }
        self.activity.set(next);
        Ok(())
    }

    pub(crate) fn poison(&self) {
        if self.lifecycle.get() == LifecycleState::Live {
            self.lifecycle.set(LifecycleState::Poisoned);
        }
    }

    #[inline]
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "recording and restore owners are introduced in U8"
        )
    )]
    fn check_live(&self) -> ApiResult<()> {
        match self.lifecycle.get() {
            LifecycleState::Live => Ok(()),
            LifecycleState::Poisoned => Err(ApiError::WorldPoisoned),
            LifecycleState::Destroyed => Err(ApiError::WorldDestroyed),
        }
    }

    #[inline]
    fn check_brand_identity(&self, brand: IdBrand) -> ApiResult<()> {
        if brand == self.brand {
            Ok(())
        } else {
            Err(ApiError::WrongWorld)
        }
    }

    #[inline]
    fn check_brand(&self, brand: IdBrand) -> ApiResult<()> {
        self.check_available()?;
        self.check_brand_identity(brand)
    }

    #[inline]
    fn check_body_native(&self, id: BodyId) -> ApiResult<()> {
        if unsafe { ffi::b2Body_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidBodyId)
        }
    }

    #[inline]
    pub(crate) fn check_body(&self, id: BodyId) -> ApiResult<()> {
        self.check_brand(id.brand())?;
        self.check_body_native(id)
    }

    #[inline]
    fn check_shape_native(&self, id: ShapeId) -> ApiResult<()> {
        if unsafe { ffi::b2Shape_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidShapeId)
        }
    }

    #[inline]
    pub(crate) fn check_shape(&self, id: ShapeId) -> ApiResult<()> {
        self.check_brand(id.brand())?;
        self.check_shape_native(id)
    }

    #[inline]
    fn check_joint_native(&self, id: JointId) -> ApiResult<()> {
        if unsafe { ffi::b2Joint_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidJointId)
        }
    }

    #[inline]
    pub(crate) fn check_joint(&self, id: JointId) -> ApiResult<()> {
        self.check_brand(id.brand())?;
        self.check_joint_native(id)
    }

    #[inline]
    fn check_chain_native(&self, id: ChainId) -> ApiResult<()> {
        if unsafe { ffi::b2Chain_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidChainId)
        }
    }

    #[inline]
    pub(crate) fn check_chain(&self, id: ChainId) -> ApiResult<()> {
        self.check_brand(id.brand())?;
        self.check_chain_native(id)
    }

    #[inline]
    fn check_contact_identity(&self, id: crate::types::ContactId) -> ApiResult<()> {
        self.check_brand(id.brand())?;
        if id.contact_epoch() == self.contact_epoch() {
            Ok(())
        } else {
            Err(ApiError::InvalidContactId)
        }
    }

    #[inline]
    fn check_contact_native(&self, id: crate::types::ContactId) -> ApiResult<()> {
        if unsafe { ffi::b2Contact_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidContactId)
        }
    }

    #[inline]
    pub(crate) fn check_contact(&self, id: crate::types::ContactId) -> ApiResult<()> {
        self.check_contact_identity(id)?;
        self.check_contact_native(id)
    }

    #[inline]
    pub(crate) fn contact_is_valid(&self, id: crate::types::ContactId) -> ApiResult<bool> {
        self.check_brand(id.brand())?;
        if id.contact_epoch() != self.contact_epoch() {
            return Ok(false);
        }
        Ok(unsafe { ffi::b2Contact_IsValid(id.into_raw()) })
    }

    pub(crate) fn bind_body(&self, raw: RawBodyId) -> ApiResult<BodyId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.try_body(raw.into_ffi())?;
        self.check_body_native(id)?;
        Ok(id)
    }

    pub(crate) fn bind_shape(&self, raw: RawShapeId) -> ApiResult<ShapeId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.try_shape(raw.into_ffi())?;
        self.check_shape_native(id)?;
        Ok(id)
    }

    pub(crate) fn bind_joint(&self, raw: RawJointId) -> ApiResult<JointId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.try_joint(raw.into_ffi())?;
        self.check_joint_native(id)?;
        Ok(id)
    }

    pub(crate) fn bind_chain(&self, raw: RawChainId) -> ApiResult<ChainId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.try_chain(raw.into_ffi())?;
        self.check_chain_native(id)?;
        Ok(id)
    }

    pub(crate) fn bind_contact(&self, raw: RawContactId) -> ApiResult<crate::types::ContactId> {
        self.check_available()?;
        let epoch = self.contact_epoch();
        raw.validate_for(self.brand, epoch)?;
        let id = self.brand.try_contact(raw.into_ffi(), epoch)?;
        self.check_contact_native(id)?;
        Ok(id)
    }

    fn poison_created_output_error<T>(&self, result: ApiResult<T>) -> ApiResult<T> {
        if result.is_err() {
            self.poison();
        }
        result
    }

    fn register_body_generation(&self, id: BodyId) -> ApiResult<()> {
        let raw = id.into_raw();
        self.object_generations
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .register(raw.index1, raw.generation)
    }

    fn register_shape_generation(&self, id: ShapeId) -> ApiResult<()> {
        let raw = id.into_raw();
        self.object_generations
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .register(raw.index1, raw.generation)
    }

    fn register_joint_generation(&self, id: JointId) -> ApiResult<()> {
        let raw = id.into_raw();
        self.object_generations
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .register(raw.index1, raw.generation)
    }

    fn register_chain_generation(&self, id: ChainId) -> ApiResult<()> {
        let raw = id.into_raw();
        self.object_generations
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .chains
            .register(raw.index1, raw.generation)
    }

    pub(crate) fn finish_created_body(&self, raw: ffi::b2BodyId) -> ApiResult<BodyId> {
        let result = (|| {
            self.check_available()?;
            let id = self.brand.try_body(raw)?;
            self.check_body_native(id)?;
            self.register_body_generation(id)?;
            Ok(id)
        })();
        self.poison_created_output_error(result)
    }

    pub(crate) fn finish_created_shape(&self, raw: ffi::b2ShapeId) -> ApiResult<ShapeId> {
        let result = (|| {
            self.check_available()?;
            let id = self.brand.try_shape(raw)?;
            self.check_shape_native(id)?;
            self.register_shape_generation(id)?;
            Ok(id)
        })();
        self.poison_created_output_error(result)
    }

    pub(crate) fn finish_created_joint(&self, raw: ffi::b2JointId) -> ApiResult<JointId> {
        let result = (|| {
            self.check_available()?;
            let id = self.brand.try_joint(raw)?;
            self.check_joint_native(id)?;
            self.register_joint_generation(id)?;
            Ok(id)
        })();
        self.poison_created_output_error(result)
    }

    pub(crate) fn finish_created_chain(&self, raw: ffi::b2ChainId) -> ApiResult<ChainId> {
        let result = (|| {
            self.check_available()?;
            let id = self.brand.try_chain(raw)?;
            self.check_chain_native(id)?;

            let count = unsafe { ffi::b2Chain_GetSegmentCount(id.into_raw()) };
            let segments = unsafe {
                crate::core::ffi_vec::try_read_mapped_from_ffi(
                    count,
                    |out, capacity| ffi::b2Chain_GetSegments(id.into_raw(), out, capacity),
                    |shape| self.brand.try_shape(shape),
                )
            }?;
            for &segment in &segments {
                self.check_shape_native(segment)?;
            }

            self.register_chain_generation(id)?;
            for segment in segments {
                self.register_shape_generation(segment)?;
            }
            Ok(id)
        })();
        self.poison_created_output_error(result)
    }

    pub(crate) fn owned_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.owned_bodies.load(Ordering::Relaxed),
            self.owned_shapes.load(Ordering::Relaxed),
            self.owned_joints.load(Ordering::Relaxed),
            self.owned_chains.load(Ordering::Relaxed),
        )
    }

    #[inline]
    fn owned_destroy_gate(&self) -> OwnedDestroyGate {
        if crate::core::callback_state::in_callback() {
            return OwnedDestroyGate::Defer;
        }
        match self.lifecycle.get() {
            LifecycleState::Live => {}
            LifecycleState::Poisoned | LifecycleState::Destroyed => {
                return OwnedDestroyGate::WorldTeardown;
            }
        }
        if self.activity.get() != ActivityState::Idle || self.events_buffers_are_borrowed() {
            OwnedDestroyGate::Defer
        } else {
            OwnedDestroyGate::Ready
        }
    }

    pub(crate) fn check_owned_policy_change(&self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.check_available()
    }

    pub(crate) fn defer_destroy(&self, d: DeferredDestroy) {
        self.deferred_destroys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(d);
    }

    pub(crate) fn events_buffers_are_borrowed(&self) -> bool {
        self.borrowed_event_buffers.load(Ordering::Relaxed) > 0
    }

    pub(crate) fn borrow_event_buffers(self: &Rc<Self>) -> BorrowedEventBuffersGuard {
        self.borrowed_event_buffers.fetch_add(1, Ordering::Relaxed);
        BorrowedEventBuffersGuard {
            core: Rc::clone(self),
        }
    }

    fn body_shapes_for_destroy(&self, id: BodyId) -> ApiResult<Vec<ShapeId>> {
        let raw = id.into_raw();
        let count = unsafe { ffi::b2Body_GetShapeCount(raw) };
        unsafe {
            crate::core::ffi_vec::try_read_mapped_from_ffi(
                count,
                |out, capacity| ffi::b2Body_GetShapes(raw, out, capacity),
                |shape| self.brand.try_shape(shape),
            )
        }
    }

    fn body_joints_for_destroy(&self, id: BodyId) -> ApiResult<Vec<JointId>> {
        let raw = id.into_raw();
        let count = unsafe { ffi::b2Body_GetJointCount(raw) };
        unsafe {
            crate::core::ffi_vec::try_read_mapped_from_ffi(
                count,
                |out, capacity| ffi::b2Body_GetJoints(raw, out, capacity),
                |joint| self.brand.try_joint(joint),
            )
        }
    }

    fn chain_shapes_for_destroy(&self, id: ChainId) -> ApiResult<Vec<ShapeId>> {
        let raw = id.into_raw();
        let count = unsafe { ffi::b2Chain_GetSegmentCount(raw) };
        unsafe {
            crate::core::ffi_vec::try_read_mapped_from_ffi(
                count,
                |out, capacity| ffi::b2Chain_GetSegments(raw, out, capacity),
                |shape| self.brand.try_shape(shape),
            )
        }
    }

    fn check_user_data_mutable(
        &self,
        body: Option<BodyId>,
        shapes: &[ShapeId],
        joints: &[JointId],
    ) -> ApiResult<()> {
        let entries = {
            let store = self
                .user_data
                .try_borrow()
                .map_err(|_| ApiError::ReentrantAccess)?;
            let mut entries = Vec::new();
            if let Some(body) = body
                && let Some(entry) = store.bodies.get(&crate::core::user_data::IdKey::from(body))
            {
                entries.push(Rc::clone(entry));
            }
            entries.extend(shapes.iter().filter_map(|id| {
                store
                    .shapes
                    .get(&crate::core::user_data::IdKey::from(*id))
                    .cloned()
            }));
            entries.extend(joints.iter().filter_map(|id| {
                store
                    .joints
                    .get(&crate::core::user_data::IdKey::from(*id))
                    .cloned()
            }));
            entries
        };

        for entry in entries {
            entry.check_mutable()?;
        }
        Ok(())
    }

    fn check_object_destroy_preconditions(&self, brand: IdBrand) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.check_available()?;
        self.check_brand_identity(brand)?;
        if self.events_buffers_are_borrowed() {
            return Err(ApiError::WorldBusy);
        }
        Ok(())
    }

    fn retire_user_data(
        &self,
        body: Option<BodyId>,
        shapes: &[ShapeId],
        joints: &[JointId],
    ) -> Vec<crate::core::user_data::ErasedUserData> {
        let mut retired = Vec::new();
        if let Some(body) = body
            && let Some(value) = self
                .clear_body_user_data(body)
                .expect("user-data mutability checked before native destroy")
        {
            retired.push(value);
        }
        for &shape in shapes {
            if let Some(value) = self
                .clear_shape_user_data(shape)
                .expect("user-data mutability checked before native destroy")
            {
                retired.push(value);
            }
        }
        for &joint in joints {
            if let Some(value) = self
                .clear_joint_user_data(joint)
                .expect("user-data mutability checked before native destroy")
            {
                retired.push(value);
            }
        }
        retired
    }

    pub(crate) fn destroy_body_now(&self, id: BodyId) -> ApiResult<()> {
        self.check_object_destroy_preconditions(id.brand())?;
        self.check_body_native(id)?;
        let shapes = self.body_shapes_for_destroy(id)?;
        let joints = self.body_joints_for_destroy(id)?;
        self.check_user_data_mutable(Some(id), &shapes, &joints)?;
        #[cfg(feature = "serialize")]
        self.cleanup_before_destroy_body(id);
        unsafe { ffi::b2DestroyBody(id.into_raw()) };
        drop(self.retire_user_data(Some(id), &shapes, &joints));
        Ok(())
    }

    pub(crate) fn destroy_shape_now(&self, id: ShapeId, update_body_mass: bool) -> ApiResult<()> {
        self.check_object_destroy_preconditions(id.brand())?;
        self.check_shape_native(id)?;
        self.check_user_data_mutable(None, core::slice::from_ref(&id), &[])?;
        unsafe { ffi::b2DestroyShape(id.into_raw(), update_body_mass) };
        #[cfg(feature = "serialize")]
        self.remove_shape_flags(id);
        drop(self.retire_user_data(None, core::slice::from_ref(&id), &[]));
        Ok(())
    }

    pub(crate) fn destroy_joint_now(&self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        self.check_object_destroy_preconditions(id.brand())?;
        self.check_joint_native(id)?;
        self.check_user_data_mutable(None, &[], core::slice::from_ref(&id))?;
        unsafe { ffi::b2DestroyJoint(id.into_raw(), wake_bodies) };
        drop(self.retire_user_data(None, &[], core::slice::from_ref(&id)));
        Ok(())
    }

    pub(crate) fn destroy_chain_now(&self, id: ChainId) -> ApiResult<()> {
        self.check_object_destroy_preconditions(id.brand())?;
        self.check_chain_native(id)?;
        let shapes = self.chain_shapes_for_destroy(id)?;
        self.check_user_data_mutable(None, &shapes, &[])?;
        unsafe { ffi::b2DestroyChain(id.into_raw()) };
        #[cfg(feature = "serialize")]
        self.remove_chain(id);
        drop(self.retire_user_data(None, &shapes, &[]));
        Ok(())
    }

    fn destroy_deferred_now(&self, item: DeferredDestroy) -> ApiResult<()> {
        match item {
            DeferredDestroy::Body(id) => self.destroy_body_now(id),
            DeferredDestroy::Shape {
                id,
                update_body_mass,
            } => self.destroy_shape_now(id, update_body_mass),
            DeferredDestroy::Joint { id, wake_bodies } => self.destroy_joint_now(id, wake_bodies),
            DeferredDestroy::Chain(id) => self.destroy_chain_now(id),
        }
    }

    pub(crate) fn destroy_owned_or_defer(&self, item: DeferredDestroy) {
        match self.owned_destroy_gate() {
            OwnedDestroyGate::Defer => self.defer_destroy(item),
            OwnedDestroyGate::WorldTeardown => {}
            OwnedDestroyGate::Ready => match self.destroy_deferred_now(item) {
                Ok(()) => {}
                Err(error) if item.is_stale_error(error) => {}
                Err(ApiError::WorldPoisoned | ApiError::WorldDestroyed) => {}
                Err(_) => self.defer_destroy(item),
            },
        }
    }

    pub(crate) fn process_deferred_destroys(&self) {
        if self.owned_destroy_gate() != OwnedDestroyGate::Ready {
            return;
        }
        let mut pending = self
            .deferred_destroys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if pending.is_empty() {
            return;
        }
        let items = core::mem::take(&mut *pending);
        drop(pending);

        let mut retry = Vec::new();
        for item in items {
            match self.destroy_deferred_now(item) {
                Ok(()) => {}
                Err(error) if item.is_stale_error(error) => {}
                Err(_) => retry.push(item),
            }
        }

        if !retry.is_empty() {
            let mut pending = self
                .deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            retry.append(&mut pending);
            *pending = retry;
        }
    }

    /// End the native world's lifetime while retaining an inert Rust shell for residual handles.
    pub(crate) fn shutdown_native(&self) {
        if self.native_destroyed.get() {
            return;
        }
        self.lifecycle.set(LifecycleState::Destroyed);
        self.activity.set(ActivityState::Idle);
        self.shutdown_requested.set(true);
        if self.native_calls.get() == 0 {
            self.finish_native_shutdown();
        }
    }

    fn finish_native_shutdown(&self) {
        if self.native_destroyed.replace(true) {
            return;
        }
        debug_assert!(self.shutdown_requested.get());
        debug_assert_eq!(self.native_calls.get(), 0);

        {
            let _guard = crate::core::box2d_lock::lock();
            // SAFETY: `World` owns the native lifetime and this method transitions the shared
            // lifecycle to `Destroyed` before making the one idempotent teardown call.
            unsafe { ffi::b2DestroyWorld(self.id) };
        }

        if let Some(slot) = self
            .material_mix_slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            crate::core::material_mix_registry::set_friction_ptr(slot, core::ptr::null_mut());
            crate::core::material_mix_registry::set_restitution_ptr(slot, core::ptr::null_mut());
            crate::core::material_mix_registry::release_slot(slot);
        }

        let custom_filter = self
            .custom_filter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let pre_solve = self
            .pre_solve
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let friction_mix = self
            .friction_mix
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let restitution_mix = self
            .restitution_mix
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();

        self.worker_callbacks.clear_panic();
        self.deferred_destroys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        #[cfg(feature = "serialize")]
        {
            *self
                .registries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                crate::core::serialize_registry::Registries::default();
        }

        // Replacing the whole store severs core -> user data -> handle -> core cycles. Payloads
        // are dropped only after native teardown, so their owned handles cannot call object FFI.
        let user_data = self
            .user_data
            .replace(crate::core::user_data::UserDataStore::default());

        drop(custom_filter);
        drop(pre_solve);
        drop(friction_mix);
        drop(restitution_mix);
        drop(user_data);
    }

    pub(crate) fn clear_world_user_data(
        &self,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.world.as_ref().cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.world = None;
        Ok(retired)
    }

    pub(crate) fn clear_body_user_data(
        &self,
        id: BodyId,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.bodies.get(&key).cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.bodies.remove(&key);
        Ok(retired)
    }

    pub(crate) fn clear_shape_user_data(
        &self,
        id: ShapeId,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.shapes.get(&key).cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.shapes.remove(&key);
        Ok(retired)
    }

    pub(crate) fn clear_joint_user_data(
        &self,
        id: JointId,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.joints.get(&key).cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.joints.remove(&key);
        Ok(retired)
    }

    pub(crate) fn set_world_user_data<T: 'static>(
        &self,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .world
            .clone();
        if let Some(entry) = entry {
            return entry.replace(value);
        }

        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value);
        self.user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .world = Some(entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_body_user_data<T: 'static>(
        &self,
        id: BodyId,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .get(&key)
            .cloned();
        if let Some(entry) = entry {
            return entry.replace(value);
        }

        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value);
        self.user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .insert(key, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .get(&key)
            .cloned();
        if let Some(entry) = entry {
            return entry.replace(value);
        }

        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value);
        self.user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .insert(key, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_joint_user_data<T: 'static>(
        &self,
        id: JointId,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .get(&key)
            .cloned();
        if let Some(entry) = entry {
            return entry.replace(value);
        }

        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value);
        self.user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .insert(key, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn try_with_world_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .world
            .clone();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn try_with_body_user_data<T: 'static, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .get(&key)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn try_with_shape_user_data<T: 'static, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .get(&key)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn try_with_joint_user_data<T: 'static, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .get(&key)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn try_with_body_user_data_mut<T: 'static, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&mut T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .get(&key)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with_mut(f)
    }

    pub(crate) fn try_with_shape_user_data_mut<T: 'static, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&mut T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .get(&key)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with_mut(f)
    }

    pub(crate) fn try_with_joint_user_data_mut<T: 'static, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&mut T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let key = crate::core::user_data::IdKey::from(id);
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .get(&key)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with_mut(f)
    }

    pub(crate) fn take_world_user_data<T: 'static>(&self) -> crate::error::ApiResult<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.world.as_ref().cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.world = None;
        Ok(value)
    }

    pub(crate) fn take_body_user_data<T: 'static>(
        &self,
        id: BodyId,
    ) -> crate::error::ApiResult<Option<T>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.bodies.get(&key).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.bodies.remove(&key);
        Ok(value)
    }

    pub(crate) fn take_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
    ) -> crate::error::ApiResult<Option<T>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.shapes.get(&key).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.shapes.remove(&key);
        Ok(value)
    }

    pub(crate) fn take_joint_user_data<T: 'static>(
        &self,
        id: JointId,
    ) -> crate::error::ApiResult<Option<T>> {
        let key = crate::core::user_data::IdKey::from(id);
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.joints.get(&key).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.joints.remove(&key);
        Ok(value)
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
    pub(crate) fn record_shape_flags(&self, sid: ShapeId, body: BodyId, def: &ffi::b2ShapeDef) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_shape_flags(sid, body, def);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn remove_chain(&self, id: crate::types::ChainId) {
        self.registries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove_chain(id);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn remove_shape_flags(&self, sid: ShapeId) {
        self.registries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove_shape_flags(sid);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn cleanup_before_destroy_body(&self, id: BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        let mut r = self
            .registries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        r.remove_shape_flags_for_body(id);
        r.remove_chains_for_body(id);
        r.remove_body(id);
    }
}

pub(crate) struct BorrowedEventBuffersGuard {
    core: Rc<WorldCore>,
}

pub(crate) struct NativeCallGuard {
    core: Rc<WorldCore>,
}

impl Drop for NativeCallGuard {
    fn drop(&mut self) {
        let depth = self.core.native_calls.get();
        debug_assert!(depth > 0, "native call counter underflow");
        self.core.native_calls.set(depth.saturating_sub(1));
        if depth == 1 && self.core.shutdown_requested.get() {
            self.core.finish_native_shutdown();
        }
    }
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
        self.shutdown_native();
    }
}

#[cfg(test)]
mod identity_tests {
    use super::{GenerationLedger, LifecycleState};
    use crate::id::ContactEpoch;
    use crate::{ApiError, BodyBuilder, World, WorldDef};
    use boxdd_sys::ffi;

    #[test]
    fn generation_ledger_rejects_repeated_or_wrapped_slot_generations() {
        let mut ledger = GenerationLedger::default();

        assert_eq!(ledger.register(1, u16::MAX - 1), Ok(()));
        assert_eq!(ledger.register(1, u16::MAX), Ok(()));
        assert_eq!(
            ledger.register(1, 0),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(
            ledger.register(1, u16::MAX),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(ledger.register(2, 0), Ok(()));
    }

    #[test]
    fn contact_epoch_exhaustion_poisons_the_world() {
        let world = World::new(WorldDef::default()).unwrap();
        world
            .core()
            .contact_epoch
            .set(ContactEpoch::new_for_test(u64::MAX));

        assert_eq!(
            world.core().advance_contact_epoch(),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
    }

    #[test]
    fn post_create_generation_conflict_poisons_the_world() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyBuilder::new().build());
        let core = world.core_rc();

        // Creation paths register this generation. The explicit registration keeps this test
        // valid while those paths are migrated in parallel.
        let _ = core.register_body_generation(body);
        assert_eq!(
            core.finish_created_body(body.into_raw()),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
    }

    #[test]
    fn invalid_native_creation_outputs_poison_without_registration() {
        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_body(ffi::b2BodyId {
                index1: 0,
                world0: core.brand().world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidBodyId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
        assert!(
            core.object_generations
                .borrow()
                .bodies
                .last_by_slot
                .is_empty()
        );

        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_shape(ffi::b2ShapeId {
                index1: 0,
                world0: core.brand().world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidShapeId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
        assert!(
            core.object_generations
                .borrow()
                .shapes
                .last_by_slot
                .is_empty()
        );

        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_joint(ffi::b2JointId {
                index1: 0,
                world0: core.brand().world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidJointId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
        assert!(
            core.object_generations
                .borrow()
                .joints
                .last_by_slot
                .is_empty()
        );

        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_chain(ffi::b2ChainId {
                index1: 0,
                world0: core.brand().world0(),
                generation: 0,
            }),
            Err(ApiError::InvalidChainId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
        assert!(
            core.object_generations
                .borrow()
                .chains
                .last_by_slot
                .is_empty()
        );
    }

    #[test]
    fn foreign_identity_wins_while_target_event_buffers_are_borrowed() {
        let mut source = World::new(WorldDef::default()).unwrap();
        let foreign = source.create_body_id(BodyBuilder::new().build());
        let target = World::new(WorldDef::default()).unwrap();
        let target_core = target.core_rc();
        let deferred_before = target_core
            .deferred_destroys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        let _event_borrow = target_core.borrow_event_buffers();

        assert_eq!(
            target_core.destroy_body_now(foreign),
            Err(ApiError::WrongWorld)
        );
        assert_eq!(
            target_core
                .deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            deferred_before
        );
        assert_eq!(source.core().check_body(foreign), Ok(()));
    }

    #[test]
    fn native_call_guard_defers_world_teardown_but_terminalizes_rust_state() {
        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        let call = core.begin_native_call().unwrap();

        drop(world);

        assert_eq!(core.check_available(), Err(ApiError::WorldDestroyed));
        assert!(unsafe { ffi::b2World_IsValid(core.id) });
        assert!(!core.native_destroyed.get());

        drop(call);

        assert!(!unsafe { ffi::b2World_IsValid(core.id) });
        assert!(core.native_destroyed.get());
    }

    #[test]
    fn destroy_checks_identity_and_native_validity_before_unrelated_user_data() {
        let mut source = World::new(WorldDef::default()).unwrap();
        let foreign = source.create_body_id(BodyBuilder::new().build());
        let mut target = World::new(WorldDef::default()).unwrap();
        target.set_user_data(7_u32);
        let target_core = target.core_rc();

        let foreign_result =
            target.with_user_data::<u32, _>(|_| target_core.destroy_body_now(foreign));
        assert_eq!(foreign_result, Some(Err(ApiError::WrongWorld)));
        assert_eq!(source.core().check_body(foreign), Ok(()));

        let stale = target.create_body_id(BodyBuilder::new().build());
        target.destroy_body_id(stale);
        let stale_result = target.with_user_data::<u32, _>(|_| target_core.destroy_body_now(stale));
        assert_eq!(stale_result, Some(Err(ApiError::InvalidBodyId)));
    }
}

#[cfg(test)]
mod auto_trait_tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread::{self, ThreadId};

    use static_assertions::{assert_impl_all, assert_not_impl_any};

    use super::{ActivityState, LifecycleState, WorkerCallbackState, WorldCore, ffi};
    use crate::{ApiError, BodyBuilder, BodyType, ShapeDef, Vec2, World, WorldDef, shapes};

    assert_impl_all!(WorkerCallbackState: Send, Sync);
    assert_not_impl_any!(WorldCore: Send, Sync);

    static PANICKING_PAYLOAD_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct PanickingPayload;

    impl Drop for PanickingPayload {
        fn drop(&mut self) {
            PANICKING_PAYLOAD_DROPS.fetch_add(1, Ordering::SeqCst);
            panic!("panic payload destructor must not run on a worker callback stack");
        }
    }

    struct WorldCyclePayload {
        _handle: crate::WorldHandle,
        dropped_on: Rc<RefCell<Option<ThreadId>>>,
    }

    impl Drop for WorldCyclePayload {
        fn drop(&mut self) {
            *self.dropped_on.borrow_mut() = Some(thread::current().id());
        }
    }

    struct BodyCyclePayload {
        _owned_body: crate::OwnedBody,
        _handle: crate::WorldHandle,
        dropped_on: Rc<RefCell<Option<ThreadId>>>,
    }

    impl Drop for BodyCyclePayload {
        fn drop(&mut self) {
            *self.dropped_on.borrow_mut() = Some(thread::current().id());
        }
    }

    #[test]
    fn callback_and_owner_state_have_distinct_threading_contracts() {
        // Compile-time assertions above are the behavior under test.
    }

    #[test]
    fn competing_worker_panic_payload_is_not_dropped_on_the_callback_stack() {
        PANICKING_PAYLOAD_DROPS.store(0, Ordering::SeqCst);
        let world = World::new(WorldDef::default()).unwrap();
        let worker = &world.core().worker_callbacks;

        worker.record_panic(Box::new("first panic"));
        worker.record_panic(Box::new(PanickingPayload));

        assert_eq!(PANICKING_PAYLOAD_DROPS.load(Ordering::SeqCst), 0);
        let first = worker.take_panic().expect("first panic payload");
        assert_eq!(first.downcast_ref::<&str>(), Some(&"first panic"));
        worker.clear_panic();
    }

    #[test]
    fn lifecycle_and_activity_are_orthogonal() {
        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core();

        assert_eq!(core.lifecycle(), LifecycleState::Live);
        assert_eq!(core.activity(), ActivityState::Idle);
        assert_eq!(
            core.set_activity(ActivityState::Idle, ActivityState::Recording),
            Ok(())
        );
        assert_eq!(core.check_available(), Err(ApiError::WorldBusy));
        assert_eq!(
            core.set_activity(ActivityState::Recording, ActivityState::Idle),
            Ok(())
        );
        assert_eq!(
            core.set_activity(ActivityState::Idle, ActivityState::Restoring),
            Ok(())
        );
        assert_eq!(core.check_available(), Err(ApiError::WorldBusy));
        assert_eq!(
            core.set_activity(ActivityState::Restoring, ActivityState::Idle),
            Ok(())
        );

        core.poison();
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
        assert_eq!(core.activity(), ActivityState::Idle);
        assert_eq!(core.check_available(), Err(ApiError::WorldPoisoned));
    }

    #[test]
    fn world_drop_breaks_world_user_data_handle_cycles() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let raw_world = world.raw();
        let survivor = world.handle();
        let cycle_handle = world.handle();
        let core = world.core_rc();
        let weak = Rc::downgrade(&core);
        let dropped_on = Rc::new(RefCell::new(None));
        let owner_thread = thread::current().id();

        world.set_user_data(WorldCyclePayload {
            _handle: cycle_handle,
            dropped_on: Rc::clone(&dropped_on),
        });
        drop(world);

        assert!(!unsafe { ffi::b2World_IsValid(raw_world) });
        assert_eq!(core.lifecycle(), LifecycleState::Destroyed);
        assert_eq!(survivor.try_gravity(), Err(ApiError::WorldDestroyed));
        assert_eq!(*dropped_on.borrow(), Some(owner_thread));

        drop(survivor);
        drop(core);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn world_drop_breaks_body_user_data_owned_handle_cycles() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let mut owner =
            world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let payload_body = world.create_body_owned(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 0.0])
                .build(),
        );
        let survivor = world.handle();
        let payload_handle = world.handle();
        let raw_world = world.raw();
        let core = world.core_rc();
        let weak = Rc::downgrade(&core);
        let dropped_on = Rc::new(RefCell::new(None));
        let owner_thread = thread::current().id();

        owner.set_user_data(BodyCyclePayload {
            _owned_body: payload_body,
            _handle: payload_handle,
            dropped_on: Rc::clone(&dropped_on),
        });
        drop(world);

        assert!(!unsafe { ffi::b2World_IsValid(raw_world) });
        assert_eq!(core.lifecycle(), LifecycleState::Destroyed);
        assert_eq!(owner.try_position(), Err(ApiError::WorldDestroyed));
        assert_eq!(survivor.try_gravity(), Err(ApiError::WorldDestroyed));
        assert_eq!(*dropped_on.borrow(), Some(owner_thread));

        drop(owner);
        drop(survivor);
        drop(core);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn world_drop_recovers_a_poisoned_box2d_lock() {
        let world = World::new(WorldDef::default()).unwrap();
        let raw_world = world.raw();
        let core = world.core_rc();

        let poison = std::panic::catch_unwind(|| {
            let _guard = crate::core::box2d_lock::lock();
            panic!("poison the Box2D lock for teardown coverage");
        });
        assert!(poison.is_err());

        drop(world);
        assert!(!unsafe { ffi::b2World_IsValid(raw_world) });
        assert_eq!(core.lifecycle(), LifecycleState::Destroyed);
    }

    #[test]
    fn stale_owned_drop_is_ignored_during_unrelated_user_data_borrow() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_owned(BodyBuilder::new().build());
        let id = body.id();
        let core = world.core_rc();
        world.destroy_body_id(id);
        world.set_user_data(7_u32);

        assert_eq!(
            world.with_user_data::<u32, _>(|value| {
                assert_eq!(*value, 7);
                drop(body);
                core.deferred_destroys
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .len()
            }),
            Some(0)
        );

        core.process_deferred_destroys();
        assert!(
            core.deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn busy_owned_drop_stays_pending_until_the_world_is_idle() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let id = body.id();
        let core = world.core_rc();

        core.set_activity(ActivityState::Idle, ActivityState::Recording)
            .unwrap();
        drop(body);
        assert_eq!(
            core.deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            1
        );

        core.process_deferred_destroys();
        assert_eq!(
            core.deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            1
        );

        core.set_activity(ActivityState::Recording, ActivityState::Idle)
            .unwrap();
        core.process_deferred_destroys();
        assert_eq!(core.check_body(id), Err(ApiError::InvalidBodyId));
        assert!(
            core.deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn busy_into_id_keeps_raii_armed_during_unwind() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let id = body.id();
        let core = world.core_rc();
        core.set_activity(ActivityState::Idle, ActivityState::Recording)
            .unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body.into_id()));
        assert!(result.is_err());
        assert_eq!(
            core.deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            1
        );

        core.set_activity(ActivityState::Recording, ActivityState::Idle)
            .unwrap();
        core.process_deferred_destroys();
        assert_eq!(core.check_body(id), Err(ApiError::InvalidBodyId));
    }

    #[test]
    fn poisoned_owned_drops_leave_native_objects_for_world_teardown() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let owned_body = world.create_body_owned(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([10.0_f32, 0.0])
                .build(),
        );
        let body_a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let body_b = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 0.0])
                .build(),
        );
        let owned_shape = world.create_circle_shape_for_owned(
            body_a,
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5),
        );
        let owned_joint = world.create_distance_joint_owned(
            &crate::joints::DistanceJointDef::new(crate::joints::JointBase::new(body_a, body_b))
                .length(1.0),
        );
        let chain_def = crate::shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build();
        let owned_chain = world.create_chain_for_owned(body_b, &chain_def);
        let body_id = owned_body.id();
        let shape_id = owned_shape.id();
        let joint_id = owned_joint.id();
        let chain_id = owned_chain.id();
        let core = world.core_rc();

        core.poison();
        drop(owned_body);
        drop(owned_shape);
        drop(owned_joint);
        drop(owned_chain);

        assert!(unsafe { ffi::b2Body_IsValid(body_id.into_raw()) });
        assert!(unsafe { ffi::b2Shape_IsValid(shape_id.into_raw()) });
        assert!(unsafe { ffi::b2Joint_IsValid(joint_id.into_raw()) });
        assert!(unsafe { ffi::b2Chain_IsValid(chain_id.into_raw()) });
        assert!(
            core.deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );

        let raw_world = core.id;
        drop(world);
        assert!(!unsafe { ffi::b2World_IsValid(raw_world) });
        assert_eq!(core.lifecycle(), LifecycleState::Destroyed);
    }
}
