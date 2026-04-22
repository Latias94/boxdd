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

    pub fn shape_enable_sensor_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_EnableSensorEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_sensor_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_EnableSensorEvents(raw_shape_id(shape), flag) }
        Ok(())
    }

    pub fn shape_enable_contact_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_EnableContactEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_contact_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_EnableContactEvents(raw_shape_id(shape), flag) }
        Ok(())
    }

    pub fn shape_enable_pre_solve_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_EnablePreSolveEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_pre_solve_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_EnablePreSolveEvents(raw_shape_id(shape), flag) }
        Ok(())
    }

    pub fn shape_enable_hit_events(&mut self, shape: ShapeId, flag: bool) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_EnableHitEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_hit_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_EnableHitEvents(raw_shape_id(shape), flag) }
        Ok(())
    }
}
