use crate::{
    collision::{CastOutput, ShapeCastInput},
    core::{
        foundation::{
            TransientFoundationLease, current_length_units_per_meter, transient_native_lease,
        },
        math::Transform,
    },
    error::{Error, Result},
    query::Aabb,
    types::{MassData, Vec2, WorldTransform},
};
use boxdd_sys::ffi;
use core::fmt;
use smallvec::SmallVec;
use std::collections::HashMap;

mod capsule;
mod chain_segment;
mod circle;
mod polygon;
mod segment;

/// Maximum number of vertices supported by a convex Box2D polygon.
pub const MAX_POLYGON_VERTICES: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;

const MAX_POLYGON_INPUT_POINTS: usize = MAX_POLYGON_VERTICES + 1;
const POLYGON_COMPONENT_TOLERANCE: f32 = 16.0 * f32::EPSILON;
const POLYGON_EDGE_CANCELLATION_TOLERANCE: f32 = 4.0 * f32::EPSILON;
const POLYGON_CENTROID_COORDINATE_TOLERANCE: f64 = 2.0 * f32::EPSILON as f64;
const POLYGON_CENTROID_ACCUMULATION_TOLERANCE: f64 = 16.0 * f32::EPSILON as f64;
const POLYGON_NORMAL_LENGTH_SQUARED_MIN: f32 = 0.9994;
const POLYGON_NORMAL_LENGTH_SQUARED_MAX: f32 = 1.0006;
const POLYGON_ONE_THIRD: f64 = 1.0 / 3.0;

const _: () = {
    assert!(core::mem::size_of::<Vec2>() == core::mem::size_of::<ffi::b2Vec2>());
    assert!(core::mem::align_of::<Vec2>() == core::mem::align_of::<ffi::b2Vec2>());
};

#[inline]
fn materialize_ray_input<VO: Into<Vec2>, VT: Into<Vec2>>(
    origin: VO,
    translation: VT,
) -> ffi::b2RayCastInput {
    let origin = origin.into();
    let translation = translation.into();
    raw_ray_input(origin, translation)
}

#[inline]
fn check_ray_input_valid(operation: &'static str, input: &ffi::b2RayCastInput) -> Result<()> {
    if !geometry_vec2_is_valid(Vec2::from_raw(input.origin)) {
        return Err(Error::invalid_argument(
            operation,
            "origin",
            "a finite vector",
        ));
    }
    if !geometry_vec2_is_valid(Vec2::from_raw(input.translation)) {
        return Err(Error::invalid_argument(
            operation,
            "translation",
            "a finite vector",
        ));
    }
    Ok(())
}

#[inline]
fn raw_ray_input(origin: Vec2, translation: Vec2) -> ffi::b2RayCastInput {
    ffi::b2RayCastInput {
        origin: origin.into_raw(),
        translation: translation.into_raw(),
        maxFraction: 1.0,
    }
}

#[inline]
fn collect_polygon_points<I, P>(
    points: I,
) -> Option<SmallVec<[ffi::b2Vec2; MAX_POLYGON_INPUT_POINTS]>>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    let mut pts: SmallVec<[ffi::b2Vec2; MAX_POLYGON_INPUT_POINTS]> =
        SmallVec::with_capacity(MAX_POLYGON_INPUT_POINTS);
    for point in points {
        if pts.len() == MAX_POLYGON_INPUT_POINTS {
            return None;
        }
        pts.push(point.into().into_raw());
    }

    if pts.is_empty() || pts.len() > MAX_POLYGON_VERTICES {
        return None;
    }

    Some(pts)
}

#[inline]
fn polygon_points_are_valid(points: &[ffi::b2Vec2]) -> bool {
    points
        .iter()
        .copied()
        .all(|point| geometry_vec2_is_valid(Vec2::from_raw(point)))
}

#[inline]
fn compute_hull_from_points(
    operation: &'static str,
    points: &[ffi::b2Vec2],
    _lease: &TransientFoundationLease,
) -> Result<Option<ffi::b2Hull>> {
    let hull = unsafe { ffi::b2ComputeHull(points.as_ptr(), points.len() as i32) };
    validate_native_hull(operation, hull)
}

#[inline]
fn validate_native_hull(operation: &'static str, hull: ffi::b2Hull) -> Result<Option<ffi::b2Hull>> {
    if hull.count == 0 {
        return Ok(None);
    }
    let count = usize::try_from(hull.count).map_err(|_| Error::InvalidNativeOutput {
        operation,
        output: "hull",
        constraint: "an empty hull or three to Box2D's maximum finite convex hull points",
    })?;
    if !(3..=MAX_POLYGON_VERTICES).contains(&count)
        || !hull.points[..count]
            .iter()
            .copied()
            .map(Vec2::from_raw)
            .all(Vec2::is_valid)
        || !unsafe { ffi::b2ValidateHull(&hull) }
    {
        return Err(Error::InvalidNativeOutput {
            operation,
            output: "hull",
            constraint: "an empty hull or three to Box2D's maximum finite convex hull points",
        });
    }
    Ok(Some(hull))
}

#[inline]
fn geometry_float_is_valid(value: f32) -> bool {
    value.is_finite()
}

#[inline]
fn geometry_scalar_is_non_negative_finite(value: f32) -> bool {
    geometry_float_is_valid(value) && value >= 0.0
}

#[inline]
fn geometry_vec2_is_valid(value: Vec2) -> bool {
    value.is_valid()
}

#[inline]
fn geometry_density_is_valid(value: f32) -> bool {
    geometry_scalar_is_non_negative_finite(value)
}

#[inline]
fn minimum_shape_segment_length_squared() -> Result<f32> {
    let linear_slop = 0.005 * current_length_units_per_meter()?;
    Ok(linear_slop * linear_slop)
}

#[inline]
fn point_pair_has_minimum_separation(a: Vec2, b: Vec2) -> Result<bool> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let separation_squared = dx * dx + dy * dy;
    let minimum_separation_squared = minimum_shape_segment_length_squared()?;
    Ok(separation_squared.is_finite()
        && minimum_separation_squared.is_finite()
        && separation_squared > minimum_separation_squared)
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
enum SeparationCell {
    Grid(i64),
    Exact(u32),
}

#[inline]
fn separation_cell(value: f32, cell_width: f32) -> SeparationCell {
    if cell_width == 0.0 {
        return SeparationCell::Exact(if value == 0.0 { 0 } else { value.to_bits() });
    }

    let scaled = f64::from(value) / f64::from(cell_width);
    let lower = i64::MIN as f64 / 2.0;
    let upper = i64::MAX as f64 / 2.0;
    if (lower..=upper).contains(&scaled) {
        SeparationCell::Grid(scaled.floor() as i64)
    } else {
        // At this magnitude adjacent finite f32 values are farther apart than the cell width, so
        // only an exactly equal coordinate can be close enough to matter.
        SeparationCell::Exact(value.to_bits())
    }
}

#[inline]
fn neighboring_separation_cells(cell: SeparationCell) -> [Option<SeparationCell>; 3] {
    match cell {
        SeparationCell::Grid(value) => [
            Some(SeparationCell::Grid(value - 1)),
            Some(cell),
            Some(SeparationCell::Grid(value + 1)),
        ],
        SeparationCell::Exact(_) => [None, Some(cell), None],
    }
}

/// Validate Box2D's chain-wide rule that every pair of points is farther apart than linear slop.
pub(crate) fn points_have_minimum_pairwise_separation(
    points: &[Vec2],
    length_units_per_meter: f32,
) -> core::result::Result<bool, std::collections::TryReserveError> {
    if !crate::core::length_scale::is_safe_length_units_per_meter(length_units_per_meter) {
        return Ok(false);
    }

    let cell_width = 0.005 * length_units_per_meter;
    if !cell_width.is_finite() {
        return Ok(false);
    }

    let mut cells: HashMap<(SeparationCell, SeparationCell), SmallVec<[Vec2; 4]>> = HashMap::new();
    cells.try_reserve(points.len())?;

    for &point in points {
        if !point.is_valid() {
            return Ok(false);
        }

        let x_cell = separation_cell(point.x, cell_width);
        let y_cell = separation_cell(point.y, cell_width);
        for neighbor_x in neighboring_separation_cells(x_cell).into_iter().flatten() {
            for neighbor_y in neighboring_separation_cells(y_cell).into_iter().flatten() {
                let Some(neighbors) = cells.get(&(neighbor_x, neighbor_y)) else {
                    continue;
                };
                if neighbors.iter().any(|neighbor| {
                    let dx = point.x - neighbor.x;
                    let dy = point.y - neighbor.y;
                    dx.hypot(dy) <= cell_width
                }) {
                    return Ok(false);
                }
            }
        }

        cells.entry((x_cell, y_cell)).or_default().push(point);
    }

    Ok(true)
}

#[inline]
fn geometry_is_valid_or_err(
    operation: &'static str,
    argument: &'static str,
    constraint: &'static str,
    valid: bool,
) -> Result<()> {
    if valid {
        Ok(())
    } else {
        Err(Error::invalid_argument(operation, argument, constraint))
    }
}

#[inline]
fn check_valid_geometry_vec2(
    operation: &'static str,
    argument: &'static str,
    value: Vec2,
) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        argument,
        "a finite vector",
        geometry_vec2_is_valid(value),
    )
}

#[inline]
fn check_non_negative_finite_density(operation: &'static str, density: f32) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "density",
        "a finite value greater than or equal to zero",
        geometry_density_is_valid(density),
    )
}

#[inline]
fn check_positive_finite_polygon_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        argument,
        "a finite value greater than zero",
        geometry_float_is_valid(value) && value > 0.0,
    )
}

#[inline]
fn check_non_negative_finite_polygon_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        argument,
        "a finite value greater than or equal to zero",
        geometry_scalar_is_non_negative_finite(value),
    )
}

#[inline]
fn check_transform_valid(operation: &'static str, transform: Transform) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "transform",
        "a finite rigid transform",
        transform.is_valid(),
    )
}

#[inline]
fn check_world_transform_valid(operation: &'static str, transform: WorldTransform) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "transform",
        "a finite rigid world transform",
        transform.is_valid(),
    )
}

#[inline]
fn check_native_geometry_aabb(operation: &'static str, raw: ffi::b2AABB) -> Result<Aabb> {
    // SAFETY: this immediately validates the complete native output before publication.
    let aabb = Aabb::from_raw_unvalidated(raw);
    if aabb.is_valid() {
        Ok(aabb)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "aabb",
            constraint: "finite ordered lower and upper bounds",
        })
    }
}

#[inline]
fn circle_helper_geometry_is_valid(circle: Circle) -> bool {
    geometry_vec2_is_valid(circle.center) && geometry_scalar_is_non_negative_finite(circle.radius)
}

#[inline]
fn check_circle_helper_geometry_valid(operation: &'static str, circle: Circle) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "circle",
        "finite center coordinates and a finite non-negative radius",
        circle_helper_geometry_is_valid(circle),
    )
}

#[inline]
fn segment_helper_geometry_is_valid(segment: Segment) -> bool {
    geometry_vec2_is_valid(segment.point1) && geometry_vec2_is_valid(segment.point2)
}

#[inline]
fn segment_geometry_is_valid(segment: Segment) -> bool {
    segment_helper_geometry_is_valid(segment)
        && point_pair_has_minimum_separation(segment.point1, segment.point2).unwrap_or(false)
}

#[inline]
fn check_segment_geometry_valid_for_operation(
    operation: &'static str,
    segment: Segment,
) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "segment",
        "finite endpoints separated by Box2D's minimum segment length",
        segment_helper_geometry_is_valid(segment)
            && point_pair_has_minimum_separation(segment.point1, segment.point2)?,
    )
}

#[inline]
fn chain_segment_geometry_is_valid(segment: ChainSegment) -> bool {
    geometry_vec2_is_valid(segment.ghost1)
        && segment_geometry_is_valid(segment.segment)
        && geometry_vec2_is_valid(segment.ghost2)
}

#[inline]
fn check_chain_segment_geometry_valid_for_operation(
    operation: &'static str,
    segment: ChainSegment,
) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "chain_segment",
        "finite ghost points and segment endpoints separated by Box2D's minimum length",
        geometry_vec2_is_valid(segment.ghost1)
            && segment_helper_geometry_is_valid(segment.segment)
            && point_pair_has_minimum_separation(segment.segment.point1, segment.segment.point2)?
            && geometry_vec2_is_valid(segment.ghost2),
    )
}

#[inline]
fn check_segment_helper_geometry_valid(operation: &'static str, segment: Segment) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "segment",
        "finite endpoint coordinates",
        segment_helper_geometry_is_valid(segment),
    )
}

#[inline]
fn capsule_helper_geometry_is_valid(capsule: Capsule) -> bool {
    geometry_vec2_is_valid(capsule.center1)
        && geometry_vec2_is_valid(capsule.center2)
        && geometry_scalar_is_non_negative_finite(capsule.radius)
}

#[inline]
fn capsule_geometry_is_valid(capsule: Capsule) -> bool {
    capsule_helper_geometry_is_valid(capsule)
        && point_pair_has_minimum_separation(capsule.center1, capsule.center2).unwrap_or(false)
}

#[inline]
fn check_capsule_geometry_valid_for_operation(
    operation: &'static str,
    capsule: Capsule,
) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "capsule",
        "finite geometry with endpoints separated by Box2D's minimum length and a non-negative radius",
        capsule_helper_geometry_is_valid(capsule)
            && point_pair_has_minimum_separation(capsule.center1, capsule.center2)?,
    )
}

#[inline]
fn check_capsule_helper_geometry_valid(operation: &'static str, capsule: Capsule) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "capsule",
        "finite endpoints and a finite non-negative radius",
        capsule_helper_geometry_is_valid(capsule),
    )
}

#[inline]
pub(crate) fn polygon_semantics_are_valid(
    vertices: &[Vec2],
    normals: &[Vec2],
    centroid: Vec2,
    radius: f32,
    minimum_edge_length_squared: f32,
) -> bool {
    if !(3..=MAX_POLYGON_VERTICES).contains(&vertices.len())
        || normals.len() != vertices.len()
        || !centroid.is_valid()
        || !geometry_scalar_is_non_negative_finite(radius)
        || !minimum_edge_length_squared.is_finite()
        || minimum_edge_length_squared < 0.0
        || !vertices.iter().copied().all(geometry_vec2_is_valid)
        || !normals.iter().copied().all(geometry_vec2_is_valid)
    {
        return false;
    }

    let origin = vertices[0];
    let mut area = 0.0_f32;
    let mut precise_area = 0.0_f64;
    let mut center_offset_x = 0.0_f64;
    let mut center_offset_y = 0.0_f64;
    let mut center_accumulation_x = 0.0_f64;
    let mut center_accumulation_y = 0.0_f64;
    for index in 1..vertices.len() - 1 {
        let edge1_x = vertices[index].x - origin.x;
        let edge1_y = vertices[index].y - origin.y;
        let edge2_x = vertices[index + 1].x - origin.x;
        let edge2_y = vertices[index + 1].y - origin.y;
        let triangle_area = 0.5 * (edge1_x * edge2_y - edge1_y * edge2_x);
        area += triangle_area;

        // Native helpers can encode an exact center even when f32 fan summation leaves residue.
        let precise_edge1_x = f64::from(vertices[index].x) - f64::from(origin.x);
        let precise_edge1_y = f64::from(vertices[index].y) - f64::from(origin.y);
        let precise_edge2_x = f64::from(vertices[index + 1].x) - f64::from(origin.x);
        let precise_edge2_y = f64::from(vertices[index + 1].y) - f64::from(origin.y);
        let precise_triangle_area =
            0.5 * (precise_edge1_x * precise_edge2_y - precise_edge1_y * precise_edge2_x);
        precise_area += precise_triangle_area;
        let centroid_weight = precise_triangle_area * POLYGON_ONE_THIRD;
        let contribution_x = centroid_weight * (precise_edge1_x + precise_edge2_x);
        let contribution_y = centroid_weight * (precise_edge1_y + precise_edge2_y);
        center_offset_x += contribution_x;
        center_offset_y += contribution_y;
        center_accumulation_x += contribution_x.abs();
        center_accumulation_y += contribution_y.abs();
    }
    if !area.is_finite()
        || area <= f32::EPSILON
        || !precise_area.is_finite()
        || precise_area <= f64::from(f32::EPSILON)
        || !center_offset_x.is_finite()
        || !center_offset_y.is_finite()
    {
        return false;
    }
    let expected_centroid_x = f64::from(origin.x) + center_offset_x / precise_area;
    let expected_centroid_y = f64::from(origin.y) + center_offset_y / precise_area;
    let coordinate_scale_x = vertices
        .iter()
        .map(|vertex| f64::from(vertex.x).abs())
        .fold(1.0_f64, f64::max);
    let coordinate_scale_y = vertices
        .iter()
        .map(|vertex| f64::from(vertex.y).abs())
        .fold(1.0_f64, f64::max);
    // Bound native f32 coordinate rounding separately from fan-sum accumulation.
    if !polygon_component_matches(
        centroid.x,
        expected_centroid_x,
        coordinate_scale_x,
        center_accumulation_x / precise_area,
    ) || !polygon_component_matches(
        centroid.y,
        expected_centroid_y,
        coordinate_scale_y,
        center_accumulation_y / precise_area,
    ) {
        return false;
    }

    for edge_index in 0..vertices.len() {
        let next_index = (edge_index + 1) % vertices.len();
        let start = vertices[edge_index];
        let end = vertices[next_index];
        let edge_x = end.x - start.x;
        let edge_y = end.y - start.y;
        let edge_length_squared = edge_x * edge_x + edge_y * edge_y;
        if !edge_length_squared.is_finite() || edge_length_squared <= minimum_edge_length_squared {
            return false;
        }

        if !polygon_normal_matches_edge(start, end, normals[edge_index], edge_x, edge_y) {
            return false;
        }

        for (point_index, point) in vertices.iter().copied().enumerate() {
            if point_index == edge_index || point_index == next_index {
                continue;
            }
            let cross = edge_x * (point.y - start.y) - edge_y * (point.x - start.x);
            if !cross.is_finite() || cross <= 0.0 {
                return false;
            }
        }
    }

    true
}

#[inline]
fn polygon_normal_matches_edge(
    start: Vec2,
    end: Vec2,
    normal: Vec2,
    edge_x: f32,
    edge_y: f32,
) -> bool {
    let normal_length_squared = normal.x * normal.x + normal.y * normal.y;
    if normal_length_squared <= POLYGON_NORMAL_LENGTH_SQUARED_MIN
        || normal_length_squared >= POLYGON_NORMAL_LENGTH_SQUARED_MAX
    {
        return false;
    }

    let perpendicular_residual = edge_x * normal.x + edge_y * normal.y;
    let outward_alignment = edge_y * normal.x - edge_x * normal.y;
    let vertex_scale = start
        .x
        .abs()
        .max(start.y.abs())
        .max(end.x.abs())
        .max(end.y.abs())
        .max(1.0);
    let normal_scale = normal.x.abs() + normal.y.abs();
    let residual_tolerance = POLYGON_COMPONENT_TOLERANCE * (edge_x.abs() + edge_y.abs())
        + POLYGON_EDGE_CANCELLATION_TOLERANCE * vertex_scale * normal_scale;

    perpendicular_residual.is_finite()
        && outward_alignment.is_finite()
        && outward_alignment > 0.0
        && residual_tolerance.is_finite()
        && perpendicular_residual.abs() <= residual_tolerance
}

#[inline]
fn polygon_component_matches(
    actual: f32,
    expected: f64,
    vertex_coordinate_scale: f64,
    accumulation_scale: f64,
) -> bool {
    if !actual.is_finite()
        || !expected.is_finite()
        || !vertex_coordinate_scale.is_finite()
        || !accumulation_scale.is_finite()
    {
        return false;
    }
    let coordinate_scale = f64::from(actual)
        .abs()
        .max(expected.abs())
        .max(vertex_coordinate_scale)
        .max(1.0);
    let tolerance = POLYGON_CENTROID_COORDINATE_TOLERANCE * coordinate_scale
        + POLYGON_CENTROID_ACCUMULATION_TOLERANCE * accumulation_scale.max(1.0);
    (f64::from(actual) - expected).abs() <= tolerance
}

#[inline]
fn polygon_helper_geometry_is_valid(polygon: Polygon) -> bool {
    if !(3..=MAX_POLYGON_VERTICES as i32).contains(&polygon.raw.count) {
        return false;
    }
    polygon_semantics_are_valid(
        polygon.vertices(),
        polygon.normals(),
        polygon.centroid(),
        polygon.radius(),
        f32::EPSILON * f32::EPSILON,
    )
}

#[inline]
fn check_polygon_helper_geometry_valid(operation: &'static str, polygon: Polygon) -> Result<()> {
    geometry_is_valid_or_err(
        operation,
        "polygon",
        "a valid convex Box2D polygon",
        polygon_helper_geometry_is_valid(polygon),
    )
}

#[inline]
fn require_hull_from_points(
    operation: &'static str,
    points: &[ffi::b2Vec2],
    lease: &TransientFoundationLease,
) -> Result<ffi::b2Hull> {
    compute_hull_from_points(operation, points, lease)?.ok_or(Error::invalid_argument(
        operation,
        "points",
        "points that form a non-degenerate convex hull",
    ))
}

/// Circle geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Circle {
    pub(crate) center: Vec2,
    pub(crate) radius: f32,
}

/// Line segment geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Segment {
    pub(crate) point1: Vec2,
    pub(crate) point2: Vec2,
}

/// One-sided chain segment geometry with ghost vertices on both ends.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Copy, Clone)]
pub struct ChainSegment {
    pub(crate) ghost1: Vec2,
    pub(crate) segment: Segment,
    pub(crate) ghost2: Vec2,
}

/// Capsule geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Capsule {
    pub(crate) center1: Vec2,
    pub(crate) center2: Vec2,
    pub(crate) radius: f32,
}

/// Convex polygon geometry in local shape space.
///
/// Construct polygons with helpers such as [`square_polygon`], [`box_polygon`],
/// [`rounded_box_polygon`], [`offset_box_polygon`], or [`polygon_from_points`]
/// instead of filling raw vertices manually.
#[doc(alias = "polygon")]
#[derive(Copy, Clone)]
pub struct Polygon {
    raw: ffi::b2Polygon,
}

/// Circle helper.
#[inline]
pub fn circle<C: Into<Vec2>>(center: C, radius: f32) -> Result<Circle> {
    Circle::new(center, radius)
}

/// Segment helper.
#[inline]
pub fn segment<P1: Into<Vec2>, P2: Into<Vec2>>(point1: P1, point2: P2) -> Result<Segment> {
    Segment::new(point1, point2)
}

/// Chain segment helper.
#[inline]
pub fn chain_segment<G1, P1, P2, G2>(
    ghost1: G1,
    point1: P1,
    point2: P2,
    ghost2: G2,
) -> Result<ChainSegment>
where
    G1: Into<Vec2>,
    P1: Into<Vec2>,
    P2: Into<Vec2>,
    G2: Into<Vec2>,
{
    ChainSegment::new(ghost1, point1, point2, ghost2)
}

/// Capsule helper.
#[inline]
pub fn capsule<C1: Into<Vec2>, C2: Into<Vec2>>(
    center1: C1,
    center2: C2,
    radius: f32,
) -> Result<Capsule> {
    Capsule::new(center1, center2, radius)
}

/// Axis-aligned box polygon helper.
#[inline]
pub fn box_polygon(half_width: f32, half_height: f32) -> Result<Polygon> {
    Polygon::box_polygon(half_width, half_height)
}

/// Axis-aligned square polygon helper.
#[inline]
pub fn square_polygon(half_width: f32) -> Result<Polygon> {
    Polygon::square_polygon(half_width)
}

/// Axis-aligned rounded box polygon helper.
#[inline]
pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Result<Polygon> {
    Polygon::rounded_box_polygon(half_width, half_height, radius)
}

/// Offset box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn offset_box_polygon(
    half_width: f32,
    half_height: f32,
    transform: Transform,
) -> Result<Polygon> {
    Polygon::offset_box_polygon(half_width, half_height, transform)
}

/// Offset rounded box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn offset_rounded_box_polygon(
    half_width: f32,
    half_height: f32,
    radius: f32,
    transform: Transform,
) -> Result<Polygon> {
    Polygon::offset_rounded_box_polygon(half_width, half_height, radius, transform)
}

/// Build a polygon from arbitrary points by computing a convex hull.
#[inline]
pub fn polygon_from_points<I, P>(points: I, radius: f32) -> Result<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::from_points(points, radius)
}

/// Build an offset polygon from arbitrary points by computing a convex hull first.
#[inline]
pub fn offset_polygon_from_points<I, P>(
    points: I,
    radius: f32,
    transform: Transform,
) -> Result<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::offset_from_points(points, radius, transform)
}

/// Use native Box2D hull computation to check whether a point set produces a valid convex hull.
#[inline]
pub fn polygon_hull_is_valid<I, P>(points: I) -> Result<bool>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::hull_is_valid(points)
}
