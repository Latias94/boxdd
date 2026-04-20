//! Standalone low-level collision geometry helpers.
//!
//! This module wraps Box2D's standalone collision algorithms without exposing raw FFI
//! structs. It is intentionally more explicit than the high-level `World` query API and
//! is useful when you want to run geometric tests or contact-manifold generation without
//! a world instance.

use crate::{
    core::math::{Rot, Transform},
    query::Aabb,
    shapes::{Capsule, ChainSegment, Circle, Polygon, Segment},
    types::{Manifold, Vec2},
};
use boxdd_sys::ffi;
use core::fmt;

/// Maximum number of points supported by a Box2D shape proxy.
pub const MAX_SHAPE_PROXY_POINTS: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;

const _: () = {
    assert!(core::mem::size_of::<Vec2>() == core::mem::size_of::<ffi::b2Vec2>());
    assert!(core::mem::align_of::<Vec2>() == core::mem::align_of::<ffi::b2Vec2>());
};

#[inline]
fn ray_cast_axis(
    origin: f32,
    translation: f32,
    lower: f32,
    upper: f32,
    enter_normal: Vec2,
    exit_normal: Vec2,
    tmin: &mut f32,
    tmax: &mut f32,
    normal: &mut Vec2,
) -> bool {
    if translation.abs() < f32::EPSILON {
        return lower <= origin && origin <= upper;
    }

    let inv_translation = 1.0 / translation;
    let mut t1 = (lower - origin) * inv_translation;
    let mut t2 = (upper - origin) * inv_translation;
    let mut n1 = enter_normal;
    let mut n2 = exit_normal;

    if t1 > t2 {
        core::mem::swap(&mut t1, &mut t2);
        core::mem::swap(&mut n1, &mut n2);
    }

    if t1 > *tmin {
        *tmin = t1;
        *normal = n1;
    }

    if t2 < *tmax {
        *tmax = t2;
    }

    *tmin <= *tmax
}

/// A Box2D point-cloud proxy used by distance, shape-cast, and TOI algorithms.
///
/// Returns `None` from [`ShapeProxy::new`] when the iterator is empty or contains more
/// than [`MAX_SHAPE_PROXY_POINTS`] points.
#[doc(alias = "shape_proxy")]
#[derive(Copy, Clone)]
pub struct ShapeProxy {
    raw: ffi::b2ShapeProxy,
}

impl ShapeProxy {
    /// Build a proxy from `1..=MAX_SHAPE_PROXY_POINTS` points and an external radius.
    pub fn new<I, P>(points: I, radius: f32) -> Option<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let mut raw_points = [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_SHAPE_PROXY_POINTS];
        let mut count = 0usize;

        for point in points {
            if count == MAX_SHAPE_PROXY_POINTS {
                return None;
            }
            raw_points[count] = point.into().into();
            count += 1;
        }

        if count == 0 {
            return None;
        }

        let raw = unsafe { ffi::b2MakeProxy(raw_points.as_ptr(), count as i32, radius) };
        Some(Self { raw })
    }

    /// The points stored in this proxy.
    #[inline]
    pub fn points(&self) -> &[Vec2] {
        let count = self.count();
        unsafe { core::slice::from_raw_parts(self.raw.points.as_ptr().cast::<Vec2>(), count) }
    }

    /// The number of points stored in this proxy.
    #[inline]
    pub fn count(&self) -> usize {
        self.raw.count.clamp(0, MAX_SHAPE_PROXY_POINTS as i32) as usize
    }

    /// The proxy's external radius.
    #[inline]
    pub fn radius(&self) -> f32 {
        self.raw.radius
    }

    #[inline]
    fn raw(self) -> ffi::b2ShapeProxy {
        self.raw
    }
}

impl fmt::Debug for ShapeProxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapeProxy")
            .field("points", &self.points())
            .field("radius", &self.radius())
            .finish()
    }
}

/// Warm-start cache for repeated GJK distance calls.
#[doc(alias = "simplex_cache")]
#[derive(Copy, Clone)]
pub struct SimplexCache {
    raw: ffi::b2SimplexCache,
}

impl Default for SimplexCache {
    fn default() -> Self {
        Self {
            raw: ffi::b2SimplexCache {
                count: 0,
                indexA: [0; 3],
                indexB: [0; 3],
            },
        }
    }
}

impl SimplexCache {
    /// Create a zeroed cache for the first distance query.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the cache to its initial zeroed state.
    #[inline]
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The number of cached simplex points.
    #[inline]
    pub fn count(&self) -> usize {
        self.raw.count.min(3) as usize
    }

    /// Cached simplex indices for shape A.
    #[inline]
    pub fn index_a(&self) -> &[u8] {
        &self.raw.indexA[..self.count()]
    }

    /// Cached simplex indices for shape B.
    #[inline]
    pub fn index_b(&self) -> &[u8] {
        &self.raw.indexB[..self.count()]
    }

    #[inline]
    fn raw_mut(&mut self) -> *mut ffi::b2SimplexCache {
        &mut self.raw
    }
}

impl fmt::Debug for SimplexCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimplexCache")
            .field("count", &self.count())
            .field("index_a", &self.index_a())
            .field("index_b", &self.index_b())
            .finish()
    }
}

/// Result of [`segment_distance`].
#[doc(alias = "segment_distance_result")]
#[derive(Copy, Clone, Debug)]
pub struct SegmentDistanceResult {
    pub closest1: Vec2,
    pub closest2: Vec2,
    pub fraction1: f32,
    pub fraction2: f32,
    pub distance_squared: f32,
}

impl SegmentDistanceResult {
    #[inline]
    pub fn from_raw(raw: ffi::b2SegmentDistanceResult) -> Self {
        Self {
            closest1: raw.closest1.into(),
            closest2: raw.closest2.into(),
            fraction1: raw.fraction1,
            fraction2: raw.fraction2,
            distance_squared: raw.distanceSquared,
        }
    }
}

/// Low-level ray-cast or shape-cast output.
#[doc(alias = "cast_output")]
#[derive(Copy, Clone, Debug)]
pub struct CastOutput {
    pub normal: Vec2,
    pub point: Vec2,
    pub fraction: f32,
    pub iterations: i32,
    pub hit: bool,
}

impl CastOutput {
    pub const MISS: Self = Self {
        normal: Vec2::ZERO,
        point: Vec2::ZERO,
        fraction: 0.0,
        iterations: 0,
        hit: false,
    };

    #[inline]
    pub fn from_raw(raw: ffi::b2CastOutput) -> Self {
        Self {
            normal: raw.normal.into(),
            point: raw.point.into(),
            fraction: raw.fraction,
            iterations: raw.iterations,
            hit: raw.hit,
        }
    }
}

/// Input for [`shape_distance`].
#[doc(alias = "distance_input")]
#[derive(Copy, Clone, Debug)]
pub struct DistanceInput {
    pub proxy_a: ShapeProxy,
    pub proxy_b: ShapeProxy,
    pub transform_a: Transform,
    pub transform_b: Transform,
    pub use_radii: bool,
}

impl DistanceInput {
    /// Build distance input with `use_radii = false`.
    #[inline]
    pub fn new(
        proxy_a: ShapeProxy,
        proxy_b: ShapeProxy,
        transform_a: Transform,
        transform_b: Transform,
    ) -> Self {
        Self {
            proxy_a,
            proxy_b,
            transform_a,
            transform_b,
            use_radii: false,
        }
    }

    /// Set whether proxy radii should affect the distance result.
    #[inline]
    pub fn with_radii(mut self, use_radii: bool) -> Self {
        self.use_radii = use_radii;
        self
    }
}

impl From<DistanceInput> for ffi::b2DistanceInput {
    #[inline]
    fn from(input: DistanceInput) -> Self {
        Self {
            proxyA: input.proxy_a.raw(),
            proxyB: input.proxy_b.raw(),
            transformA: input.transform_a.into(),
            transformB: input.transform_b.into(),
            useRadii: input.use_radii,
        }
    }
}

/// Output from [`shape_distance`].
#[doc(alias = "distance_output")]
#[derive(Copy, Clone, Debug)]
pub struct DistanceOutput {
    pub point_a: Vec2,
    pub point_b: Vec2,
    pub normal: Vec2,
    pub distance: f32,
    pub iterations: i32,
    pub simplex_count: i32,
}

impl DistanceOutput {
    #[inline]
    pub fn from_raw(raw: ffi::b2DistanceOutput) -> Self {
        Self {
            point_a: raw.pointA.into(),
            point_b: raw.pointB.into(),
            normal: raw.normal.into(),
            distance: raw.distance,
            iterations: raw.iterations,
            simplex_count: raw.simplexCount,
        }
    }
}

/// Input for [`shape_cast`].
#[doc(alias = "shape_cast_pair_input")]
#[derive(Copy, Clone, Debug)]
pub struct ShapeCastPairInput {
    pub proxy_a: ShapeProxy,
    pub proxy_b: ShapeProxy,
    pub transform_a: Transform,
    pub transform_b: Transform,
    pub translation_b: Vec2,
    pub max_fraction: f32,
    pub can_encroach: bool,
}

impl ShapeCastPairInput {
    /// Build a shape cast where shape B moves by `translation_b`.
    #[inline]
    pub fn new<V: Into<Vec2>>(
        proxy_a: ShapeProxy,
        proxy_b: ShapeProxy,
        transform_a: Transform,
        transform_b: Transform,
        translation_b: V,
    ) -> Self {
        Self {
            proxy_a,
            proxy_b,
            transform_a,
            transform_b,
            translation_b: translation_b.into(),
            max_fraction: 1.0,
            can_encroach: false,
        }
    }

    /// Limit the portion of `translation_b` considered by the cast.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction;
        self
    }

    /// Allow shapes with radius to encroach slightly when initially touching.
    #[inline]
    pub fn with_can_encroach(mut self, can_encroach: bool) -> Self {
        self.can_encroach = can_encroach;
        self
    }
}

impl From<ShapeCastPairInput> for ffi::b2ShapeCastPairInput {
    #[inline]
    fn from(input: ShapeCastPairInput) -> Self {
        Self {
            proxyA: input.proxy_a.raw(),
            proxyB: input.proxy_b.raw(),
            transformA: input.transform_a.into(),
            transformB: input.transform_b.into(),
            translationB: input.translation_b.into(),
            maxFraction: input.max_fraction,
            canEncroach: input.can_encroach,
        }
    }
}

/// Sweep input used by continuous collision algorithms.
#[doc(alias = "sweep")]
#[derive(Copy, Clone, Debug)]
pub struct Sweep {
    pub local_center: Vec2,
    pub c1: Vec2,
    pub c2: Vec2,
    pub q1: Rot,
    pub q2: Rot,
}

impl Sweep {
    #[inline]
    pub fn new<LC: Into<Vec2>, C1: Into<Vec2>, C2: Into<Vec2>>(
        local_center: LC,
        c1: C1,
        c2: C2,
        q1: Rot,
        q2: Rot,
    ) -> Self {
        Self {
            local_center: local_center.into(),
            c1: c1.into(),
            c2: c2.into(),
            q1,
            q2,
        }
    }

    /// Evaluate the sweep transform at `time` in the `[0, 1]` interval.
    #[inline]
    pub fn transform_at(self, time: f32) -> Transform {
        let raw: ffi::b2Sweep = self.into();
        unsafe { ffi::b2GetSweepTransform(&raw, time) }.into()
    }
}

impl From<Sweep> for ffi::b2Sweep {
    #[inline]
    fn from(sweep: Sweep) -> Self {
        Self {
            localCenter: sweep.local_center.into(),
            c1: sweep.c1.into(),
            c2: sweep.c2.into(),
            q1: sweep.q1.into(),
            q2: sweep.q2.into(),
        }
    }
}

/// Input for [`time_of_impact`].
#[doc(alias = "toi_input")]
#[derive(Copy, Clone, Debug)]
pub struct ToiInput {
    pub proxy_a: ShapeProxy,
    pub proxy_b: ShapeProxy,
    pub sweep_a: Sweep,
    pub sweep_b: Sweep,
    pub max_fraction: f32,
}

impl ToiInput {
    /// Build TOI input with `max_fraction = 1.0`.
    #[inline]
    pub fn new(proxy_a: ShapeProxy, proxy_b: ShapeProxy, sweep_a: Sweep, sweep_b: Sweep) -> Self {
        Self {
            proxy_a,
            proxy_b,
            sweep_a,
            sweep_b,
            max_fraction: 1.0,
        }
    }

    /// Limit the sweep interval to `[0, max_fraction]`.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction;
        self
    }
}

impl From<ToiInput> for ffi::b2TOIInput {
    #[inline]
    fn from(input: ToiInput) -> Self {
        Self {
            proxyA: input.proxy_a.raw(),
            proxyB: input.proxy_b.raw(),
            sweepA: input.sweep_a.into(),
            sweepB: input.sweep_b.into(),
            maxFraction: input.max_fraction,
        }
    }
}

/// Result state from [`time_of_impact`].
#[doc(alias = "toi_state")]
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ToiState {
    Unknown = ffi::b2TOIState_b2_toiStateUnknown,
    Failed = ffi::b2TOIState_b2_toiStateFailed,
    Overlapped = ffi::b2TOIState_b2_toiStateOverlapped,
    Hit = ffi::b2TOIState_b2_toiStateHit,
    Separated = ffi::b2TOIState_b2_toiStateSeparated,
}

impl ToiState {
    #[inline]
    pub const fn from_raw(raw: ffi::b2TOIState) -> Self {
        match raw {
            ffi::b2TOIState_b2_toiStateFailed => Self::Failed,
            ffi::b2TOIState_b2_toiStateOverlapped => Self::Overlapped,
            ffi::b2TOIState_b2_toiStateHit => Self::Hit,
            ffi::b2TOIState_b2_toiStateSeparated => Self::Separated,
            _ => Self::Unknown,
        }
    }
}

/// Output from [`time_of_impact`].
#[doc(alias = "toi_output")]
#[derive(Copy, Clone, Debug)]
pub struct ToiOutput {
    pub state: ToiState,
    pub point: Vec2,
    pub normal: Vec2,
    pub fraction: f32,
}

impl ToiOutput {
    #[inline]
    pub fn from_raw(raw: ffi::b2TOIOutput) -> Self {
        Self {
            state: ToiState::from_raw(raw.state),
            point: raw.point.into(),
            normal: raw.normal.into(),
            fraction: raw.fraction,
        }
    }
}

/// Compute the closest points between two line segments.
pub fn segment_distance<P1, Q1, P2, Q2>(p1: P1, q1: Q1, p2: P2, q2: Q2) -> SegmentDistanceResult
where
    P1: Into<Vec2>,
    Q1: Into<Vec2>,
    P2: Into<Vec2>,
    Q2: Into<Vec2>,
{
    SegmentDistanceResult::from_raw(unsafe {
        ffi::b2SegmentDistance(
            p1.into().into(),
            q1.into().into(),
            p2.into().into(),
            q2.into().into(),
        )
    })
}

/// Compute the closest distance between two shape proxies.
pub fn shape_distance(input: DistanceInput, cache: &mut SimplexCache) -> DistanceOutput {
    let raw_input: ffi::b2DistanceInput = input.into();
    DistanceOutput::from_raw(unsafe {
        ffi::b2ShapeDistance(&raw_input, cache.raw_mut(), core::ptr::null_mut(), 0)
    })
}

/// Cast shape B against shape A.
pub fn shape_cast(input: ShapeCastPairInput) -> CastOutput {
    let raw_input: ffi::b2ShapeCastPairInput = input.into();
    CastOutput::from_raw(unsafe { ffi::b2ShapeCast(&raw_input) })
}

/// Compute the time of impact between two moving shape proxies.
pub fn time_of_impact(input: ToiInput) -> ToiOutput {
    let raw_input: ffi::b2TOIInput = input.into();
    ToiOutput::from_raw(unsafe { ffi::b2TimeOfImpact(&raw_input) })
}

/// Compute the contact manifold between two circles.
#[doc(alias = "b2CollideCircles")]
pub fn collide_circles(
    circle_a: Circle,
    transform_a: Transform,
    circle_b: Circle,
    transform_b: Transform,
) -> Manifold {
    let raw_a = circle_a.into_raw();
    let raw_b = circle_b.into_raw();
    unsafe { ffi::b2CollideCircles(&raw_a, transform_a.into(), &raw_b, transform_b.into()) }.into()
}

/// Compute the contact manifold between a capsule and a circle.
#[doc(alias = "b2CollideCapsuleAndCircle")]
pub fn collide_capsule_and_circle(
    capsule_a: Capsule,
    transform_a: Transform,
    circle_b: Circle,
    transform_b: Transform,
) -> Manifold {
    let raw_a = capsule_a.into_raw();
    let raw_b = circle_b.into_raw();
    unsafe {
        ffi::b2CollideCapsuleAndCircle(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between a segment and a circle.
#[doc(alias = "b2CollideSegmentAndCircle")]
pub fn collide_segment_and_circle(
    segment_a: Segment,
    transform_a: Transform,
    circle_b: Circle,
    transform_b: Transform,
) -> Manifold {
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    unsafe {
        ffi::b2CollideSegmentAndCircle(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between a polygon and a circle.
#[doc(alias = "b2CollidePolygonAndCircle")]
pub fn collide_polygon_and_circle(
    polygon_a: Polygon,
    transform_a: Transform,
    circle_b: Circle,
    transform_b: Transform,
) -> Manifold {
    let raw_a = polygon_a.into_raw();
    let raw_b = circle_b.into_raw();
    unsafe {
        ffi::b2CollidePolygonAndCircle(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between two capsules.
#[doc(alias = "b2CollideCapsules")]
pub fn collide_capsules(
    capsule_a: Capsule,
    transform_a: Transform,
    capsule_b: Capsule,
    transform_b: Transform,
) -> Manifold {
    let raw_a = capsule_a.into_raw();
    let raw_b = capsule_b.into_raw();
    unsafe { ffi::b2CollideCapsules(&raw_a, transform_a.into(), &raw_b, transform_b.into()) }.into()
}

/// Compute the contact manifold between a segment and a capsule.
#[doc(alias = "b2CollideSegmentAndCapsule")]
pub fn collide_segment_and_capsule(
    segment_a: Segment,
    transform_a: Transform,
    capsule_b: Capsule,
    transform_b: Transform,
) -> Manifold {
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    unsafe {
        ffi::b2CollideSegmentAndCapsule(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between a polygon and a capsule.
#[doc(alias = "b2CollidePolygonAndCapsule")]
pub fn collide_polygon_and_capsule(
    polygon_a: Polygon,
    transform_a: Transform,
    capsule_b: Capsule,
    transform_b: Transform,
) -> Manifold {
    let raw_a = polygon_a.into_raw();
    let raw_b = capsule_b.into_raw();
    unsafe {
        ffi::b2CollidePolygonAndCapsule(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between two polygons.
#[doc(alias = "b2CollidePolygons")]
pub fn collide_polygons(
    polygon_a: Polygon,
    transform_a: Transform,
    polygon_b: Polygon,
    transform_b: Transform,
) -> Manifold {
    let raw_a = polygon_a.into_raw();
    let raw_b = polygon_b.into_raw();
    unsafe { ffi::b2CollidePolygons(&raw_a, transform_a.into(), &raw_b, transform_b.into()) }.into()
}

/// Compute the contact manifold between a segment and a polygon.
#[doc(alias = "b2CollideSegmentAndPolygon")]
pub fn collide_segment_and_polygon(
    segment_a: Segment,
    transform_a: Transform,
    polygon_b: Polygon,
    transform_b: Transform,
) -> Manifold {
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    unsafe {
        ffi::b2CollideSegmentAndPolygon(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between a chain segment and a circle.
#[doc(alias = "b2CollideChainSegmentAndCircle")]
pub fn collide_chain_segment_and_circle(
    segment_a: ChainSegment,
    transform_a: Transform,
    circle_b: Circle,
    transform_b: Transform,
) -> Manifold {
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    unsafe {
        ffi::b2CollideChainSegmentAndCircle(&raw_a, transform_a.into(), &raw_b, transform_b.into())
    }
    .into()
}

/// Compute the contact manifold between a chain segment and a capsule.
///
/// Provide `cache` when repeatedly colliding against nearby rounded shapes to
/// warm-start the internal edge solver.
#[doc(alias = "b2CollideChainSegmentAndCapsule")]
pub fn collide_chain_segment_and_capsule(
    segment_a: ChainSegment,
    transform_a: Transform,
    capsule_b: Capsule,
    transform_b: Transform,
    cache: Option<&mut SimplexCache>,
) -> Manifold {
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let cache_ptr = match cache {
        Some(cache) => cache.raw_mut(),
        None => core::ptr::null_mut(),
    };
    unsafe {
        ffi::b2CollideChainSegmentAndCapsule(
            &raw_a,
            transform_a.into(),
            &raw_b,
            transform_b.into(),
            cache_ptr,
        )
    }
    .into()
}

/// Compute the contact manifold between a chain segment and a polygon.
///
/// Provide `cache` when repeatedly colliding against nearby rounded polygons to
/// warm-start the internal edge solver.
#[doc(alias = "b2CollideChainSegmentAndPolygon")]
pub fn collide_chain_segment_and_polygon(
    segment_a: ChainSegment,
    transform_a: Transform,
    polygon_b: Polygon,
    transform_b: Transform,
    cache: Option<&mut SimplexCache>,
) -> Manifold {
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    let cache_ptr = match cache {
        Some(cache) => cache.raw_mut(),
        None => core::ptr::null_mut(),
    };
    unsafe {
        ffi::b2CollideChainSegmentAndPolygon(
            &raw_a,
            transform_a.into(),
            &raw_b,
            transform_b.into(),
            cache_ptr,
        )
    }
    .into()
}

impl Aabb {
    /// Check whether this AABB is valid for Box2D queries.
    #[inline]
    pub fn is_valid(self) -> bool {
        unsafe { ffi::b2IsValidAABB(self.into_raw()) }
    }

    /// Ray cast against this AABB using Box2D-style `origin + translation`.
    ///
    /// Initial overlap returns a hit with zero fraction, zero normal, and `point = origin`.
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        if !self.is_valid() {
            return CastOutput::MISS;
        }

        let origin = origin.into();
        let translation = translation.into();
        let mut tmin = 0.0_f32;
        let mut tmax = 1.0_f32;
        let mut normal = Vec2::ZERO;

        if !ray_cast_axis(
            origin.x,
            translation.x,
            self.lower.x,
            self.upper.x,
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            &mut tmin,
            &mut tmax,
            &mut normal,
        ) {
            return CastOutput::MISS;
        }

        if !ray_cast_axis(
            origin.y,
            translation.y,
            self.lower.y,
            self.upper.y,
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            &mut tmin,
            &mut tmax,
            &mut normal,
        ) {
            return CastOutput::MISS;
        }

        if !(0.0..=1.0).contains(&tmin) {
            return CastOutput::MISS;
        }

        CastOutput {
            normal,
            point: Vec2::new(
                origin.x + tmin * translation.x,
                origin.y + tmin * translation.y,
            ),
            fraction: tmin,
            iterations: 0,
            hit: true,
        }
    }
}
