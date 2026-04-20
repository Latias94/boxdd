use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::core::world_core::WorldCore;
use crate::error::{ApiError, ApiResult};
use crate::types::{BodyId, MassData, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;

/// A RAII-owned body that is destroyed on drop.
///
/// This handle is not `Send` so it cannot be dropped on another thread. It keeps the underlying
/// world alive via an internal reference-counted core.
pub struct OwnedBody {
    id: BodyId,
    core: Arc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

fn body_contact_data_into_impl(id: BodyId, out: &mut Vec<ffi::b2ContactData>) {
    let cap = unsafe { ffi::b2Body_GetContactCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        });
    }
}

fn body_contact_data_impl(id: BodyId) -> Vec<ffi::b2ContactData> {
    let cap = unsafe { ffi::b2Body_GetContactCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Body_GetContactData(id, ptr, cap)
        })
    }
}

impl OwnedBody {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: BodyId) -> Self {
        core.owned_bodies
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            core,
            destroy_on_drop: true,
            _not_send: PhantomData,
        }
    }

    pub fn id(&self) -> BodyId {
        self.id
    }

    pub fn world_id(&self) -> ffi::b2WorldId {
        self.assert_valid();
        unsafe { ffi::b2Body_GetWorld(self.id) }
    }

    pub fn try_world_id(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetWorld(self.id) })
    }

    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Body_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Body_IsValid(self.id) })
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_body_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_body_valid(self.id)
    }

    pub fn position(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetPosition(self.id) })
    }

    pub fn try_position(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe { ffi::b2Body_GetPosition(self.id) }))
    }

    pub fn linear_velocity(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(self.id) })
    }

    pub fn try_linear_velocity(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLinearVelocity(self.id)
        }))
    }

    pub fn angular_velocity(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetAngularVelocity(self.id) }
    }

    pub fn try_angular_velocity(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetAngularVelocity(self.id) })
    }

    pub fn transform(&self) -> ffi::b2Transform {
        self.assert_valid();
        unsafe { ffi::b2Body_GetTransform(self.id) }
    }

    pub fn try_transform(&self) -> ApiResult<ffi::b2Transform> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetTransform(self.id) })
    }

    pub fn transform_ex(&self) -> crate::Transform {
        crate::Transform::from(self.transform())
    }

    pub fn local_point<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(self.id, p) })
    }

    pub fn try_local_point<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(self.id, p) }))
    }

    pub fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(self.id, p) })
    }

    pub fn try_world_point<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(self.id, p) }))
    }

    pub fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Vec2 {
        self.assert_valid();
        let v: ffi::b2Vec2 = world_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalVector(self.id, v) })
    }

    pub fn try_local_vector<V: Into<Vec2>>(&self, world_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let v: ffi::b2Vec2 = world_vector.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalVector(self.id, v)
        }))
    }

    pub fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Vec2 {
        self.assert_valid();
        let v: ffi::b2Vec2 = local_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldVector(self.id, v) })
    }

    pub fn try_world_vector<V: Into<Vec2>>(&self, local_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let v: ffi::b2Vec2 = local_vector.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldVector(self.id, v)
        }))
    }

    pub fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPointVelocity(self.id, p) })
    }

    pub fn try_local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalPointVelocity(self.id, p)
        }))
    }

    pub fn world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPointVelocity(self.id, p) })
    }

    pub fn try_world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldPointVelocity(self.id, p)
        }))
    }

    pub fn set_position_and_rotation<V: Into<Vec2>>(&mut self, p: V, angle_radians: f32) {
        self.assert_valid();
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(self.id, pos, rot) };
    }

    pub fn try_set_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        p: V,
        angle_radians: f32,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(self.id, pos, rot) };
        Ok(())
    }

    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        self.assert_valid();
        let vel: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(self.id, vel) }
    }

    pub fn try_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        self.check_valid()?;
        let vel: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(self.id, vel) }
        Ok(())
    }

    pub fn set_angular_velocity(&mut self, w: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetAngularVelocity(self.id, w) }
    }

    pub fn try_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetAngularVelocity(self.id, w) }
        Ok(())
    }

    pub fn set_target_transform(&mut self, target: crate::Transform, time_step: f32, wake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetTargetTransform(self.id, target.into(), time_step, wake) };
    }

    pub fn try_set_target_transform(
        &mut self,
        target: crate::Transform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetTargetTransform(self.id, target.into(), time_step, wake) };
        Ok(())
    }

    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        self.assert_valid();
        let f: ffi::b2Vec2 = force.into().into();
        unsafe { ffi::b2Body_ApplyForceToCenter(self.id, f, wake) };
    }

    pub fn try_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        force: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let f: ffi::b2Vec2 = force.into().into();
        unsafe { ffi::b2Body_ApplyForceToCenter(self.id, f, wake) };
        Ok(())
    }

    pub fn apply_force<F: Into<Vec2>, P: Into<Vec2>>(&mut self, force: F, point: P, wake: bool) {
        self.assert_valid();
        let f: ffi::b2Vec2 = force.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyForce(self.id, f, p, wake) };
    }

    pub fn try_apply_force<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let f: ffi::b2Vec2 = force.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyForce(self.id, f, p, wake) };
        Ok(())
    }
    pub fn apply_torque(&mut self, torque: f32, wake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_ApplyTorque(self.id, torque, wake) }
    }

    pub fn try_apply_torque(&mut self, torque: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ApplyTorque(self.id, torque, wake) }
        Ok(())
    }

    pub fn clear_forces(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_ClearForces(self.id) };
    }

    pub fn try_clear_forces(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ClearForces(self.id) };
        Ok(())
    }
    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        self.assert_valid();
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(self.id, i, wake) };
    }

    pub fn try_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(self.id, i, wake) };
        Ok(())
    }

    pub fn apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        self.assert_valid();
        let i: ffi::b2Vec2 = impulse.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulse(self.id, i, p, wake) };
    }
    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_ApplyAngularImpulse(self.id, impulse, wake) }
    }

    pub fn try_apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let i: ffi::b2Vec2 = impulse.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulse(self.id, i, p, wake) };
        Ok(())
    }

    pub fn try_apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ApplyAngularImpulse(self.id, impulse, wake) }
        Ok(())
    }

    pub fn mass(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetMass(self.id) }
    }

    pub fn try_mass(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetMass(self.id) })
    }

    pub fn rotational_inertia(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetRotationalInertia(self.id) }
    }

    pub fn try_rotational_inertia(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetRotationalInertia(self.id) })
    }

    pub fn local_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetLocalCenterOfMass(self.id) })
    }

    pub fn try_local_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalCenterOfMass(self.id)
        }))
    }

    pub fn world_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetWorldCenterOfMass(self.id) })
    }

    pub fn try_world_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldCenterOfMass(self.id)
        }))
    }

    pub fn mass_data(&self) -> MassData {
        self.assert_valid();
        unsafe { ffi::b2Body_GetMassData(self.id) }
    }

    pub fn try_mass_data(&self) -> ApiResult<MassData> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetMassData(self.id) })
    }

    pub fn set_mass_data(&mut self, mass_data: MassData) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetMassData(self.id, mass_data) };
    }

    pub fn try_set_mass_data(&mut self, mass_data: MassData) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetMassData(self.id, mass_data) };
        Ok(())
    }

    pub fn apply_mass_from_shapes(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_ApplyMassFromShapes(self.id) };
    }

    pub fn try_apply_mass_from_shapes(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ApplyMassFromShapes(self.id) };
        Ok(())
    }

    pub fn body_type(&self) -> BodyType {
        self.assert_valid();
        BodyType::from(unsafe { ffi::b2Body_GetType(self.id) })
    }

    pub fn try_body_type(&self) -> ApiResult<BodyType> {
        self.check_valid()?;
        Ok(BodyType::from(unsafe { ffi::b2Body_GetType(self.id) }))
    }
    pub fn set_body_type(&mut self, t: BodyType) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetType(self.id, t.into()) }
    }

    pub fn try_set_body_type(&mut self, t: BodyType) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetType(self.id, t.into()) }
        Ok(())
    }

    pub fn gravity_scale(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetGravityScale(self.id) }
    }
    pub fn try_gravity_scale(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetGravityScale(self.id) })
    }
    pub fn set_gravity_scale(&mut self, v: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetGravityScale(self.id, v) }
    }

    pub fn try_set_gravity_scale(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetGravityScale(self.id, v) }
        Ok(())
    }

    pub fn linear_damping(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetLinearDamping(self.id) }
    }
    pub fn try_linear_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetLinearDamping(self.id) })
    }
    pub fn set_linear_damping(&mut self, v: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetLinearDamping(self.id, v) }
    }
    pub fn try_set_linear_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetLinearDamping(self.id, v) }
        Ok(())
    }
    pub fn angular_damping(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetAngularDamping(self.id) }
    }
    pub fn try_angular_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetAngularDamping(self.id) })
    }
    pub fn set_angular_damping(&mut self, v: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetAngularDamping(self.id, v) }
    }
    pub fn try_set_angular_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetAngularDamping(self.id, v) }
        Ok(())
    }

    pub fn is_awake(&self) -> bool {
        self.assert_valid();
        unsafe { ffi::b2Body_IsAwake(self.id) }
    }
    pub fn try_is_awake(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_IsAwake(self.id) })
    }
    pub fn set_awake(&mut self, awake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetAwake(self.id, awake) }
    }
    pub fn try_set_awake(&mut self, awake: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetAwake(self.id, awake) }
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.assert_valid();
        unsafe { ffi::b2Body_IsEnabled(self.id) }
    }
    pub fn try_is_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_IsEnabled(self.id) })
    }
    pub fn enable(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_Enable(self.id) }
    }
    pub fn try_enable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_Enable(self.id) }
        Ok(())
    }
    pub fn disable(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_Disable(self.id) }
    }
    pub fn try_disable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_Disable(self.id) }
        Ok(())
    }

    pub fn is_bullet(&self) -> bool {
        self.assert_valid();
        unsafe { ffi::b2Body_IsBullet(self.id) }
    }
    pub fn try_is_bullet(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_IsBullet(self.id) })
    }
    pub fn set_bullet(&mut self, flag: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetBullet(self.id, flag) }
    }

    pub fn try_set_bullet(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetBullet(self.id, flag) }
        Ok(())
    }

    pub fn set_name(&mut self, name: &str) {
        self.assert_valid();
        let cs = CString::new(name).expect("body name contains an interior NUL byte");
        unsafe { ffi::b2Body_SetName(self.id, cs.as_ptr()) }
    }

    pub fn try_set_name(&mut self, name: &str) -> ApiResult<()> {
        self.check_valid()?;
        let cs = CString::new(name).map_err(|_| ApiError::NulByteInString)?;
        unsafe { ffi::b2Body_SetName(self.id, cs.as_ptr()) }
        Ok(())
    }

    pub fn name(&self) -> Option<String> {
        self.assert_valid();
        let ptr = unsafe { ffi::b2Body_GetName(self.id) };
        if ptr.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }

    pub fn try_name(&self) -> ApiResult<Option<String>> {
        self.check_valid()?;
        let ptr = unsafe { ffi::b2Body_GetName(self.id) };
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            ))
        }
    }

    pub fn contact_data(&self) -> Vec<ffi::b2ContactData> {
        self.assert_valid();
        body_contact_data_impl(self.id)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        self.assert_valid();
        body_contact_data_into_impl(self.id, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        self.check_valid()?;
        Ok(body_contact_data_impl(self.id))
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        body_contact_data_into_impl(self.id, out);
        Ok(())
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> BodyId {
        self.id
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// The caller must ensure that `p` is either null or points to a valid object
    /// for the entire time the body may access it, and that any lifetimes/aliasing rules
    /// are upheld. Box2D treats this as an opaque pointer and may store/use it across steps.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut c_void) {
        self.assert_valid();
        let _ = self.core.clear_body_user_data(self.id);
        unsafe { ffi::b2Body_SetUserData(self.id, p) }
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr(&mut self, p: *mut c_void) -> ApiResult<()> {
        self.check_valid()?;
        let _ = self.core.clear_body_user_data(self.id);
        unsafe { ffi::b2Body_SetUserData(self.id, p) }
        Ok(())
    }
    pub fn user_data_ptr(&self) -> *mut c_void {
        self.assert_valid();
        unsafe { ffi::b2Body_GetUserData(self.id) }
    }

    pub fn try_user_data_ptr(&self) -> ApiResult<*mut c_void> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetUserData(self.id) })
    }

    /// Set typed user data on this body.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the body is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        self.assert_valid();
        let p = self.core.set_body_user_data(self.id, value);
        unsafe { ffi::b2Body_SetUserData(self.id, p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        self.check_valid()?;
        let p = self.core.set_body_user_data(self.id, value);
        unsafe { ffi::b2Body_SetUserData(self.id, p) };
        Ok(())
    }

    /// Clear typed user data on this body. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        self.assert_valid();
        let had = self.core.clear_body_user_data(self.id);
        if had {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        had
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        self.check_valid()?;
        let had = self.core.clear_body_user_data(self.id);
        if had {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_body_user_data(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_body_user_data(self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_body_user_data_mut(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_body_user_data_mut(self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        self.assert_valid();
        let v = self
            .core
            .take_body_user_data::<T>(self.id)
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        self.check_valid()?;
        let v = self.core.take_body_user_data::<T>(self.id)?;
        if v.is_some() {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(v)
    }

    /// Disarm RAII and return the raw id for manual lifetime management.
    pub fn into_id(mut self) -> BodyId {
        self.destroy_on_drop = false;
        self.id
    }

    /// Destroy the body immediately and disarm drop.
    pub fn destroy(mut self) {
        if self.destroy_on_drop && unsafe { ffi::b2Body_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Body(self.id));
            } else {
                #[cfg(feature = "serialize")]
                self.core.cleanup_before_destroy_body(self.id);
                unsafe { ffi::b2DestroyBody(self.id) };
                let _ = self.core.clear_body_user_data(self.id);
            }
        }
        self.destroy_on_drop = false;
    }
}

impl Drop for OwnedBody {
    fn drop(&mut self) {
        let _ = self.core.id;
        let prev = self
            .core
            .owned_bodies
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug_assert!(prev > 0, "owned body counter underflow");
        if self.destroy_on_drop && unsafe { ffi::b2Body_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Body(self.id));
            } else {
                #[cfg(feature = "serialize")]
                self.core.cleanup_before_destroy_body(self.id);
                unsafe { ffi::b2DestroyBody(self.id) };
                let _ = self.core.clear_body_user_data(self.id);
            }
        }
    }
}

/// Body types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

impl From<BodyType> for ffi::b2BodyType {
    fn from(t: BodyType) -> Self {
        match t {
            BodyType::Static => ffi::b2BodyType_b2_staticBody,
            BodyType::Kinematic => ffi::b2BodyType_b2_kinematicBody,
            BodyType::Dynamic => ffi::b2BodyType_b2_dynamicBody,
        }
    }
}

impl From<ffi::b2BodyType> for BodyType {
    fn from(t: ffi::b2BodyType) -> Self {
        match t {
            x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
            x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
            _ => BodyType::Dynamic,
        }
    }
}

/// Body definition wrapper with builder API.
#[derive(Clone, Debug)]
pub struct BodyDef(pub(crate) ffi::b2BodyDef);

impl Default for BodyDef {
    fn default() -> Self {
        let def = unsafe { ffi::b2DefaultBodyDef() };
        Self(def)
    }
}

/// Fluent builder for `BodyDef`.
#[doc(alias = "body_builder")]
#[doc(alias = "bodybuilder")]
///
/// Chain methods to configure a body and finish with `build()`. This maps
/// to the upstream `b2BodyDef` fields.
#[derive(Clone, Debug)]
pub struct BodyBuilder {
    def: BodyDef,
}

impl BodyBuilder {
    /// Start a new builder with default `BodyDef`.
    pub fn new() -> Self {
        Self {
            def: BodyDef::default(),
        }
    }
    /// Set the body type (static, kinematic, dynamic).
    pub fn body_type(mut self, t: BodyType) -> Self {
        self.def.0.type_ = t.into();
        self
    }
    /// Initial world-space position.
    pub fn position<V: Into<Vec2>>(mut self, p: V) -> Self {
        self.def.0.position = ffi::b2Vec2::from(p.into());
        self
    }
    /// Initial rotation in radians.
    pub fn angle(mut self, radians: f32) -> Self {
        // Build a rotation from angle
        let (s, c) = radians.sin_cos();
        self.def.0.rotation = ffi::b2Rot { c, s };
        self
    }
    /// Initial linear velocity (m/s).
    pub fn linear_velocity<V: Into<Vec2>>(mut self, v: V) -> Self {
        self.def.0.linearVelocity = ffi::b2Vec2::from(v.into());
        self
    }
    /// Initial angular velocity (rad/s).
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.def.0.angularVelocity = v;
        self
    }
    /// Linear damping (drag-like term).
    pub fn linear_damping(mut self, v: f32) -> Self {
        self.def.0.linearDamping = v;
        self
    }
    /// Angular damping.
    pub fn angular_damping(mut self, v: f32) -> Self {
        self.def.0.angularDamping = v;
        self
    }
    /// Per-body gravity scale (1 = normal gravity).
    pub fn gravity_scale(mut self, v: f32) -> Self {
        self.def.0.gravityScale = v;
        self
    }
    /// Allow body to go to sleep.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.0.enableSleep = flag;
        self
    }
    /// Awake/asleep flag at creation.
    pub fn awake(mut self, flag: bool) -> Self {
        self.def.0.isAwake = flag;
        self
    }
    /// Treat as bullet (CCD).
    pub fn bullet(mut self, flag: bool) -> Self {
        self.def.0.isBullet = flag;
        self
    }
    /// Enable/disable simulation for this body.
    pub fn enabled(mut self, flag: bool) -> Self {
        self.def.0.isEnabled = flag;
        self
    }

    #[must_use]
    pub fn build(self) -> BodyDef {
        self.def
    }
}

impl From<BodyDef> for BodyBuilder {
    fn from(def: BodyDef) -> Self {
        Self { def }
    }
}

// serde support for BodyDef via a transparent config struct
#[cfg(feature = "serde")]
impl serde::Serialize for BodyDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            body_type: super::body::BodyType,
            position: crate::types::Vec2,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            enabled: bool,
        }
        let angle = self.0.rotation.s.atan2(self.0.rotation.c);
        let r = Repr {
            body_type: match self.0.type_ {
                x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
                x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
                _ => BodyType::Dynamic,
            },
            position: crate::types::Vec2::from(self.0.position),
            angle,
            linear_velocity: crate::types::Vec2::from(self.0.linearVelocity),
            angular_velocity: self.0.angularVelocity,
            linear_damping: self.0.linearDamping,
            angular_damping: self.0.angularDamping,
            gravity_scale: self.0.gravityScale,
            enable_sleep: self.0.enableSleep,
            awake: self.0.isAwake,
            bullet: self.0.isBullet,
            enabled: self.0.isEnabled,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BodyDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            body_type: super::body::BodyType,
            position: crate::types::Vec2,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            enabled: bool,
        }
        let r = Repr::deserialize(deserializer)?;
        let b = BodyBuilder::new()
            .body_type(r.body_type)
            .position(r.position)
            .angle(r.angle)
            .linear_velocity(r.linear_velocity)
            .angular_velocity(r.angular_velocity)
            .linear_damping(r.linear_damping)
            .angular_damping(r.angular_damping)
            .gravity_scale(r.gravity_scale)
            .enable_sleep(r.enable_sleep)
            .awake(r.awake)
            .bullet(r.bullet)
            .enabled(r.enabled);
        Ok(b.build())
    }
}

impl Default for BodyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A body handle with lifetime tied to the owning world.
pub struct Body<'w> {
    pub(crate) id: BodyId,
    pub(crate) core: Arc<WorldCore>,
    _world: PhantomData<&'w World>,
}

impl<'w> Body<'w> {
    pub(crate) fn new(core: Arc<WorldCore>, id: BodyId) -> Self {
        Self {
            id,
            core,
            _world: PhantomData,
        }
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_body_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_body_valid(self.id)
    }

    pub fn id(&self) -> BodyId {
        self.id
    }

    pub fn world_id(&self) -> ffi::b2WorldId {
        self.assert_valid();
        unsafe { ffi::b2Body_GetWorld(self.id) }
    }

    pub fn try_world_id(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetWorld(self.id) })
    }

    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Body_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Body_IsValid(self.id) })
    }

    // Queries
    pub fn position(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetPosition(self.id) })
    }

    pub fn try_position(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe { ffi::b2Body_GetPosition(self.id) }))
    }

    pub fn linear_velocity(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(self.id) })
    }

    pub fn try_linear_velocity(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLinearVelocity(self.id)
        }))
    }

    pub fn angular_velocity(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetAngularVelocity(self.id) }
    }

    pub fn try_angular_velocity(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetAngularVelocity(self.id) })
    }

    pub fn transform(&self) -> ffi::b2Transform {
        self.assert_valid();
        unsafe { ffi::b2Body_GetTransform(self.id) }
    }

    pub fn try_transform(&self) -> ApiResult<ffi::b2Transform> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetTransform(self.id) })
    }
    pub fn transform_ex(&self) -> crate::Transform {
        crate::Transform::from(self.transform())
    }

    pub fn local_point<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(self.id, p) })
    }

    pub fn try_local_point<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(self.id, p) }))
    }

    pub fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(self.id, p) })
    }

    pub fn try_world_point<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(self.id, p) }))
    }

    pub fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Vec2 {
        self.assert_valid();
        let v: ffi::b2Vec2 = world_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalVector(self.id, v) })
    }

    pub fn try_local_vector<V: Into<Vec2>>(&self, world_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let v: ffi::b2Vec2 = world_vector.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalVector(self.id, v)
        }))
    }

    pub fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Vec2 {
        self.assert_valid();
        let v: ffi::b2Vec2 = local_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldVector(self.id, v) })
    }

    pub fn try_world_vector<V: Into<Vec2>>(&self, local_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let v: ffi::b2Vec2 = local_vector.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldVector(self.id, v)
        }))
    }

    pub fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPointVelocity(self.id, p) })
    }

    pub fn try_local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalPointVelocity(self.id, p)
        }))
    }

    pub fn world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPointVelocity(self.id, p) })
    }

    pub fn try_world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldPointVelocity(self.id, p)
        }))
    }

    // Mutations
    pub fn set_position_and_rotation<V: Into<Vec2>>(&mut self, p: V, angle_radians: f32) {
        self.assert_valid();
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(self.id, pos, rot) };
    }
    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        self.assert_valid();
        let vel: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(self.id, vel) }
    }

    pub fn try_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        self.check_valid()?;
        let vel: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(self.id, vel) }
        Ok(())
    }

    pub fn set_angular_velocity(&mut self, w: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetAngularVelocity(self.id, w) }
    }

    pub fn try_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetAngularVelocity(self.id, w) }
        Ok(())
    }

    pub fn set_target_transform(&mut self, target: crate::Transform, time_step: f32, wake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetTargetTransform(self.id, target.into(), time_step, wake) };
    }

    pub fn try_set_target_transform(
        &mut self,
        target: crate::Transform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetTargetTransform(self.id, target.into(), time_step, wake) };
        Ok(())
    }

    pub fn contact_data(&self) -> Vec<ffi::b2ContactData> {
        self.assert_valid();
        body_contact_data_impl(self.id)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        self.assert_valid();
        body_contact_data_into_impl(self.id, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        self.check_valid()?;
        Ok(body_contact_data_impl(self.id))
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        body_contact_data_into_impl(self.id, out);
        Ok(())
    }

    // Forces/impulses
    pub fn apply_force<F: Into<Vec2>, P: Into<Vec2>>(&mut self, force: F, point: P, wake: bool) {
        self.assert_valid();
        let f: ffi::b2Vec2 = force.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyForce(self.id, f, p, wake) };
    }

    pub fn try_apply_force<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let f: ffi::b2Vec2 = force.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyForce(self.id, f, p, wake) };
        Ok(())
    }
    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        self.assert_valid();
        let f: ffi::b2Vec2 = force.into().into();
        unsafe { ffi::b2Body_ApplyForceToCenter(self.id, f, wake) };
    }

    pub fn try_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        force: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let f: ffi::b2Vec2 = force.into().into();
        unsafe { ffi::b2Body_ApplyForceToCenter(self.id, f, wake) };
        Ok(())
    }
    pub fn apply_torque(&mut self, torque: f32, wake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_ApplyTorque(self.id, torque, wake) }
    }

    pub fn try_apply_torque(&mut self, torque: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ApplyTorque(self.id, torque, wake) }
        Ok(())
    }

    pub fn clear_forces(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_ClearForces(self.id) };
    }

    pub fn try_clear_forces(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ClearForces(self.id) };
        Ok(())
    }

    pub fn apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        self.assert_valid();
        let i: ffi::b2Vec2 = impulse.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulse(self.id, i, p, wake) };
    }

    pub fn try_apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let i: ffi::b2Vec2 = impulse.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulse(self.id, i, p, wake) };
        Ok(())
    }
    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        self.assert_valid();
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(self.id, i, wake) };
    }

    pub fn try_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(self.id, i, wake) };
        Ok(())
    }
    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_ApplyAngularImpulse(self.id, impulse, wake) }
    }

    pub fn try_apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ApplyAngularImpulse(self.id, impulse, wake) }
        Ok(())
    }

    pub fn mass(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetMass(self.id) }
    }

    pub fn try_mass(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetMass(self.id) })
    }

    pub fn rotational_inertia(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetRotationalInertia(self.id) }
    }

    pub fn try_rotational_inertia(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetRotationalInertia(self.id) })
    }

    pub fn local_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetLocalCenterOfMass(self.id) })
    }

    pub fn try_local_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalCenterOfMass(self.id)
        }))
    }

    pub fn world_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        Vec2::from(unsafe { ffi::b2Body_GetWorldCenterOfMass(self.id) })
    }

    pub fn try_world_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldCenterOfMass(self.id)
        }))
    }

    pub fn mass_data(&self) -> MassData {
        self.assert_valid();
        unsafe { ffi::b2Body_GetMassData(self.id) }
    }

    pub fn try_mass_data(&self) -> ApiResult<MassData> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetMassData(self.id) })
    }

    pub fn set_mass_data(&mut self, mass_data: MassData) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetMassData(self.id, mass_data) };
    }

    pub fn try_set_mass_data(&mut self, mass_data: MassData) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetMassData(self.id, mass_data) };
        Ok(())
    }

    pub fn apply_mass_from_shapes(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_ApplyMassFromShapes(self.id) };
    }

    pub fn try_apply_mass_from_shapes(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_ApplyMassFromShapes(self.id) };
        Ok(())
    }

    pub fn body_type(&self) -> BodyType {
        self.assert_valid();
        BodyType::from(unsafe { ffi::b2Body_GetType(self.id) })
    }

    pub fn try_body_type(&self) -> ApiResult<BodyType> {
        self.check_valid()?;
        Ok(BodyType::from(unsafe { ffi::b2Body_GetType(self.id) }))
    }
    pub fn set_body_type(&mut self, t: BodyType) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetType(self.id, t.into()) }
    }

    pub fn try_set_body_type(&mut self, t: BodyType) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetType(self.id, t.into()) }
        Ok(())
    }

    pub fn gravity_scale(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetGravityScale(self.id) }
    }
    pub fn try_gravity_scale(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetGravityScale(self.id) })
    }
    pub fn set_gravity_scale(&mut self, v: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetGravityScale(self.id, v) }
    }

    pub fn try_set_gravity_scale(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetGravityScale(self.id, v) }
        Ok(())
    }

    pub fn linear_damping(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetLinearDamping(self.id) }
    }
    pub fn try_linear_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetLinearDamping(self.id) })
    }
    pub fn set_linear_damping(&mut self, v: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetLinearDamping(self.id, v) }
    }
    pub fn try_set_linear_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetLinearDamping(self.id, v) }
        Ok(())
    }
    pub fn angular_damping(&self) -> f32 {
        self.assert_valid();
        unsafe { ffi::b2Body_GetAngularDamping(self.id) }
    }
    pub fn try_angular_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetAngularDamping(self.id) })
    }
    pub fn set_angular_damping(&mut self, v: f32) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetAngularDamping(self.id, v) }
    }

    pub fn try_set_angular_damping(&mut self, v: f32) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetAngularDamping(self.id, v) }
        Ok(())
    }

    pub fn is_awake(&self) -> bool {
        self.assert_valid();
        unsafe { ffi::b2Body_IsAwake(self.id) }
    }
    pub fn set_awake(&mut self, awake: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetAwake(self.id, awake) }
    }

    pub fn try_is_awake(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_IsAwake(self.id) })
    }

    pub fn try_set_awake(&mut self, awake: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetAwake(self.id, awake) }
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.assert_valid();
        unsafe { ffi::b2Body_IsEnabled(self.id) }
    }
    pub fn enable(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_Enable(self.id) }
    }
    pub fn disable(&mut self) {
        self.assert_valid();
        unsafe { ffi::b2Body_Disable(self.id) }
    }

    pub fn try_is_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_IsEnabled(self.id) })
    }

    pub fn try_enable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_Enable(self.id) }
        Ok(())
    }

    pub fn try_disable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_Disable(self.id) }
        Ok(())
    }

    pub fn is_bullet(&self) -> bool {
        self.assert_valid();
        unsafe { ffi::b2Body_IsBullet(self.id) }
    }
    pub fn set_bullet(&mut self, flag: bool) {
        self.assert_valid();
        unsafe { ffi::b2Body_SetBullet(self.id, flag) }
    }

    pub fn try_is_bullet(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_IsBullet(self.id) })
    }

    pub fn try_set_bullet(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Body_SetBullet(self.id, flag) }
        Ok(())
    }

    // Names and user data (raw pointer)
    pub fn set_name(&mut self, name: &str) {
        self.assert_valid();
        let cs = CString::new(name).expect("body name contains an interior NUL byte");
        unsafe { ffi::b2Body_SetName(self.id, cs.as_ptr()) }
    }

    pub fn try_set_name(&mut self, name: &str) -> ApiResult<()> {
        self.check_valid()?;
        let cs = CString::new(name).map_err(|_| ApiError::NulByteInString)?;
        unsafe { ffi::b2Body_SetName(self.id, cs.as_ptr()) }
        Ok(())
    }

    pub fn name(&self) -> Option<String> {
        self.assert_valid();
        let ptr = unsafe { ffi::b2Body_GetName(self.id) };
        if ptr.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }

    pub fn try_name(&self) -> ApiResult<Option<String>> {
        self.check_valid()?;
        let ptr = unsafe { ffi::b2Body_GetName(self.id) };
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            ))
        }
    }
    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// The caller must ensure that `p` is either null or points to a valid object
    /// for the entire time the body may access it, and that any lifetimes/aliasing rules
    /// are upheld. Box2D treats this as an opaque pointer and may store/use it across steps.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut c_void) {
        self.assert_valid();
        let _ = self.core.clear_body_user_data(self.id);
        unsafe { ffi::b2Body_SetUserData(self.id, p) }
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr(&mut self, p: *mut c_void) -> ApiResult<()> {
        self.check_valid()?;
        let _ = self.core.clear_body_user_data(self.id);
        unsafe { ffi::b2Body_SetUserData(self.id, p) }
        Ok(())
    }
    pub fn user_data_ptr(&self) -> *mut c_void {
        self.assert_valid();
        unsafe { ffi::b2Body_GetUserData(self.id) }
    }

    pub fn try_user_data_ptr(&self) -> ApiResult<*mut c_void> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Body_GetUserData(self.id) })
    }

    /// Set typed user data on this body.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the body is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        self.assert_valid();
        let p = self.core.set_body_user_data(self.id, value);
        unsafe { ffi::b2Body_SetUserData(self.id, p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        self.check_valid()?;
        let p = self.core.set_body_user_data(self.id, value);
        unsafe { ffi::b2Body_SetUserData(self.id, p) };
        Ok(())
    }

    /// Clear typed user data on this body. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        self.assert_valid();
        let had = self.core.clear_body_user_data(self.id);
        if had {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        had
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        self.check_valid()?;
        let had = self.core.clear_body_user_data(self.id);
        if had {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_body_user_data(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_body_user_data(self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_body_user_data_mut(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_body_user_data_mut(self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        self.assert_valid();
        let v = self
            .core
            .take_body_user_data::<T>(self.id)
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        self.check_valid()?;
        let v = self.core.take_body_user_data::<T>(self.id)?;
        if v.is_some() {
            unsafe { ffi::b2Body_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(v)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> BodyId {
        self.id
    }
}
