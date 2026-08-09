use crate::core::identity_registry::OutputIdentityResolver;
use crate::core::world_core::WorldCore;
use crate::error::Result;
use crate::types::ShapeId;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct SensorBeginTouchEvent {
    pub sensor_shape: ShapeId,
    pub visitor_shape: ShapeId,
}

impl SensorBeginTouchEvent {
    fn from_raw(
        identities: &OutputIdentityResolver<'_>,
        raw: ffi::b2SensorBeginTouchEvent,
    ) -> Result<Self> {
        Ok(Self {
            sensor_shape: identities.shape(raw.sensorShapeId)?,
            visitor_shape: identities.shape(raw.visitorShapeId)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SensorEndTouchEvent {
    pub sensor_shape: ShapeId,
    pub visitor_shape: ShapeId,
}

impl SensorEndTouchEvent {
    fn from_raw(
        identities: &OutputIdentityResolver<'_>,
        raw: ffi::b2SensorEndTouchEvent,
    ) -> Result<Self> {
        Ok(Self {
            sensor_shape: identities.shape(raw.sensorShapeId)?,
            visitor_shape: identities.shape(raw.visitorShapeId)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct SensorEvents {
    pub begin: Vec<SensorBeginTouchEvent>,
    pub end: Vec<SensorEndTouchEvent>,
}

/// A borrowed view of sensor events from one completed step.
pub struct SensorEventsView<'step> {
    events: &'step SensorEvents,
}

impl<'step> SensorEventsView<'step> {
    pub(super) fn new(events: &'step SensorEvents) -> Self {
        Self { events }
    }

    pub fn begin(&self) -> &[SensorBeginTouchEvent] {
        &self.events.begin
    }

    pub fn end(&self) -> &[SensorEndTouchEvent] {
        &self.events.end
    }

    pub fn is_empty(&self) -> bool {
        self.events.begin.is_empty() && self.events.end.is_empty()
    }

    pub fn to_owned(&self) -> Result<SensorEvents> {
        let mut out = SensorEvents::default();
        self.clone_into(&mut out)?;
        Ok(out)
    }

    pub fn clone_into(&self, out: &mut SensorEvents) -> Result<()> {
        let result = (|| {
            super::clone_into(&self.events.begin, &mut out.begin)?;
            super::clone_into(&self.events.end, &mut out.end)
        })();
        if result.is_err() {
            out.begin.clear();
            out.end.clear();
        }
        result
    }
}

pub(super) fn capture(
    out: &mut SensorEvents,
    raw: ffi::b2SensorEvents,
    core: &WorldCore,
) -> Result<()> {
    // SAFETY: The completed-step capability prevents mutation while these slices are mapped.
    let begin = unsafe { super::ffi_slice(raw.beginEvents, raw.beginCount) }?;
    // SAFETY: Same completed-step lifetime as `begin`.
    let end = unsafe { super::ffi_slice(raw.endEvents, raw.endCount) }?;

    if begin.is_empty() && end.is_empty() {
        out.begin.clear();
        out.end.clear();
        return Ok(());
    }

    let result = (|| {
        super::prepare_mapped(&mut out.begin, begin.len())?;
        super::prepare_mapped(&mut out.end, end.len())?;
        core.with_output_identity_resolver(|identities| {
            super::extend_mapped(&mut out.begin, begin, |event| {
                SensorBeginTouchEvent::from_raw(identities, *event)
            })?;
            super::extend_mapped(&mut out.end, end, |event| {
                SensorEndTouchEvent::from_raw(identities, *event)
            })
        })
    })();
    if result.is_err() {
        out.begin.clear();
        out.end.clear();
    }
    result
}
