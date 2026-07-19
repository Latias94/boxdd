//! Standalone low-level collision geometry helpers.
//!
//! This module wraps Box2D's standalone collision algorithms without exposing raw FFI
//! structs. It is intentionally more explicit than the high-level `World` query API and
//! is useful when you want to run geometric tests or contact-manifold generation without
//! a world instance.
//!
//! Pairwise distance, cast, and manifold APIs run in shape A's local frame: callers pass
//! the transform and translation of B in A, and outputs are returned in A. World poses are
//! deliberately outside this module's contract.

use crate::{
    core::math::{Rot, Transform},
    error::{ApiError, ApiResult},
    query::Aabb,
    shapes::{Capsule, ChainSegment, Circle, Polygon, Segment},
    types::Vec2,
};
use boxdd_sys::ffi;
use core::fmt;

/// Maximum number of points supported by a Box2D shape proxy.
pub const MAX_SHAPE_PROXY_POINTS: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;

/// Maximum number of points in a standalone collision manifold.
pub const MAX_LOCAL_MANIFOLD_POINTS: usize = 2;

const _: () = {
    assert!(core::mem::size_of::<Vec2>() == core::mem::size_of::<ffi::b2Vec2>());
    assert!(core::mem::align_of::<Vec2>() == core::mem::align_of::<ffi::b2Vec2>());
};

#[inline]
fn check_collision_vec2_valid(value: Vec2) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_collision_rot_valid(value: Rot) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_collision_transform_valid(value: Transform) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_collision_non_negative_finite_scalar(value: f32) -> ApiResult<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_collision_unit_interval_scalar(value: f32) -> ApiResult<()> {
    if crate::is_valid_float(value) && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_collision_input_valid(name: &str, valid: bool) {
    assert!(valid, "{name} contains invalid Box2D input");
}

struct RayCastAxisInput {
    origin: f32,
    translation: f32,
    lower: f32,
    upper: f32,
    enter_normal: Vec2,
    exit_normal: Vec2,
}

struct RayCastAxisState {
    tmin: f32,
    tmax: f32,
    normal: Vec2,
}

#[inline]
fn ray_cast_axis(input: RayCastAxisInput, state: &mut RayCastAxisState) -> bool {
    if input.translation.abs() < f32::EPSILON {
        return input.lower <= input.origin && input.origin <= input.upper;
    }

    let inv_translation = 1.0 / input.translation;
    let mut t1 = (input.lower - input.origin) * inv_translation;
    let mut t2 = (input.upper - input.origin) * inv_translation;
    let mut n1 = input.enter_normal;
    let mut n2 = input.exit_normal;

    if t1 > t2 {
        core::mem::swap(&mut t1, &mut t2);
        core::mem::swap(&mut n1, &mut n2);
    }

    if t1 > state.tmin {
        state.tmin = t1;
        state.normal = n1;
    }

    if t2 < state.tmax {
        state.tmax = t2;
    }

    state.tmin <= state.tmax
}

/// A Box2D point-cloud proxy used by distance, shape-cast, and TOI algorithms.
///
/// Returns `None` from [`ShapeProxy::new`] when the iterator is empty, contains more than
/// [`MAX_SHAPE_PROXY_POINTS`] points, or contains invalid Box2D coordinates/radius data.
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
        Self::try_new(points, radius).ok()
    }

    /// Build a proxy from `1..=MAX_SHAPE_PROXY_POINTS` valid points and an external radius.
    pub fn try_new<I, P>(points: I, radius: f32) -> ApiResult<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        check_collision_non_negative_finite_scalar(radius)?;
        let mut raw_points = [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_SHAPE_PROXY_POINTS];
        let mut count = 0usize;

        for point in points {
            if count == MAX_SHAPE_PROXY_POINTS {
                return Err(ApiError::InvalidArgument);
            }
            let point = point.into();
            check_collision_vec2_valid(point)?;
            raw_points[count] = point.into_raw();
            count += 1;
        }

        if count == 0 {
            return Err(ApiError::InvalidArgument);
        }

        let raw = unsafe { ffi::b2MakeProxy(raw_points.as_ptr(), count as i32, radius) };
        Ok(Self { raw })
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

    /// Validate this proxy for Box2D standalone collision algorithms.
    pub fn validate(&self) -> ApiResult<()> {
        if !(1..=MAX_SHAPE_PROXY_POINTS as i32).contains(&self.raw.count) {
            return Err(ApiError::InvalidArgument);
        }
        check_collision_non_negative_finite_scalar(self.raw.radius)?;
        for point in self.points().iter().copied() {
            check_collision_vec2_valid(point)?;
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn into_raw(self) -> ffi::b2ShapeProxy {
        self.raw
    }

    #[inline]
    fn raw(self) -> ffi::b2ShapeProxy {
        self.into_raw()
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

/// Input for shape-specific casts against circles, capsules, segments, and polygons.
#[doc(alias = "shape_cast_input")]
#[derive(Copy, Clone, Debug)]
pub struct ShapeCastInput {
    pub proxy: ShapeProxy,
    pub translation: Vec2,
    pub max_fraction: f32,
    pub can_encroach: bool,
}

impl ShapeCastInput {
    /// Build a shape cast over `proxy` moving by `translation`.
    #[inline]
    pub fn new<T: Into<Vec2>>(proxy: ShapeProxy, translation: T) -> Self {
        Self {
            proxy,
            translation: translation.into(),
            max_fraction: 1.0,
            can_encroach: false,
        }
    }

    /// Limit the portion of `translation` considered by the cast.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction;
        self
    }

    /// Allow encroachment when initially touching.
    #[inline]
    pub fn with_can_encroach(mut self, can_encroach: bool) -> Self {
        self.can_encroach = can_encroach;
        self
    }

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        self.proxy.validate()?;
        check_collision_vec2_valid(self.translation)?;
        check_collision_unit_interval_scalar(self.max_fraction)
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2ShapeCastInput {
        ffi::b2ShapeCastInput {
            proxy: self.proxy.into_raw(),
            translation: self.translation.into_raw(),
            maxFraction: self.max_fraction,
            canEncroach: self.can_encroach,
        }
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

/// A contact point expressed in shape A's local frame.
///
/// Unlike [`crate::types::ManifoldPoint`], this is purely geometric data from a
/// standalone collision query. It does not contain solver impulses or persistence state.
#[doc(alias = "local_manifold_point")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct LocalManifoldPoint {
    /// Contact point in shape A's local frame.
    pub point: Vec2,
    /// Signed separation; negative values indicate penetration.
    pub separation: f32,
    /// Stable feature-pair identifier supplied by Box2D.
    pub id: u16,
}

impl LocalManifoldPoint {
    #[inline]
    pub const fn from_raw(raw: ffi::b2LocalManifoldPoint) -> Self {
        Self {
            point: Vec2::from_raw(raw.point),
            separation: raw.separation,
            id: raw.id,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2LocalManifoldPoint {
        ffi::b2LocalManifoldPoint {
            point: self.point.into_raw(),
            separation: self.separation,
            id: self.id,
        }
    }
}

/// Pure geometric contact manifold expressed in shape A's local frame.
///
/// `normal` points from shape A to shape B. Every point returned by [`Self::points`]
/// is also expressed in A's frame. Convert them with shape A's pose only at a
/// presentation or world-query boundary; standalone collision never consumes world coordinates.
#[doc(alias = "local_manifold")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct LocalManifold {
    pub normal: Vec2,
    pub contact_points: [LocalManifoldPoint; MAX_LOCAL_MANIFOLD_POINTS],
    pub point_count: i32,
}

impl LocalManifold {
    /// The initialized contact points in this manifold.
    #[inline]
    pub fn points(&self) -> &[LocalManifoldPoint] {
        &self.contact_points[..self.point_count()]
    }

    /// The number of initialized contact points.
    #[inline]
    pub fn point_count(&self) -> usize {
        self.point_count.clamp(0, MAX_LOCAL_MANIFOLD_POINTS as i32) as usize
    }

    /// Whether this query found no contact points.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.point_count() == 0
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2LocalManifold) -> Self {
        Self {
            normal: Vec2::from_raw(raw.normal),
            contact_points: raw.points.map(LocalManifoldPoint::from_raw),
            point_count: raw.pointCount.clamp(0, MAX_LOCAL_MANIFOLD_POINTS as i32),
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2LocalManifold {
        ffi::b2LocalManifold {
            normal: self.normal.into_raw(),
            points: self.contact_points.map(LocalManifoldPoint::into_raw),
            pointCount: self.point_count.clamp(0, MAX_LOCAL_MANIFOLD_POINTS as i32),
        }
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
            closest1: Vec2::from_raw(raw.closest1),
            closest2: Vec2::from_raw(raw.closest2),
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
            normal: Vec2::from_raw(raw.normal),
            point: Vec2::from_raw(raw.point),
            fraction: raw.fraction,
            iterations: raw.iterations,
            hit: raw.hit,
        }
    }
}

/// Input for [`shape_distance`], evaluated entirely in shape A's local frame.
#[doc(alias = "distance_input")]
#[derive(Copy, Clone, Debug)]
pub struct DistanceInput {
    pub proxy_a: ShapeProxy,
    pub proxy_b: ShapeProxy,
    /// Transform of shape B in shape A's local frame.
    pub transform_b_in_a: Transform,
    pub use_radii: bool,
}

impl DistanceInput {
    /// Build distance input with `use_radii = false`.
    ///
    /// `transform_b_in_a` maps shape B's local coordinates into shape A's local frame.
    #[inline]
    pub fn new(proxy_a: ShapeProxy, proxy_b: ShapeProxy, transform_b_in_a: Transform) -> Self {
        Self {
            proxy_a,
            proxy_b,
            transform_b_in_a,
            use_radii: false,
        }
    }

    /// Set whether proxy radii should affect the distance result.
    #[inline]
    pub fn with_radii(mut self, use_radii: bool) -> Self {
        self.use_radii = use_radii;
        self
    }

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        self.proxy_a.validate()?;
        self.proxy_b.validate()?;
        check_collision_transform_valid(self.transform_b_in_a)?;
        Ok(())
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2DistanceInput {
        ffi::b2DistanceInput {
            proxyA: self.proxy_a.raw(),
            proxyB: self.proxy_b.raw(),
            transform: self.transform_b_in_a.into_raw(),
            useRadii: self.use_radii,
        }
    }
}

/// Output from [`shape_distance`], expressed in shape A's local frame.
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
            point_a: Vec2::from_raw(raw.pointA),
            point_b: Vec2::from_raw(raw.pointB),
            normal: Vec2::from_raw(raw.normal),
            distance: raw.distance,
            iterations: raw.iterations,
            simplex_count: raw.simplexCount,
        }
    }
}

/// Input for [`shape_cast`], evaluated entirely in shape A's local frame.
#[doc(alias = "shape_cast_pair_input")]
#[derive(Copy, Clone, Debug)]
pub struct ShapeCastPairInput {
    pub proxy_a: ShapeProxy,
    pub proxy_b: ShapeProxy,
    /// Transform of shape B in shape A's local frame.
    pub transform_b_in_a: Transform,
    /// Translation of shape B expressed in shape A's local frame.
    pub translation_b_in_a: Vec2,
    pub max_fraction: f32,
    pub can_encroach: bool,
}

impl ShapeCastPairInput {
    /// Build a shape cast where B starts at `transform_b_in_a` and moves by
    /// `translation_b_in_a`, both expressed in shape A's local frame.
    #[inline]
    pub fn new<V: Into<Vec2>>(
        proxy_a: ShapeProxy,
        proxy_b: ShapeProxy,
        transform_b_in_a: Transform,
        translation_b_in_a: V,
    ) -> Self {
        Self {
            proxy_a,
            proxy_b,
            transform_b_in_a,
            translation_b_in_a: translation_b_in_a.into(),
            max_fraction: 1.0,
            can_encroach: false,
        }
    }

    /// Limit the portion of `translation_b_in_a` considered by the cast.
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

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        self.proxy_a.validate()?;
        self.proxy_b.validate()?;
        check_collision_transform_valid(self.transform_b_in_a)?;
        check_collision_vec2_valid(self.translation_b_in_a)?;
        check_collision_unit_interval_scalar(self.max_fraction)?;
        Ok(())
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2ShapeCastPairInput {
        ffi::b2ShapeCastPairInput {
            proxyA: self.proxy_a.raw(),
            proxyB: self.proxy_b.raw(),
            transform: self.transform_b_in_a.into_raw(),
            translationB: self.translation_b_in_a.into_raw(),
            maxFraction: self.max_fraction,
            canEncroach: self.can_encroach,
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

    #[inline]
    pub fn from_raw(raw: ffi::b2Sweep) -> Self {
        Self {
            local_center: Vec2::from_raw(raw.localCenter),
            c1: Vec2::from_raw(raw.c1),
            c2: Vec2::from_raw(raw.c2),
            q1: Rot::from_raw(raw.q1),
            q2: Rot::from_raw(raw.q2),
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2Sweep {
        ffi::b2Sweep {
            localCenter: self.local_center.into_raw(),
            c1: self.c1.into_raw(),
            c2: self.c2.into_raw(),
            q1: self.q1.into_raw(),
            q2: self.q2.into_raw(),
        }
    }

    /// Validate this sweep for Box2D continuous-collision algorithms.
    pub fn validate(&self) -> ApiResult<()> {
        check_collision_vec2_valid(self.local_center)?;
        check_collision_vec2_valid(self.c1)?;
        check_collision_vec2_valid(self.c2)?;
        check_collision_rot_valid(self.q1)?;
        check_collision_rot_valid(self.q2)?;
        Ok(())
    }

    /// Evaluate the sweep transform at `time` in the `[0, 1]` interval.
    #[inline]
    pub fn transform_at(self, time: f32) -> Transform {
        let raw = self.into_raw();
        Transform::from_raw(unsafe { ffi::b2GetSweepTransform(&raw, time) })
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

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        self.proxy_a.validate()?;
        self.proxy_b.validate()?;
        self.sweep_a.validate()?;
        self.sweep_b.validate()?;
        check_collision_unit_interval_scalar(self.max_fraction)?;
        Ok(())
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2TOIInput {
        ffi::b2TOIInput {
            proxyA: self.proxy_a.raw(),
            proxyB: self.proxy_b.raw(),
            sweepA: self.sweep_a.into_raw(),
            sweepB: self.sweep_b.into_raw(),
            maxFraction: self.max_fraction,
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
            point: Vec2::from_raw(raw.point),
            normal: Vec2::from_raw(raw.normal),
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
    let p1 = p1.into();
    let q1 = q1.into();
    let p2 = p2.into();
    let q2 = q2.into();
    assert_collision_input_valid(
        "segment_distance p1",
        check_collision_vec2_valid(p1).is_ok(),
    );
    assert_collision_input_valid(
        "segment_distance q1",
        check_collision_vec2_valid(q1).is_ok(),
    );
    assert_collision_input_valid(
        "segment_distance p2",
        check_collision_vec2_valid(p2).is_ok(),
    );
    assert_collision_input_valid(
        "segment_distance q2",
        check_collision_vec2_valid(q2).is_ok(),
    );
    SegmentDistanceResult::from_raw(unsafe {
        ffi::b2SegmentDistance(p1.into_raw(), q1.into_raw(), p2.into_raw(), q2.into_raw())
    })
}

/// Compute the closest points between two line segments with recoverable validation.
pub fn try_segment_distance<P1, Q1, P2, Q2>(
    p1: P1,
    q1: Q1,
    p2: P2,
    q2: Q2,
) -> ApiResult<SegmentDistanceResult>
where
    P1: Into<Vec2>,
    Q1: Into<Vec2>,
    P2: Into<Vec2>,
    Q2: Into<Vec2>,
{
    let p1 = p1.into();
    let q1 = q1.into();
    let p2 = p2.into();
    let q2 = q2.into();
    check_collision_vec2_valid(p1)?;
    check_collision_vec2_valid(q1)?;
    check_collision_vec2_valid(p2)?;
    check_collision_vec2_valid(q2)?;
    Ok(SegmentDistanceResult::from_raw(unsafe {
        ffi::b2SegmentDistance(p1.into_raw(), q1.into_raw(), p2.into_raw(), q2.into_raw())
    }))
}

/// Compute the closest distance between two shape proxies.
pub fn shape_distance(input: DistanceInput, cache: &mut SimplexCache) -> DistanceOutput {
    assert_collision_input_valid("shape_distance input", input.validate().is_ok());
    let raw_input = input.into_raw();
    DistanceOutput::from_raw(unsafe {
        ffi::b2ShapeDistance(&raw_input, cache.raw_mut(), core::ptr::null_mut(), 0)
    })
}

/// Compute the closest distance between two shape proxies with recoverable validation.
pub fn try_shape_distance(
    input: DistanceInput,
    cache: &mut SimplexCache,
) -> ApiResult<DistanceOutput> {
    input.validate()?;
    let raw_input = input.into_raw();
    Ok(DistanceOutput::from_raw(unsafe {
        ffi::b2ShapeDistance(&raw_input, cache.raw_mut(), core::ptr::null_mut(), 0)
    }))
}

/// Cast shape B against shape A.
///
/// The hit point and normal are returned in shape A's local frame.
pub fn shape_cast(input: ShapeCastPairInput) -> CastOutput {
    assert_collision_input_valid("shape_cast input", input.validate().is_ok());
    let raw_input = input.into_raw();
    CastOutput::from_raw(unsafe { ffi::b2ShapeCast(&raw_input) })
}

/// Cast shape B against shape A with recoverable validation.
///
/// The hit point and normal are returned in shape A's local frame.
pub fn try_shape_cast(input: ShapeCastPairInput) -> ApiResult<CastOutput> {
    input.validate()?;
    let raw_input = input.into_raw();
    Ok(CastOutput::from_raw(unsafe {
        ffi::b2ShapeCast(&raw_input)
    }))
}

/// Compute the time of impact between two moving shape proxies.
pub fn time_of_impact(input: ToiInput) -> ToiOutput {
    assert_collision_input_valid("time_of_impact input", input.validate().is_ok());
    let raw_input = input.into_raw();
    ToiOutput::from_raw(unsafe { ffi::b2TimeOfImpact(&raw_input) })
}

/// Compute the time of impact between two moving shape proxies with recoverable validation.
pub fn try_time_of_impact(input: ToiInput) -> ApiResult<ToiOutput> {
    input.validate()?;
    let raw_input = input.into_raw();
    Ok(ToiOutput::from_raw(unsafe {
        ffi::b2TimeOfImpact(&raw_input)
    }))
}

/// Compute the contact manifold between two circles.
///
/// `transform_b_in_a` maps shape B's local coordinates into shape A's local frame.
/// The returned manifold is expressed in shape A's frame.
#[doc(alias = "b2CollideCircles")]
pub fn collide_circles(
    circle_a: Circle,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("circle_a", circle_a.validate().is_ok());
    assert_collision_input_valid("circle_b", circle_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = circle_a.into_raw();
    let raw_b = circle_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideCircles(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between two circles with recoverable validation.
pub fn try_collide_circles(
    circle_a: Circle,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    circle_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = circle_a.into_raw();
    let raw_b = circle_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideCircles(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a capsule and a circle.
///
/// `transform_b_in_a` maps the circle into the capsule's local frame. The returned
/// manifold is expressed in the capsule's frame.
#[doc(alias = "b2CollideCapsuleAndCircle")]
pub fn collide_capsule_and_circle(
    capsule_a: Capsule,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("capsule_a", capsule_a.validate().is_ok());
    assert_collision_input_valid("circle_b", circle_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = capsule_a.into_raw();
    let raw_b = circle_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideCapsuleAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a capsule and a circle with recoverable validation.
pub fn try_collide_capsule_and_circle(
    capsule_a: Capsule,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    capsule_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = capsule_a.into_raw();
    let raw_b = circle_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideCapsuleAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a segment and a circle.
///
/// `transform_b_in_a` maps the circle into the segment's local frame. The returned
/// manifold is expressed in the segment's frame.
#[doc(alias = "b2CollideSegmentAndCircle")]
pub fn collide_segment_and_circle(
    segment_a: Segment,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("segment_a", segment_a.validate().is_ok());
    assert_collision_input_valid("circle_b", circle_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideSegmentAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a segment and a circle with recoverable validation.
pub fn try_collide_segment_and_circle(
    segment_a: Segment,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    segment_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideSegmentAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a polygon and a circle.
///
/// `transform_b_in_a` maps the circle into the polygon's local frame. The returned
/// manifold is expressed in the polygon's frame.
#[doc(alias = "b2CollidePolygonAndCircle")]
pub fn collide_polygon_and_circle(
    polygon_a: Polygon,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("polygon_a", polygon_a.validate().is_ok());
    assert_collision_input_valid("circle_b", circle_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = polygon_a.into_raw();
    let raw_b = circle_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollidePolygonAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a polygon and a circle with recoverable validation.
pub fn try_collide_polygon_and_circle(
    polygon_a: Polygon,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    polygon_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = polygon_a.into_raw();
    let raw_b = circle_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollidePolygonAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between two capsules.
///
/// `transform_b_in_a` maps capsule B into capsule A's local frame. The returned
/// manifold is expressed in capsule A's frame.
#[doc(alias = "b2CollideCapsules")]
pub fn collide_capsules(
    capsule_a: Capsule,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("capsule_a", capsule_a.validate().is_ok());
    assert_collision_input_valid("capsule_b", capsule_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = capsule_a.into_raw();
    let raw_b = capsule_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideCapsules(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between two capsules with recoverable validation.
pub fn try_collide_capsules(
    capsule_a: Capsule,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    capsule_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = capsule_a.into_raw();
    let raw_b = capsule_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideCapsules(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a segment and a capsule.
///
/// `transform_b_in_a` maps the capsule into the segment's local frame. The returned
/// manifold is expressed in the segment's frame.
#[doc(alias = "b2CollideSegmentAndCapsule")]
pub fn collide_segment_and_capsule(
    segment_a: Segment,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("segment_a", segment_a.validate().is_ok());
    assert_collision_input_valid("capsule_b", capsule_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideSegmentAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a segment and a capsule with recoverable validation.
pub fn try_collide_segment_and_capsule(
    segment_a: Segment,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    segment_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideSegmentAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a polygon and a capsule.
///
/// `transform_b_in_a` maps the capsule into the polygon's local frame. The returned
/// manifold is expressed in the polygon's frame.
#[doc(alias = "b2CollidePolygonAndCapsule")]
pub fn collide_polygon_and_capsule(
    polygon_a: Polygon,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("polygon_a", polygon_a.validate().is_ok());
    assert_collision_input_valid("capsule_b", capsule_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = polygon_a.into_raw();
    let raw_b = capsule_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollidePolygonAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a polygon and a capsule with recoverable validation.
pub fn try_collide_polygon_and_capsule(
    polygon_a: Polygon,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    polygon_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = polygon_a.into_raw();
    let raw_b = capsule_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollidePolygonAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between two polygons.
///
/// `transform_b_in_a` maps polygon B into polygon A's local frame. The returned
/// manifold is expressed in polygon A's frame.
#[doc(alias = "b2CollidePolygons")]
pub fn collide_polygons(
    polygon_a: Polygon,
    polygon_b: Polygon,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("polygon_a", polygon_a.validate().is_ok());
    assert_collision_input_valid("polygon_b", polygon_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = polygon_a.into_raw();
    let raw_b = polygon_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollidePolygons(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between two polygons with recoverable validation.
pub fn try_collide_polygons(
    polygon_a: Polygon,
    polygon_b: Polygon,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    polygon_a.validate()?;
    polygon_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = polygon_a.into_raw();
    let raw_b = polygon_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollidePolygons(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a segment and a polygon.
///
/// `transform_b_in_a` maps the polygon into the segment's local frame. The returned
/// manifold is expressed in the segment's frame.
#[doc(alias = "b2CollideSegmentAndPolygon")]
pub fn collide_segment_and_polygon(
    segment_a: Segment,
    polygon_b: Polygon,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("segment_a", segment_a.validate().is_ok());
    assert_collision_input_valid("polygon_b", polygon_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideSegmentAndPolygon(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a segment and a polygon with recoverable validation.
pub fn try_collide_segment_and_polygon(
    segment_a: Segment,
    polygon_b: Polygon,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    segment_a.validate()?;
    polygon_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideSegmentAndPolygon(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a chain segment and a circle.
///
/// `transform_b_in_a` maps the circle into the chain segment's local frame. The
/// returned manifold is expressed in the chain segment's frame.
#[doc(alias = "b2CollideChainSegmentAndCircle")]
pub fn collide_chain_segment_and_circle(
    segment_a: ChainSegment,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> LocalManifold {
    assert_collision_input_valid("segment_a", segment_a.validate().is_ok());
    assert_collision_input_valid("circle_b", circle_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideChainSegmentAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
}

/// Compute the contact manifold between a chain segment and a circle with recoverable validation.
pub fn try_collide_chain_segment_and_circle(
    segment_a: ChainSegment,
    circle_b: Circle,
    transform_b_in_a: Transform,
) -> ApiResult<LocalManifold> {
    segment_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideChainSegmentAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    }))
}

/// Compute the contact manifold between a chain segment and a capsule.
///
/// `transform_b_in_a` maps the capsule into the chain segment's local frame. The
/// returned manifold is expressed in the chain segment's frame.
///
/// Provide `cache` when repeatedly colliding against nearby rounded shapes to
/// warm-start the internal edge solver.
#[doc(alias = "b2CollideChainSegmentAndCapsule")]
pub fn collide_chain_segment_and_capsule(
    segment_a: ChainSegment,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
    cache: Option<&mut SimplexCache>,
) -> LocalManifold {
    assert_collision_input_valid("segment_a", segment_a.validate().is_ok());
    assert_collision_input_valid("capsule_b", capsule_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let mut fallback_cache = SimplexCache::default();
    let cache_ptr = cache.unwrap_or(&mut fallback_cache).raw_mut();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideChainSegmentAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw(), cache_ptr)
    })
}

/// Compute the contact manifold between a chain segment and a capsule with recoverable validation.
pub fn try_collide_chain_segment_and_capsule(
    segment_a: ChainSegment,
    capsule_b: Capsule,
    transform_b_in_a: Transform,
    cache: Option<&mut SimplexCache>,
) -> ApiResult<LocalManifold> {
    segment_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let mut fallback_cache = SimplexCache::default();
    let cache_ptr = cache.unwrap_or(&mut fallback_cache).raw_mut();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideChainSegmentAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw(), cache_ptr)
    }))
}

/// Compute the contact manifold between a chain segment and a polygon.
///
/// `transform_b_in_a` maps the polygon into the chain segment's local frame. The
/// returned manifold is expressed in the chain segment's frame.
///
/// Provide `cache` when repeatedly colliding against nearby rounded polygons to
/// warm-start the internal edge solver.
#[doc(alias = "b2CollideChainSegmentAndPolygon")]
pub fn collide_chain_segment_and_polygon(
    segment_a: ChainSegment,
    polygon_b: Polygon,
    transform_b_in_a: Transform,
    cache: Option<&mut SimplexCache>,
) -> LocalManifold {
    assert_collision_input_valid("segment_a", segment_a.validate().is_ok());
    assert_collision_input_valid("polygon_b", polygon_b.validate().is_ok());
    assert_collision_input_valid(
        "transform_b_in_a",
        check_collision_transform_valid(transform_b_in_a).is_ok(),
    );
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    let mut fallback_cache = SimplexCache::default();
    let cache_ptr = cache.unwrap_or(&mut fallback_cache).raw_mut();
    LocalManifold::from_raw(unsafe {
        ffi::b2CollideChainSegmentAndPolygon(&raw_a, &raw_b, transform_b_in_a.into_raw(), cache_ptr)
    })
}

/// Compute the contact manifold between a chain segment and a polygon with recoverable validation.
pub fn try_collide_chain_segment_and_polygon(
    segment_a: ChainSegment,
    polygon_b: Polygon,
    transform_b_in_a: Transform,
    cache: Option<&mut SimplexCache>,
) -> ApiResult<LocalManifold> {
    segment_a.validate()?;
    polygon_b.validate()?;
    check_collision_transform_valid(transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    let mut fallback_cache = SimplexCache::default();
    let cache_ptr = cache.unwrap_or(&mut fallback_cache).raw_mut();
    Ok(LocalManifold::from_raw(unsafe {
        ffi::b2CollideChainSegmentAndPolygon(&raw_a, &raw_b, transform_b_in_a.into_raw(), cache_ptr)
    }))
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
        let mut axis_state = RayCastAxisState {
            tmin: 0.0,
            tmax: 1.0,
            normal: Vec2::ZERO,
        };

        if !ray_cast_axis(
            RayCastAxisInput {
                origin: origin.x,
                translation: translation.x,
                lower: self.lower.x,
                upper: self.upper.x,
                enter_normal: Vec2::new(-1.0, 0.0),
                exit_normal: Vec2::new(1.0, 0.0),
            },
            &mut axis_state,
        ) {
            return CastOutput::MISS;
        }

        if !ray_cast_axis(
            RayCastAxisInput {
                origin: origin.y,
                translation: translation.y,
                lower: self.lower.y,
                upper: self.upper.y,
                enter_normal: Vec2::new(0.0, -1.0),
                exit_normal: Vec2::new(0.0, 1.0),
            },
            &mut axis_state,
        ) {
            return CastOutput::MISS;
        }

        if !(0.0..=1.0).contains(&axis_state.tmin) {
            return CastOutput::MISS;
        }

        CastOutput {
            normal: axis_state.normal,
            point: Vec2::new(
                origin.x + axis_state.tmin * translation.x,
                origin.y + axis_state.tmin * translation.y,
            ),
            fraction: axis_state.tmin,
            iterations: 0,
            hit: true,
        }
    }
}
