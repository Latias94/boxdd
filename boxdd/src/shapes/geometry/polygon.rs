use super::*;

impl Polygon {
    #[inline]
    /// Construct from the raw Box2D geometry value.
    pub fn from_raw(raw: ffi::b2Polygon) -> Self {
        Self { raw }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Polygon {
        self.raw
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.raw.count.clamp(0, MAX_POLYGON_VERTICES as i32) as usize
    }

    #[inline]
    pub fn vertices(&self) -> &[Vec2] {
        unsafe {
            core::slice::from_raw_parts(self.raw.vertices.as_ptr().cast::<Vec2>(), self.count())
        }
    }

    #[inline]
    pub fn normals(&self) -> &[Vec2] {
        unsafe {
            core::slice::from_raw_parts(self.raw.normals.as_ptr().cast::<Vec2>(), self.count())
        }
    }

    #[inline]
    pub fn centroid(&self) -> Vec2 {
        Vec2::from_raw(self.raw.centroid)
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.raw.radius
    }

    #[inline]
    /// Validate this polygon for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        if !(1..=MAX_POLYGON_VERTICES as i32).contains(&self.raw.count) {
            return false;
        }
        if !Vec2::from_raw(self.raw.centroid).is_valid()
            || !geometry_scalar_is_non_negative_finite(self.raw.radius)
        {
            return false;
        }
        self.vertices().iter().copied().all(Vec2::is_valid)
            && self.normals().iter().copied().all(Vec2::is_valid)
    }

    #[inline]
    /// Validate this polygon for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        geometry_is_valid_or_err(self.is_valid())
    }

    #[inline]
    pub fn square_polygon(half_width: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        Self::from_raw(unsafe { ffi::b2MakeSquare(half_width) })
    }

    #[inline]
    pub fn try_square_polygon(half_width: f32) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        Ok(Self::from_raw(unsafe { ffi::b2MakeSquare(half_width) }))
    }

    #[inline]
    pub fn box_polygon(half_width: f32, half_height: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        Self::from_raw(unsafe { ffi::b2MakeBox(half_width, half_height) })
    }

    #[inline]
    pub fn try_box_polygon(half_width: f32, half_height: f32) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        check_positive_finite_polygon_scalar(half_height)?;
        Ok(Self::from_raw(unsafe {
            ffi::b2MakeBox(half_width, half_height)
        }))
    }

    #[inline]
    pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        assert_non_negative_finite_polygon_scalar("radius", radius);
        Self::from_raw(unsafe { ffi::b2MakeRoundedBox(half_width, half_height, radius) })
    }

    #[inline]
    pub fn try_rounded_box_polygon(
        half_width: f32,
        half_height: f32,
        radius: f32,
    ) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        check_positive_finite_polygon_scalar(half_height)?;
        check_non_negative_finite_polygon_scalar(radius)?;
        Ok(Self::from_raw(unsafe {
            ffi::b2MakeRoundedBox(half_width, half_height, radius)
        }))
    }

    #[inline]
    pub fn offset_box_polygon(half_width: f32, half_height: f32, transform: Transform) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        assert_transform_valid(transform);
        Self::from_raw(unsafe {
            ffi::b2MakeOffsetBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
            )
        })
    }

    #[inline]
    pub fn try_offset_box_polygon(
        half_width: f32,
        half_height: f32,
        transform: Transform,
    ) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        check_positive_finite_polygon_scalar(half_height)?;
        check_transform_valid(transform)?;
        Ok(Self::from_raw(unsafe {
            ffi::b2MakeOffsetBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
            )
        }))
    }

    #[inline]
    pub fn offset_rounded_box_polygon(
        half_width: f32,
        half_height: f32,
        radius: f32,
        transform: Transform,
    ) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        assert_non_negative_finite_polygon_scalar("radius", radius);
        assert_transform_valid(transform);
        Self::from_raw(unsafe {
            ffi::b2MakeOffsetRoundedBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
                radius,
            )
        })
    }

    #[inline]
    pub fn try_offset_rounded_box_polygon(
        half_width: f32,
        half_height: f32,
        radius: f32,
        transform: Transform,
    ) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        check_positive_finite_polygon_scalar(half_height)?;
        check_non_negative_finite_polygon_scalar(radius)?;
        check_transform_valid(transform)?;
        Ok(Self::from_raw(unsafe {
            ffi::b2MakeOffsetRoundedBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
                radius,
            )
        }))
    }

    #[inline]
    pub fn from_points<I, P>(points: I, radius: f32) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        Self::try_from_points(points, radius).ok()
    }

    #[inline]
    pub fn try_from_points<I, P>(points: I, radius: f32) -> ApiResult<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        check_non_negative_finite_polygon_scalar(radius)?;
        let hull = try_compute_hull_from_points(points)?;
        Ok(Self::from_raw(unsafe { ffi::b2MakePolygon(&hull, radius) }))
    }

    #[inline]
    pub fn offset_from_points<I, P>(points: I, radius: f32, transform: Transform) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        Self::try_offset_from_points(points, radius, transform).ok()
    }

    #[inline]
    pub fn try_offset_from_points<I, P>(
        points: I,
        radius: f32,
        transform: Transform,
    ) -> ApiResult<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        check_non_negative_finite_polygon_scalar(radius)?;
        check_transform_valid(transform)?;
        let hull = try_compute_hull_from_points(points)?;
        Ok(Self::from_raw(unsafe {
            if radius == 0.0 {
                ffi::b2MakeOffsetPolygon(
                    &hull,
                    transform.position().into_raw(),
                    transform.rotation().into_raw(),
                )
            } else {
                ffi::b2MakeOffsetRoundedPolygon(
                    &hull,
                    transform.position().into_raw(),
                    transform.rotation().into_raw(),
                    radius,
                )
            }
        }))
    }

    #[inline]
    pub fn hull_is_valid<I, P>(points: I) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        compute_hull_from_points(points).is_some_and(|hull| unsafe { ffi::b2ValidateHull(&hull) })
    }

    #[inline]
    pub fn transformed(self, transform: Transform) -> Self {
        assert_polygon_helper_geometry_valid(self);
        assert_transform_valid(transform);
        Self::from_raw(unsafe { ffi::b2TransformPolygon(transform.into_raw(), &self.raw) })
    }

    #[inline]
    pub fn try_transformed(self, transform: Transform) -> ApiResult<Self> {
        check_polygon_helper_geometry_valid(self)?;
        check_transform_valid(transform)?;
        Ok(Self::from_raw(unsafe {
            ffi::b2TransformPolygon(transform.into_raw(), &self.raw)
        }))
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        assert_polygon_helper_geometry_valid(self);
        assert_non_negative_finite_density(density);
        let raw = self.into_raw();
        MassData::from_raw(unsafe { ffi::b2ComputePolygonMass(&raw, density) })
    }

    #[inline]
    pub fn try_mass_data(self, density: f32) -> ApiResult<MassData> {
        check_polygon_helper_geometry_valid(self)?;
        check_non_negative_finite_density(density)?;
        let raw = self.into_raw();
        Ok(MassData::from_raw(unsafe {
            ffi::b2ComputePolygonMass(&raw, density)
        }))
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        assert_polygon_helper_geometry_valid(self);
        assert_transform_valid(transform);
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputePolygonAABB(&raw, transform.into_raw()) })
    }

    #[inline]
    pub fn try_aabb(self, transform: Transform) -> ApiResult<Aabb> {
        check_polygon_helper_geometry_valid(self)?;
        check_transform_valid(transform)?;
        let raw = self.into_raw();
        Ok(Aabb::from_raw(unsafe {
            ffi::b2ComputePolygonAABB(&raw, transform.into_raw())
        }))
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        assert_polygon_helper_geometry_valid(self);
        let point = point.into();
        assert_valid_geometry_vec2("point", point);
        let raw = self.into_raw();
        unsafe { ffi::b2PointInPolygon(&raw, point.into_raw()) }
    }

    #[inline]
    pub fn try_contains_point<P: Into<Vec2>>(self, point: P) -> ApiResult<bool> {
        check_polygon_helper_geometry_valid(self)?;
        let point = point.into();
        check_valid_geometry_vec2(point)?;
        let raw = self.into_raw();
        Ok(unsafe { ffi::b2PointInPolygon(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        assert_polygon_helper_geometry_valid(self);
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastPolygon(&raw, &input) })
    }

    #[inline]
    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        check_polygon_helper_geometry_valid(self)?;
        let raw = self.into_raw();
        let input = try_make_ray_input(origin, translation)?;
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2RayCastPolygon(&raw, &input)
        }))
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> CastOutput {
        assert_polygon_helper_geometry_valid(self);
        assert!(
            input.validate().is_ok(),
            "shape cast input contains invalid Box2D data"
        );
        let raw = self.into_raw();
        let input = input.into_raw();
        CastOutput::from_raw(unsafe { ffi::b2ShapeCastPolygon(&raw, &input) })
    }

    #[inline]
    pub fn try_shape_cast(self, input: ShapeCastInput) -> ApiResult<CastOutput> {
        check_polygon_helper_geometry_valid(self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        Ok(CastOutput::from_raw(unsafe {
            ffi::b2ShapeCastPolygon(&raw, &input)
        }))
    }
}

impl fmt::Debug for Polygon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Polygon")
            .field("vertices", &self.vertices())
            .field("normals", &self.normals())
            .field("centroid", &self.centroid())
            .field("radius", &self.radius())
            .finish()
    }
}
