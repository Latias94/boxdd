use crate::id::IdBrand;
use crate::types::ShapeId;
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

/// Zero-copy view wrappers for the Rust-owned completed-step sensor events.
/// Data is borrowed and valid only for the duration of the closure passed
/// to `with_sensor_events_view`.
#[derive(Copy, Clone)]
pub struct SensorBeginTouch<'a> {
    event: &'a SensorBeginTouchEvent,
}
impl<'a> SensorBeginTouch<'a> {
    pub fn sensor_shape(&self) -> ShapeId {
        self.event.sensor_shape
    }
    pub fn visitor_shape(&self) -> ShapeId {
        self.event.visitor_shape
    }
}

#[derive(Copy, Clone)]
pub struct SensorEndTouch<'a> {
    event: &'a SensorEndTouchEvent,
}
impl<'a> SensorEndTouch<'a> {
    pub fn sensor_shape(&self) -> ShapeId {
        self.event.sensor_shape
    }
    pub fn visitor_shape(&self) -> ShapeId {
        self.event.visitor_shape
    }
}

pub struct SensorBeginIter<'a> {
    iter: core::slice::Iter<'a, SensorBeginTouchEvent>,
}
impl<'a> Iterator for SensorBeginIter<'a> {
    type Item = SensorBeginTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| SensorBeginTouch { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct SensorEndIter<'a> {
    iter: core::slice::Iter<'a, SensorEndTouchEvent>,
}
impl<'a> Iterator for SensorEndIter<'a> {
    type Item = SensorEndTouch<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| SensorEndTouch { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[derive(Clone, Debug)]
pub struct SensorBeginTouchEvent {
    pub sensor_shape: ShapeId,
    pub visitor_shape: ShapeId,
}

impl SensorBeginTouchEvent {
    /// Copy a native sensor-begin event into an owned Rust value.
    pub(crate) fn from_raw_in(brand: IdBrand, raw: ffi::b2SensorBeginTouchEvent) -> Self {
        Self {
            sensor_shape: brand
                .try_shape(raw.sensorShapeId)
                .expect("Box2D sensor begin event contained an invalid sensor shape id"),
            visitor_shape: brand
                .try_shape(raw.visitorShapeId)
                .expect("Box2D sensor begin event contained an invalid visitor shape id"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SensorEndTouchEvent {
    pub sensor_shape: ShapeId,
    pub visitor_shape: ShapeId,
}

impl SensorEndTouchEvent {
    /// Copy a native sensor-end event into an owned Rust value.
    pub(crate) fn from_raw_in(brand: IdBrand, raw: ffi::b2SensorEndTouchEvent) -> Self {
        Self {
            // End-event ids may be stale. Structural validation prevents foreign/null ids from
            // becoming target-bound values without requiring the native objects to remain live.
            sensor_shape: brand
                .try_shape(raw.sensorShapeId)
                .expect("Box2D sensor end event contained an invalid sensor shape id"),
            visitor_shape: brand
                .try_shape(raw.visitorShapeId)
                .expect("Box2D sensor end event contained an invalid visitor shape id"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SensorEvents {
    pub begin: Vec<SensorBeginTouchEvent>,
    pub end: Vec<SensorEndTouchEvent>,
}

impl SensorEvents {
    pub(super) fn capture_into(mut self, raw: ffi::b2SensorEvents, brand: IdBrand) -> Self {
        // SAFETY: Each pointer/count pair belongs to the completed step and is fully consumed
        // before the next world mutation.
        self.begin = unsafe {
            super::capture_ffi_vec(self.begin, raw.beginEvents, raw.beginCount, |event| {
                SensorBeginTouchEvent::from_raw_in(brand, *event)
            })
        };
        // SAFETY: Same completed-step ownership contract as `begin` above.
        self.end = unsafe {
            super::capture_ffi_vec(self.end, raw.endEvents, raw.endCount, |event| {
                SensorEndTouchEvent::from_raw_in(brand, *event)
            })
        };
        self
    }
}

fn sensor_events_snapshot_impl(cache: &super::EventCache) -> SensorEvents {
    std::clone::Clone::clone(&cache.snapshot().sensor)
}

fn sensor_events_into_impl(cache: &super::EventCache, out: &mut SensorEvents) {
    let snapshot = cache.snapshot();
    super::map_snapshot_into(&mut out.begin, &snapshot.sensor.begin, Clone::clone);
    super::map_snapshot_into(&mut out.end, &snapshot.sensor.end, Clone::clone);
}

impl World {
    pub fn sensor_events(&self) -> SensorEvents {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        sensor_events_snapshot_impl(self.event_cache())
    }

    pub fn sensor_events_into(&self, out: &mut SensorEvents) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        sensor_events_into_impl(self.event_cache(), out);
    }

    pub fn try_sensor_events(&self) -> crate::error::ApiResult<SensorEvents> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(sensor_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_sensor_events_into(&self, out: &mut SensorEvents) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        sensor_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl WorldHandle {
    pub fn sensor_events(&self) -> SensorEvents {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        sensor_events_snapshot_impl(self.event_cache())
    }

    pub fn sensor_events_into(&self, out: &mut SensorEvents) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        sensor_events_into_impl(self.event_cache(), out);
    }

    pub fn try_sensor_events(&self) -> crate::error::ApiResult<SensorEvents> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(sensor_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_sensor_events_into(&self, out: &mut SensorEvents) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        sensor_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl World {
    /// Low-level raw view over sensor events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// Call this immediately after the latest world step and before any operation that may mutate
    /// the world. The returned slices borrow transient Box2D storage. While `f` runs, you must not
    /// perform any operation that can mutate that storage.
    ///
    /// Dropping `Owned*` handles inside `f` is OK; destruction is deferred until after this call.
    pub unsafe fn with_sensor_events_raw<T>(
        &self,
        f: impl FnOnce(&[ffi::b2SensorBeginTouchEvent], &[ffi::b2SensorEndTouchEvent]) -> T,
    ) -> T {
        self.with_borrowed_event_buffers(|| {
            // Low-level raw view exposing FFI slices; valid only within this call.
            // Prefer `with_sensor_events_view` to avoid leaking FFI types.
            let raw = unsafe { ffi::b2World_GetSensorEvents(self.raw()) };
            // SAFETY: The enclosing raw-view contract keeps the native buffers valid for `f`.
            unsafe {
                super::with_ffi_slice(raw.beginEvents, raw.beginCount, |begin| {
                    super::with_ffi_slice(raw.endEvents, raw.endCount, |end| f(begin, end))
                })
            }
        })
    }

    /// Low-level raw view over sensor events with recoverable callback-lock checking.
    ///
    /// # Safety
    /// Same safety contract as `with_sensor_events_raw`.
    pub unsafe fn try_with_sensor_events_raw<T>(
        &self,
        f: impl FnOnce(&[ffi::b2SensorBeginTouchEvent], &[ffi::b2SensorEndTouchEvent]) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetSensorEvents(self.raw()) };
            // SAFETY: The enclosing raw-view contract keeps the native buffers valid for `f`.
            unsafe {
                super::with_ffi_slice(raw.beginEvents, raw.beginCount, |begin| {
                    super::with_ffi_slice(raw.endEvents, raw.endCount, |end| f(begin, end))
                })
            }
        })
    }

    /// Zero-copy view over the Rust-owned completed-step sensor events.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes/joints immediately;
    /// the destruction is deferred until after the view ends to preserve existing event-view
    /// ordering semantics.
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
        self.with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(
                SensorBeginIter {
                    iter: snapshot.sensor.begin.iter(),
                },
                SensorEndIter {
                    iter: snapshot.sensor.end.iter(),
                },
            )
        })
    }

    /// Zero-copy view over sensor events with recoverable callback-lock checking.
    pub fn try_with_sensor_events_view<T>(
        &self,
        f: impl FnOnce(SensorBeginIter<'_>, SensorEndIter<'_>) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(
                SensorBeginIter {
                    iter: snapshot.sensor.begin.iter(),
                },
                SensorEndIter {
                    iter: snapshot.sensor.end.iter(),
                },
            )
        })
    }
}
