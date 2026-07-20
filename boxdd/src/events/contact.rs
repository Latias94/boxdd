use crate::id::{ContactEpoch, IdBrand};
use crate::types::{ContactId, Position, ShapeId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

/// Zero-copy view wrappers for contact events.
/// These types borrow the Rust-owned completed-step snapshot.
/// The borrowed data is only valid for the duration of the closure passed
/// to `with_contact_events_view`.
#[derive(Copy, Clone)]
pub struct ContactBeginTouch<'a> {
    event: &'a ContactBeginTouchEvent,
}
impl<'a> ContactBeginTouch<'a> {
    pub fn shape_a(&self) -> ShapeId {
        self.event.shape_a
    }
    pub fn shape_b(&self) -> ShapeId {
        self.event.shape_b
    }
    pub fn contact_id(&self) -> ContactId {
        self.event.contact_id
    }
}

#[derive(Copy, Clone)]
pub struct ContactEndTouch<'a> {
    event: &'a ContactEndTouchEvent,
}
impl<'a> ContactEndTouch<'a> {
    pub fn shape_a(&self) -> ShapeId {
        self.event.shape_a
    }
    pub fn shape_b(&self) -> ShapeId {
        self.event.shape_b
    }
    pub fn contact_id(&self) -> ContactId {
        self.event.contact_id
    }
}

#[derive(Copy, Clone)]
pub struct ContactHit<'a> {
    event: &'a ContactHitEvent,
}
impl<'a> ContactHit<'a> {
    pub fn shape_a(&self) -> ShapeId {
        self.event.shape_a
    }
    pub fn shape_b(&self) -> ShapeId {
        self.event.shape_b
    }
    pub fn contact_id(&self) -> ContactId {
        self.event.contact_id
    }
    pub fn point(&self) -> Position {
        self.event.point
    }
    pub fn normal(&self) -> Vec2 {
        self.event.normal
    }
    pub fn approach_speed(&self) -> f32 {
        self.event.approach_speed
    }
}

pub struct BeginIter<'a> {
    iter: core::slice::Iter<'a, ContactBeginTouchEvent>,
}
impl<'a> Iterator for BeginIter<'a> {
    type Item = ContactBeginTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| ContactBeginTouch { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct EndIter<'a> {
    iter: core::slice::Iter<'a, ContactEndTouchEvent>,
}
impl<'a> Iterator for EndIter<'a> {
    type Item = ContactEndTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| ContactEndTouch { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct HitIter<'a> {
    iter: core::slice::Iter<'a, ContactHitEvent>,
}
impl<'a> Iterator for HitIter<'a> {
    type Item = ContactHit<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| ContactHit { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[derive(Clone, Debug)]
pub struct ContactBeginTouchEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
}

impl ContactBeginTouchEvent {
    /// Copy a native contact-begin event into an owned Rust value.
    pub(crate) fn from_raw_in(
        brand: IdBrand,
        contact_epoch: ContactEpoch,
        raw: ffi::b2ContactBeginTouchEvent,
    ) -> Self {
        Self {
            shape_a: brand
                .try_shape(raw.shapeIdA)
                .expect("Box2D contact begin event contained an invalid shape A id"),
            shape_b: brand
                .try_shape(raw.shapeIdB)
                .expect("Box2D contact begin event contained an invalid shape B id"),
            contact_id: brand
                .try_contact(raw.contactId, contact_epoch)
                .expect("Box2D contact begin event contained an invalid contact id"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContactEndTouchEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
}

impl ContactEndTouchEvent {
    /// Copy a native contact-end event into an owned Rust value.
    pub(crate) fn from_raw_in(
        brand: IdBrand,
        contact_epoch: ContactEpoch,
        raw: ffi::b2ContactEndTouchEvent,
    ) -> Self {
        Self {
            // End-event ids may be stale. `try_*` only checks structural identity here; runtime
            // APIs still perform native validity checks before sending them back to Box2D.
            shape_a: brand
                .try_shape(raw.shapeIdA)
                .expect("Box2D contact end event contained an invalid shape A id"),
            shape_b: brand
                .try_shape(raw.shapeIdB)
                .expect("Box2D contact end event contained an invalid shape B id"),
            contact_id: brand
                .try_contact(raw.contactId, contact_epoch)
                .expect("Box2D contact end event contained an invalid contact id"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContactHitEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
    pub point: Position,
    pub normal: Vec2,
    pub approach_speed: f32,
}

impl ContactHitEvent {
    /// Copy a native contact-hit event into an owned Rust value.
    pub(crate) fn from_raw_in(
        brand: IdBrand,
        contact_epoch: ContactEpoch,
        raw: ffi::b2ContactHitEvent,
    ) -> Self {
        Self {
            shape_a: brand
                .try_shape(raw.shapeIdA)
                .expect("Box2D contact hit event contained an invalid shape A id"),
            shape_b: brand
                .try_shape(raw.shapeIdB)
                .expect("Box2D contact hit event contained an invalid shape B id"),
            contact_id: brand
                .try_contact(raw.contactId, contact_epoch)
                .expect("Box2D contact hit event contained an invalid contact id"),
            point: Position::from_raw(raw.point),
            normal: Vec2::from_raw(raw.normal),
            approach_speed: raw.approachSpeed,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ContactEvents {
    pub begin: Vec<ContactBeginTouchEvent>,
    pub end: Vec<ContactEndTouchEvent>,
    pub hit: Vec<ContactHitEvent>,
}

pub(super) fn capture_native_events_into(
    world: ffi::b2WorldId,
    brand: IdBrand,
    contact_epoch: ContactEpoch,
    out: &mut ContactEvents,
) {
    let raw = unsafe { ffi::b2World_GetContactEvents(world) };
    let begin = if raw.beginCount > 0 && !raw.beginEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.beginEvents, raw.beginCount as usize) }
    } else {
        &[][..]
    };
    let end = if raw.endCount > 0 && !raw.endEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.endEvents, raw.endCount as usize) }
    } else {
        &[][..]
    };
    let hit = if raw.hitCount > 0 && !raw.hitEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.hitEvents, raw.hitCount as usize) }
    } else {
        &[][..]
    };

    super::map_snapshot_into(&mut out.begin, begin, |event| {
        ContactBeginTouchEvent::from_raw_in(brand, contact_epoch, *event)
    });
    super::map_snapshot_into(&mut out.end, end, |event| {
        ContactEndTouchEvent::from_raw_in(brand, contact_epoch, *event)
    });
    super::map_snapshot_into(&mut out.hit, hit, |event| {
        ContactHitEvent::from_raw_in(brand, contact_epoch, *event)
    });
}

fn contact_events_snapshot_impl(cache: &super::EventCache) -> ContactEvents {
    cache.snapshot().contact.clone()
}

fn contact_events_into_impl(cache: &super::EventCache, out: &mut ContactEvents) {
    let snapshot = cache.snapshot();
    super::map_snapshot_into(&mut out.begin, &snapshot.contact.begin, Clone::clone);
    super::map_snapshot_into(&mut out.end, &snapshot.contact.end, Clone::clone);
    super::map_snapshot_into(&mut out.hit, &snapshot.contact.hit, Clone::clone);
}

impl World {
    pub fn contact_events(&self) -> ContactEvents {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        contact_events_snapshot_impl(self.event_cache())
    }

    pub fn contact_events_into(&self, out: &mut ContactEvents) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        contact_events_into_impl(self.event_cache(), out);
    }

    pub fn try_contact_events(&self) -> crate::error::ApiResult<ContactEvents> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(contact_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_contact_events_into(&self, out: &mut ContactEvents) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        contact_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl WorldHandle {
    pub fn contact_events(&self) -> ContactEvents {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        contact_events_snapshot_impl(self.event_cache())
    }

    pub fn contact_events_into(&self, out: &mut ContactEvents) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        contact_events_into_impl(self.event_cache(), out);
    }

    pub fn try_contact_events(&self) -> crate::error::ApiResult<ContactEvents> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(contact_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_contact_events_into(&self, out: &mut ContactEvents) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        contact_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl World {
    /// Low-level raw view over contact events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// Call this immediately after the latest world step and before any operation that may mutate
    /// the world. The returned slices borrow transient Box2D storage. While `f` runs, you must not
    /// perform any operation that can mutate that storage.
    ///
    /// Dropping `Owned*` handles inside `f` is OK; destruction is deferred until after this call.
    pub unsafe fn with_contact_events_raw<T>(
        &self,
        f: impl FnOnce(
            &[ffi::b2ContactBeginTouchEvent],
            &[ffi::b2ContactEndTouchEvent],
            &[ffi::b2ContactHitEvent],
        ) -> T,
    ) -> T {
        self.with_borrowed_event_buffers(|| {
            // Low-level raw view over contact events.
            // Exposes FFI slices directly; they are only valid within this call.
            // Prefer `with_contact_events_view` for a safe, FFI-opaque interface.
            let raw = unsafe { ffi::b2World_GetContactEvents(self.raw()) };
            let begin = if raw.beginCount > 0 && !raw.beginEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.beginEvents, raw.beginCount as usize) }
            } else {
                &[][..]
            };
            let end = if raw.endCount > 0 && !raw.endEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.endEvents, raw.endCount as usize) }
            } else {
                &[][..]
            };
            let hit = if raw.hitCount > 0 && !raw.hitEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.hitEvents, raw.hitCount as usize) }
            } else {
                &[][..]
            };
            f(begin, end, hit)
        })
    }

    /// Low-level raw view over contact events with recoverable callback-lock checking.
    ///
    /// # Safety
    /// Same safety contract as `with_contact_events_raw`.
    pub unsafe fn try_with_contact_events_raw<T>(
        &self,
        f: impl FnOnce(
            &[ffi::b2ContactBeginTouchEvent],
            &[ffi::b2ContactEndTouchEvent],
            &[ffi::b2ContactHitEvent],
        ) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetContactEvents(self.raw()) };
            let begin = if raw.beginCount > 0 && !raw.beginEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.beginEvents, raw.beginCount as usize) }
            } else {
                &[][..]
            };
            let end = if raw.endCount > 0 && !raw.endEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.endEvents, raw.endCount as usize) }
            } else {
                &[][..]
            };
            let hit = if raw.hitCount > 0 && !raw.hitEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.hitEvents, raw.hitCount as usize) }
            } else {
                &[][..]
            };
            f(begin, end, hit)
        })
    }

    /// Zero-copy view over the Rust-owned completed-step contact events.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes immediately; the
    /// destruction is deferred until after the view ends to preserve existing event-view ordering
    /// semantics.
    ///
    /// Example
    /// ```rust
    /// use boxdd::prelude::*;
    /// let mut world = World::new(WorldDef::default()).unwrap();
    /// world.with_contact_events_view(|begin, end, hit| {
    ///     let nb = begin.count();
    ///     let ne = end.count();
    ///     let nh = hit.count();
    ///     assert!(nb + ne + nh >= 0);
    /// });
    /// ```
    pub fn with_contact_events_view<T>(
        &self,
        f: impl FnOnce(BeginIter<'_>, EndIter<'_>, HitIter<'_>) -> T,
    ) -> T {
        self.with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(
                BeginIter {
                    iter: snapshot.contact.begin.iter(),
                },
                EndIter {
                    iter: snapshot.contact.end.iter(),
                },
                HitIter {
                    iter: snapshot.contact.hit.iter(),
                },
            )
        })
    }

    /// Zero-copy view over contact events with recoverable callback-lock checking.
    pub fn try_with_contact_events_view<T>(
        &self,
        f: impl FnOnce(BeginIter<'_>, EndIter<'_>, HitIter<'_>) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(
                BeginIter {
                    iter: snapshot.contact.begin.iter(),
                },
                EndIter {
                    iter: snapshot.contact.end.iter(),
                },
                HitIter {
                    iter: snapshot.contact.hit.iter(),
                },
            )
        })
    }
}
