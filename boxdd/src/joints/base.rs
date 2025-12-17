use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::body::Body;
use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::fmt;
use std::os::raw::c_void;

/// A scoped joint handle tied to a mutable borrow of the world.
pub struct Joint<'w> {
    pub(crate) id: ffi::b2JointId,
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
    core: Arc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
    wake_bodies_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

impl OwnedJoint {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: JointId) -> Self {
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
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Joint_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Joint_IsValid(self.id) })
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

    pub fn linear_separation(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetLinearSeparation(self.id) }
    }

    pub fn try_linear_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetLinearSeparation(self.id) })
    }

    pub fn angular_separation(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetAngularSeparation(self.id) }
    }

    pub fn try_angular_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetAngularSeparation(self.id) })
    }

    pub fn constraint_force(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Joint_GetConstraintForce(self.id) })
    }

    pub fn try_constraint_force(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Joint_GetConstraintForce(self.id)
        }))
    }

    pub fn constraint_torque(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetConstraintTorque(self.id) }
    }

    pub fn try_constraint_torque(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetConstraintTorque(self.id) })
    }

    pub fn force_threshold(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetForceThreshold(self.id) }
    }
    pub fn try_force_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetForceThreshold(self.id) })
    }
    pub fn set_force_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        unsafe { ffi::b2Joint_SetForceThreshold(self.id, threshold) }
    }
    pub fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Joint_SetForceThreshold(self.id, threshold) }
        Ok(())
    }
    pub fn torque_threshold(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetTorqueThreshold(self.id) }
    }
    pub fn try_torque_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetTorqueThreshold(self.id) })
    }
    pub fn set_torque_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        unsafe { ffi::b2Joint_SetTorqueThreshold(self.id, threshold) }
    }
    pub fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Joint_SetTorqueThreshold(self.id, threshold) }
        Ok(())
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as Box2D may read it.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut c_void) {
        self.assert_valid();
        let _ = self.core.clear_joint_user_data(self.id);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) }
    }
    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr`.
    pub unsafe fn try_set_user_data_ptr(&mut self, p: *mut c_void) -> ApiResult<()> {
        self.check_valid()?;
        let _ = self.core.clear_joint_user_data(self.id);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) }
        Ok(())
    }
    pub fn user_data_ptr(&self) -> *mut c_void {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetUserData(self.id) }
    }

    pub fn try_user_data_ptr(&self) -> ApiResult<*mut c_void> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetUserData(self.id) })
    }

    /// Set typed user data on this joint.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the joint is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        self.assert_valid();
        let p = self.core.set_joint_user_data(self.id, value);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        self.check_valid()?;
        let p = self.core.set_joint_user_data(self.id, value);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) };
        Ok(())
    }

    /// Clear typed user data on this joint. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        self.assert_valid();
        let had = self.core.clear_joint_user_data(self.id);
        if had {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        had
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        self.check_valid()?;
        let had = self.core.clear_joint_user_data(self.id);
        if had {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_joint_user_data(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_joint_user_data(self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_joint_user_data_mut(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_joint_user_data_mut(self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        self.assert_valid();
        let v = self
            .core
            .take_joint_user_data::<T>(self.id)
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        self.check_valid()?;
        let v = self.core.take_joint_user_data::<T>(self.id)?;
        if v.is_some() {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(v)
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
        if self.destroy_on_drop && unsafe { ffi::b2Joint_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Joint {
                        id: self.id,
                        wake_bodies,
                    });
            } else {
                unsafe { ffi::b2DestroyJoint(self.id, wake_bodies) };
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
        if self.destroy_on_drop && unsafe { ffi::b2Joint_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Joint {
                        id: self.id,
                        wake_bodies: self.wake_bodies_on_drop,
                    });
            } else {
                unsafe { ffi::b2DestroyJoint(self.id, self.wake_bodies_on_drop) };
                let _ = self.core.clear_joint_user_data(self.id);
            }
        }
    }
}

impl<'w> Joint<'w> {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: JointId) -> Self {
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
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Joint_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Joint_IsValid(self.id) })
    }
    pub fn linear_separation(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetLinearSeparation(self.id) }
    }

    pub fn try_linear_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetLinearSeparation(self.id) })
    }

    pub fn angular_separation(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetAngularSeparation(self.id) }
    }

    pub fn try_angular_separation(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetAngularSeparation(self.id) })
    }

    pub fn constraint_force(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Joint_GetConstraintForce(self.id) })
    }

    pub fn try_constraint_force(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Joint_GetConstraintForce(self.id)
        }))
    }

    pub fn constraint_torque(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetConstraintTorque(self.id) }
    }

    pub fn try_constraint_torque(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetConstraintTorque(self.id) })
    }

    pub fn force_threshold(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetForceThreshold(self.id) }
    }
    pub fn set_force_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        unsafe { ffi::b2Joint_SetForceThreshold(self.id, threshold) }
    }

    pub fn try_force_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetForceThreshold(self.id) })
    }
    pub fn try_set_force_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Joint_SetForceThreshold(self.id, threshold) }
        Ok(())
    }

    pub fn torque_threshold(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetTorqueThreshold(self.id) }
    }
    pub fn set_torque_threshold(&mut self, threshold: f32) {
        self.assert_valid();
        unsafe { ffi::b2Joint_SetTorqueThreshold(self.id, threshold) }
    }

    pub fn try_torque_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetTorqueThreshold(self.id) })
    }
    pub fn try_set_torque_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Joint_SetTorqueThreshold(self.id, threshold) }
        Ok(())
    }

    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as Box2D may read it.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut c_void) {
        self.assert_valid();
        let _ = self.core.clear_joint_user_data(self.id);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) }
    }
    /// Set an opaque user data pointer on this joint.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr`.
    pub unsafe fn try_set_user_data_ptr(&mut self, p: *mut c_void) -> ApiResult<()> {
        self.check_valid()?;
        let _ = self.core.clear_joint_user_data(self.id);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) }
        Ok(())
    }
    pub fn user_data_ptr(&self) -> *mut c_void {
        self.assert_valid();
        unsafe { ffi::b2Joint_GetUserData(self.id) }
    }

    pub fn try_user_data_ptr(&self) -> ApiResult<*mut c_void> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Joint_GetUserData(self.id) })
    }

    /// Set typed user data on this joint.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the joint is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        self.assert_valid();
        let p = self.core.set_joint_user_data(self.id, value);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        self.check_valid()?;
        let p = self.core.set_joint_user_data(self.id, value);
        unsafe { ffi::b2Joint_SetUserData(self.id, p) };
        Ok(())
    }

    /// Clear typed user data on this joint. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        self.assert_valid();
        let had = self.core.clear_joint_user_data(self.id);
        if had {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        had
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        self.check_valid()?;
        let had = self.core.clear_joint_user_data(self.id);
        if had {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_joint_user_data(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_joint_user_data(self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_joint_user_data_mut(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_joint_user_data_mut(self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        self.assert_valid();
        let v = self
            .core
            .take_joint_user_data::<T>(self.id)
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        self.check_valid()?;
        let v = self.core.take_joint_user_data::<T>(self.id)?;
        if v.is_some() {
            unsafe { ffi::b2Joint_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(v)
    }

    /// Destroy this joint immediately.
    pub fn destroy(self, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(self.id) } {
            unsafe { ffi::b2DestroyJoint(self.id, wake_bodies) };
            let _ = self.core.clear_joint_user_data(self.id);
        }
    }

    pub fn try_destroy(self, wake_bodies: bool) -> ApiResult<()> {
        self.check_valid()?;
        if unsafe { ffi::b2Joint_IsValid(self.id) } {
            unsafe { ffi::b2DestroyJoint(self.id, wake_bodies) };
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
        self.base.0.bodyIdA = a.id;
        self.base.0.bodyIdB = b.id;
        self
    }
    /// Attach two bodies by raw ids.
    pub fn bodies_by_id(mut self, a: BodyId, b: BodyId) -> Self {
        self.base.0.bodyIdA = a;
        self.base.0.bodyIdB = b;
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
            p: ffi::b2Vec2::from(pos_a.into()),
            q: ffi::b2Rot { c: ca, s: sa },
        };
        self.base.0.localFrameB = ffi::b2Transform {
            p: ffi::b2Vec2::from(pos_b.into()),
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
        let ta = body_a.transform();
        let tb = body_b.transform();
        let wa: ffi::b2Vec2 = world_a.into().into();
        let wb: ffi::b2Vec2 = world_b.into().into();
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
        let ta = body_a.transform();
        let tb = body_b.transform();
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis_w: ffi::b2Vec2 = axis_world.into().into();
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
