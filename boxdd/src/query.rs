//! Broad-phase queries, casts, and character-mover helpers.
//!
//! - AABB overlap: collect matching shape ids.
//! - Ray casts: closest or all hits along a path.
//! - Shape overlap / casting: build a temporary proxy from points + radius (accepts `Into<Vec2>` points).
//! - Offset proxies: apply translation + rotation to the proxy for queries in local frames.
//! - Character mover helpers: cast a capsule mover, collect collision planes, solve planes, and clip velocity.
//!
//! Note: Box2D proxies support at most `B2_MAX_POLYGON_VERTICES` points (8). Extra points are ignored.
//!
//! Filters: use `QueryFilter` to restrict categories/masks.
use crate::types::{ShapeId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;
use smallvec::SmallVec;
use std::any::Any;

const MAX_PROXY_POINTS: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;
type ProxyPoints = SmallVec<[ffi::b2Vec2; MAX_PROXY_POINTS]>;
type PanicPayload = Box<dyn Any + Send + 'static>;

fn collect_proxy_points<I, P>(points: I) -> ProxyPoints
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    let mut out = SmallVec::<[ffi::b2Vec2; MAX_PROXY_POINTS]>::new();
    for p in points.into_iter().take(MAX_PROXY_POINTS) {
        out.push(ffi::b2Vec2::from(p.into()));
    }
    out
}

struct CollectCtx<'a, T> {
    out: &'a mut Vec<T>,
    panicked: bool,
    panic: Option<PanicPayload>,
}

impl<'a, T> CollectCtx<'a, T> {
    fn from_cleared(out: &'a mut Vec<T>) -> Self {
        Self {
            out,
            panicked: false,
            panic: None,
        }
    }

    fn push(&mut self, value: T) -> bool {
        if self.panicked {
            return false;
        }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.out.push(value);
        }));
        match r {
            Ok(()) => true,
            Err(p) => {
                self.panicked = true;
                self.panic = Some(p);
                false
            }
        }
    }

    fn resume_unwind_if_needed(self) {
        if let Some(p) = self.panic {
            std::panic::resume_unwind(p);
        }
    }
}

unsafe extern "C" fn collect_shape_id_cb(
    shape_id: ffi::b2ShapeId,
    ctx: *mut core::ffi::c_void,
) -> bool {
    let ctx = unsafe { &mut *(ctx as *mut CollectCtx<'_, ShapeId>) };
    ctx.push(shape_id)
}

#[allow(clippy::unnecessary_cast)]
unsafe extern "C" fn collect_ray_result_cb(
    shape_id: ffi::b2ShapeId,
    point: ffi::b2Vec2,
    normal: ffi::b2Vec2,
    fraction: f32,
    ctx: *mut core::ffi::c_void,
) -> f32 {
    let ctx = unsafe { &mut *(ctx as *mut CollectCtx<'_, RayResult>) };
    if ctx.push(RayResult {
        shape_id,
        point: point.into(),
        normal: normal.into(),
        fraction,
        hit: true,
    }) {
        1.0f32
    } else {
        0.0
    }
}

unsafe extern "C" fn collect_mover_plane_result_cb(
    shape_id: ffi::b2ShapeId,
    plane: *const ffi::b2PlaneResult,
    ctx: *mut core::ffi::c_void,
) -> bool {
    let ctx = unsafe { &mut *(ctx as *mut CollectCtx<'_, MoverPlaneResult>) };
    let plane = unsafe { *plane };
    ctx.push(MoverPlaneResult {
        shape_id,
        plane: plane.plane.into(),
        point: plane.point.into(),
        hit: plane.hit,
    })
}

fn make_capsule<V1: Into<Vec2>, V2: Into<Vec2>>(c1: V1, c2: V2, radius: f32) -> ffi::b2Capsule {
    crate::shapes::Capsule::new(c1, c2, radius).into()
}

fn overlap_aabb_into_impl(
    world: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    out.clear();
    let mut ctx = CollectCtx::from_cleared(out);
    unsafe {
        let _ = ffi::b2World_OverlapAABB(
            world,
            aabb.into(),
            filter.0,
            Some(collect_shape_id_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn overlap_aabb_impl(world: ffi::b2WorldId, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_aabb_into_impl(world, aabb, filter, &mut out);
    out
}

fn cast_ray_closest_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    world: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> RayResult {
    let o: ffi::b2Vec2 = origin.into().into();
    let t: ffi::b2Vec2 = translation.into().into();
    let raw = unsafe { ffi::b2World_CastRayClosest(world, o, t, filter.0) };
    RayResult::from(raw)
}

fn cast_ray_all_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    world: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult> {
    let mut out = Vec::new();
    cast_ray_all_into_impl(world, origin, translation, filter, &mut out);
    out
}

fn cast_ray_all_into_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    world: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    out.clear();
    let mut ctx = CollectCtx::from_cleared(out);
    let o: ffi::b2Vec2 = origin.into().into();
    let t: ffi::b2Vec2 = translation.into().into();
    unsafe {
        let _ = ffi::b2World_CastRay(
            world,
            o,
            t,
            filter.0,
            Some(collect_ray_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn overlap_polygon_points_into_impl<I, P>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    out.clear();
    let pts = collect_proxy_points(points);
    if pts.is_empty() {
        return;
    }
    let proxy = unsafe { ffi::b2MakeProxy(pts.as_ptr(), pts.len() as i32, radius) };
    let mut ctx = CollectCtx::from_cleared(out);
    unsafe {
        let _ = ffi::b2World_OverlapShape(
            world,
            &proxy,
            filter.0,
            Some(collect_shape_id_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn overlap_polygon_points_impl<I, P>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
) -> Vec<ShapeId>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    let mut out = Vec::new();
    overlap_polygon_points_into_impl(world, points, radius, filter, &mut out);
    out
}

fn cast_shape_points_into_impl<I, P, VT>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    VT: Into<Vec2>,
{
    out.clear();
    let pts = collect_proxy_points(points);
    if pts.is_empty() {
        return;
    }
    let proxy = unsafe { ffi::b2MakeProxy(pts.as_ptr(), pts.len() as i32, radius) };
    let mut ctx = CollectCtx::from_cleared(out);
    let t: ffi::b2Vec2 = translation.into().into();
    unsafe {
        let _ = ffi::b2World_CastShape(
            world,
            &proxy,
            t,
            filter.0,
            Some(collect_ray_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn cast_shape_points_impl<I, P, VT>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    VT: Into<Vec2>,
{
    let mut out = Vec::new();
    cast_shape_points_into_impl(world, points, radius, translation, filter, &mut out);
    out
}

fn cast_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
    world: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
) -> f32 {
    let cap = make_capsule(c1, c2, radius);
    let t: ffi::b2Vec2 = translation.into().into();
    unsafe { ffi::b2World_CastMover(world, &cap, t, filter.0) }
}

fn collide_mover_into_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
    world: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<MoverPlaneResult>,
) {
    out.clear();
    let cap = make_capsule(c1, c2, radius);
    let mut ctx = CollectCtx::from_cleared(out);
    unsafe {
        ffi::b2World_CollideMover(
            world,
            &cap,
            filter.0,
            Some(collect_mover_plane_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn collide_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
    world: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    filter: QueryFilter,
) -> Vec<MoverPlaneResult> {
    let mut out = Vec::new();
    collide_mover_into_impl(world, c1, c2, radius, filter, &mut out);
    out
}

fn overlap_polygon_points_with_offset_into_impl<I, P, V, A>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
{
    out.clear();
    let pts = collect_proxy_points(points);
    if pts.is_empty() {
        return;
    }
    let (s, c) = angle_radians.into().sin_cos();
    let pos: ffi::b2Vec2 = position.into().into();
    let proxy = unsafe {
        ffi::b2MakeOffsetProxy(
            pts.as_ptr(),
            pts.len() as i32,
            radius,
            pos,
            ffi::b2Rot { c, s },
        )
    };
    let mut ctx = CollectCtx::from_cleared(out);
    unsafe {
        let _ = ffi::b2World_OverlapShape(
            world,
            &proxy,
            filter.0,
            Some(collect_shape_id_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn overlap_polygon_points_with_offset_impl<I, P, V, A>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    filter: QueryFilter,
) -> Vec<ShapeId>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
{
    let mut out = Vec::new();
    overlap_polygon_points_with_offset_into_impl(
        world,
        points,
        radius,
        position,
        angle_radians,
        filter,
        &mut out,
    );
    out
}

fn cast_shape_points_with_offset_into_impl<I, P, V, A, VT>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
    VT: Into<Vec2>,
{
    out.clear();
    let pts = collect_proxy_points(points);
    if pts.is_empty() {
        return;
    }
    let (s, c) = angle_radians.into().sin_cos();
    let pos: ffi::b2Vec2 = position.into().into();
    let proxy = unsafe {
        ffi::b2MakeOffsetProxy(
            pts.as_ptr(),
            pts.len() as i32,
            radius,
            pos,
            ffi::b2Rot { c, s },
        )
    };
    let mut ctx = CollectCtx::from_cleared(out);
    let t: ffi::b2Vec2 = translation.into().into();
    unsafe {
        let _ = ffi::b2World_CastShape(
            world,
            &proxy,
            t,
            filter.0,
            Some(collect_ray_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.resume_unwind_if_needed();
}

fn cast_shape_points_with_offset_impl<I, P, V, A, VT>(
    world: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
    VT: Into<Vec2>,
{
    let mut out = Vec::new();
    cast_shape_points_with_offset_into_impl(
        world,
        points,
        radius,
        position,
        angle_radians,
        translation,
        filter,
        &mut out,
    );
    out
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

impl From<Aabb> for ffi::b2AABB {
    fn from(a: Aabb) -> Self {
        ffi::b2AABB {
            lowerBound: a.lower.into(),
            upperBound: a.upper.into(),
        }
    }
}

impl From<ffi::b2AABB> for Aabb {
    fn from(a: ffi::b2AABB) -> Self {
        Self {
            lower: a.lowerBound.into(),
            upper: a.upperBound.into(),
        }
    }
}

impl Aabb {
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

impl From<ffi::b2RayResult> for RayResult {
    fn from(r: ffi::b2RayResult) -> Self {
        Self {
            shape_id: r.shapeId,
            point: r.point.into(),
            normal: r.normal.into(),
            fraction: r.fraction,
            hit: r.hit,
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
}

impl From<ffi::b2Plane> for Plane {
    #[inline]
    fn from(plane: ffi::b2Plane) -> Self {
        Self {
            normal: plane.normal.into(),
            offset: plane.offset,
        }
    }
}

impl From<Plane> for ffi::b2Plane {
    #[inline]
    fn from(plane: Plane) -> Self {
        Self {
            normal: plane.normal.into(),
            offset: plane.offset,
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
}

impl From<ffi::b2CollisionPlane> for CollisionPlane {
    #[inline]
    fn from(plane: ffi::b2CollisionPlane) -> Self {
        Self {
            plane: plane.plane.into(),
            push_limit: plane.pushLimit,
            push: plane.push,
            clip_velocity: plane.clipVelocity,
        }
    }
}

impl From<CollisionPlane> for ffi::b2CollisionPlane {
    #[inline]
    fn from(plane: CollisionPlane) -> Self {
        Self {
            plane: plane.plane.into(),
            pushLimit: plane.push_limit,
            push: plane.push,
            clipVelocity: plane.clip_velocity,
        }
    }
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

impl From<ffi::b2PlaneSolverResult> for PlaneSolverResult {
    #[inline]
    fn from(result: ffi::b2PlaneSolverResult) -> Self {
        Self {
            translation: result.translation.into(),
            iteration_count: result.iterationCount,
        }
    }
}

#[inline]
fn raw_collision_planes_mut(planes: &mut [CollisionPlane]) -> *mut ffi::b2CollisionPlane {
    if planes.is_empty() {
        core::ptr::null_mut()
    } else {
        planes.as_mut_ptr().cast()
    }
}

#[inline]
fn raw_collision_planes(planes: &[CollisionPlane]) -> *const ffi::b2CollisionPlane {
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
    let raw = unsafe {
        ffi::b2SolvePlanes(
            target_delta.into().into(),
            raw_collision_planes_mut(planes),
            planes.len() as i32,
        )
    };
    raw.into()
}

/// Clip a velocity or movement vector against solved collision planes.
#[inline]
pub fn clip_vector<V: Into<Vec2>>(vector: V, planes: &[CollisionPlane]) -> Vec2 {
    unsafe {
        ffi::b2ClipVector(
            vector.into().into(),
            raw_collision_planes(planes),
            planes.len() as i32,
        )
    }
    .into()
}

impl World {
    /// Overlap test for all shapes in an AABB. Returns matching shape ids.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, Aabb, QueryFilter};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let b = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
    /// let hits = world.overlap_aabb(Aabb { lower: Vec2::new(-1.0, -1.0), upper: Vec2::new(1.0, 3.0) }, QueryFilter::default());
    /// assert!(!hits.is_empty());
    /// ```
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        crate::core::callback_state::assert_not_in_callback();
        overlap_aabb_impl(self.raw(), aabb, filter)
    }

    /// Overlap test for all shapes in an AABB and write matching shape ids into `out`.
    ///
    /// `out` is cleared before new hits are appended so its allocation can be reused across frames.
    pub fn overlap_aabb_into(&self, aabb: Aabb, filter: QueryFilter, out: &mut Vec<ShapeId>) {
        crate::core::callback_state::assert_not_in_callback();
        overlap_aabb_into_impl(self.raw(), aabb, filter, out);
    }

    pub fn try_overlap_aabb(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(overlap_aabb_impl(self.raw(), aabb, filter))
    }

    pub fn try_overlap_aabb_into(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        overlap_aabb_into_impl(self.raw(), aabb, filter, out);
        Ok(())
    }

    /// Cast a ray and return the closest hit.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hit = world.cast_ray_closest(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// if hit.hit { /* use hit.point / hit.normal */ }
    /// ```
    pub fn cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> RayResult {
        crate::core::callback_state::assert_not_in_callback();
        cast_ray_closest_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<RayResult> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_ray_closest_impl(
            self.raw(),
            origin,
            translation,
            filter,
        ))
    }

    /// Cast a ray and collect all hits along the path.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hits = world.cast_ray_all(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        crate::core::callback_state::assert_not_in_callback();
        cast_ray_all_impl(self.raw(), origin, translation, filter)
    }

    /// Cast a ray and append all hits into `out`, reusing the caller-owned allocation.
    pub fn cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        crate::core::callback_state::assert_not_in_callback();
        cast_ray_all_into_impl(self.raw(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<RayResult>> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_ray_all_impl(self.raw(), origin, translation, filter))
    }

    pub fn try_cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        cast_ray_all_into_impl(self.raw(), origin, translation, filter, out);
        Ok(())
    }

    /// Overlap polygon points (creates a temporary shape proxy from given points + radius) and collect all shape ids.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let square = [Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5), Vec2::new(0.5, 0.5), Vec2::new(-0.5, 0.5)];
    /// let hits = world.overlap_polygon_points(square, 0.0, QueryFilter::default());
    /// let _ = hits;
    /// ```
    pub fn overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_impl(self.raw(), points, radius, filter)
    }

    /// Overlap a temporary polygon proxy and write matching shape ids into `out`.
    pub fn overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out);
    }

    pub fn try_overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(overlap_polygon_points_impl(
            self.raw(),
            points,
            radius,
            filter,
        ))
    }

    pub fn try_overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out);
        Ok(())
    }

    /// Cast a polygon proxy and collect hits. Returns all intersections with fraction and contact info.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let tri = [Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.25, 0.5)];
    /// let hits = world.cast_shape_points(tri, 0.0, Vec2::new(0.0, -1.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_impl(self.raw(), points, radius, translation, filter)
    }

    /// Cast a temporary polygon proxy and write all hits into `out`.
    pub fn cast_shape_points_into<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out);
    }

    pub fn try_cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_shape_points_impl(
            self.raw(),
            points,
            radius,
            translation,
            filter,
        ))
    }

    pub fn try_cast_shape_points_into<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out);
        Ok(())
    }

    /// Cast a capsule mover and return remaining fraction (1.0 = free, < 1.0 = hit earlier).
    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        crate::core::callback_state::assert_not_in_callback();
        cast_mover_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn try_cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<f32> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_mover_impl(
            self.raw(),
            c1,
            c2,
            radius,
            translation,
            filter,
        ))
    }

    /// Collect collision planes for a capsule mover at its current position.
    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        crate::core::callback_state::assert_not_in_callback();
        collide_mover_impl(self.raw(), c1, c2, radius, filter)
    }

    /// Collect collision planes for a capsule mover and reuse `out`.
    pub fn collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) {
        crate::core::callback_state::assert_not_in_callback();
        collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out);
    }

    pub fn try_collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<MoverPlaneResult>> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(collide_mover_impl(self.raw(), c1, c2, radius, filter))
    }

    pub fn try_collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out);
        Ok(())
    }

    /// Overlap polygon points with an offset transform.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let rect = [Vec2::new(-0.5, -0.25), Vec2::new(0.5, -0.25), Vec2::new(0.5, 0.25), Vec2::new(-0.5, 0.25)];
    /// let hits = world.overlap_polygon_points_with_offset(rect, 0.0, Vec2::new(0.0, 2.0), 0.0_f32, QueryFilter::default());
    /// let _ = hits;
    /// ```
    pub fn overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    /// Overlap an offset polygon proxy and write matching shape ids into `out`.
    pub fn overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
    }

    pub fn try_overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        ))
    }

    pub fn try_overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
        Ok(())
    }

    /// Cast polygon points with an offset transform (position + angle).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let rect = [Vec2::new(-0.5, -0.25), Vec2::new(0.5, -0.25), Vec2::new(0.5, 0.25), Vec2::new(-0.5, 0.25)];
    /// let hits = world.cast_shape_points_with_offset(rect, 0.0, Vec2::new(0.0, 2.0), 0.0_f32, Vec2::new(0.0, -1.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
    }

    /// Cast an offset polygon proxy and write all hits into `out`.
    pub fn cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        );
    }

    pub fn try_cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_shape_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        ))
    }

    pub fn try_cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        cast_shape_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        );
        Ok(())
    }
}

impl WorldHandle {
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        crate::core::callback_state::assert_not_in_callback();
        overlap_aabb_impl(self.raw(), aabb, filter)
    }

    pub fn overlap_aabb_into(&self, aabb: Aabb, filter: QueryFilter, out: &mut Vec<ShapeId>) {
        crate::core::callback_state::assert_not_in_callback();
        overlap_aabb_into_impl(self.raw(), aabb, filter, out);
    }

    pub fn try_overlap_aabb(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(overlap_aabb_impl(self.raw(), aabb, filter))
    }

    pub fn try_overlap_aabb_into(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        overlap_aabb_into_impl(self.raw(), aabb, filter, out);
        Ok(())
    }

    pub fn cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> RayResult {
        crate::core::callback_state::assert_not_in_callback();
        cast_ray_closest_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<RayResult> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_ray_closest_impl(
            self.raw(),
            origin,
            translation,
            filter,
        ))
    }

    pub fn cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        crate::core::callback_state::assert_not_in_callback();
        cast_ray_all_impl(self.raw(), origin, translation, filter)
    }

    pub fn cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        crate::core::callback_state::assert_not_in_callback();
        cast_ray_all_into_impl(self.raw(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<RayResult>> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_ray_all_impl(self.raw(), origin, translation, filter))
    }

    pub fn try_cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        cast_ray_all_into_impl(self.raw(), origin, translation, filter, out);
        Ok(())
    }

    pub fn overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_impl(self.raw(), points, radius, filter)
    }

    pub fn overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out);
    }

    pub fn try_overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(overlap_polygon_points_impl(
            self.raw(),
            points,
            radius,
            filter,
        ))
    }

    pub fn try_overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out);
        Ok(())
    }

    pub fn cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_impl(self.raw(), points, radius, translation, filter)
    }

    pub fn cast_shape_points_into<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out);
    }

    pub fn try_cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_shape_points_impl(
            self.raw(),
            points,
            radius,
            translation,
            filter,
        ))
    }

    pub fn try_cast_shape_points_into<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out);
        Ok(())
    }

    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        crate::core::callback_state::assert_not_in_callback();
        cast_mover_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn try_cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<f32> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_mover_impl(
            self.raw(),
            c1,
            c2,
            radius,
            translation,
            filter,
        ))
    }

    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        crate::core::callback_state::assert_not_in_callback();
        collide_mover_impl(self.raw(), c1, c2, radius, filter)
    }

    pub fn collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) {
        crate::core::callback_state::assert_not_in_callback();
        collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out);
    }

    pub fn try_collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<MoverPlaneResult>> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(collide_mover_impl(self.raw(), c1, c2, radius, filter))
    }

    pub fn try_collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out);
        Ok(())
    }

    pub fn overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::assert_not_in_callback();
        overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
    }

    pub fn try_overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        ))
    }

    pub fn try_overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
        Ok(())
    }

    pub fn cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
    }

    pub fn cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::assert_not_in_callback();
        cast_shape_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        );
    }

    pub fn try_cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
    ) -> crate::error::ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(cast_shape_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        ))
    }

    pub fn try_cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> crate::error::ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        crate::core::callback_state::check_not_in_callback()?;
        cast_shape_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        );
        Ok(())
    }
}
