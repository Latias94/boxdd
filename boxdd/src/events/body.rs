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

fn body_events_into_impl(world: ffi::b2WorldId, out: &mut Vec<BodyMoveEvent>) {
    let raw = unsafe { ffi::b2World_GetBodyEvents(world) };
    let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
    } else {
        &[][..]
    };
    super::map_snapshot_into(out, slice, |e| BodyMoveEvent {
        body_id: e.bodyId,
        transform: Transform::from(e.transform),
        fell_asleep: e.fellAsleep,
    });
}

impl World {
    pub fn body_events(&self) -> Vec<BodyMoveEvent> {
        crate::core::callback_state::assert_not_in_callback();
        let mut out = Vec::new();
        body_events_into_impl(self.raw(), &mut out);
        out
    }

    pub fn body_events_into(&self, out: &mut Vec<BodyMoveEvent>) {
        crate::core::callback_state::assert_not_in_callback();
        body_events_into_impl(self.raw(), out);
    }

    pub fn try_body_events(&self) -> crate::error::ApiResult<Vec<BodyMoveEvent>> {
        crate::core::callback_state::check_not_in_callback()?;
        let mut out = Vec::new();
        body_events_into_impl(self.raw(), &mut out);
        Ok(out)
    }

    pub fn try_body_events_into(
        &self,
        out: &mut Vec<BodyMoveEvent>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        body_events_into_impl(self.raw(), out);
        Ok(())
    }

    // Zero-copy visitor (closure style). Data is only valid within the call.
    /// Low-level raw view over body events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// The returned slice borrows internal Box2D buffers. While `f` runs, you must not perform
    /// any operation that can mutate those buffers (e.g. stepping the world or destroying bodies).
    ///
    /// Dropping `Owned*` handles inside `f` is OK; destruction is deferred until after this call.
    pub unsafe fn with_body_events_raw<T>(
        &self,
        f: impl FnOnce(&[ffi::b2BodyMoveEvent]) -> T,
    ) -> T {
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

    /// Low-level raw view over body events with recoverable callback-lock checking.
    ///
    /// # Safety
    /// Same safety contract as `with_body_events_raw`.
    pub unsafe fn try_with_body_events_raw<T>(
        &self,
        f: impl FnOnce(&[ffi::b2BodyMoveEvent]) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
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

    /// Zero-copy view over body move events with recoverable callback-lock checking.
    pub fn try_with_body_events_view<T>(
        &self,
        f: impl FnOnce(BodyMoveIter<'_>) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
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
