use super::*;

impl Circle {
    #[inline]
    pub fn new<C: Into<Vec2>>(center: C, radius: f32) -> Self {
        Self {
            center: center.into(),
            radius,
        }
    }

    #[inline]
    /// Construct from the raw Box2D geometry value.
    pub fn from_raw(circle: ffi::b2Circle) -> Self {
        Self {
            center: Vec2::from_raw(circle.center),
            radius: circle.radius,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Circle {
        ffi::b2Circle {
            center: self.center.into_raw(),
            radius: self.radius,
        }
    }

    #[inline]
    /// Validate this circle for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        circle_helper_geometry_is_valid(self)
    }

    #[inline]
    /// Validate this circle for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        check_circle_helper_geometry_valid(self)
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        assert_circle_helper_geometry_valid(self);
        assert_non_negative_finite_density(density);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        MassData::from_raw(unsafe { ffi::b2ComputeCircleMass(&raw, density) })
    }

    #[inline]
    pub fn try_mass_data(self, density: f32) -> ApiResult<MassData> {
        check_circle_helper_geometry_valid(self)?;
        check_non_negative_finite_density(density)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(MassData::from_raw(unsafe {
            ffi::b2ComputeCircleMass(&raw, density)
        }))
    }

    /// Compute an absolute world-space AABB using `transform` as the circle's
    /// local-to-world transform.
    ///
    /// The result uses `f32` coordinates in both precision modes. Double-precision world bounds
    /// are narrowed outward by Box2D so the returned AABB remains conservative.
    #[inline]
    pub fn aabb(self, transform: WorldTransform) -> Aabb {
        assert_circle_helper_geometry_valid(self);
        assert_world_transform_valid(transform);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        Aabb::from_raw(unsafe { ffi::b2ComputeCircleAABB(&raw, transform.into_raw()) })
    }

    #[inline]
    pub fn try_aabb(self, transform: WorldTransform) -> ApiResult<Aabb> {
        check_circle_helper_geometry_valid(self)?;
        check_world_transform_valid(transform)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(Aabb::from_raw(unsafe {
            ffi::b2ComputeCircleAABB(&raw, transform.into_raw())
        }))
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let point = point.into();
        assert_circle_helper_geometry_valid(self);
        assert_valid_geometry_vec2("point", point);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        unsafe { ffi::b2PointInCircle(&raw, point.into_raw()) }
    }

    #[inline]
    pub fn try_contains_point<P: Into<Vec2>>(self, point: P) -> ApiResult<bool> {
        let point = point.into();
        check_circle_helper_geometry_valid(self)?;
        check_valid_geometry_vec2(point)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(unsafe { ffi::b2PointInCircle(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let input = materialize_ray_input(origin, translation);
        assert_circle_helper_geometry_valid(self);
        assert_ray_input_valid(&input);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        CastOutput::from_raw(unsafe { ffi::b2RayCastCircle(&raw, &input) })
    }

    #[inline]
    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        let input = materialize_ray_input(origin, translation);
        check_circle_helper_geometry_valid(self)?;
        check_ray_input_valid(&input)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2RayCastCircle(&raw, &input)
        }))
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> CastOutput {
        assert_circle_helper_geometry_valid(self);
        assert!(
            input.validate().is_ok(),
            "shape cast input contains invalid Box2D data"
        );
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = assert_transient_native_lease();
        CastOutput::from_raw(unsafe { ffi::b2ShapeCastCircle(&raw, &input) })
    }

    #[inline]
    pub fn try_shape_cast(self, input: ShapeCastInput) -> ApiResult<CastOutput> {
        check_circle_helper_geometry_valid(self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = transient_native_lease()?;
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2ShapeCastCircle(&raw, &input)
        }))
    }
}
