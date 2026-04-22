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
        self.center.is_valid() && geometry_scalar_is_non_negative_finite(self.radius)
    }

    #[inline]
    /// Validate this circle for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        geometry_is_valid_or_err(self.is_valid())
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        assert_circle_helper_geometry_valid(self);
        assert_non_negative_finite_density(density);
        let raw = self.into_raw();
        MassData::from_raw(unsafe { ffi::b2ComputeCircleMass(&raw, density) })
    }

    #[inline]
    pub fn try_mass_data(self, density: f32) -> ApiResult<MassData> {
        check_circle_helper_geometry_valid(self)?;
        check_non_negative_finite_density(density)?;
        let raw = self.into_raw();
        Ok(MassData::from_raw(unsafe {
            ffi::b2ComputeCircleMass(&raw, density)
        }))
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        assert_circle_helper_geometry_valid(self);
        assert_transform_valid(transform);
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputeCircleAABB(&raw, transform.into_raw()) })
    }

    #[inline]
    pub fn try_aabb(self, transform: Transform) -> ApiResult<Aabb> {
        check_circle_helper_geometry_valid(self)?;
        check_transform_valid(transform)?;
        let raw = self.into_raw();
        Ok(Aabb::from_raw(unsafe {
            ffi::b2ComputeCircleAABB(&raw, transform.into_raw())
        }))
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        assert_circle_helper_geometry_valid(self);
        let point = point.into();
        assert_valid_geometry_vec2("point", point);
        let raw = self.into_raw();
        unsafe { ffi::b2PointInCircle(&raw, point.into_raw()) }
    }

    #[inline]
    pub fn try_contains_point<P: Into<Vec2>>(self, point: P) -> ApiResult<bool> {
        check_circle_helper_geometry_valid(self)?;
        let point = point.into();
        check_valid_geometry_vec2(point)?;
        let raw = self.into_raw();
        Ok(unsafe { ffi::b2PointInCircle(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        assert_circle_helper_geometry_valid(self);
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastCircle(&raw, &input) })
    }

    #[inline]
    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        check_circle_helper_geometry_valid(self)?;
        let raw = self.into_raw();
        let input = try_make_ray_input(origin, translation)?;
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2RayCastCircle(&raw, &input)
        }))
    }
}
