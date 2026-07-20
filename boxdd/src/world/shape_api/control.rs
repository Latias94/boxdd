use super::*;

fn world_shape_set_circle_impl(core: &WorldCore, shape: ShapeId, circle: &crate::shapes::Circle) {
    assert_shape_target(core, shape);
    crate::shapes::assert_circle_geometry_valid(circle);
    let raw = circle.into_raw();
    unsafe { ffi::b2Shape_SetCircle(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_circle_impl(
    core: &WorldCore,
    shape: ShapeId,
    circle: &crate::shapes::Circle,
) -> crate::error::ApiResult<()> {
    check_shape_target(core, shape)?;
    crate::shapes::check_circle_geometry_valid(circle)?;
    let raw = circle.into_raw();
    unsafe { ffi::b2Shape_SetCircle(raw_shape_id(shape), &raw) }
    Ok(())
}

fn world_shape_set_segment_impl(
    core: &WorldCore,
    shape: ShapeId,
    segment: &crate::shapes::Segment,
) {
    assert_shape_target(core, shape);
    crate::shapes::assert_segment_geometry_valid(segment);
    let raw = segment.into_raw();
    unsafe { ffi::b2Shape_SetSegment(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_segment_impl(
    core: &WorldCore,
    shape: ShapeId,
    segment: &crate::shapes::Segment,
) -> crate::error::ApiResult<()> {
    check_shape_target(core, shape)?;
    crate::shapes::check_segment_geometry_valid(segment)?;
    let raw = segment.into_raw();
    unsafe { ffi::b2Shape_SetSegment(raw_shape_id(shape), &raw) }
    Ok(())
}

fn world_shape_set_capsule_impl(
    core: &WorldCore,
    shape: ShapeId,
    capsule: &crate::shapes::Capsule,
) {
    assert_shape_target(core, shape);
    crate::shapes::assert_capsule_geometry_valid(capsule);
    let raw = capsule.into_raw();
    unsafe { ffi::b2Shape_SetCapsule(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_capsule_impl(
    core: &WorldCore,
    shape: ShapeId,
    capsule: &crate::shapes::Capsule,
) -> crate::error::ApiResult<()> {
    check_shape_target(core, shape)?;
    crate::shapes::check_capsule_geometry_valid(capsule)?;
    let raw = capsule.into_raw();
    unsafe { ffi::b2Shape_SetCapsule(raw_shape_id(shape), &raw) }
    Ok(())
}

fn world_shape_set_polygon_impl(
    core: &WorldCore,
    shape: ShapeId,
    polygon: &crate::shapes::Polygon,
) {
    assert_shape_target(core, shape);
    crate::shapes::assert_polygon_geometry_valid(polygon);
    let raw = polygon.into_raw();
    unsafe { ffi::b2Shape_SetPolygon(raw_shape_id(shape), &raw) }
}

fn try_world_shape_set_polygon_impl(
    core: &WorldCore,
    shape: ShapeId,
    polygon: &crate::shapes::Polygon,
) -> crate::error::ApiResult<()> {
    check_shape_target(core, shape)?;
    crate::shapes::check_polygon_geometry_valid(polygon)?;
    let raw = polygon.into_raw();
    unsafe { ffi::b2Shape_SetPolygon(raw_shape_id(shape), &raw) }
    Ok(())
}

impl World {
    pub fn shape_set_circle(&mut self, shape: ShapeId, circle: &crate::shapes::Circle) {
        world_shape_set_circle_impl(&self.core, shape, circle)
    }

    pub fn try_shape_set_circle(
        &mut self,
        shape: ShapeId,
        circle: &crate::shapes::Circle,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_circle_impl(&self.core, shape, circle)
    }

    pub fn shape_set_segment(&mut self, shape: ShapeId, segment: &crate::shapes::Segment) {
        world_shape_set_segment_impl(&self.core, shape, segment)
    }

    pub fn try_shape_set_segment(
        &mut self,
        shape: ShapeId,
        segment: &crate::shapes::Segment,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_segment_impl(&self.core, shape, segment)
    }

    pub fn shape_set_capsule(&mut self, shape: ShapeId, capsule: &crate::shapes::Capsule) {
        world_shape_set_capsule_impl(&self.core, shape, capsule)
    }

    pub fn try_shape_set_capsule(
        &mut self,
        shape: ShapeId,
        capsule: &crate::shapes::Capsule,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_capsule_impl(&self.core, shape, capsule)
    }

    pub fn shape_set_polygon(&mut self, shape: ShapeId, polygon: &crate::shapes::Polygon) {
        world_shape_set_polygon_impl(&self.core, shape, polygon)
    }

    pub fn try_shape_set_polygon(
        &mut self,
        shape: ShapeId,
        polygon: &crate::shapes::Polygon,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_polygon_impl(&self.core, shape, polygon)
    }

    pub fn shape_set_surface_material(&mut self, shape: ShapeId, material: &SurfaceMaterial) {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_set_surface_material_impl(shape, material)
    }

    pub fn try_shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &SurfaceMaterial,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
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
        assert_shape_target(&self.core, shape);
        let wind = wind.into();
        crate::shapes::assert_shape_vec2_valid("wind", wind);
        assert_shape_target(&self.core, shape);
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
        check_shape_target(&self.core, shape)?;
        let wind = wind.into();
        crate::shapes::check_shape_vec2_valid(wind)?;
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_apply_wind_impl(shape, wind, drag, lift, wake);
        Ok(())
    }

    pub fn shape_enable_sensor_events(&mut self, shape: ShapeId, flag: bool) {
        assert_shape_target(&self.core, shape);
        unsafe { ffi::b2Shape_EnableSensorEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_sensor_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        unsafe { ffi::b2Shape_EnableSensorEvents(raw_shape_id(shape), flag) }
        Ok(())
    }

    pub fn shape_enable_contact_events(&mut self, shape: ShapeId, flag: bool) {
        assert_shape_target(&self.core, shape);
        unsafe { ffi::b2Shape_EnableContactEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_contact_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        unsafe { ffi::b2Shape_EnableContactEvents(raw_shape_id(shape), flag) }
        Ok(())
    }

    pub fn shape_enable_pre_solve_events(&mut self, shape: ShapeId, flag: bool) {
        assert_shape_target(&self.core, shape);
        unsafe { ffi::b2Shape_EnablePreSolveEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_pre_solve_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        unsafe { ffi::b2Shape_EnablePreSolveEvents(raw_shape_id(shape), flag) }
        Ok(())
    }

    pub fn shape_enable_hit_events(&mut self, shape: ShapeId, flag: bool) {
        assert_shape_target(&self.core, shape);
        unsafe { ffi::b2Shape_EnableHitEvents(raw_shape_id(shape), flag) }
    }

    pub fn try_shape_enable_hit_events(
        &mut self,
        shape: ShapeId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        unsafe { ffi::b2Shape_EnableHitEvents(raw_shape_id(shape), flag) }
        Ok(())
    }
}
