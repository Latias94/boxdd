use std::marker::PhantomData;
use std::os::raw::c_void;
use std::rc::Rc;
use std::sync::Arc;

use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use crate::query::Aabb;
use crate::types::{BodyId, ContactData, JointId, MassData, ShapeId, Vec2};
use boxdd_sys::ffi;

use super::definition::BodyType;
use super::runtime::{BodyRuntimeHandle, raw_body_id};

/// A RAII-owned body that is destroyed on drop.
///
/// This handle is not `Send` so it cannot be dropped on another thread. It keeps the underlying
/// world alive via an internal reference-counted core.
pub struct OwnedBody {
    pub(crate) id: BodyId,
    pub(crate) core: Arc<WorldCore>,
    destroy_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

impl OwnedBody {
    pub(crate) fn new(core: Arc<WorldCore>, id: BodyId) -> Self {
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

    pub(crate) fn core_arc(&self) -> Arc<WorldCore> {
        Arc::clone(&self.core)
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        BodyRuntimeHandle::world_id_raw(self)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        BodyRuntimeHandle::try_world_id_raw(self)
    }

    pub fn is_valid(&self) -> bool {
        BodyRuntimeHandle::is_valid(self)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        BodyRuntimeHandle::try_is_valid(self)
    }

    pub fn position(&self) -> Vec2 {
        BodyRuntimeHandle::position(self)
    }

    pub fn try_position(&self) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_position(self)
    }

    pub fn linear_velocity(&self) -> Vec2 {
        BodyRuntimeHandle::linear_velocity(self)
    }

    pub fn try_linear_velocity(&self) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_linear_velocity(self)
    }

    pub fn angular_velocity(&self) -> f32 {
        BodyRuntimeHandle::angular_velocity(self)
    }

    pub fn try_angular_velocity(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_angular_velocity(self)
    }

    pub fn rotation(&self) -> crate::Rot {
        BodyRuntimeHandle::rotation(self)
    }

    pub fn try_rotation(&self) -> ApiResult<crate::Rot> {
        BodyRuntimeHandle::try_rotation(self)
    }

    pub fn rotation_raw(&self) -> ffi::b2Rot {
        BodyRuntimeHandle::rotation_raw(self)
    }

    pub fn try_rotation_raw(&self) -> ApiResult<ffi::b2Rot> {
        BodyRuntimeHandle::try_rotation_raw(self)
    }

    pub fn transform(&self) -> crate::Transform {
        BodyRuntimeHandle::transform(self)
    }

    pub fn try_transform(&self) -> ApiResult<crate::Transform> {
        BodyRuntimeHandle::try_transform(self)
    }

    pub fn transform_raw(&self) -> ffi::b2Transform {
        BodyRuntimeHandle::transform_raw(self)
    }

    pub fn try_transform_raw(&self) -> ApiResult<ffi::b2Transform> {
        BodyRuntimeHandle::try_transform_raw(self)
    }

    pub fn aabb(&self) -> Aabb {
        BodyRuntimeHandle::aabb(self)
    }

    pub fn try_aabb(&self) -> ApiResult<Aabb> {
        BodyRuntimeHandle::try_aabb(self)
    }

    pub fn local_point<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        BodyRuntimeHandle::local_point(self, world_point)
    }

    pub fn try_local_point<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_local_point(self, world_point)
    }

    pub fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        BodyRuntimeHandle::world_point(self, local_point)
    }

    pub fn try_world_point<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_world_point(self, local_point)
    }

    pub fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Vec2 {
        BodyRuntimeHandle::local_vector(self, world_vector)
    }

    pub fn try_local_vector<V: Into<Vec2>>(&self, world_vector: V) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_local_vector(self, world_vector)
    }

    pub fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Vec2 {
        BodyRuntimeHandle::world_vector(self, local_vector)
    }

    pub fn try_world_vector<V: Into<Vec2>>(&self, local_vector: V) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_world_vector(self, local_vector)
    }

    pub fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        BodyRuntimeHandle::local_point_velocity(self, local_point)
    }

    pub fn try_local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_local_point_velocity(self, local_point)
    }

    pub fn world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        BodyRuntimeHandle::world_point_velocity(self, world_point)
    }

    pub fn try_world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_world_point_velocity(self, world_point)
    }

    pub fn set_position_and_rotation<V: Into<Vec2>>(&mut self, p: V, angle_radians: f32) {
        BodyRuntimeHandle::set_position_and_rotation(self, p, angle_radians);
    }

    pub fn try_set_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        p: V,
        angle_radians: f32,
    ) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_position_and_rotation(self, p, angle_radians)
    }

    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        BodyRuntimeHandle::set_linear_velocity(self, v)
    }

    pub fn try_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_linear_velocity(self, v)
    }

    pub fn set_angular_velocity(&mut self, w: f32) {
        BodyRuntimeHandle::set_angular_velocity(self, w)
    }

    pub fn try_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_angular_velocity(self, w)
    }

    pub fn set_target_transform(&mut self, target: crate::Transform, time_step: f32, wake: bool) {
        BodyRuntimeHandle::set_target_transform(self, target, time_step, wake);
    }

    pub fn try_set_target_transform(
        &mut self,
        target: crate::Transform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_target_transform(self, target, time_step, wake)
    }

    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        BodyRuntimeHandle::apply_force_to_center(self, force, wake);
    }

    pub fn try_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        force: V,
        wake: bool,
    ) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_force_to_center(self, force, wake)
    }

    pub fn apply_force<F: Into<Vec2>, P: Into<Vec2>>(&mut self, force: F, point: P, wake: bool) {
        BodyRuntimeHandle::apply_force(self, force, point, wake);
    }

    pub fn try_apply_force<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_force(self, force, point, wake)
    }
    pub fn apply_torque(&mut self, torque: f32, wake: bool) {
        BodyRuntimeHandle::apply_torque(self, torque, wake)
    }

    pub fn try_apply_torque(&mut self, torque: f32, wake: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_torque(self, torque, wake)
    }

    pub fn clear_forces(&mut self) {
        BodyRuntimeHandle::clear_forces(self);
    }

    pub fn try_clear_forces(&mut self) -> ApiResult<()> {
        BodyRuntimeHandle::try_clear_forces(self)
    }
    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        BodyRuntimeHandle::apply_linear_impulse_to_center(self, impulse, wake);
    }

    pub fn try_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_linear_impulse_to_center(self, impulse, wake)
    }

    pub fn apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        BodyRuntimeHandle::apply_linear_impulse(self, impulse, point, wake);
    }
    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        BodyRuntimeHandle::apply_angular_impulse(self, impulse, wake)
    }

    pub fn try_apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_linear_impulse(self, impulse, point, wake)
    }

    pub fn try_apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_angular_impulse(self, impulse, wake)
    }

    pub fn mass(&self) -> f32 {
        BodyRuntimeHandle::mass(self)
    }

    pub fn try_mass(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_mass(self)
    }

    pub fn rotational_inertia(&self) -> f32 {
        BodyRuntimeHandle::rotational_inertia(self)
    }

    pub fn try_rotational_inertia(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_rotational_inertia(self)
    }

    pub fn local_center_of_mass(&self) -> Vec2 {
        BodyRuntimeHandle::local_center_of_mass(self)
    }

    pub fn try_local_center_of_mass(&self) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_local_center_of_mass(self)
    }

    pub fn world_center_of_mass(&self) -> Vec2 {
        BodyRuntimeHandle::world_center_of_mass(self)
    }

    pub fn try_world_center_of_mass(&self) -> ApiResult<Vec2> {
        BodyRuntimeHandle::try_world_center_of_mass(self)
    }

    pub fn mass_data(&self) -> MassData {
        BodyRuntimeHandle::mass_data(self)
    }

    pub fn try_mass_data(&self) -> ApiResult<MassData> {
        BodyRuntimeHandle::try_mass_data(self)
    }

    pub fn set_mass_data(&mut self, mass_data: MassData) {
        BodyRuntimeHandle::set_mass_data(self, mass_data);
    }

    pub fn try_set_mass_data(&mut self, mass_data: MassData) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_mass_data(self, mass_data)
    }

    pub fn apply_mass_from_shapes(&mut self) {
        BodyRuntimeHandle::apply_mass_from_shapes(self);
    }

    pub fn try_apply_mass_from_shapes(&mut self) -> ApiResult<()> {
        BodyRuntimeHandle::try_apply_mass_from_shapes(self)
    }

    pub fn shape_count(&self) -> i32 {
        BodyRuntimeHandle::shape_count(self)
    }

    pub fn try_shape_count(&self) -> ApiResult<i32> {
        BodyRuntimeHandle::try_shape_count(self)
    }

    pub fn shapes(&self) -> Vec<ShapeId> {
        BodyRuntimeHandle::shapes(self)
    }

    pub fn shapes_into(&self, out: &mut Vec<ShapeId>) {
        BodyRuntimeHandle::shapes_into(self, out);
    }

    pub fn try_shapes(&self) -> ApiResult<Vec<ShapeId>> {
        BodyRuntimeHandle::try_shapes(self)
    }

    pub fn try_shapes_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        BodyRuntimeHandle::try_shapes_into(self, out)
    }

    pub fn joint_count(&self) -> i32 {
        BodyRuntimeHandle::joint_count(self)
    }

    pub fn try_joint_count(&self) -> ApiResult<i32> {
        BodyRuntimeHandle::try_joint_count(self)
    }

    pub fn joints(&self) -> Vec<JointId> {
        BodyRuntimeHandle::joints(self)
    }

    pub fn joints_into(&self, out: &mut Vec<JointId>) {
        BodyRuntimeHandle::joints_into(self, out);
    }

    pub fn try_joints(&self) -> ApiResult<Vec<JointId>> {
        BodyRuntimeHandle::try_joints(self)
    }

    pub fn try_joints_into(&self, out: &mut Vec<JointId>) -> ApiResult<()> {
        BodyRuntimeHandle::try_joints_into(self, out)
    }

    pub fn body_type(&self) -> BodyType {
        BodyRuntimeHandle::body_type(self)
    }

    pub fn try_body_type(&self) -> ApiResult<BodyType> {
        BodyRuntimeHandle::try_body_type(self)
    }
    pub fn set_body_type(&mut self, t: BodyType) {
        BodyRuntimeHandle::set_body_type(self, t)
    }

    pub fn try_set_body_type(&mut self, t: BodyType) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_body_type(self, t)
    }

    pub fn gravity_scale(&self) -> f32 {
        BodyRuntimeHandle::gravity_scale(self)
    }
    pub fn try_gravity_scale(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_gravity_scale(self)
    }
    pub fn set_gravity_scale(&mut self, v: f32) {
        BodyRuntimeHandle::set_gravity_scale(self, v)
    }

    pub fn try_set_gravity_scale(&mut self, v: f32) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_gravity_scale(self, v)
    }

    pub fn linear_damping(&self) -> f32 {
        BodyRuntimeHandle::linear_damping(self)
    }
    pub fn try_linear_damping(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_linear_damping(self)
    }
    pub fn set_linear_damping(&mut self, v: f32) {
        BodyRuntimeHandle::set_linear_damping(self, v)
    }
    pub fn try_set_linear_damping(&mut self, v: f32) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_linear_damping(self, v)
    }
    pub fn angular_damping(&self) -> f32 {
        BodyRuntimeHandle::angular_damping(self)
    }
    pub fn try_angular_damping(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_angular_damping(self)
    }
    pub fn set_angular_damping(&mut self, v: f32) {
        BodyRuntimeHandle::set_angular_damping(self, v)
    }
    pub fn try_set_angular_damping(&mut self, v: f32) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_angular_damping(self, v)
    }

    pub fn enable_sleep(&mut self, flag: bool) {
        BodyRuntimeHandle::enable_sleep(self, flag)
    }

    pub fn try_enable_sleep(&mut self, flag: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_enable_sleep(self, flag)
    }

    pub fn is_sleep_enabled(&self) -> bool {
        BodyRuntimeHandle::is_sleep_enabled(self)
    }

    pub fn try_is_sleep_enabled(&self) -> ApiResult<bool> {
        BodyRuntimeHandle::try_is_sleep_enabled(self)
    }

    pub fn set_sleep_threshold(&mut self, sleep_threshold: f32) {
        BodyRuntimeHandle::set_sleep_threshold(self, sleep_threshold)
    }

    pub fn try_set_sleep_threshold(&mut self, sleep_threshold: f32) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_sleep_threshold(self, sleep_threshold)
    }

    pub fn sleep_threshold(&self) -> f32 {
        BodyRuntimeHandle::sleep_threshold(self)
    }

    pub fn try_sleep_threshold(&self) -> ApiResult<f32> {
        BodyRuntimeHandle::try_sleep_threshold(self)
    }

    pub fn is_awake(&self) -> bool {
        BodyRuntimeHandle::is_awake(self)
    }
    pub fn try_is_awake(&self) -> ApiResult<bool> {
        BodyRuntimeHandle::try_is_awake(self)
    }
    pub fn set_awake(&mut self, awake: bool) {
        BodyRuntimeHandle::set_awake(self, awake)
    }
    pub fn try_set_awake(&mut self, awake: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_awake(self, awake)
    }

    pub fn is_enabled(&self) -> bool {
        BodyRuntimeHandle::is_enabled(self)
    }
    pub fn try_is_enabled(&self) -> ApiResult<bool> {
        BodyRuntimeHandle::try_is_enabled(self)
    }
    pub fn enable(&mut self) {
        BodyRuntimeHandle::enable(self)
    }
    pub fn try_enable(&mut self) -> ApiResult<()> {
        BodyRuntimeHandle::try_enable(self)
    }
    pub fn disable(&mut self) {
        BodyRuntimeHandle::disable(self)
    }
    pub fn try_disable(&mut self) -> ApiResult<()> {
        BodyRuntimeHandle::try_disable(self)
    }

    pub fn is_bullet(&self) -> bool {
        BodyRuntimeHandle::is_bullet(self)
    }
    pub fn try_is_bullet(&self) -> ApiResult<bool> {
        BodyRuntimeHandle::try_is_bullet(self)
    }
    pub fn set_bullet(&mut self, flag: bool) {
        BodyRuntimeHandle::set_bullet(self, flag)
    }

    pub fn try_set_bullet(&mut self, flag: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_bullet(self, flag)
    }

    pub fn enable_contact_events(&mut self, flag: bool) {
        BodyRuntimeHandle::enable_contact_events(self, flag)
    }

    pub fn try_enable_contact_events(&mut self, flag: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_enable_contact_events(self, flag)
    }

    pub fn enable_hit_events(&mut self, flag: bool) {
        BodyRuntimeHandle::enable_hit_events(self, flag)
    }

    pub fn try_enable_hit_events(&mut self, flag: bool) -> ApiResult<()> {
        BodyRuntimeHandle::try_enable_hit_events(self, flag)
    }

    pub fn set_name(&mut self, name: &str) {
        BodyRuntimeHandle::set_name(self, name)
    }

    pub fn try_set_name(&mut self, name: &str) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_name(self, name)
    }

    pub fn name(&self) -> Option<String> {
        BodyRuntimeHandle::name(self)
    }

    pub fn try_name(&self) -> ApiResult<Option<String>> {
        BodyRuntimeHandle::try_name(self)
    }

    pub fn contact_data(&self) -> Vec<ContactData> {
        BodyRuntimeHandle::contact_data(self)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        BodyRuntimeHandle::contact_data_into(self, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        BodyRuntimeHandle::try_contact_data(self)
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        BodyRuntimeHandle::try_contact_data_into(self, out)
    }

    pub fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        BodyRuntimeHandle::contact_data_raw(self)
    }

    pub fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        BodyRuntimeHandle::contact_data_raw_into(self, out);
    }

    pub fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        BodyRuntimeHandle::try_contact_data_raw(self)
    }

    pub fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        BodyRuntimeHandle::try_contact_data_raw_into(self, out)
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
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { BodyRuntimeHandle::set_user_data_ptr_raw(self, p) }
    }

    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { BodyRuntimeHandle::try_set_user_data_ptr_raw(self, p) }
    }
    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        BodyRuntimeHandle::user_data_ptr_raw(self)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        BodyRuntimeHandle::try_user_data_ptr_raw(self)
    }

    /// Set typed user data on this body.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the body is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        BodyRuntimeHandle::set_user_data(self, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        BodyRuntimeHandle::try_set_user_data(self, value)
    }

    /// Clear typed user data on this body. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        BodyRuntimeHandle::clear_user_data(self)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        BodyRuntimeHandle::try_clear_user_data(self)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        BodyRuntimeHandle::with_user_data(self, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        BodyRuntimeHandle::try_with_user_data(self, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        BodyRuntimeHandle::with_user_data_mut(self, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        BodyRuntimeHandle::try_with_user_data_mut(self, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        BodyRuntimeHandle::take_user_data(self)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        BodyRuntimeHandle::try_take_user_data(self)
    }

    /// Disarm RAII and return the raw id for manual lifetime management.
    pub fn into_id(mut self) -> BodyId {
        self.destroy_on_drop = false;
        self.id
    }

    /// Destroy the body immediately and disarm drop.
    pub fn destroy(mut self) {
        if self.destroy_on_drop && unsafe { ffi::b2Body_IsValid(raw_body_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Body(self.id));
            } else {
                #[cfg(feature = "serialize")]
                self.core.cleanup_before_destroy_body(self.id);
                unsafe { ffi::b2DestroyBody(raw_body_id(self.id)) };
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
        if self.destroy_on_drop && unsafe { ffi::b2Body_IsValid(raw_body_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Body(self.id));
            } else {
                #[cfg(feature = "serialize")]
                self.core.cleanup_before_destroy_body(self.id);
                unsafe { ffi::b2DestroyBody(raw_body_id(self.id)) };
                let _ = self.core.clear_body_user_data(self.id);
            }
        }
    }
}
