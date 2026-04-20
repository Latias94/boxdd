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
    pub fn mass_data(self, density: f32) -> MassData {
        let raw: ffi::b2Circle = self.into();
        unsafe { ffi::b2ComputeCircleMass(&raw, density) }.into()
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw: ffi::b2Circle = self.into();
        unsafe { ffi::b2ComputeCircleAABB(&raw, transform.into()) }.into()
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let raw: ffi::b2Circle = self.into();
        unsafe { ffi::b2PointInCircle(&raw, point.into().into()) }
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let raw: ffi::b2Circle = self.into();
        let input = make_ray_input(origin, translation);
        unsafe { ffi::b2RayCastCircle(&raw, &input) }.into()
    }
}

impl From<Circle> for ffi::b2Circle {
    #[inline]
    fn from(circle: Circle) -> Self {
        Self {
            center: circle.center.into(),
            radius: circle.radius,
        }
    }
}

impl From<ffi::b2Circle> for Circle {
    #[inline]
    fn from(circle: ffi::b2Circle) -> Self {
        Self {
            center: circle.center.into(),
            radius: circle.radius,
        }
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
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw: ffi::b2Segment = self.into();
        unsafe { ffi::b2ComputeSegmentAABB(&raw, transform.into()) }.into()
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
        one_sided: bool,
    ) -> CastOutput {
        let raw: ffi::b2Segment = self.into();
        let input = make_ray_input(origin, translation);
        unsafe { ffi::b2RayCastSegment(&raw, &input, one_sided) }.into()
    }
}

impl From<Segment> for ffi::b2Segment {
    #[inline]
    fn from(segment: Segment) -> Self {
        Self {
            point1: segment.point1.into(),
            point2: segment.point2.into(),
        }
    }
}

impl From<ffi::b2Segment> for Segment {
    #[inline]
    fn from(segment: ffi::b2Segment) -> Self {
        Self {
            point1: segment.point1.into(),
            point2: segment.point2.into(),
        }
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

impl From<ChainSegment> for ffi::b2ChainSegment {
    #[inline]
    fn from(segment: ChainSegment) -> Self {
        Self {
            ghost1: segment.ghost1.into(),
            segment: segment.segment.into(),
            ghost2: segment.ghost2.into(),
            chainId: segment.chain_id,
        }
    }
}

impl From<ffi::b2ChainSegment> for ChainSegment {
    #[inline]
    fn from(segment: ffi::b2ChainSegment) -> Self {
        Self {
            ghost1: segment.ghost1.into(),
            segment: segment.segment.into(),
            ghost2: segment.ghost2.into(),
            chain_id: segment.chainId,
        }
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
    pub fn mass_data(self, density: f32) -> MassData {
        let raw: ffi::b2Capsule = self.into();
        unsafe { ffi::b2ComputeCapsuleMass(&raw, density) }.into()
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw: ffi::b2Capsule = self.into();
        unsafe { ffi::b2ComputeCapsuleAABB(&raw, transform.into()) }.into()
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let raw: ffi::b2Capsule = self.into();
        unsafe { ffi::b2PointInCapsule(&raw, point.into().into()) }
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let raw: ffi::b2Capsule = self.into();
        let input = make_ray_input(origin, translation);
        unsafe { ffi::b2RayCastCapsule(&raw, &input) }.into()
    }
}

impl From<Capsule> for ffi::b2Capsule {
    #[inline]
    fn from(capsule: Capsule) -> Self {
        Self {
            center1: capsule.center1.into(),
            center2: capsule.center2.into(),
            radius: capsule.radius,
        }
    }
}

impl From<ffi::b2Capsule> for Capsule {
    #[inline]
    fn from(capsule: ffi::b2Capsule) -> Self {
        Self {
            center1: capsule.center1.into(),
            center2: capsule.center2.into(),
            radius: capsule.radius,
        }
    }
}

/// Convex polygon geometry in local shape space.
///
/// Construct polygons with helpers such as [`box_polygon`] or [`polygon_from_points`]
/// instead of filling raw vertices manually.
#[doc(alias = "polygon")]
#[derive(Copy, Clone)]
pub struct Polygon {
    raw: ffi::b2Polygon,
}

impl Polygon {
    #[inline]
    pub fn new(raw: ffi::b2Polygon) -> Self {
        Self { raw }
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
    pub fn box_polygon(half_width: f32, half_height: f32) -> Self {
        unsafe { ffi::b2MakeBox(half_width, half_height) }.into()
    }

    #[inline]
    pub fn from_points<I, P>(points: I, radius: f32) -> Option<Self>
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

        let hull = unsafe { ffi::b2ComputeHull(pts.as_ptr(), pts.len() as i32) };
        if hull.count <= 0 {
            return None;
        }

        Some(unsafe { ffi::b2MakePolygon(&hull, radius) }.into())
    }

    #[inline]
    pub fn transformed(self, transform: Transform) -> Self {
        unsafe { ffi::b2TransformPolygon(transform.into(), &self.raw) }.into()
    }

    #[inline]
    pub fn mass_data(self, density: f32) -> MassData {
        let raw: ffi::b2Polygon = self.into();
        unsafe { ffi::b2ComputePolygonMass(&raw, density) }.into()
    }

    #[inline]
    pub fn aabb(self, transform: Transform) -> Aabb {
        let raw: ffi::b2Polygon = self.into();
        unsafe { ffi::b2ComputePolygonAABB(&raw, transform.into()) }.into()
    }

    #[inline]
    pub fn contains_point<P: Into<Vec2>>(self, point: P) -> bool {
        let raw: ffi::b2Polygon = self.into();
        unsafe { ffi::b2PointInPolygon(&raw, point.into().into()) }
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        let raw: ffi::b2Polygon = self.into();
        let input = make_ray_input(origin, translation);
        unsafe { ffi::b2RayCastPolygon(&raw, &input) }.into()
    }

    #[inline]
    pub(crate) fn raw(self) -> ffi::b2Polygon {
        self.raw
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

impl From<Polygon> for ffi::b2Polygon {
    #[inline]
    fn from(polygon: Polygon) -> Self {
        polygon.raw
    }
}

impl From<ffi::b2Polygon> for Polygon {
    #[inline]
    fn from(polygon: ffi::b2Polygon) -> Self {
        Self { raw: polygon }
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

/// Build a polygon from arbitrary points by computing a convex hull.
#[inline]
pub fn polygon_from_points<I, P>(points: I, radius: f32) -> Option<Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    Polygon::from_points(points, radius)
}
