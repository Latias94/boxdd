use crate::{
    collision::{CastOutput, ShapeCastInput},
    core::math::Transform,
    error::{ApiError, ApiResult},
    query::Aabb,
    types::{MassData, Vec2},
};
use boxdd_sys::ffi;
use core::fmt;
use smallvec::SmallVec;

mod capsule;
mod chain_segment;
mod circle;
mod polygon;
mod segment;

/// Maximum number of vertices supported by a convex Box2D polygon.
pub const MAX_POLYGON_VERTICES: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;

const MAX_POLYGON_INPUT_POINTS: usize = MAX_POLYGON_VERTICES + 1;

const _: () = {
    assert!(core::mem::size_of::<Vec2>() == core::mem::size_of::<ffi::b2Vec2>());
    assert!(core::mem::align_of::<Vec2>() == core::mem::align_of::<ffi::b2Vec2>());
    assert!(core::mem::size_of::<ChainSegment>() == core::mem::size_of::<ffi::b2ChainSegment>());
    assert!(core::mem::align_of::<ChainSegment>() == core::mem::align_of::<ffi::b2ChainSegment>());
};

#[inline]
fn make_ray_input<VO: Into<Vec2>, VT: Into<Vec2>>(
    origin: VO,
    translation: VT,
) -> ffi::b2RayCastInput {
    let origin = origin.into();
    let translation = translation.into();
    let input = raw_ray_input(origin, translation);
    assert!(
        raw_ray_input_is_valid(&input),
        "ray input must be valid Box2D ray data, got origin={origin:?}, translation={translation:?}"
    );
    input
}

#[inline]
fn try_make_ray_input<VO: Into<Vec2>, VT: Into<Vec2>>(
    origin: VO,
    translation: VT,
) -> ApiResult<ffi::b2RayCastInput> {
    let origin = origin.into();
    let translation = translation.into();
    let input = raw_ray_input(origin, translation);
    geometry_is_valid_or_err(raw_ray_input_is_valid(&input))?;
    Ok(input)
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
fn raw_ray_input_is_valid(input: &ffi::b2RayCastInput) -> bool {
    unsafe { ffi::b2IsValidRay(input) }
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
        let point = point.into();
        if !point.is_valid() {
            return None;
        }
        pts.push(point.into_raw());
    }

    if pts.is_empty() || pts.len() > MAX_POLYGON_VERTICES {
        return None;
    }

    Some(pts)
}

#[inline]
fn compute_hull_from_points<I, P>(points: I) -> Option<ffi::b2Hull>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    let pts = collect_polygon_points(points)?;
    let hull = unsafe { ffi::b2ComputeHull(pts.as_ptr(), pts.len() as i32) };
    (hull.count > 0).then_some(hull)
}

#[inline]
fn geometry_scalar_is_non_negative_finite(value: f32) -> bool {
    crate::is_valid_float(value) && value >= 0.0
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
fn minimum_shape_segment_length_squared() -> f32 {
    let linear_slop = 0.005 * crate::length_units_per_meter();
    linear_slop * linear_slop
}

#[inline]
fn point_pair_has_minimum_separation(a: Vec2, b: Vec2) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dx * dx + dy * dy > minimum_shape_segment_length_squared()
}

#[inline]
fn geometry_is_valid_or_err(valid: bool) -> ApiResult<()> {
    if valid {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[track_caller]
fn assert_valid_geometry_vec2(name: &str, value: Vec2) {
    assert!(
        geometry_vec2_is_valid(value),
        "{name} must be a valid Box2D vector, got {:?}",
        value
    );
}

#[inline]
fn check_valid_geometry_vec2(value: Vec2) -> ApiResult<()> {
    geometry_is_valid_or_err(geometry_vec2_is_valid(value))
}

#[track_caller]
fn assert_non_negative_finite_density(density: f32) {
    assert!(
        geometry_density_is_valid(density),
        "density must be finite and >= 0.0, got {density}"
    );
}

#[inline]
fn check_non_negative_finite_density(density: f32) -> ApiResult<()> {
    geometry_is_valid_or_err(geometry_density_is_valid(density))
}

#[track_caller]
fn assert_positive_finite_polygon_scalar(name: &str, value: f32) {
    assert!(
        crate::is_valid_float(value) && value > 0.0,
        "{name} must be finite and > 0.0, got {value}"
    );
}

#[inline]
fn check_positive_finite_polygon_scalar(value: f32) -> ApiResult<()> {
    geometry_is_valid_or_err(crate::is_valid_float(value) && value > 0.0)
}

#[track_caller]
fn assert_non_negative_finite_polygon_scalar(name: &str, value: f32) {
    assert!(
        geometry_scalar_is_non_negative_finite(value),
        "{name} must be finite and >= 0.0, got {value}"
    );
}

#[inline]
fn check_non_negative_finite_polygon_scalar(value: f32) -> ApiResult<()> {
    geometry_is_valid_or_err(geometry_scalar_is_non_negative_finite(value))
}

#[track_caller]
fn assert_transform_valid(transform: Transform) {
    assert!(
        transform.is_valid(),
        "transform must be a valid Box2D transform, got {:?}",
        transform
    );
}

#[inline]
fn check_transform_valid(transform: Transform) -> ApiResult<()> {
    geometry_is_valid_or_err(transform.is_valid())
}

#[inline]
fn circle_helper_geometry_is_valid(circle: Circle) -> bool {
    circle.is_valid()
}

#[track_caller]
fn assert_circle_helper_geometry_valid(circle: Circle) {
    assert!(
        circle_helper_geometry_is_valid(circle),
        "circle must contain valid Box2D geometry, got {:?}",
        circle
    );
}

#[inline]
fn check_circle_helper_geometry_valid(circle: Circle) -> ApiResult<()> {
    geometry_is_valid_or_err(circle_helper_geometry_is_valid(circle))
}

#[inline]
fn segment_helper_geometry_is_valid(segment: Segment) -> bool {
    segment.point1.is_valid() && segment.point2.is_valid()
}

#[track_caller]
fn assert_segment_helper_geometry_valid(segment: Segment) {
    assert!(
        segment_helper_geometry_is_valid(segment),
        "segment must contain valid Box2D coordinates, got {:?}",
        segment
    );
}

#[inline]
fn check_segment_helper_geometry_valid(segment: Segment) -> ApiResult<()> {
    geometry_is_valid_or_err(segment_helper_geometry_is_valid(segment))
}

#[inline]
fn capsule_helper_geometry_is_valid(capsule: Capsule) -> bool {
    capsule.center1.is_valid()
        && capsule.center2.is_valid()
        && geometry_scalar_is_non_negative_finite(capsule.radius)
}

#[track_caller]
fn assert_capsule_helper_geometry_valid(capsule: Capsule) {
    assert!(
        capsule_helper_geometry_is_valid(capsule),
        "capsule must contain valid Box2D geometry, got {:?}",
        capsule
    );
}

#[inline]
fn check_capsule_helper_geometry_valid(capsule: Capsule) -> ApiResult<()> {
    geometry_is_valid_or_err(capsule_helper_geometry_is_valid(capsule))
}

#[inline]
fn polygon_helper_geometry_is_valid(polygon: Polygon) -> bool {
    polygon.is_valid()
}

#[track_caller]
fn assert_polygon_helper_geometry_valid(polygon: Polygon) {
    assert!(
        polygon_helper_geometry_is_valid(polygon),
        "polygon must contain valid Box2D geometry, got {:?}",
        polygon
    );
}

#[inline]
fn check_polygon_helper_geometry_valid(polygon: Polygon) -> ApiResult<()> {
    geometry_is_valid_or_err(polygon_helper_geometry_is_valid(polygon))
}

#[inline]
fn try_compute_hull_from_points<I, P>(points: I) -> ApiResult<ffi::b2Hull>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    compute_hull_from_points(points).ok_or(ApiError::InvalidArgument)
}

/// Circle geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

/// Line segment geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Segment {
    pub point1: Vec2,
    pub point2: Vec2,
}

/// One-sided chain segment geometry with ghost vertices on both ends.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ChainSegment {
    pub ghost1: Vec2,
    pub segment: Segment,
    pub ghost2: Vec2,
    #[cfg_attr(feature = "serde", serde(skip, default))]
    chain_id: i32,
}

/// Capsule geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Capsule {
    pub center1: Vec2,
    pub center2: Vec2,
    pub radius: f32,
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
pub fn circle<C: Into<Vec2>>(center: C, radius: f32) -> Circle {
    Circle::new(center, radius)
}

/// Segment helper.
#[inline]
pub fn segment<P1: Into<Vec2>, P2: Into<Vec2>>(point1: P1, point2: P2) -> Segment {
    Segment::new(point1, point2)
}

/// Chain segment helper.
#[inline]
pub fn chain_segment<G1, P1, P2, G2>(ghost1: G1, point1: P1, point2: P2, ghost2: G2) -> ChainSegment
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
pub fn capsule<C1: Into<Vec2>, C2: Into<Vec2>>(center1: C1, center2: C2, radius: f32) -> Capsule {
    Capsule::new(center1, center2, radius)
}

/// Axis-aligned box polygon helper.
#[inline]
pub fn box_polygon(half_width: f32, half_height: f32) -> Polygon {
    Polygon::box_polygon(half_width, half_height)
}

/// Recoverable axis-aligned box polygon helper.
#[inline]
pub fn try_box_polygon(half_width: f32, half_height: f32) -> ApiResult<Polygon> {
    Polygon::try_box_polygon(half_width, half_height)
}

/// Axis-aligned square polygon helper.
#[inline]
pub fn square_polygon(half_width: f32) -> Polygon {
    Polygon::square_polygon(half_width)
}

/// Recoverable axis-aligned square polygon helper.
#[inline]
pub fn try_square_polygon(half_width: f32) -> ApiResult<Polygon> {
    Polygon::try_square_polygon(half_width)
}

/// Axis-aligned rounded box polygon helper.
#[inline]
pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Polygon {
    Polygon::rounded_box_polygon(half_width, half_height, radius)
}

/// Recoverable axis-aligned rounded box polygon helper.
#[inline]
pub fn try_rounded_box_polygon(
    half_width: f32,
    half_height: f32,
    radius: f32,
) -> ApiResult<Polygon> {
    Polygon::try_rounded_box_polygon(half_width, half_height, radius)
}

/// Offset box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn offset_box_polygon(half_width: f32, half_height: f32, transform: Transform) -> Polygon {
    Polygon::offset_box_polygon(half_width, half_height, transform)
}

/// Recoverable offset box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn try_offset_box_polygon(
    half_width: f32,
    half_height: f32,
    transform: Transform,
) -> ApiResult<Polygon> {
    Polygon::try_offset_box_polygon(half_width, half_height, transform)
}

/// Offset rounded box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn offset_rounded_box_polygon(
    half_width: f32,
    half_height: f32,
    radius: f32,
    transform: Transform,
) -> Polygon {
    Polygon::offset_rounded_box_polygon(half_width, half_height, radius, transform)
}

/// Recoverable offset rounded box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn try_offset_rounded_box_polygon(
    half_width: f32,
    half_height: f32,
    radius: f32,
    transform: Transform,
) -> ApiResult<Polygon> {
    Polygon::try_offset_rounded_box_polygon(half_width, half_height, radius, transform)
}

/// Build a polygon from arbitrary points by computing a convex hull.
#[inline]
pub fn polygon_from_points<I, P>(points: I, radius: f32) -> Option<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::from_points(points, radius)
}

/// Recoverably build a polygon from arbitrary points by computing a convex hull.
#[inline]
pub fn try_polygon_from_points<I, P>(points: I, radius: f32) -> ApiResult<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::try_from_points(points, radius)
}

/// Build an offset polygon from arbitrary points by computing a convex hull first.
#[inline]
pub fn offset_polygon_from_points<I, P>(
    points: I,
    radius: f32,
    transform: Transform,
) -> Option<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::offset_from_points(points, radius, transform)
}

/// Recoverably build an offset polygon from arbitrary points by computing a convex hull first.
#[inline]
pub fn try_offset_polygon_from_points<I, P>(
    points: I,
    radius: f32,
    transform: Transform,
) -> ApiResult<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::try_offset_from_points(points, radius, transform)
}

/// Check whether a point set can produce a valid Box2D convex hull.
#[inline]
pub fn polygon_hull_is_valid<I, P>(points: I) -> bool
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::hull_is_valid(points)
}
