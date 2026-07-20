use super::*;

impl World {
    /// Return recorded shape flags for shapes created via this wrapper.
    #[cfg(feature = "serialize")]
    pub fn shape_flags(&self, sid: ShapeId) -> Option<ShapeFlagsRecord> {
        assert_shape_target(&self.core, sid);
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .shape_flags(sid)
    }

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

    /// Return the closest point on a shape to `target` (in world coordinates).
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
}
