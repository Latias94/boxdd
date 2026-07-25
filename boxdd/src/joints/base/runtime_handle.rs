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

pub(crate) trait JointRuntimeHandle {
    fn joint_id(&self) -> JointId;
    fn joint_world_core(&self) -> &WorldCore;

    #[inline]
    #[track_caller]
    fn assert_valid(&self) {
        self.check_valid()
            .expect("joint handle is unavailable, foreign, or invalid");
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::joints::runtime::check_joint_access(self.joint_world_core(), self.joint_id(), None)
            .map(|_| ())
    }

    fn is_valid(&self) -> bool {
        self.try_is_valid()
            .expect("joint handle is unavailable or foreign")
    }

    fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        let core = self.joint_world_core();
        core.check_available()?;
        core.joint_is_valid(self.joint_id())
    }

    fn joint_type(&self) -> JointType {
        self.try_joint_type()
            .expect("joint handle is unavailable or Box2D returned an unknown joint type")
    }

    fn try_joint_type(&self) -> ApiResult<JointType> {
        self.check_valid()?;
        try_joint_type_impl(self.joint_world_core(), self.joint_id())
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
        joint_body_a_id_in_impl(self.joint_world_core().brand(), self.joint_id())
    }

    fn try_body_a_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_a_id_in_impl(
            self.joint_world_core().brand(),
            self.joint_id(),
        ))
    }

    fn body_b_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_b_id_in_impl(self.joint_world_core().brand(), self.joint_id())
    }

    fn try_body_b_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_b_id_in_impl(
            self.joint_world_core().brand(),
            self.joint_id(),
        ))
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
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            flag,
            JOINT_SET_COLLIDE_CONNECTED,
        );
    }

    fn try_set_collide_connected(&mut self, flag: bool) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            flag,
            JOINT_SET_COLLIDE_CONNECTED,
        )
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
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            tuning,
            JOINT_SET_CONSTRAINT_TUNING,
        );
    }

    fn try_set_constraint_tuning(&mut self, tuning: ConstraintTuning) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            tuning,
            JOINT_SET_CONSTRAINT_TUNING,
        )
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
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            frame,
            JOINT_SET_LOCAL_FRAME_A,
        );
    }

    fn try_set_local_frame_a(&mut self, frame: crate::Transform) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            frame,
            JOINT_SET_LOCAL_FRAME_A,
        )
    }

    fn set_local_frame_b(&mut self, frame: crate::Transform) {
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            frame,
            JOINT_SET_LOCAL_FRAME_B,
        );
    }

    fn try_set_local_frame_b(&mut self, frame: crate::Transform) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            frame,
            JOINT_SET_LOCAL_FRAME_B,
        )
    }

    fn wake_bodies(&mut self) {
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            (),
            JOINT_WAKE_BODIES,
        );
    }

    fn try_wake_bodies(&mut self) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            (),
            JOINT_WAKE_BODIES,
        )
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
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            threshold,
            JOINT_SET_FORCE_THRESHOLD,
        );
    }

    fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            threshold,
            JOINT_SET_FORCE_THRESHOLD,
        )
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
        crate::joints::runtime::joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            threshold,
            JOINT_SET_TORQUE_THRESHOLD,
        );
    }

    fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        crate::joints::runtime::try_joint_set_checked_in_impl(
            self.joint_world_core(),
            self.joint_id(),
            threshold,
            JOINT_SET_TORQUE_THRESHOLD,
        )
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
        joint_user_data_ptr_raw_checked_impl(self.joint_world_core(), self.joint_id())
    }

    fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_joint_user_data_ptr_raw_impl(self.joint_world_core(), self.joint_id())
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

impl OwnedJoint {
    #[inline]
    pub(in crate::joints) fn runtime_world_core(&self) -> &WorldCore {
        JointRuntimeHandle::joint_world_core(self)
    }
}

impl Joint<'_> {
    #[inline]
    pub(in crate::joints) fn runtime_world_core(&self) -> &WorldCore {
        JointRuntimeHandle::joint_world_core(self)
    }
}
