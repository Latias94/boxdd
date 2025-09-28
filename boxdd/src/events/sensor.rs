use crate::types::ShapeId;
use crate::world::World;
use boxdd_sys::ffi;

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
}
