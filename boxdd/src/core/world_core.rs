use boxdd_sys::ffi;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{
    CustomFilterCtx, MaterialMixCtx, PreSolveCtx, WorkerCallbackState,
};
use crate::error::{ApiError, ApiResult};
use crate::id::{
    ContactEpoch, IdBrand, RawBodyId, RawChainId, RawContactId, RawJointId, RawShapeId,
};
use crate::joints::JointType;
use crate::types::{BodyId, ChainId, JointId, ShapeId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum LifecycleState {
    Live,
    Poisoned,
    Destroyed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActivityState {
    Idle,
    Recording,
    Restoring,
}

/// The activity state an internal semantic operation is authorized to use.
///
/// Public world and handle APIs always select `Idle`. The recording owner is
/// the only caller allowed to select `Recording`, so existing aliases remain
/// gated for the complete session lifetime.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorldAccess {
    Idle,
    Recording,
}

impl WorldAccess {
    #[inline]
    const fn activity(self) -> ActivityState {
        match self {
            Self::Idle => ActivityState::Idle,
            Self::Recording => ActivityState::Recording,
        }
    }
}

pub(crate) struct WorldCore {
    self_weak: Weak<WorldCore>,
    pub(crate) id: ffi::b2WorldId,
    pub(crate) brand: IdBrand,
    lifecycle: Cell<LifecycleState>,
    activity: Cell<ActivityState>,
    native_calls: Cell<usize>,
    #[cfg(test)]
    native_object_checks: Cell<usize>,
    user_data_accesses: Cell<usize>,
    shutdown_requested: Cell<bool>,
    native_destroyed: Cell<bool>,
    contact_epoch: Cell<ContactEpoch>,
    identities: Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) custom_filter: Mutex<Option<Box<CustomFilterCtx>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pre_solve: Mutex<Option<Box<PreSolveCtx>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) material_mix_slot: Mutex<Option<usize>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) friction_mix: Mutex<Option<Box<MaterialMixCtx>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) restitution_mix: Mutex<Option<Box<MaterialMixCtx>>>,
    #[cfg(not(target_arch = "wasm32"))]
    friction_mixer_present: Cell<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    restitution_mixer_present: Cell<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) worker_callbacks: Arc<WorkerCallbackState>,
    pub(crate) deferred_destroys: Mutex<VecDeque<DeferredDestroy>>,
    pub(crate) user_data: RefCell<crate::core::user_data::UserDataStore>,
    pub(crate) borrowed_event_buffers: AtomicUsize,
    pub(crate) owned_bodies: AtomicUsize,
    pub(crate) owned_shapes: AtomicUsize,
    pub(crate) owned_joints: AtomicUsize,
    pub(crate) owned_chains: AtomicUsize,
    // Keep this last as a fallback: Rust drops struct fields in declaration order. Normal teardown
    // takes the lease into a local, but an unexpectedly borrowed field must still be destroyed
    // before process-global replay can begin.
    foundation_lease: Cell<Option<crate::core::foundation::OrdinaryWorldLease>>,
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
    pub(crate) fn new(
        id: ffi::b2WorldId,
        brand: IdBrand,
        foundation_lease: crate::core::foundation::OrdinaryWorldLease,
        friction_mixer_present: bool,
        restitution_mixer_present: bool,
    ) -> Rc<Self> {
        let identities = crate::core::identity_registry::ActiveIdentityRegistry::new(brand);
        Self::new_with_identities(
            id,
            brand,
            foundation_lease,
            friction_mixer_present,
            restitution_mixer_present,
            identities,
        )
    }

    pub(crate) fn new_from_snapshot(
        id: ffi::b2WorldId,
        brand: IdBrand,
        foundation_lease: crate::core::foundation::OrdinaryWorldLease,
        entries: &[boxdd_sys::adapter::SnapshotEntry],
    ) -> ApiResult<Rc<Self>> {
        let identities =
            crate::core::identity_registry::ActiveIdentityRegistry::from_snapshot_entries(
                brand, entries,
            )?;
        Ok(Self::new_with_identities(
            id,
            brand,
            foundation_lease,
            false,
            false,
            identities,
        ))
    }

    fn new_with_identities(
        id: ffi::b2WorldId,
        brand: IdBrand,
        foundation_lease: crate::core::foundation::OrdinaryWorldLease,
        friction_mixer_present: bool,
        restitution_mixer_present: bool,
        identities: Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    ) -> Rc<Self> {
        #[cfg(target_arch = "wasm32")]
        let _ = (friction_mixer_present, restitution_mixer_present);
        Rc::new_cyclic(|self_weak| Self {
            self_weak: self_weak.clone(),
            id,
            brand,
            lifecycle: Cell::new(LifecycleState::Live),
            activity: Cell::new(ActivityState::Idle),
            native_calls: Cell::new(0),
            #[cfg(test)]
            native_object_checks: Cell::new(0),
            user_data_accesses: Cell::new(0),
            shutdown_requested: Cell::new(false),
            native_destroyed: Cell::new(false),
            contact_epoch: Cell::new(ContactEpoch::INITIAL),
            identities: Arc::clone(&identities),
            #[cfg(not(target_arch = "wasm32"))]
            custom_filter: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            pre_solve: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            material_mix_slot: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            friction_mix: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            restitution_mix: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            friction_mixer_present: Cell::new(friction_mixer_present),
            #[cfg(not(target_arch = "wasm32"))]
            restitution_mixer_present: Cell::new(restitution_mixer_present),
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks: WorkerCallbackState::new(brand, Arc::clone(&identities)),
            deferred_destroys: Mutex::new(VecDeque::new()),
            user_data: RefCell::new(crate::core::user_data::UserDataStore::default()),
            borrowed_event_buffers: AtomicUsize::new(0),
            owned_bodies: AtomicUsize::new(0),
            owned_shapes: AtomicUsize::new(0),
            owned_joints: AtomicUsize::new(0),
            owned_chains: AtomicUsize::new(0),
            foundation_lease: Cell::new(Some(foundation_lease)),
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
                WorldCore::poison(self);
                return Err(error);
            }
        };
        self.contact_epoch.set(next);
        Ok(next)
    }

    pub(crate) fn prepare_contact_epoch(&self) -> ApiResult<ContactEpoch> {
        self.contact_epoch.get().checked_next()
    }

    pub(crate) fn commit_contact_epoch(&self, next: ContactEpoch) -> ApiResult<()> {
        if self.contact_epoch.get().checked_next()? != next {
            return Err(ApiError::WorldBusy);
        }
        self.contact_epoch.set(next);
        Ok(())
    }

    #[inline]
    pub(crate) fn check_available(&self) -> ApiResult<()> {
        self.check_access(WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_access(&self, access: WorldAccess) -> ApiResult<()> {
        match self.lifecycle.get() {
            LifecycleState::Live => {}
            LifecycleState::Poisoned => return Err(ApiError::WorldPoisoned),
            LifecycleState::Destroyed => return Err(ApiError::WorldDestroyed),
        }
        if self.activity.get() == access.activity() {
            Ok(())
        } else {
            Err(ApiError::WorldBusy)
        }
    }

    #[cfg(test)]
    pub(crate) fn native_object_check_count_for_test(&self) -> usize {
        self.native_object_checks.get()
    }

    #[inline]
    fn record_native_object_check(&self) {
        #[cfg(test)]
        self.native_object_checks.set(
            self.native_object_checks
                .get()
                .checked_add(1)
                .expect("native object check counter overflow"),
        );
    }

    /// Authorize an operation owned by the active recording session.
    ///
    /// Ordinary world and handle entries continue to use `check_available`, so
    /// this does not make the recording activity visible through existing
    /// aliases. Public callers must perform the callback gate before entering
    /// this activity check.
    pub(crate) fn check_recording_available(&self) -> ApiResult<()> {
        self.check_access(WorldAccess::Recording)
    }

    /// Release the recording activity after the native world has been stopped.
    ///
    /// This deliberately remains available for a poisoned world: recording
    /// teardown must not leave the orthogonal activity state stuck when a
    /// callback panic or another terminal error occurred during the session.
    pub(crate) fn finish_recording_activity(&self) -> ApiResult<()> {
        if self.activity.get() != ActivityState::Recording {
            return Err(ApiError::WorldBusy);
        }
        self.activity.set(ActivityState::Idle);
        Ok(())
    }

    pub(crate) fn finish_restore_activity(&self) -> ApiResult<()> {
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        self.activity.set(ActivityState::Idle);
        Ok(())
    }

    pub(crate) fn check_snapshot_preconditions(&self) -> ApiResult<()> {
        self.check_available()?;
        if self.native_calls.get() != 0
            || self.user_data_accesses.get() != 0
            || self.borrowed_event_buffers.load(Ordering::Acquire) != 0
            || !self
                .deferred_destroys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        {
            return Err(ApiError::WorldBusy);
        }
        Ok(())
    }

    pub(crate) fn snapshot_callbacks_satisfy(
        &self,
        requires_custom_filter: bool,
        requires_pre_solve: bool,
    ) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            (!requires_custom_filter
                || self
                    .custom_filter
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .is_some())
                && (!requires_pre_solve
                    || self
                        .pre_solve
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .is_some())
        }
        #[cfg(target_arch = "wasm32")]
        {
            !requires_custom_filter && !requires_pre_solve
        }
    }

    pub(crate) fn snapshot_callback_presence(&self) -> (bool, bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            (
                self.custom_filter
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .is_some(),
                self.pre_solve
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .is_some(),
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            (false, false)
        }
    }

    #[inline]
    pub(crate) fn mixer_presence(&self) -> (bool, bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            (
                self.friction_mixer_present.get(),
                self.restitution_mixer_present.get(),
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            (false, false)
        }
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_friction_mixer_present(&self, present: bool) {
        self.friction_mixer_present.set(present);
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_restitution_mixer_present(&self, present: bool) {
        self.restitution_mixer_present.set(present);
    }

    pub(crate) fn begin_native_call_with_access(
        self: &Rc<Self>,
        access: WorldAccess,
    ) -> ApiResult<NativeCallGuard> {
        self.check_access(access)?;
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

    fn begin_user_data_access(&self) -> ApiResult<UserDataAccessGuard<'_>> {
        self.begin_user_data_access_with(WorldAccess::Idle)
    }

    fn begin_user_data_access_with(
        &self,
        access: WorldAccess,
    ) -> ApiResult<UserDataAccessGuard<'_>> {
        self.check_access(access)?;
        let depth = self
            .user_data_accesses
            .get()
            .checked_add(1)
            .expect("user-data access depth overflow");
        self.user_data_accesses.set(depth);
        Ok(UserDataAccessGuard { core: self })
    }

    #[inline]
    pub(crate) fn lifecycle(&self) -> LifecycleState {
        self.lifecycle.get()
    }

    #[inline]
    pub(crate) fn activity(&self) -> ActivityState {
        self.activity.get()
    }

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
        if std::cell::Cell::get(&self.lifecycle) == LifecycleState::Live {
            std::cell::Cell::set(&self.lifecycle, LifecycleState::Poisoned);
        }
    }

    #[inline]
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
        self.check_brand_with_access(brand, WorldAccess::Idle)
    }

    #[inline]
    fn check_brand_with_access(&self, brand: IdBrand, access: WorldAccess) -> ApiResult<()> {
        self.check_access(access)?;
        self.check_brand_identity(brand)
    }

    #[inline]
    pub(crate) fn check_body_identity(&self, id: BodyId) -> ApiResult<()> {
        self.check_body_identity_with_access(id, WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_body_identity_with_access(
        &self,
        id: BodyId,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_brand_with_access(id.brand(), access)?;
        if !self.identities.contains_body(id) {
            return Err(ApiError::InvalidBodyId);
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn check_body_native_after_identity(&self, id: BodyId) -> ApiResult<()> {
        self.record_native_object_check();
        if unsafe { ffi::b2Body_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidBodyId)
        }
    }

    #[inline]
    pub(crate) fn check_body(&self, id: BodyId) -> ApiResult<()> {
        self.check_body_with_access(id, WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_body_with_access(&self, id: BodyId, access: WorldAccess) -> ApiResult<()> {
        self.check_body_identity_with_access(id, access)?;
        self.check_body_native_after_identity(id)
    }

    #[inline]
    fn check_shape_native(&self, id: ShapeId) -> ApiResult<()> {
        if !self.identities.contains_shape(id) {
            return Err(ApiError::InvalidShapeId);
        }
        self.record_native_object_check();
        if unsafe { ffi::b2Shape_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidShapeId)
        }
    }

    #[inline]
    pub(crate) fn check_shape(&self, id: ShapeId) -> ApiResult<()> {
        self.check_shape_with_access(id, WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_shape_with_access(
        &self,
        id: ShapeId,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_brand_with_access(id.brand(), access)?;
        self.check_shape_native(id)
    }

    #[inline]
    pub(crate) fn check_joint_identity(&self, id: JointId) -> ApiResult<JointType> {
        self.check_joint_identity_with_access(id, WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_joint_identity_with_access(
        &self,
        id: JointId,
        access: WorldAccess,
    ) -> ApiResult<JointType> {
        self.check_brand_with_access(id.brand(), access)?;
        self.identities.joint_type(id)
    }

    #[inline]
    pub(crate) fn check_joint_native_after_identity(&self, id: JointId) -> ApiResult<()> {
        self.record_native_object_check();
        if unsafe { ffi::b2Joint_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidJointId)
        }
    }

    #[inline]
    pub(crate) fn check_joint(&self, id: JointId) -> ApiResult<()> {
        self.check_joint_with_access(id, WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_joint_with_access(
        &self,
        id: JointId,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_joint_identity_with_access(id, access)?;
        self.check_joint_native_after_identity(id)
    }

    #[inline]
    fn check_chain_native(&self, id: ChainId) -> ApiResult<()> {
        if !self.identities.contains_chain(id) {
            return Err(ApiError::InvalidChainId);
        }
        self.record_native_object_check();
        if unsafe { ffi::b2Chain_IsValid(id.into_raw()) } {
            Ok(())
        } else {
            Err(ApiError::InvalidChainId)
        }
    }

    #[inline]
    pub(crate) fn check_chain(&self, id: ChainId) -> ApiResult<()> {
        self.check_chain_with_access(id, WorldAccess::Idle)
    }

    #[inline]
    pub(crate) fn check_chain_with_access(
        &self,
        id: ChainId,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_brand_with_access(id.brand(), access)?;
        self.check_chain_native(id)
    }

    pub(crate) fn body_is_valid(&self, id: BodyId) -> ApiResult<bool> {
        self.check_brand(id.brand())?;
        if !self.identities.contains_body(id) {
            return Ok(false);
        }
        self.record_native_object_check();
        Ok(unsafe { ffi::b2Body_IsValid(id.into_raw()) })
    }

    pub(crate) fn shape_is_valid(&self, id: ShapeId) -> ApiResult<bool> {
        self.check_brand(id.brand())?;
        if !self.identities.contains_shape(id) {
            return Ok(false);
        }
        self.record_native_object_check();
        Ok(unsafe { ffi::b2Shape_IsValid(id.into_raw()) })
    }

    pub(crate) fn joint_is_valid(&self, id: JointId) -> ApiResult<bool> {
        self.check_brand(id.brand())?;
        if !self.identities.contains_joint(id) {
            return Ok(false);
        }
        self.record_native_object_check();
        Ok(unsafe { ffi::b2Joint_IsValid(id.into_raw()) })
    }

    pub(crate) fn chain_is_valid(&self, id: ChainId) -> ApiResult<bool> {
        self.check_brand(id.brand())?;
        if !self.identities.contains_chain(id) {
            return Ok(false);
        }
        self.record_native_object_check();
        Ok(unsafe { ffi::b2Chain_IsValid(id.into_raw()) })
    }

    pub(crate) fn identity_manifest(
        &self,
    ) -> ApiResult<crate::core::identity_registry::IdentityManifest> {
        self.check_available()?;
        self.identities.snapshot_manifest()
    }

    pub(crate) fn identity_manifest_while_restoring(
        &self,
    ) -> ApiResult<crate::core::identity_registry::IdentityManifest> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        self.identities.snapshot_manifest()
    }

    pub(crate) fn prepare_identity_restore(
        &self,
        manifest: &crate::core::identity_registry::IdentityManifest,
    ) -> ApiResult<crate::core::identity_registry::PreparedIdentityRestore> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        self.identities.prepare_restore(manifest)
    }

    pub(crate) fn commit_identity_restore(
        &self,
        prepared: crate::core::identity_registry::PreparedIdentityRestore,
    ) -> ApiResult<()> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        self.identities.commit_restore(prepared)
    }

    pub(crate) fn user_data_manifest_while_restoring(
        &self,
    ) -> ApiResult<crate::core::user_data::UserDataManifest> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        self.user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .snapshot_manifest()
    }

    pub(crate) fn prepare_user_data_restore(
        &self,
        manifest: &crate::core::user_data::UserDataManifest,
        identity_manifest: &crate::core::identity_registry::IdentityManifest,
        identities: &crate::core::identity_registry::PreparedIdentityRestore,
    ) -> ApiResult<crate::core::user_data::PreparedUserDataRestore> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        self.user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .prepare_restore(manifest, identity_manifest, identities)
    }

    pub(crate) fn commit_user_data_restore(
        &self,
        prepared: crate::core::user_data::PreparedUserDataRestore,
    ) -> ApiResult<crate::core::user_data::CommittedUserDataRestore> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(ApiError::WorldBusy);
        }
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        prepared.commit(&mut store)
    }

    pub(crate) fn clear_retired_identity_outputs(&self) {
        crate::core::identity_registry::ActiveIdentityRegistry::clear_retired_outputs(
            &self.identities,
        );
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
        let id = self.brand.body(raw.into_ffi(), raw.registration_nonce());
        self.check_body_identity(id)?;
        self.check_body_native_after_identity(id)?;
        Ok(id)
    }

    pub(crate) fn bind_shape(&self, raw: RawShapeId) -> ApiResult<ShapeId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.shape(raw.into_ffi(), raw.registration_nonce());
        self.check_shape_native(id)?;
        Ok(id)
    }

    pub(crate) fn bind_joint(&self, raw: RawJointId) -> ApiResult<JointId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.joint(raw.into_ffi(), raw.registration_nonce());
        self.check_joint_identity(id)?;
        self.check_joint_native_after_identity(id)?;
        Ok(id)
    }

    pub(crate) fn bind_chain(&self, raw: RawChainId) -> ApiResult<ChainId> {
        self.check_available()?;
        raw.validate_for(self.brand)?;
        let id = self.brand.chain(raw.into_ffi(), raw.registration_nonce());
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

    pub(crate) fn finish_created_body_with_access(
        &self,
        raw: ffi::b2BodyId,
        access: WorldAccess,
    ) -> ApiResult<BodyId> {
        let result = (|| {
            self.check_access(access)?;
            self.brand.check_body_raw(raw)?;
            self.record_native_object_check();
            if !unsafe { ffi::b2Body_IsValid(raw) } {
                return Err(ApiError::InvalidBodyId);
            }
            self.identities.register_body(raw)
        })();
        self.poison_created_output_error(result)
    }

    pub(crate) fn finish_created_shape_with_access(
        &self,
        raw: ffi::b2ShapeId,
        access: WorldAccess,
    ) -> ApiResult<ShapeId> {
        let result = (|| {
            self.check_access(access)?;
            self.brand.check_shape_raw(raw)?;
            self.record_native_object_check();
            if !unsafe { ffi::b2Shape_IsValid(raw) } {
                return Err(ApiError::InvalidShapeId);
            }
            let body = self
                .identities
                .resolve_body(unsafe { ffi::b2Shape_GetBody(raw) })?;
            self.identities.register_shape(raw, body)
        })();
        self.poison_created_output_error(result)
    }

    #[cfg(test)]
    pub(crate) fn finish_created_joint(
        &self,
        raw: ffi::b2JointId,
        body_a: BodyId,
        body_b: BodyId,
        kind: JointType,
    ) -> ApiResult<JointId> {
        self.finish_created_joint_with_access(raw, body_a, body_b, kind, WorldAccess::Idle)
    }

    pub(crate) fn finish_created_joint_with_access(
        &self,
        raw: ffi::b2JointId,
        body_a: BodyId,
        body_b: BodyId,
        kind: JointType,
        access: WorldAccess,
    ) -> ApiResult<JointId> {
        let result = (|| {
            self.check_access(access)?;
            self.brand.check_joint_raw(raw)?;
            self.record_native_object_check();
            if !unsafe { ffi::b2Joint_IsValid(raw) } {
                return Err(ApiError::InvalidJointId);
            }
            self.identities.register_joint(raw, body_a, body_b, kind)
        })();
        self.poison_created_output_error(result)
    }

    #[cfg(test)]
    pub(crate) fn finish_created_chain(&self, raw: ffi::b2ChainId) -> ApiResult<ChainId> {
        self.finish_created_chain_with_access(raw, WorldAccess::Idle)
    }

    pub(crate) fn finish_created_chain_with_access(
        &self,
        raw: ffi::b2ChainId,
        access: WorldAccess,
    ) -> ApiResult<ChainId> {
        let result = (|| {
            self.check_access(access)?;
            self.brand.check_chain_raw(raw)?;
            self.record_native_object_check();
            if !unsafe { ffi::b2Chain_IsValid(raw) } {
                return Err(ApiError::InvalidChainId);
            }
            let count = unsafe { ffi::b2Chain_GetSegmentCount(raw) };
            let segments: Vec<ffi::b2ShapeId> = unsafe {
                crate::core::ffi_vec::try_read_mapped_from_ffi(
                    count,
                    |out, capacity| ffi::b2Chain_GetSegments(raw, out, capacity),
                    Ok,
                )
            }?;
            let first = segments.first().copied().ok_or(ApiError::InvalidChainId)?;
            for &segment in &segments {
                self.brand.check_shape_raw(segment)?;
                if !unsafe { ffi::b2Shape_IsValid(segment) } {
                    return Err(ApiError::InvalidShapeId);
                }
            }
            let body = self
                .identities
                .resolve_body(unsafe { ffi::b2Shape_GetBody(first) })?;
            let (id, _) = self.identities.register_chain(raw, body, &segments)?;
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
            .push_back(d);
        if let Some(core) = self.self_weak.upgrade() {
            let _ = crate::core::callback_state::register_deferred_core(core);
        }
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
                && let Some(entry) = store.bodies.get(&body)
            {
                entries.push(Rc::clone(entry));
            }
            entries.extend(shapes.iter().filter_map(|id| store.shapes.get(id).cloned()));
            entries.extend(joints.iter().filter_map(|id| store.joints.get(id).cloned()));
            entries
        };

        for entry in entries {
            entry.check_mutable()?;
        }
        Ok(())
    }

    fn check_object_destroy_preconditions_with(
        &self,
        brand: IdBrand,
        access: WorldAccess,
    ) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.check_access(access)?;
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

    fn drop_retired_user_data(retired: Vec<crate::core::user_data::ErasedUserData>) {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        for value in retired {
            panic.run_cleanup(|| drop(value));
        }
        panic.resume_or_forget();
    }

    pub(crate) fn destroy_body_now(&self, id: BodyId) -> ApiResult<()> {
        self.destroy_body_now_with_access(id, WorldAccess::Idle)
    }

    pub(crate) fn destroy_body_now_with_access(
        &self,
        id: BodyId,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_object_destroy_preconditions_with(id.brand(), access)?;
        self.check_body_identity_with_access(id, access)?;
        self.check_body_native_after_identity(id)?;
        let shapes = self.body_shapes_for_destroy(id)?;
        let joints = self.body_joints_for_destroy(id)?;
        self.check_user_data_mutable(Some(id), &shapes, &joints)?;
        let _user_data_cleanup = self.begin_user_data_access_with(access)?;
        unsafe { ffi::b2DestroyBody(id.into_raw()) };
        let unregistered = self.identities.unregister_body(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(Some(id), &shapes, &joints));
        Ok(())
    }

    pub(crate) fn destroy_shape_now(&self, id: ShapeId, update_body_mass: bool) -> ApiResult<()> {
        self.destroy_shape_now_with_access(id, update_body_mass, WorldAccess::Idle)
    }

    pub(crate) fn destroy_shape_now_with_access(
        &self,
        id: ShapeId,
        update_body_mass: bool,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_object_destroy_preconditions_with(id.brand(), access)?;
        self.check_shape_native(id)?;
        if unsafe { ffi::b2Shape_GetParentChain(id.into_raw()) }.index1 != 0 {
            return Err(ApiError::ChainOwnedShape);
        }
        self.check_user_data_mutable(None, core::slice::from_ref(&id), &[])?;
        let _user_data_cleanup = self.begin_user_data_access_with(access)?;
        unsafe { ffi::b2DestroyShape(id.into_raw(), update_body_mass) };
        let unregistered = self.identities.unregister_shape(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(None, core::slice::from_ref(&id), &[]));
        Ok(())
    }

    pub(crate) fn destroy_joint_now(&self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        self.destroy_joint_now_with_access(id, wake_bodies, WorldAccess::Idle)
    }

    pub(crate) fn destroy_joint_now_with_access(
        &self,
        id: JointId,
        wake_bodies: bool,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_object_destroy_preconditions_with(id.brand(), access)?;
        self.check_joint_identity_with_access(id, access)?;
        self.check_joint_native_after_identity(id)?;
        self.check_user_data_mutable(None, &[], core::slice::from_ref(&id))?;
        let _user_data_cleanup = self.begin_user_data_access_with(access)?;
        unsafe { ffi::b2DestroyJoint(id.into_raw(), wake_bodies) };
        let unregistered = self.identities.unregister_joint(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(None, &[], core::slice::from_ref(&id)));
        Ok(())
    }

    pub(crate) fn destroy_chain_now(&self, id: ChainId) -> ApiResult<()> {
        self.destroy_chain_now_with_access(id, WorldAccess::Idle)
    }

    pub(crate) fn destroy_chain_now_with_access(
        &self,
        id: ChainId,
        access: WorldAccess,
    ) -> ApiResult<()> {
        self.check_object_destroy_preconditions_with(id.brand(), access)?;
        self.check_chain_with_access(id, access)?;
        let shapes = self.chain_shapes_for_destroy(id)?;
        self.check_user_data_mutable(None, &shapes, &[])?;
        let _user_data_cleanup = self.begin_user_data_access_with(access)?;
        unsafe { ffi::b2DestroyChain(id.into_raw()) };
        let unregistered = self.identities.unregister_chain(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(None, &shapes, &[]));
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
        if self.shutdown_requested.get() && !self.native_destroyed.get() {
            if !crate::core::callback_state::in_callback()
                && self.native_calls.get() == 0
                && self.user_data_accesses.get() == 0
            {
                self.finish_native_shutdown();
            }
            return;
        }
        if self.owned_destroy_gate() != OwnedDestroyGate::Ready {
            return;
        }
        let pending_count = self
            .deferred_destroys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        let mut panic = crate::core::callback_state::PanicSlot::default();

        for _ in 0..pending_count {
            if self.owned_destroy_gate() != OwnedDestroyGate::Ready {
                break;
            }
            let item = {
                let mut pending = self
                    .deferred_destroys
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Some(item) = pending.pop_front() else {
                    break;
                };
                item
            };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.destroy_deferred_now(item)
            })) {
                Ok(Ok(())) => {}
                Ok(Err(error)) if item.is_stale_error(error) => {}
                Ok(Err(_)) => self.defer_destroy(item),
                Err(payload) => panic.capture(payload),
            }
        }

        // This cleanup can be reached from a destructor after an outer panic has already begun.
        // Never start a second unwind or drop a later arbitrary payload on that stack.
        panic.resume_or_forget();
    }

    /// End the native world's lifetime while retaining an inert Rust shell for residual handles.
    pub(crate) fn shutdown_native(&self) {
        if self.native_destroyed.get() {
            return;
        }
        self.lifecycle.set(LifecycleState::Destroyed);
        self.activity.set(ActivityState::Idle);
        self.shutdown_requested.set(true);
        if crate::core::callback_state::in_callback() {
            if let Some(core) = self.self_weak.upgrade()
                && let Err(core) = crate::core::callback_state::register_deferred_core(core)
            {
                // Worker/process callbacks have no owner-thread drain boundary. Keep the inert
                // core, native world, callback state, and foundation lease alive permanently
                // instead of re-entering Box2D or allowing replay over leaked native state.
                core::mem::forget(core);
            }
            return;
        }
        if self.native_calls.get() == 0 && self.user_data_accesses.get() == 0 {
            self.finish_native_shutdown();
        }
    }

    fn finish_native_shutdown(&self) {
        if self.native_destroyed.replace(true) {
            return;
        }
        debug_assert!(self.shutdown_requested.get());
        debug_assert_eq!(self.native_calls.get(), 0);
        debug_assert_eq!(self.user_data_accesses.get(), 0);

        {
            let _world_slot_guard = crate::core::foundation::lock_world_slot_mutation();
            // SAFETY: `World` owns the native lifetime and this method transitions the shared
            // lifecycle to `Destroyed` before making the one idempotent teardown call.
            unsafe { ffi::b2DestroyWorld(self.id) };
        }
        // Native destruction has joined the scheduler. From this point no safe id, including one
        // held by a surviving worker callback context, may resolve to a live registration.
        self.identities.clear_and_uninstall();
        // Keep this local alive until every callback owner and arbitrary user-data payload below
        // has been released. Rust also drops it last if one of those destructors panics.
        let mut foundation_lease = self.foundation_lease.take();
        let mut panic = crate::core::callback_state::PanicSlot::default();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(slot) = self
                .material_mix_slot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
            {
                crate::core::material_mix_registry::set_friction_ptr(slot, core::ptr::null_mut());
                crate::core::material_mix_registry::set_restitution_ptr(
                    slot,
                    core::ptr::null_mut(),
                );
                crate::core::material_mix_registry::release_slot(slot);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let custom_filter = self
            .custom_filter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        #[cfg(not(target_arch = "wasm32"))]
        let pre_solve = self
            .pre_solve
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        #[cfg(not(target_arch = "wasm32"))]
        let friction_mix = self
            .friction_mix
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        #[cfg(not(target_arch = "wasm32"))]
        let restitution_mix = self
            .restitution_mix
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();

        #[cfg(not(target_arch = "wasm32"))]
        let worker_panic = self.worker_callbacks.take_panic();
        self.deferred_destroys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        // Draining the entries severs core -> user data -> handle -> core cycles. Each arbitrary
        // payload is dropped behind its own panic boundary after native teardown.
        let user_data_entries = self
            .user_data
            .try_borrow_mut()
            .ok()
            .map(|mut store| store.drain_entries());
        if user_data_entries.is_none() {
            // Retaining the lease is conservative but necessary: the still-borrowed store may own
            // arbitrary payloads, and replay must not begin before those payloads are releasable.
            self.foundation_lease.set(foundation_lease.take());
            panic.capture(Box::new(
                "world user data remained borrowed during native shutdown",
            ));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            panic.run_cleanup(|| drop(custom_filter));
            panic.run_cleanup(|| drop(pre_solve));
            panic.run_cleanup(|| drop(friction_mix));
            panic.run_cleanup(|| drop(restitution_mix));
            if let Some(payload) = worker_panic {
                panic.run_cleanup(|| drop(payload));
            }
        }
        if let Some(entries) = user_data_entries {
            for entry in entries {
                panic.run_cleanup(|| {
                    let value = entry
                        .take_erased()
                        .expect("world user data cannot remain borrowed during native shutdown");
                    drop(value);
                });
            }
        }
        drop(foundation_lease);
        panic.resume_or_forget();
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
        store.mark_changed();
        Ok(retired)
    }

    pub(crate) fn clear_body_user_data(
        &self,
        id: BodyId,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.bodies.get(&id).cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.bodies.remove(&id);
        store.mark_changed();
        Ok(retired)
    }

    pub(crate) fn clear_shape_user_data(
        &self,
        id: ShapeId,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.shapes.get(&id).cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.shapes.remove(&id);
        store.mark_changed();
        Ok(retired)
    }

    pub(crate) fn clear_joint_user_data(
        &self,
        id: JointId,
    ) -> ApiResult<Option<crate::core::user_data::ErasedUserData>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.joints.get(&id).cloned() else {
            return Ok(None);
        };
        let retired = entry.take_erased()?;
        store.joints.remove(&id);
        store.mark_changed();
        Ok(retired)
    }

    pub(crate) fn set_world_user_data<T: 'static>(
        &self,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.world.clone();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value, version);
        store.world = Some(entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_body_user_data<T: 'static>(
        &self,
        id: BodyId,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.bodies.get(&id).cloned();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        store
            .bodies
            .try_reserve(1)
            .map_err(|_| ApiError::UserDataAllocationFailed)?;
        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value, version);
        store.bodies.insert(id, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.shapes.get(&id).cloned();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        store
            .shapes
            .try_reserve(1)
            .map_err(|_| ApiError::UserDataAllocationFailed)?;
        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value, version);
        store.shapes.insert(id, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_joint_user_data<T: 'static>(
        &self,
        id: JointId,
        value: T,
    ) -> ApiResult<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.joints.get(&id).cloned();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        store
            .joints
            .try_reserve(1)
            .map_err(|_| ApiError::UserDataAllocationFailed)?;
        let (entry, pointer) = crate::core::user_data::UserDataEntry::new(value, version);
        store.joints.insert(id, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn try_with_world_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        let _access = self.begin_user_data_access()?;
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
        let _access = self.begin_user_data_access()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .get(&id)
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
        let _access = self.begin_user_data_access()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .get(&id)
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
        let _access = self.begin_user_data_access()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .get(&id)
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
        let _access = self.begin_user_data_access()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .bodies
            .get(&id)
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
        let _access = self.begin_user_data_access()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .shapes
            .get(&id)
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
        let _access = self.begin_user_data_access()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| ApiError::ReentrantAccess)?
            .joints
            .get(&id)
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
        store.mark_changed();
        Ok(value)
    }

    pub(crate) fn take_body_user_data<T: 'static>(
        &self,
        id: BodyId,
    ) -> crate::error::ApiResult<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.bodies.get(&id).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.bodies.remove(&id);
        store.mark_changed();
        Ok(value)
    }

    pub(crate) fn take_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
    ) -> crate::error::ApiResult<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.shapes.get(&id).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.shapes.remove(&id);
        store.mark_changed();
        Ok(value)
    }

    pub(crate) fn take_joint_user_data<T: 'static>(
        &self,
        id: JointId,
    ) -> crate::error::ApiResult<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| ApiError::ReentrantAccess)?;
        let Some(entry) = store.joints.get(&id).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.joints.remove(&id);
        store.mark_changed();
        Ok(value)
    }
}

pub(crate) struct BorrowedEventBuffersGuard {
    core: Rc<WorldCore>,
}

pub(crate) struct NativeCallGuard {
    core: Rc<WorldCore>,
}

struct UserDataAccessGuard<'a> {
    core: &'a WorldCore,
}

impl Drop for NativeCallGuard {
    fn drop(&mut self) {
        let depth = self.core.native_calls.get();
        debug_assert!(depth > 0, "native call counter underflow");
        self.core.native_calls.set(depth.saturating_sub(1));
        if depth == 1
            && self.core.shutdown_requested.get()
            && self.core.user_data_accesses.get() == 0
        {
            self.core.finish_native_shutdown();
        }
    }
}

impl Drop for UserDataAccessGuard<'_> {
    fn drop(&mut self) {
        let depth = self.core.user_data_accesses.get();
        debug_assert!(depth > 0, "user-data access counter underflow");
        self.core.user_data_accesses.set(depth.saturating_sub(1));
        if depth == 1 && self.core.shutdown_requested.get() && self.core.native_calls.get() == 0 {
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
    use super::{LifecycleState, WorldAccess};
    use crate::id::ContactEpoch;
    use crate::shapes::{Circle, ShapeDef, circle};
    use crate::{ApiError, BodyBuilder, BodyId, BodyType, ShapeId, World, WorldDef};
    use boxdd_sys::ffi;

    fn create_raw_circle(body: BodyId, def: &ShapeDef, circle: Circle) -> ffi::b2ShapeId {
        let def = def.clone().into_raw();
        let circle = circle.into_raw();
        unsafe { ffi::b2CreateCircleShape(body.into_raw(), &def, &circle) }
    }

    fn raw_shape_eq(left: ffi::b2ShapeId, right: ffi::b2ShapeId) -> bool {
        left.index1 == right.index1
            && left.world0 == right.world0
            && left.generation == right.generation
    }

    fn destroy_registered_shape(world: &World, shape: ShapeId) {
        unsafe { ffi::b2DestroyShape(shape.into_raw(), true) };
        assert!(world.core().identities.unregister_shape(shape));
    }

    fn expose_pending_native_end_events(world: &World) {
        // This test-only raw step deliberately leaves the Rust retired-output table intact so the
        // native end event and its host identity can be inspected in the vulnerable reuse window.
        unsafe { ffi::b2World_Step(world.raw(), 1.0 / 60.0, 4) };
    }

    fn recycle_raw_shape_slot_until_retired_key_conflicts(
        world: &World,
        body: BodyId,
        def: &ShapeDef,
        circle: Circle,
        retired: ShapeId,
    ) -> ffi::b2ShapeId {
        let retired_raw = retired.into_raw();
        for offset in 1..=u16::MAX {
            let expected_generation = retired_raw.generation.wrapping_add(offset);
            let raw = create_raw_circle(body, def, circle);
            assert_eq!(raw.index1, retired_raw.index1);
            assert_eq!(raw.generation, expected_generation);
            let active = world
                .core()
                .finish_created_shape_with_access(raw, WorldAccess::Idle)
                .unwrap();
            destroy_registered_shape(world, active);
        }

        let raw = create_raw_circle(body, def, circle);
        assert!(raw_shape_eq(raw, retired_raw));
        raw
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

        assert_eq!(
            core.finish_created_body_with_access(body.into_raw(), WorldAccess::Idle),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
    }

    #[test]
    fn retained_contact_end_shape_key_rejects_u16_generation_wrap_and_poisons_world() {
        let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
        let static_body = world.create_body_id(BodyBuilder::new().build());
        let dynamic_body = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .gravity_scale(0.0)
                .build(),
        );
        let contact_def = ShapeDef::builder().enable_contact_events(true).build();
        let circle = circle([0.0_f32, 0.0], 0.5);
        let static_shape = world.create_circle_shape_for(static_body, &contact_def, &circle);
        let retired = world.create_circle_shape_for(dynamic_body, &contact_def, &circle);

        world.step(1.0 / 60.0, 4);
        assert!(world.contact_events().begin.iter().any(|event| {
            (event.shape_a == static_shape && event.shape_b == retired)
                || (event.shape_a == retired && event.shape_b == static_shape)
        }));

        destroy_registered_shape(&world, retired);
        expose_pending_native_end_events(&world);
        let end_raw = unsafe { ffi::b2World_GetContactEvents(world.raw()) };
        assert!(end_raw.endCount > 0);
        assert!(!end_raw.endEvents.is_null());
        let end =
            unsafe { core::slice::from_raw_parts(end_raw.endEvents, end_raw.endCount as usize) };
        assert!(end.iter().any(|event| {
            raw_shape_eq(event.shapeIdA, retired.into_raw())
                || raw_shape_eq(event.shapeIdB, retired.into_raw())
        }));
        assert_eq!(world.brand().try_shape(retired.into_raw()), Ok(retired));

        let conflicting_raw = recycle_raw_shape_slot_until_retired_key_conflicts(
            &world,
            dynamic_body,
            &contact_def,
            circle,
            retired,
        );
        assert_eq!(
            world
                .core()
                .finish_created_shape_with_access(conflicting_raw, WorldAccess::Idle),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
    }

    #[test]
    fn retained_sensor_end_shape_key_rejects_u16_generation_wrap_and_poisons_world() {
        let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
        let sensor_body = world.create_body_id(BodyBuilder::new().build());
        let visitor_body = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .gravity_scale(0.0)
                .build(),
        );
        let sensor_def = ShapeDef::builder()
            .sensor(true)
            .enable_sensor_events(true)
            .build();
        let visitor_def = ShapeDef::builder().enable_sensor_events(true).build();
        let circle = circle([0.0_f32, 0.0], 0.5);
        let sensor = world.create_circle_shape_for(sensor_body, &sensor_def, &circle);
        let retired = world.create_circle_shape_for(visitor_body, &visitor_def, &circle);

        world.step(1.0 / 60.0, 4);
        assert!(
            world
                .sensor_events()
                .begin
                .iter()
                .any(|event| { event.sensor_shape == sensor && event.visitor_shape == retired })
        );

        destroy_registered_shape(&world, retired);
        expose_pending_native_end_events(&world);
        let end_raw = unsafe { ffi::b2World_GetSensorEvents(world.raw()) };
        assert!(end_raw.endCount > 0);
        assert!(!end_raw.endEvents.is_null());
        let end =
            unsafe { core::slice::from_raw_parts(end_raw.endEvents, end_raw.endCount as usize) };
        assert!(end.iter().any(|event| {
            raw_shape_eq(event.sensorShapeId, sensor.into_raw())
                && raw_shape_eq(event.visitorShapeId, retired.into_raw())
        }));
        assert_eq!(world.brand().try_shape(retired.into_raw()), Ok(retired));

        let conflicting_raw = recycle_raw_shape_slot_until_retired_key_conflicts(
            &world,
            visitor_body,
            &visitor_def,
            circle,
            retired,
        );
        assert_eq!(
            world
                .core()
                .finish_created_shape_with_access(conflicting_raw, WorldAccess::Idle),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
    }

    #[test]
    fn invalid_native_creation_outputs_poison_without_registration() {
        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_body_with_access(
                ffi::b2BodyId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
                WorldAccess::Idle,
            ),
            Err(ApiError::InvalidBodyId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);

        let world = World::new(WorldDef::default()).unwrap();
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_shape_with_access(
                ffi::b2ShapeId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
                WorldAccess::Idle,
            ),
            Err(ApiError::InvalidShapeId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);

        let mut world = World::new(WorldDef::default()).unwrap();
        let body_a = world.create_body_id(crate::BodyBuilder::new().build());
        let body_b = world.create_body_id(crate::BodyBuilder::new().build());
        let core = world.core_rc();
        assert_eq!(
            core.finish_created_joint(
                ffi::b2JointId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
                body_a,
                body_b,
                crate::JointType::Distance,
            ),
            Err(ApiError::InvalidJointId)
        );
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);

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
        let call = core
            .begin_native_call_with_access(WorldAccess::Idle)
            .unwrap();

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

    use super::{ActivityState, LifecycleState, WorldCore, ffi};
    #[cfg(not(target_arch = "wasm32"))]
    use crate::core::callback_state::WorkerCallbackState;
    use crate::{ApiError, BodyBuilder, BodyType, ShapeDef, Vec2, World, WorldDef, shapes};

    #[cfg(not(target_arch = "wasm32"))]
    assert_impl_all!(WorkerCallbackState: Send, Sync);
    assert_not_impl_any!(WorldCore: Send, Sync);

    #[cfg(not(target_arch = "wasm32"))]
    static PANICKING_PAYLOAD_DROPS: AtomicUsize = AtomicUsize::new(0);

    #[cfg(not(target_arch = "wasm32"))]
    struct PanickingPayload;

    #[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(not(target_arch = "wasm32"))]
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
    fn world_drop_recovers_a_poisoned_world_slot_lock() {
        let world = World::new(WorldDef::default()).unwrap();
        let raw_world = world.raw();
        let core = world.core_rc();

        let poison = std::panic::catch_unwind(|| {
            let _guard = crate::core::foundation::lock_world_slot_mutation();
            panic!("poison the Box2D world-slot lock for teardown coverage");
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
