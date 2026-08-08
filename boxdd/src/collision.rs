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
    core::{
        foundation::transient_native_lease,
        math::{Rot, Transform},
    },
    error::{Error, Result},
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
fn check_collision_vec2_valid(
    operation: &'static str,
    argument: &'static str,
    value: Vec2,
) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite vector",
        ))
    }
}

#[inline]
fn check_collision_rot_valid(
    operation: &'static str,
    argument: &'static str,
    value: Rot,
) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a normalized finite rotation",
        ))
    }
}

#[inline]
fn check_collision_transform_valid(operation: &'static str, value: Transform) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            "transform_b_in_a",
            "a finite rigid transform",
        ))
    }
}

#[inline]
fn check_collision_non_negative_finite_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value greater than or equal to zero",
        ))
    }
}

#[inline]
fn check_collision_finite_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    if crate::is_valid_float(value) {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value",
        ))
    }
}

#[inline]
fn check_collision_non_negative_int(
    operation: &'static str,
    argument: &'static str,
    value: i32,
) -> Result<()> {
    if value >= 0 {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a non-negative native int",
        ))
    }
}

#[inline]
fn collision_unit_vector_is_valid(value: Vec2) -> bool {
    value.is_valid() && (1.0 - (value.x * value.x + value.y * value.y)).abs() < 100.0 * f32::EPSILON
}

#[inline]
fn check_collision_unit_interval_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    if crate::is_valid_float(value) && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value in 0.0..=1.0",
        ))
    }
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
/// Construction fails when the iterator is empty, contains more than
/// [`MAX_SHAPE_PROXY_POINTS`] points, or contains invalid Box2D coordinates/radius data.
#[doc(alias = "shape_proxy")]
#[derive(Copy, Clone)]
pub struct ShapeProxy {
    raw: ffi::b2ShapeProxy,
}

impl ShapeProxy {
    /// Build a proxy from `1..=MAX_SHAPE_PROXY_POINTS` valid points and an external radius.
    pub fn new<I, P>(points: I, radius: f32) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let (raw_points, count) = collect_shape_proxy_points("ShapeProxy::new", points, radius)?;
        Ok(Self {
            raw: ffi::b2ShapeProxy {
                points: raw_points,
                count,
                radius,
            },
        })
    }

    /// Build a proxy and apply a rigid transform to every stored point.
    #[doc(alias = "b2MakeOffsetProxy")]
    pub fn offset_from_points<I, P>(points: I, radius: f32, transform: Transform) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        Self::offset_from_points_for(
            "ShapeProxy::offset_from_points",
            "points/transform",
            points,
            radius,
            transform,
        )
    }

    pub(crate) fn offset_from_points_for<I, P>(
        operation: &'static str,
        transformed_argument: &'static str,
        points: I,
        radius: f32,
        transform: Transform,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let (mut raw_points, count) = collect_shape_proxy_points(operation, points, radius)?;
        if !transform.is_valid() {
            return Err(Error::invalid_argument(
                operation,
                "transform",
                "a finite rigid transform",
            ));
        }
        for point in raw_points.iter_mut().take(count as usize) {
            let transformed = transform.transform_point(Vec2::from_raw(*point));
            if !transformed.is_valid() {
                return Err(Error::invalid_argument(
                    operation,
                    transformed_argument,
                    "a transform whose proxy points remain finite",
                ));
            }
            *point = transformed.into_raw();
        }
        Ok(Self {
            raw: ffi::b2ShapeProxy {
                points: raw_points,
                count,
                radius,
            },
        })
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
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_SHAPE_PROXY_POINTS as i32).contains(&self.raw.count) {
            return Err(Error::invalid_argument(
                "ShapeProxy::validate",
                "points",
                "between 1 and Box2D's maximum shape-proxy point count",
            ));
        }
        check_collision_non_negative_finite_scalar(
            "ShapeProxy::validate",
            "radius",
            self.raw.radius,
        )?;
        for point in self.points().iter().copied() {
            check_collision_vec2_valid("ShapeProxy::validate", "points", point)?;
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

fn collect_shape_proxy_points<I, P>(
    operation: &'static str,
    points: I,
    radius: f32,
) -> Result<([ffi::b2Vec2; MAX_SHAPE_PROXY_POINTS], i32)>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    check_collision_non_negative_finite_scalar(operation, "radius", radius)?;
    let mut raw_points = [ffi::b2Vec2 { x: 0.0, y: 0.0 }; MAX_SHAPE_PROXY_POINTS];
    let mut count = 0usize;

    for point in points {
        if count == MAX_SHAPE_PROXY_POINTS {
            return Err(Error::invalid_argument(
                operation,
                "points",
                "no more than Box2D's maximum shape-proxy point count",
            ));
        }
        let point = point.into();
        check_collision_vec2_valid(operation, "points", point)?;
        raw_points[count] = point.into_raw();
        count += 1;
    }

    if count == 0 {
        return Err(Error::invalid_argument(
            operation,
            "points",
            "at least one point",
        ));
    }

    Ok((raw_points, count as i32))
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
    pub(crate) proxy: ShapeProxy,
    pub(crate) translation: Vec2,
    pub(crate) max_fraction: f32,
    pub(crate) can_encroach: bool,
}

impl ShapeCastInput {
    /// Build a shape cast over `proxy` moving by `translation`.
    #[inline]
    pub fn new<T: Into<Vec2>>(proxy: ShapeProxy, translation: T) -> Result<Self> {
        let input = Self {
            proxy,
            translation: translation.into(),
            max_fraction: 1.0,
            can_encroach: false,
        };
        input.validate()?;
        Ok(input)
    }

    /// Limit the portion of `translation` considered by the cast.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Result<Self> {
        check_collision_unit_interval_scalar(
            "ShapeCastInput::with_max_fraction",
            "max_fraction",
            max_fraction,
        )?;
        self.max_fraction = max_fraction;
        Ok(self)
    }

    /// Allow encroachment when initially touching.
    #[inline]
    pub fn with_can_encroach(mut self, can_encroach: bool) -> Self {
        self.can_encroach = can_encroach;
        self
    }

    #[inline]
    pub const fn proxy(self) -> ShapeProxy {
        self.proxy
    }

    #[inline]
    pub const fn translation(self) -> Vec2 {
        self.translation
    }

    #[inline]
    pub const fn max_fraction(self) -> f32 {
        self.max_fraction
    }

    #[inline]
    pub const fn can_encroach(self) -> bool {
        self.can_encroach
    }

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> Result<()> {
        self.proxy.validate()?;
        check_collision_vec2_valid("ShapeCastInput::validate", "translation", self.translation)?;
        check_collision_unit_interval_scalar(
            "ShapeCastInput::validate",
            "max_fraction",
            self.max_fraction,
        )
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

    fn validate_for(
        &self,
        operation: &'static str,
        proxy_a_count: usize,
        proxy_b_count: usize,
    ) -> Result<()> {
        let count = usize::from(self.raw.count);
        if count > 3 {
            return Err(Error::invalid_argument(
                operation,
                "cache.count",
                "a simplex point count in 0..=3",
            ));
        }
        if self.raw.indexA[..count]
            .iter()
            .any(|index| usize::from(*index) >= proxy_a_count)
        {
            return Err(Error::invalid_argument(
                operation,
                "cache.index_a",
                "indices within shape A's proxy points",
            ));
        }
        if self.raw.indexB[..count]
            .iter()
            .any(|index| usize::from(*index) >= proxy_b_count)
        {
            return Err(Error::invalid_argument(
                operation,
                "cache.index_b",
                "indices within shape B's proxy points",
            ));
        }
        Ok(())
    }

    fn validate_native_for(
        &self,
        operation: &'static str,
        proxy_a_count: usize,
        proxy_b_count: usize,
    ) -> Result<()> {
        self.validate_for(operation, proxy_a_count, proxy_b_count)
            .map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "simplex_cache",
                constraint: "at most three in-range proxy point indices",
            })
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

fn commit_native_simplex_cache(
    cache: Option<&mut SimplexCache>,
    staged: SimplexCache,
    operation: &'static str,
    proxy_a_count: usize,
    proxy_b_count: usize,
) -> Result<()> {
    staged.validate_native_for(operation, proxy_a_count, proxy_b_count)?;
    if let Some(cache) = cache {
        *cache = staged;
    }
    Ok(())
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
    pub fn from_raw(raw: ffi::b2LocalManifoldPoint) -> Result<Self> {
        let point = Self::from_raw_unvalidated(raw);
        point.validate_for("LocalManifoldPoint::from_raw")?;
        Ok(point)
    }

    #[inline]
    const fn from_raw_unvalidated(raw: ffi::b2LocalManifoldPoint) -> Self {
        Self {
            point: Vec2::from_raw(raw.point),
            separation: raw.separation,
            id: raw.id,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_for("LocalManifoldPoint::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        check_collision_vec2_valid(operation, "point", self.point)?;
        check_collision_finite_scalar(operation, "separation", self.separation)
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
/// `normal` points from shape A to shape B whenever that direction is defined. For coincident
/// geometry, Box2D emits a zero normal; Safe Rust replaces it with the deterministic positive-X
/// unit vector. Every point returned by [`Self::points`] is also expressed in A's frame. Convert
/// them with shape A's pose only at a presentation or world-query boundary; standalone collision
/// never consumes world coordinates.
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
    pub fn from_raw(raw: ffi::b2LocalManifold) -> Result<Self> {
        let manifold = Self::from_raw_unvalidated(raw);
        manifold.validate_for("LocalManifold::from_raw")?;
        Ok(manifold)
    }

    #[inline]
    fn from_native(operation: &'static str, raw: ffi::b2LocalManifold) -> Result<Self> {
        let mut manifold = Self::from_raw_unvalidated(raw);
        // Box2D deliberately returns a zero normal for coincident centers. The contact direction
        // is undefined there, so expose a deterministic unit vector instead of weakening the Safe
        // value invariant or rejecting otherwise valid native output.
        if manifold.point_count > 0 && manifold.normal == Vec2::ZERO {
            manifold.normal = Vec2::new(1.0, 0.0);
        }
        manifold
            .validate_for(operation)
            .map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "local_manifold",
                constraint: "zero to two finite contact points and a unit normal when non-empty",
            })?;
        Ok(manifold)
    }

    #[inline]
    fn from_raw_unvalidated(raw: ffi::b2LocalManifold) -> Self {
        Self {
            normal: Vec2::from_raw(raw.normal),
            contact_points: raw.points.map(LocalManifoldPoint::from_raw_unvalidated),
            point_count: raw.pointCount,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_for("LocalManifold::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        if !(0..=MAX_LOCAL_MANIFOLD_POINTS as i32).contains(&self.point_count) {
            return Err(Error::invalid_argument(
                operation,
                "point_count",
                "a contact point count in 0..=2",
            ));
        }
        check_collision_vec2_valid(operation, "normal", self.normal)?;
        if self.point_count > 0 && !collision_unit_vector_is_valid(self.normal) {
            return Err(Error::invalid_argument(
                operation,
                "normal",
                "a finite unit vector when the manifold is non-empty",
            ));
        }
        for point in self.points() {
            point.validate_for(operation)?;
        }
        Ok(())
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2LocalManifold {
        ffi::b2LocalManifold {
            normal: self.normal.into_raw(),
            points: self.contact_points.map(LocalManifoldPoint::into_raw),
            pointCount: self.point_count,
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
    pub fn from_raw(raw: ffi::b2SegmentDistanceResult) -> Result<Self> {
        let result = Self {
            closest1: Vec2::from_raw(raw.closest1),
            closest2: Vec2::from_raw(raw.closest2),
            fraction1: raw.fraction1,
            fraction2: raw.fraction2,
            distance_squared: raw.distanceSquared,
        };
        result.validate_for("SegmentDistanceResult::from_raw")?;
        Ok(result)
    }

    fn from_native(operation: &'static str, raw: ffi::b2SegmentDistanceResult) -> Result<Self> {
        Self::from_raw(raw).map_err(|_| Error::InvalidNativeOutput {
            operation,
            output: "segment_distance",
            constraint: "finite points, unit-interval fractions, and a non-negative squared distance",
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_for("SegmentDistanceResult::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        check_collision_vec2_valid(operation, "closest1", self.closest1)?;
        check_collision_vec2_valid(operation, "closest2", self.closest2)?;
        check_collision_unit_interval_scalar(operation, "fraction1", self.fraction1)?;
        check_collision_unit_interval_scalar(operation, "fraction2", self.fraction2)?;
        check_collision_non_negative_finite_scalar(
            operation,
            "distance_squared",
            self.distance_squared,
        )
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
    pub fn from_raw(raw: ffi::b2CastOutput) -> Result<Self> {
        let output = Self {
            normal: Vec2::from_raw(raw.normal),
            point: Vec2::from_raw(raw.point),
            fraction: raw.fraction,
            iterations: raw.iterations,
            hit: raw.hit,
        };
        output.validate_for("CastOutput::from_raw")?;
        Ok(output)
    }

    pub(crate) fn from_native(operation: &'static str, raw: ffi::b2CastOutput) -> Result<Self> {
        let output = Self {
            normal: Vec2::from_raw(raw.normal),
            point: Vec2::from_raw(raw.point),
            fraction: raw.fraction,
            iterations: raw.iterations,
            hit: raw.hit,
        };
        output
            .validate_for(operation)
            .map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "cast_output",
                constraint: "finite hit data, a unit-interval fraction, and non-negative iterations",
            })?;
        Ok(output)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_for("CastOutput::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        check_collision_vec2_valid(operation, "normal", self.normal)?;
        check_collision_vec2_valid(operation, "point", self.point)?;
        check_collision_unit_interval_scalar(operation, "fraction", self.fraction)?;
        check_collision_non_negative_int(operation, "iterations", self.iterations)?;
        if self.hit && self.fraction > 0.0 && !collision_unit_vector_is_valid(self.normal) {
            return Err(Error::invalid_argument(
                operation,
                "normal",
                "a finite unit vector for a non-overlap hit",
            ));
        }
        if self.hit
            && self.fraction == 0.0
            && self.normal != Vec2::ZERO
            && !collision_unit_vector_is_valid(self.normal)
        {
            return Err(Error::invalid_argument(
                operation,
                "normal",
                "a finite unit vector, or zero for an initial overlap",
            ));
        }
        Ok(())
    }
}

/// Input for [`shape_distance`], evaluated entirely in shape A's local frame.
#[doc(alias = "distance_input")]
#[derive(Copy, Clone, Debug)]
pub struct DistanceInput {
    pub(crate) proxy_a: ShapeProxy,
    pub(crate) proxy_b: ShapeProxy,
    /// Transform of shape B in shape A's local frame.
    pub(crate) transform_b_in_a: Transform,
    pub(crate) use_radii: bool,
}

impl DistanceInput {
    /// Build distance input with `use_radii = false`.
    ///
    /// `transform_b_in_a` maps shape B's local coordinates into shape A's local frame.
    #[inline]
    pub fn new(
        proxy_a: ShapeProxy,
        proxy_b: ShapeProxy,
        transform_b_in_a: Transform,
    ) -> Result<Self> {
        let input = Self {
            proxy_a,
            proxy_b,
            transform_b_in_a,
            use_radii: false,
        };
        input.validate()?;
        Ok(input)
    }

    /// Set whether proxy radii should affect the distance result.
    #[inline]
    pub fn with_radii(mut self, use_radii: bool) -> Self {
        self.use_radii = use_radii;
        self
    }

    #[inline]
    pub const fn proxy_a(self) -> ShapeProxy {
        self.proxy_a
    }

    #[inline]
    pub const fn proxy_b(self) -> ShapeProxy {
        self.proxy_b
    }

    #[inline]
    pub const fn transform_b_in_a(self) -> Transform {
        self.transform_b_in_a
    }

    #[inline]
    pub const fn use_radii(self) -> bool {
        self.use_radii
    }

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> Result<()> {
        self.proxy_a.validate()?;
        self.proxy_b.validate()?;
        check_collision_transform_valid("DistanceInput::validate", self.transform_b_in_a)?;
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
    pub fn from_raw(raw: ffi::b2DistanceOutput) -> Result<Self> {
        let output = Self {
            point_a: Vec2::from_raw(raw.pointA),
            point_b: Vec2::from_raw(raw.pointB),
            normal: Vec2::from_raw(raw.normal),
            distance: raw.distance,
            iterations: raw.iterations,
            simplex_count: raw.simplexCount,
        };
        output.validate_for("DistanceOutput::from_raw")?;
        Ok(output)
    }

    fn from_native(operation: &'static str, raw: ffi::b2DistanceOutput) -> Result<Self> {
        let output = Self {
            point_a: Vec2::from_raw(raw.pointA),
            point_b: Vec2::from_raw(raw.pointB),
            normal: Vec2::from_raw(raw.normal),
            distance: raw.distance,
            iterations: raw.iterations,
            simplex_count: raw.simplexCount,
        };
        output
            .validate_for(operation)
            .map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "distance_output",
                constraint: "finite points and distance with valid normal and non-negative counters",
            })?;
        Ok(output)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_for("DistanceOutput::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        check_collision_vec2_valid(operation, "point_a", self.point_a)?;
        check_collision_vec2_valid(operation, "point_b", self.point_b)?;
        check_collision_vec2_valid(operation, "normal", self.normal)?;
        check_collision_non_negative_finite_scalar(operation, "distance", self.distance)?;
        check_collision_non_negative_int(operation, "iterations", self.iterations)?;
        check_collision_non_negative_int(operation, "simplex_count", self.simplex_count)?;
        if self.distance > 0.0 && !collision_unit_vector_is_valid(self.normal) {
            return Err(Error::invalid_argument(
                operation,
                "normal",
                "a finite unit vector when distance is positive",
            ));
        }
        if self.distance == 0.0
            && self.normal != Vec2::ZERO
            && !collision_unit_vector_is_valid(self.normal)
        {
            return Err(Error::invalid_argument(
                operation,
                "normal",
                "a finite unit vector, or zero when distance is zero",
            ));
        }
        Ok(())
    }
}

/// Input for [`shape_cast`], evaluated entirely in shape A's local frame.
#[doc(alias = "shape_cast_pair_input")]
#[derive(Copy, Clone, Debug)]
pub struct ShapeCastPairInput {
    pub(crate) proxy_a: ShapeProxy,
    pub(crate) proxy_b: ShapeProxy,
    /// Transform of shape B in shape A's local frame.
    pub(crate) transform_b_in_a: Transform,
    /// Translation of shape B expressed in shape A's local frame.
    pub(crate) translation_b_in_a: Vec2,
    pub(crate) max_fraction: f32,
    pub(crate) can_encroach: bool,
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
    ) -> Result<Self> {
        let input = Self {
            proxy_a,
            proxy_b,
            transform_b_in_a,
            translation_b_in_a: translation_b_in_a.into(),
            max_fraction: 1.0,
            can_encroach: false,
        };
        input.validate()?;
        Ok(input)
    }

    /// Limit the portion of `translation_b_in_a` considered by the cast.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Result<Self> {
        check_collision_unit_interval_scalar(
            "ShapeCastPairInput::with_max_fraction",
            "max_fraction",
            max_fraction,
        )?;
        self.max_fraction = max_fraction;
        Ok(self)
    }

    /// Allow shapes with radius to encroach slightly when initially touching.
    #[inline]
    pub fn with_can_encroach(mut self, can_encroach: bool) -> Self {
        self.can_encroach = can_encroach;
        self
    }

    #[inline]
    pub const fn proxy_a(self) -> ShapeProxy {
        self.proxy_a
    }

    #[inline]
    pub const fn proxy_b(self) -> ShapeProxy {
        self.proxy_b
    }

    #[inline]
    pub const fn transform_b_in_a(self) -> Transform {
        self.transform_b_in_a
    }

    #[inline]
    pub const fn translation_b_in_a(self) -> Vec2 {
        self.translation_b_in_a
    }

    #[inline]
    pub const fn max_fraction(self) -> f32 {
        self.max_fraction
    }

    #[inline]
    pub const fn can_encroach(self) -> bool {
        self.can_encroach
    }

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> Result<()> {
        self.proxy_a.validate()?;
        self.proxy_b.validate()?;
        check_collision_transform_valid("ShapeCastPairInput::validate", self.transform_b_in_a)?;
        check_collision_vec2_valid(
            "ShapeCastPairInput::validate",
            "translation_b_in_a",
            self.translation_b_in_a,
        )?;
        check_collision_unit_interval_scalar(
            "ShapeCastPairInput::validate",
            "max_fraction",
            self.max_fraction,
        )?;
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
    pub(crate) local_center: Vec2,
    pub(crate) c1: Vec2,
    pub(crate) c2: Vec2,
    pub(crate) q1: Rot,
    pub(crate) q2: Rot,
}

impl Sweep {
    #[inline]
    pub fn new<LC: Into<Vec2>, C1: Into<Vec2>, C2: Into<Vec2>>(
        local_center: LC,
        c1: C1,
        c2: C2,
        q1: Rot,
        q2: Rot,
    ) -> Result<Self> {
        let sweep = Self {
            local_center: local_center.into(),
            c1: c1.into(),
            c2: c2.into(),
            q1,
            q2,
        };
        sweep.validate()?;
        Ok(sweep)
    }

    #[inline]
    /// Construct from a raw Box2D sweep after validating its invariants.
    pub fn from_raw(raw: ffi::b2Sweep) -> Result<Self> {
        let sweep = Self {
            local_center: Vec2::from_raw(raw.localCenter),
            c1: Vec2::from_raw(raw.c1),
            c2: Vec2::from_raw(raw.c2),
            q1: Rot::from_raw(raw.q1)?,
            q2: Rot::from_raw(raw.q2)?,
        };
        sweep.validate()?;
        Ok(sweep)
    }

    #[inline]
    pub const fn local_center(self) -> Vec2 {
        self.local_center
    }

    #[inline]
    pub const fn start_center(self) -> Vec2 {
        self.c1
    }

    #[inline]
    pub const fn end_center(self) -> Vec2 {
        self.c2
    }

    #[inline]
    pub const fn start_rotation(self) -> Rot {
        self.q1
    }

    #[inline]
    pub const fn end_rotation(self) -> Rot {
        self.q2
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
    pub fn validate(&self) -> Result<()> {
        check_collision_vec2_valid("Sweep::validate", "local_center", self.local_center)?;
        check_collision_vec2_valid("Sweep::validate", "c1", self.c1)?;
        check_collision_vec2_valid("Sweep::validate", "c2", self.c2)?;
        check_collision_rot_valid("Sweep::validate", "q1", self.q1)?;
        check_collision_rot_valid("Sweep::validate", "q2", self.q2)?;
        Ok(())
    }

    /// Evaluate the sweep transform at `time` in the `[0, 1]` interval.
    ///
    /// The complete sweep and time are validated before foundation activity is leased and before
    /// Box2D is called.
    #[inline]
    pub fn transform_at(self, time: f32) -> Result<Transform> {
        self.validate()?;
        check_collision_unit_interval_scalar("Sweep::transform_at", "time", time)?;
        let _lease = transient_native_lease()?;
        let raw = self.into_raw();
        Transform::from_raw(unsafe { ffi::b2GetSweepTransform(&raw, time) }).map_err(|_| {
            Error::InvalidNativeOutput {
                operation: "Sweep::transform_at",
                output: "transform",
                constraint: "a finite rigid transform",
            }
        })
    }
}

/// Input for [`time_of_impact`].
#[doc(alias = "toi_input")]
#[derive(Copy, Clone, Debug)]
pub struct ToiInput {
    pub(crate) proxy_a: ShapeProxy,
    pub(crate) proxy_b: ShapeProxy,
    pub(crate) sweep_a: Sweep,
    pub(crate) sweep_b: Sweep,
    pub(crate) max_fraction: f32,
}

impl ToiInput {
    /// Build TOI input with `max_fraction = 1.0`.
    #[inline]
    pub fn new(
        proxy_a: ShapeProxy,
        proxy_b: ShapeProxy,
        sweep_a: Sweep,
        sweep_b: Sweep,
    ) -> Result<Self> {
        let input = Self {
            proxy_a,
            proxy_b,
            sweep_a,
            sweep_b,
            max_fraction: 1.0,
        };
        input.validate()?;
        Ok(input)
    }

    /// Limit the sweep interval to `[0, max_fraction]`.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Result<Self> {
        check_collision_unit_interval_scalar(
            "ToiInput::with_max_fraction",
            "max_fraction",
            max_fraction,
        )?;
        self.max_fraction = max_fraction;
        Ok(self)
    }

    #[inline]
    pub const fn proxy_a(self) -> ShapeProxy {
        self.proxy_a
    }

    #[inline]
    pub const fn proxy_b(self) -> ShapeProxy {
        self.proxy_b
    }

    #[inline]
    pub const fn sweep_a(self) -> Sweep {
        self.sweep_a
    }

    #[inline]
    pub const fn sweep_b(self) -> Sweep {
        self.sweep_b
    }

    #[inline]
    pub const fn max_fraction(self) -> f32 {
        self.max_fraction
    }

    /// Validate this input before crossing the Box2D FFI boundary.
    pub fn validate(&self) -> Result<()> {
        self.proxy_a.validate()?;
        self.proxy_b.validate()?;
        self.sweep_a.validate()?;
        self.sweep_b.validate()?;
        check_collision_unit_interval_scalar(
            "ToiInput::validate",
            "max_fraction",
            self.max_fraction,
        )?;
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
    pub const fn from_raw(raw: ffi::b2TOIState) -> Option<Self> {
        match raw {
            ffi::b2TOIState_b2_toiStateUnknown => Some(Self::Unknown),
            ffi::b2TOIState_b2_toiStateFailed => Some(Self::Failed),
            ffi::b2TOIState_b2_toiStateOverlapped => Some(Self::Overlapped),
            ffi::b2TOIState_b2_toiStateHit => Some(Self::Hit),
            ffi::b2TOIState_b2_toiStateSeparated => Some(Self::Separated),
            _ => None,
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
    pub fn from_raw(raw: ffi::b2TOIOutput) -> Result<Self> {
        let output = Self {
            state: ToiState::from_raw(raw.state).ok_or_else(|| {
                Error::invalid_argument("ToiOutput::from_raw", "state", "a known Box2D TOI state")
            })?,
            point: Vec2::from_raw(raw.point),
            normal: Vec2::from_raw(raw.normal),
            fraction: raw.fraction,
        };
        output.validate_for("ToiOutput::from_raw")?;
        Ok(output)
    }

    fn from_native(operation: &'static str, raw: ffi::b2TOIOutput) -> Result<Self> {
        let state = ToiState::from_raw(raw.state).ok_or(Error::InvalidNativeOutput {
            operation,
            output: "toi_output.state",
            constraint: "a known Box2D TOI state",
        })?;
        let output = Self {
            state,
            point: Vec2::from_raw(raw.point),
            normal: Vec2::from_raw(raw.normal),
            fraction: raw.fraction,
        };
        output
            .validate_for(operation)
            .map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "toi_output",
                constraint: "finite hit data and a unit-interval fraction",
            })?;
        Ok(output)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_for("ToiOutput::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        check_collision_vec2_valid(operation, "point", self.point)?;
        check_collision_vec2_valid(operation, "normal", self.normal)?;
        check_collision_unit_interval_scalar(operation, "fraction", self.fraction)?;
        if self.state == ToiState::Hit && !collision_unit_vector_is_valid(self.normal) {
            return Err(Error::invalid_argument(
                operation,
                "normal",
                "a finite unit vector for a TOI hit",
            ));
        }
        Ok(())
    }
}

/// Compute the closest points between two line segments.
pub fn segment_distance<P1, Q1, P2, Q2>(
    p1: P1,
    q1: Q1,
    p2: P2,
    q2: Q2,
) -> Result<SegmentDistanceResult>
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
    check_collision_vec2_valid("segment_distance", "p1", p1)?;
    check_collision_vec2_valid("segment_distance", "q1", q1)?;
    check_collision_vec2_valid("segment_distance", "p2", p2)?;
    check_collision_vec2_valid("segment_distance", "q2", q2)?;
    let _lease = transient_native_lease()?;
    SegmentDistanceResult::from_native("segment_distance", unsafe {
        ffi::b2SegmentDistance(p1.into_raw(), q1.into_raw(), p2.into_raw(), q2.into_raw())
    })
}

/// Compute the closest distance between two shape proxies.
pub fn shape_distance(input: DistanceInput, cache: &mut SimplexCache) -> Result<DistanceOutput> {
    input.validate()?;
    let proxy_a_count = input.proxy_a.count();
    let proxy_b_count = input.proxy_b.count();
    cache.validate_for("shape_distance", proxy_a_count, proxy_b_count)?;
    let raw_input = input.into_raw();
    let mut staged_cache = *cache;
    let _lease = transient_native_lease()?;
    let output = DistanceOutput::from_native("shape_distance", unsafe {
        ffi::b2ShapeDistance(&raw_input, staged_cache.raw_mut(), core::ptr::null_mut(), 0)
    })?;
    if output.simplex_count != 0 {
        return Err(Error::InvalidNativeOutput {
            operation: "shape_distance",
            output: "distance_output.simplex_count",
            constraint: "zero when no simplex output buffer was supplied",
        });
    }
    commit_native_simplex_cache(
        Some(cache),
        staged_cache,
        "shape_distance",
        proxy_a_count,
        proxy_b_count,
    )?;
    Ok(output)
}

/// Cast shape B against shape A.
///
/// The hit point and normal are returned in shape A's local frame.
pub fn shape_cast(input: ShapeCastPairInput) -> Result<CastOutput> {
    input.validate()?;
    let raw_input = input.into_raw();
    let _lease = transient_native_lease()?;
    CastOutput::from_native("shape_cast", unsafe { ffi::b2ShapeCast(&raw_input) })
}

/// Compute the time of impact between two moving shape proxies.
pub fn time_of_impact(input: ToiInput) -> Result<ToiOutput> {
    input.validate()?;
    let raw_input = input.into_raw();
    let _lease = transient_native_lease()?;
    ToiOutput::from_native("time_of_impact", unsafe { ffi::b2TimeOfImpact(&raw_input) })
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
) -> Result<LocalManifold> {
    circle_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid("collide_circles", transform_b_in_a)?;
    let raw_a = circle_a.into_raw();
    let raw_b = circle_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_circles", unsafe {
        ffi::b2CollideCircles(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    capsule_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid("collide_capsule_and_circle", transform_b_in_a)?;
    let raw_a = capsule_a.into_raw();
    let raw_b = circle_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_capsule_and_circle", unsafe {
        ffi::b2CollideCapsuleAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    segment_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid("collide_segment_and_circle", transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_segment_and_circle", unsafe {
        ffi::b2CollideSegmentAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    polygon_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid("collide_polygon_and_circle", transform_b_in_a)?;
    let raw_a = polygon_a.into_raw();
    let raw_b = circle_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_polygon_and_circle", unsafe {
        ffi::b2CollidePolygonAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    capsule_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid("collide_capsules", transform_b_in_a)?;
    let raw_a = capsule_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_capsules", unsafe {
        ffi::b2CollideCapsules(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    segment_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid("collide_segment_and_capsule", transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_segment_and_capsule", unsafe {
        ffi::b2CollideSegmentAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    polygon_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid("collide_polygon_and_capsule", transform_b_in_a)?;
    let raw_a = polygon_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_polygon_and_capsule", unsafe {
        ffi::b2CollidePolygonAndCapsule(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    polygon_a.validate()?;
    polygon_b.validate()?;
    check_collision_transform_valid("collide_polygons", transform_b_in_a)?;
    let raw_a = polygon_a.into_raw();
    let raw_b = polygon_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_polygons", unsafe {
        ffi::b2CollidePolygons(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    segment_a.validate()?;
    polygon_b.validate()?;
    check_collision_transform_valid("collide_segment_and_polygon", transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_segment_and_polygon", unsafe {
        ffi::b2CollideSegmentAndPolygon(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    segment_a.validate()?;
    circle_b.validate()?;
    check_collision_transform_valid("collide_chain_segment_and_circle", transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = circle_b.into_raw();
    let _lease = transient_native_lease()?;
    LocalManifold::from_native("collide_chain_segment_and_circle", unsafe {
        ffi::b2CollideChainSegmentAndCircle(&raw_a, &raw_b, transform_b_in_a.into_raw())
    })
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
) -> Result<LocalManifold> {
    segment_a.validate()?;
    capsule_b.validate()?;
    check_collision_transform_valid("collide_chain_segment_and_capsule", transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = capsule_b.into_raw();
    let mut staged_cache = cache.as_deref().copied().unwrap_or_default();
    staged_cache.validate_for("collide_chain_segment_and_capsule", 2, 2)?;
    let _lease = transient_native_lease()?;
    let manifold = LocalManifold::from_native("collide_chain_segment_and_capsule", unsafe {
        ffi::b2CollideChainSegmentAndCapsule(
            &raw_a,
            &raw_b,
            transform_b_in_a.into_raw(),
            staged_cache.raw_mut(),
        )
    })?;
    commit_native_simplex_cache(
        cache,
        staged_cache,
        "collide_chain_segment_and_capsule",
        2,
        2,
    )?;
    Ok(manifold)
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
) -> Result<LocalManifold> {
    segment_a.validate()?;
    polygon_b.validate()?;
    check_collision_transform_valid("collide_chain_segment_and_polygon", transform_b_in_a)?;
    let raw_a = segment_a.into_raw();
    let raw_b = polygon_b.into_raw();
    let proxy_b_count = usize::try_from(raw_b.count).map_err(|_| {
        Error::invalid_argument(
            "collide_chain_segment_and_polygon",
            "polygon_b",
            "a polygon with a representable point count",
        )
    })?;
    let mut staged_cache = cache.as_deref().copied().unwrap_or_default();
    staged_cache.validate_for("collide_chain_segment_and_polygon", 2, proxy_b_count)?;
    let _lease = transient_native_lease()?;
    let manifold = LocalManifold::from_native("collide_chain_segment_and_polygon", unsafe {
        ffi::b2CollideChainSegmentAndPolygon(
            &raw_a,
            &raw_b,
            transform_b_in_a.into_raw(),
            staged_cache.raw_mut(),
        )
    })?;
    commit_native_simplex_cache(
        cache,
        staged_cache,
        "collide_chain_segment_and_polygon",
        2,
        proxy_b_count,
    )?;
    Ok(manifold)
}

impl Aabb {
    /// Check whether this AABB is valid for Box2D queries.
    #[inline]
    pub fn is_valid(self) -> bool {
        let width = self.upper.x - self.lower.x;
        let height = self.upper.y - self.lower.y;
        width >= 0.0 && height >= 0.0 && self.lower.is_valid() && self.upper.is_valid()
    }

    /// Validate this AABB for collision queries.
    #[inline]
    pub fn validate(self) -> Result<()> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(Error::invalid_argument(
                "Aabb::validate",
                "self",
                "finite ordered lower and upper bounds",
            ))
        }
    }

    /// Ray cast against this AABB using Box2D-style `origin + translation`.
    ///
    /// Initial overlap returns a hit with zero fraction, zero normal, and `point = origin`.
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
    ) -> Result<CastOutput> {
        let origin = origin.into();
        let translation = translation.into();
        self.validate()?;
        check_collision_vec2_valid("Aabb::ray_cast", "origin", origin)?;
        check_collision_vec2_valid("Aabb::ray_cast", "translation", translation)?;
        Ok(self.ray_cast_validated(origin, translation))
    }

    fn ray_cast_validated(self, origin: Vec2, translation: Vec2) -> CastOutput {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_proxy_owns_point_validation_and_storage_invariants() {
        let maximum =
            ShapeProxy::new((0..MAX_SHAPE_PROXY_POINTS).map(|_| [0.0_f32, 0.0]), 0.25).unwrap();
        assert_eq!(maximum.count(), MAX_SHAPE_PROXY_POINTS);

        assert_eq!(
            ShapeProxy::new((0..=MAX_SHAPE_PROXY_POINTS).map(|_| [0.0_f32, 0.0]), 0.25,)
                .unwrap_err(),
            Error::invalid_argument(
                "ShapeProxy::new",
                "points",
                "no more than Box2D's maximum shape-proxy point count",
            )
        );

        let raw = ShapeProxy::new([[1.0_f32, 2.0]], 0.25).unwrap().into_raw();
        assert_eq!(raw.count, 1);
        assert_eq!(raw.radius, 0.25);
        assert_eq!(raw.points[0].x, 1.0);
        assert_eq!(raw.points[0].y, 2.0);
        assert!(
            raw.points[1..]
                .iter()
                .all(|point| point.x == 0.0 && point.y == 0.0)
        );

        assert_eq!(
            ShapeProxy::offset_from_points(
                [[f32::MAX, 0.0]],
                0.0,
                Transform::from_pos_angle([f32::MAX, 0.0], 0.0).unwrap(),
            )
            .unwrap_err(),
            Error::invalid_argument(
                "ShapeProxy::offset_from_points",
                "points/transform",
                "a transform whose proxy points remain finite",
            )
        );
    }

    #[test]
    fn invalid_native_simplex_cache_is_not_published() {
        let original = SimplexCache {
            raw: ffi::b2SimplexCache {
                count: 1,
                indexA: [0, 0, 0],
                indexB: [0, 0, 0],
            },
        };

        for staged in [
            SimplexCache {
                raw: ffi::b2SimplexCache {
                    count: 4,
                    indexA: [0, 0, 0],
                    indexB: [0, 0, 0],
                },
            },
            SimplexCache {
                raw: ffi::b2SimplexCache {
                    count: 1,
                    indexA: [2, 0, 0],
                    indexB: [0, 0, 0],
                },
            },
        ] {
            let mut target = original;
            assert_eq!(
                commit_native_simplex_cache(Some(&mut target), staged, "test_query", 2, 2),
                Err(Error::InvalidNativeOutput {
                    operation: "test_query",
                    output: "simplex_cache",
                    constraint: "at most three in-range proxy point indices",
                })
            );
            assert_eq!(target.raw.count, original.raw.count);
            assert_eq!(target.raw.indexA, original.raw.indexA);
            assert_eq!(target.raw.indexB, original.raw.indexB);
        }
    }

    #[test]
    fn aabb_validation_matches_upstream_finite_and_ordering_rules() {
        assert!(
            Aabb::new([0.0_f32, 0.0], [0.0_f32, 0.0])
                .unwrap()
                .is_valid()
        );
        assert!(
            Aabb::new([f32::MIN, f32::MIN], [f32::MAX, f32::MAX])
                .unwrap()
                .is_valid()
        );

        for invalid in [
            Aabb::new([1.0_f32, 0.0], [0.0_f32, 1.0]),
            Aabb::new([0.0_f32, 1.0], [1.0_f32, 0.0]),
            Aabb::new([f32::NAN, 0.0], [1.0_f32, 1.0]),
            Aabb::new([0.0_f32, 0.0], [f32::INFINITY, 1.0]),
            Aabb::new([f32::NEG_INFINITY, 0.0], [1.0_f32, 1.0]),
        ] {
            assert!(invalid.is_err());
        }
    }

    #[test]
    fn worldless_native_collision_calls_obey_the_callback_gate() {
        crate::Foundation::initialize_default().unwrap();

        let proxy = ShapeProxy::new([[0.0_f32, 0.0]], 0.0).unwrap();
        let circle = Circle::new([0.0_f32, 0.0], 0.5).unwrap();
        let invalid_circle = Circle {
            center: Vec2::new(f32::NAN, 0.0),
            radius: 0.5,
        };
        let aabb = Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap();

        {
            let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
            let proxy_input_was_materialized = std::cell::Cell::new(false);

            assert_eq!(
                segment_distance(
                    [0.0_f32, 0.0],
                    [1.0_f32, 0.0],
                    [0.0_f32, 1.0],
                    [1.0_f32, 1.0],
                )
                .unwrap_err(),
                Error::InCallback
            );
            assert_eq!(
                collide_circles(circle, circle, Transform::IDENTITY).unwrap_err(),
                Error::InCallback
            );
            assert_eq!(
                collide_circles(invalid_circle, circle, Transform::IDENTITY).unwrap_err(),
                Error::invalid_argument(
                    "Circle::validate",
                    "circle",
                    "finite center coordinates and a finite non-negative radius",
                )
            );
            let callback_proxy = ShapeProxy::new(
                core::iter::once_with(|| {
                    proxy_input_was_materialized.set(true);
                    [0.0_f32, 0.0]
                }),
                0.0,
            )
            .unwrap();
            assert_eq!(callback_proxy.points(), &[Vec2::ZERO]);
            assert!(proxy_input_was_materialized.get());
            assert!(aabb.is_valid());
            assert!(aabb.ray_cast([-2.0_f32, 0.0], [4.0_f32, 0.0]).unwrap().hit);
            assert_eq!(
                segment_distance(
                    [f32::NAN, 0.0],
                    [1.0_f32, 0.0],
                    [0.0_f32, 1.0],
                    [1.0_f32, 1.0],
                )
                .unwrap_err(),
                Error::invalid_argument("segment_distance", "p1", "a finite vector")
            );
        }

        assert!(
            shape_distance(
                DistanceInput::new(proxy, proxy, Transform::IDENTITY).unwrap(),
                &mut SimplexCache::default(),
            )
            .is_ok()
        );
    }

    #[test]
    fn sweep_transform_validation_precedes_foundation_activity() {
        let valid_sweep = Sweep::new(
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [1.0_f32, 0.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        )
        .unwrap();
        let invalid_sweep = Sweep {
            local_center: Vec2::new(f32::NAN, 0.0),
            c1: Vec2::ZERO,
            c2: Vec2::new(1.0, 0.0),
            q1: Rot::IDENTITY,
            q2: Rot::IDENTITY,
        };
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            invalid_sweep.transform_at(0.5).unwrap_err(),
            Error::invalid_argument("Sweep::validate", "local_center", "a finite vector",)
        );
        assert_eq!(
            valid_sweep.transform_at(f32::NAN).unwrap_err(),
            Error::invalid_argument("Sweep::transform_at", "time", "a finite value in 0.0..=1.0",)
        );
        assert_eq!(
            valid_sweep.transform_at(0.5).unwrap_err(),
            Error::InCallback
        );
    }
}
