use super::*;

impl Polygon {
    #[inline]
    /// Construct from the raw Box2D geometry value.
    ///
    /// # Safety
    /// `raw` must describe a finite, strictly convex, counter-clockwise polygon with three to
    /// [`MAX_POLYGON_VERTICES`] vertices. Every normal must be the corresponding unit outward edge
    /// normal, `centroid` must match the vertices, and `radius` must be finite and non-negative.
    /// Violating these invariants can trigger native Box2D assertions in otherwise safe methods.
    pub unsafe fn from_raw(raw: ffi::b2Polygon) -> Self {
        Self { raw }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Polygon {
        self.raw
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.raw
            .count
            .clamp(0, ::boxdd_sys::ffi::B2_MAX_POLYGON_VERTICES as i32) as usize
    }

    #[inline]
    pub fn vertices(&self) -> &[Vec2] {
        unsafe {
            ::std::slice::from_raw_parts(self.raw.vertices.as_ptr().cast::<Vec2>(), self.count())
        }
    }

    #[inline]
    pub fn normals(&self) -> &[Vec2] {
        unsafe {
            ::std::slice::from_raw_parts(self.raw.normals.as_ptr().cast::<Vec2>(), self.count())
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
        polygon_helper_geometry_is_valid(self)
    }

    #[inline]
    /// Validate this polygon for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        check_polygon_helper_geometry_valid(self)
    }

    #[inline]
    pub fn square_polygon(half_width: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        let _lease = assert_transient_native_lease();
        // SAFETY: Box2D constructs a complete polygon from the validated half width.
        unsafe { Self::from_raw(ffi::b2MakeSquare(half_width)) }
    }

    #[inline]
    pub fn try_square_polygon(half_width: f32) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        let _lease = transient_native_lease()?;
        // SAFETY: Box2D constructs a complete polygon from the validated half width.
        Ok(unsafe { Self::from_raw(ffi::b2MakeSquare(half_width)) })
    }

    #[inline]
    pub fn box_polygon(half_width: f32, half_height: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        let _lease = assert_transient_native_lease();
        // SAFETY: Box2D constructs a complete polygon from the validated half extents.
        unsafe { Self::from_raw(ffi::b2MakeBox(half_width, half_height)) }
    }

    #[inline]
    pub fn try_box_polygon(half_width: f32, half_height: f32) -> ApiResult<Self> {
        check_positive_finite_polygon_scalar(half_width)?;
        check_positive_finite_polygon_scalar(half_height)?;
        let _lease = transient_native_lease()?;
        // SAFETY: Box2D constructs a complete polygon from the validated half extents.
        Ok(unsafe { Self::from_raw(ffi::b2MakeBox(half_width, half_height)) })
    }

    #[inline]
    pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        assert_non_negative_finite_polygon_scalar("radius", radius);
        let _lease = assert_transient_native_lease();
        // SAFETY: Box2D constructs a complete polygon from the validated dimensions and radius.
        unsafe { Self::from_raw(ffi::b2MakeRoundedBox(half_width, half_height, radius)) }
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
        let _lease = transient_native_lease()?;
        // SAFETY: Box2D constructs a complete polygon from the validated dimensions and radius.
        Ok(unsafe { Self::from_raw(ffi::b2MakeRoundedBox(half_width, half_height, radius)) })
    }

    #[inline]
    pub fn offset_box_polygon(half_width: f32, half_height: f32, transform: Transform) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        assert_transform_valid(transform);
        let _lease = assert_transient_native_lease();
        // SAFETY: Box2D constructs a complete polygon from validated extents and transform.
        unsafe {
            Self::from_raw(ffi::b2MakeOffsetBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
            ))
        }
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
        let _lease = transient_native_lease()?;
        // SAFETY: Box2D constructs a complete polygon from validated extents and transform.
        Ok(unsafe {
            Self::from_raw(ffi::b2MakeOffsetBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
            ))
        })
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
        let _lease = assert_transient_native_lease();
        // SAFETY: Box2D constructs a complete polygon from validated dimensions and transform.
        unsafe {
            Self::from_raw(ffi::b2MakeOffsetRoundedBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
                radius,
            ))
        }
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
        let _lease = transient_native_lease()?;
        // SAFETY: Box2D constructs a complete polygon from validated dimensions and transform.
        Ok(unsafe {
            Self::from_raw(ffi::b2MakeOffsetRoundedBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
                radius,
            ))
        })
    }

    #[inline]
    pub fn from_points<I, P>(points: I, radius: f32) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let points = collect_polygon_points(points)?;
        check_non_negative_finite_polygon_scalar(radius).ok()?;
        polygon_points_are_valid(&points).then_some(())?;
        let _lease = assert_transient_native_lease();
        let hull = compute_hull_from_points(&points, &_lease)?;
        // SAFETY: b2ComputeHull returned a validated native hull and radius was checked above.
        Some(unsafe { Self::from_raw(ffi::b2MakePolygon(&hull, radius)) })
    }

    #[inline]
    pub fn try_from_points<I, P>(points: I, radius: f32) -> ApiResult<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let points = collect_polygon_points(points).ok_or(ApiError::InvalidArgument)?;
        check_non_negative_finite_polygon_scalar(radius)?;
        geometry_is_valid_or_err(polygon_points_are_valid(&points))?;
        let _lease = transient_native_lease()?;
        let hull = try_compute_hull_from_points(&points, &_lease)?;
        // SAFETY: b2ComputeHull returned a validated native hull and radius was checked above.
        Ok(unsafe { Self::from_raw(ffi::b2MakePolygon(&hull, radius)) })
    }

    #[inline]
    pub fn offset_from_points<I, P>(points: I, radius: f32, transform: Transform) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let points = collect_polygon_points(points)?;
        check_non_negative_finite_polygon_scalar(radius).ok()?;
        check_transform_valid(transform).ok()?;
        polygon_points_are_valid(&points).then_some(())?;
        let _lease = assert_transient_native_lease();
        let hull = compute_hull_from_points(&points, &_lease)?;
        // SAFETY: Box2D constructs the polygon from its own hull and a validated transform/radius.
        Some(unsafe {
            Self::from_raw(if radius == 0.0 {
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
            })
        })
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
        let points = collect_polygon_points(points).ok_or(ApiError::InvalidArgument)?;
        check_non_negative_finite_polygon_scalar(radius)?;
        check_transform_valid(transform)?;
        geometry_is_valid_or_err(polygon_points_are_valid(&points))?;
        let _lease = transient_native_lease()?;
        let hull = try_compute_hull_from_points(&points, &_lease)?;
        // SAFETY: Box2D constructs the polygon from its own hull and a validated transform/radius.
        Ok(unsafe {
            Self::from_raw(if radius == 0.0 {
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
            })
        })
    }

    #[inline]
    /// Return whether Box2D can compute and validate a convex hull from `points`.
    ///
    /// This performs native Box2D work and therefore panics when called from a Box2D callback or
    /// while replay owns exclusive foundation access. Use [`Self::try_hull_is_valid`] when those
    /// activity errors must be recoverable.
    pub fn hull_is_valid<I, P>(points: I) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let Some(points) = collect_polygon_points(points) else {
            return false;
        };
        if !polygon_points_are_valid(&points) {
            return false;
        }
        let _lease = assert_transient_native_lease();
        let Some(hull) = compute_hull_from_points(&points, &_lease) else {
            return false;
        };
        unsafe { ffi::b2ValidateHull(&hull) }
    }

    /// Recoverable native check for whether `points` produce a valid Box2D convex hull.
    ///
    /// Point conversion and finite-value validation complete before foundation activity is leased.
    /// Degenerate point sets return `Ok(false)`; malformed input or unavailable foundation activity
    /// returns an error.
    #[inline]
    pub fn try_hull_is_valid<I, P>(points: I) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let points = collect_polygon_points(points).ok_or(ApiError::InvalidArgument)?;
        geometry_is_valid_or_err(polygon_points_are_valid(&points))?;
        let lease = transient_native_lease()?;
        let Some(hull) = compute_hull_from_points(&points, &lease) else {
            return Ok(false);
        };
        Ok(unsafe { ffi::b2ValidateHull(&hull) })
    }

    #[inline]
    pub fn transformed(self, transform: Transform) -> Self {
        assert_polygon_helper_geometry_valid(self);
        assert_transform_valid(transform);
        let _lease = assert_transient_native_lease();
        // SAFETY: the source polygon invariant and transform were validated before the native call.
        unsafe { Self::from_raw(ffi::b2TransformPolygon(transform.into_raw(), &self.raw)) }
    }

    #[inline]
    pub fn try_transformed(self, transform: Transform) -> ApiResult<Self> {
        check_polygon_helper_geometry_valid(self)?;
        check_transform_valid(transform)?;
        let _lease = transient_native_lease()?;
        // SAFETY: the source polygon invariant and transform were validated before the native call.
        Ok(unsafe { Self::from_raw(ffi::b2TransformPolygon(transform.into_raw(), &self.raw)) })
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        assert_polygon_helper_geometry_valid(self);
        assert_non_negative_finite_density(density);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        MassData::from_raw(unsafe { ffi::b2ComputePolygonMass(&raw, density) })
    }

    #[inline]
    pub fn try_mass_data(self, density: f32) -> ApiResult<MassData> {
        check_polygon_helper_geometry_valid(self)?;
        check_non_negative_finite_density(density)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(MassData::from_raw(unsafe {
            ffi::b2ComputePolygonMass(&raw, density)
        }))
    }

    /// Compute an absolute world-space AABB using `transform` as the polygon's
    /// local-to-world transform.
    ///
    /// The result uses `f32` coordinates in both precision modes. Double-precision world bounds
    /// are narrowed outward by Box2D so the returned AABB remains conservative.
    #[inline]
    pub fn aabb(self, transform: WorldTransform) -> Aabb {
        assert_polygon_helper_geometry_valid(self);
        assert_world_transform_valid(transform);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        Aabb::from_raw(unsafe { ffi::b2ComputePolygonAABB(&raw, transform.into_raw()) })
    }

    #[inline]
    pub fn try_aabb(self, transform: WorldTransform) -> ApiResult<Aabb> {
        check_polygon_helper_geometry_valid(self)?;
        check_world_transform_valid(transform)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(Aabb::from_raw(unsafe {
            ffi::b2ComputePolygonAABB(&raw, transform.into_raw())
        }))
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let point = point.into();
        assert_polygon_helper_geometry_valid(self);
        assert_valid_geometry_vec2("point", point);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        unsafe { ffi::b2PointInPolygon(&raw, point.into_raw()) }
    }

    #[inline]
    pub fn try_contains_point<P: Into<Vec2>>(self, point: P) -> ApiResult<bool> {
        let point = point.into();
        check_polygon_helper_geometry_valid(self)?;
        check_valid_geometry_vec2(point)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(unsafe { ffi::b2PointInPolygon(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let input = materialize_ray_input(origin, translation);
        assert_polygon_helper_geometry_valid(self);
        assert_ray_input_valid(&input);
        let raw = self.into_raw();
        let _lease = assert_transient_native_lease();
        CastOutput::from_raw(unsafe { ffi::b2RayCastPolygon(&raw, &input) })
    }

    #[inline]
    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        let input = materialize_ray_input(origin, translation);
        check_polygon_helper_geometry_valid(self)?;
        check_ray_input_valid(&input)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
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
        let _lease = assert_transient_native_lease();
        CastOutput::from_raw(unsafe { ffi::b2ShapeCastPolygon(&raw, &input) })
    }

    #[inline]
    pub fn try_shape_cast(self, input: ShapeCastInput) -> ApiResult<CastOutput> {
        check_polygon_helper_geometry_valid(self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = transient_native_lease()?;
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
