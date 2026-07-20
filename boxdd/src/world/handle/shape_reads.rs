use super::*;

#[inline]
fn assert_shape_target(core: &WorldCore, shape: ShapeId) {
    crate::core::callback_state::assert_not_in_callback();
    core.check_shape(shape)
        .expect("shape must be live and belong to this world");
}

#[inline]
fn check_shape_target(core: &WorldCore, shape: ShapeId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_shape(shape)
}

impl WorldHandle {
    pub fn shape_surface_material(&self, shape: ShapeId) -> SurfaceMaterial {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_surface_material_impl(shape)
    }

    pub fn try_shape_surface_material(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<SurfaceMaterial> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_surface_material_impl(shape))
    }

    pub fn shape_body_id(&self, shape: ShapeId) -> BodyId {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_body_id_in_impl(&self.core, shape)
            .expect("Box2D returned an invalid body id for a validated shape")
    }

    pub fn try_shape_body_id(&self, shape: ShapeId) -> crate::error::ApiResult<BodyId> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_body_id_in_impl(&self.core, shape)
    }

    pub fn shape_aabb(&self, shape: ShapeId) -> Aabb {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_aabb_impl(shape)
    }

    pub fn try_shape_aabb(&self, shape: ShapeId) -> crate::error::ApiResult<Aabb> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_aabb_impl(shape))
    }

    /// Test an absolute world-space point against a shape.
    pub fn shape_test_point(&self, shape: ShapeId, point: Position) -> bool {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_test_point_checked_impl(shape, point)
    }

    pub fn try_shape_test_point(
        &self,
        shape: ShapeId,
        point: Position,
    ) -> crate::error::ApiResult<bool> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::try_shape_test_point_checked_impl(shape, point)
    }

    /// Cast from an absolute world `origin` by a local `translation`.
    ///
    /// The returned hit point is an absolute world position.
    pub fn shape_ray_cast<VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: Position,
        translation: VT,
    ) -> WorldCastOutput {
        assert_shape_target(&self.core, shape);
        let translation = translation.into();
        let origin = crate::body::assert_valid_body_position("origin", origin);
        crate::shapes::assert_shape_vec2_valid("translation", translation);
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_ray_cast_checked_impl(shape, origin, translation)
    }

    pub fn try_shape_ray_cast<VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: Position,
        translation: VT,
    ) -> crate::error::ApiResult<WorldCastOutput> {
        check_shape_target(&self.core, shape)?;
        let translation = translation.into();
        let origin = crate::body::check_valid_body_position(origin)?;
        crate::shapes::check_shape_vec2_valid(translation)?;
        check_shape_target(&self.core, shape)?;
        crate::shapes::try_shape_ray_cast_checked_impl(shape, origin, translation)
    }

    /// Return the closest absolute world position on a shape to `target`.
    pub fn shape_closest_point(&self, shape: ShapeId, target: Position) -> Position {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_closest_point_checked_impl(shape, target)
    }

    pub fn try_shape_closest_point(
        &self,
        shape: ShapeId,
        target: Position,
    ) -> crate::error::ApiResult<Position> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::try_shape_closest_point_checked_impl(shape, target)
    }

    pub fn shape_mass_data(&self, shape: ShapeId) -> MassData {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_mass_data_impl(shape)
    }

    pub fn try_shape_mass_data(&self, shape: ShapeId) -> crate::error::ApiResult<MassData> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_mass_data_impl(shape))
    }

    pub fn shape_sensor_events_enabled(&self, shape: ShapeId) -> bool {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_events_enabled_impl(shape)
    }

    pub fn try_shape_sensor_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_sensor_events_enabled_impl(shape))
    }

    pub fn shape_contact_events_enabled(&self, shape: ShapeId) -> bool {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_contact_events_enabled_impl(shape)
    }

    pub fn try_shape_contact_events_enabled(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<bool> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_contact_events_enabled_impl(shape))
    }

    pub fn shape_pre_solve_events_enabled(&self, shape: ShapeId) -> bool {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_pre_solve_events_enabled_impl(shape)
    }

    pub fn try_shape_pre_solve_events_enabled(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<bool> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_pre_solve_events_enabled_impl(shape))
    }

    pub fn shape_hit_events_enabled(&self, shape: ShapeId) -> bool {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_hit_events_enabled_impl(shape)
    }

    pub fn try_shape_hit_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_hit_events_enabled_impl(shape))
    }

    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_capacity_impl(shape)
    }

    pub fn try_shape_sensor_capacity(&self, shape: ShapeId) -> crate::error::ApiResult<i32> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_sensor_capacity_impl(shape))
    }

    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_in_impl(self.core.brand(), shape)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    pub fn shape_sensor_overlaps_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_into_in_impl(self.core.brand(), shape, out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    pub fn try_shape_sensor_overlaps(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_in_impl(self.core.brand(), shape)
    }

    pub fn try_shape_sensor_overlaps_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_into_in_impl(self.core.brand(), shape, out)
    }

    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_valid_in_impl(self.core.brand(), shape)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    pub fn try_shape_sensor_overlaps_valid(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_valid_in_impl(self.core.brand(), shape)
    }

    pub fn shape_sensor_overlaps_valid_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_valid_into_in_impl(self.core.brand(), shape, out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    pub fn try_shape_sensor_overlaps_valid_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_valid_into_in_impl(self.core.brand(), shape, out)
    }
}
