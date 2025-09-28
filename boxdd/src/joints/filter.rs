use crate::types::BodyId;
use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase};

// Filter joint (no params beyond base)
#[derive(Clone, Debug)]
/// Filter joint definition (maps to `b2FilterJointDef`). A lightweight joint
/// used primarily for contact filtering scenarios.
pub struct FilterJointDef(pub(crate) ffi::b2FilterJointDef);

impl FilterJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2FilterJointDef = unsafe { ffi::b2DefaultFilterJointDef() };
        def.base = base.0;
        Self(def)
    }
}

/// Builder for a filter joint that disables collision between two bodies while keeping them in the same island.
/// Fluent builder for filter joints.
pub struct FilterJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) body_a: BodyId,
    pub(crate) body_b: BodyId,
    pub(crate) def: FilterJointDef,
}

impl<'w> FilterJointBuilder<'w> {
    /// Whether the attached bodies should collide with each other.
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }
    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        let base = super::JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .build();
        self.def.0.base = base.0;
        self.world.create_filter_joint(&self.def)
    }
}
