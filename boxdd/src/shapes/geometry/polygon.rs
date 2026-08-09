use super::*;

impl Polygon {
    #[inline]
    /// Construct from a raw Box2D geometry value after validating its full polygon invariant.
    pub fn from_raw(raw: ffi::b2Polygon) -> Result<Self> {
        let polygon = Self::from_raw_unvalidated(raw);
        check_polygon_helper_geometry_valid("Polygon::from_raw", polygon)?;
        Ok(polygon)
    }

    #[inline]
    pub(crate) const fn from_raw_unvalidated(raw: ffi::b2Polygon) -> Self {
        Self { raw }
    }

    #[inline]
    fn from_native(operation: &'static str, raw: ffi::b2Polygon) -> Result<Self> {
        let polygon = Self::from_raw_unvalidated(raw);
        check_polygon_helper_geometry_valid(operation, polygon).map_err(|_| {
            Error::InvalidNativeOutput {
                operation,
                output: "polygon",
                constraint: "a valid finite convex polygon with consistent normals and centroid",
            }
        })?;
        Ok(polygon)
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
    pub fn validate(self) -> Result<()> {
        check_polygon_helper_geometry_valid("Polygon::validate", self)
    }

    #[inline]
    pub fn square_polygon(half_width: f32) -> Result<Self> {
        check_positive_finite_polygon_scalar("Polygon::square_polygon", "half_width", half_width)?;
        let _lease = transient_native_lease()?;
        Self::from_native("Polygon::square_polygon", unsafe {
            ffi::b2MakeSquare(half_width)
        })
    }

    #[inline]
    pub fn box_polygon(half_width: f32, half_height: f32) -> Result<Self> {
        check_positive_finite_polygon_scalar("Polygon::box_polygon", "half_width", half_width)?;
        check_positive_finite_polygon_scalar("Polygon::box_polygon", "half_height", half_height)?;
        let _lease = transient_native_lease()?;
        Self::from_native("Polygon::box_polygon", unsafe {
            ffi::b2MakeBox(half_width, half_height)
        })
    }

    #[inline]
    pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Result<Self> {
        check_positive_finite_polygon_scalar(
            "Polygon::rounded_box_polygon",
            "half_width",
            half_width,
        )?;
        check_positive_finite_polygon_scalar(
            "Polygon::rounded_box_polygon",
            "half_height",
            half_height,
        )?;
        check_non_negative_finite_polygon_scalar("Polygon::rounded_box_polygon", "radius", radius)?;
        let _lease = transient_native_lease()?;
        Self::from_native("Polygon::rounded_box_polygon", unsafe {
            ffi::b2MakeRoundedBox(half_width, half_height, radius)
        })
    }

    #[inline]
    pub fn offset_box_polygon(
        half_width: f32,
        half_height: f32,
        transform: Transform,
    ) -> Result<Self> {
        check_positive_finite_polygon_scalar(
            "Polygon::offset_box_polygon",
            "half_width",
            half_width,
        )?;
        check_positive_finite_polygon_scalar(
            "Polygon::offset_box_polygon",
            "half_height",
            half_height,
        )?;
        check_transform_valid("Polygon::offset_box_polygon", transform)?;
        let _lease = transient_native_lease()?;
        Self::from_native("Polygon::offset_box_polygon", unsafe {
            ffi::b2MakeOffsetBox(
                half_width,
                half_height,
                transform.position().into_raw(),
                transform.rotation().into_raw(),
            )
        })
    }

    #[inline]
    pub fn offset_rounded_box_polygon(
        half_width: f32,
        half_height: f32,
        radius: f32,
        transform: Transform,
    ) -> Result<Self> {
        check_positive_finite_polygon_scalar(
            "Polygon::offset_rounded_box_polygon",
            "half_width",
            half_width,
        )?;
        check_positive_finite_polygon_scalar(
            "Polygon::offset_rounded_box_polygon",
            "half_height",
            half_height,
        )?;
        check_non_negative_finite_polygon_scalar(
            "Polygon::offset_rounded_box_polygon",
            "radius",
            radius,
        )?;
        check_transform_valid("Polygon::offset_rounded_box_polygon", transform)?;
        let _lease = transient_native_lease()?;
        Self::from_native("Polygon::offset_rounded_box_polygon", unsafe {
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
    pub fn from_points<I, P>(points: I, radius: f32) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        const OPERATION: &str = "Polygon::from_points";
        let points = collect_polygon_points(points).ok_or(Error::invalid_argument(
            OPERATION,
            "points",
            "between 1 and Box2D's maximum polygon vertex count",
        ))?;
        check_non_negative_finite_polygon_scalar(OPERATION, "radius", radius)?;
        geometry_is_valid_or_err(
            OPERATION,
            "points",
            "finite point coordinates",
            polygon_points_are_valid(&points),
        )?;
        let _lease = transient_native_lease()?;
        let hull = require_hull_from_points(OPERATION, &points, &_lease)?;
        Self::from_native(OPERATION, unsafe { ffi::b2MakePolygon(&hull, radius) })
    }

    #[inline]
    pub fn offset_from_points<I, P>(points: I, radius: f32, transform: Transform) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        const OPERATION: &str = "Polygon::offset_from_points";
        let points = collect_polygon_points(points).ok_or(Error::invalid_argument(
            OPERATION,
            "points",
            "between 1 and Box2D's maximum polygon vertex count",
        ))?;
        check_non_negative_finite_polygon_scalar(OPERATION, "radius", radius)?;
        check_transform_valid(OPERATION, transform)?;
        geometry_is_valid_or_err(
            OPERATION,
            "points",
            "finite point coordinates",
            polygon_points_are_valid(&points),
        )?;
        let _lease = transient_native_lease()?;
        let hull = require_hull_from_points(OPERATION, &points, &_lease)?;
        let hull = materialize_offset_hull(OPERATION, hull, transform)?;
        let identity = Transform::IDENTITY;
        Self::from_native(OPERATION, unsafe {
            if radius == 0.0 {
                ffi::b2MakeOffsetPolygon(
                    &hull,
                    identity.position().into_raw(),
                    identity.rotation().into_raw(),
                )
            } else {
                ffi::b2MakeOffsetRoundedPolygon(
                    &hull,
                    identity.position().into_raw(),
                    identity.rotation().into_raw(),
                    radius,
                )
            }
        })
    }

    /// Return whether Box2D can compute and validate a convex hull from `points`.
    ///
    /// Point conversion and finite-value validation complete before foundation activity is leased.
    /// Degenerate point sets return `Ok(false)`; malformed input or unavailable foundation activity
    /// returns an error.
    #[inline]
    pub fn hull_is_valid<I, P>(points: I) -> Result<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        const OPERATION: &str = "Polygon::hull_is_valid";
        let points = collect_polygon_points(points).ok_or(Error::invalid_argument(
            OPERATION,
            "points",
            "between 1 and Box2D's maximum polygon vertex count",
        ))?;
        geometry_is_valid_or_err(
            OPERATION,
            "points",
            "finite point coordinates",
            polygon_points_are_valid(&points),
        )?;
        let lease = transient_native_lease()?;
        let Some(_hull) = compute_hull_from_points(OPERATION, &points, &lease)? else {
            return Ok(false);
        };
        Ok(true)
    }

    #[inline]
    pub fn transformed(self, transform: Transform) -> Result<Self> {
        check_polygon_helper_geometry_valid("Polygon::transformed", self)?;
        check_transform_valid("Polygon::transformed", transform)?;
        let _lease = transient_native_lease()?;
        Self::from_native("Polygon::transformed", unsafe {
            ffi::b2TransformPolygon(transform.into_raw(), &self.raw)
        })
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> Result<MassData> {
        check_polygon_helper_geometry_valid("Polygon::mass_data", self)?;
        check_non_negative_finite_density("Polygon::mass_data", density)?;
        check_polygon_mass_calculation_safe("Polygon::mass_data", self, density)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        MassData::from_native("Polygon::mass_data", unsafe {
            ffi::b2ComputePolygonMass(&raw, density)
        })
    }

    /// Compute an absolute world-space AABB using `transform` as the polygon's
    /// local-to-world transform.
    ///
    /// The result uses `f32` coordinates in both precision modes. Double-precision world bounds
    /// are narrowed outward by Box2D so the returned AABB remains conservative.
    #[inline]
    pub fn aabb(self, transform: WorldTransform) -> Result<Aabb> {
        check_polygon_helper_geometry_valid("Polygon::aabb", self)?;
        check_world_transform_valid("Polygon::aabb", transform)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        check_native_geometry_aabb("Polygon::aabb", unsafe {
            ffi::b2ComputePolygonAABB(&raw, transform.into_raw())
        })
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> Result<bool> {
        let point = point.into();
        check_polygon_helper_geometry_valid("Polygon::contains_point", self)?;
        check_valid_geometry_vec2("Polygon::contains_point", "point", point)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        Ok(unsafe { ffi::b2PointInPolygon(&raw, point.into_raw()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> Result<CastOutput> {
        let input = materialize_ray_input(origin, translation);
        check_polygon_helper_geometry_valid("Polygon::ray_cast", self)?;
        check_ray_input_valid("Polygon::ray_cast", &input)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Polygon::ray_cast", unsafe {
            ffi::b2RayCastPolygon(&raw, &input)
        })
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> Result<CastOutput> {
        check_polygon_helper_geometry_valid("Polygon::shape_cast", self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Polygon::shape_cast", unsafe {
            ffi::b2ShapeCastPolygon(&raw, &input)
        })
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

fn materialize_offset_hull(
    operation: &'static str,
    hull: ffi::b2Hull,
    transform: Transform,
) -> Result<ffi::b2Hull> {
    let count = usize::try_from(hull.count).map_err(|_| Error::InvalidNativeOutput {
        operation,
        output: "hull",
        constraint: "three to Box2D's maximum finite convex hull points",
    })?;
    if !(3..=MAX_POLYGON_VERTICES).contains(&count) {
        return Err(Error::InvalidNativeOutput {
            operation,
            output: "hull",
            constraint: "three to Box2D's maximum finite convex hull points",
        });
    }

    let mut transformed = ffi::b2Hull {
        points: [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_POLYGON_VERTICES],
        count: hull.count,
    };
    for index in 0..count {
        transformed.points[index] = transform
            .transform_point(Vec2::from_raw(hull.points[index]))
            .into_raw();
    }

    // Materialize the transform before FFI so the identity path consumes exactly the checked f32 hull.
    if !offset_hull_is_safe_for_native(&transformed)
        || !unsafe { ffi::b2ValidateHull(&transformed) }
    {
        return Err(Error::invalid_argument(
            operation,
            "transform",
            "a finite rigid transform that preserves a non-degenerate f32 convex hull",
        ));
    }

    Ok(transformed)
}

fn offset_hull_is_safe_for_native(hull: &ffi::b2Hull) -> bool {
    let Ok(count) = usize::try_from(hull.count) else {
        return false;
    };
    if !(3..=MAX_POLYGON_VERTICES).contains(&count) {
        return false;
    }

    let points = &hull.points[..count];
    if !points
        .iter()
        .copied()
        .map(Vec2::from_raw)
        .all(Vec2::is_valid)
    {
        return false;
    }

    let minimum_edge_length_squared = f32::EPSILON * f32::EPSILON;
    for index in 0..count {
        let next = (index + 1) % count;
        let dx = points[next].x - points[index].x;
        let dy = points[next].y - points[index].y;
        let length_squared = dx * dx + dy * dy;
        if !length_squared.is_finite() || length_squared <= minimum_edge_length_squared {
            return false;
        }
    }

    let reference = Vec2::from_raw(points[0]);
    let mut center = Vec2::ZERO;
    let mut area = 0.0_f32;
    const INV3: f32 = 1.0 / 3.0;
    for index in 1..count - 1 {
        let first = Vec2::from_raw(points[index]);
        let second = Vec2::from_raw(points[index + 1]);
        let edge1_x = first.x - reference.x;
        let edge1_y = first.y - reference.y;
        let edge2_x = second.x - reference.x;
        let edge2_y = second.y - reference.y;
        let determinant = edge1_x * edge2_y - edge1_y * edge2_x;
        let triangle_area = 0.5 * determinant;
        let next_area = area + triangle_area;
        let weight = triangle_area * INV3;
        let next_center = Vec2::new(
            center.x + weight * (edge1_x + edge2_x),
            center.y + weight * (edge1_y + edge2_y),
        );
        if !determinant.is_finite()
            || !triangle_area.is_finite()
            || !next_area.is_finite()
            || !next_center.is_valid()
        {
            return false;
        }
        area = next_area;
        center = next_center;
    }

    if area <= f32::EPSILON {
        return false;
    }
    let inverse_area = 1.0 / area;
    let centroid = Vec2::new(
        reference.x + center.x * inverse_area,
        reference.y + center.y * inverse_area,
    );
    centroid.is_valid()
}

fn check_polygon_mass_calculation_safe(
    operation: &'static str,
    polygon: Polygon,
    density: f32,
) -> Result<()> {
    if polygon_mass_calculation_is_safe(polygon, density) {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            "polygon/density",
            "finite geometry and density that preserve finite Box2D mass properties",
        ))
    }
}

fn polygon_mass_calculation_is_safe(polygon: Polygon, density: f32) -> bool {
    let count = polygon.count();
    let source_vertices = polygon.vertices();
    let normals = polygon.normals();
    let radius = polygon.radius();
    let mut vertices = [Vec2::ZERO; MAX_POLYGON_VERTICES];

    if radius > 0.0 {
        let radius_scale = 1.412_f32 * radius;
        if !radius_scale.is_finite() {
            return false;
        }

        for index in 0..count {
            let previous = if index == 0 { count - 1 } else { index - 1 };
            let normal_x = normals[previous].x + normals[index].x;
            let normal_y = normals[previous].y + normals[index].y;
            let normal_length_squared = normal_x * normal_x + normal_y * normal_y;
            if !normal_length_squared.is_finite() {
                return false;
            }

            let normal_length = normal_length_squared.sqrt();
            let midpoint = if normal_length < f32::EPSILON {
                Vec2::ZERO
            } else {
                let inverse_length = 1.0 / normal_length;
                Vec2::new(inverse_length * normal_x, inverse_length * normal_y)
            };
            let vertex = Vec2::new(
                source_vertices[index].x + radius_scale * midpoint.x,
                source_vertices[index].y + radius_scale * midpoint.y,
            );
            if !midpoint.is_valid() || !vertex.is_valid() {
                return false;
            }
            vertices[index] = vertex;
        }
    } else {
        vertices[..count].copy_from_slice(source_vertices);
    }

    let reference = vertices[0];
    let mut center = Vec2::ZERO;
    let mut area = 0.0_f32;
    let mut rotational_inertia = 0.0_f32;
    const INV3: f32 = 1.0 / 3.0;
    for index in 1..count - 1 {
        let edge1_x = vertices[index].x - reference.x;
        let edge1_y = vertices[index].y - reference.y;
        let edge2_x = vertices[index + 1].x - reference.x;
        let edge2_y = vertices[index + 1].y - reference.y;
        let determinant = edge1_x * edge2_y - edge1_y * edge2_x;
        let triangle_area = 0.5 * determinant;
        let next_area = area + triangle_area;
        let center_weight = triangle_area * INV3;
        let next_center = Vec2::new(
            center.x + center_weight * (edge1_x + edge2_x),
            center.y + center_weight * (edge1_y + edge2_y),
        );
        let int_x2 = edge1_x * edge1_x + edge2_x * edge1_x + edge2_x * edge2_x;
        let int_y2 = edge1_y * edge1_y + edge2_y * edge1_y + edge2_y * edge2_y;
        let inertia_factor = 0.25_f32 * INV3 * determinant;
        let next_rotational_inertia = rotational_inertia + inertia_factor * (int_x2 + int_y2);
        if !determinant.is_finite()
            || !triangle_area.is_finite()
            || !next_area.is_finite()
            || !next_center.is_valid()
            || !int_x2.is_finite()
            || !int_y2.is_finite()
            || !inertia_factor.is_finite()
            || !next_rotational_inertia.is_finite()
        {
            return false;
        }
        area = next_area;
        center = next_center;
        rotational_inertia = next_rotational_inertia;
    }

    if !area.is_finite() || area <= f32::EPSILON {
        return false;
    }

    let mass = density * area;
    let inverse_area = 1.0 / area;
    let center = Vec2::new(center.x * inverse_area, center.y * inverse_area);
    let center_of_mass = Vec2::new(reference.x + center.x, reference.y + center.y);
    let center_squared = center.x * center.x + center.y * center.y;
    let inertia_before_shift = density * rotational_inertia;
    let rotational_inertia = inertia_before_shift - mass * center_squared;

    mass.is_finite()
        && mass >= 0.0
        && center.is_valid()
        && center_of_mass.is_valid()
        && center_squared.is_finite()
        && inertia_before_shift.is_finite()
        && rotational_inertia.is_finite()
        && rotational_inertia >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVALID_POLYGON_OUTPUT: Error = Error::InvalidNativeOutput {
        operation: "test_polygon",
        output: "polygon",
        constraint: "a valid finite convex polygon with consistent normals and centroid",
    };

    fn valid_square_raw() -> ffi::b2Polygon {
        let mut vertices = [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_POLYGON_VERTICES];
        vertices[..4].copy_from_slice(&[
            ffi::b2Vec2 { x: -1.0, y: -1.0 },
            ffi::b2Vec2 { x: 1.0, y: -1.0 },
            ffi::b2Vec2 { x: 1.0, y: 1.0 },
            ffi::b2Vec2 { x: -1.0, y: 1.0 },
        ]);

        let mut normals = [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_POLYGON_VERTICES];
        normals[..4].copy_from_slice(&[
            ffi::b2Vec2 { x: 0.0, y: -1.0 },
            ffi::b2Vec2 { x: 1.0, y: 0.0 },
            ffi::b2Vec2 { x: 0.0, y: 1.0 },
            ffi::b2Vec2 { x: -1.0, y: 0.0 },
        ]);

        ffi::b2Polygon {
            vertices,
            normals,
            centroid: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            radius: 0.0,
            count: 4,
        }
    }

    #[test]
    fn native_polygon_validation_rejects_invalid_results() {
        assert!(Polygon::from_native("test_polygon", valid_square_raw()).is_ok());

        let mut invalid_count = valid_square_raw();
        invalid_count.count = ffi::B2_MAX_POLYGON_VERTICES as i32 + 1;
        assert_eq!(
            Polygon::from_native("test_polygon", invalid_count).unwrap_err(),
            INVALID_POLYGON_OUTPUT
        );

        let mut invalid_vertex = valid_square_raw();
        invalid_vertex.vertices[0].x = f32::NAN;
        assert_eq!(
            Polygon::from_native("test_polygon", invalid_vertex).unwrap_err(),
            INVALID_POLYGON_OUTPUT
        );
    }

    #[test]
    fn native_hull_validation_rejects_invalid_results_without_publication() {
        let empty = ffi::b2Hull {
            points: [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_POLYGON_VERTICES],
            count: 0,
        };
        assert!(validate_native_hull("test_hull", empty).unwrap().is_none());

        let mut invalid_count = empty;
        invalid_count.count = ffi::B2_MAX_POLYGON_VERTICES as i32 + 1;
        assert!(matches!(
            validate_native_hull("test_hull", invalid_count),
            Err(Error::InvalidNativeOutput {
                operation: "test_hull",
                output: "hull",
                ..
            })
        ));

        let mut invalid_point = empty;
        invalid_point.count = 3;
        invalid_point.points[0].x = f32::NAN;
        assert!(matches!(
            validate_native_hull("test_hull", invalid_point),
            Err(Error::InvalidNativeOutput {
                operation: "test_hull",
                output: "hull",
                ..
            })
        ));
    }
}
