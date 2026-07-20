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

#[inline]
fn map_snapshot_into<TRaw, T>(out: &mut Vec<T>, slice: &[TRaw], map: impl FnMut(&TRaw) -> T) {
    out.clear();
    if out.capacity() < slice.len() {
        out.reserve(slice.len());
    }
    out.extend(slice.iter().map(map));
}

mod body;
mod contact;
mod joint;
mod sensor;

#[derive(Default)]
struct EventSnapshot {
    body: Vec<body::BodyMoveEvent>,
    contact: contact::ContactEvents,
    joint: Vec<joint::JointEvent>,
    sensor: sensor::SensorEvents,
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
    pub(crate) fn capture_completed_step(
        &self,
        world: ffi::b2WorldId,
        brand: IdBrand,
        contact_epoch: ContactEpoch,
    ) {
        // Detach the staging buffers so a panic while validating or allocating cannot leave the
        // published snapshot containing event classes from different steps.
        let mut next = {
            let mut state = self.state.borrow_mut();
            core::mem::take(&mut state.staging)
        };

        // No Box2D mutation may occur between these reads. `World::step` calls this immediately
        // after b2World_Step returns and before deferred destruction is flushed.
        body::capture_native_events_into(world, brand, &mut next.body);
        contact::capture_native_events_into(world, brand, contact_epoch, &mut next.contact);
        joint::capture_native_events_into(world, brand, &mut next.joint);
        sensor::capture_native_events_into(world, brand, &mut next.sensor);

        let mut state = self.state.borrow_mut();
        core::mem::swap(&mut state.current, &mut next);
        state.staging = next;
    }

    fn snapshot(&self) -> Ref<'_, EventSnapshot> {
        Ref::map(self.state.borrow(), |state| &state.current)
    }
}

pub use body::BodyMoveEvent;
pub use contact::{ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactHitEvent};
pub use joint::JointEvent;
pub use sensor::{SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents};

#[cfg(test)]
mod tests {
    use crate::{ApiError, ContactEvents, SensorEvents, World, WorldDef};

    #[test]
    fn snapshot_map_grows_from_existing_capacity_to_the_full_input() {
        let input = [5_u8; 10];
        let mut out = Vec::with_capacity(8);

        super::map_snapshot_into(&mut out, &input, |value| u16::from(*value));

        assert!(out.capacity() >= 10);
        assert_eq!(out, vec![5_u16; 10]);
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
