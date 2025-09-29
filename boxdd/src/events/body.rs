use crate::Transform;
use crate::world::World;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct BodyMoveEvent {
    pub body_id: ffi::b2BodyId,
    pub transform: Transform,
    pub fell_asleep: bool,
}

/// Zero-copy view wrapper for a body move event.
/// Borrowed data is valid only within the closure passed to
/// `with_body_events_view`.
#[derive(Copy, Clone)]
pub struct BodyMove<'a>(&'a ffi::b2BodyMoveEvent);
impl<'a> BodyMove<'a> {
    pub fn body_id(&self) -> ffi::b2BodyId {
        self.0.bodyId
    }
    pub fn transform(&self) -> Transform {
        Transform::from(self.0.transform)
    }
    pub fn fell_asleep(&self) -> bool {
        self.0.fellAsleep
    }
}

pub struct BodyMoveIter<'a>(core::slice::Iter<'a, ffi::b2BodyMoveEvent>);
impl<'a> Iterator for BodyMoveIter<'a> {
    type Item = BodyMove<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(BodyMove)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
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

    /// Zero-copy safe view over body move events without exposing raw FFI types.
    /// Borrowed data is valid only within the closure; do not store references.
    ///
    /// Example
    /// ```rust
    /// use boxdd::prelude::*;
    /// let mut world = World::new(WorldDef::default()).unwrap();
    /// world.with_body_events_view(|it| {
    ///     for e in it { let _ = (e.body_id(), e.fell_asleep()); }
    /// });
    /// ```
    pub fn with_body_events_view<T>(&self, f: impl FnOnce(BodyMoveIter<'_>) -> T) -> T {
        let raw = unsafe { ffi::b2World_GetBodyEvents(self.raw()) };
        let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
            unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
        } else {
            &[][..]
        };
        f(BodyMoveIter(slice.iter()))
    }
}
