use crate::Transform;
use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct BodyMoveEvent {
    pub body_id: BodyId,
    pub transform: Transform,
    pub fell_asleep: bool,
}

/// Zero-copy view wrapper for a body move event.
/// Borrowed data is valid only within the closure passed to
/// `with_body_events_view`.
#[derive(Copy, Clone)]
pub struct BodyMove<'a>(&'a ffi::b2BodyMoveEvent);
impl<'a> BodyMove<'a> {
    pub fn body_id(&self) -> BodyId {
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
        crate::core::callback_state::assert_not_in_callback();
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
    /// Low-level raw view over body events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// The returned slice borrows internal Box2D buffers. While `f` runs, you must not perform
    /// any operation that can mutate those buffers (e.g. stepping the world or destroying bodies).
    ///
    /// Dropping `Owned*` handles inside `f` is OK; destruction is deferred until after this call.
    pub unsafe fn with_body_events<T>(&self, f: impl FnOnce(&[ffi::b2BodyMoveEvent]) -> T) -> T {
        self.with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetBodyEvents(self.raw()) };
            let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
            } else {
                &[][..]
            };
            f(slice)
        })
    }

    /// Zero-copy view over body move events without exposing raw FFI types.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes/joints immediately;
    /// the destruction is deferred until after the view ends to keep the borrowed buffers valid.
    ///
    /// Example
    /// ```rust
    /// use boxdd::prelude::*;
    /// let mut world = World::new(WorldDef::default()).unwrap();
    /// world.with_body_events_view(|it| {
    ///     for e in it { let _ = (e.body_id(), e.fell_asleep()); }
    /// });
    /// ```
    ///
    pub fn with_body_events_view<T>(&self, f: impl FnOnce(BodyMoveIter<'_>) -> T) -> T {
        self.with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetBodyEvents(self.raw()) };
            let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
            } else {
                &[][..]
            };
            f(BodyMoveIter(slice.iter()))
        })
    }
}
