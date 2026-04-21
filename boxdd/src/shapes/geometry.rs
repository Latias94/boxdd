use crate::{
    collision::CastOutput,
    core::math::Transform,
    error::{ApiError, ApiResult},
    query::Aabb,
    types::{MassData, Vec2},
};
use boxdd_sys::ffi;
use core::fmt;
use smallvec::SmallVec;

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
    assert_valid_geometry_vec2("origin", origin);
    assert_valid_geometry_vec2("translation", translation);
    raw_ray_input(origin, translation)
}

#[inline]
fn try_make_ray_input<VO: Into<Vec2>, VT: Into<Vec2>>(
    origin: VO,
    translation: VT,
) -> ApiResult<ffi::b2RayCastInput> {
    let origin = origin.into();
    let translation = translation.into();
    check_valid_geometry_vec2(origin)?;
    check_valid_geometry_vec2(translation)?;
    Ok(raw_ray_input(origin, translation))
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

#[track_caller]
fn assert_non_negative_finite_polygon_scalar(name: &str, value: f32) {
    assert!(
        geometry_scalar_is_non_negative_finite(value),
        "{name} must be finite and >= 0.0, got {value}"
    );
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

/// Circle geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

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

/// Line segment geometry in local shape space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Segment {
    pub point1: Vec2,
    pub point2: Vec2,
}

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

impl ChainSegment {
    #[inline]
    pub fn new<G1, P1, P2, G2>(ghost1: G1, point1: P1, point2: P2, ghost2: G2) -> Self
    where
        G1: Into<Vec2>,
        P1: Into<Vec2>,
        P2: Into<Vec2>,
        G2: Into<Vec2>,
    {
        Self {
            ghost1: ghost1.into(),
            segment: Segment::new(point1, point2),
            ghost2: ghost2.into(),
            chain_id: 0,
        }
    }

    #[inline]
    pub fn from_segment<G1: Into<Vec2>, G2: Into<Vec2>>(
        ghost1: G1,
        segment: Segment,
        ghost2: G2,
    ) -> Self {
        Self {
            ghost1: ghost1.into(),
            segment,
            ghost2: ghost2.into(),
            chain_id: 0,
        }
    }

    #[inline]
    pub fn chain_id_raw(self) -> i32 {
        self.chain_id
    }

    #[inline]
    /// Construct from the raw Box2D geometry value.
    pub fn from_raw(segment: ffi::b2ChainSegment) -> Self {
        Self {
            ghost1: Vec2::from_raw(segment.ghost1),
            segment: Segment::from_raw(segment.segment),
            ghost2: Vec2::from_raw(segment.ghost2),
            chain_id: segment.chainId,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2ChainSegment {
        ffi::b2ChainSegment {
            ghost1: self.ghost1.into_raw(),
            segment: self.segment.into_raw(),
            ghost2: self.ghost2.into_raw(),
            chainId: self.chain_id,
        }
    }

    #[inline]
    /// Validate this chain segment for standalone collision use.
    pub fn is_valid(self) -> bool {
        self.ghost1.is_valid() && self.segment.is_valid() && self.ghost2.is_valid()
    }

    #[inline]
    /// Validate this chain segment for standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        geometry_is_valid_or_err(self.is_valid())
    }
}

impl fmt::Debug for ChainSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainSegment")
            .field("ghost1", &self.ghost1)
            .field("segment", &self.segment)
            .field("ghost2", &self.ghost2)
            .field("chain_id_raw", &self.chain_id)
            .finish()
    }
}

impl PartialEq for ChainSegment {
    fn eq(&self, other: &Self) -> bool {
        self.ghost1 == other.ghost1 && self.segment == other.segment && self.ghost2 == other.ghost2
    }
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
    pub fn box_polygon(half_width: f32, half_height: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        Self::from_raw(unsafe { ffi::b2MakeBox(half_width, half_height) })
    }

    #[inline]
    pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Self {
        assert_positive_finite_polygon_scalar("half_width", half_width);
        assert_positive_finite_polygon_scalar("half_height", half_height);
        assert_non_negative_finite_polygon_scalar("radius", radius);
        Self::from_raw(unsafe { ffi::b2MakeRoundedBox(half_width, half_height, radius) })
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
    pub fn from_points<I, P>(points: I, radius: f32) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        if !geometry_scalar_is_non_negative_finite(radius) {
            return None;
        }
        let hull = compute_hull_from_points(points)?;
        Some(Self::from_raw(unsafe { ffi::b2MakePolygon(&hull, radius) }))
    }

    #[inline]
    pub fn offset_from_points<I, P>(points: I, radius: f32, transform: Transform) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        if !geometry_scalar_is_non_negative_finite(radius) || !transform.is_valid() {
            return None;
        }
        let hull = compute_hull_from_points(points)?;
        Some(Self::from_raw(unsafe {
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

/// Axis-aligned square polygon helper.
#[inline]
pub fn square_polygon(half_width: f32) -> Polygon {
    Polygon::square_polygon(half_width)
}

/// Axis-aligned rounded box polygon helper.
#[inline]
pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Polygon {
    Polygon::rounded_box_polygon(half_width, half_height, radius)
}

/// Offset box polygon helper using the crate's `Transform` vocabulary.
#[inline]
pub fn offset_box_polygon(half_width: f32, half_height: f32, transform: Transform) -> Polygon {
    Polygon::offset_box_polygon(half_width, half_height, transform)
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

/// Build a polygon from arbitrary points by computing a convex hull.
#[inline]
pub fn polygon_from_points<I, P>(points: I, radius: f32) -> Option<Polygon>
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
) -> Option<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::offset_from_points(points, radius, transform)
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
