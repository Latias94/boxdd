use super::*;

fn world_shape_set_circle_impl(core: &WorldCore, shape: ShapeId, circle: &crate::shapes::Circle) {
    crate::shapes::assert_circle_geometry_valid(circle);
    crate::shapes::assert_orphan_shape_mutation_target(core, shape);
    crate::shapes::shape_set_circle_impl(shape, circle)
}

fn try_world_shape_set_circle_impl(
    core: &WorldCore,
    shape: ShapeId,
    circle: &crate::shapes::Circle,
) -> crate::error::ApiResult<()> {
    try_world_shape_set_circle_with_access(
        core,
        shape,
        circle,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_world_shape_set_circle_with_access(
    core: &WorldCore,
    shape: ShapeId,
    circle: &crate::shapes::Circle,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::shapes::check_circle_geometry_valid(circle)?;
    crate::shapes::try_check_orphan_shape_mutation_target_with_access(core, shape, access)?;
    crate::shapes::shape_set_circle_impl(shape, circle);
    Ok(())
}

fn world_shape_set_segment_impl(
    core: &WorldCore,
    shape: ShapeId,
    segment: &crate::shapes::Segment,
) {
    crate::shapes::assert_segment_geometry_valid(segment);
    crate::shapes::assert_orphan_shape_mutation_target(core, shape);
    crate::shapes::shape_set_segment_impl(shape, segment)
}

fn try_world_shape_set_segment_impl(
    core: &WorldCore,
    shape: ShapeId,
    segment: &crate::shapes::Segment,
) -> crate::error::ApiResult<()> {
    try_world_shape_set_segment_with_access(
        core,
        shape,
        segment,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_world_shape_set_segment_with_access(
    core: &WorldCore,
    shape: ShapeId,
    segment: &crate::shapes::Segment,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::shapes::check_segment_geometry_valid(segment)?;
    crate::shapes::try_check_orphan_shape_mutation_target_with_access(core, shape, access)?;
    crate::shapes::shape_set_segment_impl(shape, segment);
    Ok(())
}

fn world_shape_set_capsule_impl(
    core: &WorldCore,
    shape: ShapeId,
    capsule: &crate::shapes::Capsule,
) {
    crate::shapes::assert_capsule_geometry_valid(capsule);
    crate::shapes::assert_orphan_shape_mutation_target(core, shape);
    crate::shapes::shape_set_capsule_impl(shape, capsule)
}

fn try_world_shape_set_capsule_impl(
    core: &WorldCore,
    shape: ShapeId,
    capsule: &crate::shapes::Capsule,
) -> crate::error::ApiResult<()> {
    try_world_shape_set_capsule_with_access(
        core,
        shape,
        capsule,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_world_shape_set_capsule_with_access(
    core: &WorldCore,
    shape: ShapeId,
    capsule: &crate::shapes::Capsule,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::shapes::check_capsule_geometry_valid(capsule)?;
    crate::shapes::try_check_orphan_shape_mutation_target_with_access(core, shape, access)?;
    crate::shapes::shape_set_capsule_impl(shape, capsule);
    Ok(())
}

fn world_shape_set_polygon_impl(
    core: &WorldCore,
    shape: ShapeId,
    polygon: &crate::shapes::Polygon,
) {
    crate::shapes::assert_polygon_geometry_valid(polygon);
    crate::shapes::assert_orphan_shape_mutation_target(core, shape);
    crate::shapes::shape_set_polygon_impl(shape, polygon)
}

fn try_world_shape_set_polygon_impl(
    core: &WorldCore,
    shape: ShapeId,
    polygon: &crate::shapes::Polygon,
) -> crate::error::ApiResult<()> {
    try_world_shape_set_polygon_with_access(
        core,
        shape,
        polygon,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_world_shape_set_polygon_with_access(
    core: &WorldCore,
    shape: ShapeId,
    polygon: &crate::shapes::Polygon,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::shapes::check_polygon_geometry_valid(polygon)?;
    crate::shapes::try_check_orphan_shape_mutation_target_with_access(core, shape, access)?;
    crate::shapes::shape_set_polygon_impl(shape, polygon);
    Ok(())
}

pub(crate) fn try_world_shape_set_surface_material_with_access(
    core: &WorldCore,
    shape: ShapeId,
    material: &SurfaceMaterial,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::shapes::try_shape_set_surface_material_with_access(core, shape, material, access)
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

    /// Change a shape into an orphan chain segment, or update an existing orphan segment.
    pub fn shape_set_chain_segment(
        &mut self,
        shape: ShapeId,
        chain_segment: &crate::shapes::ChainSegment,
    ) {
        crate::shapes::shape_set_chain_segment_checked_impl(&self.core, shape, chain_segment)
    }

    pub fn try_shape_set_chain_segment(
        &mut self,
        shape: ShapeId,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> crate::error::ApiResult<()> {
        crate::shapes::try_shape_set_chain_segment_checked_impl(&self.core, shape, chain_segment)
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
        crate::shapes::assert_surface_material_valid(material);
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_set_surface_material_impl(shape, material)
    }

    pub fn try_shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &SurfaceMaterial,
    ) -> crate::error::ApiResult<()> {
        try_world_shape_set_surface_material_with_access(
            &self.core,
            shape,
            material,
            crate::core::world_core::WorldAccess::Idle,
        )
    }

    /// Apply wind force/torque approximation to a shape.
    ///
    /// `wind` must be finite and `drag` must be finite and non-negative. `lift` must be finite;
    /// negative values reverse the perpendicular lift direction.
    pub fn shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) {
        crate::shapes::try_shape_apply_wind_with_access(
            &self.core,
            shape,
            wind,
            drag,
            lift,
            wake,
            crate::core::world_core::WorldAccess::Idle,
        )
        .expect("world received invalid shape wind parameters")
    }

    /// Fallible form of [`Self::shape_apply_wind`].
    ///
    /// Returns `ApiError::InvalidArgument` when a numeric parameter violates its constraints.
    pub fn try_shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::shapes::try_shape_apply_wind_with_access(
            &self.core,
            shape,
            wind,
            drag,
            lift,
            wake,
            crate::core::world_core::WorldAccess::Idle,
        )
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
