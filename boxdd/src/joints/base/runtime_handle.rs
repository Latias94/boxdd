use super::user_data::{
    joint_clear_user_data_checked_impl, joint_set_user_data_checked_impl,
    joint_set_user_data_ptr_raw_checked_impl, joint_take_user_data_checked_impl,
    joint_user_data_ptr_raw_checked_impl, joint_with_user_data_checked_impl,
    joint_with_user_data_mut_checked_impl, try_joint_clear_user_data_checked_impl,
    try_joint_set_user_data_checked_impl, try_joint_set_user_data_ptr_raw_impl,
    try_joint_take_user_data_checked_impl, try_joint_user_data_ptr_raw_impl,
    try_joint_with_user_data_checked_impl, try_joint_with_user_data_mut_checked_impl,
};
use super::*;
use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use std::os::raw::c_void;

fn joint_is_valid_checked_impl(id: JointId) -> ApiResult<bool> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(joint_is_valid_impl(id))
}

fn joint_is_valid_panicking_impl(id: JointId) -> bool {
    crate::core::callback_state::assert_not_in_callback();
    joint_is_valid_impl(id)
}

pub(crate) trait JointRuntimeHandle {
    fn joint_id(&self) -> JointId;
    fn joint_world_core(&self) -> &WorldCore;

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_joint_valid(self.joint_id());
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_joint_valid(self.joint_id())
    }

    fn is_valid(&self) -> bool {
        joint_is_valid_panicking_impl(self.joint_id())
    }

    fn try_is_valid(&self) -> ApiResult<bool> {
        joint_is_valid_checked_impl(self.joint_id())
    }

    fn joint_type(&self) -> JointType {
        self.assert_valid();
        joint_type_impl(self.joint_id())
    }

    fn try_joint_type(&self) -> ApiResult<JointType> {
        self.check_valid()?;
        Ok(joint_type_impl(self.joint_id()))
    }

    fn joint_type_raw(&self) -> ffi::b2JointType {
        self.assert_valid();
        joint_type_raw_impl(self.joint_id())
    }

    fn try_joint_type_raw(&self) -> ApiResult<ffi::b2JointType> {
        self.check_valid()?;
        Ok(joint_type_raw_impl(self.joint_id()))
    }

    fn body_a_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_a_id_impl(self.joint_id())
    }

    fn try_body_a_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_a_id_impl(self.joint_id()))
    }

    fn body_b_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_b_id_impl(self.joint_id())
    }

    fn try_body_b_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_b_id_impl(self.joint_id()))
    }

    fn world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        joint_world_id_raw_impl(self.joint_id())
    }

    fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(joint_world_id_raw_impl(self.joint_id()))
    }

    fn collide_connected(&self) -> bool {
        self.assert_valid();
        joint_collide_connected_impl(self.joint_id())
    }

    fn try_collide_connected(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(joint_collide_connected_impl(self.joint_id()))
    }

    fn set_collide_connected(&mut self, flag: bool) {
        self.assert_valid();
        joint_set_collide_connected_impl(self.joint_id(), flag);
    }

    fn try_set_collide_connected(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_collide_connected_impl(self.joint_id(), flag);
        Ok(())
    }

    fn constraint_tuning(&self) -> ConstraintTuning {
        self.assert_valid();
        joint_constraint_tuning_impl(self.joint_id())
    }

    fn try_constraint_tuning(&self) -> ApiResult<ConstraintTuning> {
        self.check_valid()?;
        Ok(joint_constraint_tuning_impl(self.joint_id()))
    }

    fn set_constraint_tuning(&mut self, tuning: ConstraintTuning) {
        self.assert_valid();
        joint_set_constraint_tuning_impl(self.joint_id(), tuning);
    }

    fn try_set_constraint_tuning(&mut self, tuning: ConstraintTuning) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_constraint_tuning_impl(self.joint_id(), tuning);
        Ok(())
    }

    fn local_frame_a(&self) -> crate::Transform {
        self.assert_valid();
        joint_local_frame_a_impl(self.joint_id())
    }

    fn try_local_frame_a(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(joint_local_frame_a_impl(self.joint_id()))
    }

    fn local_frame_b(&self) -> crate::Transform {
        self.assert_valid();
        joint_local_frame_b_impl(self.joint_id())
    }

    fn try_local_frame_b(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(joint_local_frame_b_impl(self.joint_id()))
    }

    fn set_local_frame_a(&mut self, frame: crate::Transform) {
        self.assert_valid();
        assert_joint_local_frame_valid(frame);
        joint_set_local_frame_a_impl(self.joint_id(), frame);
    }

    fn try_set_local_frame_a(&mut self, frame: crate::Transform) -> ApiResult<()> {
        self.check_valid()?;
        check_joint_local_frame_valid(frame)?;
        joint_set_local_frame_a_impl(self.joint_id(), frame);
        Ok(())
    }

    fn set_local_frame_b(&mut self, frame: crate::Transform) {
        self.assert_valid();
        assert_joint_local_frame_valid(frame);
        joint_set_local_frame_b_impl(self.joint_id(), frame);
    }

    fn try_set_local_frame_b(&mut self, frame: crate::Transform) -> ApiResult<()> {
        self.check_valid()?;
        check_joint_local_frame_valid(frame)?;
        joint_set_local_frame_b_impl(self.joint_id(), frame);
        Ok(())
    }

    fn wake_bodies(&mut self) {
        self.assert_valid();
        joint_wake_bodies_impl(self.joint_id());
    }

    fn try_wake_bodies(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        joint_wake_bodies_impl(self.joint_id());
        Ok(())
    }

    fn linear_separation(&self) -> f32 {
        self.assert_valid();
        joint_linear_separation_impl(self.joint_id())
    }

    fn try_linear_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_linear_separation_impl(self.joint_id()))
    }

    fn angular_separation(&self) -> f32 {
        self.assert_valid();
        joint_angular_separation_impl(self.joint_id())
    }

    fn try_angular_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_angular_separation_impl(self.joint_id()))
    }

    fn constraint_force(&self) -> Vec2 {
        self.assert_valid();
        joint_constraint_force_impl(self.joint_id())
    }

    fn try_constraint_force(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(joint_constraint_force_impl(self.joint_id()))
    }

    fn constraint_torque(&self) -> f32 {
        self.assert_valid();
        joint_constraint_torque_impl(self.joint_id())
    }

    fn try_constraint_torque(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_constraint_torque_impl(self.joint_id()))
    }

    fn force_threshold(&self) -> f32 {
        self.assert_valid();
        joint_force_threshold_impl(self.joint_id())
    }

    fn try_force_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_force_threshold_impl(self.joint_id()))
    }

    fn set_force_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        joint_set_force_threshold_impl(self.joint_id(), threshold);
    }

    fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_force_threshold_impl(self.joint_id(), threshold);
        Ok(())
    }

    fn torque_threshold(&self) -> f32 {
        self.assert_valid();
        joint_torque_threshold_impl(self.joint_id())
    }

    fn try_torque_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_torque_threshold_impl(self.joint_id()))
    }

    fn set_torque_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        joint_set_torque_threshold_impl(self.joint_id(), threshold);
    }

    fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_torque_threshold_impl(self.joint_id(), threshold);
        Ok(())
    }

    unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe {
            joint_set_user_data_ptr_raw_checked_impl(self.joint_world_core(), self.joint_id(), p)
        }
    }

    unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { try_joint_set_user_data_ptr_raw_impl(self.joint_world_core(), self.joint_id(), p) }
    }

    fn user_data_ptr_raw(&self) -> *mut c_void {
        joint_user_data_ptr_raw_checked_impl(self.joint_id())
    }

    fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_joint_user_data_ptr_raw_impl(self.joint_id())
    }

    fn set_user_data<T: 'static>(&mut self, value: T) {
        joint_set_user_data_checked_impl(self.joint_world_core(), self.joint_id(), value);
    }

    fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_joint_set_user_data_checked_impl(self.joint_world_core(), self.joint_id(), value)
    }

    fn clear_user_data(&mut self) -> bool {
        joint_clear_user_data_checked_impl(self.joint_world_core(), self.joint_id())
    }

    fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_joint_clear_user_data_checked_impl(self.joint_world_core(), self.joint_id())
    }

    fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        joint_with_user_data_checked_impl(self.joint_world_core(), self.joint_id(), f)
    }

    fn try_with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> ApiResult<Option<R>> {
        try_joint_with_user_data_checked_impl(self.joint_world_core(), self.joint_id(), f)
    }

    fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        joint_with_user_data_mut_checked_impl(self.joint_world_core(), self.joint_id(), f)
    }

    fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_joint_with_user_data_mut_checked_impl(self.joint_world_core(), self.joint_id(), f)
    }

    fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        joint_take_user_data_checked_impl(self.joint_world_core(), self.joint_id())
    }

    fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_joint_take_user_data_checked_impl(self.joint_world_core(), self.joint_id())
    }
}

impl JointRuntimeHandle for OwnedJoint {
    fn joint_id(&self) -> JointId {
        self.id
    }

    fn joint_world_core(&self) -> &WorldCore {
        self.core.as_ref()
    }
}

impl<'w> JointRuntimeHandle for Joint<'w> {
    fn joint_id(&self) -> JointId {
        self.id
    }

    fn joint_world_core(&self) -> &WorldCore {
        self.core.as_ref()
    }
}
