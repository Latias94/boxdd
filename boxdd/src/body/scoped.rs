use crate::error::Result;
use crate::types::BodyId;
use crate::world::BodyProof;

/// A body handle with lifetime tied to the owning world.
pub struct Body<'w> {
    pub(crate) proof: BodyProof<'w>,
}

impl<'w> Body<'w> {
    pub(crate) fn new(proof: BodyProof<'w>) -> Self {
        Self { proof }
    }

    pub fn id(&self) -> BodyId {
        self.proof.id()
    }

    /// Destroy this body and every attached shape or joint.
    pub fn destroy(self) -> Result<()> {
        self.proof.call(|body| body.destroy())
    }
}
