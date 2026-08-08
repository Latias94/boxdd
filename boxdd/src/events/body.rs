use crate::core::identity_registry::OutputIdentityResolver;
use crate::core::world_core::WorldCore;
use crate::error::Result;
use crate::types::{BodyId, WorldTransform};
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct BodyMoveEvent {
    pub body_id: BodyId,
    pub transform: WorldTransform,
    pub fell_asleep: bool,
}

impl BodyMoveEvent {
    fn from_raw(
        identities: &OutputIdentityResolver<'_>,
        raw: ffi::b2BodyMoveEvent,
    ) -> Result<Self> {
        let transform = WorldTransform::from_raw(raw.transform).map_err(|_| {
            crate::Error::InvalidNativeOutput {
                operation: "CompletedStep::body_events",
                output: "transform",
                constraint: "a finite rigid world transform",
            }
        })?;
        Ok(Self {
            body_id: identities.body(raw.bodyId)?,
            transform,
            fell_asleep: raw.fellAsleep,
        })
    }
}

/// A borrowed view of body-move events from one completed step.
pub struct BodyEvents<'step> {
    events: &'step [BodyMoveEvent],
}

impl<'step> BodyEvents<'step> {
    pub(super) fn new(events: &'step [BodyMoveEvent]) -> Self {
        Self { events }
    }

    pub fn iter(&self) -> core::slice::Iter<'_, BodyMoveEvent> {
        self.events.iter()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn to_owned(&self) -> Result<Vec<BodyMoveEvent>> {
        super::to_owned(self.events)
    }

    pub fn clone_into(&self, out: &mut Vec<BodyMoveEvent>) -> Result<()> {
        super::clone_into(self.events, out)
    }
}

impl<'view, 'step> IntoIterator for &'view BodyEvents<'step> {
    type Item = &'view BodyMoveEvent;
    type IntoIter = core::slice::Iter<'view, BodyMoveEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(super) fn capture(
    out: &mut Vec<BodyMoveEvent>,
    raw: ffi::b2BodyEvents,
    core: &WorldCore,
) -> Result<()> {
    // SAFETY: The completed-step capability prevents mutation while the returned slice is mapped.
    let raw = unsafe { super::ffi_slice(raw.moveEvents, raw.moveCount) }?;
    if raw.is_empty() {
        out.clear();
        return Ok(());
    }
    super::prepare_mapped(out, raw.len())?;
    core.with_output_identity_resolver(|identities| {
        super::extend_mapped(out, raw, |event| {
            BodyMoveEvent::from_raw(identities, *event)
        })
    })
}
