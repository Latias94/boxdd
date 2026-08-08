//! Lazy, borrow-scoped event access for one completed simulation step.
//!
//! `World::step` does not fetch any native event buffer. A [`CompletedStep`] fetches and binds one
//! event family only when that family is requested, keeps the mapped storage borrowed while it is
//! viewed, and reuses the same allocation on later steps.

use crate::core::world_core::WorldCore;
use crate::error::{Error, Result};
use crate::id::ContactEpoch;
use crate::world::OwnerAdapter;
use boxdd_sys::ffi;
use std::cell::{Cell, UnsafeCell};

mod body;
mod contact;
mod joint;
mod sensor;

pub use body::{BodyEvents, BodyMoveEvent};
pub use contact::{
    ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactEventsView, ContactHitEvent,
};
pub use joint::{JointEvent, JointEvents};
pub use sensor::{SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents, SensorEventsView};

#[derive(Copy, Clone, Default)]
enum Materialization {
    #[default]
    Unfetched,
    Fetching,
    Ready,
    Failed(Error),
}

struct LazyEventSlot<T> {
    state: Cell<Materialization>,
    value: UnsafeCell<T>,
}

impl<T: Default> Default for LazyEventSlot<T> {
    fn default() -> Self {
        Self {
            state: Cell::new(Materialization::Unfetched),
            value: UnsafeCell::new(T::default()),
        }
    }
}

impl<T> LazyEventSlot<T> {
    fn reset(&self) {
        self.state.set(Materialization::Unfetched);
    }

    fn get_or_materialize(&self, fetch: impl FnOnce(&mut T) -> Result<()>) -> Result<&T> {
        match self.state.get() {
            Materialization::Ready => return Ok(self.value()),
            Materialization::Failed(error) => return Err(error),
            Materialization::Fetching => return Err(Error::ReentrantAccess),
            Materialization::Unfetched => {}
        }

        self.state.set(Materialization::Fetching);
        let mut reset_on_unwind = FetchReset {
            state: &self.state,
            armed: true,
        };
        // SAFETY: `Unfetched` proves this slot has issued no reference for the current step.
        // `CompletedStep` exclusively borrows the owner, so another step cannot reset and mutate
        // this storage while a returned view is live. Leaking a view and its capability makes the
        // references inaccessible before Safe Rust can borrow the owner again.
        let result = fetch(unsafe { &mut *self.value.get() });
        match result {
            Ok(()) => self.state.set(Materialization::Ready),
            Err(error) => self.state.set(Materialization::Failed(error)),
        }
        reset_on_unwind.armed = false;
        result?;
        Ok(self.value())
    }

    fn value(&self) -> &T {
        // SAFETY: Values are mutated only during `Fetching`, before `Ready` is published. See the
        // exclusive completed-step invariant in `get_or_materialize`.
        unsafe { &*self.value.get() }
    }
}

struct FetchReset<'a> {
    state: &'a Cell<Materialization>,
    armed: bool,
}

impl Drop for FetchReset<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.state.set(Materialization::Unfetched);
        }
    }
}

/// Reusable event storage owned by one world.
#[derive(Default)]
pub(crate) struct EventStorage {
    body: LazyEventSlot<Vec<BodyMoveEvent>>,
    contact: LazyEventSlot<ContactEvents>,
    joint: LazyEventSlot<Vec<JointEvent>>,
    sensor: LazyEventSlot<SensorEvents>,
    #[cfg(test)]
    getter_calls: [Cell<usize>; 4],
}

impl EventStorage {
    pub(crate) fn begin_step(&self) {
        self.body.reset();
        self.contact.reset();
        self.joint.reset();
        self.sensor.reset();
        #[cfg(test)]
        for count in &self.getter_calls {
            count.set(0);
        }
    }

    pub(crate) fn invalidate(&self) {
        self.begin_step();
    }

    fn body(&self, core: &WorldCore) -> Result<&Vec<BodyMoveEvent>> {
        self.body.get_or_materialize(|out| {
            // SAFETY: `CompletedStep` prevents another owner operation from invalidating Box2D's
            // transient buffer until this call has copied it into reusable Rust storage.
            let raw = self.get_body_events(core.id);
            body::capture(out, raw, core)
        })
    }

    fn contact(&self, core: &WorldCore, contact_epoch: ContactEpoch) -> Result<&ContactEvents> {
        self.contact.get_or_materialize(|out| {
            // SAFETY: See `body`; all three arrays belong to this completed step.
            let raw = self.get_contact_events(core.id);
            contact::capture(out, raw, core, contact_epoch)
        })
    }

    fn joint(&self, core: &WorldCore) -> Result<&Vec<JointEvent>> {
        self.joint.get_or_materialize(|out| {
            // SAFETY: See `body`.
            let raw = self.get_joint_events(core.id);
            joint::capture(out, raw, core)
        })
    }

    fn sensor(&self, core: &WorldCore) -> Result<&SensorEvents> {
        self.sensor.get_or_materialize(|out| {
            // SAFETY: See `body`; both arrays belong to this completed step.
            let raw = self.get_sensor_events(core.id);
            sensor::capture(out, raw, core)
        })
    }

    #[inline]
    fn get_body_events(&self, world: ffi::b2WorldId) -> ffi::b2BodyEvents {
        self.record_getter(0);
        unsafe { ffi::b2World_GetBodyEvents(world) }
    }

    #[inline]
    fn get_contact_events(&self, world: ffi::b2WorldId) -> ffi::b2ContactEvents {
        self.record_getter(1);
        unsafe { ffi::b2World_GetContactEvents(world) }
    }

    #[inline]
    fn get_joint_events(&self, world: ffi::b2WorldId) -> ffi::b2JointEvents {
        self.record_getter(2);
        unsafe { ffi::b2World_GetJointEvents(world) }
    }

    #[inline]
    fn get_sensor_events(&self, world: ffi::b2WorldId) -> ffi::b2SensorEvents {
        self.record_getter(3);
        unsafe { ffi::b2World_GetSensorEvents(world) }
    }

    #[inline]
    fn record_getter(&self, family: usize) {
        #[cfg(test)]
        self.getter_calls[family].set(
            self.getter_calls[family]
                .get()
                .checked_add(1)
                .expect("event getter test counter overflow"),
        );
        #[cfg(not(test))]
        let _ = family;
    }

    #[cfg(test)]
    pub(crate) fn getter_calls_for_test(&self) -> [usize; 4] {
        self.getter_calls.each_ref().map(Cell::get)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PublicationPhase {
    Idle,
    Pending,
    Published,
}

/// World-owned state for one lazily materialized completed step.
///
/// Publication and retired output identities are one transaction: a pending publication rolls
/// both back on unwind or recording postflight failure, while a published capability retires both
/// on drop or at the next owner operation.
pub(crate) struct CompletedStepState {
    phase: Cell<PublicationPhase>,
    storage: EventStorage,
}

impl Default for CompletedStepState {
    fn default() -> Self {
        Self {
            phase: Cell::new(PublicationPhase::Idle),
            storage: EventStorage::default(),
        }
    }
}

impl CompletedStepState {
    pub(crate) fn begin_step(&self) {
        debug_assert_eq!(self.phase.get(), PublicationPhase::Idle);
        self.storage.begin_step();
    }

    pub(crate) fn publish_pending<'world>(
        &'world self,
        core: &'world WorldCore,
        contact_epoch: ContactEpoch,
    ) -> PendingStepPublication<'world> {
        let previous = self.phase.replace(PublicationPhase::Pending);
        debug_assert_eq!(previous, PublicationPhase::Idle);
        PendingStepPublication {
            state: self,
            core,
            contact_epoch,
            armed: true,
        }
    }

    pub(crate) fn retire(&self, core: &WorldCore) {
        if self.phase.replace(PublicationPhase::Idle) == PublicationPhase::Idle {
            return;
        }
        core.release_completed_step_outputs();
    }

    pub(crate) fn invalidate(&self, core: &WorldCore) {
        self.retire(core);
        self.storage.invalidate();
    }

    pub(crate) fn storage(&self) -> &EventStorage {
        &self.storage
    }

    #[cfg(test)]
    pub(crate) fn is_active_for_test(&self) -> bool {
        self.phase.get() != PublicationPhase::Idle
    }
}

/// A native step whose transient outputs are not yet visible through Safe Rust.
///
/// Recording keeps this guard alive through its native writer postflight check. Any early return
/// or unwind retires the native outputs before a `CompletedStep` can be constructed.
pub(crate) struct PendingStepPublication<'world> {
    state: &'world CompletedStepState,
    core: &'world WorldCore,
    contact_epoch: ContactEpoch,
    armed: bool,
}

impl PendingStepPublication<'_> {
    pub(crate) fn commit(mut self) -> ContactEpoch {
        let previous = self.state.phase.replace(PublicationPhase::Published);
        debug_assert_eq!(previous, PublicationPhase::Pending);
        self.armed = false;
        self.contact_epoch
    }
}

impl Drop for PendingStepPublication<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.state.retire(self.core);
        }
    }
}

/// The event families produced by one world step that reached native simulation.
///
/// This capability owns a real exclusive borrow of the ordinary world or recording session which
/// stepped it. It exposes no mutable world access, so Box2D's transient event buffers remain valid
/// until the capability ends. Dropping the value retires that state immediately; forgetting it is
/// still harmless because the next owner operation retires it explicitly.
///
/// [`Self::post_step_error`] reports a callback, task, or recording-writer failure discovered
/// after Box2D advanced. An outer error from `World::step` or `RecordingSession::step` therefore
/// always means native simulation was not called.
#[must_use = "inspect the post-step error and event families, or explicitly drop the completed step"]
pub struct CompletedStep<'owner> {
    owner: &'owner mut dyn OwnerAdapter,
    contact_epoch: ContactEpoch,
    post_step_error: Option<crate::Error>,
}

impl core::fmt::Debug for CompletedStep<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CompletedStep")
            .field("post_step_error", &self.post_step_error)
            .finish_non_exhaustive()
    }
}

impl Drop for CompletedStep<'_> {
    fn drop(&mut self) {
        self.owner
            .capability_completed_step()
            .retire(self.owner.capability_core());
    }
}

impl<'owner> CompletedStep<'owner> {
    pub(crate) fn after_validated_step(
        owner: &'owner mut impl OwnerAdapter,
        contact_epoch: ContactEpoch,
        post_step_error: Option<crate::Error>,
    ) -> Self {
        Self {
            owner,
            contact_epoch,
            post_step_error,
        }
    }

    /// Returns a failure discovered after native simulation advanced.
    ///
    /// Event families remain available for ordinary worlds. A recording-writer failure is sticky,
    /// so event materialization through that recording session also returns the writer error.
    pub const fn post_step_error(&self) -> Option<crate::Error> {
        self.post_step_error
    }

    /// Restores the traditional result-first projection.
    ///
    /// On error this drops the completed-step capability and retires its transient outputs. Use
    /// [`Self::post_step_error`] when the caller must still publish events or synchronize state
    /// after native advancement.
    pub fn into_result(self) -> Result<Self> {
        match self.post_step_error {
            Some(error) => Err(error),
            None => Ok(self),
        }
    }

    fn check_owner_status(&self) -> Result<()> {
        // The ordinary capability preflight is an owner boundary and would retire this completed
        // step. For the two sealed owners, postflight is the non-retiring writer-status check.
        // Native event getters and Rust-side mapping are read-only with respect to the owner, so
        // one check before entering that path is sufficient.
        crate::core::callback_state::check_not_in_callback()?;
        self.owner.capability_postflight()
    }

    /// Borrow mapped body-move events, fetching this native family at most once.
    pub fn body_events(&self) -> Result<BodyEvents<'_>> {
        self.check_owner_status()?;
        let events = self
            .owner
            .capability_completed_step()
            .storage()
            .body(self.owner.capability_core())?;
        Ok(BodyEvents::new(events))
    }

    /// Borrow mapped contact events, fetching this native family at most once.
    pub fn contact_events(&self) -> Result<ContactEventsView<'_>> {
        self.check_owner_status()?;
        let events = self
            .owner
            .capability_completed_step()
            .storage()
            .contact(self.owner.capability_core(), self.contact_epoch)?;
        Ok(ContactEventsView::new(events))
    }

    /// Borrow mapped joint events, fetching this native family at most once.
    pub fn joint_events(&self) -> Result<JointEvents<'_>> {
        self.check_owner_status()?;
        let events = self
            .owner
            .capability_completed_step()
            .storage()
            .joint(self.owner.capability_core())?;
        Ok(JointEvents::new(events))
    }

    /// Borrow mapped sensor events, fetching this native family at most once.
    pub fn sensor_events(&self) -> Result<SensorEventsView<'_>> {
        self.check_owner_status()?;
        let events = self
            .owner
            .capability_completed_step()
            .storage()
            .sensor(self.owner.capability_core())?;
        Ok(SensorEventsView::new(events))
    }

    /// Materialize every family and detach an owned snapshot from the world borrow.
    pub fn to_owned(&self) -> Result<StepEventsSnapshot> {
        Ok(StepEventsSnapshot {
            body: self.body_events()?.to_owned()?,
            contact: self.contact_events()?.to_owned()?,
            joint: self.joint_events()?.to_owned()?,
            sensor: self.sensor_events()?.to_owned()?,
        })
    }
}

/// An owned event snapshot which remains valid after the world is mutated again.
#[derive(Clone, Debug, Default)]
pub struct StepEventsSnapshot {
    pub body: Vec<BodyMoveEvent>,
    pub contact: ContactEvents,
    pub joint: Vec<JointEvent>,
    pub sensor: SensorEvents,
}

/// Validate and borrow one transient native pointer/count pair.
#[inline]
unsafe fn ffi_slice<'a, T>(pointer: *const T, count: i32) -> Result<&'a [T]> {
    if count < 0 {
        return Err(Error::NegativeFfiOutputCount { count });
    }
    let len = count as usize;
    if len == 0 {
        return Ok(&[]);
    }
    if pointer.is_null()
        || !pointer.addr().is_multiple_of(core::mem::align_of::<T>())
        || len
            .checked_mul(core::mem::size_of::<T>())
            .filter(|bytes| *bytes <= isize::MAX as usize)
            .and_then(|bytes| pointer.addr().checked_add(bytes))
            .is_none()
    {
        return Err(Error::InvalidNativeEventBuffer);
    }
    // SAFETY: The checked pointer/count pair comes from Box2D's completed-step output and the
    // caller consumes the returned slice before another native owner operation can run.
    Ok(unsafe { core::slice::from_raw_parts(pointer, len) })
}

fn prepare_mapped<T>(out: &mut Vec<T>, len: usize) -> Result<()> {
    out.clear();
    out.try_reserve(len)
        .map_err(|_| Error::FfiOutputAllocationFailed)
}

fn extend_mapped<TRaw, T>(
    out: &mut Vec<T>,
    raw: &[TRaw],
    mut map: impl FnMut(&TRaw) -> Result<T>,
) -> Result<()> {
    debug_assert!(out.capacity() >= raw.len());
    for value in raw {
        match map(value) {
            Ok(value) => out.push(value),
            Err(error) => {
                out.clear();
                return Err(error);
            }
        }
    }
    Ok(())
}

fn clone_into<T: Clone>(source: &[T], out: &mut Vec<T>) -> Result<()> {
    out.clear();
    out.try_reserve(source.len())
        .map_err(|_| Error::FfiOutputAllocationFailed)?;
    out.extend_from_slice(source);
    Ok(())
}

fn to_owned<T: Clone>(source: &[T]) -> Result<Vec<T>> {
    let mut out = Vec::new();
    clone_into(source, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_null_slice_is_valid_and_broken_pairs_are_rejected() {
        let empty = unsafe { ffi_slice::<u32>(core::ptr::null(), 0) }.unwrap();
        assert!(empty.is_empty());
        assert_eq!(
            unsafe { ffi_slice::<u32>(core::ptr::null(), 1) },
            Err(Error::InvalidNativeEventBuffer)
        );
        assert_eq!(
            unsafe { ffi_slice::<u32>(core::ptr::null(), -1) },
            Err(Error::NegativeFfiOutputCount { count: -1 })
        );
    }

    #[test]
    fn failed_materialization_is_cached() {
        let slot = LazyEventSlot::<Vec<BodyMoveEvent>>::default();
        let calls = Cell::new(0);

        let first = slot.get_or_materialize(|_| {
            calls.set(calls.get() + 1);
            Err(Error::InvalidNativeEventBuffer)
        });
        let second = slot.get_or_materialize(|_| panic!("a failed slot must not be fetched twice"));

        assert_eq!(first.unwrap_err(), Error::InvalidNativeEventBuffer);
        assert_eq!(second.unwrap_err(), Error::InvalidNativeEventBuffer);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn panicked_materialization_can_be_retried() {
        let slot = LazyEventSlot::<Vec<BodyMoveEvent>>::default();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = slot.get_or_materialize(|_| -> Result<()> {
                panic!("injected event materialization panic")
            });
        }));

        assert!(panic.is_err());
        assert!(slot.get_or_materialize(|_| Ok(())).unwrap().is_empty());
    }

    #[test]
    fn ignored_step_fetches_no_event_family() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();

        drop(world.step(0.0, 1).unwrap());

        assert_eq!(world.event_storage().getter_calls_for_test(), [0; 4]);
    }

    #[test]
    fn each_requested_family_is_fetched_once() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let completed = world.step(0.0, 1).unwrap();

        assert!(completed.body_events().unwrap().is_empty());
        assert!(completed.body_events().unwrap().is_empty());
        assert!(completed.contact_events().unwrap().is_empty());
        assert!(completed.contact_events().unwrap().is_empty());
        assert!(completed.joint_events().unwrap().is_empty());
        assert!(completed.joint_events().unwrap().is_empty());
        assert!(completed.sensor_events().unwrap().is_empty());
        assert!(completed.sensor_events().unwrap().is_empty());

        assert_eq!(
            completed
                .owner
                .capability_completed_step()
                .storage()
                .getter_calls_for_test(),
            [1; 4]
        );
    }

    #[test]
    fn event_materialization_is_rejected_inside_an_unrelated_callback() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let completed = world.step(0.0, 1).unwrap();

        let result = {
            let _callback = crate::core::callback_state::CallbackGuard::enter();
            completed.body_events()
        };

        assert!(matches!(result, Err(Error::InCallback)));
        assert_eq!(
            completed
                .owner
                .capability_completed_step()
                .storage()
                .getter_calls_for_test(),
            [0; 4]
        );
    }

    #[test]
    fn forgotten_completed_step_is_retired_by_the_next_owner_boundary() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let completed = world.step(0.0, 1).unwrap();
        core::mem::forget(completed);
        assert!(world.completed_step_active_for_test());

        drop(world.step(0.0, 1).unwrap());
        assert!(!world.completed_step_active_for_test());
    }

    #[test]
    fn dropped_completed_step_is_retired_immediately() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let completed = world.step(0.0, 1).unwrap();
        assert!(
            completed
                .owner
                .capability_completed_step()
                .is_active_for_test()
        );

        drop(completed);

        assert!(!world.completed_step_active_for_test());
    }

    #[test]
    fn owned_events_remain_detached_after_the_next_step() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let owned = world.step(0.0, 1).unwrap().to_owned().unwrap();

        drop(world.step(0.0, 1).unwrap());

        assert!(owned.body.is_empty());
        assert!(owned.contact.begin.is_empty());
        assert!(owned.joint.is_empty());
        assert!(owned.sensor.begin.is_empty());
    }

    #[test]
    fn borrowed_view_can_reuse_caller_owned_capacity() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let completed = world.step(0.0, 1).unwrap();
        let events = completed.body_events().unwrap();
        let mut owned = Vec::with_capacity(8);
        let allocation = owned.as_ptr();

        events.clone_into(&mut owned).unwrap();

        assert_eq!(owned.as_ptr(), allocation);
        assert!(owned.is_empty());
    }

    #[test]
    fn mapped_and_cloned_buffers_grow_then_reuse_capacity() {
        let source = (0_u32..10).collect::<Vec<_>>();

        let mut mapped = Vec::with_capacity(8);
        prepare_mapped(&mut mapped, source.len()).unwrap();
        extend_mapped(&mut mapped, &source, |value| Ok(*value)).unwrap();
        assert_eq!(mapped, source);
        let mapped_allocation = mapped.as_ptr();
        prepare_mapped(&mut mapped, source.len()).unwrap();
        extend_mapped(&mut mapped, &source, |value| Ok(*value)).unwrap();
        assert_eq!(mapped.as_ptr(), mapped_allocation);

        let mut cloned = Vec::with_capacity(8);
        clone_into(&source, &mut cloned).unwrap();
        assert_eq!(cloned, source);
        let cloned_allocation = cloned.as_ptr();
        clone_into(&source, &mut cloned).unwrap();
        assert_eq!(cloned.as_ptr(), cloned_allocation);
    }
}
