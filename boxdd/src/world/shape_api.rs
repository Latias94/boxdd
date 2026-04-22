use super::*;

fn world_shape_set_circle_impl(shape: ShapeId, circle: &crate::shapes::Circle) {
    crate::core::debug_checks::assert_shape_valid(shape);
    crate::shapes::assert_circle_geometry_valid(circle);
    let raw = circle.into_raw();
    unsafe { ffi::b2Shape_SetCircle(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_circle_impl(
    shape: ShapeId,
    circle: &crate::shapes::Circle,
) -> crate::error::ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(shape)?;
    crate::shapes::check_circle_geometry_valid(circle)?;
    let raw = circle.into_raw();
    unsafe { ffi::b2Shape_SetCircle(raw_shape_id(shape), &raw) }
    Ok(())
}

fn world_shape_set_segment_impl(shape: ShapeId, segment: &crate::shapes::Segment) {
    crate::core::debug_checks::assert_shape_valid(shape);
    crate::shapes::assert_segment_geometry_valid(segment);
    let raw = segment.into_raw();
    unsafe { ffi::b2Shape_SetSegment(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_segment_impl(
    shape: ShapeId,
    segment: &crate::shapes::Segment,
) -> crate::error::ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(shape)?;
    crate::shapes::check_segment_geometry_valid(segment)?;
    let raw = segment.into_raw();
    unsafe { ffi::b2Shape_SetSegment(raw_shape_id(shape), &raw) }
    Ok(())
}

fn world_shape_set_capsule_impl(shape: ShapeId, capsule: &crate::shapes::Capsule) {
    crate::core::debug_checks::assert_shape_valid(shape);
    crate::shapes::assert_capsule_geometry_valid(capsule);
    let raw = capsule.into_raw();
    unsafe { ffi::b2Shape_SetCapsule(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_capsule_impl(
    shape: ShapeId,
    capsule: &crate::shapes::Capsule,
) -> crate::error::ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(shape)?;
    crate::shapes::check_capsule_geometry_valid(capsule)?;
    let raw = capsule.into_raw();
    unsafe { ffi::b2Shape_SetCapsule(raw_shape_id(shape), &raw) }
    Ok(())
}

fn world_shape_set_polygon_impl(shape: ShapeId, polygon: &crate::shapes::Polygon) {
    crate::core::debug_checks::assert_shape_valid(shape);
    crate::shapes::assert_polygon_geometry_valid(polygon);
    let raw = polygon.into_raw();
    unsafe { ffi::b2Shape_SetPolygon(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_polygon_impl(
    shape: ShapeId,
    polygon: &crate::shapes::Polygon,
) -> crate::error::ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(shape)?;
    crate::shapes::check_polygon_geometry_valid(polygon)?;
    let raw = polygon.into_raw();
    unsafe { ffi::b2Shape_SetPolygon(raw_shape_id(shape), &raw) }
    Ok(())
}

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

    // Shape helpers (ID-style)
    pub fn shape_set_circle(&mut self, shape: ShapeId, circle: &crate::shapes::Circle) {
        world_shape_set_circle_impl(shape, circle)
    }

    pub fn try_shape_set_circle(
        &mut self,
        shape: ShapeId,
        circle: &crate::shapes::Circle,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_circle_impl(shape, circle)
    }

    pub fn shape_set_segment(&mut self, shape: ShapeId, segment: &crate::shapes::Segment) {
        world_shape_set_segment_impl(shape, segment)
    }

    pub fn try_shape_set_segment(
        &mut self,
        shape: ShapeId,
        segment: &crate::shapes::Segment,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_segment_impl(shape, segment)
    }

    pub fn shape_set_capsule(&mut self, shape: ShapeId, capsule: &crate::shapes::Capsule) {
        world_shape_set_capsule_impl(shape, capsule)
    }

    pub fn try_shape_set_capsule(
        &mut self,
        shape: ShapeId,
        capsule: &crate::shapes::Capsule,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_capsule_impl(shape, capsule)
    }

    pub fn shape_set_polygon(&mut self, shape: ShapeId, polygon: &crate::shapes::Polygon) {
        world_shape_set_polygon_impl(shape, polygon)
    }

    pub fn try_shape_set_polygon(
        &mut self,
        shape: ShapeId,
        polygon: &crate::shapes::Polygon,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_polygon_impl(shape, polygon)
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

    pub fn shape_set_surface_material(&mut self, shape: ShapeId, material: &SurfaceMaterial) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_set_surface_material_impl(shape, material)
    }

    pub fn try_shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &SurfaceMaterial,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_set_surface_material_impl(shape, material);
        Ok(())
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

    /// Apply wind force/torque approximation to a shape.
    pub fn shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_apply_wind_impl(shape, wind, drag, lift, wake)
    }

    pub fn try_shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_apply_wind_impl(shape, wind, drag, lift, wake);
        Ok(())
    }

    pub fn shape_mass_data(&self, shape: ShapeId) -> MassData {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_mass_data_impl(shape)
    }

    pub fn try_shape_mass_data(&self, shape: ShapeId) -> crate::error::ApiResult<MassData> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_mass_data_impl(shape))
    }

    pub fn shape_enable_sensor_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_sensor_events_impl(shape, flag)
    }

    pub fn try_shape_enable_sensor_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_sensor_events_impl(shape, flag);
        Ok(())
    }

    pub fn shape_sensor_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_events_enabled_impl(shape)
    }

    pub fn try_shape_sensor_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_events_enabled_impl(shape))
    }

    pub fn shape_enable_contact_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_contact_events_impl(shape, flag)
    }

    pub fn try_shape_enable_contact_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_contact_events_impl(shape, flag);
        Ok(())
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

    pub fn shape_enable_pre_solve_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_pre_solve_events_impl(shape, flag)
    }

    pub fn try_shape_enable_pre_solve_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_pre_solve_events_impl(shape, flag);
        Ok(())
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

    pub fn shape_enable_hit_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_enable_hit_events_impl(shape, flag)
    }

    pub fn try_shape_enable_hit_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_enable_hit_events_impl(shape, flag);
        Ok(())
    }

    pub fn shape_hit_events_enabled(&self, shape: ShapeId) -> bool {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_hit_events_enabled_impl(shape)
    }

    pub fn try_shape_hit_events_enabled(&self, shape: ShapeId) -> crate::error::ApiResult<bool> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_hit_events_enabled_impl(shape))
    }

    // Sensor helpers (ID-style)
    /// Get the maximum capacity required to retrieve sensor overlaps for a shape id.
    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_capacity_impl(shape)
    }

    pub fn try_shape_sensor_capacity(&self, shape: ShapeId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_capacity_impl(shape))
    }

    /// Get overlapped shapes for a sensor shape id. Returns empty if not a sensor.
    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_impl(shape)
    }

    pub fn shape_sensor_overlaps_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_into_impl(shape, out);
    }

    pub fn try_shape_sensor_overlaps(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_overlaps_impl(shape))
    }

    pub fn try_shape_sensor_overlaps_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_sensor_overlaps_into_impl(shape, out);
        Ok(())
    }

    /// Get overlapped shapes for a sensor shape id, filtered to valid (non-destroyed) ids.
    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_valid_impl(shape)
    }

    pub fn try_shape_sensor_overlaps_valid(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_overlaps_valid_impl(shape))
    }

    pub fn shape_sensor_overlaps_valid_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_valid_into_impl(shape, out);
    }

    pub fn try_shape_sensor_overlaps_valid_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_sensor_overlaps_valid_into_impl(shape, out);
        Ok(())
    }
}
