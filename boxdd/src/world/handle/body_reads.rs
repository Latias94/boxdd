use super::*;

impl WorldHandle {
    pub fn body_transform(&self, body: BodyId) -> Transform {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_transform_impl(body)
    }

    pub fn try_body_transform(&self, body: BodyId) -> crate::error::ApiResult<Transform> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_transform_impl(body))
    }

    pub fn body_position(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_position_impl(body)
    }

    pub fn try_body_position(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_position_impl(body))
    }

    pub fn body_linear_velocity(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_linear_velocity_impl(body)
    }

    pub fn try_body_linear_velocity(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_linear_velocity_impl(body))
    }

    pub fn body_angular_velocity(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_angular_velocity_impl(body)
    }

    pub fn try_body_angular_velocity(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_angular_velocity_impl(body))
    }

    pub fn body_rotation(&self, body: BodyId) -> crate::Rot {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_rotation_impl(body)
    }

    pub fn try_body_rotation(&self, body: BodyId) -> crate::error::ApiResult<crate::Rot> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_rotation_impl(body))
    }

    pub fn body_aabb(&self, body: BodyId) -> Aabb {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_aabb_impl(body)
    }

    pub fn try_body_aabb(&self, body: BodyId) -> crate::error::ApiResult<Aabb> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_aabb_impl(body))
    }

    pub fn body_local_point<V: Into<Vec2>>(&self, body: BodyId, world_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_local_point_impl(body, world_point)
    }

    pub fn try_body_local_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_local_point_impl(body, world_point))
    }

    pub fn body_world_point<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_world_point_impl(body, local_point)
    }

    pub fn try_body_world_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_world_point_impl(body, local_point))
    }

    pub fn body_local_vector<V: Into<Vec2>>(&self, body: BodyId, world_vector: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_local_vector_impl(body, world_vector)
    }

    pub fn try_body_local_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_local_vector_impl(body, world_vector))
    }

    pub fn body_world_vector<V: Into<Vec2>>(&self, body: BodyId, local_vector: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_world_vector_impl(body, local_vector)
    }

    pub fn try_body_world_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_world_vector_impl(body, local_vector))
    }

    pub fn body_local_point_velocity<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_local_point_velocity_impl(body, local_point)
    }

    pub fn try_body_local_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_local_point_velocity_impl(
            body,
            local_point,
        ))
    }

    pub fn body_world_point_velocity<V: Into<Vec2>>(&self, body: BodyId, world_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_world_point_velocity_impl(body, world_point)
    }

    pub fn try_body_world_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_world_point_velocity_impl(
            body,
            world_point,
        ))
    }

    pub fn body_mass(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_mass_impl(body)
    }

    pub fn try_body_mass(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_mass_impl(body))
    }

    pub fn body_rotational_inertia(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_rotational_inertia_impl(body)
    }

    pub fn try_body_rotational_inertia(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_rotational_inertia_impl(body))
    }

    pub fn body_local_center_of_mass(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_local_center_of_mass_impl(body)
    }

    pub fn try_body_local_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_local_center_of_mass_impl(body))
    }

    pub fn body_world_center_of_mass(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_world_center_of_mass_impl(body)
    }

    pub fn try_body_world_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_world_center_of_mass_impl(body))
    }

    pub fn body_mass_data(&self, body: BodyId) -> MassData {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_mass_data_impl(body)
    }

    pub fn try_body_mass_data(&self, body: BodyId) -> crate::error::ApiResult<MassData> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_mass_data_impl(body))
    }

    pub fn body_shape_count(&self, body: BodyId) -> i32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_shape_count_impl(body)
    }

    pub fn try_body_shape_count(&self, body: BodyId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_shape_count_impl(body))
    }

    pub fn body_shapes(&self, body: BodyId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_shapes_impl(body)
    }

    pub fn body_shapes_into(&self, body: BodyId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_shapes_into_impl(body, out);
    }

    pub fn try_body_shapes(&self, body: BodyId) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_shapes_impl(body))
    }

    pub fn try_body_shapes_into(
        &self,
        body: BodyId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_shapes_into_impl(body, out);
        Ok(())
    }

    pub fn body_joint_count(&self, body: BodyId) -> i32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_joint_count_impl(body)
    }

    pub fn try_body_joint_count(&self, body: BodyId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_joint_count_impl(body))
    }

    pub fn body_joints(&self, body: BodyId) -> Vec<JointId> {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_joints_impl(body)
    }

    pub fn body_joints_into(&self, body: BodyId, out: &mut Vec<JointId>) {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_joints_into_impl(body, out);
    }

    pub fn try_body_joints(&self, body: BodyId) -> crate::error::ApiResult<Vec<JointId>> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_joints_impl(body))
    }

    pub fn try_body_joints_into(
        &self,
        body: BodyId,
        out: &mut Vec<JointId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::body::body_joints_into_impl(body, out);
        Ok(())
    }

    pub fn body_type(&self, body: BodyId) -> BodyType {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_type_impl(body)
    }

    pub fn try_body_type(&self, body: BodyId) -> crate::error::ApiResult<BodyType> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_type_impl(body))
    }

    pub fn body_gravity_scale(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_gravity_scale_impl(body)
    }

    pub fn try_body_gravity_scale(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_gravity_scale_impl(body))
    }

    pub fn body_linear_damping(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_linear_damping_impl(body)
    }

    pub fn try_body_linear_damping(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_linear_damping_impl(body))
    }

    pub fn body_angular_damping(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_angular_damping_impl(body)
    }

    pub fn try_body_angular_damping(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_angular_damping_impl(body))
    }

    pub fn body_is_sleep_enabled(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_sleep_enabled_impl(body)
    }

    pub fn try_body_is_sleep_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_sleep_enabled_impl(body))
    }

    pub fn body_sleep_threshold(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_sleep_threshold_impl(body)
    }

    pub fn try_body_sleep_threshold(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_sleep_threshold_impl(body))
    }

    pub fn body_is_awake(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_awake_impl(body)
    }

    pub fn try_body_is_awake(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_awake_impl(body))
    }

    pub fn body_is_enabled(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_enabled_impl(body)
    }

    pub fn try_body_is_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_enabled_impl(body))
    }

    pub fn body_motion_locks(&self, body: BodyId) -> MotionLocks {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_motion_locks_impl(body)
    }

    pub fn try_body_motion_locks(&self, body: BodyId) -> crate::error::ApiResult<MotionLocks> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_motion_locks_impl(body))
    }

    pub fn body_is_bullet(&self, body: BodyId) -> bool {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_is_bullet_impl(body)
    }

    pub fn try_body_is_bullet(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_is_bullet_impl(body))
    }

    pub fn body_name(&self, body: BodyId) -> Option<String> {
        crate::core::debug_checks::assert_body_valid(body);
        crate::body::body_name_impl(body)
    }

    pub fn try_body_name(&self, body: BodyId) -> crate::error::ApiResult<Option<String>> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(crate::body::body_name_impl(body))
    }
}
