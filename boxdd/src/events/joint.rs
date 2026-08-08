use crate::core::identity_registry::OutputIdentityResolver;
use crate::core::world_core::WorldCore;
use crate::error::Result;
use crate::types::JointId;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct JointEvent {
    pub joint_id: JointId,
}

impl JointEvent {
    fn from_raw(identities: &OutputIdentityResolver<'_>, raw: ffi::b2JointEvent) -> Result<Self> {
        Ok(Self {
            joint_id: identities.joint(raw.jointId)?,
        })
    }
}

/// A borrowed view of joint events from one completed step.
pub struct JointEvents<'step> {
    events: &'step [JointEvent],
}

impl<'step> JointEvents<'step> {
    pub(super) fn new(events: &'step [JointEvent]) -> Self {
        Self { events }
    }

    pub fn iter(&self) -> core::slice::Iter<'_, JointEvent> {
        self.events.iter()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn to_owned(&self) -> Result<Vec<JointEvent>> {
        super::to_owned(self.events)
    }

    pub fn clone_into(&self, out: &mut Vec<JointEvent>) -> Result<()> {
        super::clone_into(self.events, out)
    }
}

impl<'view, 'step> IntoIterator for &'view JointEvents<'step> {
    type Item = &'view JointEvent;
    type IntoIter = core::slice::Iter<'view, JointEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(super) fn capture(
    out: &mut Vec<JointEvent>,
    raw: ffi::b2JointEvents,
    core: &WorldCore,
) -> Result<()> {
    // SAFETY: The completed-step capability prevents mutation while the returned slice is mapped.
    let raw = unsafe { super::ffi_slice(raw.jointEvents, raw.count) }?;
    if raw.is_empty() {
        out.clear();
        return Ok(());
    }
    super::prepare_mapped(out, raw.len())?;
    core.with_output_identity_resolver(|identities| {
        super::extend_mapped(out, raw, |event| JointEvent::from_raw(identities, *event))
    })
}
