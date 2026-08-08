use crate::core::foundation::transient_native_lease;
use crate::error::Result;
use crate::types::{Position, ShapeId, Vec2};
use boxdd_sys::ffi;

pub(super) fn minimum_mover_radius() -> Result<f32> {
    Ok(0.01 * crate::core::foundation::current_length_units_per_meter()?)
}

pub(super) fn check_query_vec2_valid(
    operation: &'static str,
    argument: &'static str,
    value: Vec2,
) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            argument,
            "a finite vector",
        ))
    }
}

pub(super) fn check_query_position_valid(
    operation: &'static str,
    argument: &'static str,
    value: Position,
) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            argument,
            "a finite world position",
        ))
    }
}
pub(super) fn check_query_aabb_valid(operation: &'static str, aabb: Aabb) -> Result<()> {
    if aabb.is_valid() {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            "aabb",
            "finite ordered lower and upper bounds",
        ))
    }
}

#[inline]
pub(super) fn check_query_non_negative_finite_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            argument,
            "a finite value greater than or equal to zero",
        ))
    }
}

#[inline]
pub(super) fn check_query_mover_radius_valid(operation: &'static str, radius: f32) -> Result<()> {
    if crate::is_valid_float(radius) && radius > minimum_mover_radius()? {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            "radius",
            "a finite value greater than the configured minimum mover radius",
        ))
    }
}

/// Axis-aligned bounding box with `f32` coordinates in both precision modes.
///
/// When Box2D computes one from a [`crate::WorldTransform`] in double-precision mode, it narrows
/// the absolute bounds outward so this conservative box still contains the represented geometry.
#[doc(alias = "aabb")]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub(crate) lower: Vec2,
    pub(crate) upper: Vec2,
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Aabb {}
#[cfg(feature = "bytemuck")]
const _: () = {
    assert!(core::mem::size_of::<Aabb>() == 16);
    assert!(core::mem::align_of::<Aabb>() == 4);
};

impl Aabb {
    #[inline]
    /// Construct from a raw Box2D AABB after validating its invariants.
    pub fn from_raw(raw: ffi::b2AABB) -> Result<Self> {
        let aabb = Self {
            lower: Vec2::from_raw(raw.lowerBound),
            upper: Vec2::from_raw(raw.upperBound),
        };
        check_query_aabb_valid("Aabb::from_raw", aabb)?;
        Ok(aabb)
    }

    #[inline]
    pub(crate) fn from_raw_unvalidated(raw: ffi::b2AABB) -> Self {
        Self {
            lower: Vec2::from_raw(raw.lowerBound),
            upper: Vec2::from_raw(raw.upperBound),
        }
    }

    #[inline]
    pub const fn lower(self) -> Vec2 {
        self.lower
    }

    #[inline]
    pub const fn upper(self) -> Vec2 {
        self.upper
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
    pub fn new<L: Into<Vec2>, U: Into<Vec2>>(lower: L, upper: U) -> Result<Self> {
        let aabb = Self {
            lower: lower.into(),
            upper: upper.into(),
        };
        check_query_aabb_valid("Aabb::new", aabb)?;
        Ok(aabb)
    }
    /// Create an AABB from center and half-extents (both in world units).
    #[inline]
    pub fn from_center_half_extents<C: Into<Vec2>, H: Into<Vec2>>(
        center: C,
        half: H,
    ) -> Result<Self> {
        let c = center.into();
        let h = half.into();
        let aabb = Self {
            lower: Vec2::new(c.x - h.x, c.y - h.y),
            upper: Vec2::new(c.x + h.x, c.y + h.y),
        };
        check_query_aabb_valid("Aabb::from_center_half_extents", aabb)?;
        Ok(aabb)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Aabb {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            lower: Vec2,
            upper: Vec2,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.lower, repr.upper).map_err(serde::de::Error::custom)
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
impl TryFrom<(mint::Point2<f32>, mint::Point2<f32>)> for Aabb {
    type Error = crate::Error;

    #[inline]
    fn try_from((lower, upper): (mint::Point2<f32>, mint::Point2<f32>)) -> Result<Self> {
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
impl TryFrom<(mint::Vector2<f32>, mint::Vector2<f32>)> for Aabb {
    type Error = crate::Error;

    #[inline]
    fn try_from((lower, upper): (mint::Vector2<f32>, mint::Vector2<f32>)) -> Result<Self> {
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
impl TryFrom<(glam::Vec2, glam::Vec2)> for Aabb {
    type Error = crate::Error;

    #[inline]
    fn try_from((lower, upper): (glam::Vec2, glam::Vec2)) -> Result<Self> {
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
impl TryFrom<(nalgebra::Point2<f32>, nalgebra::Point2<f32>)> for Aabb {
    type Error = crate::Error;

    #[inline]
    fn try_from((lower, upper): (nalgebra::Point2<f32>, nalgebra::Point2<f32>)) -> Result<Self> {
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
impl TryFrom<(nalgebra::Vector2<f32>, nalgebra::Vector2<f32>)> for Aabb {
    type Error = crate::Error;

    #[inline]
    fn try_from((lower, upper): (nalgebra::Vector2<f32>, nalgebra::Vector2<f32>)) -> Result<Self> {
        Self::new(lower, upper)
    }
}

/// Filter for queries
#[doc(alias = "query_filter")]
#[derive(Copy, Clone, Debug)]
pub struct QueryFilter(pub(crate) ffi::b2QueryFilter);

impl Default for QueryFilter {
    fn default() -> Self {
        Self(crate::core::native_defaults::query_filter())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for QueryFilter {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
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
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            category_bits: u64,
            mask_bits: u64,
        }
        let r = <Repr as serde::Deserialize>::deserialize(deserializer)?;
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
    /// Surface normal in world orientation.
    ///
    /// This is a unit vector for a regular hit. Box2D returns [`Vec2::ZERO`] together with a zero
    /// [`Self::fraction`] when a shape cast starts in overlap.
    pub normal: Vec2,
    /// Fraction of the query translation at which the hit occurred.
    pub fraction: f32,
    pub hit: bool,
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

/// A collision plane used by Box2D's character mover helpers.
#[doc(alias = "plane")]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Plane {
    pub(crate) normal: Vec2,
    pub(crate) offset: f32,
}

impl Plane {
    #[inline]
    pub fn new<N: Into<Vec2>>(normal: N, offset: f32) -> Result<Self> {
        let plane = Self {
            normal: normal.into(),
            offset,
        };
        if plane.is_valid() {
            Ok(plane)
        } else {
            Err(crate::Error::invalid_argument(
                "Plane::new",
                "normal/offset",
                "a finite plane with a unit normal",
            ))
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
    /// Construct from a raw Box2D plane after validating its invariants.
    pub fn from_raw(raw: ffi::b2Plane) -> Result<Self> {
        let plane = Self {
            normal: Vec2::from_raw(raw.normal),
            offset: raw.offset,
        };
        if plane.is_valid() {
            Ok(plane)
        } else {
            Err(crate::Error::invalid_argument(
                "Plane::from_raw",
                "raw",
                "a finite plane with a unit normal",
            ))
        }
    }

    #[inline]
    pub(crate) fn from_raw_unvalidated(raw: ffi::b2Plane) -> Self {
        Self {
            normal: Vec2::from_raw(raw.normal),
            offset: raw.offset,
        }
    }

    #[inline]
    pub const fn normal(self) -> Vec2 {
        self.normal
    }

    #[inline]
    pub const fn offset(self) -> f32 {
        self.offset
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2Plane {
        ffi::b2Plane {
            normal: self.normal.into_raw(),
            offset: self.offset,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Plane {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            normal: Vec2,
            offset: f32,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.normal, repr.offset).map_err(serde::de::Error::custom)
    }
}

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
    ) -> Result<Option<CollisionPlane>> {
        self.hit
            .then(|| CollisionPlane::new(self.plane, push_limit, clip_velocity))
            .transpose()
    }

    /// Convert a valid mover-plane result into a rigid collision plane.
    ///
    /// This uses `f32::MAX` as the push limit and enables velocity clipping.
    #[inline]
    pub fn into_rigid_collision_plane(self) -> Result<Option<CollisionPlane>> {
        self.into_collision_plane(CollisionPlane::RIGID_PUSH_LIMIT, true)
    }
}

/// Collision plane input for `solve_planes` and `clip_vector`.
#[doc(alias = "collision_plane")]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CollisionPlane {
    pub(crate) plane: Plane,
    pub(crate) push_limit: f32,
    pub(crate) push: f32,
    pub(crate) clip_velocity: bool,
}

impl CollisionPlane {
    pub const RIGID_PUSH_LIMIT: f32 = f32::MAX;

    #[inline]
    pub fn new(plane: Plane, push_limit: f32, clip_velocity: bool) -> Result<Self> {
        let collision_plane = Self {
            plane,
            push_limit,
            push: 0.0,
            clip_velocity,
        };
        check_query_collision_plane_valid("CollisionPlane::new", &collision_plane)?;
        Ok(collision_plane)
    }

    #[inline]
    pub fn rigid(plane: Plane) -> Result<Self> {
        Self::new(plane, Self::RIGID_PUSH_LIMIT, true)
    }

    /// Validate this collision plane for Box2D mover solver helpers.
    pub fn validate(&self) -> Result<()> {
        check_query_collision_plane_valid("CollisionPlane::validate", self)
    }

    #[inline]
    /// Construct from a raw Box2D collision plane after validating its invariants.
    pub fn from_raw(raw: ffi::b2CollisionPlane) -> Result<Self> {
        let plane = Self::from_raw_unvalidated(raw);
        check_query_collision_plane_valid("CollisionPlane::from_raw", &plane)?;
        Ok(plane)
    }

    #[inline]
    pub(crate) fn from_raw_unvalidated(raw: ffi::b2CollisionPlane) -> Self {
        Self {
            plane: Plane::from_raw_unvalidated(raw.plane),
            push_limit: raw.pushLimit,
            push: raw.push,
            clip_velocity: raw.clipVelocity,
        }
    }

    #[inline]
    pub const fn plane(self) -> Plane {
        self.plane
    }

    #[inline]
    pub const fn push_limit(self) -> f32 {
        self.push_limit
    }

    #[inline]
    pub const fn push(self) -> f32 {
        self.push
    }

    #[inline]
    pub const fn clip_velocity(self) -> bool {
        self.clip_velocity
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

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CollisionPlane {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            plane: Plane,
            push_limit: f32,
            push: f32,
            clip_velocity: bool,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        let plane = Self {
            plane: repr.plane,
            push_limit: repr.push_limit,
            push: repr.push,
            clip_velocity: repr.clip_velocity,
        };
        check_query_collision_plane_valid("CollisionPlane::deserialize", &plane)
            .map_err(serde::de::Error::custom)?;
        Ok(plane)
    }
}

#[inline]
pub(super) fn check_query_solver_collision_plane_valid(
    operation: &'static str,
    plane: &CollisionPlane,
) -> Result<()> {
    if !plane.plane.is_valid() {
        return Err(crate::error::Error::invalid_argument(
            operation,
            "planes[].plane",
            "a finite plane with a unit normal",
        ));
    }
    check_query_non_negative_finite_scalar(operation, "planes[].push_limit", plane.push_limit)
}

#[inline]
pub(super) fn check_query_collision_plane_valid(
    operation: &'static str,
    plane: &CollisionPlane,
) -> Result<()> {
    check_query_solver_collision_plane_valid(operation, plane)?;
    check_query_non_negative_finite_scalar(operation, "planes[].push", plane.push)
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PlaneSolverResult {
    translation: Vec2,
    iteration_count: i32,
}

impl PlaneSolverResult {
    /// Construct a checked plane-solver result.
    #[inline]
    pub fn new<T: Into<Vec2>>(translation: T, iteration_count: i32) -> Result<Self> {
        let result = Self {
            translation: translation.into(),
            iteration_count,
        };
        result.validate_for("PlaneSolverResult::new")?;
        Ok(result)
    }

    /// Construct from a raw Box2D result after validating its invariants.
    #[inline]
    pub fn from_raw(raw: ffi::b2PlaneSolverResult) -> Result<Self> {
        let result = Self::from_raw_unvalidated(raw);
        result.validate_for("PlaneSolverResult::from_raw")?;
        Ok(result)
    }

    #[inline]
    fn from_native(operation: &'static str, raw: ffi::b2PlaneSolverResult) -> Result<Self> {
        let result = Self::from_raw_unvalidated(raw);
        if !result.translation.is_valid() {
            return Err(crate::error::Error::InvalidNativeOutput {
                operation,
                output: "translation",
                constraint: "a finite vector",
            });
        }
        if result.iteration_count < 0 {
            return Err(crate::error::Error::InvalidNativeOutput {
                operation,
                output: "iteration_count",
                constraint: "a non-negative native int",
            });
        }
        Ok(result)
    }

    #[inline]
    fn from_raw_unvalidated(raw: ffi::b2PlaneSolverResult) -> Self {
        Self {
            translation: Vec2::from_raw(raw.translation),
            iteration_count: raw.iterationCount,
        }
    }

    /// Validate the result's public invariants.
    #[inline]
    pub fn validate(&self) -> Result<()> {
        self.validate_for("PlaneSolverResult::validate")
    }

    #[inline]
    fn validate_for(&self, operation: &'static str) -> Result<()> {
        check_query_vec2_valid(operation, "translation", self.translation)?;
        if self.iteration_count < 0 {
            return Err(crate::error::Error::invalid_argument(
                operation,
                "iteration_count",
                "a non-negative native int",
            ));
        }
        Ok(())
    }

    /// Return the translation selected by the solver.
    #[inline]
    pub const fn translation(self) -> Vec2 {
        self.translation
    }

    /// Return the number of solver iterations.
    #[inline]
    pub const fn iteration_count(self) -> i32 {
        self.iteration_count
    }

    /// Convert this result into its raw Box2D representation.
    #[inline]
    pub fn into_raw(self) -> ffi::b2PlaneSolverResult {
        ffi::b2PlaneSolverResult {
            translation: self.translation.into_raw(),
            iterationCount: self.iteration_count,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PlaneSolverResult {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            translation: Vec2,
            iteration_count: i32,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.translation, repr.iteration_count).map_err(serde::de::Error::custom)
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

fn commit_native_solved_planes(
    planes: &mut [CollisionPlane],
    raw_planes: Vec<ffi::b2CollisionPlane>,
) -> Result<()> {
    debug_assert_eq!(planes.len(), raw_planes.len());
    if raw_planes
        .iter()
        .copied()
        .any(|plane| CollisionPlane::from_raw(plane).is_err())
    {
        return Err(crate::error::Error::InvalidNativeOutput {
            operation: "solve_planes",
            output: "planes",
            constraint: "finite valid collision planes with non-negative push values",
        });
    }
    for (plane, raw) in planes.iter_mut().zip(raw_planes) {
        *plane = CollisionPlane::from_raw_unvalidated(raw);
    }
    Ok(())
}

#[inline]
fn check_collision_plane_count(operation: &'static str, count: usize) -> Result<i32> {
    i32::try_from(count).map_err(|_| {
        crate::error::Error::invalid_argument(
            operation,
            "planes",
            "a slice length representable by a native int",
        )
    })
}

/// Solve the translation that best satisfies the supplied mover collision planes.
///
/// The `push` field on each collision plane is updated in place by Box2D.
/// Invalid values, callback re-entry, and unavailable native foundation state are reported.
#[inline]
pub fn solve_planes<V: Into<Vec2>>(
    target_delta: V,
    planes: &mut [CollisionPlane],
) -> Result<PlaneSolverResult> {
    let target_delta = target_delta.into();
    check_query_vec2_valid("solve_planes", "target_delta", target_delta)?;
    let plane_count = check_collision_plane_count("solve_planes", planes.len())?;
    for plane in planes.iter() {
        check_query_solver_collision_plane_valid("solve_planes", plane)?;
    }
    let mut raw_planes = Vec::new();
    raw_planes
        .try_reserve_exact(planes.len())
        .map_err(|_| crate::error::Error::FfiOutputAllocationFailed)?;
    raw_planes.extend(planes.iter().copied().map(CollisionPlane::into_raw));
    let _lease = transient_native_lease()?;
    let raw = unsafe {
        ffi::b2SolvePlanes(
            target_delta.into_raw(),
            if raw_planes.is_empty() {
                core::ptr::null_mut()
            } else {
                raw_planes.as_mut_ptr()
            },
            plane_count,
        )
    };
    let result = PlaneSolverResult::from_native("solve_planes", raw)?;
    commit_native_solved_planes(planes, raw_planes)?;
    Ok(result)
}

/// Clip a velocity or movement vector against solved collision planes.
#[inline]
pub fn clip_vector<V: Into<Vec2>>(vector: V, planes: &[CollisionPlane]) -> Result<Vec2> {
    let vector = vector.into();
    check_query_vec2_valid("clip_vector", "vector", vector)?;
    let plane_count = check_collision_plane_count("clip_vector", planes.len())?;
    for plane in planes.iter() {
        check_query_collision_plane_valid("clip_vector", plane)?;
    }
    let _lease = transient_native_lease()?;
    let clipped = Vec2::from_raw(unsafe {
        ffi::b2ClipVector(vector.into_raw(), raw_collision_planes(planes), plane_count)
    });
    if clipped.is_valid() {
        Ok(clipped)
    } else {
        Err(crate::error::Error::InvalidNativeOutput {
            operation: "clip_vector",
            output: "vector",
            constraint: "a finite vector",
        })
    }
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
    fn query_filter_default_is_pure_and_callback_safe() {
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
        let filter = QueryFilter::default();
        assert_eq!(filter.category_bits(), 1);
        assert_eq!(filter.mask_bits(), u64::MAX);
    }

    #[test]
    fn plane_validation_matches_box2d_normalization_tolerance() {
        assert!(Plane::new([1.0, 0.0], 0.0).unwrap().is_valid());
        assert!(Plane::new([1.000_005, 0.0], 0.0).unwrap().is_valid());
        assert!(Plane::new([1.000_01, 0.0], 0.0).is_err());
        assert!(Plane::new([f32::NAN, 0.0], 0.0).is_err());
        assert!(Plane::new([1.0, 0.0], f32::INFINITY).is_err());
    }

    #[test]
    fn invalid_native_solver_planes_are_not_partially_published() {
        let plane = Plane::new([0.0, 1.0], 0.0).unwrap();
        let original = [
            CollisionPlane::new(plane, 1.0, true).unwrap(),
            CollisionPlane::new(plane, 2.0, false).unwrap(),
        ];
        let mut output = original;
        let mut first = original[0].into_raw();
        first.push = 0.5;
        let mut second = original[1].into_raw();
        second.push = f32::NAN;

        assert!(matches!(
            commit_native_solved_planes(&mut output, vec![first, second]),
            Err(crate::Error::InvalidNativeOutput {
                operation: "solve_planes",
                output: "planes",
                ..
            })
        ));
        assert_eq!(output, original);
    }

    #[test]
    fn pure_mover_validation_is_callback_safe_and_native_calls_reject_reentry() {
        let plane = Plane::new([0.0, 1.0], 0.0).unwrap();
        let mut planes = [CollisionPlane::rigid(plane).unwrap()];
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();

        assert!(plane.is_valid());
        assert_eq!(planes[0].validate(), Ok(()));

        let (target, target_converted) = tracked(Vec2::new(0.0, -0.2));
        assert_eq!(
            solve_planes(target, &mut planes),
            Err(crate::error::Error::InCallback)
        );
        assert!(target_converted.load(Ordering::Relaxed));

        let (vector, vector_converted) = tracked(Vec2::new(0.0, -1.0));
        assert_eq!(
            clip_vector(vector, &planes),
            Err(crate::error::Error::InCallback)
        );
        assert!(vector_converted.load(Ordering::Relaxed));

        let mut invalid_planes = [CollisionPlane {
            plane: Plane {
                normal: Vec2::new(0.0, 2.0),
                offset: 0.0,
            },
            push_limit: CollisionPlane::RIGID_PUSH_LIMIT,
            push: 0.0,
            clip_velocity: true,
        }];
        assert_eq!(
            solve_planes(Vec2::ZERO, &mut invalid_planes),
            Err(crate::error::Error::invalid_argument(
                "solve_planes",
                "planes[].plane",
                "a finite plane with a unit normal",
            ))
        );
    }
}
