use crate::types::{ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

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
}
