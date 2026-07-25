use crate::core::foundation::{assert_transient_native_lease, transient_native_lease};
use crate::error::ApiResult;
use crate::types::{Position, ShapeId, Vec2};
use boxdd_sys::ffi;

pub(super) fn minimum_mover_radius() -> f32 {
    0.01 * crate::length_units_per_meter()
}

pub(super) fn assert_query_vec2_valid(name: &str, value: Vec2) {
    assert!(
        value.is_valid(),
        "{name} must be a valid Box2D vector, got {:?}",
        value
    );
}
pub(super) fn check_query_vec2_valid(value: Vec2) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(super) fn assert_query_position_valid(name: &str, value: Position) {
    assert!(
        value.is_valid(),
        "{name} must be a valid Box2D position, got {:?}",
        value
    );
}

pub(super) fn check_query_position_valid(value: Position) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn assert_query_aabb_valid(aabb: Aabb) {
    assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn check_query_aabb_valid(aabb: Aabb) -> ApiResult<()> {
    if aabb.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn assert_query_non_negative_finite_scalar(name: &str, value: f32) {
    assert!(
        crate::is_valid_float(value) && value >= 0.0,
        "{name} must be finite and >= 0.0, got {value}"
    );
}

#[inline]
pub(super) fn check_query_non_negative_finite_scalar(value: f32) -> ApiResult<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn assert_query_angle_valid(angle_radians: f32) {
    assert!(
        crate::is_valid_float(angle_radians),
        "angle_radians must be finite, got {angle_radians}"
    );
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn check_query_angle_valid(angle_radians: f32) -> ApiResult<()> {
    if crate::is_valid_float(angle_radians) {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
pub(super) fn assert_query_mover_radius_valid(radius: f32) {
    let minimum = minimum_mover_radius();
    assert!(
        crate::is_valid_float(radius) && radius > minimum,
        "mover radius must be finite and > {minimum}, got {radius}"
    );
}

#[inline]
pub(super) fn check_query_mover_radius_valid(radius: f32) -> ApiResult<()> {
    if crate::is_valid_float(radius) && radius > minimum_mover_radius() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

/// Axis-aligned bounding box with `f32` coordinates in both precision modes.
///
/// When Box2D computes one from a [`crate::WorldTransform`] in double-precision mode, it narrows
/// the absolute bounds outward so this conservative box still contains the represented geometry.
#[doc(alias = "aabb")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub lower: Vec2,
    pub upper: Vec2,
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Aabb {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for Aabb {}

#[cfg(feature = "bytemuck")]
const _: () = {
    assert!(core::mem::size_of::<Aabb>() == 16);
    assert!(core::mem::align_of::<Aabb>() == 4);
};

impl Aabb {
    #[inline]
    pub fn from_raw(raw: ffi::b2AABB) -> Self {
        Self {
            lower: Vec2::from_raw(raw.lowerBound),
            upper: Vec2::from_raw(raw.upperBound),
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2AABB {
        ffi::b2AABB {
            lowerBound: self.lower.into_raw(),
            upperBound: self.upper.into_raw(),
        }
    }

    /// Create an AABB from lower and upper points.
    #[inline]
    pub fn new<L: Into<Vec2>, U: Into<Vec2>>(lower: L, upper: U) -> Self {
        Self {
            lower: lower.into(),
            upper: upper.into(),
        }
    }
    /// Create an AABB from center and half-extents (both in world units).
    #[inline]
    pub fn from_center_half_extents<C: Into<Vec2>, H: Into<Vec2>>(center: C, half: H) -> Self {
        let c = center.into();
        let h = half.into();
        Self {
            lower: Vec2::new(c.x - h.x, c.y - h.y),
            upper: Vec2::new(c.x + h.x, c.y + h.y),
        }
    }
}

#[cfg(feature = "mint")]
impl From<Aabb> for (mint::Point2<f32>, mint::Point2<f32>) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "mint")]
impl From<(mint::Point2<f32>, mint::Point2<f32>)> for Aabb {
    #[inline]
    fn from((lower, upper): (mint::Point2<f32>, mint::Point2<f32>)) -> Self {
        Self::new(lower, upper)
    }
}

#[cfg(feature = "mint")]
impl From<Aabb> for (mint::Vector2<f32>, mint::Vector2<f32>) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "mint")]
impl From<(mint::Vector2<f32>, mint::Vector2<f32>)> for Aabb {
    #[inline]
    fn from((lower, upper): (mint::Vector2<f32>, mint::Vector2<f32>)) -> Self {
        Self::new(lower, upper)
    }
}

#[cfg(feature = "glam")]
impl From<Aabb> for (glam::Vec2, glam::Vec2) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "glam")]
impl From<(glam::Vec2, glam::Vec2)> for Aabb {
    #[inline]
    fn from((lower, upper): (glam::Vec2, glam::Vec2)) -> Self {
        Self {
            lower: lower.into(),
            upper: upper.into(),
        }
    }
}

#[cfg(feature = "nalgebra")]
impl From<Aabb> for (nalgebra::Point2<f32>, nalgebra::Point2<f32>) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "nalgebra")]
impl From<(nalgebra::Point2<f32>, nalgebra::Point2<f32>)> for Aabb {
    #[inline]
    fn from((lower, upper): (nalgebra::Point2<f32>, nalgebra::Point2<f32>)) -> Self {
        Self::new(lower, upper)
    }
}

#[cfg(feature = "nalgebra")]
impl From<Aabb> for (nalgebra::Vector2<f32>, nalgebra::Vector2<f32>) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "nalgebra")]
impl From<(nalgebra::Vector2<f32>, nalgebra::Vector2<f32>)> for Aabb {
    #[inline]
    fn from((lower, upper): (nalgebra::Vector2<f32>, nalgebra::Vector2<f32>)) -> Self {
        Self::new(lower, upper)
    }
}

/// Filter for queries
#[doc(alias = "query_filter")]
#[derive(Copy, Clone, Debug)]
pub struct QueryFilter(pub(crate) ffi::b2QueryFilter);

impl Default for QueryFilter {
    fn default() -> Self {
        let _lease = assert_transient_native_lease();
        Self(unsafe { ffi::b2DefaultQueryFilter() })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for QueryFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            category_bits: u64,
            mask_bits: u64,
        }
        Repr {
            category_bits: self.0.categoryBits,
            mask_bits: self.0.maskBits,
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for QueryFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            category_bits: u64,
            mask_bits: u64,
        }
        let r = Repr::deserialize(deserializer)?;
        Ok(Self(ffi::b2QueryFilter {
            categoryBits: r.category_bits,
            maskBits: r.mask_bits,
        }))
    }
}

impl QueryFilter {
    /// Create a query filter using Box2D's defaults.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn category_bits(&self) -> u64 {
        self.0.categoryBits
    }

    pub fn mask_bits(&self) -> u64 {
        self.0.maskBits
    }

    pub fn mask(mut self, bits: u64) -> Self {
        self.0.maskBits = bits;
        self
    }
    pub fn category(mut self, bits: u64) -> Self {
        self.0.categoryBits = bits;
        self
    }
}

/// Result of a closest ray cast
#[doc(alias = "ray_result")]
#[derive(Copy, Clone, Debug)]
pub struct RayResult {
    pub shape_id: ShapeId,
    /// Absolute world-space hit position.
    pub point: Position,
    /// Unit surface normal in world orientation.
    pub normal: Vec2,
    /// Fraction of the query translation at which the hit occurred.
    pub fraction: f32,
    pub hit: bool,
}

impl RayResult {
    #[inline]
    pub(crate) fn from_raw_in(
        brand: crate::id::IdBrand,
        raw: ffi::b2RayResult,
    ) -> crate::error::ApiResult<Option<Self>> {
        if !raw.hit {
            return Ok(None);
        }

        Ok(Some(Self {
            shape_id: brand.try_shape(raw.shapeId)?,
            point: Position::from_raw(raw.point),
            normal: Vec2::from_raw(raw.normal),
            fraction: raw.fraction,
            hit: true,
        }))
    }
}

/// Complete result of a closest ray cast, including broad-phase traversal statistics.
///
/// The statistics are retained even when [`Self::hit`] is `None`.
#[doc(alias = "closest_ray_cast_result")]
#[derive(Copy, Clone, Debug)]
pub struct ClosestRayCastResult {
    /// Closest shape hit, or `None` when the ray did not hit a shape.
    pub hit: Option<RayResult>,
    /// Number of broad-phase tree nodes visited by Box2D.
    pub node_visits: i32,
    /// Number of broad-phase leaves visited by Box2D.
    pub leaf_visits: i32,
}

impl ClosestRayCastResult {
    #[inline]
    pub(crate) fn from_raw_in(
        brand: crate::id::IdBrand,
        raw: ffi::b2RayResult,
    ) -> crate::error::ApiResult<Self> {
        Ok(Self {
            hit: RayResult::from_raw_in(brand, raw)?,
            node_visits: raw.nodeVisits,
            leaf_visits: raw.leafVisits,
        })
    }
}

/// A collision plane used by Box2D's character mover helpers.
#[doc(alias = "plane")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec2,
    pub offset: f32,
}

impl Plane {
    #[inline]
    pub fn new<N: Into<Vec2>>(normal: N, offset: f32) -> Self {
        Self {
            normal: normal.into(),
            offset,
        }
    }

    /// Return whether this plane is valid for Box2D mover algorithms.
    #[inline]
    pub fn is_valid(self) -> bool {
        self.normal.is_valid()
            && (1.0 - (self.normal.x * self.normal.x + self.normal.y * self.normal.y)).abs()
                < 100.0 * f32::EPSILON
            && self.offset.is_finite()
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2Plane) -> Self {
        Self {
            normal: Vec2::from_raw(raw.normal),
            offset: raw.offset,
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2Plane {
        ffi::b2Plane {
            normal: self.normal.into_raw(),
            offset: self.offset,
        }
    }
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Plane {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for Plane {}

const _: () = {
    assert!(core::mem::size_of::<Plane>() == core::mem::size_of::<ffi::b2Plane>());
    assert!(core::mem::align_of::<Plane>() == core::mem::align_of::<ffi::b2Plane>());
};

/// Result item returned by `collide_mover`.
#[doc(alias = "plane_result")]
#[derive(Copy, Clone, Debug)]
pub struct MoverPlaneResult {
    pub shape_id: ShapeId,
    pub plane: Plane,
    /// Contact point relative to the `origin` supplied to `collide_mover`.
    pub point: Vec2,
    pub hit: bool,
}

impl MoverPlaneResult {
    /// Convert a valid mover-plane result into a collision plane for `solve_planes`.
    ///
    /// Returns `None` when `hit` is `false`, matching Box2D's guidance to ignore that result.
    #[inline]
    pub fn into_collision_plane(
        self,
        push_limit: f32,
        clip_velocity: bool,
    ) -> Option<CollisionPlane> {
        self.hit
            .then(|| CollisionPlane::new(self.plane, push_limit, clip_velocity))
    }

    /// Convert a valid mover-plane result into a rigid collision plane.
    ///
    /// This uses `f32::MAX` as the push limit and enables velocity clipping.
    #[inline]
    pub fn into_rigid_collision_plane(self) -> Option<CollisionPlane> {
        self.into_collision_plane(CollisionPlane::RIGID_PUSH_LIMIT, true)
    }
}

/// Collision plane input for `solve_planes` and `clip_vector`.
#[doc(alias = "collision_plane")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CollisionPlane {
    pub plane: Plane,
    pub push_limit: f32,
    pub push: f32,
    pub clip_velocity: bool,
}

impl CollisionPlane {
    pub const RIGID_PUSH_LIMIT: f32 = f32::MAX;

    #[inline]
    pub fn new(plane: Plane, push_limit: f32, clip_velocity: bool) -> Self {
        Self {
            plane,
            push_limit,
            push: 0.0,
            clip_velocity,
        }
    }

    #[inline]
    pub fn rigid(plane: Plane) -> Self {
        Self::new(plane, Self::RIGID_PUSH_LIMIT, true)
    }

    /// Validate this collision plane for Box2D mover solver helpers.
    pub fn validate(&self) -> ApiResult<()> {
        check_query_collision_plane_valid(self)
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2CollisionPlane) -> Self {
        Self {
            plane: Plane::from_raw(raw.plane),
            push_limit: raw.pushLimit,
            push: raw.push,
            clip_velocity: raw.clipVelocity,
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2CollisionPlane {
        ffi::b2CollisionPlane {
            plane: self.plane.into_raw(),
            pushLimit: self.push_limit,
            push: self.push,
            clipVelocity: self.clip_velocity,
        }
    }
}

#[inline]
pub(super) fn assert_query_solver_collision_plane_valid(plane: &CollisionPlane) {
    assert!(
        check_query_solver_collision_plane_valid(plane).is_ok(),
        "collision plane must be solver-valid, got {:?}",
        plane
    );
}

#[inline]
pub(super) fn check_query_solver_collision_plane_valid(plane: &CollisionPlane) -> ApiResult<()> {
    if !plane.plane.is_valid() {
        return Err(crate::error::ApiError::InvalidArgument);
    }
    check_query_non_negative_finite_scalar(plane.push_limit)
}

#[inline]
pub(super) fn assert_query_collision_plane_valid(plane: &CollisionPlane) {
    assert!(
        check_query_collision_plane_valid(plane).is_ok(),
        "collision plane must be valid, got {:?}",
        plane
    );
}

#[inline]
pub(super) fn check_query_collision_plane_valid(plane: &CollisionPlane) -> ApiResult<()> {
    check_query_solver_collision_plane_valid(plane)?;
    check_query_non_negative_finite_scalar(plane.push)
}

const _: () = {
    assert!(
        core::mem::size_of::<CollisionPlane>() == core::mem::size_of::<ffi::b2CollisionPlane>()
    );
    assert!(
        core::mem::align_of::<CollisionPlane>() == core::mem::align_of::<ffi::b2CollisionPlane>()
    );
};

/// Result returned by `solve_planes`.
#[doc(alias = "plane_solver_result")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PlaneSolverResult {
    pub translation: Vec2,
    pub iteration_count: i32,
}

impl PlaneSolverResult {
    #[inline]
    pub fn from_raw(raw: ffi::b2PlaneSolverResult) -> Self {
        Self {
            translation: Vec2::from_raw(raw.translation),
            iteration_count: raw.iterationCount,
        }
    }
}

#[inline]
pub(super) fn raw_collision_planes_mut(
    planes: &mut [CollisionPlane],
) -> *mut ffi::b2CollisionPlane {
    if planes.is_empty() {
        core::ptr::null_mut()
    } else {
        planes.as_mut_ptr().cast()
    }
}

#[inline]
pub(super) fn raw_collision_planes(planes: &[CollisionPlane]) -> *const ffi::b2CollisionPlane {
    if planes.is_empty() {
        core::ptr::null()
    } else {
        planes.as_ptr().cast()
    }
}

#[inline]
fn assert_collision_plane_count(count: usize) -> i32 {
    i32::try_from(count).expect("collision plane count exceeds Box2D limits")
}

#[inline]
fn check_collision_plane_count(count: usize) -> ApiResult<i32> {
    i32::try_from(count).map_err(|_| crate::error::ApiError::InvalidArgument)
}

/// Solve the translation that best satisfies the supplied mover collision planes.
///
/// The `push` field on each collision plane is updated in place by Box2D.
#[inline]
pub fn solve_planes<V: Into<Vec2>>(
    target_delta: V,
    planes: &mut [CollisionPlane],
) -> PlaneSolverResult {
    let target_delta = target_delta.into();
    assert_query_vec2_valid("target_delta", target_delta);
    let plane_count = assert_collision_plane_count(planes.len());
    for plane in planes.iter() {
        assert_query_solver_collision_plane_valid(plane);
    }
    let _lease = assert_transient_native_lease();
    let raw = unsafe {
        ffi::b2SolvePlanes(
            target_delta.into_raw(),
            raw_collision_planes_mut(planes),
            plane_count,
        )
    };
    PlaneSolverResult::from_raw(raw)
}

/// Solve the translation that best satisfies the supplied mover collision planes.
///
/// Returns `ApiError::InvalidArgument` when `target_delta` or any collision plane is invalid.
#[inline]
pub fn try_solve_planes<V: Into<Vec2>>(
    target_delta: V,
    planes: &mut [CollisionPlane],
) -> ApiResult<PlaneSolverResult> {
    let target_delta = target_delta.into();
    check_query_vec2_valid(target_delta)?;
    let plane_count = check_collision_plane_count(planes.len())?;
    for plane in planes.iter() {
        check_query_solver_collision_plane_valid(plane)?;
    }
    let _lease = transient_native_lease()?;
    let raw = unsafe {
        ffi::b2SolvePlanes(
            target_delta.into_raw(),
            raw_collision_planes_mut(planes),
            plane_count,
        )
    };
    Ok(PlaneSolverResult::from_raw(raw))
}

/// Clip a velocity or movement vector against solved collision planes.
#[inline]
pub fn clip_vector<V: Into<Vec2>>(vector: V, planes: &[CollisionPlane]) -> Vec2 {
    let vector = vector.into();
    assert_query_vec2_valid("vector", vector);
    let plane_count = assert_collision_plane_count(planes.len());
    for plane in planes.iter() {
        assert_query_collision_plane_valid(plane);
    }
    let _lease = assert_transient_native_lease();
    Vec2::from_raw(unsafe {
        ffi::b2ClipVector(vector.into_raw(), raw_collision_planes(planes), plane_count)
    })
}

/// Clip a velocity or movement vector against solved collision planes.
///
/// Returns `ApiError::InvalidArgument` when `vector` or any collision plane state is invalid.
#[inline]
pub fn try_clip_vector<V: Into<Vec2>>(vector: V, planes: &[CollisionPlane]) -> ApiResult<Vec2> {
    let vector = vector.into();
    check_query_vec2_valid(vector)?;
    let plane_count = check_collision_plane_count(planes.len())?;
    for plane in planes.iter() {
        check_query_collision_plane_valid(plane)?;
    }
    let _lease = transient_native_lease()?;
    Ok(Vec2::from_raw(unsafe {
        ffi::b2ClipVector(vector.into_raw(), raw_collision_planes(planes), plane_count)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct TrackedVec2 {
        converted: Arc<AtomicBool>,
        value: Vec2,
    }

    impl From<TrackedVec2> for Vec2 {
        fn from(value: TrackedVec2) -> Self {
            value.converted.store(true, Ordering::Relaxed);
            value.value
        }
    }

    fn tracked(value: Vec2) -> (TrackedVec2, Arc<AtomicBool>) {
        let converted = Arc::new(AtomicBool::new(false));
        (
            TrackedVec2 {
                converted: Arc::clone(&converted),
                value,
            },
            converted,
        )
    }

    #[test]
    fn query_filter_default_rejects_callback_reentry_before_native_activity() {
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
        let result = std::panic::catch_unwind(QueryFilter::default);

        assert!(result.is_err());
    }

    #[test]
    fn plane_validation_matches_box2d_normalization_tolerance() {
        assert!(Plane::new([1.0, 0.0], 0.0).is_valid());
        assert!(Plane::new([1.000_005, 0.0], 0.0).is_valid());
        assert!(!Plane::new([1.000_01, 0.0], 0.0).is_valid());
        assert!(!Plane::new([f32::NAN, 0.0], 0.0).is_valid());
        assert!(!Plane::new([1.0, 0.0], f32::INFINITY).is_valid());
    }

    #[test]
    fn pure_mover_validation_is_callback_safe_and_native_calls_reject_reentry() {
        let plane = Plane::new([0.0, 1.0], 0.0);
        let mut planes = [CollisionPlane::rigid(plane)];
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();

        assert!(plane.is_valid());
        assert_eq!(planes[0].validate(), Ok(()));

        let (target, target_converted) = tracked(Vec2::new(0.0, -0.2));
        assert_eq!(
            try_solve_planes(target, &mut planes),
            Err(crate::error::ApiError::InCallback)
        );
        assert!(target_converted.load(Ordering::Relaxed));

        let (vector, vector_converted) = tracked(Vec2::new(0.0, -1.0));
        assert_eq!(
            try_clip_vector(vector, &planes),
            Err(crate::error::ApiError::InCallback)
        );
        assert!(vector_converted.load(Ordering::Relaxed));

        let mut invalid_planes = [CollisionPlane::rigid(Plane::new([0.0, 2.0], 0.0))];
        assert_eq!(
            try_solve_planes(Vec2::ZERO, &mut invalid_planes),
            Err(crate::error::ApiError::InvalidArgument)
        );

        let (target, target_converted) = tracked(Vec2::new(0.0, -0.2));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                solve_planes(target, &mut planes)
            }))
            .is_err()
        );
        assert!(target_converted.load(Ordering::Relaxed));
    }
}
