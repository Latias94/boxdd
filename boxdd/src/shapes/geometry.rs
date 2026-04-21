use crate::{
    collision::CastOutput,
    core::math::Transform,
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
    ffi::b2RayCastInput {
        origin: origin.into().into(),
        translation: translation.into().into(),
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
        pts.push(point.into().into());
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
            center: circle.center.into(),
            radius: circle.radius,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Circle {
        ffi::b2Circle {
            center: self.center.into(),
            radius: self.radius,
        }
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        let raw = self.into_raw();
        MassData::from_raw(unsafe { ffi::b2ComputeCircleMass(&raw, density) })
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputeCircleAABB(&raw, transform.into()) })
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let raw = self.into_raw();
        unsafe { ffi::b2PointInCircle(&raw, point.into().into()) }
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastCircle(&raw, &input) })
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
            point1: segment.point1.into(),
            point2: segment.point2.into(),
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Segment {
        ffi::b2Segment {
            point1: self.point1.into(),
            point2: self.point2.into(),
        }
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputeSegmentAABB(&raw, transform.into()) })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
        one_sided: bool,
    ) -> CastOutput {
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastSegment(&raw, &input, one_sided) })
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
            ghost1: segment.ghost1.into(),
            segment: Segment::from_raw(segment.segment),
            ghost2: segment.ghost2.into(),
            chain_id: segment.chainId,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2ChainSegment {
        ffi::b2ChainSegment {
            ghost1: self.ghost1.into(),
            segment: self.segment.into_raw(),
            ghost2: self.ghost2.into(),
            chainId: self.chain_id,
        }
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
            center1: capsule.center1.into(),
            center2: capsule.center2.into(),
            radius: capsule.radius,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Capsule {
        ffi::b2Capsule {
            center1: self.center1.into(),
            center2: self.center2.into(),
            radius: self.radius,
        }
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        let raw = self.into_raw();
        MassData::from_raw(unsafe { ffi::b2ComputeCapsuleMass(&raw, density) })
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputeCapsuleAABB(&raw, transform.into()) })
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let raw = self.into_raw();
        unsafe { ffi::b2PointInCapsule(&raw, point.into().into()) }
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastCapsule(&raw, &input) })
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
        self.raw.centroid.into()
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.raw.radius
    }

    #[inline]
    pub fn square_polygon(half_width: f32) -> Self {
        Self::from_raw(unsafe { ffi::b2MakeSquare(half_width) })
    }

    #[inline]
    pub fn box_polygon(half_width: f32, half_height: f32) -> Self {
        Self::from_raw(unsafe { ffi::b2MakeBox(half_width, half_height) })
    }

    #[inline]
    pub fn rounded_box_polygon(half_width: f32, half_height: f32, radius: f32) -> Self {
        Self::from_raw(unsafe { ffi::b2MakeRoundedBox(half_width, half_height, radius) })
    }

    #[inline]
    pub fn offset_box_polygon(half_width: f32, half_height: f32, transform: Transform) -> Self {
        Self::from_raw(unsafe {
            ffi::b2MakeOffsetBox(
                half_width,
                half_height,
                transform.position().into(),
                transform.rotation().into(),
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
        Self::from_raw(unsafe {
            ffi::b2MakeOffsetRoundedBox(
                half_width,
                half_height,
                transform.position().into(),
                transform.rotation().into(),
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
        let hull = compute_hull_from_points(points)?;
        Some(Self::from_raw(unsafe { ffi::b2MakePolygon(&hull, radius) }))
    }

    #[inline]
    pub fn offset_from_points<I, P>(points: I, radius: f32, transform: Transform) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let hull = compute_hull_from_points(points)?;
        Some(Self::from_raw(unsafe {
            if radius == 0.0 {
                ffi::b2MakeOffsetPolygon(
                    &hull,
                    transform.position().into(),
                    transform.rotation().into(),
                )
            } else {
                ffi::b2MakeOffsetRoundedPolygon(
                    &hull,
                    transform.position().into(),
                    transform.rotation().into(),
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
        Self::from_raw(unsafe { ffi::b2TransformPolygon(transform.into(), &self.raw) })
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        let raw = self.into_raw();
        MassData::from_raw(unsafe { ffi::b2ComputePolygonMass(&raw, density) })
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw = self.into_raw();
        Aabb::from_raw(unsafe { ffi::b2ComputePolygonAABB(&raw, transform.into()) })
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let raw = self.into_raw();
        unsafe { ffi::b2PointInPolygon(&raw, point.into().into()) }
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let raw = self.into_raw();
        let input = make_ray_input(origin, translation);
        CastOutput::from_raw(unsafe { ffi::b2RayCastPolygon(&raw, &input) })
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
