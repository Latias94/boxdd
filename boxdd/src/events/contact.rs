use crate::types::{ContactId, ShapeId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

/// Zero-copy view wrappers for contact events.
/// These types borrow the underlying FFI events but expose a safe Rust API.
/// The borrowed data is only valid for the duration of the closure passed
/// to `with_contact_events_view`.
#[derive(Copy, Clone)]
pub struct ContactBeginTouch<'a>(&'a ffi::b2ContactBeginTouchEvent);
impl<'a> ContactBeginTouch<'a> {
    pub fn shape_a(&self) -> ShapeId {
        self.0.shapeIdA
    }
    pub fn shape_b(&self) -> ShapeId {
        self.0.shapeIdB
    }
    pub fn contact_id(&self) -> ContactId {
        self.0.contactId
    }
}

#[derive(Copy, Clone)]
pub struct ContactEndTouch<'a>(&'a ffi::b2ContactEndTouchEvent);
impl<'a> ContactEndTouch<'a> {
    pub fn shape_a(&self) -> ShapeId {
        self.0.shapeIdA
    }
    pub fn shape_b(&self) -> ShapeId {
        self.0.shapeIdB
    }
}

#[derive(Copy, Clone)]
pub struct ContactHit<'a>(&'a ffi::b2ContactHitEvent);
impl<'a> ContactHit<'a> {
    pub fn shape_a(&self) -> ShapeId {
        self.0.shapeIdA
    }
    pub fn shape_b(&self) -> ShapeId {
        self.0.shapeIdB
    }
    pub fn point(&self) -> Vec2 {
        self.0.point.into()
    }
    pub fn normal(&self) -> Vec2 {
        self.0.normal.into()
    }
    pub fn approach_speed(&self) -> f32 {
        self.0.approachSpeed
    }
}

pub struct BeginIter<'a>(core::slice::Iter<'a, ffi::b2ContactBeginTouchEvent>);
impl<'a> Iterator for BeginIter<'a> {
    type Item = ContactBeginTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ContactBeginTouch)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

pub struct EndIter<'a>(core::slice::Iter<'a, ffi::b2ContactEndTouchEvent>);
impl<'a> Iterator for EndIter<'a> {
    type Item = ContactEndTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ContactEndTouch)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

pub struct HitIter<'a>(core::slice::Iter<'a, ffi::b2ContactHitEvent>);
impl<'a> Iterator for HitIter<'a> {
    type Item = ContactHit<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ContactHit)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

#[derive(Clone, Debug)]
pub struct ContactBeginTouchEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub contact_id: ContactId,
}

#[derive(Clone, Debug)]
pub struct ContactEndTouchEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
}

#[derive(Clone, Debug)]
pub struct ContactHitEvent {
    pub shape_a: ShapeId,
    pub shape_b: ShapeId,
    pub point: Vec2,
    pub normal: Vec2,
    pub approach_speed: f32,
}

#[derive(Clone, Debug, Default)]
pub struct ContactEvents {
    pub begin: Vec<ContactBeginTouchEvent>,
    pub end: Vec<ContactEndTouchEvent>,
    pub hit: Vec<ContactHitEvent>,
}

fn contact_events_into_impl(world: ffi::b2WorldId, out: &mut ContactEvents) {
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

    super::map_snapshot_into(&mut out.begin, begin, |e| ContactBeginTouchEvent {
        shape_a: e.shapeIdA,
        shape_b: e.shapeIdB,
        contact_id: e.contactId,
    });
    super::map_snapshot_into(&mut out.end, end, |e| ContactEndTouchEvent {
        shape_a: e.shapeIdA,
        shape_b: e.shapeIdB,
    });
    super::map_snapshot_into(&mut out.hit, hit, |e| ContactHitEvent {
        shape_a: e.shapeIdA,
        shape_b: e.shapeIdB,
        point: e.point.into(),
        normal: e.normal.into(),
        approach_speed: e.approachSpeed,
    });
}

macro_rules! impl_contact_event_snapshot_methods {
    ($world_ty:ty) => {
        impl $world_ty {
            pub fn contact_events(&self) -> ContactEvents {
                crate::core::callback_state::assert_not_in_callback();
                let mut out = ContactEvents::default();
                contact_events_into_impl(self.raw(), &mut out);
                out
            }

            pub fn contact_events_into(&self, out: &mut ContactEvents) {
                crate::core::callback_state::assert_not_in_callback();
                contact_events_into_impl(self.raw(), out);
            }

            pub fn try_contact_events(&self) -> crate::error::ApiResult<ContactEvents> {
                crate::core::callback_state::check_not_in_callback()?;
                let mut out = ContactEvents::default();
                contact_events_into_impl(self.raw(), &mut out);
                Ok(out)
            }

            pub fn try_contact_events_into(
                &self,
                out: &mut ContactEvents,
            ) -> crate::error::ApiResult<()> {
                crate::core::callback_state::check_not_in_callback()?;
                contact_events_into_impl(self.raw(), out);
                Ok(())
            }
        }
    };
}

impl_contact_event_snapshot_methods!(World);
impl_contact_event_snapshot_methods!(WorldHandle);

impl World {
    /// Low-level raw view over contact events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// The returned slices borrow internal Box2D buffers. While `f` runs, you must not perform
    /// any operation that can mutate those buffers (e.g. stepping the world or destroying bodies).
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

    /// Zero-copy view over contact events without exposing raw FFI types.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes immediately; the
    /// destruction is deferred until after the view ends to keep the borrowed buffers valid.
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
            f(
                BeginIter(begin.iter()),
                EndIter(end.iter()),
                HitIter(hit.iter()),
            )
        })
    }

    /// Zero-copy view over contact events with recoverable callback-lock checking.
    pub fn try_with_contact_events_view<T>(
        &self,
        f: impl FnOnce(BeginIter<'_>, EndIter<'_>, HitIter<'_>) -> T,
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
            f(
                BeginIter(begin.iter()),
                EndIter(end.iter()),
                HitIter(hit.iter()),
            )
        })
    }
}
