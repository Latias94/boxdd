//! Event snapshots and zero-copy visitors.
//!
//! - Snapshot getters like `body_events`, `sensor_events`, `contact_events`, `joint_events`
//!   copy event data into owned Rust collections and are safe to keep after stepping.
//! - Reusable-buffer snapshot getters like `*_events_into` reuse caller-owned storage for the same
//!   owned event data.
//! - Zero-copy visitors like `with_*` variants pass FFI slices valid only for the
//!   duration of the closure and current step.
//! - Owned snapshot getters are available on both [`crate::World`] and `WorldHandle`.
//! - Borrowed zero-copy views and raw event-buffer access intentionally stay on [`crate::World`]:
//!   they are tied to completed-step world buffers and the world's deferred-destroy flush semantics.

#[inline]
fn map_snapshot_into<TRaw, T>(out: &mut Vec<T>, slice: &[TRaw], map: impl FnMut(&TRaw) -> T) {
    out.clear();
    if out.capacity() < slice.len() {
        out.reserve(slice.len() - out.capacity());
    }
    out.extend(slice.iter().map(map));
}

mod body;
mod contact;
mod joint;
mod sensor;

pub use body::BodyMoveEvent;
pub use contact::{ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactHitEvent};
pub use joint::JointEvent;
pub use sensor::{SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents};

#[cfg(test)]
mod tests {
    use crate::{ApiError, ContactEvents, SensorEvents, World, WorldDef};

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
