//! Event snapshots and zero-copy visitors.
//!
//! - Snapshot getters like `body_events`, `sensor_events`, `contact_events`, `joint_events`
//!   clone the Rust-owned snapshot captured when the latest world step completed.
//! - Reusable-buffer snapshot getters like `*_events_into` reuse caller-owned storage for the same
//!   owned event data.
//! - Safe zero-copy visitors borrow the Rust-owned completed-step snapshot for the duration of the
//!   closure. They never dereference Box2D's transient event buffers after world mutation.
//! - Owned snapshot getters are available on both [`crate::World`] and `WorldHandle`.
//! - Borrowed zero-copy views and raw event-buffer access intentionally stay on [`crate::World`]:
//!   the safe views preserve the world's deferred-destroy flush semantics, while unsafe raw access
//!   remains tied directly to Box2D's transient buffers.

use crate::id::{ContactEpoch, IdBrand};
use boxdd_sys::ffi;
use std::cell::{Ref, RefCell};

/// Run a closure over a native output array after validating its count/pointer pair.
///
/// Box2D's event APIs guarantee a non-negative count and a non-null pointer for non-empty
/// arrays. Treating a broken pair as an empty array would hide an ABI violation and can turn
/// a later ID conversion into misleading state, so the checked boundary fails immediately.
#[inline]
pub(super) unsafe fn with_ffi_slice<T, R>(
    pointer: *const T,
    count: i32,
    f: impl FnOnce(&[T]) -> R,
) -> R {
    let len = checked_ffi_slice_len(
        pointer.cast::<u8>(),
        count,
        core::mem::size_of::<T>(),
        core::mem::align_of::<T>(),
    );
    if len == 0 {
        return f(&[]);
    }
    // SAFETY: The caller is crossing the documented Box2D event-output boundary. The count and
    // pointer invariants were checked above, and the closure cannot outlive this call. The caller
    // remains responsible for the allocation, initialization, and aliasing guarantees that cannot
    // be reconstructed from a C pointer/count pair.
    let slice = unsafe { core::slice::from_raw_parts(pointer, len) };
    f(slice)
}

#[inline]
fn checked_ffi_slice_len(
    pointer: *const u8,
    count: i32,
    element_size: usize,
    element_align: usize,
) -> usize {
    assert!(count >= 0, "Box2D returned a negative event count");
    let len = count as usize;
    if len == 0 {
        return 0;
    }

    assert!(!pointer.is_null(), "Box2D returned a null event pointer");
    assert!(
        pointer.addr().is_multiple_of(element_align),
        "Box2D returned a misaligned event pointer"
    );
    let byte_span = len
        .checked_mul(element_size)
        .filter(|span| *span <= isize::MAX as usize)
        .expect("Box2D returned an event array larger than Rust permits");
    assert!(
        pointer.addr().checked_add(byte_span).is_some(),
        "Box2D returned an event array whose address range wraps"
    );
    len
}

#[inline]
fn map_snapshot_into<TRaw, T>(out: &mut Vec<T>, slice: &[TRaw], map: impl FnMut(&TRaw) -> T) {
    out.clear();
    if out.capacity() < slice.len() {
        out.reserve(slice.len());
    }
    out.extend(slice.iter().map(map));
}

/// Refill an owned event vector from one checked native pointer/count pair.
///
/// Taking and returning the vector makes the ownership transfer explicit to the typed event slot,
/// while `map_snapshot_into` preserves its allocation across completed steps.
#[inline]
unsafe fn capture_ffi_vec<TRaw, T>(
    mut out: Vec<T>,
    pointer: *const TRaw,
    count: i32,
    map: impl FnMut(&TRaw) -> T,
) -> Vec<T> {
    // SAFETY: The caller provides one Box2D-owned event pointer/count pair and consumes every
    // element before the next world mutation.
    unsafe {
        with_ffi_slice(pointer, count, |slice| {
            map_snapshot_into(&mut out, slice, map);
        });
    }
    out
}

mod body;
mod contact;
mod joint;
mod sensor;

#[derive(Default)]
struct EventSnapshot {
    body: body::BodyEventSlot,
    contact: contact::ContactEvents,
    joint: joint::JointEventSlot,
    sensor: sensor::SensorEvents,
}

/// The four native event containers observed at one completed-step boundary.
///
/// The pointers remain transient; this value is consumed immediately by `EventSnapshot` before
/// any deferred destruction can mutate the Box2D world.
struct RawEventSnapshot {
    body: ffi::b2BodyEvents,
    contact: ffi::b2ContactEvents,
    joint: ffi::b2JointEvents,
    sensor: ffi::b2SensorEvents,
}

impl RawEventSnapshot {
    fn capture(world: ffi::b2WorldId) -> Self {
        // SAFETY: `World::step` invokes this after `b2World_Step` and before any subsequent world
        // mutation. Each returned container is copied by value and consumed synchronously.
        unsafe {
            Self {
                body: ffi::b2World_GetBodyEvents(world),
                contact: ffi::b2World_GetContactEvents(world),
                joint: ffi::b2World_GetJointEvents(world),
                sensor: ffi::b2World_GetSensorEvents(world),
            }
        }
    }
}

impl EventSnapshot {
    fn capture_into(
        mut self,
        raw: RawEventSnapshot,
        brand: IdBrand,
        contact_epoch: ContactEpoch,
    ) -> Self {
        self.body = body::BodyEventSlot::capture_into(self.body, raw.body, brand);
        self.contact =
            contact::ContactEvents::capture_into(self.contact, raw.contact, brand, contact_epoch);
        self.joint = joint::JointEventSlot::capture_into(self.joint, raw.joint, brand);
        self.sensor = sensor::SensorEvents::capture_into(self.sensor, raw.sensor, brand);
        self
    }
}

/// Shared cache of the event data from the latest completed world step.
///
/// Box2D owns its event arrays and may invalidate them as soon as the world is mutated. Keeping the
/// copy at the `World` boundary lets both `World` and `WorldHandle` expose safe event APIs without
/// extending the native buffers' lifetime.
#[derive(Default)]
pub(crate) struct EventCache {
    state: RefCell<EventCacheState>,
}

#[derive(Default)]
struct EventCacheState {
    current: EventSnapshot,
    staging: EventSnapshot,
}

impl EventCache {
    pub(crate) fn invalidate(&self) {
        let mut state = self.state.borrow_mut();
        state.current = EventSnapshot::default();
        state.staging = EventSnapshot::default();
    }

    fn snapshot(&self) -> Ref<'_, EventSnapshot> {
        std::cell::Ref::map(self.state.borrow(), |state| &state.current)
    }

    fn take_staging(&self) -> EventSnapshot {
        core::mem::take(&mut self.state.borrow_mut().staging)
    }

    fn publish(&self, mut next: EventSnapshot) {
        let mut state = self.state.borrow_mut();
        std::mem::swap(&mut state.current, &mut next);
        state.staging = next;
    }
}

pub(crate) fn capture_completed_step(
    cache: &EventCache,
    world: ffi::b2WorldId,
    brand: IdBrand,
    contact_epoch: ContactEpoch,
) {
    // No Box2D mutation may occur between these reads. `World::step` calls this immediately
    // after b2World_Step returns and before deferred destruction is flushed.
    // Detaching staging first preserves capacity reuse. `publish` is reached only after every
    // typed slot has completed, so validation/allocation panics cannot expose a mixed-step view.
    cache.publish(EventSnapshot::capture_into(
        cache.take_staging(),
        RawEventSnapshot::capture(world),
        brand,
        contact_epoch,
    ));
}

pub use body::BodyMoveEvent;
pub use contact::{ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactHitEvent};
pub use joint::JointEvent;
pub use sensor::{SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents};

#[cfg(test)]
mod tests {
    use crate::{ApiError, ContactEvents, SensorEvents, World, WorldDef};

    fn empty_raw_snapshot() -> super::RawEventSnapshot {
        super::RawEventSnapshot {
            body: super::ffi::b2BodyEvents {
                moveEvents: core::ptr::null_mut(),
                moveCount: 0,
            },
            contact: super::ffi::b2ContactEvents {
                beginEvents: core::ptr::null_mut(),
                endEvents: core::ptr::null_mut(),
                hitEvents: core::ptr::null_mut(),
                beginCount: 0,
                endCount: 0,
                hitCount: 0,
            },
            joint: super::ffi::b2JointEvents {
                jointEvents: core::ptr::null_mut(),
                count: 0,
            },
            sensor: super::ffi::b2SensorEvents {
                beginEvents: core::ptr::null_mut(),
                endEvents: core::ptr::null_mut(),
                beginCount: 0,
                endCount: 0,
            },
        }
    }

    #[test]
    fn snapshot_map_grows_from_existing_capacity_to_the_full_input() {
        let input = [5_u8; 10];
        let mut out = Vec::with_capacity(8);

        super::map_snapshot_into(&mut out, &input, |value| u16::from(*value));

        assert!(out.capacity() >= 10);
        assert_eq!(out, vec![5_u16; 10]);
    }

    #[test]
    fn ffi_slice_accepts_empty_null_and_rejects_broken_pairs() {
        let empty =
            unsafe { super::with_ffi_slice::<u32, _>(core::ptr::null(), 0, |slice| slice.len()) };
        assert_eq!(empty, 0);

        let values = [1_u32, 2, 3];
        let sum = unsafe {
            super::with_ffi_slice(values.as_ptr(), values.len() as i32, |slice| {
                slice.iter().sum::<u32>()
            })
        };
        assert_eq!(sum, 6);

        let null_non_empty = std::panic::catch_unwind(|| unsafe {
            super::with_ffi_slice::<u32, _>(core::ptr::null(), 1, |_| ())
        });
        assert!(null_non_empty.is_err());

        let negative = std::panic::catch_unwind(|| unsafe {
            super::with_ffi_slice::<u32, _>(core::ptr::NonNull::dangling().as_ptr(), -1, |_| ())
        });
        assert!(negative.is_err());

        let aligned = core::ptr::NonNull::<u32>::dangling().as_ptr();
        let misaligned = aligned.cast::<u8>().wrapping_add(1).cast::<u32>();
        let misaligned_non_empty = std::panic::catch_unwind(|| unsafe {
            super::with_ffi_slice::<u32, _>(misaligned, 1, |_| ())
        });
        assert!(misaligned_non_empty.is_err());

        let oversized = std::panic::catch_unwind(|| {
            super::checked_ffi_slice_len(
                core::ptr::NonNull::<u64>::dangling().as_ptr().cast(),
                2,
                usize::MAX,
                core::mem::align_of::<u64>(),
            )
        });
        assert!(oversized.is_err());
    }

    #[test]
    fn typed_event_capture_reuses_every_staging_allocation() {
        let world = World::new(WorldDef::default()).unwrap();
        let staging = super::EventSnapshot {
            body: super::body::BodyEventSlot {
                values: Vec::with_capacity(3),
            },
            contact: ContactEvents {
                begin: Vec::with_capacity(5),
                end: Vec::with_capacity(7),
                hit: Vec::with_capacity(11),
            },
            joint: super::joint::JointEventSlot {
                values: Vec::with_capacity(13),
            },
            sensor: SensorEvents {
                begin: Vec::with_capacity(17),
                end: Vec::with_capacity(19),
            },
        };
        let pointers = (
            staging.body.values.as_ptr(),
            staging.contact.begin.as_ptr(),
            staging.contact.end.as_ptr(),
            staging.contact.hit.as_ptr(),
            staging.joint.values.as_ptr(),
            staging.sensor.begin.as_ptr(),
            staging.sensor.end.as_ptr(),
        );

        let captured = super::EventSnapshot::capture_into(
            staging,
            empty_raw_snapshot(),
            world.brand(),
            world.core().contact_epoch(),
        );

        assert_eq!(captured.body.values.as_ptr(), pointers.0);
        assert_eq!(captured.contact.begin.as_ptr(), pointers.1);
        assert_eq!(captured.contact.end.as_ptr(), pointers.2);
        assert_eq!(captured.contact.hit.as_ptr(), pointers.3);
        assert_eq!(captured.joint.values.as_ptr(), pointers.4);
        assert_eq!(captured.sensor.begin.as_ptr(), pointers.5);
        assert_eq!(captured.sensor.end.as_ptr(), pointers.6);
    }

    #[test]
    fn failed_typed_event_capture_keeps_published_snapshot() {
        let world = World::new(WorldDef::default()).unwrap();
        let cache = super::EventCache::default();
        {
            let mut state = cache.state.borrow_mut();
            state.current.contact.begin = Vec::with_capacity(5);
            state.current.sensor.end = Vec::with_capacity(7);
            state.staging.body.values = Vec::with_capacity(11);
        }
        let published = {
            let state = cache.state.borrow();
            (
                state.current.contact.begin.as_ptr(),
                state.current.contact.begin.capacity(),
                state.current.sensor.end.as_ptr(),
                state.current.sensor.end.capacity(),
            )
        };
        let mut invalid = empty_raw_snapshot();
        invalid.body.moveCount = 1;

        let capture = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cache.publish(super::EventSnapshot::capture_into(
                cache.take_staging(),
                invalid,
                world.brand(),
                world.core().contact_epoch(),
            ));
        }));

        assert!(capture.is_err());
        let state = cache.state.borrow();
        assert_eq!(state.current.contact.begin.as_ptr(), published.0);
        assert_eq!(state.current.contact.begin.capacity(), published.1);
        assert_eq!(state.current.sensor.end.as_ptr(), published.2);
        assert_eq!(state.current.sensor.end.capacity(), published.3);
    }

    #[test]
    fn try_event_snapshot_apis_return_in_callback() {
        let world = World::new(WorldDef::default()).unwrap();
        let handle = world.handle();
        let mut body_events = Vec::new();
        let mut joint_events = Vec::new();
        let mut contact_events = ContactEvents::default();
        let mut sensor_events = SensorEvents::default();
        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(world.try_body_events().unwrap_err(), ApiError::InCallback);
        assert_eq!(
            world.try_body_events_into(&mut body_events).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world.try_contact_events().unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world
                .try_contact_events_into(&mut contact_events)
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(world.try_sensor_events().unwrap_err(), ApiError::InCallback);
        assert_eq!(
            world
                .try_sensor_events_into(&mut sensor_events)
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(world.try_joint_events().unwrap_err(), ApiError::InCallback);
        assert_eq!(
            world.try_joint_events_into(&mut joint_events).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(handle.try_body_events().unwrap_err(), ApiError::InCallback);
        assert_eq!(
            handle.try_body_events_into(&mut body_events).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            handle.try_contact_events().unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            handle
                .try_contact_events_into(&mut contact_events)
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            handle.try_sensor_events().unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            handle
                .try_sensor_events_into(&mut sensor_events)
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(handle.try_joint_events().unwrap_err(), ApiError::InCallback);
        assert_eq!(
            handle.try_joint_events_into(&mut joint_events).unwrap_err(),
            ApiError::InCallback
        );
    }

    #[test]
    fn try_event_view_apis_return_in_callback() {
        let world = World::new(WorldDef::default()).unwrap();
        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world
                .try_with_body_events_view(|it| it.count())
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world
                .try_with_contact_events_view(|begin, end, hit| begin.count()
                    + end.count()
                    + hit.count())
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world
                .try_with_sensor_events_view(|begin, end| begin.count() + end.count())
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world
                .try_with_joint_events_view(|it| it.count())
                .unwrap_err(),
            ApiError::InCallback
        );
    }

    #[test]
    fn try_event_raw_apis_return_in_callback() {
        let world = World::new(WorldDef::default()).unwrap();
        let _g = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            unsafe { world.try_with_body_events_raw(|events| events.len()) }.unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            unsafe {
                world.try_with_contact_events_raw(|begin, end, hit| {
                    begin.len() + end.len() + hit.len()
                })
            }
            .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            unsafe { world.try_with_sensor_events_raw(|begin, end| begin.len() + end.len()) }
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            unsafe { world.try_with_joint_events_raw(|events| events.len()) }.unwrap_err(),
            ApiError::InCallback
        );
    }
}
