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

#[derive(Default)]
pub(super) struct BodyEventSlot {
    pub(super) values: Vec<BodyMoveEvent>,
}

impl BodyEventSlot {
    pub(super) fn capture_into(mut self, raw: ffi::b2BodyEvents, brand: IdBrand) -> Self {
        // SAFETY: The raw container belongs to the completed step and is consumed before any
        // subsequent world mutation.
        self.values = unsafe {
            super::capture_ffi_vec(self.values, raw.moveEvents, raw.moveCount, |event| {
                BodyMoveEvent::from_raw_in(brand, *event)
            })
        };
        self
    }

    pub(super) fn to_owned(&self) -> Vec<BodyMoveEvent> {
        std::clone::Clone::clone(&self.values)
    }

    fn clone_into(&self, out: &mut Vec<BodyMoveEvent>) {
        super::map_snapshot_into(out, &self.values, Clone::clone);
    }

    fn iter(&self) -> core::slice::Iter<'_, BodyMoveEvent> {
        self.values.iter()
    }
}

fn body_events_into_impl(cache: &super::EventCache, out: &mut Vec<BodyMoveEvent>) {
    let snapshot = cache.snapshot();
    BodyEventSlot::clone_into(&snapshot.body, out);
}

fn body_events_snapshot_impl(cache: &super::EventCache) -> Vec<BodyMoveEvent> {
    BodyEventSlot::to_owned(&cache.snapshot().body)
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
            // SAFETY: The enclosing raw-view contract keeps the native buffer valid for `f`.
            unsafe { super::with_ffi_slice(raw.moveEvents, raw.moveCount, f) }
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
            // SAFETY: The enclosing raw-view contract keeps the native buffer valid for `f`.
            unsafe { super::with_ffi_slice(raw.moveEvents, raw.moveCount, f) }
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
                iter: BodyEventSlot::iter(&snapshot.body),
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
                iter: BodyEventSlot::iter(&snapshot.body),
            })
        })
    }
}
