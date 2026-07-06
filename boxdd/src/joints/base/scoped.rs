use super::runtime_handle::JointRuntimeHandle;
use super::*;
use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::sync::Arc;

impl fmt::Debug for Joint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Joint").field("id", &self.id).finish()
    }
}

impl<'w> Joint<'w> {
    pub(crate) fn new(core: Arc<WorldCore>, id: JointId) -> Self {
        Self {
            id,
            core,
            _world: PhantomData,
        }
    }

    pub fn id(&self) -> JointId {
        self.id
    }

    pub fn is_valid(&self) -> bool {
        JointRuntimeHandle::is_valid(self)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        JointRuntimeHandle::try_is_valid(self)
    }

    pub fn joint_type(&self) -> JointType {
        JointRuntimeHandle::joint_type(self)
    }

    pub fn try_joint_type(&self) -> ApiResult<JointType> {
        JointRuntimeHandle::try_joint_type(self)
    }

    pub fn joint_type_raw(&self) -> ffi::b2JointType {
        JointRuntimeHandle::joint_type_raw(self)
    }

    pub fn try_joint_type_raw(&self) -> ApiResult<ffi::b2JointType> {
        JointRuntimeHandle::try_joint_type_raw(self)
    }

    pub fn body_a_id(&self) -> BodyId {
        JointRuntimeHandle::body_a_id(self)
    }

    pub fn try_body_a_id(&self) -> ApiResult<BodyId> {
        JointRuntimeHandle::try_body_a_id(self)
    }

    pub fn body_b_id(&self) -> BodyId {
        JointRuntimeHandle::body_b_id(self)
    }

    pub fn try_body_b_id(&self) -> ApiResult<BodyId> {
        JointRuntimeHandle::try_body_b_id(self)
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        JointRuntimeHandle::world_id_raw(self)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        JointRuntimeHandle::try_world_id_raw(self)
    }

    pub fn collide_connected(&self) -> bool {
        JointRuntimeHandle::collide_connected(self)
    }

    pub fn try_collide_connected(&self) -> ApiResult<bool> {
        JointRuntimeHandle::try_collide_connected(self)
    }

    pub fn set_collide_connected(&mut self, flag: bool) {
        JointRuntimeHandle::set_collide_connected(self, flag)
    }

    pub fn try_set_collide_connected(&mut self, flag: bool) -> ApiResult<()> {
        JointRuntimeHandle::try_set_collide_connected(self, flag)
    }

    pub fn constraint_tuning(&self) -> ConstraintTuning {
        JointRuntimeHandle::constraint_tuning(self)
    }

    pub fn try_constraint_tuning(&self) -> ApiResult<ConstraintTuning> {
        JointRuntimeHandle::try_constraint_tuning(self)
    }

    pub fn set_constraint_tuning(&mut self, tuning: ConstraintTuning) {
        JointRuntimeHandle::set_constraint_tuning(self, tuning)
    }

    pub fn try_set_constraint_tuning(&mut self, tuning: ConstraintTuning) -> ApiResult<()> {
        JointRuntimeHandle::try_set_constraint_tuning(self, tuning)
    }

    pub fn local_frame_a(&self) -> crate::Transform {
        JointRuntimeHandle::local_frame_a(self)
    }

    pub fn try_local_frame_a(&self) -> ApiResult<crate::Transform> {
        JointRuntimeHandle::try_local_frame_a(self)
    }

    pub fn local_frame_b(&self) -> crate::Transform {
        JointRuntimeHandle::local_frame_b(self)
    }

    pub fn try_local_frame_b(&self) -> ApiResult<crate::Transform> {
        JointRuntimeHandle::try_local_frame_b(self)
    }

    pub fn set_local_frame_a(&mut self, frame: crate::Transform) {
        JointRuntimeHandle::set_local_frame_a(self, frame)
    }

    pub fn try_set_local_frame_a(&mut self, frame: crate::Transform) -> ApiResult<()> {
        JointRuntimeHandle::try_set_local_frame_a(self, frame)
    }

    pub fn set_local_frame_b(&mut self, frame: crate::Transform) {
        JointRuntimeHandle::set_local_frame_b(self, frame)
    }

    pub fn try_set_local_frame_b(&mut self, frame: crate::Transform) -> ApiResult<()> {
        JointRuntimeHandle::try_set_local_frame_b(self, frame)
    }

    pub fn wake_bodies(&mut self) {
        JointRuntimeHandle::wake_bodies(self)
    }

    pub fn try_wake_bodies(&mut self) -> ApiResult<()> {
        JointRuntimeHandle::try_wake_bodies(self)
    }

    pub fn linear_separation(&self) -> f32 {
        JointRuntimeHandle::linear_separation(self)
    }

    pub fn try_linear_separation(&self) -> ApiResult<f32> {
        JointRuntimeHandle::try_linear_separation(self)
    }

    pub fn angular_separation(&self) -> f32 {
        JointRuntimeHandle::angular_separation(self)
    }

    pub fn try_angular_separation(&self) -> ApiResult<f32> {
        JointRuntimeHandle::try_angular_separation(self)
    }

    pub fn constraint_force(&self) -> Vec2 {
        JointRuntimeHandle::constraint_force(self)
    }

    pub fn try_constraint_force(&self) -> ApiResult<Vec2> {
        JointRuntimeHandle::try_constraint_force(self)
    }

    pub fn constraint_torque(&self) -> f32 {
        JointRuntimeHandle::constraint_torque(self)
    }

    pub fn try_constraint_torque(&self) -> ApiResult<f32> {
        JointRuntimeHandle::try_constraint_torque(self)
    }

    pub fn force_threshold(&self) -> f32 {
        JointRuntimeHandle::force_threshold(self)
    }

    pub fn set_force_threshold(&mut self, threshold: f32) {
        JointRuntimeHandle::set_force_threshold(self, threshold)
    }

    pub fn try_force_threshold(&self) -> ApiResult<f32> {
        JointRuntimeHandle::try_force_threshold(self)
    }

    pub fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        JointRuntimeHandle::try_set_force_threshold(self, threshold)
    }

    pub fn torque_threshold(&self) -> f32 {
        JointRuntimeHandle::torque_threshold(self)
    }

    pub fn set_torque_threshold(&mut self, threshold: f32) {
        JointRuntimeHandle::set_torque_threshold(self, threshold)
    }

    pub fn try_torque_threshold(&self) -> ApiResult<f32> {
        JointRuntimeHandle::try_torque_threshold(self)
    }

    pub fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        JointRuntimeHandle::try_set_torque_threshold(self, threshold)
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as Box2D may read it.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { JointRuntimeHandle::set_user_data_ptr_raw(self, p) }
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { JointRuntimeHandle::try_set_user_data_ptr_raw(self, p) }
    }

    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        JointRuntimeHandle::user_data_ptr_raw(self)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        JointRuntimeHandle::try_user_data_ptr_raw(self)
    }

    /// Set typed user data on this joint.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the joint is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        JointRuntimeHandle::set_user_data(self, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        JointRuntimeHandle::try_set_user_data(self, value)
    }

    /// Clear typed user data on this joint. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        JointRuntimeHandle::clear_user_data(self)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        JointRuntimeHandle::try_clear_user_data(self)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        JointRuntimeHandle::with_user_data(self, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        JointRuntimeHandle::try_with_user_data(self, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        JointRuntimeHandle::with_user_data_mut(self, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        JointRuntimeHandle::try_with_user_data_mut(self, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        JointRuntimeHandle::take_user_data(self)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        JointRuntimeHandle::try_take_user_data(self)
    }

    /// Destroy this joint immediately.
    pub fn destroy(self, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(raw_joint_id(self.id)) } {
            unsafe { ffi::b2DestroyJoint(raw_joint_id(self.id), wake_bodies) };
            let _ = self.core.clear_joint_user_data(self.id);
        }
    }

    pub fn try_destroy(self, wake_bodies: bool) -> ApiResult<()> {
        self.check_valid()?;
        if unsafe { ffi::b2Joint_IsValid(raw_joint_id(self.id)) } {
            unsafe { ffi::b2DestroyJoint(raw_joint_id(self.id), wake_bodies) };
            let _ = self.core.clear_joint_user_data(self.id);
        }
        Ok(())
    }
}
