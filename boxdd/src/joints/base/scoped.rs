use super::*;
use crate::error::Result;
use crate::types::JointId;
use std::fmt;

impl fmt::Debug for Joint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Joint")
            .field("id", &self.id())
            .field("kind", &self.cached_kind())
            .finish()
    }
}

impl<'w> Joint<'w> {
    pub(crate) fn new(proof: crate::world::JointProof<'w>) -> Self {
        Self { proof }
    }

    pub fn id(&self) -> JointId {
        self.proof.id()
    }

    /// Destroy this joint immediately.
    pub fn destroy(self, wake_bodies: bool) -> Result<()> {
        self.proof.call(|joint| joint.destroy(wake_bodies))
    }
}
