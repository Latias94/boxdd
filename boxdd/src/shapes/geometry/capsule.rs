use super::*;

impl Capsule {
    #[inline]
    pub fn new<C1: Into<Vec2>, C2: Into<Vec2>>(center1: C1, center2: C2, radius: f32) -> Self {
        Self {
            center1: center1.into(),
            center2: center2.into(),
            radius,
        }
    }

    #[inline]
    /// Construct from the raw Box2D geometry value.
    pub fn from_raw(capsule: ffi::b2Capsule) -> Self {
        Self {
            center1: Vec2::from_raw(capsule.center1),
            center2: Vec2::from_raw(capsule.center2),
            radius: capsule.radius,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Capsule {
        ffi::b2Capsule {
            center1: self.center1.into_raw(),
            center2: self.center2.into_raw(),
            radius: self.radius,
        }
    }

    #[inline]
    /// Validate this capsule for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        self.center1.is_valid()
            && self.center2.is_valid()
            && geometry_scalar_is_non_negative_finite(self.radius)
            && point_pair_has_minimum_separation(self.center1, self.center2)
    }

    #[inline]
    /// Validate this capsule for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        geometry_is_valid_or_err(self.is_valid())
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        assert_capsule_helper_geometry_valid(self);
        assert_non_negative_finite_density(density);
        let raw = self.into_raw();
        MassData::from_raw(unsafe { ffi::b2ComputeCapsuleMass(&raw, density) })
    }

    #[inline]
    pub fn try_mass_data(self, density: f32) -> ApiResult<MassData> {
        check_capsule_helper_geometry_valid(self)?;
        check_non_negative_finite_density(density)?;
        let raw = self.into_raw();
        Ok(MassData::from_raw(unsafe {
            ffi::b2ComputeCapsuleMass(&raw, density)
        }))
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        assert_capsule_helper_geometry_valid(self);
        assert_transform_valid(transform);
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputeCapsuleAABB(&raw, transform.into_raw()) })
    }

    #[inline]
    pub fn try_aabb(self, transform: Transform) -> ApiResult<Aabb> {
        check_capsule_helper_geometry_valid(self)?;
        check_transform_valid(transform)?;
        let raw = self.into_raw();
        Ok(Aabb::from_raw(unsafe {
            ffi::b2ComputeCapsuleAABB(&raw, transform.into_raw())
        }))
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        assert_capsule_helper_geometry_valid(self);
        let point = point.into();
        assert_valid_geometry_vec2("point", point);
        let raw = self.into_raw();
        unsafe { ffi::b2PointInCapsule(&raw, point.into_raw()) }
    }

    #[inline]
    pub fn try_contains_point<P: Into<Vec2>>(self, point: P) -> ApiResult<bool> {
        check_capsule_helper_geometry_valid(self)?;
        let point = point.into();
        check_valid_geometry_vec2(point)?;
        let raw = self.into_raw();
        Ok(unsafe { ffi::b2PointInCapsule(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        assert_capsule_helper_geometry_valid(self);
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastCapsule(&raw, &input) })
    }

    #[inline]
    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        check_capsule_helper_geometry_valid(self)?;
        let raw = self.into_raw();
        let input = try_make_ray_input(origin, translation)?;
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2RayCastCapsule(&raw, &input)
        }))
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> CastOutput {
        assert_capsule_helper_geometry_valid(self);
        assert!(
            input.validate().is_ok(),
            "shape cast input contains invalid Box2D data"
        );
        let raw = self.into_raw();
        let input = input.into_raw();
        CastOutput::from_raw(unsafe { ffi::b2ShapeCastCapsule(&raw, &input) })
    }

    #[inline]
    pub fn try_shape_cast(self, input: ShapeCastInput) -> ApiResult<CastOutput> {
        check_capsule_helper_geometry_valid(self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2ShapeCastCapsule(&raw, &input)
        }))
    }
}
