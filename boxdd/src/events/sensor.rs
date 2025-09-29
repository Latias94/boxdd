use crate::types::ShapeId;
use crate::world::World;
use boxdd_sys::ffi;

/// Zero-copy view wrappers for sensor events.
/// Data is borrowed and valid only for the duration of the closure passed
/// to `with_sensor_events_view`.
#[derive(Copy, Clone)]
pub struct SensorBeginTouch<'a>(&'a ffi::b2SensorBeginTouchEvent);
impl<'a> SensorBeginTouch<'a> {
    pub fn sensor_shape(&self) -> ShapeId {
        self.0.sensorShapeId
    }
    pub fn visitor_shape(&self) -> ShapeId {
        self.0.visitorShapeId
    }
}

#[derive(Copy, Clone)]
pub struct SensorEndTouch<'a>(&'a ffi::b2SensorEndTouchEvent);
impl<'a> SensorEndTouch<'a> {
    pub fn sensor_shape(&self) -> ShapeId {
        self.0.sensorShapeId
    }
    pub fn visitor_shape(&self) -> ShapeId {
        self.0.visitorShapeId
    }
}

pub struct SensorBeginIter<'a>(core::slice::Iter<'a, ffi::b2SensorBeginTouchEvent>);
impl<'a> Iterator for SensorBeginIter<'a> {
    type Item = SensorBeginTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(SensorBeginTouch)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

pub struct SensorEndIter<'a>(core::slice::Iter<'a, ffi::b2SensorEndTouchEvent>);
impl<'a> Iterator for SensorEndIter<'a> {
    type Item = SensorEndTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(SensorEndTouch)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
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
pub struct SensorEvents {
    pub begin: Vec<SensorBeginTouchEvent>,
    pub end: Vec<SensorEndTouchEvent>,
}

impl World {
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

    pub fn with_sensor_events<T>(
        &self,
        f: impl FnOnce(&[ffi::b2SensorBeginTouchEvent], &[ffi::b2SensorEndTouchEvent]) -> T,
    ) -> T {
        // Low-level raw view exposing FFI slices; valid only within this call.
        // Prefer `with_sensor_events_view` to avoid leaking FFI types.
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

    /// Zero-copy safe view over sensor events without exposing raw FFI types.
    /// Borrowed data is valid only within the closure; do not store references.
    ///
    /// Example
    /// ```rust
    /// use boxdd::prelude::*;
    /// let mut world = World::new(WorldDef::default()).unwrap();
    /// world.with_sensor_events_view(|beg, end| {
    ///     let _ = (beg.count(), end.count());
    /// });
    /// ```
    pub fn with_sensor_events_view<T>(
        &self,
        f: impl FnOnce(SensorBeginIter<'_>, SensorEndIter<'_>) -> T,
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
        f(SensorBeginIter(begin.iter()), SensorEndIter(end.iter()))
    }
}
