use super::attachments::*;
use super::user_data::*;
use super::*;
use crate::core::world_core::WorldCore;
use crate::error::{ApiError, ApiResult};
use crate::types::ContactData;
use std::ffi::CString;
use std::os::raw::c_void;

use super::super::definition::{
    assert_mass_data_valid, assert_non_negative_finite_body_scalar, check_mass_data_valid,
    check_non_negative_finite_body_scalar,
};
use super::super::owned::OwnedBody;
use super::super::scoped::Body;

#[inline]
fn body_world_id_checked_impl(id: BodyId) -> ffi::b2WorldId {
    crate::core::debug_checks::assert_body_valid(id);
    body_world_id_impl(id)
}

#[inline]
fn try_body_world_id_raw_impl(id: BodyId) -> ApiResult<ffi::b2WorldId> {
    crate::core::debug_checks::check_body_valid(id)?;
    Ok(body_world_id_impl(id))
}

#[inline]
fn body_is_valid_checked_impl(id: BodyId) -> bool {
    crate::core::callback_state::assert_not_in_callback();
    body_is_valid_impl(id)
}

#[inline]
fn try_body_is_valid_impl(id: BodyId) -> ApiResult<bool> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(body_is_valid_impl(id))
}

pub(crate) trait BodyRuntimeHandle {
    fn body_id(&self) -> BodyId;
    fn body_world_core(&self) -> &WorldCore;

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_body_valid(self.body_id());
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_body_valid(self.body_id())
    }

    fn world_id_raw(&self) -> ffi::b2WorldId {
        body_world_id_checked_impl(self.body_id())
    }

    fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        try_body_world_id_raw_impl(self.body_id())
    }

    fn is_valid(&self) -> bool {
        body_is_valid_checked_impl(self.body_id())
    }

    fn try_is_valid(&self) -> ApiResult<bool> {
        try_body_is_valid_impl(self.body_id())
    }

    fn position(&self) -> Vec2 {
        self.assert_valid();
        body_position_impl(self.body_id())
    }

    fn try_position(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_position_impl(self.body_id()))
    }

    fn linear_velocity(&self) -> Vec2 {
        self.assert_valid();
        body_linear_velocity_impl(self.body_id())
    }

    fn try_linear_velocity(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_linear_velocity_impl(self.body_id()))
    }

    fn angular_velocity(&self) -> f32 {
        self.assert_valid();
        body_angular_velocity_impl(self.body_id())
    }

    fn try_angular_velocity(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_angular_velocity_impl(self.body_id()))
    }

    fn rotation(&self) -> crate::Rot {
        self.assert_valid();
        body_rotation_impl(self.body_id())
    }

    fn try_rotation(&self) -> ApiResult<crate::Rot> {
        self.check_valid()?;
        Ok(body_rotation_impl(self.body_id()))
    }

    fn rotation_raw(&self) -> ffi::b2Rot {
        self.assert_valid();
        body_rotation_raw_impl(self.body_id())
    }

    fn try_rotation_raw(&self) -> ApiResult<ffi::b2Rot> {
        self.check_valid()?;
        Ok(body_rotation_raw_impl(self.body_id()))
    }

    fn transform(&self) -> crate::Transform {
        self.assert_valid();
        body_transform_impl(self.body_id())
    }

    fn try_transform(&self) -> ApiResult<crate::Transform> {
        self.check_valid()?;
        Ok(body_transform_impl(self.body_id()))
    }

    fn transform_raw(&self) -> ffi::b2Transform {
        self.assert_valid();
        body_transform_raw_impl(self.body_id())
    }

    fn try_transform_raw(&self) -> ApiResult<ffi::b2Transform> {
        self.check_valid()?;
        Ok(body_transform_raw_impl(self.body_id()))
    }

    fn aabb(&self) -> Aabb {
        self.assert_valid();
        body_aabb_impl(self.body_id())
    }

    fn try_aabb(&self) -> ApiResult<Aabb> {
        self.check_valid()?;
        Ok(body_aabb_impl(self.body_id()))
    }

    fn local_point<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        body_local_point_impl(self.body_id(), world_point)
    }

    fn try_local_point<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_point_impl(self.body_id(), world_point))
    }

    fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        body_world_point_impl(self.body_id(), local_point)
    }

    fn try_world_point<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_point_impl(self.body_id(), local_point))
    }

    fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Vec2 {
        self.assert_valid();
        body_local_vector_impl(self.body_id(), world_vector)
    }

    fn try_local_vector<V: Into<Vec2>>(&self, world_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_vector_impl(self.body_id(), world_vector))
    }

    fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Vec2 {
        self.assert_valid();
        body_world_vector_impl(self.body_id(), local_vector)
    }

    fn try_world_vector<V: Into<Vec2>>(&self, local_vector: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_vector_impl(self.body_id(), local_vector))
    }

    fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Vec2 {
        self.assert_valid();
        body_local_point_velocity_impl(self.body_id(), local_point)
    }

    fn try_local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_point_velocity_impl(self.body_id(), local_point))
    }

    fn world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> Vec2 {
        self.assert_valid();
        body_world_point_velocity_impl(self.body_id(), world_point)
    }

    fn try_world_point_velocity<V: Into<Vec2>>(&self, world_point: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_point_velocity_impl(self.body_id(), world_point))
    }

    fn set_position_and_rotation<V: Into<Vec2>>(&mut self, position: V, angle_radians: f32) {
        self.assert_valid();
        body_set_position_and_rotation_impl(self.body_id(), position, angle_radians);
    }

    fn try_set_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        position: V,
        angle_radians: f32,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_set_position_and_rotation_impl(self.body_id(), position, angle_radians);
        Ok(())
    }

    fn set_linear_velocity<V: Into<Vec2>>(&mut self, velocity: V) {
        self.assert_valid();
        body_set_linear_velocity_impl(self.body_id(), velocity)
    }

    fn try_set_linear_velocity<V: Into<Vec2>>(&mut self, velocity: V) -> ApiResult<()> {
        self.check_valid()?;
        body_set_linear_velocity_impl(self.body_id(), velocity);
        Ok(())
    }

    fn set_angular_velocity(&mut self, angular_velocity: f32) {
        self.assert_valid();
        body_set_angular_velocity_impl(self.body_id(), angular_velocity)
    }

    fn try_set_angular_velocity(&mut self, angular_velocity: f32) -> ApiResult<()> {
        self.check_valid()?;
        body_set_angular_velocity_impl(self.body_id(), angular_velocity);
        Ok(())
    }

    fn set_target_transform(&mut self, target: crate::Transform, time_step: f32, wake: bool) {
        self.assert_valid();
        body_set_target_transform_impl(self.body_id(), target, time_step, wake);
    }

    fn try_set_target_transform(
        &mut self,
        target: crate::Transform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_set_target_transform_impl(self.body_id(), target, time_step, wake);
        Ok(())
    }

    fn contact_data(&self) -> Vec<ContactData> {
        body_contact_data_checked_impl(self.body_id())
    }

    fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        body_contact_data_into_checked_impl(self.body_id(), out);
    }

    fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        try_body_contact_data_impl(self.body_id())
    }

    fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        try_body_contact_data_into_impl(self.body_id(), out)
    }

    fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        body_contact_data_raw_checked_impl(self.body_id())
    }

    fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        body_contact_data_raw_into_checked_impl(self.body_id(), out);
    }

    fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        try_body_contact_data_raw_impl(self.body_id())
    }

    fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        try_body_contact_data_raw_into_impl(self.body_id(), out)
    }

    fn apply_force<F: Into<Vec2>, P: Into<Vec2>>(&mut self, force: F, point: P, wake: bool) {
        self.assert_valid();
        body_apply_force_impl(self.body_id(), force, point, wake);
    }

    fn try_apply_force<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_force_impl(self.body_id(), force, point, wake);
        Ok(())
    }

    fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        self.assert_valid();
        body_apply_force_to_center_impl(self.body_id(), force, wake);
    }

    fn try_apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_force_to_center_impl(self.body_id(), force, wake);
        Ok(())
    }

    fn apply_torque(&mut self, torque: f32, wake: bool) {
        self.assert_valid();
        body_apply_torque_impl(self.body_id(), torque, wake)
    }

    fn try_apply_torque(&mut self, torque: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_torque_impl(self.body_id(), torque, wake);
        Ok(())
    }

    fn clear_forces(&mut self) {
        self.assert_valid();
        body_clear_forces_impl(self.body_id());
    }

    fn try_clear_forces(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_clear_forces_impl(self.body_id());
        Ok(())
    }

    fn apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        self.assert_valid();
        body_apply_linear_impulse_impl(self.body_id(), impulse, point, wake);
    }

    fn try_apply_linear_impulse<F: Into<Vec2>, P: Into<Vec2>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_linear_impulse_impl(self.body_id(), impulse, point, wake);
        Ok(())
    }

    fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        self.assert_valid();
        body_apply_linear_impulse_to_center_impl(self.body_id(), impulse, wake);
    }

    fn try_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_linear_impulse_to_center_impl(self.body_id(), impulse, wake);
        Ok(())
    }

    fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        self.assert_valid();
        body_apply_angular_impulse_impl(self.body_id(), impulse, wake)
    }

    fn try_apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_angular_impulse_impl(self.body_id(), impulse, wake);
        Ok(())
    }

    fn mass(&self) -> f32 {
        self.assert_valid();
        body_mass_impl(self.body_id())
    }

    fn try_mass(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_mass_impl(self.body_id()))
    }

    fn rotational_inertia(&self) -> f32 {
        self.assert_valid();
        body_rotational_inertia_impl(self.body_id())
    }

    fn try_rotational_inertia(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_rotational_inertia_impl(self.body_id()))
    }

    fn local_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        body_local_center_of_mass_impl(self.body_id())
    }

    fn try_local_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_local_center_of_mass_impl(self.body_id()))
    }

    fn world_center_of_mass(&self) -> Vec2 {
        self.assert_valid();
        body_world_center_of_mass_impl(self.body_id())
    }

    fn try_world_center_of_mass(&self) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(body_world_center_of_mass_impl(self.body_id()))
    }

    fn mass_data(&self) -> MassData {
        self.assert_valid();
        body_mass_data_impl(self.body_id())
    }

    fn try_mass_data(&self) -> ApiResult<MassData> {
        self.check_valid()?;
        Ok(body_mass_data_impl(self.body_id()))
    }

    fn set_mass_data(&mut self, mass_data: MassData) {
        self.assert_valid();
        assert_mass_data_valid(mass_data);
        body_set_mass_data_impl(self.body_id(), mass_data);
    }

    fn try_set_mass_data(&mut self, mass_data: MassData) -> ApiResult<()> {
        self.check_valid()?;
        check_mass_data_valid(mass_data)?;
        body_set_mass_data_impl(self.body_id(), mass_data);
        Ok(())
    }

    fn apply_mass_from_shapes(&mut self) {
        self.assert_valid();
        body_apply_mass_from_shapes_impl(self.body_id());
    }

    fn try_apply_mass_from_shapes(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_apply_mass_from_shapes_impl(self.body_id());
        Ok(())
    }

    fn shape_count(&self) -> i32 {
        body_shape_count_checked_impl(self.body_id())
    }

    fn try_shape_count(&self) -> ApiResult<i32> {
        try_body_shape_count_impl(self.body_id())
    }

    fn shapes(&self) -> Vec<ShapeId> {
        body_shapes_checked_impl(self.body_id())
    }

    fn shapes_into(&self, out: &mut Vec<ShapeId>) {
        body_shapes_into_checked_impl(self.body_id(), out);
    }

    fn try_shapes(&self) -> ApiResult<Vec<ShapeId>> {
        try_body_shapes_impl(self.body_id())
    }

    fn try_shapes_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_body_shapes_into_impl(self.body_id(), out)
    }

    fn joint_count(&self) -> i32 {
        body_joint_count_checked_impl(self.body_id())
    }

    fn try_joint_count(&self) -> ApiResult<i32> {
        try_body_joint_count_impl(self.body_id())
    }

    fn joints(&self) -> Vec<JointId> {
        body_joints_checked_impl(self.body_id())
    }

    fn joints_into(&self, out: &mut Vec<JointId>) {
        body_joints_into_checked_impl(self.body_id(), out);
    }

    fn try_joints(&self) -> ApiResult<Vec<JointId>> {
        try_body_joints_impl(self.body_id())
    }

    fn try_joints_into(&self, out: &mut Vec<JointId>) -> ApiResult<()> {
        try_body_joints_into_impl(self.body_id(), out)
    }

    fn body_type(&self) -> BodyType {
        self.assert_valid();
        body_type_impl(self.body_id())
    }

    fn try_body_type(&self) -> ApiResult<BodyType> {
        self.check_valid()?;
        Ok(body_type_impl(self.body_id()))
    }

    fn set_body_type(&mut self, body_type: BodyType) {
        self.assert_valid();
        body_set_type_impl(self.body_id(), body_type)
    }

    fn try_set_body_type(&mut self, body_type: BodyType) -> ApiResult<()> {
        self.check_valid()?;
        body_set_type_impl(self.body_id(), body_type);
        Ok(())
    }

    fn gravity_scale(&self) -> f32 {
        self.assert_valid();
        body_gravity_scale_impl(self.body_id())
    }

    fn try_gravity_scale(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_gravity_scale_impl(self.body_id()))
    }

    fn set_gravity_scale(&mut self, gravity_scale: f32) {
        self.assert_valid();
        assert!(
            crate::is_valid_float(gravity_scale),
            "gravity_scale must be finite, got {gravity_scale}"
        );
        body_set_gravity_scale_impl(self.body_id(), gravity_scale)
    }

    fn try_set_gravity_scale(&mut self, gravity_scale: f32) -> ApiResult<()> {
        self.check_valid()?;
        if !crate::is_valid_float(gravity_scale) {
            return Err(ApiError::InvalidArgument);
        }
        body_set_gravity_scale_impl(self.body_id(), gravity_scale);
        Ok(())
    }

    fn linear_damping(&self) -> f32 {
        self.assert_valid();
        body_linear_damping_impl(self.body_id())
    }

    fn try_linear_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_linear_damping_impl(self.body_id()))
    }

    fn set_linear_damping(&mut self, linear_damping: f32) {
        self.assert_valid();
        assert_non_negative_finite_body_scalar("linear_damping", linear_damping);
        body_set_linear_damping_impl(self.body_id(), linear_damping)
    }

    fn try_set_linear_damping(&mut self, linear_damping: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_body_scalar(linear_damping)?;
        body_set_linear_damping_impl(self.body_id(), linear_damping);
        Ok(())
    }

    fn angular_damping(&self) -> f32 {
        self.assert_valid();
        body_angular_damping_impl(self.body_id())
    }

    fn try_angular_damping(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_angular_damping_impl(self.body_id()))
    }

    fn set_angular_damping(&mut self, angular_damping: f32) {
        self.assert_valid();
        assert_non_negative_finite_body_scalar("angular_damping", angular_damping);
        body_set_angular_damping_impl(self.body_id(), angular_damping)
    }

    fn try_set_angular_damping(&mut self, angular_damping: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_body_scalar(angular_damping)?;
        body_set_angular_damping_impl(self.body_id(), angular_damping);
        Ok(())
    }

    fn enable_sleep(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_sleep_impl(self.body_id(), flag)
    }

    fn try_enable_sleep(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_sleep_impl(self.body_id(), flag);
        Ok(())
    }

    fn is_sleep_enabled(&self) -> bool {
        self.assert_valid();
        body_is_sleep_enabled_impl(self.body_id())
    }

    fn try_is_sleep_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_sleep_enabled_impl(self.body_id()))
    }

    fn set_sleep_threshold(&mut self, sleep_threshold: f32) {
        self.assert_valid();
        body_set_sleep_threshold_impl(self.body_id(), sleep_threshold)
    }

    fn try_set_sleep_threshold(&mut self, sleep_threshold: f32) -> ApiResult<()> {
        self.check_valid()?;
        body_set_sleep_threshold_impl(self.body_id(), sleep_threshold);
        Ok(())
    }

    fn sleep_threshold(&self) -> f32 {
        self.assert_valid();
        body_sleep_threshold_impl(self.body_id())
    }

    fn try_sleep_threshold(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(body_sleep_threshold_impl(self.body_id()))
    }

    fn is_awake(&self) -> bool {
        self.assert_valid();
        body_is_awake_impl(self.body_id())
    }

    fn try_is_awake(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_awake_impl(self.body_id()))
    }

    fn set_awake(&mut self, awake: bool) {
        self.assert_valid();
        body_set_awake_impl(self.body_id(), awake)
    }

    fn try_set_awake(&mut self, awake: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_set_awake_impl(self.body_id(), awake);
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.assert_valid();
        body_is_enabled_impl(self.body_id())
    }

    fn try_is_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_enabled_impl(self.body_id()))
    }

    fn enable(&mut self) {
        self.assert_valid();
        body_enable_impl(self.body_id())
    }

    fn try_enable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_impl(self.body_id());
        Ok(())
    }

    fn disable(&mut self) {
        self.assert_valid();
        body_disable_impl(self.body_id())
    }

    fn try_disable(&mut self) -> ApiResult<()> {
        self.check_valid()?;
        body_disable_impl(self.body_id());
        Ok(())
    }

    fn is_bullet(&self) -> bool {
        self.assert_valid();
        body_is_bullet_impl(self.body_id())
    }

    fn try_is_bullet(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(body_is_bullet_impl(self.body_id()))
    }

    fn set_bullet(&mut self, flag: bool) {
        self.assert_valid();
        body_set_bullet_impl(self.body_id(), flag)
    }

    fn try_set_bullet(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_set_bullet_impl(self.body_id(), flag);
        Ok(())
    }

    fn enable_contact_events(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_contact_events_impl(self.body_id(), flag)
    }

    fn try_enable_contact_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_contact_events_impl(self.body_id(), flag);
        Ok(())
    }

    fn enable_hit_events(&mut self, flag: bool) {
        self.assert_valid();
        body_enable_hit_events_impl(self.body_id(), flag)
    }

    fn try_enable_hit_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        body_enable_hit_events_impl(self.body_id(), flag);
        Ok(())
    }

    fn set_name(&mut self, name: &str) {
        self.assert_valid();
        let cstr = CString::new(name).expect("body name contains an interior NUL byte");
        body_set_name_impl(self.body_id(), &cstr)
    }

    fn try_set_name(&mut self, name: &str) -> ApiResult<()> {
        self.check_valid()?;
        let cstr = CString::new(name).map_err(|_| ApiError::NulByteInString)?;
        body_set_name_impl(self.body_id(), &cstr);
        Ok(())
    }

    fn name(&self) -> Option<String> {
        self.assert_valid();
        body_name_impl(self.body_id())
    }

    fn try_name(&self) -> ApiResult<Option<String>> {
        self.check_valid()?;
        Ok(body_name_impl(self.body_id()))
    }

    unsafe fn set_user_data_ptr_raw(&mut self, user_data: *mut c_void) {
        unsafe {
            body_set_user_data_ptr_raw_checked_impl(
                self.body_world_core(),
                self.body_id(),
                user_data,
            )
        }
    }

    unsafe fn try_set_user_data_ptr_raw(&mut self, user_data: *mut c_void) -> ApiResult<()> {
        unsafe {
            try_body_set_user_data_ptr_raw_impl(self.body_world_core(), self.body_id(), user_data)
        }
    }

    fn user_data_ptr_raw(&self) -> *mut c_void {
        body_user_data_ptr_raw_checked_impl(self.body_id())
    }

    fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_body_user_data_ptr_raw_impl(self.body_id())
    }

    fn set_user_data<T: 'static>(&mut self, value: T) {
        body_set_user_data_checked_impl(self.body_world_core(), self.body_id(), value);
    }

    fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_body_set_user_data_checked_impl(self.body_world_core(), self.body_id(), value)
    }

    fn clear_user_data(&mut self) -> bool {
        body_clear_user_data_checked_impl(self.body_world_core(), self.body_id())
    }

    fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_body_clear_user_data_checked_impl(self.body_world_core(), self.body_id())
    }

    fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        body_with_user_data_checked_impl(self.body_world_core(), self.body_id(), f)
    }

    fn try_with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> ApiResult<Option<R>> {
        try_body_with_user_data_checked_impl(self.body_world_core(), self.body_id(), f)
    }

    fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        body_with_user_data_mut_checked_impl(self.body_world_core(), self.body_id(), f)
    }

    fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_body_with_user_data_mut_checked_impl(self.body_world_core(), self.body_id(), f)
    }

    fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        body_take_user_data_checked_impl(self.body_world_core(), self.body_id())
    }

    fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_body_take_user_data_checked_impl(self.body_world_core(), self.body_id())
    }
}

impl BodyRuntimeHandle for OwnedBody {
    fn body_id(&self) -> BodyId {
        self.id
    }

    fn body_world_core(&self) -> &WorldCore {
        self.core.as_ref()
    }
}

impl<'w> BodyRuntimeHandle for Body<'w> {
    fn body_id(&self) -> BodyId {
        self.id
    }

    fn body_world_core(&self) -> &WorldCore {
        self.core.as_ref()
    }
}
