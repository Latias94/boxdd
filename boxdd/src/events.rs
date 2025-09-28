//! Event snapshots and zero-copy visitors.
//!
//! Notes on lifetimes and safety:
//! - Snapshot getters like `body_events`, `sensor_events`, `contact_events`, `joint_events` copy
//!   event data into owned Rust collections. Those are safe to keep after stepping the world.
//! - Zero-copy visitors like `with_body_events`/`with_contact_events` pass slices that are only
//!   valid for the duration of the closure call and the current step. Do not store references
//!   from those slices.
//! - The underlying buffers are managed by Box2D and can be invalidated by the next `World::step`.
//!
use crate::Transform;
use crate::types::{BodyId, JointId, ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct BodyMoveEvent {
    pub body_id: BodyId,
    pub transform: Transform,
    pub fell_asleep: bool,
}

#[derive(Clone, Debug)]
pub struct SensorBeginTouchEvent {
    pub sensor_shape: ShapeId,
    pub visitor_shape: ShapeId,
}
#[derive(Clone, Debug)]
pub struct SensorEndTouchEvent {
    pub sensor_shape: ShapeId,
    pub visitor_shape: ShapeId,
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

#[derive(Clone, Debug)]
pub struct SensorEvents {
    pub begin: Vec<SensorBeginTouchEvent>,
    pub end: Vec<SensorEndTouchEvent>,
}

#[derive(Clone, Debug)]
pub struct JointEvent {
    pub joint_id: JointId,
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

    pub fn sensor_events(&self) -> SensorEvents {
        let raw = unsafe { ffi::b2World_GetSensorEvents(self.raw()) };
        let mut begin = Vec::new();
        let mut end = Vec::new();
        if raw.beginCount > 0 && !raw.beginEvents.is_null() {
            let s =
                unsafe { core::slice::from_raw_parts(raw.beginEvents, raw.beginCount as usize) };
            begin.extend(s.iter().map(|e| SensorBeginTouchEvent {
                sensor_shape: e.sensorShapeId,
                visitor_shape: e.visitorShapeId,
            }));
        }
        if raw.endCount > 0 && !raw.endEvents.is_null() {
            let s = unsafe { core::slice::from_raw_parts(raw.endEvents, raw.endCount as usize) };
            end.extend(s.iter().map(|e| SensorEndTouchEvent {
                sensor_shape: e.sensorShapeId,
                visitor_shape: e.visitorShapeId,
            }));
        }
        SensorEvents { begin, end }
    }

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

    pub fn joint_events(&self) -> Vec<JointEvent> {
        let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
        if raw.count <= 0 || raw.jointEvents.is_null() {
            return Vec::new();
        }
        let s = unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) };
        s.iter()
            .map(|e| JointEvent {
                joint_id: e.jointId,
            })
            .collect()
    }

    // Zero-copy visitors (closure style). Data is only valid within the call.
    pub fn with_body_events<T>(&self, f: impl FnOnce(&[ffi::b2BodyMoveEvent]) -> T) -> T {
        let raw = unsafe { ffi::b2World_GetBodyEvents(self.raw()) };
        let slice = if raw.moveCount > 0 && !raw.moveEvents.is_null() {
            unsafe { core::slice::from_raw_parts(raw.moveEvents, raw.moveCount as usize) }
        } else {
            &[][..]
        };
        f(slice)
    }

    pub fn with_sensor_events<T>(
        &self,
        f: impl FnOnce(&[ffi::b2SensorBeginTouchEvent], &[ffi::b2SensorEndTouchEvent]) -> T,
    ) -> T {
        let raw = unsafe { ffi::b2World_GetSensorEvents(self.raw()) };
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
        f(begin, end)
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

    pub fn with_joint_events<T>(&self, f: impl FnOnce(&[ffi::b2JointEvent]) -> T) -> T {
        let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
        let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
            unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
        } else {
            &[][..]
        };
        f(slice)
    }
}
