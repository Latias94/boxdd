use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::body::Body;
use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::fmt;
use std::os::raw::c_void;

/// A scoped joint handle tied to a mutable borrow of the world.
pub struct Joint<'w> {
    pub(crate) id: JointId,
    pub(crate) core: Arc<crate::core::world_core::WorldCore>,
    pub(crate) _world: PhantomData<&'w World>,
}

impl fmt::Debug for Joint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Joint").field("id", &self.id).finish()
    }
}

/// A RAII-owned joint that is destroyed on drop.
pub struct OwnedJoint {
    id: JointId,
    core: Arc<WorldCore>,
    destroy_on_drop: bool,
    wake_bodies_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

/// Joint kinds reported by Box2D.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JointType {
    Distance,
    Filter,
    Motor,
    Prismatic,
    Revolute,
    Weld,
    Wheel,
}

impl JointType {
    #[inline]
    pub const fn from_raw(raw: ffi::b2JointType) -> Option<Self> {
        match raw {
            ffi::b2JointType_b2_distanceJoint => Some(Self::Distance),
            ffi::b2JointType_b2_filterJoint => Some(Self::Filter),
            ffi::b2JointType_b2_motorJoint => Some(Self::Motor),
            ffi::b2JointType_b2_prismaticJoint => Some(Self::Prismatic),
            ffi::b2JointType_b2_revoluteJoint => Some(Self::Revolute),
            ffi::b2JointType_b2_weldJoint => Some(Self::Weld),
            ffi::b2JointType_b2_wheelJoint => Some(Self::Wheel),
            _ => None,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2JointType {
        match self {
            Self::Distance => ffi::b2JointType_b2_distanceJoint,
            Self::Filter => ffi::b2JointType_b2_filterJoint,
            Self::Motor => ffi::b2JointType_b2_motorJoint,
            Self::Prismatic => ffi::b2JointType_b2_prismaticJoint,
            Self::Revolute => ffi::b2JointType_b2_revoluteJoint,
            Self::Weld => ffi::b2JointType_b2_weldJoint,
            Self::Wheel => ffi::b2JointType_b2_wheelJoint,
        }
    }
}

impl TryFrom<ffi::b2JointType> for JointType {
    type Error = ffi::b2JointType;

    #[inline]
    fn try_from(value: ffi::b2JointType) -> Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}

#[inline]
fn joint_type_from_ffi(raw: ffi::b2JointType) -> JointType {
    JointType::from_raw(raw).expect("Box2D returned an unknown joint type")
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

/// Shared constraint tuning (Hertz + damping ratio) used by Box2D joints.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ConstraintTuning {
    pub hertz: f32,
    pub damping_ratio: f32,
}

impl ConstraintTuning {
    #[inline]
    pub const fn new(hertz: f32, damping_ratio: f32) -> Self {
        Self {
            hertz,
            damping_ratio,
        }
    }
}

#[inline]
pub(crate) fn joint_is_valid_impl(id: JointId) -> bool {
    unsafe { ffi::b2Joint_IsValid(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_type_raw_impl(id: JointId) -> ffi::b2JointType {
    unsafe { ffi::b2Joint_GetType(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_type_impl(id: JointId) -> JointType {
    joint_type_from_ffi(joint_type_raw_impl(id))
}

#[inline]
pub(crate) fn joint_body_a_id_impl(id: JointId) -> BodyId {
    BodyId::from_raw(unsafe { ffi::b2Joint_GetBodyA(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_body_b_id_impl(id: JointId) -> BodyId {
    BodyId::from_raw(unsafe { ffi::b2Joint_GetBodyB(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_linear_separation_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetLinearSeparation(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_angular_separation_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetAngularSeparation(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_constraint_force_impl(id: JointId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Joint_GetConstraintForce(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_constraint_torque_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetConstraintTorque(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_collide_connected_impl(id: JointId) -> bool {
    unsafe { ffi::b2Joint_GetCollideConnected(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_set_collide_connected_impl(id: JointId, flag: bool) {
    unsafe { ffi::b2Joint_SetCollideConnected(raw_joint_id(id), flag) }
}

#[inline]
pub(crate) fn joint_constraint_tuning_impl(id: JointId) -> ConstraintTuning {
    let mut hertz = 0.0f32;
    let mut damping_ratio = 0.0f32;
    unsafe { ffi::b2Joint_GetConstraintTuning(raw_joint_id(id), &mut hertz, &mut damping_ratio) };
    ConstraintTuning::new(hertz, damping_ratio)
}

#[inline]
pub(crate) fn joint_set_constraint_tuning_impl(id: JointId, tuning: ConstraintTuning) {
    unsafe {
        ffi::b2Joint_SetConstraintTuning(raw_joint_id(id), tuning.hertz, tuning.damping_ratio)
    }
}

#[inline]
pub(crate) fn joint_local_frame_a_impl(id: JointId) -> crate::Transform {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameA(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_local_frame_b_impl(id: JointId) -> crate::Transform {
    crate::Transform::from_raw(unsafe { ffi::b2Joint_GetLocalFrameB(raw_joint_id(id)) })
}

#[inline]
pub(crate) fn joint_wake_bodies_impl(id: JointId) {
    unsafe { ffi::b2Joint_WakeBodies(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_force_threshold_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetForceThreshold(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_set_force_threshold_impl(id: JointId, threshold: f32) {
    unsafe { ffi::b2Joint_SetForceThreshold(raw_joint_id(id), threshold) }
}

#[inline]
pub(crate) fn joint_torque_threshold_impl(id: JointId) -> f32 {
    unsafe { ffi::b2Joint_GetTorqueThreshold(raw_joint_id(id)) }
}

#[inline]
pub(crate) fn joint_set_torque_threshold_impl(id: JointId, threshold: f32) {
    unsafe { ffi::b2Joint_SetTorqueThreshold(raw_joint_id(id), threshold) }
}

unsafe fn joint_set_user_data_ptr_impl(
    world_core: &WorldCore,
    id: JointId,
    user_data: *mut c_void,
) {
    let _ = world_core.clear_joint_user_data(id);
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), user_data) }
}

#[inline]
fn joint_user_data_ptr_impl(id: JointId) -> *mut c_void {
    unsafe { ffi::b2Joint_GetUserData(raw_joint_id(id)) }
}

fn joint_set_user_data_impl<T: 'static>(world_core: &WorldCore, id: JointId, value: T) {
    let user_data = world_core.set_joint_user_data(id, value);
    unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), user_data) };
}

fn joint_clear_user_data_impl(world_core: &WorldCore, id: JointId) -> bool {
    let had = world_core.clear_joint_user_data(id);
    if had {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), core::ptr::null_mut()) };
    }
    had
}

fn joint_with_user_data_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_joint_user_data(id, f)
}

fn joint_with_user_data_mut_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    world_core.try_with_joint_user_data_mut(id, f)
}

fn joint_take_user_data_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<Option<T>> {
    let value = world_core.take_joint_user_data::<T>(id)?;
    if value.is_some() {
        unsafe { ffi::b2Joint_SetUserData(raw_joint_id(id), core::ptr::null_mut()) };
    }
    Ok(value)
}

fn joint_is_valid_checked_impl(id: JointId) -> bool {
    crate::core::callback_state::assert_not_in_callback();
    joint_is_valid_impl(id)
}

fn try_joint_is_valid_impl(id: JointId) -> ApiResult<bool> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(joint_is_valid_impl(id))
}

unsafe fn joint_set_user_data_ptr_raw_checked_impl(
    world_core: &WorldCore,
    id: JointId,
    p: *mut c_void,
) {
    crate::core::debug_checks::assert_joint_valid(id);
    unsafe { joint_set_user_data_ptr_impl(world_core, id, p) }
}

unsafe fn try_joint_set_user_data_ptr_raw_impl(
    world_core: &WorldCore,
    id: JointId,
    p: *mut c_void,
) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)?;
    unsafe { joint_set_user_data_ptr_impl(world_core, id, p) }
    Ok(())
}

fn joint_user_data_ptr_raw_checked_impl(id: JointId) -> *mut c_void {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_user_data_ptr_impl(id)
}

fn try_joint_user_data_ptr_raw_impl(id: JointId) -> ApiResult<*mut c_void> {
    crate::core::debug_checks::check_joint_valid(id)?;
    Ok(joint_user_data_ptr_impl(id))
}

fn joint_set_user_data_checked_impl<T: 'static>(world_core: &WorldCore, id: JointId, value: T) {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_set_user_data_impl(world_core, id, value);
}

fn try_joint_set_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
    value: T,
) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_set_user_data_impl(world_core, id, value);
    Ok(())
}

fn joint_clear_user_data_checked_impl(world_core: &WorldCore, id: JointId) -> bool {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_clear_user_data_impl(world_core, id)
}

fn try_joint_clear_user_data_checked_impl(world_core: &WorldCore, id: JointId) -> ApiResult<bool> {
    crate::core::debug_checks::check_joint_valid(id)?;
    Ok(joint_clear_user_data_impl(world_core, id))
}

fn joint_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_with_user_data_impl(world_core, id, f).expect("user data type mismatch")
}

fn try_joint_with_user_data_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_with_user_data_impl(world_core, id, f)
}

fn joint_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_with_user_data_mut_impl(world_core, id, f).expect("user data type mismatch")
}

fn try_joint_with_user_data_mut_checked_impl<T: 'static, R>(
    world_core: &WorldCore,
    id: JointId,
    f: impl FnOnce(&mut T) -> R,
) -> ApiResult<Option<R>> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_with_user_data_mut_impl(world_core, id, f)
}

fn joint_take_user_data_checked_impl<T: 'static>(world_core: &WorldCore, id: JointId) -> Option<T> {
    crate::core::debug_checks::assert_joint_valid(id);
    joint_take_user_data_impl(world_core, id).expect("user data type mismatch")
}

fn try_joint_take_user_data_checked_impl<T: 'static>(
    world_core: &WorldCore,
    id: JointId,
) -> ApiResult<Option<T>> {
    crate::core::debug_checks::check_joint_valid(id)?;
    joint_take_user_data_impl(world_core, id)
}

impl OwnedJoint {
    pub(crate) fn new(core: Arc<WorldCore>, id: JointId) -> Self {
        core.owned_joints
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            core,
            destroy_on_drop: true,
            wake_bodies_on_drop: true,
            _not_send: PhantomData,
        }
    }

    pub fn id(&self) -> JointId {
        self.id
    }

    pub fn is_valid(&self) -> bool {
        joint_is_valid_checked_impl(self.id)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        try_joint_is_valid_impl(self.id)
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_joint_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_joint_valid(self.id)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> JointId {
        self.id
    }

    pub fn joint_type(&self) -> JointType {
        self.assert_valid();
        joint_type_impl(self.id)
    }

    pub fn try_joint_type(&self) -> ApiResult<JointType> {
        self.check_valid()?;
        Ok(joint_type_impl(self.id))
    }

    pub fn joint_type_raw(&self) -> ffi::b2JointType {
        self.assert_valid();
        joint_type_raw_impl(self.id)
    }

    pub fn try_joint_type_raw(&self) -> ApiResult<ffi::b2JointType> {
        self.check_valid()?;
        Ok(joint_type_raw_impl(self.id))
    }

    pub fn body_a_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_a_id_impl(self.id)
    }

    pub fn try_body_a_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_a_id_impl(self.id))
    }

    pub fn body_b_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_b_id_impl(self.id)
    }

    pub fn try_body_b_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_b_id_impl(self.id))
    }

    pub fn collide_connected(&self) -> bool {
        self.assert_valid();
        joint_collide_connected_impl(self.id)
    }

    pub fn try_collide_connected(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(joint_collide_connected_impl(self.id))
    }

    pub fn set_collide_connected(&mut self, flag: bool) {
        self.assert_valid();
        joint_set_collide_connected_impl(self.id, flag)
    }

    pub fn try_set_collide_connected(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_collide_connected_impl(self.id, flag);
        Ok(())
    }

    pub fn constraint_tuning(&self) -> ConstraintTuning {
        self.assert_valid();
        joint_constraint_tuning_impl(self.id)
    }

    pub fn try_constraint_tuning(&self) -> ApiResult<ConstraintTuning> {
        self.check_valid()?;
        Ok(joint_constraint_tuning_impl(self.id))
    }

    pub fn set_constraint_tuning(&mut self, tuning: ConstraintTuning) {
        self.assert_valid();
        joint_set_constraint_tuning_impl(self.id, tuning)
    }

    pub fn try_set_constraint_tuning(&mut self, tuning: ConstraintTuning) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_constraint_tuning_impl(self.id, tuning);
        Ok(())
    }

    pub fn local_frame_a(&self) -> crate::Transform {
        self.assert_valid();
        joint_local_frame_a_impl(self.id)
    }

    pub fn try_local_frame_a(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(joint_local_frame_a_impl(self.id))
    }

    pub fn local_frame_b(&self) -> crate::Transform {
        self.assert_valid();
        joint_local_frame_b_impl(self.id)
    }

    pub fn try_local_frame_b(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(joint_local_frame_b_impl(self.id))
    }

    pub fn wake_bodies(&mut self) {
        self.assert_valid();
        joint_wake_bodies_impl(self.id)
    }

    pub fn try_wake_bodies(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        joint_wake_bodies_impl(self.id);
        Ok(())
    }

    pub fn linear_separation(&self) -> f32 {
        self.assert_valid();
        joint_linear_separation_impl(self.id)
    }

    pub fn try_linear_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_linear_separation_impl(self.id))
    }

    pub fn angular_separation(&self) -> f32 {
        self.assert_valid();
        joint_angular_separation_impl(self.id)
    }

    pub fn try_angular_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_angular_separation_impl(self.id))
    }

    pub fn constraint_force(&self) -> Vec2 {
        self.assert_valid();
        joint_constraint_force_impl(self.id)
    }

    pub fn try_constraint_force(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(joint_constraint_force_impl(self.id))
    }

    pub fn constraint_torque(&self) -> f32 {
        self.assert_valid();
        joint_constraint_torque_impl(self.id)
    }

    pub fn try_constraint_torque(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_constraint_torque_impl(self.id))
    }

    pub fn force_threshold(&self) -> f32 {
        self.assert_valid();
        joint_force_threshold_impl(self.id)
    }
    pub fn try_force_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_force_threshold_impl(self.id))
    }
    pub fn set_force_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        joint_set_force_threshold_impl(self.id, threshold)
    }
    pub fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_force_threshold_impl(self.id, threshold);
        Ok(())
    }
    pub fn torque_threshold(&self) -> f32 {
        self.assert_valid();
        joint_torque_threshold_impl(self.id)
    }
    pub fn try_torque_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_torque_threshold_impl(self.id))
    }
    pub fn set_torque_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        joint_set_torque_threshold_impl(self.id, threshold)
    }
    pub fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_torque_threshold_impl(self.id, threshold);
        Ok(())
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as Box2D may read it.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { joint_set_user_data_ptr_raw_checked_impl(self.core.as_ref(), self.id, p) }
    }
    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { try_joint_set_user_data_ptr_raw_impl(self.core.as_ref(), self.id, p) }
    }
    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        joint_user_data_ptr_raw_checked_impl(self.id)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_joint_user_data_ptr_raw_impl(self.id)
    }

    /// Set typed user data on this joint.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the joint is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        joint_set_user_data_checked_impl(self.core.as_ref(), self.id, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_joint_set_user_data_checked_impl(self.core.as_ref(), self.id, value)
    }

    /// Clear typed user data on this joint. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        joint_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_joint_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        joint_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        try_joint_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        joint_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_joint_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        joint_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_joint_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn wake_bodies_on_drop(mut self, flag: bool) -> Self {
        self.wake_bodies_on_drop = flag;
        self
    }

    pub fn into_id(mut self) -> JointId {
        self.destroy_on_drop = false;
        self.id
    }

    pub fn destroy(mut self, wake_bodies: bool) {
        if self.destroy_on_drop && unsafe { ffi::b2Joint_IsValid(raw_joint_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Joint {
                        id: self.id,
                        wake_bodies,
                    });
            } else {
                unsafe { ffi::b2DestroyJoint(raw_joint_id(self.id), wake_bodies) };
                let _ = self.core.clear_joint_user_data(self.id);
            }
        }
        self.destroy_on_drop = false;
    }
}

impl Drop for OwnedJoint {
    fn drop(&mut self) {
        let _ = self.core.id;
        let prev = self
            .core
            .owned_joints
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug_assert!(prev > 0, "owned joint counter underflow");
        if self.destroy_on_drop && unsafe { ffi::b2Joint_IsValid(raw_joint_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Joint {
                        id: self.id,
                        wake_bodies: self.wake_bodies_on_drop,
                    });
            } else {
                unsafe { ffi::b2DestroyJoint(raw_joint_id(self.id), self.wake_bodies_on_drop) };
                let _ = self.core.clear_joint_user_data(self.id);
            }
        }
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

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_joint_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_joint_valid(self.id)
    }

    pub fn id(&self) -> JointId {
        self.id
    }

    pub fn is_valid(&self) -> bool {
        joint_is_valid_checked_impl(self.id)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        try_joint_is_valid_impl(self.id)
    }

    pub fn joint_type(&self) -> JointType {
        self.assert_valid();
        joint_type_impl(self.id)
    }

    pub fn try_joint_type(&self) -> ApiResult<JointType> {
        self.check_valid()?;
        Ok(joint_type_impl(self.id))
    }

    pub fn joint_type_raw(&self) -> ffi::b2JointType {
        self.assert_valid();
        joint_type_raw_impl(self.id)
    }

    pub fn try_joint_type_raw(&self) -> ApiResult<ffi::b2JointType> {
        self.check_valid()?;
        Ok(joint_type_raw_impl(self.id))
    }

    pub fn body_a_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_a_id_impl(self.id)
    }

    pub fn try_body_a_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_a_id_impl(self.id))
    }

    pub fn body_b_id(&self) -> BodyId {
        self.assert_valid();
        joint_body_b_id_impl(self.id)
    }

    pub fn try_body_b_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(joint_body_b_id_impl(self.id))
    }

    pub fn collide_connected(&self) -> bool {
        self.assert_valid();
        joint_collide_connected_impl(self.id)
    }

    pub fn try_collide_connected(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(joint_collide_connected_impl(self.id))
    }

    pub fn set_collide_connected(&mut self, flag: bool) {
        self.assert_valid();
        joint_set_collide_connected_impl(self.id, flag)
    }

    pub fn try_set_collide_connected(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_collide_connected_impl(self.id, flag);
        Ok(())
    }

    pub fn constraint_tuning(&self) -> ConstraintTuning {
        self.assert_valid();
        joint_constraint_tuning_impl(self.id)
    }

    pub fn try_constraint_tuning(&self) -> ApiResult<ConstraintTuning> {
        self.check_valid()?;
        Ok(joint_constraint_tuning_impl(self.id))
    }

    pub fn set_constraint_tuning(&mut self, tuning: ConstraintTuning) {
        self.assert_valid();
        joint_set_constraint_tuning_impl(self.id, tuning)
    }

    pub fn try_set_constraint_tuning(&mut self, tuning: ConstraintTuning) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_constraint_tuning_impl(self.id, tuning);
        Ok(())
    }

    pub fn local_frame_a(&self) -> crate::Transform {
        self.assert_valid();
        joint_local_frame_a_impl(self.id)
    }

    pub fn try_local_frame_a(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(joint_local_frame_a_impl(self.id))
    }

    pub fn local_frame_b(&self) -> crate::Transform {
        self.assert_valid();
        joint_local_frame_b_impl(self.id)
    }

    pub fn try_local_frame_b(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(joint_local_frame_b_impl(self.id))
    }

    pub fn wake_bodies(&mut self) {
        self.assert_valid();
        joint_wake_bodies_impl(self.id)
    }

    pub fn try_wake_bodies(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        joint_wake_bodies_impl(self.id);
        Ok(())
    }

    pub fn linear_separation(&self) -> f32 {
        self.assert_valid();
        joint_linear_separation_impl(self.id)
    }

    pub fn try_linear_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_linear_separation_impl(self.id))
    }

    pub fn angular_separation(&self) -> f32 {
        self.assert_valid();
        joint_angular_separation_impl(self.id)
    }

    pub fn try_angular_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_angular_separation_impl(self.id))
    }

    pub fn constraint_force(&self) -> Vec2 {
        self.assert_valid();
        joint_constraint_force_impl(self.id)
    }

    pub fn try_constraint_force(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(joint_constraint_force_impl(self.id))
    }

    pub fn constraint_torque(&self) -> f32 {
        self.assert_valid();
        joint_constraint_torque_impl(self.id)
    }

    pub fn try_constraint_torque(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_constraint_torque_impl(self.id))
    }

    pub fn force_threshold(&self) -> f32 {
        self.assert_valid();
        joint_force_threshold_impl(self.id)
    }
    pub fn set_force_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        joint_set_force_threshold_impl(self.id, threshold)
    }

    pub fn try_force_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_force_threshold_impl(self.id))
    }
    pub fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_force_threshold_impl(self.id, threshold);
        Ok(())
    }

    pub fn torque_threshold(&self) -> f32 {
        self.assert_valid();
        joint_torque_threshold_impl(self.id)
    }
    pub fn set_torque_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        joint_set_torque_threshold_impl(self.id, threshold)
    }

    pub fn try_torque_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(joint_torque_threshold_impl(self.id))
    }
    pub fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        joint_set_torque_threshold_impl(self.id, threshold);
        Ok(())
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as Box2D may read it.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { joint_set_user_data_ptr_raw_checked_impl(self.core.as_ref(), self.id, p) }
    }
    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { try_joint_set_user_data_ptr_raw_impl(self.core.as_ref(), self.id, p) }
    }
    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        joint_user_data_ptr_raw_checked_impl(self.id)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_joint_user_data_ptr_raw_impl(self.id)
    }

    /// Set typed user data on this joint.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the joint is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        joint_set_user_data_checked_impl(self.core.as_ref(), self.id, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_joint_set_user_data_checked_impl(self.core.as_ref(), self.id, value)
    }

    /// Clear typed user data on this joint. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        joint_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_joint_clear_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        joint_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        try_joint_with_user_data_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        joint_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_joint_with_user_data_mut_checked_impl(self.core.as_ref(), self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        joint_take_user_data_checked_impl(self.core.as_ref(), self.id)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_joint_take_user_data_checked_impl(self.core.as_ref(), self.id)
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

/// Base joint definition builder for common properties.
///
/// This configures `b2JointDef` fields shared by all joint types. Typically
/// you construct a specific joint def (e.g. `RevoluteJointDef`) with this as
/// its `base`.
#[derive(Clone, Debug)]
pub struct JointBase(pub(crate) ffi::b2JointDef);

impl Default for JointBase {
    fn default() -> Self {
        // No default constructor provided for b2JointDef; zero is OK for POD and we'll set fields explicitly.
        // Use identity frames by default.
        let mut base: ffi::b2JointDef = unsafe { core::mem::zeroed() };
        base.drawScale = 1.0;
        base.localFrameA = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        base.localFrameB = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        Self(base)
    }
}

impl JointBase {
    /// Start building a new `JointBase` from defaults.
    pub fn builder() -> JointBaseBuilder {
        JointBaseBuilder::new()
    }

    /// Construct from the raw Box2D joint base definition value.
    #[inline]
    pub fn from_raw(raw: ffi::b2JointDef) -> Self {
        Self(raw)
    }

    /// Attached body A id.
    #[inline]
    pub fn body_a_id(&self) -> BodyId {
        BodyId::from_raw(self.0.bodyIdA)
    }

    /// Attached body B id.
    #[inline]
    pub fn body_b_id(&self) -> BodyId {
        BodyId::from_raw(self.0.bodyIdB)
    }

    /// Local frame on body A.
    #[inline]
    pub fn local_frame_a(&self) -> crate::Transform {
        crate::Transform::from_raw(self.0.localFrameA)
    }

    /// Local frame on body B.
    #[inline]
    pub fn local_frame_b(&self) -> crate::Transform {
        crate::Transform::from_raw(self.0.localFrameB)
    }

    /// Whether the connected bodies should collide with each other.
    #[inline]
    pub fn collide_connected(&self) -> bool {
        self.0.collideConnected
    }

    /// Force threshold used for joint events.
    #[inline]
    pub fn force_threshold(&self) -> f32 {
        self.0.forceThreshold
    }

    /// Torque threshold used for joint events.
    #[inline]
    pub fn torque_threshold(&self) -> f32 {
        self.0.torqueThreshold
    }

    /// Shared constraint tuning on the base definition.
    #[inline]
    pub fn constraint_tuning(&self) -> ConstraintTuning {
        ConstraintTuning::new(self.0.constraintHertz, self.0.constraintDampingRatio)
    }

    /// Debug draw scale.
    #[inline]
    pub fn draw_scale(&self) -> f32 {
        self.0.drawScale
    }

    /// Convert into the raw Box2D joint base definition value.
    #[inline]
    pub fn into_raw(self) -> ffi::b2JointDef {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct JointBaseBuilder {
    base: JointBase,
}

impl JointBaseBuilder {
    /// Create a new base with identity local frames.
    pub fn new() -> Self {
        Self {
            base: JointBase::default(),
        }
    }
    /// Attach two bodies using scoped body handles.
    pub fn bodies<'w>(mut self, a: &Body<'w>, b: &Body<'w>) -> Self {
        self.base.0.bodyIdA = a.id.into_raw();
        self.base.0.bodyIdB = b.id.into_raw();
        self
    }
    /// Attach two bodies by raw ids.
    pub fn bodies_by_id(mut self, a: BodyId, b: BodyId) -> Self {
        self.base.0.bodyIdA = a.into_raw();
        self.base.0.bodyIdB = b.into_raw();
        self
    }
    /// Set local frames from positions and angles (radians).
    pub fn local_frames<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        pos_a: VA,
        angle_a: f32,
        pos_b: VB,
        angle_b: f32,
    ) -> Self {
        let (sa, ca) = angle_a.sin_cos();
        let (sb, cb) = angle_b.sin_cos();
        self.base.0.localFrameA = ffi::b2Transform {
            p: pos_a.into().into_raw(),
            q: ffi::b2Rot { c: ca, s: sa },
        };
        self.base.0.localFrameB = ffi::b2Transform {
            p: pos_b.into().into_raw(),
            q: ffi::b2Rot { c: cb, s: sb },
        };
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.base.0.collideConnected = flag;
        self
    }
    /// Force threshold for joint events.
    pub fn force_threshold(mut self, v: f32) -> Self {
        self.base.0.forceThreshold = v;
        self
    }
    /// Torque threshold for joint events.
    pub fn torque_threshold(mut self, v: f32) -> Self {
        self.base.0.torqueThreshold = v;
        self
    }
    /// Advanced constraint tuning frequency in Hertz.
    pub fn constraint_hertz(mut self, v: f32) -> Self {
        self.base.0.constraintHertz = v;
        self
    }
    /// Advanced constraint damping ratio.
    pub fn constraint_damping_ratio(mut self, v: f32) -> Self {
        self.base.0.constraintDampingRatio = v;
        self
    }
    pub fn draw_scale(mut self, v: f32) -> Self {
        self.base.0.drawScale = v;
        self
    }
    pub fn local_frames_raw(mut self, a: ffi::b2Transform, b: ffi::b2Transform) -> Self {
        self.base.0.localFrameA = a;
        self.base.0.localFrameB = b;
        self
    }
    /// Set local anchor positions from world points (rotation remains identity).
    pub fn local_points_from_world<'w, V: Into<crate::types::Vec2>>(
        mut self,
        body_a: &Body<'w>,
        world_a: V,
        body_b: &Body<'w>,
        world_b: V,
    ) -> Self {
        let ta = body_a.transform_raw();
        let tb = body_b.transform_raw();
        let wa: ffi::b2Vec2 = world_a.into().into_raw();
        let wb: ffi::b2Vec2 = world_b.into().into_raw();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ident = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        let mut fa = ident;
        let mut fb = ident;
        fa.p = la;
        fb.p = lb;
        self.base.0.localFrameA = fa;
        self.base.0.localFrameB = fb;
        self
    }
    pub fn build(self) -> JointBase {
        self.base
    }
    /// Set local frames using world anchors and a shared world axis (X-axis of joint frame).
    /// This computes localFrameA/B.rotation so that their X-axis aligns with the given world axis,
    /// and localFrameA/B.position to the given world anchor points.
    pub fn frames_from_world_with_axis<'w, VA, VB, AX>(
        mut self,
        body_a: &Body<'w>,
        anchor_a_world: VA,
        axis_world: AX,
        body_b: &Body<'w>,
        anchor_b_world: VB,
    ) -> Self
    where
        VA: Into<crate::types::Vec2>,
        VB: Into<crate::types::Vec2>,
        AX: Into<crate::types::Vec2>,
    {
        let ta = body_a.transform_raw();
        let tb = body_b.transform_raw();
        let wa: ffi::b2Vec2 = anchor_a_world.into().into_raw();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into_raw();
        let axis_w: ffi::b2Vec2 = axis_world.into().into_raw();
        // Local frames: positions from anchors, rotations from world axis
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis_w);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis_w);
        self.base.0.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.base.0.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self
    }
}

impl Default for JointBaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<JointBase> for JointBaseBuilder {
    fn from(base: JointBase) -> Self {
        Self { base }
    }
}
