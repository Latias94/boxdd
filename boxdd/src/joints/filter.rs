use crate::JointId;
use crate::world::World;
use boxdd_sys::ffi;

use super::JointBase;
use crate::error::Result;

// Filter joint (no params beyond base)
#[derive(Clone, Debug)]
/// Filter joint definition (maps to `b2FilterJointDef`). A lightweight joint
/// used primarily for contact filtering scenarios.
pub struct FilterJointDef {
    base: JointBase,
}

impl FilterJointDef {
    pub fn new(base: JointBase) -> Self {
        Self { base }
    }

    #[inline]
    pub fn base(&self) -> &JointBase {
        &self.base
    }

    #[inline]
    pub(crate) fn base_mut(&mut self) -> &mut JointBase {
        &mut self.base
    }

    pub(crate) fn to_raw(&self) -> ffi::b2FilterJointDef {
        crate::core::native_defaults::filter_joint_def(self.base.to_raw())
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        super::check_filter_joint_def_valid(self)
    }
}

/// Builder for a filter joint that disables collision between two bodies while keeping them in the same island.
/// Fluent builder for filter joints.
pub struct FilterJointBuilder<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) def: FilterJointDef,
}

impl<'w> FilterJointBuilder<'w> {
    /// Whether the attached bodies should collide with each other.
    pub fn collide_connected(mut self, flag: bool) -> Self {
        let base = *self.def.base();
        *self.def.base_mut() = base.with_collide_connected(flag);
        self
    }
    pub fn build(self) -> Result<JointId> {
        self.world.create_filter_joint(&self.def)
    }
}
