use super::*;

#[inline]
fn weld_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearHertz)
}

#[inline]
fn weld_set_linear_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetLinearHertz)
}

#[inline]
fn weld_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearDampingRatio)
}

#[inline]
fn weld_set_linear_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetLinearDampingRatio)
}

#[inline]
fn weld_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularHertz)
}

#[inline]
fn weld_set_angular_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetAngularHertz)
}

#[inline]
fn weld_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularDampingRatio)
}

#[inline]
fn weld_set_angular_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetAngularDampingRatio)
}

trait WeldJointRuntimeHandle {
    fn weld_joint_id(&self) -> JointId;

    fn weld_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn weld_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_linear_hertz_impl,
        );
    }

    fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_linear_hertz_impl,
        )
    }

    fn weld_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        );
    }

    fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    fn weld_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn weld_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_angular_hertz_impl,
        );
    }

    fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_angular_hertz_impl,
        )
    }

    fn weld_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        );
    }

    fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }
}

impl WeldJointRuntimeHandle for OwnedJoint {
    fn weld_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> WeldJointRuntimeHandle for Joint<'w> {
    fn weld_joint_id(&self) -> JointId {
        self.id()
    }
}

impl World {
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_linear_hertz_impl)
    }

    pub fn try_weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_linear_hertz_impl)
    }

    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    pub fn try_weld_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_angular_hertz_impl)
    }

    pub fn try_weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_angular_hertz_impl)
    }

    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }

    pub fn try_weld_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }
}

impl WorldHandle {
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }
}

impl OwnedJoint {
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
}

impl<'w> Joint<'w> {
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
}
