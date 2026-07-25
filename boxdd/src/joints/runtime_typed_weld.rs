use super::*;

#[inline]
fn weld_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearHertz)
}

const WELD_SET_LINEAR_HERTZ: JointSetOp<f32> = JointSetOp::new(JointWriteKind::WeldSetLinearHertz);

#[inline]
fn weld_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearDampingRatio)
}

const WELD_SET_LINEAR_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::WeldSetLinearDampingRatio);

#[inline]
fn weld_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularHertz)
}

const WELD_SET_ANGULAR_HERTZ: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::WeldSetAngularHertz);

#[inline]
fn weld_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularDampingRatio)
}

const WELD_SET_ANGULAR_DAMPING_RATIO: JointSetOp<f32> =
    JointSetOp::new(JointWriteKind::WeldSetAngularDampingRatio);

trait WeldJointRuntimeHandle: TypedJointRuntimeHandle {
    fn weld_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn weld_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            hertz,
            WELD_SET_LINEAR_HERTZ,
        );
    }

    fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            hertz,
            WELD_SET_LINEAR_HERTZ,
        )
    }

    fn weld_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            damping_ratio,
            WELD_SET_LINEAR_DAMPING_RATIO,
        );
    }

    fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            damping_ratio,
            WELD_SET_LINEAR_DAMPING_RATIO,
        )
    }

    fn weld_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn weld_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            hertz,
            WELD_SET_ANGULAR_HERTZ,
        );
    }

    fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            hertz,
            WELD_SET_ANGULAR_HERTZ,
        )
    }

    fn weld_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            damping_ratio,
            WELD_SET_ANGULAR_DAMPING_RATIO,
        );
    }

    fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.typed_joint_world_core(),
            self.typed_joint_id(),
            JointType::Weld,
            damping_ratio,
            WELD_SET_ANGULAR_DAMPING_RATIO,
        )
    }
}

impl WeldJointRuntimeHandle for OwnedJoint {}

impl WeldJointRuntimeHandle for Joint<'_> {}

impl World {
    /// Returns the selected weld joint's linear spring frequency in hertz.
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Weld, weld_linear_hertz_impl)
    }

    /// Fallible variant of weld_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(self.core(), id, JointType::Weld, weld_linear_hertz_impl)
    }

    /// Sets the selected weld joint's linear spring frequency in hertz; the value must be finite and non-negative.
    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            hertz,
            WELD_SET_LINEAR_HERTZ,
        )
    }

    /// Fallible variant of weld_set_linear_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            hertz,
            WELD_SET_LINEAR_HERTZ,
        )
    }

    /// Returns the selected weld joint's linear spring damping ratio.
    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    /// Fallible variant of weld_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    /// Sets the selected weld joint's linear spring damping ratio; the value must be finite and non-negative.
    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            damping_ratio,
            WELD_SET_LINEAR_DAMPING_RATIO,
        )
    }

    /// Fallible variant of weld_set_linear_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            damping_ratio,
            WELD_SET_LINEAR_DAMPING_RATIO,
        )
    }

    /// Returns the selected weld joint's angular spring frequency in hertz.
    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Weld, weld_angular_hertz_impl)
    }

    /// Fallible variant of weld_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    /// Sets the selected weld joint's angular spring frequency in hertz; the value must be finite and non-negative.
    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            hertz,
            WELD_SET_ANGULAR_HERTZ,
        )
    }

    /// Fallible variant of weld_set_angular_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            hertz,
            WELD_SET_ANGULAR_HERTZ,
        )
    }

    /// Returns the selected weld joint's angular spring damping ratio.
    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    /// Fallible variant of weld_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    /// Sets the selected weld joint's angular spring damping ratio; the value must be finite and non-negative.
    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            damping_ratio,
            WELD_SET_ANGULAR_DAMPING_RATIO,
        )
    }

    /// Fallible variant of weld_set_angular_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            damping_ratio,
            WELD_SET_ANGULAR_DAMPING_RATIO,
        )
    }
}

impl WorldHandle {
    /// Returns the selected weld joint's linear spring frequency in hertz.
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Weld, weld_linear_hertz_impl)
    }

    /// Fallible variant of weld_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(self.core(), id, JointType::Weld, weld_linear_hertz_impl)
    }

    /// Returns the selected weld joint's linear spring damping ratio.
    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    /// Fallible variant of weld_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    /// Returns the selected weld joint's angular spring frequency in hertz.
    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(self.core(), id, JointType::Weld, weld_angular_hertz_impl)
    }

    /// Fallible variant of weld_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    /// Returns the selected weld joint's angular spring damping ratio.
    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    /// Fallible variant of weld_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_in_impl(
            self.core(),
            id,
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }
}

impl OwnedJoint {
    /// Returns the selected weld joint's linear spring frequency in hertz.
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    /// Fallible variant of weld_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    /// Sets the selected weld joint's linear spring frequency in hertz; the value must be finite and non-negative.
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    /// Fallible variant of weld_set_linear_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    /// Returns the selected weld joint's linear spring damping ratio.
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    /// Fallible variant of weld_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    /// Sets the selected weld joint's linear spring damping ratio; the value must be finite and non-negative.
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of weld_set_linear_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    /// Returns the selected weld joint's angular spring frequency in hertz.
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    /// Fallible variant of weld_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    /// Sets the selected weld joint's angular spring frequency in hertz; the value must be finite and non-negative.
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    /// Fallible variant of weld_set_angular_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    /// Returns the selected weld joint's angular spring damping ratio.
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    /// Fallible variant of weld_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    /// Sets the selected weld joint's angular spring damping ratio; the value must be finite and non-negative.
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of weld_set_angular_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
}

impl<'w> Joint<'w> {
    /// Returns the selected weld joint's linear spring frequency in hertz.
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    /// Fallible variant of weld_linear_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    /// Sets the selected weld joint's linear spring frequency in hertz; the value must be finite and non-negative.
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    /// Fallible variant of weld_set_linear_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    /// Returns the selected weld joint's linear spring damping ratio.
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    /// Fallible variant of weld_linear_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    /// Sets the selected weld joint's linear spring damping ratio; the value must be finite and non-negative.
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of weld_set_linear_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    /// Returns the selected weld joint's angular spring frequency in hertz.
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    /// Fallible variant of weld_angular_hertz; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    /// Sets the selected weld joint's angular spring frequency in hertz; the value must be finite and non-negative.
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    /// Fallible variant of weld_set_angular_hertz; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    /// Returns the selected weld joint's angular spring damping ratio.
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    /// Fallible variant of weld_angular_damping_ratio; reports lifecycle, identity, or joint-kind errors instead of panicking.
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    /// Sets the selected weld joint's angular spring damping ratio; the value must be finite and non-negative.
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    /// Fallible variant of weld_set_angular_damping_ratio; reports lifecycle, identity, joint-kind, or argument errors instead of panicking.
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
}
