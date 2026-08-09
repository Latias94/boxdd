use crate::core::callback_state::MaterialMixCtx;
use crate::world::MaterialMixInput;
use boxdd_sys::ffi;
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, RwLock};

pub(crate) const MATERIAL_MIX_SLOT_COUNT: usize = 64;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LeaseGeneration(NonZeroU64);

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PublicationGeneration(NonZeroU64);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum MaterialMixRegistryError {
    SlotsExhausted,
    InvalidSlot,
    SlotPoisoned,
    StaleLease,
    InvalidOwnerState,
    PublicationGenerationExhausted,
}

pub(crate) struct MaterialMixerRegistration {
    identity: crate::recording::MixerId,
    context: Arc<MaterialMixCtx>,
}

impl Clone for MaterialMixerRegistration {
    fn clone(&self) -> Self {
        Self {
            identity: self.identity,
            context: Arc::clone(&self.context),
        }
    }
}

impl MaterialMixerRegistration {
    pub(crate) fn new(identity: crate::recording::MixerId, context: Arc<MaterialMixCtx>) -> Self {
        Self { identity, context }
    }
}

struct MaterialMixPublication {
    lease_generation: LeaseGeneration,
    publication_generation: PublicationGeneration,
    friction: Option<MaterialMixerRegistration>,
    restitution: Option<MaterialMixerRegistration>,
}

impl MaterialMixPublication {
    fn callback(&self, kind: MaterialMixKind) -> Option<Arc<MaterialMixCtx>> {
        self.registration(kind)
            .map(|registration| Arc::clone(&registration.context))
    }

    fn callback_ref(&self, kind: MaterialMixKind) -> Option<&MaterialMixCtx> {
        self.registration(kind)
            .map(|registration| registration.context.as_ref())
    }

    fn registration(&self, kind: MaterialMixKind) -> Option<&MaterialMixerRegistration> {
        match kind {
            MaterialMixKind::Friction => self.friction.as_ref(),
            MaterialMixKind::Restitution => self.restitution.as_ref(),
        }
    }

    fn identities(&self) -> crate::recording::MixerIdentities {
        crate::recording::MixerIdentities::new(
            self.friction
                .as_ref()
                .map(|registration| registration.identity),
            self.restitution
                .as_ref()
                .map(|registration| registration.identity),
        )
    }
}

#[derive(Copy, Clone)]
enum MaterialMixKind {
    Friction,
    Restitution,
}

struct MaterialMixSlotState {
    active_generation: Option<LeaseGeneration>,
    next_generation: Option<NonZeroU64>,
    last_publication_generation: Option<PublicationGeneration>,
    publication: Option<Arc<MaterialMixPublication>>,
}

impl MaterialMixSlotState {
    const fn new() -> Self {
        Self {
            active_generation: None,
            next_generation: NonZeroU64::new(1),
            last_publication_generation: None,
            publication: None,
        }
    }
}

struct MaterialMixSlot {
    state: RwLock<MaterialMixSlotState>,
    active_publication: AtomicPtr<MaterialMixPublication>,
}

impl MaterialMixSlot {
    const fn new() -> Self {
        Self {
            state: RwLock::new(MaterialMixSlotState::new()),
            active_publication: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    fn try_acquire(
        &self,
        slot: usize,
    ) -> Result<Option<MaterialMixLease>, MaterialMixRegistryError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| MaterialMixRegistryError::SlotPoisoned)?;
        if state.active_generation.is_some()
            || state.publication.is_some()
            || !self.active_publication.load(Ordering::Acquire).is_null()
        {
            return Ok(None);
        }
        let Some(next_generation) = state.next_generation else {
            return Ok(None);
        };
        let generation = LeaseGeneration(next_generation);
        state.next_generation = next_generation
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new);
        state.active_generation = Some(generation);
        state.last_publication_generation = None;
        Ok(Some(MaterialMixLease {
            slot,
            generation,
            next_publication_generation: NonZeroU64::new(1),
            released: false,
        }))
    }

    fn publish(
        &self,
        lease_generation: LeaseGeneration,
        publication: Arc<MaterialMixPublication>,
    ) -> Result<Option<Arc<MaterialMixPublication>>, MaterialMixRegistryError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| MaterialMixRegistryError::SlotPoisoned)?;
        if state.active_generation != Some(lease_generation)
            || publication.lease_generation != lease_generation
        {
            return Err(MaterialMixRegistryError::StaleLease);
        }
        if state
            .last_publication_generation
            .is_some_and(|generation| generation >= publication.publication_generation)
        {
            return Err(MaterialMixRegistryError::StaleLease);
        }
        if !self.active_publication.load(Ordering::Acquire).is_null() {
            return Err(MaterialMixRegistryError::InvalidOwnerState);
        }
        state.last_publication_generation = Some(publication.publication_generation);
        Ok(state.publication.replace(publication))
    }

    fn release(
        &self,
        lease_generation: LeaseGeneration,
    ) -> Result<Option<Arc<MaterialMixPublication>>, MaterialMixRegistryError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| MaterialMixRegistryError::SlotPoisoned)?;
        if state.active_generation != Some(lease_generation) {
            return Err(MaterialMixRegistryError::StaleLease);
        }
        if !self.active_publication.load(Ordering::Acquire).is_null() {
            return Err(MaterialMixRegistryError::InvalidOwnerState);
        }
        let retired = state.publication.take();
        state.active_generation = None;
        state.last_publication_generation = None;
        Ok(retired)
    }

    fn callback(&self, kind: MaterialMixKind) -> Option<Arc<MaterialMixCtx>> {
        // Only clone immutable ownership while locked. User code and the final context destructor
        // always run after the read guard has been released. This is the defensive fallback when
        // no native-call snapshot is active; ordinary native calls never reach this path.
        let state = self.state.read().ok()?;
        state
            .publication
            .as_ref()
            .and_then(|publication| publication.callback(kind))
    }

    fn activate(
        &self,
        lease_generation: LeaseGeneration,
        publication: &Arc<MaterialMixPublication>,
    ) -> Result<(), MaterialMixRegistryError> {
        let state = self
            .state
            .read()
            .map_err(|_| MaterialMixRegistryError::SlotPoisoned)?;
        if state.active_generation != Some(lease_generation)
            || publication.lease_generation != lease_generation
            || !state
                .publication
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, publication))
        {
            return Err(MaterialMixRegistryError::StaleLease);
        }

        let pointer = Arc::as_ptr(publication).cast_mut();
        self.active_publication
            .compare_exchange(
                core::ptr::null_mut(),
                pointer,
                Ordering::Release,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| MaterialMixRegistryError::InvalidOwnerState)
    }

    fn invoke(
        &self,
        kind: MaterialMixKind,
        value_a: f32,
        user_material_id_a: u64,
        value_b: f32,
        user_material_id_b: u64,
        default_mix: fn(f32, f32) -> f32,
    ) -> f32 {
        let publication = self.active_publication.load(Ordering::Acquire);
        if publication.is_null() {
            return invoke_mix_callback(
                self.callback(kind),
                value_a,
                user_material_id_a,
                value_b,
                user_material_id_b,
                default_mix,
            );
        }

        // SAFETY: the active-call guard owns an `Arc<MaterialMixPublication>` from before the
        // native call begins until after Box2D synchronously joins every worker. Publish/release
        // fail closed while this pointer is present, so every callback in this native call observes
        // the same generation. Only the guard's exact-pointer CAS clears it.
        let publication = unsafe { &*publication };
        invoke_mix_callback_ref(
            publication.callback_ref(kind),
            value_a,
            user_material_id_a,
            value_b,
            user_material_id_b,
            default_mix,
        )
    }
}

static MATERIAL_MIX_SLOTS: [MaterialMixSlot; MATERIAL_MIX_SLOT_COUNT] =
    [const { MaterialMixSlot::new() }; MATERIAL_MIX_SLOT_COUNT];

#[inline]
fn slot_ref(slot: usize) -> Result<&'static MaterialMixSlot, MaterialMixRegistryError> {
    MATERIAL_MIX_SLOTS
        .get(slot)
        .ok_or(MaterialMixRegistryError::InvalidSlot)
}

fn acquire_slot() -> Result<MaterialMixLease, MaterialMixRegistryError> {
    for (idx, slot) in MATERIAL_MIX_SLOTS.iter().enumerate() {
        match slot.try_acquire(idx) {
            Ok(Some(lease)) => return Ok(lease),
            Ok(None) | Err(MaterialMixRegistryError::SlotPoisoned) => {}
            Err(error) => return Err(error),
        }
    }
    Err(MaterialMixRegistryError::SlotsExhausted)
}

struct MaterialMixLease {
    slot: usize,
    generation: LeaseGeneration,
    next_publication_generation: Option<NonZeroU64>,
    released: bool,
}

impl MaterialMixLease {
    fn publish(
        &mut self,
        friction: Option<MaterialMixerRegistration>,
        restitution: Option<MaterialMixerRegistration>,
    ) -> Result<MaterialMixPublicationUpdate, MaterialMixOperationFailure> {
        let Some(next_generation) = self.next_publication_generation else {
            return Err(MaterialMixOperationFailure::with_registrations(
                MaterialMixRegistryError::PublicationGenerationExhausted,
                friction,
                restitution,
            ));
        };
        let publication = Arc::new(MaterialMixPublication {
            lease_generation: self.generation,
            publication_generation: PublicationGeneration(next_generation),
            friction,
            restitution,
        });
        let published = Arc::clone(&publication);
        let retired_registry =
            match slot_ref(self.slot).and_then(|slot| slot.publish(self.generation, publication)) {
                Ok(retired) => retired,
                Err(error) => {
                    return Err(MaterialMixOperationFailure::with_attempted(
                        error, published,
                    ));
                }
            };
        self.next_publication_generation = next_generation
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new);
        Ok(MaterialMixPublicationUpdate {
            slot: self.slot,
            publication: published,
            retired: RetiredMaterialMixPublications {
                registry: retired_registry,
                ..RetiredMaterialMixPublications::default()
            },
        })
    }

    fn release(&mut self) -> Result<Option<Arc<MaterialMixPublication>>, MaterialMixRegistryError> {
        if self.released {
            return Err(MaterialMixRegistryError::StaleLease);
        }
        let retired = slot_ref(self.slot)?.release(self.generation)?;
        self.released = true;
        Ok(retired)
    }
}

/// Owns the immutable mixer publication visible to one synchronous native call.
///
/// Box2D joins all worker callbacks before returning from the call. Clearing the fast lookup in
/// `Drop` therefore happens before this guard can release the publication that callbacks borrowed.
#[must_use = "an active material-mix snapshot must outlive its synchronous native call"]
pub(crate) struct ActiveMaterialMixSnapshot {
    slot: usize,
    publication: Option<Arc<MaterialMixPublication>>,
    _owner_thread: PhantomData<Rc<()>>,
}

impl ActiveMaterialMixSnapshot {
    pub(crate) fn finish(self) {
        drop(self);
    }
}

impl Drop for ActiveMaterialMixSnapshot {
    fn drop(&mut self) {
        let Some(publication) = self.publication.take() else {
            return;
        };
        let pointer = Arc::as_ptr(&publication).cast_mut();
        if let Ok(slot) = slot_ref(self.slot) {
            // Publication mutation is rejected while this snapshot is active. Keep the exact
            // pointer comparison as a defensive guard so inconsistent internal state cannot clear
            // an unrelated publication.
            let _ = slot.active_publication.compare_exchange(
                pointer,
                core::ptr::null_mut(),
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        // Keep a possible final callback-owned destructor behind the same boundary used by every
        // other callback-owned value so an outer unwind cannot turn a cleanup panic into SIGABRT.
        let mut panic = crate::core::callback_state::PanicSlot::default();
        panic.run_cleanup(|| drop(publication));
        panic.resume_or_forget();
    }
}

#[derive(Default)]
#[must_use = "retired material-mix callbacks must be released after dropping registry locks"]
pub(crate) struct RetiredMaterialMixPublications {
    owner: Option<Arc<MaterialMixPublication>>,
    registry: Option<Arc<MaterialMixPublication>>,
    attempted: Option<Arc<MaterialMixPublication>>,
    unpublished_friction: Option<MaterialMixerRegistration>,
    unpublished_restitution: Option<MaterialMixerRegistration>,
}

impl RetiredMaterialMixPublications {
    fn with_owner(mut self, owner: Option<Arc<MaterialMixPublication>>) -> Self {
        self.owner = owner;
        self
    }

    pub(crate) fn drain_panics(mut self, panic: &mut crate::core::callback_state::PanicSlot) {
        // Keep every context alive while publications are dismantled, then release contexts behind
        // independent panic boundaries so two user destructors cannot cause a double unwind.
        let mut contexts = Vec::with_capacity(10);
        for publication in [
            self.owner.as_ref(),
            self.registry.as_ref(),
            self.attempted.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(context) = publication.callback(MaterialMixKind::Friction) {
                contexts.push(context);
            }
            if let Some(context) = publication.callback(MaterialMixKind::Restitution) {
                contexts.push(context);
            }
        }
        if let Some(registration) = self.unpublished_friction.as_ref() {
            contexts.push(Arc::clone(&registration.context));
        }
        if let Some(registration) = self.unpublished_restitution.as_ref() {
            contexts.push(Arc::clone(&registration.context));
        }

        let owner = self.owner.take();
        let registry = self.registry.take();
        let attempted = self.attempted.take();
        let unpublished_friction = self.unpublished_friction.take();
        let unpublished_restitution = self.unpublished_restitution.take();
        panic.run_cleanup(|| drop(owner));
        panic.run_cleanup(|| drop(registry));
        panic.run_cleanup(|| drop(attempted));
        panic.run_cleanup(|| drop(unpublished_friction));
        panic.run_cleanup(|| drop(unpublished_restitution));
        for context in contexts {
            panic.run_cleanup(|| drop(context));
        }
    }

    pub(crate) fn resume_drop_panics(self) {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        self.drain_panics(&mut panic);
        panic.resume_or_forget();
    }
}

pub(crate) struct MaterialMixOperationFailure {
    error: MaterialMixRegistryError,
    retired: RetiredMaterialMixPublications,
}

impl MaterialMixOperationFailure {
    fn with_registrations(
        error: MaterialMixRegistryError,
        friction: Option<MaterialMixerRegistration>,
        restitution: Option<MaterialMixerRegistration>,
    ) -> Self {
        Self {
            error,
            retired: RetiredMaterialMixPublications {
                unpublished_friction: friction,
                unpublished_restitution: restitution,
                ..RetiredMaterialMixPublications::default()
            },
        }
    }

    fn with_attempted(
        error: MaterialMixRegistryError,
        attempted: Arc<MaterialMixPublication>,
    ) -> Self {
        Self {
            error,
            retired: RetiredMaterialMixPublications {
                attempted: Some(attempted),
                ..RetiredMaterialMixPublications::default()
            },
        }
    }

    pub(crate) const fn error(&self) -> MaterialMixRegistryError {
        self.error
    }

    pub(crate) fn into_retired(self) -> RetiredMaterialMixPublications {
        self.retired
    }
}

struct MaterialMixPublicationUpdate {
    slot: usize,
    publication: Arc<MaterialMixPublication>,
    retired: RetiredMaterialMixPublications,
}

#[derive(Default)]
pub(crate) struct OwnedMaterialMixSlot {
    lease: Option<MaterialMixLease>,
    publication: Option<Arc<MaterialMixPublication>>,
}

impl OwnedMaterialMixSlot {
    pub(crate) fn set_friction(
        &mut self,
        registration: MaterialMixerRegistration,
    ) -> Result<MaterialMixOwnerUpdate, MaterialMixOperationFailure> {
        self.set(MaterialMixKind::Friction, registration)
    }

    pub(crate) fn set_restitution(
        &mut self,
        registration: MaterialMixerRegistration,
    ) -> Result<MaterialMixOwnerUpdate, MaterialMixOperationFailure> {
        self.set(MaterialMixKind::Restitution, registration)
    }

    fn set(
        &mut self,
        kind: MaterialMixKind,
        registration: MaterialMixerRegistration,
    ) -> Result<MaterialMixOwnerUpdate, MaterialMixOperationFailure> {
        let acquired = if self.lease.is_none() {
            match acquire_slot() {
                Ok(lease) => {
                    self.lease = Some(lease);
                    true
                }
                Err(error) => {
                    let (friction, restitution) = match kind {
                        MaterialMixKind::Friction => (Some(registration), None),
                        MaterialMixKind::Restitution => (None, Some(registration)),
                    };
                    return Err(MaterialMixOperationFailure::with_registrations(
                        error,
                        friction,
                        restitution,
                    ));
                }
            }
        } else {
            false
        };

        let current_friction = self
            .publication
            .as_ref()
            .and_then(|publication| publication.friction.clone());
        let current_restitution = self
            .publication
            .as_ref()
            .and_then(|publication| publication.restitution.clone());
        let (friction, restitution) = match kind {
            MaterialMixKind::Friction => (Some(registration), current_restitution),
            MaterialMixKind::Restitution => (current_friction, Some(registration)),
        };
        let Some(lease) = self.lease.as_mut() else {
            return Err(MaterialMixOperationFailure::with_registrations(
                MaterialMixRegistryError::InvalidOwnerState,
                friction,
                restitution,
            ));
        };
        let result = lease.publish(friction, restitution);
        let update = match result {
            Ok(update) => update,
            Err(mut failure) => {
                if acquired {
                    let retired_registry = self
                        .lease
                        .as_mut()
                        .and_then(|lease| lease.release().ok().flatten());
                    failure.retired.registry = retired_registry;
                    self.lease = None;
                }
                return Err(failure);
            }
        };
        let retired = update
            .retired
            .with_owner(self.publication.replace(update.publication));
        Ok(MaterialMixOwnerUpdate {
            slot: update.slot,
            retired,
        })
    }

    pub(crate) fn clear_friction(
        &mut self,
    ) -> Result<RetiredMaterialMixPublications, MaterialMixOperationFailure> {
        self.clear(MaterialMixKind::Friction)
    }

    pub(crate) fn clear_restitution(
        &mut self,
    ) -> Result<RetiredMaterialMixPublications, MaterialMixOperationFailure> {
        self.clear(MaterialMixKind::Restitution)
    }

    fn clear(
        &mut self,
        kind: MaterialMixKind,
    ) -> Result<RetiredMaterialMixPublications, MaterialMixOperationFailure> {
        let Some(publication) = self.publication.as_ref() else {
            return Ok(RetiredMaterialMixPublications::default());
        };
        if publication.registration(kind).is_none() {
            return Ok(RetiredMaterialMixPublications::default());
        }
        let mut friction = publication.friction.clone();
        let mut restitution = publication.restitution.clone();
        match kind {
            MaterialMixKind::Friction => friction = None,
            MaterialMixKind::Restitution => restitution = None,
        }
        if friction.is_none() && restitution.is_none() {
            return self.release_all();
        }
        let Some(lease) = self.lease.as_mut() else {
            return Err(MaterialMixOperationFailure::with_registrations(
                MaterialMixRegistryError::InvalidOwnerState,
                friction,
                restitution,
            ));
        };
        let update = lease.publish(friction, restitution)?;
        Ok(update
            .retired
            .with_owner(self.publication.replace(update.publication)))
    }

    pub(crate) fn release_all(
        &mut self,
    ) -> Result<RetiredMaterialMixPublications, MaterialMixOperationFailure> {
        let Some(lease) = self.lease.as_mut() else {
            return Ok(RetiredMaterialMixPublications {
                owner: self.publication.take(),
                ..RetiredMaterialMixPublications::default()
            });
        };
        match lease.release() {
            Ok(registry) => {
                self.lease = None;
                Ok(RetiredMaterialMixPublications {
                    owner: self.publication.take(),
                    registry,
                    ..RetiredMaterialMixPublications::default()
                })
            }
            Err(error) => {
                if error == MaterialMixRegistryError::InvalidOwnerState {
                    return Err(MaterialMixOperationFailure {
                        error,
                        retired: RetiredMaterialMixPublications::default(),
                    });
                }
                self.lease = None;
                Err(MaterialMixOperationFailure {
                    error,
                    retired: RetiredMaterialMixPublications {
                        owner: self.publication.take(),
                        ..RetiredMaterialMixPublications::default()
                    },
                })
            }
        }
    }

    pub(crate) fn detach_after_native_destroyed(&mut self) -> RetiredMaterialMixPublications {
        match self.release_all() {
            Ok(retired) => retired,
            Err(failure) => failure.retired,
        }
    }

    pub(crate) fn activate_snapshot(
        &self,
    ) -> Result<Option<ActiveMaterialMixSnapshot>, MaterialMixRegistryError> {
        let (Some(lease), Some(publication)) = (&self.lease, &self.publication) else {
            return if self.lease.is_none() && self.publication.is_none() {
                Ok(None)
            } else {
                Err(MaterialMixRegistryError::InvalidOwnerState)
            };
        };
        let publication = Arc::clone(publication);
        slot_ref(lease.slot)?.activate(lease.generation, &publication)?;
        Ok(Some(ActiveMaterialMixSnapshot {
            slot: lease.slot,
            publication: Some(publication),
            _owner_thread: PhantomData,
        }))
    }

    pub(crate) fn presence(&self) -> (bool, bool) {
        self.publication
            .as_ref()
            .map_or((false, false), |publication| {
                (
                    publication.friction.is_some(),
                    publication.restitution.is_some(),
                )
            })
    }

    pub(crate) fn identities(&self) -> crate::recording::MixerIdentities {
        self.publication
            .as_ref()
            .map_or_else(crate::recording::MixerIdentities::default, |publication| {
                publication.identities()
            })
    }

    pub(crate) fn clearing_friction_releases_slot(&self) -> bool {
        matches!(self.presence(), (true, false))
    }

    pub(crate) fn clearing_restitution_releases_slot(&self) -> bool {
        matches!(self.presence(), (false, true))
    }

    #[cfg(test)]
    pub(crate) fn slot_for_test(&self) -> Option<usize> {
        self.lease.as_ref().map(|lease| lease.slot)
    }
}

#[must_use = "the native callback and retired publication must be committed after publication"]
pub(crate) struct MaterialMixOwnerUpdate {
    slot: usize,
    retired: RetiredMaterialMixPublications,
}

impl MaterialMixOwnerUpdate {
    pub(crate) const fn slot(&self) -> usize {
        self.slot
    }

    pub(crate) fn into_retired(self) -> RetiredMaterialMixPublications {
        self.retired
    }
}

#[inline]
fn default_friction_mix(friction_a: f32, friction_b: f32) -> f32 {
    (friction_a * friction_b).sqrt()
}

#[inline]
fn default_restitution_mix(restitution_a: f32, restitution_b: f32) -> f32 {
    restitution_a.max(restitution_b)
}

fn invoke_mix_callback_ref(
    context: Option<&MaterialMixCtx>,
    value_a: f32,
    user_material_id_a: u64,
    value_b: f32,
    user_material_id_b: u64,
    default_mix: fn(f32, f32) -> f32,
) -> f32 {
    let Some(context) = context else {
        return default_mix(value_a, value_b);
    };

    let fallback = default_mix(value_a, value_b);
    crate::core::callback_state::invoke_worker_callback(&context.worker, fallback, || {
        let mixed = (context.cb)(
            MaterialMixInput::new(value_a, user_material_id_a),
            MaterialMixInput::new(value_b, user_material_id_b),
        );
        assert!(
            mixed.is_finite() && mixed >= 0.0,
            "material mixing callback must return a finite non-negative coefficient, got {mixed}"
        );
        mixed
    })
}

fn invoke_mix_callback(
    context: Option<Arc<MaterialMixCtx>>,
    value_a: f32,
    user_material_id_a: u64,
    value_b: f32,
    user_material_id_b: u64,
    default_mix: fn(f32, f32) -> f32,
) -> f32 {
    let Some(context) = context else {
        return default_mix(value_a, value_b);
    };

    // The registry fallback may race callback replacement, so it pins the context independently.
    // Retiring that final Arc can run arbitrary user destructors and must stay behind the worker
    // panic boundary. Ordinary native calls use `invoke_mix_callback_ref` and avoid both clones.
    let worker = Arc::clone(&context.worker);
    let result = invoke_mix_callback_ref(
        Some(context.as_ref()),
        value_a,
        user_material_id_a,
        value_b,
        user_material_id_b,
        default_mix,
    );
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(context))) {
        worker.record_panic(payload);
    }
    result
}

#[inline]
fn invoke_friction_callback(
    slot: usize,
    friction_a: f32,
    user_material_id_a: u64,
    friction_b: f32,
    user_material_id_b: u64,
) -> f32 {
    slot_ref(slot).map_or_else(
        |_| default_friction_mix(friction_a, friction_b),
        |slot| {
            slot.invoke(
                MaterialMixKind::Friction,
                friction_a,
                user_material_id_a,
                friction_b,
                user_material_id_b,
                default_friction_mix,
            )
        },
    )
}

#[inline]
fn invoke_restitution_callback(
    slot: usize,
    restitution_a: f32,
    user_material_id_a: u64,
    restitution_b: f32,
    user_material_id_b: u64,
) -> f32 {
    slot_ref(slot).map_or_else(
        |_| default_restitution_mix(restitution_a, restitution_b),
        |slot| {
            slot.invoke(
                MaterialMixKind::Restitution,
                restitution_a,
                user_material_id_a,
                restitution_b,
                user_material_id_b,
                default_restitution_mix,
            )
        },
    )
}

type MaterialMixTrampoline = unsafe extern "C" fn(f32, u64, f32, u64) -> f32;

unsafe extern "C" fn friction_trampoline<const SLOT: usize>(
    friction_a: f32,
    user_material_id_a: u64,
    friction_b: f32,
    user_material_id_b: u64,
) -> f32 {
    invoke_friction_callback(
        SLOT,
        friction_a,
        user_material_id_a,
        friction_b,
        user_material_id_b,
    )
}

unsafe extern "C" fn restitution_trampoline<const SLOT: usize>(
    restitution_a: f32,
    user_material_id_a: u64,
    restitution_b: f32,
    user_material_id_b: u64,
) -> f32 {
    invoke_restitution_callback(
        SLOT,
        restitution_a,
        user_material_id_a,
        restitution_b,
        user_material_id_b,
    )
}

macro_rules! material_mix_trampolines {
    ($trampoline:ident) => {
        [
            $trampoline::<0>,
            $trampoline::<1>,
            $trampoline::<2>,
            $trampoline::<3>,
            $trampoline::<4>,
            $trampoline::<5>,
            $trampoline::<6>,
            $trampoline::<7>,
            $trampoline::<8>,
            $trampoline::<9>,
            $trampoline::<10>,
            $trampoline::<11>,
            $trampoline::<12>,
            $trampoline::<13>,
            $trampoline::<14>,
            $trampoline::<15>,
            $trampoline::<16>,
            $trampoline::<17>,
            $trampoline::<18>,
            $trampoline::<19>,
            $trampoline::<20>,
            $trampoline::<21>,
            $trampoline::<22>,
            $trampoline::<23>,
            $trampoline::<24>,
            $trampoline::<25>,
            $trampoline::<26>,
            $trampoline::<27>,
            $trampoline::<28>,
            $trampoline::<29>,
            $trampoline::<30>,
            $trampoline::<31>,
            $trampoline::<32>,
            $trampoline::<33>,
            $trampoline::<34>,
            $trampoline::<35>,
            $trampoline::<36>,
            $trampoline::<37>,
            $trampoline::<38>,
            $trampoline::<39>,
            $trampoline::<40>,
            $trampoline::<41>,
            $trampoline::<42>,
            $trampoline::<43>,
            $trampoline::<44>,
            $trampoline::<45>,
            $trampoline::<46>,
            $trampoline::<47>,
            $trampoline::<48>,
            $trampoline::<49>,
            $trampoline::<50>,
            $trampoline::<51>,
            $trampoline::<52>,
            $trampoline::<53>,
            $trampoline::<54>,
            $trampoline::<55>,
            $trampoline::<56>,
            $trampoline::<57>,
            $trampoline::<58>,
            $trampoline::<59>,
            $trampoline::<60>,
            $trampoline::<61>,
            $trampoline::<62>,
            $trampoline::<63>,
        ]
    };
}

static FRICTION_TRAMPOLINES: [MaterialMixTrampoline; MATERIAL_MIX_SLOT_COUNT] =
    material_mix_trampolines!(friction_trampoline);
static RESTITUTION_TRAMPOLINES: [MaterialMixTrampoline; MATERIAL_MIX_SLOT_COUNT] =
    material_mix_trampolines!(restitution_trampoline);

#[inline]
pub(crate) fn friction_callback(slot: usize) -> ffi::b2FrictionCallback {
    Some(FRICTION_TRAMPOLINES[slot])
}

#[inline]
pub(crate) fn restitution_callback(slot: usize) -> ffi::b2RestitutionCallback {
    Some(RESTITUTION_TRAMPOLINES[slot])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static_assertions::assert_not_impl_any!(ActiveMaterialMixSnapshot: Send, Sync);

    const MIXER: crate::MixerId = crate::MixerId::from_bytes([0x71; 32]);

    struct DropFlag(Arc<AtomicUsize>);

    impl DropFlag {
        fn touch(&self) {}
    }

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct PanicOnDrop;

    impl PanicOnDrop {
        fn touch(&self) {}
    }

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("intentional material-mix callback drop panic");
        }
    }

    fn registration<F>(
        worker: &Arc<crate::core::callback_state::WorkerCallbackState>,
        callback: F,
    ) -> MaterialMixerRegistration
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        MaterialMixerRegistration::new(
            MIXER,
            Arc::new(MaterialMixCtx {
                worker: Arc::clone(worker),
                cb: Box::new(callback),
            }),
        )
    }

    fn publication(
        lease: &MaterialMixLease,
        generation: u64,
        friction: Option<MaterialMixerRegistration>,
        restitution: Option<MaterialMixerRegistration>,
    ) -> Arc<MaterialMixPublication> {
        Arc::new(MaterialMixPublication {
            lease_generation: lease.generation,
            publication_generation: PublicationGeneration(NonZeroU64::new(generation).unwrap()),
            friction,
            restitution,
        })
    }

    #[test]
    fn null_callbacks_preserve_box2d_default_mixing_rules() {
        let friction = invoke_mix_callback(None, 0.25, 11, 0.81, 22, default_friction_mix);
        let restitution = invoke_mix_callback(None, 0.25, 11, 0.81, 22, default_restitution_mix);

        assert_eq!(friction, (0.25_f32 * 0.81).sqrt());
        assert_eq!(restitution, 0.81);
        assert_eq!(default_friction_mix(f32::MAX, f32::MAX), f32::INFINITY);
    }

    #[test]
    fn every_material_mix_trampoline_routes_to_its_array_index() {
        let worker = crate::core::callback_state::WorkerCallbackState::new();
        let mut owners = Vec::with_capacity(MATERIAL_MIX_SLOT_COUNT);

        for expected_slot in 0..MATERIAL_MIX_SLOT_COUNT {
            let mut owner = OwnedMaterialMixSlot::default();
            let friction = registration(&worker, move |_, _| expected_slot as f32);
            let update = owner.set_friction(friction).unwrap_or_else(|failure| {
                panic!("failed to register friction slot: {:?}", failure.error())
            });
            assert_eq!(update.slot(), expected_slot);
            drop(update.into_retired());

            let restitution = registration(&worker, move |_, _| (1000 + expected_slot) as f32);
            let update = owner
                .set_restitution(restitution)
                .unwrap_or_else(|failure| {
                    panic!("failed to register restitution slot: {:?}", failure.error())
                });
            assert_eq!(update.slot(), expected_slot);
            drop(update.into_retired());
            let active = owner
                .activate_snapshot()
                .unwrap_or_else(|error| panic!("failed to activate material-mix slot: {error:?}"))
                .expect("a configured material-mix slot must publish a native-call snapshot");
            owners.push((owner, active));
        }

        for slot in 0..MATERIAL_MIX_SLOT_COUNT {
            let friction = unsafe { FRICTION_TRAMPOLINES[slot](0.25, 1, 0.81, 2) };
            let restitution = unsafe { RESTITUTION_TRAMPOLINES[slot](0.25, 1, 0.81, 2) };
            assert_eq!(friction, slot as f32);
            assert_eq!(restitution, (1000 + slot) as f32);
        }

        for (mut owner, active) in owners {
            active.finish();
            let retired = owner.release_all().unwrap_or_else(|failure| {
                panic!("failed to release material-mix slot: {:?}", failure.error())
            });
            drop(retired);
        }
    }

    #[test]
    fn panicking_material_callbacks_use_exact_defaults_and_worker_reuse() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let worker = Arc::clone(&world.core().worker_callbacks);
        let context = Arc::new(MaterialMixCtx {
            worker: Arc::clone(&worker),
            cb: Box::new(|_, _| -> f32 { panic!("friction mix test panic") }),
        });

        let first = invoke_mix_callback(
            Some(Arc::clone(&context)),
            0.25,
            11,
            0.81,
            22,
            default_friction_mix,
        );
        let second = invoke_mix_callback(
            Some(Arc::clone(&context)),
            0.25,
            11,
            0.81,
            22,
            default_friction_mix,
        );
        let expected_friction = (0.25_f32 * 0.81).sqrt();
        assert_eq!(first, expected_friction);
        assert_eq!(second, expected_friction);

        let mut panic = crate::core::callback_state::PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert!(panic.into_result(()).is_err());
        worker.begin_call().unwrap();
        drop(context);

        let context = Arc::new(MaterialMixCtx {
            worker: Arc::clone(&worker),
            cb: Box::new(|_, _| -> f32 { panic!("restitution mix test panic") }),
        });
        let first = invoke_mix_callback(
            Some(Arc::clone(&context)),
            0.25,
            11,
            0.81,
            22,
            default_restitution_mix,
        );
        let second = invoke_mix_callback(
            Some(Arc::clone(&context)),
            0.25,
            11,
            0.81,
            22,
            default_restitution_mix,
        );
        assert_eq!(first, 0.81);
        assert_eq!(second, 0.81);

        let mut panic = crate::core::callback_state::PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert!(panic.into_result(()).is_err());
        worker.begin_call().unwrap();
        drop(context);
    }

    #[test]
    fn callback_destructor_panic_is_captured_before_returning_to_native() {
        let worker = crate::core::callback_state::WorkerCallbackState::new();
        let marker = PanicOnDrop;
        let context = Arc::new(MaterialMixCtx {
            worker: Arc::clone(&worker),
            cb: Box::new(move |_, _| {
                marker.touch();
                0.5
            }),
        });

        assert_eq!(
            invoke_mix_callback(Some(context), 0.25, 1, 0.81, 2, default_friction_mix,),
            0.5
        );
        let mut panic = crate::core::callback_state::PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert!(panic.into_result(()).is_err());
    }

    #[test]
    fn active_snapshot_invocation_does_not_read_the_registry_lock() {
        let slot = Arc::new(MaterialMixSlot::new());
        let lease = slot.try_acquire(0).unwrap().unwrap();
        let worker = crate::core::callback_state::WorkerCallbackState::new();
        let owner = publication(&lease, 1, Some(registration(&worker, |_, _| 0.5)), None);
        assert!(
            slot.publish(lease.generation, Arc::clone(&owner))
                .unwrap()
                .is_none()
        );
        slot.activate(lease.generation, &owner).unwrap();

        let poison_target = Arc::clone(&slot);
        assert!(
            std::panic::catch_unwind(move || {
                let _guard = poison_target.state.write().unwrap();
                panic!("poison material-mix registry after publishing the call snapshot");
            })
            .is_err()
        );
        assert_eq!(
            slot.invoke(
                MaterialMixKind::Friction,
                0.25,
                1,
                0.81,
                2,
                default_friction_mix,
            ),
            0.5
        );

        slot.active_publication
            .store(core::ptr::null_mut(), Ordering::Release);
        drop(owner);
    }

    #[test]
    fn active_call_pins_generation_until_snapshot_finishes() {
        let slot = MaterialMixSlot::new();
        let old_lease = slot.try_acquire(0).unwrap().unwrap();
        let old_generation = old_lease.generation;
        let worker = crate::core::callback_state::WorkerCallbackState::new();
        let old_calls = Arc::new(AtomicUsize::new(0));
        let new_calls = Arc::new(AtomicUsize::new(0));
        let old_dropped = Arc::new(AtomicUsize::new(0));
        let old_marker = DropFlag(Arc::clone(&old_dropped));
        let old_registration = registration(&worker, {
            let old_calls = Arc::clone(&old_calls);
            move |_, _| {
                old_marker.touch();
                old_calls.fetch_add(1, Ordering::SeqCst);
                0.25
            }
        });
        let old_owner = publication(&old_lease, 1, Some(old_registration), None);
        assert!(
            slot.publish(old_generation, Arc::clone(&old_owner))
                .unwrap()
                .is_none()
        );
        slot.activate(old_generation, &old_owner).unwrap();

        assert!(matches!(
            slot.release(old_generation),
            Err(MaterialMixRegistryError::InvalidOwnerState)
        ));
        let replacement = publication(
            &old_lease,
            2,
            Some(registration(&worker, {
                let new_calls = Arc::clone(&new_calls);
                move |_, _| {
                    new_calls.fetch_add(1, Ordering::SeqCst);
                    0.81
                }
            })),
            None,
        );
        assert!(matches!(
            slot.publish(old_generation, replacement),
            Err(MaterialMixRegistryError::InvalidOwnerState)
        ));

        for _ in 0..2 {
            assert_eq!(
                slot.invoke(
                    MaterialMixKind::Friction,
                    0.25,
                    1,
                    0.81,
                    2,
                    default_friction_mix,
                ),
                0.25
            );
        }
        assert_eq!(old_calls.load(Ordering::SeqCst), 2);
        assert_eq!(new_calls.load(Ordering::SeqCst), 0);

        let old_pointer = Arc::as_ptr(&old_owner).cast_mut();
        assert_eq!(
            slot.active_publication.compare_exchange(
                old_pointer,
                core::ptr::null_mut(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ),
            Ok(old_pointer)
        );
        let old_registry = slot.release(old_generation).unwrap().unwrap();
        drop(old_registry);
        drop(old_owner);
        assert_eq!(old_dropped.load(Ordering::SeqCst), 1);

        let new_lease = slot.try_acquire(0).unwrap().unwrap();
        assert_ne!(new_lease.generation, old_generation);
        let new_registration = registration(&worker, {
            let new_calls = Arc::clone(&new_calls);
            move |_, _| {
                new_calls.fetch_add(1, Ordering::SeqCst);
                0.81
            }
        });
        let new_owner = publication(&new_lease, 1, Some(new_registration), None);
        assert!(
            slot.publish(new_lease.generation, Arc::clone(&new_owner))
                .unwrap()
                .is_none()
        );
        slot.activate(new_lease.generation, &new_owner).unwrap();
        let new_result = slot.invoke(
            MaterialMixKind::Friction,
            0.25,
            1,
            0.81,
            2,
            default_friction_mix,
        );
        assert_eq!(new_result, 0.81);
        assert_eq!(new_calls.load(Ordering::SeqCst), 1);

        let new_pointer = Arc::as_ptr(&new_owner).cast_mut();
        assert_eq!(
            slot.active_publication.compare_exchange(
                new_pointer,
                core::ptr::null_mut(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ),
            Ok(new_pointer)
        );
        drop(slot.release(new_lease.generation).unwrap());
        drop(new_owner);
    }

    #[test]
    fn stale_generation_cannot_release_or_replace_a_reused_slot() {
        let slot = MaterialMixSlot::new();
        let first = slot.try_acquire(0).unwrap().unwrap();
        let first_generation = first.generation;
        assert!(slot.try_acquire(0).unwrap().is_none());
        assert!(slot.release(first_generation).unwrap().is_none());

        let second = slot.try_acquire(0).unwrap().unwrap();
        assert_ne!(second.generation, first_generation);
        assert!(matches!(
            slot.release(first_generation),
            Err(MaterialMixRegistryError::StaleLease)
        ));
        let worker = crate::core::callback_state::WorkerCallbackState::new();
        let stale_publication =
            publication(&first, 1, Some(registration(&worker, |_, _| 0.25)), None);
        assert!(matches!(
            slot.publish(first_generation, stale_publication),
            Err(MaterialMixRegistryError::StaleLease)
        ));
        assert!(slot.callback(MaterialMixKind::Friction).is_none());
        assert!(slot.release(second.generation).unwrap().is_none());
    }

    #[test]
    fn poisoned_and_generation_exhausted_slots_fail_closed() {
        let poisoned = Arc::new(MaterialMixSlot::new());
        let poison_target = Arc::clone(&poisoned);
        assert!(
            std::panic::catch_unwind(move || {
                let _guard = poison_target.state.write().unwrap();
                panic!("poison material-mix slot");
            })
            .is_err()
        );
        assert!(matches!(
            poisoned.try_acquire(0),
            Err(MaterialMixRegistryError::SlotPoisoned)
        ));
        assert!(poisoned.callback(MaterialMixKind::Friction).is_none());

        let exhausted = MaterialMixSlot::new();
        exhausted.state.write().unwrap().next_generation = NonZeroU64::new(u64::MAX);
        let final_lease = exhausted.try_acquire(0).unwrap().unwrap();
        assert_eq!(final_lease.generation.0.get(), u64::MAX);
        assert!(exhausted.release(final_lease.generation).unwrap().is_none());
        assert!(exhausted.try_acquire(0).unwrap().is_none());

        let publication_exhausted = MaterialMixSlot::new();
        let mut lease = publication_exhausted.try_acquire(0).unwrap().unwrap();
        lease.next_publication_generation = None;
        let worker = crate::core::callback_state::WorkerCallbackState::new();
        let failure = match lease.publish(Some(registration(&worker, |_, _| 0.5)), None) {
            Err(failure) => failure,
            Ok(update) => {
                drop(update);
                panic!("an exhausted publication generation must fail closed");
            }
        };
        assert_eq!(
            failure.error(),
            MaterialMixRegistryError::PublicationGenerationExhausted
        );
        failure.into_retired().resume_drop_panics();
        assert!(
            publication_exhausted
                .release(lease.generation)
                .unwrap()
                .is_none()
        );
    }
}
