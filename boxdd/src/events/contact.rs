use crate::types::{ShapeId, Vec2};
use crate::world::World;
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
    pub fn contact_id(&self) -> ffi::b2ContactId {
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
    pub contact_id: ffi::b2ContactId,
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

#[derive(Clone, Debug)]
pub struct ContactEvents {
    pub begin: Vec<ContactBeginTouchEvent>,
    pub end: Vec<ContactEndTouchEvent>,
    pub hit: Vec<ContactHitEvent>,
}

impl World {
    pub fn contact_events(&self) -> ContactEvents {
        let raw = unsafe { ffi::b2World_GetContactEvents(self.raw()) };
        let mut begin = Vec::new();
        let mut end = Vec::new();
        let mut hit = Vec::new();
        if raw.beginCount > 0 && !raw.beginEvents.is_null() {
            let s =
                unsafe { core::slice::from_raw_parts(raw.beginEvents, raw.beginCount as usize) };
            begin.extend(s.iter().map(|e| ContactBeginTouchEvent {
                shape_a: e.shapeIdA,
                shape_b: e.shapeIdB,
                contact_id: e.contactId,
            }));
        }
        if raw.endCount > 0 && !raw.endEvents.is_null() {
            let s = unsafe { core::slice::from_raw_parts(raw.endEvents, raw.endCount as usize) };
            end.extend(s.iter().map(|e| ContactEndTouchEvent {
                shape_a: e.shapeIdA,
                shape_b: e.shapeIdB,
            }));
        }
        if raw.hitCount > 0 && !raw.hitEvents.is_null() {
            let s = unsafe { core::slice::from_raw_parts(raw.hitEvents, raw.hitCount as usize) };
            hit.extend(s.iter().map(|e| ContactHitEvent {
                shape_a: e.shapeIdA,
                shape_b: e.shapeIdB,
                point: e.point.into(),
                normal: e.normal.into(),
                approach_speed: e.approachSpeed,
            }));
        }
        ContactEvents { begin, end, hit }
    }

    pub fn with_contact_events<T>(
        &self,
        f: impl FnOnce(
            &[ffi::b2ContactBeginTouchEvent],
            &[ffi::b2ContactEndTouchEvent],
            &[ffi::b2ContactHitEvent],
        ) -> T,
    ) -> T {
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
    }

    /// Zero-copy safe view over contact events without exposing raw FFI types.
    /// The borrowed data is valid only within the closure; do not store references.
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
    }
}
