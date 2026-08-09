use boxdd_sys::ffi;
use std::cell::{Cell, RefCell};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{CustomFilterCb, PreSolveCb, WorkerCallbackState};
use crate::error::{Error, Result};
use crate::id::{ContactEpoch, IdBrand};
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

struct ExclusiveActivityLease {
    state: Rc<Cell<ActivityState>>,
    activity: ActivityState,
    armed: bool,
}

impl ExclusiveActivityLease {
    fn begin(core: &WorldCore, activity: ActivityState) -> Result<Self> {
        core.begin_activity(activity)?;
        Ok(Self {
            state: Rc::clone(&core.activity),
            activity,
            armed: true,
        })
    }

    fn finish(&mut self) -> Result<()> {
        if !self.armed {
            return Ok(());
        }
        release_activity(&self.state, self.activity)?;
        self.armed = false;
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ExclusiveActivityLease {
    fn drop(&mut self) {
        if self.armed && self.state.get() == self.activity {
            let _ = release_activity(&self.state, self.activity);
        }
    }
}

pub(crate) struct RecordingActivityLease(ExclusiveActivityLease);

impl RecordingActivityLease {
    pub(crate) fn finish(&mut self) -> Result<()> {
        self.0.finish()
    }
}

pub(crate) struct RestoreActivityLease(ExclusiveActivityLease);

impl RestoreActivityLease {
    pub(crate) fn finish(&mut self) -> Result<()> {
        self.0.finish()
    }

    pub(crate) fn disarm(&mut self) {
        self.0.disarm();
    }

    pub(crate) fn is_armed(&self) -> bool {
        self.0.armed
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct CallbackRegistrationGeneration(Arc<()>);

#[cfg(not(target_arch = "wasm32"))]
impl CallbackRegistrationGeneration {
    /// Allocate an identity token whose address cannot be reused while a snapshot retains it.
    fn fresh() -> Self {
        Self(Arc::new(()))
    }

    fn is_same_registration(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct CallbackRegistration<T> {
    context: Box<T>,
    generation: CallbackRegistrationGeneration,
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> CallbackRegistration<T> {
    pub(crate) fn new(context: Box<T>) -> Self {
        Self {
            context,
            generation: CallbackRegistrationGeneration::fresh(),
        }
    }

    pub(crate) fn context(&self) -> &T {
        &self.context
    }

    fn generation(&self) -> CallbackRegistrationGeneration {
        self.generation.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CallbackRegistrationGenerations {
    #[cfg(not(target_arch = "wasm32"))]
    custom_filter: Option<CallbackRegistrationGeneration>,
    #[cfg(not(target_arch = "wasm32"))]
    pre_solve: Option<CallbackRegistrationGeneration>,
}

impl CallbackRegistrationGenerations {
    pub(crate) fn matches(&self, other: &Self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            same_callback_registration(&self.custom_filter, &other.custom_filter)
                && same_callback_registration(&self.pre_solve, &other.pre_solve)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (self, other);
            true
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn same_callback_registration(
    left: &Option<CallbackRegistrationGeneration>,
    right: &Option<CallbackRegistrationGeneration>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.is_same_registration(right),
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

fn release_activity(state: &Cell<ActivityState>, expected: ActivityState) -> Result<()> {
    if expected == ActivityState::Idle || state.get() != expected {
        return Err(Error::WorldBusy);
    }
    state.set(ActivityState::Idle);
    Ok(())
}

fn same_body_raw(left: ffi::b2BodyId, right: ffi::b2BodyId) -> bool {
    left.index1 == right.index1
        && left.world0 == right.world0
        && left.generation == right.generation
}

fn same_chain_raw(left: ffi::b2ChainId, right: ffi::b2ChainId) -> bool {
    left.index1 == right.index1
        && left.world0 == right.world0
        && left.generation == right.generation
}

pub(crate) struct WorldCore {
    pub(crate) id: ffi::b2WorldId,
    pub(crate) brand: IdBrand,
    length_scale: crate::core::length_scale::LengthScale,
    lifecycle: Cell<LifecycleState>,
    // Activity leases share only this state cell. Native ownership remains exclusively in World.
    activity: Rc<Cell<ActivityState>>,
    #[cfg(test)]
    access_checks: Cell<usize>,
    #[cfg(test)]
    native_object_checks: Cell<usize>,
    #[cfg(test)]
    creation_compensations: Cell<usize>,
    #[cfg(test)]
    fail_next_creation_compensation: Cell<bool>,
    native_destroyed: Cell<bool>,
    contact_epoch: Cell<ContactEpoch>,
    identities: Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) custom_filter: Mutex<Option<CallbackRegistration<Arc<CustomFilterCb>>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pre_solve: Mutex<Option<CallbackRegistration<Arc<PreSolveCb>>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) material_mix: Mutex<crate::core::material_mix_registry::OwnedMaterialMixSlot>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) worker_callbacks: Arc<WorkerCallbackState>,
    pub(crate) user_data: RefCell<crate::core::user_data::UserDataStore>,
    _pin: PhantomPinned,
    // Keep this last as a fallback: Rust drops struct fields in declaration order. Normal teardown
    // takes the lease into a local, but an unexpectedly borrowed field must still be destroyed
    // before process-global replay can begin.
    foundation_lease: Cell<Option<crate::core::foundation::OrdinaryWorldLease>>,
}

#[derive(Copy, Clone)]
enum NativeCreation {
    Body(ffi::b2BodyId),
    Shape {
        raw: ffi::b2ShapeId,
        update_body_mass: bool,
    },
    Joint(ffi::b2JointId),
    Chain(ffi::b2ChainId),
}

/// RAII rollback for a native object which has not reached Rust identity publication.
#[must_use = "a native creation must be published or compensated"]
pub(crate) struct NativeCreationGuard<'core> {
    core: &'core WorldCore,
    native: NativeCreation,
    armed: bool,
}

impl NativeCreationGuard<'_> {
    pub(crate) fn commit(&mut self) {
        self.armed = false;
    }
}

impl Drop for NativeCreationGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.core.compensate_creation(self.native);
        }
    }
}

impl WorldCore {
    pub(crate) fn new(
        id: ffi::b2WorldId,
        brand: IdBrand,
        length_scale: crate::core::length_scale::LengthScale,
        foundation_lease: crate::core::foundation::OrdinaryWorldLease,
    ) -> Pin<Box<Self>> {
        let identities = crate::core::identity_registry::ActiveIdentityRegistry::new(brand);
        Self::new_with_identities(id, brand, length_scale, foundation_lease, identities)
    }

    fn new_with_identities(
        id: ffi::b2WorldId,
        brand: IdBrand,
        length_scale: crate::core::length_scale::LengthScale,
        foundation_lease: crate::core::foundation::OrdinaryWorldLease,
        identities: Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    ) -> Pin<Box<Self>> {
        Box::pin(Self {
            id,
            brand,
            length_scale,
            lifecycle: Cell::new(LifecycleState::Live),
            activity: Rc::new(Cell::new(ActivityState::Idle)),
            #[cfg(test)]
            access_checks: Cell::new(0),
            #[cfg(test)]
            native_object_checks: Cell::new(0),
            #[cfg(test)]
            creation_compensations: Cell::new(0),
            #[cfg(test)]
            fail_next_creation_compensation: Cell::new(false),
            native_destroyed: Cell::new(false),
            contact_epoch: Cell::new(ContactEpoch::INITIAL),
            identities,
            #[cfg(not(target_arch = "wasm32"))]
            custom_filter: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            pre_solve: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            material_mix: Mutex::new(
                crate::core::material_mix_registry::OwnedMaterialMixSlot::default(),
            ),
            #[cfg(not(target_arch = "wasm32"))]
            worker_callbacks: WorkerCallbackState::new(),
            user_data: RefCell::new(crate::core::user_data::UserDataStore::default()),
            _pin: PhantomPinned,
            foundation_lease: Cell::new(Some(foundation_lease)),
        })
    }

    #[inline]
    pub(crate) const fn brand(&self) -> IdBrand {
        self.brand
    }

    #[inline]
    pub(crate) fn check_definition_length_scale(
        &self,
        operation: &'static str,
        definition: crate::core::length_scale::LengthScale,
    ) -> Result<()> {
        self.length_scale.check_definition(operation, definition)
    }

    #[inline]
    pub(crate) const fn length_scale(&self) -> crate::core::length_scale::LengthScale {
        self.length_scale
    }

    #[inline]
    pub(crate) fn joint_base(
        &self,
        body_a: crate::BodyId,
        body_b: crate::BodyId,
    ) -> crate::JointBase {
        crate::JointBase::with_length_scale(body_a, body_b, self.length_scale)
    }

    #[inline]
    pub(crate) fn contact_epoch(&self) -> ContactEpoch {
        self.contact_epoch.get()
    }

    pub(crate) fn advance_contact_epoch(&self) -> Result<ContactEpoch> {
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

    pub(crate) fn prepare_contact_epoch(&self) -> Result<ContactEpoch> {
        self.contact_epoch.get().checked_next()
    }

    pub(crate) fn commit_contact_epoch(&self, next: ContactEpoch) -> Result<()> {
        if self.contact_epoch.get().checked_next()? != next {
            return Err(Error::WorldBusy);
        }
        self.contact_epoch.set(next);
        Ok(())
    }

    #[inline]
    pub(crate) fn check_available(&self) -> Result<()> {
        self.check_activity(ActivityState::Idle)
    }

    #[inline]
    fn check_activity(&self, expected: ActivityState) -> Result<()> {
        #[cfg(test)]
        self.access_checks.set(
            self.access_checks
                .get()
                .checked_add(1)
                .expect("world access check counter overflow"),
        );
        match self.lifecycle.get() {
            LifecycleState::Live => {}
            LifecycleState::Poisoned => return Err(Error::WorldPoisoned),
            LifecycleState::Destroyed => return Err(Error::WorldDestroyed),
        }
        if self.activity.get() == expected {
            Ok(())
        } else {
            Err(Error::WorldBusy)
        }
    }

    #[cfg(test)]
    pub(crate) fn access_check_count_for_test(&self) -> usize {
        self.access_checks.get()
    }

    #[cfg(test)]
    pub(crate) fn native_object_check_count_for_test(&self) -> usize {
        self.native_object_checks.get()
    }

    #[cfg(test)]
    pub(crate) fn identity_lock_count_for_test(&self) -> usize {
        self.identities.state_lock_count_for_test()
    }

    #[cfg(test)]
    pub(crate) fn creation_compensation_count_for_test(&self) -> usize {
        self.creation_compensations.get()
    }

    #[cfg(test)]
    pub(crate) fn fail_next_creation_compensation_for_test(&self) {
        self.fail_next_creation_compensation.set(true);
    }

    #[cfg(test)]
    pub(crate) fn fail_next_creation_reservation_for_test(&self) {
        self.identities.fail_next_creation_reservation_for_test();
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

    #[inline]
    fn native_body_is_valid(&self, raw: ffi::b2BodyId) -> bool {
        self.record_native_object_check();
        unsafe { ffi::b2Body_IsValid(raw) }
    }

    #[inline]
    fn native_shape_is_valid(&self, raw: ffi::b2ShapeId) -> bool {
        self.record_native_object_check();
        unsafe { ffi::b2Shape_IsValid(raw) }
    }

    #[inline]
    fn native_joint_is_valid(&self, raw: ffi::b2JointId) -> bool {
        self.record_native_object_check();
        unsafe { ffi::b2Joint_IsValid(raw) }
    }

    #[inline]
    fn native_chain_is_valid(&self, raw: ffi::b2ChainId) -> bool {
        self.record_native_object_check();
        unsafe { ffi::b2Chain_IsValid(raw) }
    }

    #[inline]
    fn native_contact_is_valid(&self, raw: ffi::b2ContactId) -> bool {
        self.record_native_object_check();
        unsafe { ffi::b2Contact_IsValid(raw) }
    }

    /// Authorize an operation owned by the active recording session.
    ///
    /// Ordinary world and handle entries continue to use `check_available`, so
    /// this does not make the recording activity visible through existing
    /// aliases. Public callers must perform the callback gate before entering
    /// this activity check.
    pub(crate) fn check_recording_available(&self) -> Result<()> {
        self.check_activity(ActivityState::Recording)
    }

    pub(crate) fn begin_recording_activity(&self) -> Result<RecordingActivityLease> {
        ExclusiveActivityLease::begin(self, ActivityState::Recording).map(RecordingActivityLease)
    }

    pub(crate) fn begin_restore_activity(&self) -> Result<RestoreActivityLease> {
        ExclusiveActivityLease::begin(self, ActivityState::Restoring).map(RestoreActivityLease)
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

    pub(crate) fn callback_registration_generations(&self) -> CallbackRegistrationGenerations {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let custom_filter = self
                .custom_filter
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
                .map(|registration| registration.generation());
            let pre_solve = self
                .pre_solve
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
                .map(|registration| registration.generation());
            CallbackRegistrationGenerations {
                custom_filter,
                pre_solve,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            CallbackRegistrationGenerations::default()
        }
    }

    #[inline]
    pub(crate) fn mixer_presence(&self) -> (bool, bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.material_mix
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .presence()
        }
        #[cfg(target_arch = "wasm32")]
        {
            (false, false)
        }
    }

    #[inline]
    pub(crate) fn mixer_identities(&self) -> crate::recording::MixerIdentities {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.material_mix
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .identities()
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::recording::MixerIdentities::default()
        }
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn lifecycle(&self) -> LifecycleState {
        self.lifecycle.get()
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn activity(&self) -> ActivityState {
        self.activity.get()
    }

    fn begin_activity(&self, next: ActivityState) -> Result<()> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Idle || next == ActivityState::Idle {
            return Err(Error::WorldBusy);
        }
        self.activity.set(next);
        Ok(())
    }

    pub(crate) fn poison(&self) {
        if std::cell::Cell::get(&self.lifecycle) == LifecycleState::Live {
            std::cell::Cell::set(&self.lifecycle, LifecycleState::Poisoned);
        }
    }

    fn compensate_creation(&self, native: NativeCreation) {
        #[cfg(test)]
        self.creation_compensations.set(
            self.creation_compensations
                .get()
                .checked_add(1)
                .expect("creation compensation counter overflow"),
        );
        #[cfg(test)]
        if self.fail_next_creation_compensation.replace(false) {
            self.poison();
            return;
        }

        let compensated = match native {
            NativeCreation::Body(raw) => {
                if self.brand.check_body_raw(raw).is_err() {
                    false
                } else if self.native_body_is_valid(raw) {
                    unsafe { ffi::b2DestroyBody(raw) };
                    !self.native_body_is_valid(raw)
                } else {
                    true
                }
            }
            NativeCreation::Shape {
                raw,
                update_body_mass,
            } => {
                if self.brand.check_shape_raw(raw).is_err() {
                    false
                } else if self.native_shape_is_valid(raw) {
                    unsafe { ffi::b2DestroyShape(raw, update_body_mass) };
                    !self.native_shape_is_valid(raw)
                } else {
                    true
                }
            }
            NativeCreation::Joint(raw) => {
                if self.brand.check_joint_raw(raw).is_err() {
                    false
                } else if self.native_joint_is_valid(raw) {
                    unsafe { ffi::b2DestroyJoint(raw, false) };
                    !self.native_joint_is_valid(raw)
                } else {
                    true
                }
            }
            NativeCreation::Chain(raw) => {
                if self.brand.check_chain_raw(raw).is_err() {
                    false
                } else if self.native_chain_is_valid(raw) {
                    unsafe { ffi::b2DestroyChain(raw) };
                    !self.native_chain_is_valid(raw)
                } else {
                    true
                }
            }
        };
        if !compensated {
            self.poison();
        }
    }

    #[inline]
    fn check_live(&self) -> Result<()> {
        match self.lifecycle.get() {
            LifecycleState::Live => Ok(()),
            LifecycleState::Poisoned => Err(Error::WorldPoisoned),
            LifecycleState::Destroyed => Err(Error::WorldDestroyed),
        }
    }

    #[inline]
    fn check_brand_identity(&self, brand: IdBrand) -> Result<()> {
        if brand == self.brand {
            Ok(())
        } else {
            Err(Error::WrongWorld)
        }
    }

    #[inline]
    fn check_brand(&self, brand: IdBrand) -> Result<()> {
        self.check_available()?;
        self.check_brand_identity(brand)
    }

    #[inline]
    pub(crate) fn check_body_identity(&self, id: BodyId) -> Result<()> {
        self.check_available()?;
        self.check_body_identity_after_preflight(id)
    }

    #[inline]
    pub(crate) fn check_body_identity_after_preflight(&self, id: BodyId) -> Result<()> {
        self.check_brand_identity(id.brand())?;
        if !self.identities.contains_body(id) {
            return Err(Error::InvalidBodyId);
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn check_body_native_after_identity(&self, id: BodyId) -> Result<()> {
        if self.native_body_is_valid(id.into_raw()) {
            Ok(())
        } else {
            Err(Error::InvalidBodyId)
        }
    }

    #[inline]
    pub(crate) fn check_body(&self, id: BodyId) -> Result<()> {
        self.check_available()?;
        self.check_body_after_preflight(id)
    }

    /// Validate a body after the capability owner has already authorized this activity.
    pub(crate) fn check_body_after_preflight(&self, id: BodyId) -> Result<()> {
        self.check_body_identity_after_preflight(id)?;
        self.check_body_native_after_identity(id)
    }

    #[inline]
    fn check_shape_native(&self, id: ShapeId) -> Result<()> {
        if !self.identities.contains_shape(id) {
            return Err(Error::InvalidShapeId);
        }
        if self.native_shape_is_valid(id.into_raw()) {
            Ok(())
        } else {
            Err(Error::InvalidShapeId)
        }
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn check_shape(&self, id: ShapeId) -> Result<()> {
        self.check_available()?;
        self.check_shape_after_preflight(id)
    }

    /// Validate a shape after the capability owner has already authorized this activity.
    pub(crate) fn check_shape_after_preflight(&self, id: ShapeId) -> Result<()> {
        self.check_brand_identity(id.brand())?;
        self.check_shape_native(id)
    }

    pub(crate) fn resolve_query_shape(&self, raw: ffi::b2ShapeId) -> Result<ShapeId> {
        self.with_output_identity_resolver(|resolver| resolver.active_shape(raw))
    }

    #[inline]
    fn check_joint_identity_after_preflight(&self, id: JointId) -> Result<JointType> {
        self.check_brand_identity(id.brand())?;
        self.identities.joint_type(id)
    }

    #[inline]
    pub(crate) fn check_joint_native_after_identity(&self, id: JointId) -> Result<()> {
        if self.native_joint_is_valid(id.into_raw()) {
            Ok(())
        } else {
            Err(Error::InvalidJointId)
        }
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn check_joint(&self, id: JointId) -> Result<()> {
        self.check_available()?;
        self.check_joint_after_preflight(id).map(|_| ())
    }

    /// Validate a joint and return its registered type after owner authorization.
    pub(crate) fn check_joint_after_preflight(&self, id: JointId) -> Result<JointType> {
        let joint_type = self.check_joint_identity_after_preflight(id)?;
        self.check_joint_native_after_identity(id)
            .map(|()| joint_type)
    }

    #[inline]
    fn check_chain_native(&self, id: ChainId) -> Result<()> {
        if !self.identities.contains_chain(id) {
            return Err(Error::InvalidChainId);
        }
        if self.native_chain_is_valid(id.into_raw()) {
            Ok(())
        } else {
            Err(Error::InvalidChainId)
        }
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn check_chain(&self, id: ChainId) -> Result<()> {
        self.check_available()?;
        self.check_chain_after_preflight(id)
    }

    /// Validate a chain after the capability owner has already authorized this activity.
    pub(crate) fn check_chain_after_preflight(&self, id: ChainId) -> Result<()> {
        self.check_brand_identity(id.brand())?;
        self.check_chain_native(id)
    }

    pub(crate) fn identity_manifest_while_restoring(
        &self,
    ) -> Result<crate::core::identity_registry::IdentityManifest> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(Error::WorldBusy);
        }
        self.identities.snapshot_manifest()
    }

    pub(crate) fn prepare_identity_restore(
        &self,
        manifest: &crate::core::identity_registry::IdentityManifest,
    ) -> Result<crate::core::identity_registry::PreparedIdentityRestore> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(Error::WorldBusy);
        }
        self.identities.prepare_restore(manifest)
    }

    pub(crate) fn commit_identity_restore(
        &self,
        prepared: crate::core::identity_registry::PreparedIdentityRestore,
    ) -> Result<()> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(Error::WorldBusy);
        }
        self.identities.commit_restore(prepared)
    }

    pub(crate) fn user_data_manifest_while_restoring(
        &self,
    ) -> Result<crate::core::user_data::UserDataManifest> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(Error::WorldBusy);
        }
        self.user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .snapshot_manifest()
    }

    pub(crate) fn prepare_user_data_restore(
        &self,
        manifest: &crate::core::user_data::UserDataManifest,
        identity_manifest: &crate::core::identity_registry::IdentityManifest,
        identities: &crate::core::identity_registry::PreparedIdentityRestore,
    ) -> Result<crate::core::user_data::PreparedUserDataRestore> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(Error::WorldBusy);
        }
        self.user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .prepare_restore(manifest, identity_manifest, identities)
    }

    pub(crate) fn commit_user_data_restore(
        &self,
        prepared: crate::core::user_data::PreparedUserDataRestore,
    ) -> Result<crate::core::user_data::CommittedUserDataRestore> {
        self.check_live()?;
        if self.activity.get() != ActivityState::Restoring {
            return Err(Error::WorldBusy);
        }
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        prepared.commit(&mut store)
    }

    pub(crate) fn release_completed_step_outputs(&self) {
        crate::core::identity_registry::ActiveIdentityRegistry::clear_retired_outputs(
            &self.identities,
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn step_shape_resolver(
        &self,
    ) -> Result<Arc<crate::core::identity_registry::StepShapeResolver>> {
        self.identities.step_shape_resolver()
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn fail_next_step_shape_snapshot_for_test(&self) {
        self.identities.fail_next_step_shape_snapshot_for_test();
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn hold_identity_lock_for_test(
        &self,
    ) -> crate::core::identity_registry::HeldIdentityLock<'_> {
        self.identities.hold_state_lock_for_test()
    }

    pub(crate) fn with_output_identity_resolver<T>(
        &self,
        resolve: impl FnOnce(&crate::core::identity_registry::OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        crate::core::identity_registry::ActiveIdentityRegistry::with_output_resolver(
            &self.identities,
            resolve,
        )
    }

    #[inline]
    fn check_contact_identity(&self, id: crate::types::ContactId) -> Result<()> {
        self.check_brand(id.brand())?;
        if id.contact_epoch() == self.contact_epoch() {
            Ok(())
        } else {
            Err(Error::InvalidContactId)
        }
    }

    #[inline]
    fn check_contact_native(&self, id: crate::types::ContactId) -> Result<()> {
        if self.native_contact_is_valid(id.into_raw()) {
            Ok(())
        } else {
            Err(Error::InvalidContactId)
        }
    }

    #[inline]
    pub(crate) fn check_contact(&self, id: crate::types::ContactId) -> Result<()> {
        self.check_contact_identity(id)?;
        self.check_contact_native(id)
    }

    #[inline]
    pub(crate) fn contact_is_valid(&self, id: crate::types::ContactId) -> Result<bool> {
        self.check_brand(id.brand())?;
        if id.contact_epoch() != self.contact_epoch() {
            return Ok(false);
        }
        Ok(self.native_contact_is_valid(id.into_raw()))
    }

    pub(crate) fn reserve_body_creation(
        &self,
    ) -> Result<crate::core::identity_registry::PendingBody> {
        self.identities.reserve_body()
    }

    pub(crate) fn reserve_shape_creation(
        &self,
        body: BodyId,
    ) -> Result<crate::core::identity_registry::PendingShape> {
        self.identities.reserve_shape(body)
    }

    pub(crate) fn reserve_joint_creation(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        kind: JointType,
    ) -> Result<crate::core::identity_registry::PendingJoint> {
        self.identities.reserve_joint(body_a, body_b, kind)
    }

    pub(crate) fn reserve_chain_creation(
        &self,
        body: BodyId,
        segment_count: usize,
    ) -> Result<crate::core::identity_registry::PendingChain> {
        self.identities.reserve_chain(body, segment_count)
    }

    fn claim_native_creation(&self, native: NativeCreation) -> Result<NativeCreationGuard<'_>> {
        let (belongs_to_world, already_owned, operation, output) = match native {
            NativeCreation::Body(raw) => (
                self.brand.check_body_raw(raw).is_ok(),
                self.identities.contains_body_raw(raw),
                "b2CreateBody",
                "body ID",
            ),
            NativeCreation::Shape { raw, .. } => (
                self.brand.check_shape_raw(raw).is_ok(),
                self.identities.contains_shape_raw(raw),
                "b2Create*Shape",
                "shape ID",
            ),
            NativeCreation::Joint(raw) => (
                self.brand.check_joint_raw(raw).is_ok(),
                self.identities.contains_joint_raw(raw),
                "b2Create*Joint",
                "joint ID",
            ),
            NativeCreation::Chain(raw) => (
                self.brand.check_chain_raw(raw).is_ok(),
                self.identities.contains_chain_raw(raw),
                "b2CreateChain",
                "chain ID",
            ),
        };

        // A raw identity already published by this registry is not owned by the current create
        // call. Arming rollback for it would let malformed native output destroy an older object.
        if !belongs_to_world || already_owned {
            self.poison();
            return Err(Error::InvalidNativeOutput {
                operation,
                output,
                constraint: "a new, unowned identifier in this world",
            });
        }

        Ok(NativeCreationGuard {
            core: self,
            native,
            armed: true,
        })
    }

    pub(crate) fn claim_created_body(&self, raw: ffi::b2BodyId) -> Result<NativeCreationGuard<'_>> {
        self.claim_native_creation(NativeCreation::Body(raw))
    }

    pub(crate) fn claim_created_shape(
        &self,
        raw: ffi::b2ShapeId,
        update_body_mass: bool,
    ) -> Result<NativeCreationGuard<'_>> {
        self.claim_native_creation(NativeCreation::Shape {
            raw,
            update_body_mass,
        })
    }

    pub(crate) fn claim_created_joint(
        &self,
        raw: ffi::b2JointId,
    ) -> Result<NativeCreationGuard<'_>> {
        self.claim_native_creation(NativeCreation::Joint(raw))
    }

    pub(crate) fn claim_created_chain(
        &self,
        raw: ffi::b2ChainId,
    ) -> Result<NativeCreationGuard<'_>> {
        self.claim_native_creation(NativeCreation::Chain(raw))
    }

    pub(crate) fn bind_created_body(
        &self,
        pending: crate::core::identity_registry::PendingBody,
        raw: ffi::b2BodyId,
    ) -> Result<crate::core::identity_registry::BoundBody> {
        (|| {
            self.brand.check_body_raw(raw)?;
            if !self.native_body_is_valid(raw) {
                return Err(Error::InvalidBodyId);
            }
            pending.bind(raw)
        })()
    }

    pub(crate) fn bind_created_shape(
        &self,
        pending: crate::core::identity_registry::PendingShape,
        raw: ffi::b2ShapeId,
    ) -> Result<crate::core::identity_registry::BoundShape> {
        (|| {
            self.brand.check_shape_raw(raw)?;
            if !self.native_shape_is_valid(raw) {
                return Err(Error::InvalidShapeId);
            }
            let expected_body = pending.body().into_raw();
            let actual_body = unsafe { ffi::b2Shape_GetBody(raw) };
            if !same_body_raw(expected_body, actual_body) {
                return Err(Error::InvalidShapeId);
            }
            pending.bind(raw)
        })()
    }

    pub(crate) fn bind_created_joint(
        &self,
        pending: crate::core::identity_registry::PendingJoint,
        raw: ffi::b2JointId,
    ) -> Result<crate::core::identity_registry::BoundJoint> {
        (|| {
            self.brand.check_joint_raw(raw)?;
            if !self.native_joint_is_valid(raw) {
                return Err(Error::InvalidJointId);
            }
            let [body_a, body_b] = pending.bodies().map(BodyId::into_raw);
            if !same_body_raw(body_a, unsafe { ffi::b2Joint_GetBodyA(raw) })
                || !same_body_raw(body_b, unsafe { ffi::b2Joint_GetBodyB(raw) })
            {
                return Err(Error::InvalidJointId);
            }
            pending.bind(raw)
        })()
    }

    pub(crate) fn bind_created_chain(
        &self,
        pending: crate::core::identity_registry::PendingChain,
        raw: ffi::b2ChainId,
    ) -> Result<crate::core::identity_registry::BoundChain> {
        (|| {
            self.brand.check_chain_raw(raw)?;
            if !self.native_chain_is_valid(raw) {
                return Err(Error::InvalidChainId);
            }
            let expected_body = pending.body().into_raw();
            let segment_count = unsafe { ffi::b2Chain_GetSegmentCount(raw) };
            // SAFETY: the identity reservation owns capacity for the expected number of segments;
            // the helper validates Box2D's initialized count before exposing the slice.
            unsafe {
                pending.bind_native(
                    raw,
                    segment_count,
                    |out, capacity| ffi::b2Chain_GetSegments(raw, out, capacity),
                    |segment| {
                        self.brand.check_shape_raw(segment)?;
                        if !self.native_shape_is_valid(segment)
                            || !same_body_raw(expected_body, ffi::b2Shape_GetBody(segment))
                            || !same_chain_raw(raw, ffi::b2Shape_GetParentChain(segment))
                        {
                            return Err(Error::InvalidShapeId);
                        }
                        Ok(())
                    },
                )
            }
        })()
    }

    fn body_shapes_for_destroy(&self, id: BodyId) -> Result<Vec<ShapeId>> {
        let raw = id.into_raw();
        let count = unsafe { ffi::b2Body_GetShapeCount(raw) };
        self.with_output_identity_resolver(|resolver| unsafe {
            crate::core::ffi_vec::try_read_mapped_from_ffi(
                count,
                |out, capacity| ffi::b2Body_GetShapes(raw, out, capacity),
                |shape| resolver.active_shape(shape),
            )
        })
    }

    fn body_joints_for_destroy(&self, id: BodyId) -> Result<Vec<JointId>> {
        let raw = id.into_raw();
        let count = unsafe { ffi::b2Body_GetJointCount(raw) };
        self.with_output_identity_resolver(|resolver| unsafe {
            crate::core::ffi_vec::try_read_mapped_from_ffi(
                count,
                |out, capacity| ffi::b2Body_GetJoints(raw, out, capacity),
                |joint| resolver.active_joint(joint),
            )
        })
    }

    fn chain_shapes_for_destroy(&self, id: ChainId) -> Result<Vec<ShapeId>> {
        let raw = id.into_raw();
        let count = unsafe { ffi::b2Chain_GetSegmentCount(raw) };
        self.with_output_identity_resolver(|resolver| unsafe {
            crate::core::ffi_vec::try_read_mapped_from_ffi(
                count,
                |out, capacity| ffi::b2Chain_GetSegments(raw, out, capacity),
                |shape| resolver.active_shape(shape),
            )
        })
    }

    fn check_user_data_mutable(
        &self,
        body: Option<BodyId>,
        shapes: &[ShapeId],
        joints: &[JointId],
    ) -> Result<()> {
        let entries = {
            let store = self
                .user_data
                .try_borrow()
                .map_err(|_| Error::ReentrantAccess)?;
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

    #[cfg(test)]
    fn check_object_destroy_preconditions(&self, brand: IdBrand) -> Result<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.check_available()?;
        self.check_brand_identity(brand)?;
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
                .into_erased()
        {
            retired.push(value);
        }
        for &shape in shapes {
            if let Some(value) = self
                .clear_shape_user_data(shape)
                .expect("user-data mutability checked before native destroy")
                .into_erased()
            {
                retired.push(value);
            }
        }
        for &joint in joints {
            if let Some(value) = self
                .clear_joint_user_data(joint)
                .expect("user-data mutability checked before native destroy")
                .into_erased()
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

    #[cfg(test)]
    pub(crate) fn destroy_body_now(&self, id: BodyId) -> Result<()> {
        self.check_object_destroy_preconditions(id.brand())?;
        self.check_body_identity_after_preflight(id)?;
        self.check_body_native_after_identity(id)?;
        self.destroy_acquired_body(id)
    }

    pub(crate) fn destroy_acquired_body(&self, id: BodyId) -> Result<()> {
        let shapes = self.body_shapes_for_destroy(id)?;
        let joints = self.body_joints_for_destroy(id)?;
        self.check_user_data_mutable(Some(id), &shapes, &joints)?;
        unsafe { ffi::b2DestroyBody(id.into_raw()) };
        let unregistered = self.identities.unregister_body(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(Some(id), &shapes, &joints));
        Ok(())
    }

    pub(crate) fn destroy_acquired_shape(&self, id: ShapeId, update_body_mass: bool) -> Result<()> {
        if unsafe { ffi::b2Shape_GetParentChain(id.into_raw()) }.index1 != 0 {
            return Err(Error::ChainOwnedShape);
        }
        self.check_user_data_mutable(None, core::slice::from_ref(&id), &[])?;
        unsafe { ffi::b2DestroyShape(id.into_raw(), update_body_mass) };
        let unregistered = self.identities.unregister_shape(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(None, core::slice::from_ref(&id), &[]));
        Ok(())
    }

    pub(crate) fn destroy_acquired_joint(&self, id: JointId, wake_bodies: bool) -> Result<()> {
        self.check_user_data_mutable(None, &[], core::slice::from_ref(&id))?;
        unsafe { ffi::b2DestroyJoint(id.into_raw(), wake_bodies) };
        let unregistered = self.identities.unregister_joint(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(None, &[], core::slice::from_ref(&id)));
        Ok(())
    }

    pub(crate) fn destroy_acquired_chain(&self, id: ChainId) -> Result<()> {
        let shapes = self.chain_shapes_for_destroy(id)?;
        self.check_user_data_mutable(None, &shapes, &[])?;
        unsafe { ffi::b2DestroyChain(id.into_raw()) };
        let unregistered = self.identities.unregister_chain(id);
        debug_assert!(unregistered);
        Self::drop_retired_user_data(self.retire_user_data(None, &shapes, &[]));
        Ok(())
    }

    /// End the native world's lifetime exactly once.
    pub(crate) fn shutdown_native(&self) {
        if self.native_destroyed.replace(true) {
            return;
        }
        self.lifecycle.set(LifecycleState::Destroyed);
        self.activity.set(ActivityState::Idle);

        {
            let _world_slot_guard = crate::core::foundation::lock_world_slot_mutation();
            // SAFETY: `World` owns the native lifetime and this method transitions the shared
            // lifecycle to `Destroyed` before making the one idempotent teardown call.
            unsafe { ffi::b2DestroyWorld(self.id) };
        }
        // Native destruction has joined the scheduler, so no step-local callback context remains.
        // Retire every process-local identity before releasing any host-owned payload.
        self.identities.clear();
        // Every safe user-data access borrows the unique World (or one of its capabilities), so
        // native shutdown cannot overlap a live store borrow. Drain before moving the foundation
        // lease out of the core; an invariant violation then leaves field drop order as the final
        // fallback.
        let user_data_entries = self.user_data.borrow_mut().drain_entries();
        // Keep this local alive until every callback owner and arbitrary user-data payload below
        // has been released. Rust also drops it last if one of those destructors panics.
        let foundation_lease = self.foundation_lease.take();
        let mut panic = crate::core::callback_state::PanicSlot::default();

        #[cfg(not(target_arch = "wasm32"))]
        let material_mix = self
            .material_mix
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .detach_after_native_destroyed();

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
        // Each arbitrary user-data payload is dropped behind its own panic boundary after native
        // teardown.
        #[cfg(not(target_arch = "wasm32"))]
        {
            panic.run_cleanup(|| drop(custom_filter));
            panic.run_cleanup(|| drop(pre_solve));
            material_mix.drain_panics(&mut panic);
            self.worker_callbacks.drain_panics(&mut panic);
        }
        for entry in user_data_entries {
            panic.run_cleanup(|| {
                let value = entry
                    .take_erased()
                    .expect("world user data cannot remain borrowed during native shutdown");
                drop(value);
            });
        }
        drop(foundation_lease);
        panic.resume_or_forget();
    }

    pub(crate) fn clear_world_user_data(&self) -> Result<crate::core::user_data::RetiredUserData> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(entry) = store.world.as_ref().cloned() else {
            return Ok(crate::core::user_data::RetiredUserData::default());
        };
        let retired = entry.take_erased()?;
        store.world = None;
        store.mark_changed();
        Ok(crate::core::user_data::RetiredUserData::new(retired))
    }

    pub(crate) fn clear_body_user_data(
        &self,
        id: BodyId,
    ) -> Result<crate::core::user_data::RetiredUserData> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(entry) = store.bodies.get(&id).cloned() else {
            return Ok(crate::core::user_data::RetiredUserData::default());
        };
        let retired = entry.take_erased()?;
        store.bodies.remove(&id);
        store.mark_changed();
        Ok(crate::core::user_data::RetiredUserData::new(retired))
    }

    pub(crate) fn clear_shape_user_data(
        &self,
        id: ShapeId,
    ) -> Result<crate::core::user_data::RetiredUserData> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(entry) = store.shapes.get(&id).cloned() else {
            return Ok(crate::core::user_data::RetiredUserData::default());
        };
        let retired = entry.take_erased()?;
        store.shapes.remove(&id);
        store.mark_changed();
        Ok(crate::core::user_data::RetiredUserData::new(retired))
    }

    pub(crate) fn clear_joint_user_data(
        &self,
        id: JointId,
    ) -> Result<crate::core::user_data::RetiredUserData> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(entry) = store.joints.get(&id).cloned() else {
            return Ok(crate::core::user_data::RetiredUserData::default());
        };
        let retired = entry.take_erased()?;
        store.joints.remove(&id);
        store.mark_changed();
        Ok(crate::core::user_data::RetiredUserData::new(retired))
    }

    pub(crate) fn set_world_user_data<T: 'static>(
        &self,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.world.clone();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        let (entry, pointer) =
            crate::core::user_data::UserDataEntry::new(value.into_inner(), version);
        store.world = Some(entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_body_user_data<T: 'static>(
        &self,
        id: BodyId,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.bodies.get(&id).cloned();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        store
            .bodies
            .try_reserve(1)
            .map_err(|_| Error::UserDataAllocationFailed)?;
        let (entry, pointer) =
            crate::core::user_data::UserDataEntry::new(value.into_inner(), version);
        store.bodies.insert(id, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_shape_user_data<T: 'static>(
        &self,
        id: ShapeId,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.shapes.get(&id).cloned();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        store
            .shapes
            .try_reserve(1)
            .map_err(|_| Error::UserDataAllocationFailed)?;
        let (entry, pointer) =
            crate::core::user_data::UserDataEntry::new(value.into_inner(), version);
        store.shapes.insert(id, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn set_joint_user_data<T: 'static>(
        &self,
        id: JointId,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let version = store.next_version()?;
        let entry = store.joints.get(&id).cloned();
        if let Some(entry) = entry {
            return entry.replace(value, version);
        }

        store
            .joints
            .try_reserve(1)
            .map_err(|_| Error::UserDataAllocationFailed)?;
        let (entry, pointer) =
            crate::core::user_data::UserDataEntry::new(value.into_inner(), version);
        store.joints.insert(id, entry);
        Ok(crate::core::user_data::UserDataUpdate::inserted(pointer))
    }

    pub(crate) fn try_with_world_user_data<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        self.check_available()?;
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .world
            .clone();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn borrow_body_user_data<T: 'static, R, F>(
        &self,
        id: BodyId,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .bodies
            .get(&id)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn borrow_shape_user_data<T: 'static, R, F>(
        &self,
        id: ShapeId,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .shapes
            .get(&id)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn borrow_joint_user_data<T: 'static, R, F>(
        &self,
        id: JointId,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .joints
            .get(&id)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with(f)
    }

    pub(crate) fn borrow_body_user_data_mut<T: 'static, R, F>(
        &self,
        id: BodyId,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .bodies
            .get(&id)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with_mut(f)
    }

    pub(crate) fn borrow_shape_user_data_mut<T: 'static, R, F>(
        &self,
        id: ShapeId,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .shapes
            .get(&id)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with_mut(f)
    }

    pub(crate) fn borrow_joint_user_data_mut<T: 'static, R, F>(
        &self,
        id: JointId,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> crate::error::Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        let entry = self
            .user_data
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?
            .joints
            .get(&id)
            .cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.try_with_mut(f)
    }

    pub(crate) fn take_world_user_data<T: 'static>(&self) -> crate::error::Result<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
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
    ) -> crate::error::Result<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
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
    ) -> crate::error::Result<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
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
    ) -> crate::error::Result<Option<T>> {
        let mut store = self
            .user_data
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(entry) = store.joints.get(&id).cloned() else {
            return Ok(None);
        };
        let value = entry.take::<T>()?;
        store.joints.remove(&id);
        store.mark_changed();
        Ok(value)
    }
}

impl Drop for WorldCore {
    fn drop(&mut self) {
        self.shutdown_native();
    }
}

#[cfg(test)]
mod identity_tests {
    use super::LifecycleState;
    use crate::id::ContactEpoch;
    use crate::joints::DistanceJointDef;
    use crate::shapes::{Circle, ShapeDef, circle};
    use crate::{BodyId, BodyType, ChainDef, Error, ShapeId, Vec2, World};
    use boxdd_sys::ffi;

    fn assert_native_identity_claim_rejected<T>(result: crate::Result<T>) {
        assert!(matches!(result, Err(Error::InvalidNativeOutput { .. })));
    }

    fn create_raw_circle(body: BodyId, def: &ShapeDef, circle: Circle) -> ffi::b2ShapeId {
        let def = def.prepare();
        let circle = circle.into_raw();
        unsafe { ffi::b2CreateCircleShape(body.into_raw(), &def, &circle) }
    }

    fn bind_raw_shape(world: &World, body: BodyId, raw: ffi::b2ShapeId) -> Result<ShapeId, Error> {
        let pending = world.core().reserve_shape_creation(body)?;
        world
            .core()
            .bind_created_shape(pending, raw)
            .map(crate::core::identity_registry::BoundShape::publish)
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
            let active = bind_raw_shape(world, body, raw).unwrap();
            destroy_registered_shape(world, active);
        }

        let raw = create_raw_circle(body, def, circle);
        assert!(raw_shape_eq(raw, retired_raw));
        raw
    }

    #[test]
    fn contact_epoch_exhaustion_poisons_the_world() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world
            .core()
            .contact_epoch
            .set(ContactEpoch::new_for_test(u64::MAX));

        assert_eq!(
            world.core().advance_contact_epoch(),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
    }

    #[test]
    fn binding_rejects_an_active_raw_identity_without_poisoning() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let core = world.core();

        let pending = core.reserve_body_creation().unwrap();
        assert!(matches!(
            core.bind_created_body(pending, body.into_raw()),
            Err(Error::ObjectIdentityExhausted)
        ));
        assert_eq!(core.lifecycle(), LifecycleState::Live);
    }

    #[test]
    fn creation_claim_never_compensates_an_active_native_identity() {
        {
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_builder()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let raw = body.into_raw();
            let before = world.core().creation_compensation_count_for_test();

            assert_native_identity_claim_rejected(world.core().claim_created_body(raw));

            assert!(unsafe { ffi::b2Body_IsValid(raw) });
            assert!(world.core().identities.contains_body(body));
            assert_eq!(world.core().creation_compensation_count_for_test(), before);
            assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
        }

        {
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_builder()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let circle = circle([0.0, 0.0], 0.5).unwrap();
            let shape = world
                .body(body)
                .unwrap()
                .create_circle(&ShapeDef::default(), &circle)
                .unwrap();
            let raw = shape.into_raw();
            let before = world.core().creation_compensation_count_for_test();

            assert_native_identity_claim_rejected(world.core().claim_created_shape(raw, true));

            assert!(unsafe { ffi::b2Shape_IsValid(raw) });
            assert!(world.core().identities.contains_shape(shape));
            assert_eq!(world.core().creation_compensation_count_for_test(), before);
            assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
        }

        {
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body_a = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_builder()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let body_b = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_builder()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let base = world.joint_base(body_a, body_b).unwrap();
            let joint = world
                .create_distance_joint(&DistanceJointDef::new(base))
                .unwrap();
            let raw = joint.into_raw();
            let before = world.core().creation_compensation_count_for_test();

            assert_native_identity_claim_rejected(world.core().claim_created_joint(raw));

            assert!(unsafe { ffi::b2Joint_IsValid(raw) });
            assert!(world.core().identities.contains_joint(joint));
            assert_eq!(world.core().creation_compensation_count_for_test(), before);
            assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
        }

        {
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_builder()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let def = ChainDef::builder()
                .points([
                    Vec2::new(-2.0, 0.0),
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(2.0, 0.0),
                ])
                .build()
                .unwrap();
            let chain = world.body(body).unwrap().create_chain(&def).unwrap();
            let raw = chain.into_raw();
            let before = world.core().creation_compensation_count_for_test();

            assert_native_identity_claim_rejected(world.core().claim_created_chain(raw));

            assert!(unsafe { ffi::b2Chain_IsValid(raw) });
            assert!(world.core().identities.contains_chain(chain));
            assert_eq!(world.core().creation_compensation_count_for_test(), before);
            assert_eq!(world.core().lifecycle(), LifecycleState::Poisoned);
        }
    }

    #[test]
    fn creation_reservation_failures_never_reach_native_code() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let before = world.counters().unwrap();
        let compensations = world.core().creation_compensation_count_for_test();

        world.core().fail_next_creation_reservation_for_test();
        assert_eq!(
            world.create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap()
            ),
            Err(Error::IdentityTrackingAllocationFailed)
        );

        world.core().fail_next_creation_reservation_for_test();
        let circle = circle(Vec2::ZERO, 0.5).unwrap();
        assert_eq!(
            world
                .body(body_a)
                .unwrap()
                .create_circle(&ShapeDef::default(), &circle),
            Err(Error::IdentityTrackingAllocationFailed)
        );

        let chain_def = ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build()
            .unwrap();
        world.core().fail_next_creation_reservation_for_test();
        assert_eq!(
            world.body(body_a).unwrap().create_chain(&chain_def),
            Err(Error::IdentityTrackingAllocationFailed)
        );

        world.core().fail_next_creation_reservation_for_test();
        let base = world.joint_base(body_a, body_b).unwrap();
        assert_eq!(
            world.create_distance_joint(&DistanceJointDef::new(base)),
            Err(Error::IdentityTrackingAllocationFailed)
        );

        assert_eq!(world.counters().unwrap(), before);
        assert_eq!(
            world.core().creation_compensation_count_for_test(),
            compensations
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Live);
    }

    #[test]
    fn retained_contact_end_shape_key_rejects_u16_generation_wrap_before_publication() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_builder()
                    .gravity([0.0_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let static_body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let dynamic_body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .gravity_scale(0.0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let contact_def = ShapeDef::builder()
            .enable_contact_events(true)
            .build()
            .unwrap();
        let circle = circle([0.0_f32, 0.0], 0.5).unwrap();
        let static_shape = world
            .body(static_body)
            .unwrap()
            .create_circle(&contact_def, &circle)
            .unwrap();
        let retired = world
            .body(dynamic_body)
            .unwrap()
            .create_circle(&contact_def, &circle)
            .unwrap();

        let completed = world.step(1.0 / 60.0, 4).unwrap();
        assert!(
            completed
                .contact_events()
                .unwrap()
                .begin()
                .iter()
                .any(|event| {
                    (event.shape_a == static_shape && event.shape_b == retired)
                        || (event.shape_a == retired && event.shape_b == static_shape)
                })
        );
        drop(completed);

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
        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| resolver.shape(retired.into_raw())),
            Ok(retired)
        );

        let conflicting_raw = recycle_raw_shape_slot_until_retired_key_conflicts(
            &world,
            dynamic_body,
            &contact_def,
            circle,
            retired,
        );
        assert_eq!(
            bind_raw_shape(&world, dynamic_body, conflicting_raw),
            Err(Error::ObjectIdentityExhausted)
        );
        unsafe { ffi::b2DestroyShape(conflicting_raw, true) };
        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| resolver.shape(conflicting_raw)),
            Ok(retired)
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Live);
    }

    #[test]
    fn retained_sensor_end_shape_key_rejects_u16_generation_wrap_before_publication() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_builder()
                    .gravity([0.0_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let sensor_body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let visitor_body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .gravity_scale(0.0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let sensor_def = ShapeDef::builder()
            .sensor(true)
            .enable_sensor_events(true)
            .build()
            .unwrap();
        let visitor_def = ShapeDef::builder()
            .enable_sensor_events(true)
            .build()
            .unwrap();
        let circle = circle([0.0_f32, 0.0], 0.5).unwrap();
        let sensor = world
            .body(sensor_body)
            .unwrap()
            .create_circle(&sensor_def, &circle)
            .unwrap();
        let retired = world
            .body(visitor_body)
            .unwrap()
            .create_circle(&visitor_def, &circle)
            .unwrap();

        let completed = world.step(1.0 / 60.0, 4).unwrap();
        assert!(
            completed
                .sensor_events()
                .unwrap()
                .begin()
                .iter()
                .any(|event| { event.sensor_shape == sensor && event.visitor_shape == retired })
        );
        drop(completed);

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
        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| resolver.shape(retired.into_raw())),
            Ok(retired)
        );

        let conflicting_raw = recycle_raw_shape_slot_until_retired_key_conflicts(
            &world,
            visitor_body,
            &visitor_def,
            circle,
            retired,
        );
        assert_eq!(
            bind_raw_shape(&world, visitor_body, conflicting_raw),
            Err(Error::ObjectIdentityExhausted)
        );
        unsafe { ffi::b2DestroyShape(conflicting_raw, true) };
        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| resolver.shape(conflicting_raw)),
            Ok(retired)
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Live);
    }

    #[test]
    fn invalid_native_creation_outputs_are_rejected_without_publication() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let core = world.core();
        let pending = core.reserve_body_creation().unwrap();
        assert!(matches!(
            core.bind_created_body(
                pending,
                ffi::b2BodyId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
            ),
            Err(Error::InvalidBodyId)
        ));
        assert_eq!(core.lifecycle(), LifecycleState::Live);

        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let core = world.core();
        let pending = core.reserve_shape_creation(body).unwrap();
        assert!(matches!(
            core.bind_created_shape(
                pending,
                ffi::b2ShapeId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
            ),
            Err(Error::InvalidShapeId)
        ));
        assert_eq!(core.lifecycle(), LifecycleState::Live);

        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let core = world.core();
        let pending = core
            .reserve_joint_creation(body_a, body_b, crate::JointType::Distance)
            .unwrap();
        assert!(matches!(
            core.bind_created_joint(
                pending,
                ffi::b2JointId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
            ),
            Err(Error::InvalidJointId)
        ));
        assert_eq!(core.lifecycle(), LifecycleState::Live);

        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let core = world.core();
        let pending = core.reserve_chain_creation(body, 1).unwrap();
        assert!(matches!(
            core.bind_created_chain(
                pending,
                ffi::b2ChainId {
                    index1: 0,
                    world0: core.brand().world0(),
                    generation: 0,
                },
            ),
            Err(Error::InvalidChainId)
        ));
        assert_eq!(core.lifecycle(), LifecycleState::Live);
    }

    #[test]
    fn destroy_checks_identity_and_native_validity_before_unrelated_user_data() {
        let mut source = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let foreign = source
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let mut target = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        target.set_user_data(7_u32).unwrap();

        let foreign_result = {
            let target_core = target.core();
            target.with_user_data::<u32, _>(|_| target_core.destroy_body_now(foreign))
        };
        assert_eq!(foreign_result, Ok(Some(Err(Error::WrongWorld))));
        assert_eq!(source.core().check_body(foreign), Ok(()));

        let stale = target
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        target.body(stale).unwrap().destroy().unwrap();
        let stale_result = {
            let target_core = target.core();
            target.with_user_data::<u32, _>(|_| target_core.destroy_body_now(stale))
        };
        assert_eq!(stale_result, Ok(Some(Err(Error::InvalidBodyId))));
    }

    #[test]
    fn user_data_reentry_is_scoped_to_each_entry_and_recovers_after_conflict() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        world.body(body_a).unwrap().set_user_data(10_u32).unwrap();
        world.body(body_b).unwrap().set_user_data(20_u32).unwrap();

        let core = world.core();
        let nested = core.borrow_body_user_data::<u32, _, _>(
            body_a,
            crate::core::callback_state::PendingUserValue::new(|body_a_value: &u32| {
                core.borrow_body_user_data_mut::<u32, _, _>(
                    body_b,
                    crate::core::callback_state::PendingUserValue::new(|body_b_value: &mut u32| {
                        *body_b_value += *body_a_value;
                        *body_b_value
                    }),
                )
            }),
        );
        assert_eq!(nested, Ok(Some(Ok(Some(30)))));

        let conflict = core.borrow_body_user_data::<u32, _, _>(
            body_a,
            crate::core::callback_state::PendingUserValue::new(|_: &u32| {
                core.borrow_body_user_data_mut::<u32, _, _>(
                    body_a,
                    crate::core::callback_state::PendingUserValue::new(|value: &mut u32| {
                        *value += 1;
                    }),
                )
            }),
        );
        assert_eq!(conflict, Ok(Some(Err(Error::ReentrantAccess))));

        assert_eq!(
            core.borrow_body_user_data_mut::<u32, _, _>(
                body_a,
                crate::core::callback_state::PendingUserValue::new(|value: &mut u32| {
                    *value += 5;
                    *value
                }),
            ),
            Ok(Some(15))
        );
        assert_eq!(
            core.borrow_body_user_data::<u32, _, _>(
                body_b,
                crate::core::callback_state::PendingUserValue::new(|value: &u32| *value),
            ),
            Ok(Some(30))
        );
    }
}

#[cfg(test)]
mod auto_trait_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use static_assertions::{assert_impl_all, assert_not_impl_any};

    use super::{ActivityState, LifecycleState, WorldCore};
    use crate::Error;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::core::callback_state::WorkerCallbackState;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::core::identity_registry::StepShapeResolver;

    #[cfg(not(target_arch = "wasm32"))]
    assert_impl_all!(WorkerCallbackState: Send, Sync);
    #[cfg(not(target_arch = "wasm32"))]
    assert_impl_all!(StepShapeResolver: Send, Sync);
    assert_not_impl_any!(WorldCore: Send, Sync, Unpin);

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

    #[test]
    fn callback_and_owner_state_have_distinct_threading_contracts() {
        // Compile-time assertions above are the behavior under test.
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn competing_worker_panic_payload_is_not_dropped_on_the_callback_stack() {
        PANICKING_PAYLOAD_DROPS.store(0, Ordering::SeqCst);
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let worker = &world.core().worker_callbacks;

        worker.record_panic(Box::new("first panic"));
        worker.record_panic(Box::new(PanickingPayload));

        assert_eq!(PANICKING_PAYLOAD_DROPS.load(Ordering::SeqCst), 0);
        let mut panic = crate::core::callback_state::PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert_eq!(PANICKING_PAYLOAD_DROPS.load(Ordering::SeqCst), 1);
        let first = panic.into_result(()).expect_err("first panic payload");
        assert_eq!(first.downcast_ref::<&str>(), Some(&"first panic"));
        worker.begin_call().unwrap();
    }

    #[test]
    fn lifecycle_and_activity_are_orthogonal() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let core = world.core();

        assert_eq!(core.lifecycle(), LifecycleState::Live);
        assert_eq!(core.activity(), ActivityState::Idle);
        let recording = core.begin_recording_activity().unwrap();
        assert_eq!(core.check_available(), Err(Error::WorldBusy));
        assert_eq!(core.begin_restore_activity().err(), Some(Error::WorldBusy));
        drop(recording);
        let restoring = core.begin_restore_activity().unwrap();
        assert_eq!(core.check_available(), Err(Error::WorldBusy));
        drop(restoring);

        core.poison();
        assert_eq!(core.lifecycle(), LifecycleState::Poisoned);
        assert_eq!(core.activity(), ActivityState::Idle);
        assert_eq!(core.check_available(), Err(Error::WorldPoisoned));
    }
}
