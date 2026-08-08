use super::*;

/// A scoped shape handle tied to a mutable borrow of the world.
pub struct Shape<'w> {
    pub(crate) proof: crate::world::ShapeProof<'w>,
}

impl<'w> Shape<'w> {
    pub(crate) fn new(proof: crate::world::ShapeProof<'w>) -> Self {
        Self { proof }
    }

    pub fn id(&self) -> ShapeId {
        self.proof.id()
    }

    /// Destroy this shape immediately.
    ///
    /// After destruction, any previously stored `ShapeId` referring to this shape becomes invalid.
    pub fn destroy(self, update_body_mass: bool) -> Result<()> {
        self.proof.call(|shape| shape.destroy(update_body_mass))
    }
}
