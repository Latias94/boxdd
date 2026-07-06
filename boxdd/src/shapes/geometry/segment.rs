use super::*;

impl Segment {
    #[inline]
    pub fn new<P1: Into<Vec2>, P2: Into<Vec2>>(point1: P1, point2: P2) -> Self {
        Self {
            point1: point1.into(),
            point2: point2.into(),
        }
    }

    #[inline]
    /// Construct from the raw Box2D geometry value.
    pub fn from_raw(segment: ffi::b2Segment) -> Self {
        Self {
            point1: Vec2::from_raw(segment.point1),
            point2: Vec2::from_raw(segment.point2),
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Segment {
        ffi::b2Segment {
            point1: self.point1.into_raw(),
            point2: self.point2.into_raw(),
        }
    }

    #[inline]
    /// Validate this segment for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        self.point1.is_valid()
            && self.point2.is_valid()
            && point_pair_has_minimum_separation(self.point1, self.point2)
    }

    #[inline]
    /// Validate this segment for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        geometry_is_valid_or_err(self.is_valid())
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        assert_segment_helper_geometry_valid(self);
        assert_transform_valid(transform);
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputeSegmentAABB(&raw, transform.into_raw()) })
    }

    #[inline]
    pub fn try_aabb(self, transform: Transform) -> ApiResult<Aabb> {
        check_segment_helper_geometry_valid(self)?;
        check_transform_valid(transform)?;
        let raw = self.into_raw();
        Ok(Aabb::from_raw(unsafe {
            ffi::b2ComputeSegmentAABB(&raw, transform.into_raw())
        }))
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
        one_sided: bool,
    ) -> CastOutput {
        assert_segment_helper_geometry_valid(self);
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastSegment(&raw, &input, one_sided) })
    }

    #[inline]
    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
        one_sided: bool,
    ) -> ApiResult<CastOutput> {
        check_segment_helper_geometry_valid(self)?;
        let raw = self.into_raw();
        let input = try_make_ray_input(origin, translation)?;
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2RayCastSegment(&raw, &input, one_sided)
        }))
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> CastOutput {
        assert_segment_helper_geometry_valid(self);
        assert!(
            input.validate().is_ok(),
            "shape cast input contains invalid Box2D data"
        );
        let raw = self.into_raw();
        let input = input.into_raw();
        CastOutput::from_raw(unsafe { ffi::b2ShapeCastSegment(&raw, &input) })
    }

    #[inline]
    pub fn try_shape_cast(self, input: ShapeCastInput) -> ApiResult<CastOutput> {
        check_segment_helper_geometry_valid(self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2ShapeCastSegment(&raw, &input)
        }))
    }
}
