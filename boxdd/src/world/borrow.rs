use super::*;

impl World {
    /// Acquire a body capability after validating the id once.
    pub fn body(&mut self, id: BodyId) -> crate::error::Result<Body<'_>> {
        Ok(Body::new(BodyProof::acquire(self, id)?))
    }

    /// Acquire a joint capability after validating the id once.
    pub fn joint(&mut self, id: JointId) -> crate::error::Result<crate::joints::Joint<'_>> {
        Ok(crate::joints::Joint::new(JointProof::acquire(self, id)?))
    }

    /// Acquire a shape capability after validating the id once.
    pub fn shape(&mut self, id: ShapeId) -> crate::error::Result<crate::shapes::Shape<'_>> {
        Ok(crate::shapes::Shape::new(ShapeProof::acquire(self, id)?))
    }

    /// Acquire a chain capability after validating the id once.
    pub fn chain(&mut self, id: ChainId) -> crate::error::Result<crate::shapes::chain::Chain<'_>> {
        Ok(crate::shapes::chain::Chain::new(ChainProof::acquire(
            self, id,
        )?))
    }
}
