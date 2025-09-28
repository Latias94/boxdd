use crate::Transform;
use crate::world::World;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct BodyMoveEvent {
    pub body_id: ffi::b2BodyId,
    pub transform: Transform,
    pub fell_asleep: bool,
}

impl World {
    pub fn body_events(&self) -> Vec<BodyMoveEvent> {
        let raw = unsafe { ffi::b2World_GetBodyEvents(self.raw()) };
        if raw.moveCount <= 0 || raw.moveEvents.is_null() {
            return Vec::new();
        }
        let slice = unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) };
        slice
            .iter()
            .map(|e| BodyMoveEvent {
                body_id: e.bodyId,
                transform: Transform::from(e.transform),
                fell_asleep: e.fellAsleep,
            })
            .collect()
    }

    // Zero-copy visitor (closure style). Data is only valid within the call.
    pub fn with_body_events<T>(&self, f: impl FnOnce(&[ffi::b2BodyMoveEvent]) -> T) -> T {
        let raw = unsafe { ffi::b2World_GetBodyEvents(self.raw()) };
        let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
            unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
        } else {
            &[][..]
        };
        f(slice)
    }
}
