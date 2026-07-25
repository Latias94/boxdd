use super::*;

#[inline]
fn assert_body_target(core: &WorldCore, body: BodyId) {
    crate::core::callback_state::assert_not_in_callback();
    core.check_body(body)
        .expect("body must be live and belong to this world");
}

#[inline]
fn check_body_target(core: &WorldCore, body: BodyId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_body(body)
}

impl WorldHandle {
    pub fn body_transform(&self, body: BodyId) -> WorldTransform {
        assert_body_target(&self.core, body);
        crate::body::body_transform_impl(body)
    }

    pub fn try_body_transform(&self, body: BodyId) -> crate::error::ApiResult<WorldTransform> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_transform_impl(body))
    }

    pub fn body_position(&self, body: BodyId) -> Position {
        assert_body_target(&self.core, body);
        crate::body::body_position_impl(body)
    }

    pub fn try_body_position(&self, body: BodyId) -> crate::error::ApiResult<Position> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_position_impl(body))
    }

    pub fn body_linear_velocity(&self, body: BodyId) -> Vec2 {
        assert_body_target(&self.core, body);
        crate::body::body_linear_velocity_impl(body)
    }

    pub fn try_body_linear_velocity(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_linear_velocity_impl(body))
    }

    pub fn body_angular_velocity(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_angular_velocity_impl(body)
    }

    pub fn try_body_angular_velocity(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_angular_velocity_impl(body))
    }

    pub fn body_rotation(&self, body: BodyId) -> crate::Rot {
        assert_body_target(&self.core, body);
        crate::body::body_rotation_impl(body)
    }

    pub fn try_body_rotation(&self, body: BodyId) -> crate::error::ApiResult<crate::Rot> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_rotation_impl(body))
    }

    pub fn body_aabb(&self, body: BodyId) -> Aabb {
        assert_body_target(&self.core, body);
        crate::body::body_aabb_impl(body)
    }

    pub fn try_body_aabb(&self, body: BodyId) -> crate::error::ApiResult<Aabb> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_aabb_impl(body))
    }

    pub fn body_local_point<V: Into<Position>>(&self, body: BodyId, world_point: V) -> Vec2 {
        assert_body_target(&self.core, body);
        let world_point =
            crate::body::assert_valid_body_position("world_point", world_point.into());
        assert_body_target(&self.core, body);
        let world_point = crate::body::assert_body_world_point_in_local_range(
            "world_point",
            world_point,
            crate::body::body_position_impl(body),
        );
        let result = crate::body::body_local_point_impl(body, world_point);
        crate::body::assert_valid_body_vec2("local point result", result)
    }

    pub fn try_body_local_point<V: Into<Position>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        let world_point = crate::body::check_valid_body_position(world_point.into())?;
        check_body_target(&self.core, body)?;
        let world_point = crate::body::check_body_world_point_in_local_range(
            world_point,
            crate::body::body_position_impl(body),
        )?;
        crate::body::check_valid_body_vec2(crate::body::body_local_point_impl(body, world_point))
    }

    pub fn body_world_point<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Position {
        assert_body_target(&self.core, body);
        let local_point = crate::body::assert_valid_body_vec2("local_point", local_point.into());
        assert_body_target(&self.core, body);
        let result = crate::body::body_world_point_impl(body, local_point);
        crate::body::assert_valid_body_position("world point result", result)
    }

    pub fn try_body_world_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Position> {
        check_body_target(&self.core, body)?;
        let local_point = crate::body::check_valid_body_vec2(local_point.into())?;
        check_body_target(&self.core, body)?;
        crate::body::check_valid_body_position(crate::body::body_world_point_impl(
            body,
            local_point,
        ))
    }

    pub fn body_local_vector<V: Into<Vec2>>(&self, body: BodyId, world_vector: V) -> Vec2 {
        assert_body_target(&self.core, body);
        let world_vector = crate::body::assert_valid_body_vec2("world_vector", world_vector.into());
        assert_body_target(&self.core, body);
        let result = crate::body::body_local_vector_impl(body, world_vector);
        crate::body::assert_valid_body_vec2("local vector result", result)
    }

    pub fn try_body_local_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        let world_vector = crate::body::check_valid_body_vec2(world_vector.into())?;
        check_body_target(&self.core, body)?;
        crate::body::check_valid_body_vec2(crate::body::body_local_vector_impl(body, world_vector))
    }

    pub fn body_world_vector<V: Into<Vec2>>(&self, body: BodyId, local_vector: V) -> Vec2 {
        assert_body_target(&self.core, body);
        let local_vector = crate::body::assert_valid_body_vec2("local_vector", local_vector.into());
        assert_body_target(&self.core, body);
        let result = crate::body::body_world_vector_impl(body, local_vector);
        crate::body::assert_valid_body_vec2("world vector result", result)
    }

    pub fn try_body_world_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        let local_vector = crate::body::check_valid_body_vec2(local_vector.into())?;
        check_body_target(&self.core, body)?;
        crate::body::check_valid_body_vec2(crate::body::body_world_vector_impl(body, local_vector))
    }

    pub fn body_local_point_velocity<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        assert_body_target(&self.core, body);
        let local_point = crate::body::assert_valid_body_vec2("local_point", local_point.into());
        assert_body_target(&self.core, body);
        let result = crate::body::body_local_point_velocity_impl(body, local_point);
        crate::body::assert_valid_body_vec2("local point velocity result", result)
    }

    pub fn try_body_local_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        let local_point = crate::body::check_valid_body_vec2(local_point.into())?;
        check_body_target(&self.core, body)?;
        crate::body::check_valid_body_vec2(crate::body::body_local_point_velocity_impl(
            body,
            local_point,
        ))
    }

    pub fn body_world_point_velocity<V: Into<Position>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> Vec2 {
        assert_body_target(&self.core, body);
        let world_point =
            crate::body::assert_valid_body_position("world_point", world_point.into());
        assert_body_target(&self.core, body);
        let world_point = crate::body::assert_body_world_point_in_local_range(
            "world_point",
            world_point,
            crate::body::body_world_center_of_mass_impl(body),
        );
        let result = crate::body::body_world_point_velocity_impl(body, world_point);
        crate::body::assert_valid_body_vec2("world point velocity result", result)
    }

    pub fn try_body_world_point_velocity<V: Into<Position>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        let world_point = crate::body::check_valid_body_position(world_point.into())?;
        check_body_target(&self.core, body)?;
        let world_point = crate::body::check_body_world_point_in_local_range(
            world_point,
            crate::body::body_world_center_of_mass_impl(body),
        )?;
        crate::body::check_valid_body_vec2(crate::body::body_world_point_velocity_impl(
            body,
            world_point,
        ))
    }

    pub fn body_mass(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_mass_impl(body)
    }

    pub fn try_body_mass(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_mass_impl(body))
    }

    pub fn body_rotational_inertia(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_rotational_inertia_impl(body)
    }

    pub fn try_body_rotational_inertia(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_rotational_inertia_impl(body))
    }

    pub fn body_local_center_of_mass(&self, body: BodyId) -> Vec2 {
        assert_body_target(&self.core, body);
        crate::body::body_local_center_of_mass_impl(body)
    }

    pub fn try_body_local_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_local_center_of_mass_impl(body))
    }

    pub fn body_world_center_of_mass(&self, body: BodyId) -> Position {
        assert_body_target(&self.core, body);
        crate::body::body_world_center_of_mass_impl(body)
    }

    pub fn try_body_world_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Position> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_world_center_of_mass_impl(body))
    }

    pub fn body_mass_data(&self, body: BodyId) -> MassData {
        assert_body_target(&self.core, body);
        crate::body::body_mass_data_impl(body)
    }

    pub fn try_body_mass_data(&self, body: BodyId) -> crate::error::ApiResult<MassData> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_mass_data_impl(body))
    }

    pub fn body_shape_count(&self, body: BodyId) -> i32 {
        assert_body_target(&self.core, body);
        crate::body::body_shape_count_impl(body)
    }

    pub fn try_body_shape_count(&self, body: BodyId) -> crate::error::ApiResult<i32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_shape_count_impl(body))
    }

    pub fn body_shapes(&self, body: BodyId) -> Vec<ShapeId> {
        assert_body_target(&self.core, body);
        crate::body::body_shapes_in_impl(self.core.brand(), body)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    pub fn body_shapes_into(&self, body: BodyId, out: &mut Vec<ShapeId>) {
        assert_body_target(&self.core, body);
        crate::body::body_shapes_into_in_impl(self.core.brand(), body, out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    pub fn try_body_shapes(&self, body: BodyId) -> crate::error::ApiResult<Vec<ShapeId>> {
        check_body_target(&self.core, body)?;
        crate::body::body_shapes_in_impl(self.core.brand(), body)
    }

    pub fn try_body_shapes_into(
        &self,
        body: BodyId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_shapes_into_in_impl(self.core.brand(), body, out)
    }

    pub fn body_joint_count(&self, body: BodyId) -> i32 {
        assert_body_target(&self.core, body);
        crate::body::body_joint_count_impl(body)
    }

    pub fn try_body_joint_count(&self, body: BodyId) -> crate::error::ApiResult<i32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_joint_count_impl(body))
    }

    pub fn body_joints(&self, body: BodyId) -> Vec<JointId> {
        assert_body_target(&self.core, body);
        crate::body::body_joints_in_impl(self.core.brand(), body)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    pub fn body_joints_into(&self, body: BodyId, out: &mut Vec<JointId>) {
        assert_body_target(&self.core, body);
        crate::body::body_joints_into_in_impl(self.core.brand(), body, out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    pub fn try_body_joints(&self, body: BodyId) -> crate::error::ApiResult<Vec<JointId>> {
        check_body_target(&self.core, body)?;
        crate::body::body_joints_in_impl(self.core.brand(), body)
    }

    pub fn try_body_joints_into(
        &self,
        body: BodyId,
        out: &mut Vec<JointId>,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_joints_into_in_impl(self.core.brand(), body, out)
    }

    /// Return the body's simulation type.
    ///
    /// # Panics
    ///
    /// Panics if the body or world is unavailable or Box2D returns an unknown native
    /// discriminant. An unknown discriminant poisons the world before this method panics.
    pub fn body_type(&self, body: BodyId) -> BodyType {
        self.try_body_type(body)
            .expect("body is unavailable or Box2D returned an unknown body type")
    }

    /// Try to return the body's simulation type.
    ///
    /// An unknown native discriminant returns
    /// [`ApiError::InvalidNativeBodyType`](crate::ApiError::InvalidNativeBodyType) and poisons the
    /// world.
    pub fn try_body_type(&self, body: BodyId) -> crate::error::ApiResult<BodyType> {
        check_body_target(&self.core, body)?;
        crate::body::try_body_type_impl(&self.core, body)
    }

    pub fn body_gravity_scale(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_gravity_scale_impl(body)
    }

    pub fn try_body_gravity_scale(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_gravity_scale_impl(body))
    }

    pub fn body_linear_damping(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_linear_damping_impl(body)
    }

    pub fn try_body_linear_damping(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_linear_damping_impl(body))
    }

    pub fn body_angular_damping(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_angular_damping_impl(body)
    }

    pub fn try_body_angular_damping(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_angular_damping_impl(body))
    }

    pub fn body_is_sleep_enabled(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_sleep_enabled_impl(body)
    }

    pub fn try_body_is_sleep_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_sleep_enabled_impl(body))
    }

    pub fn body_sleep_threshold(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_sleep_threshold_impl(body)
    }

    pub fn try_body_sleep_threshold(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_sleep_threshold_impl(body))
    }

    pub fn body_is_awake(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_awake_impl(body)
    }

    pub fn try_body_is_awake(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_awake_impl(body))
    }

    pub fn body_is_enabled(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_enabled_impl(body)
    }

    pub fn try_body_is_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_enabled_impl(body))
    }

    pub fn body_motion_locks(&self, body: BodyId) -> MotionLocks {
        assert_body_target(&self.core, body);
        crate::body::body_motion_locks_impl(body)
    }

    pub fn try_body_motion_locks(&self, body: BodyId) -> crate::error::ApiResult<MotionLocks> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_motion_locks_impl(body))
    }

    pub fn body_is_bullet(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_bullet_impl(body)
    }

    pub fn try_body_is_bullet(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_bullet_impl(body))
    }

    pub fn body_is_contact_recycling_enabled(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_contact_recycling_enabled_impl(body)
    }

    pub fn try_body_is_contact_recycling_enabled(
        &self,
        body: BodyId,
    ) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_contact_recycling_enabled_impl(body))
    }

    pub fn body_name(&self, body: BodyId) -> Option<String> {
        assert_body_target(&self.core, body);
        crate::body::body_name_impl(body)
    }

    pub fn try_body_name(&self, body: BodyId) -> crate::error::ApiResult<Option<String>> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_name_impl(body))
    }
}
