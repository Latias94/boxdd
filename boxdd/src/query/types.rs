use crate::error::ApiResult;
use crate::types::{ShapeId, Vec2};
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
pub(super) fn assert_query_aabb_valid(aabb: Aabb) {
    assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
}
pub(super) fn check_query_aabb_valid(aabb: Aabb) -> ApiResult<()> {
    if aabb.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
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
pub(super) fn assert_query_angle_valid(angle_radians: f32) {
    assert!(
        crate::is_valid_float(angle_radians),
        "angle_radians must be finite, got {angle_radians}"
    );
}

#[inline]
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

/// Axis-aligned bounding box
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

#[cfg(feature = "cgmath")]
impl From<Aabb> for (cgmath::Point2<f32>, cgmath::Point2<f32>) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "cgmath")]
impl From<(cgmath::Point2<f32>, cgmath::Point2<f32>)> for Aabb {
    #[inline]
    fn from((lower, upper): (cgmath::Point2<f32>, cgmath::Point2<f32>)) -> Self {
        Self::new(lower, upper)
    }
}

#[cfg(feature = "cgmath")]
impl From<Aabb> for (cgmath::Vector2<f32>, cgmath::Vector2<f32>) {
    #[inline]
    fn from(a: Aabb) -> Self {
        (a.lower.into(), a.upper.into())
    }
}

#[cfg(feature = "cgmath")]
impl From<(cgmath::Vector2<f32>, cgmath::Vector2<f32>)> for Aabb {
    #[inline]
    fn from((lower, upper): (cgmath::Vector2<f32>, cgmath::Vector2<f32>)) -> Self {
        Self::new(lower, upper)
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
    pub point: Vec2,
    pub normal: Vec2,
    pub fraction: f32,
    pub hit: bool,
}

impl RayResult {
    #[inline]
    pub fn from_raw(raw: ffi::b2RayResult) -> Self {
        Self {
            shape_id: ShapeId::from_raw(raw.shapeId),
            point: Vec2::from_raw(raw.point),
            normal: Vec2::from_raw(raw.normal),
            fraction: raw.fraction,
            hit: raw.hit,
        }
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

    #[inline]
    pub fn is_valid(self) -> bool {
        unsafe { ffi::b2IsValidPlane(self.into_raw()) }
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
    for plane in planes.iter() {
        assert_query_solver_collision_plane_valid(plane);
    }
    let raw = unsafe {
        ffi::b2SolvePlanes(
            target_delta.into_raw(),
            raw_collision_planes_mut(planes),
            planes.len() as i32,
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
    for plane in planes.iter() {
        check_query_solver_collision_plane_valid(plane)?;
    }
    let raw = unsafe {
        ffi::b2SolvePlanes(
            target_delta.into_raw(),
            raw_collision_planes_mut(planes),
            planes.len() as i32,
        )
    };
    Ok(PlaneSolverResult::from_raw(raw))
}

/// Clip a velocity or movement vector against solved collision planes.
#[inline]
pub fn clip_vector<V: Into<Vec2>>(vector: V, planes: &[CollisionPlane]) -> Vec2 {
    let vector = vector.into();
    assert_query_vec2_valid("vector", vector);
    for plane in planes.iter() {
        assert_query_collision_plane_valid(plane);
    }
    Vec2::from_raw(unsafe {
        ffi::b2ClipVector(
            vector.into_raw(),
            raw_collision_planes(planes),
            planes.len() as i32,
        )
    })
}

/// Clip a velocity or movement vector against solved collision planes.
///
/// Returns `ApiError::InvalidArgument` when `vector` or any collision plane state is invalid.
#[inline]
pub fn try_clip_vector<V: Into<Vec2>>(vector: V, planes: &[CollisionPlane]) -> ApiResult<Vec2> {
    let vector = vector.into();
    check_query_vec2_valid(vector)?;
    for plane in planes.iter() {
        check_query_collision_plane_valid(plane)?;
    }
    Ok(Vec2::from_raw(unsafe {
        ffi::b2ClipVector(
            vector.into_raw(),
            raw_collision_planes(planes),
            planes.len() as i32,
        )
    }))
}
