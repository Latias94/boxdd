use super::*;

impl World {
    /// Return recorded shape flags for shapes created via this wrapper.
    #[cfg(feature = "serialize")]
    pub fn shape_flags(&self, sid: ShapeId) -> Option<ShapeFlagsRecord> {
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .shape_flags(sid)
    }

    pub fn shape_surface_material(&self, shape: ShapeId) -> SurfaceMaterial {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_surface_material_impl(shape)
    }

    pub fn try_shape_surface_material(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<SurfaceMaterial> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_surface_material_impl(shape))
    }

    pub fn shape_body_id(&self, shape: ShapeId) -> BodyId {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_body_id_impl(shape)
    }

    pub fn try_shape_body_id(&self, shape: ShapeId) -> crate::error::ApiResult<BodyId> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_body_id_impl(shape))
    }

    pub fn shape_aabb(&self, shape: ShapeId) -> Aabb {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_aabb_impl(shape)
    }

    pub fn try_shape_aabb(&self, shape: ShapeId) -> crate::error::ApiResult<Aabb> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_aabb_impl(shape))
    }

    pub fn shape_test_point<V: Into<Vec2>>(&self, shape: ShapeId, point: V) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_test_point_impl(shape, point)
    }

    pub fn try_shape_test_point<V: Into<Vec2>>(
        &self,
        shape: ShapeId,
        point: V,
    ) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_test_point_impl(shape, point))
    }

    pub fn shape_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_ray_cast_impl(shape, origin, translation)
    }

    pub fn try_shape_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        shape: ShapeId,
        origin: VO,
        translation: VT,
    ) -> crate::error::ApiResult<CastOutput> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_ray_cast_impl(
            shape,
            origin,
            translation,
        ))
    }

    /// Return the closest point on a shape to `target` (in world coordinates).
    pub fn shape_closest_point<V: Into<Vec2>>(&self, shape: ShapeId, target: V) -> Vec2 {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_closest_point_impl(shape, target)
    }

    pub fn try_shape_closest_point<V: Into<Vec2>>(
        &self,
        shape: ShapeId,
        target: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_closest_point_impl(shape, target))
    }

    pub fn shape_mass_data(&self, shape: ShapeId) -> MassData {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_mass_data_impl(shape)
    }

    pub fn try_shape_mass_data(&self, shape: ShapeId) -> crate::error::ApiResult<MassData> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_mass_data_impl(shape))
    }

    pub fn shape_sensor_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_events_enabled_impl(shape)
    }

    pub fn try_shape_sensor_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_events_enabled_impl(shape))
    }

    pub fn shape_contact_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_contact_events_enabled_impl(shape)
    }

    pub fn try_shape_contact_events_enabled(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_contact_events_enabled_impl(shape))
    }

    pub fn shape_pre_solve_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_pre_solve_events_enabled_impl(shape)
    }

    pub fn try_shape_pre_solve_events_enabled(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_pre_solve_events_enabled_impl(shape))
    }

    pub fn shape_hit_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_hit_events_enabled_impl(shape)
    }

    pub fn try_shape_hit_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_hit_events_enabled_impl(shape))
    }
}
