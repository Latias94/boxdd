use crate::id::IdBrand;
use crate::types::{BodyId, WorldTransform};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct BodyMoveEvent {
    pub body_id: BodyId,
    pub transform: WorldTransform,
    pub fell_asleep: bool,
}

impl BodyMoveEvent {
    /// Copy a native body-move event into an owned Rust value.
    pub(crate) fn from_raw_in(brand: IdBrand, raw: ffi::b2BodyMoveEvent) -> Self {
        Self {
            body_id: brand
                .try_body(raw.bodyId)
                .expect("Box2D body event contained an invalid body id"),
            transform: WorldTransform::from_raw(raw.transform),
            fell_asleep: raw.fellAsleep,
        }
    }
}

/// Zero-copy view wrapper for a body move event.
/// Borrowed data is valid only within the closure passed to
/// `with_body_events_view`.
#[derive(Copy, Clone)]
pub struct BodyMove<'a> {
    event: &'a BodyMoveEvent,
}
impl<'a> BodyMove<'a> {
    pub fn body_id(&self) -> BodyId {
        self.event.body_id
    }
    pub fn transform(&self) -> WorldTransform {
        self.event.transform
    }
    pub fn fell_asleep(&self) -> bool {
        self.event.fell_asleep
    }
}

pub struct BodyMoveIter<'a> {
    iter: core::slice::Iter<'a, BodyMoveEvent>,
}
impl<'a> Iterator for BodyMoveIter<'a> {
    type Item = BodyMove<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| BodyMove { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(super) fn capture_native_events_into(
    world: ffi::b2WorldId,
    brand: IdBrand,
    out: &mut Vec<BodyMoveEvent>,
) {
    let raw = unsafe { ffi::b2World_GetBodyEvents(world) };
    let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
    } else {
        &[][..]
    };
    super::map_snapshot_into(out, slice, |event| {
        BodyMoveEvent::from_raw_in(brand, *event)
    });
}

fn body_events_into_impl(cache: &super::EventCache, out: &mut Vec<BodyMoveEvent>) {
    let snapshot = cache.snapshot();
    super::map_snapshot_into(out, &snapshot.body, Clone::clone);
}

fn body_events_snapshot_impl(cache: &super::EventCache) -> Vec<BodyMoveEvent> {
    cache.snapshot().body.clone()
}

impl World {
    pub fn body_events(&self) -> Vec<BodyMoveEvent> {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        body_events_snapshot_impl(self.event_cache())
    }

    pub fn body_events_into(&self, out: &mut Vec<BodyMoveEvent>) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        body_events_into_impl(self.event_cache(), out);
    }

    pub fn try_body_events(&self) -> crate::error::ApiResult<Vec<BodyMoveEvent>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(body_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_body_events_into(
        &self,
        out: &mut Vec<BodyMoveEvent>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        body_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl WorldHandle {
    pub fn body_events(&self) -> Vec<BodyMoveEvent> {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        body_events_snapshot_impl(self.event_cache())
    }

    pub fn body_events_into(&self, out: &mut Vec<BodyMoveEvent>) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        body_events_into_impl(self.event_cache(), out);
    }

    pub fn try_body_events(&self) -> crate::error::ApiResult<Vec<BodyMoveEvent>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(body_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_body_events_into(
        &self,
        out: &mut Vec<BodyMoveEvent>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        body_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl World {
    // Zero-copy visitor (closure style). Data is only valid within the call.
    /// Low-level raw view over body events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// Call this immediately after the latest world step and before any operation that may mutate
    /// the world. The returned slice borrows transient Box2D storage. While `f` runs, you must not
    /// perform any operation that can mutate that storage.
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

    /// Zero-copy view over the Rust-owned completed-step body events.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes/joints immediately;
    /// the destruction is deferred until after the view ends to preserve existing event-view
    /// ordering semantics.
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
            let snapshot = self.event_cache().snapshot();
            f(BodyMoveIter {
                iter: snapshot.body.iter(),
            })
        })
    }

    /// Zero-copy view over body move events with recoverable callback-lock checking.
    pub fn try_with_body_events_view<T>(
        &self,
        f: impl FnOnce(BodyMoveIter<'_>) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(BodyMoveIter {
                iter: snapshot.body.iter(),
            })
        })
    }
}
