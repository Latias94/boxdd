use crate::world::World;
use boxdd_sys::ffi;

use super::{Joint, JointBase, OwnedJoint};
use crate::error::ApiResult;

// Filter joint (no params beyond base)
#[derive(Clone, Debug)]
/// Filter joint definition (maps to `b2FilterJointDef`). A lightweight joint
/// used primarily for contact filtering scenarios.
pub struct FilterJointDef {
    base: JointBase,
    raw: ffi::b2FilterJointDef,
}

impl FilterJointDef {
    pub fn new(base: JointBase) -> Self {
        let _lease = crate::core::foundation::assert_transient_native_lease();
        Self {
            base,
            raw: unsafe { ffi::b2DefaultFilterJointDef() },
        }
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
        let mut raw = self.raw;
        raw.base = self.base.to_raw();
        raw
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
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
    #[must_use]
    pub fn build(self) -> Joint<'w> {
        self.world.create_filter_joint(&self.def)
    }

    pub fn try_build(self) -> ApiResult<Joint<'w>> {
        self.world.try_create_filter_joint(&self.def)
    }

    #[must_use]
    pub fn build_owned(self) -> OwnedJoint {
        self.world.create_filter_joint_owned(&self.def)
    }

    pub fn try_build_owned(self) -> ApiResult<OwnedJoint> {
        self.world.try_create_filter_joint_owned(&self.def)
    }
}
