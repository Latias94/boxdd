//! Broad-phase queries, casts, and character-mover helpers.
//!
//! - AABB and shape overlap: collect matching shape ids, reuse caller-owned buffers, or visit hits without a result container.
//! - Ray casts: closest or all hits along a path.
//! - Shape overlap / casting: build a temporary proxy from points + radius (accepts `Into<Vec2>` points).
//! - Offset proxies: apply translation + rotation to the proxy for queries in local frames.
//! - Character mover helpers: cast a capsule mover, collect collision planes, solve planes, and clip velocity.
//!
//! Note: Box2D proxies support at most `B2_MAX_POLYGON_VERTICES` points (8). Extra points are ignored.
//!
//! Filters: use `QueryFilter` to restrict categories/masks.
use crate::error::ApiResult;
use crate::types::{ShapeId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;
use smallvec::SmallVec;
use std::any::Any;

const MAX_PROXY_POINTS: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;
type ProxyPoints = SmallVec<[ffi::b2Vec2; MAX_PROXY_POINTS]>;
type PanicPayload = Box<dyn Any + Send + 'static>;

#[inline]
fn minimum_mover_radius() -> f32 {
    0.01 * crate::length_units_per_meter()
}

#[inline]
fn assert_query_vec2_valid(name: &str, value: Vec2) {
    assert!(
        value.is_valid(),
        "{name} must be a valid Box2D vector, got {:?}",
        value
    );
}

#[inline]
fn check_query_vec2_valid(value: Vec2) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_query_aabb_valid(aabb: Aabb) {
    assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
}

#[inline]
fn check_query_aabb_valid(aabb: Aabb) -> ApiResult<()> {
    if aabb.is_valid() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_query_non_negative_finite_scalar(name: &str, value: f32) {
    assert!(
        crate::is_valid_float(value) && value >= 0.0,
        "{name} must be finite and >= 0.0, got {value}"
    );
}

#[inline]
fn check_query_non_negative_finite_scalar(value: f32) -> ApiResult<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_query_angle_valid(angle_radians: f32) {
    assert!(
        crate::is_valid_float(angle_radians),
        "angle_radians must be finite, got {angle_radians}"
    );
}

#[inline]
fn check_query_angle_valid(angle_radians: f32) -> ApiResult<()> {
    if crate::is_valid_float(angle_radians) {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_query_mover_radius_valid(radius: f32) {
    let minimum = minimum_mover_radius();
    assert!(
        crate::is_valid_float(radius) && radius > minimum,
        "mover radius must be finite and > {minimum}, got {radius}"
    );
}

#[inline]
fn check_query_mover_radius_valid(radius: f32) -> ApiResult<()> {
    if crate::is_valid_float(radius) && radius > minimum_mover_radius() {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

fn collect_asserted_proxy_points<I, P>(points: I) -> ProxyPoints
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    let mut out = SmallVec::<[ffi::b2Vec2; MAX_PROXY_POINTS]>::new();
    for p in points.into_iter().take(MAX_PROXY_POINTS) {
        let point = p.into();
        assert_query_vec2_valid("points", point);
        out.push(point.into_raw());
    }
    out
}

fn try_collect_proxy_points<I, P>(points: I) -> ApiResult<ProxyPoints>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    let mut out = SmallVec::<[ffi::b2Vec2; MAX_PROXY_POINTS]>::new();
    for p in points.into_iter().take(MAX_PROXY_POINTS) {
        let point = p.into();
        check_query_vec2_valid(point)?;
        out.push(point.into_raw());
    }
    Ok(out)
}

#[inline]
fn make_proxy_from_points(points: &ProxyPoints, radius: f32) -> Option<ffi::b2ShapeProxy> {
    (!points.is_empty())
        .then(|| unsafe { ffi::b2MakeProxy(points.as_ptr(), points.len() as i32, radius) })
}

#[inline]
fn make_offset_proxy_from_points(
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
) -> Option<ffi::b2ShapeProxy> {
    (!points.is_empty()).then(|| {
        let (s, c) = angle_radians.sin_cos();
        unsafe {
            ffi::b2MakeOffsetProxy(
                points.as_ptr(),
                points.len() as i32,
                radius,
                position.into_raw(),
                ffi::b2Rot { c, s },
            )
        }
    })
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

struct VisitShapeIdCtx<'a, F> {
    visit: &'a mut F,
    stopped_early: bool,
    panic: Option<PanicPayload>,
}

impl<'a, F> VisitShapeIdCtx<'a, F>
where
    F: FnMut(ShapeId) -> bool,
{
    fn new(visit: &'a mut F) -> Self {
        Self {
            visit,
            stopped_early: false,
            panic: None,
        }
    }

    fn visit(&mut self, shape_id: ShapeId) -> bool {
        if self.stopped_early || self.panic.is_some() {
            return false;
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.visit)(shape_id))) {
            Ok(true) => true,
            Ok(false) => {
                self.stopped_early = true;
                false
            }
            Err(p) => {
                self.panic = Some(p);
                false
            }
        }
    }

    fn finish(self) -> bool {
        if let Some(p) = self.panic {
            std::panic::resume_unwind(p);
        }
        !self.stopped_early
    }
}

unsafe extern "C" fn visit_shape_id_cb<F>(
    shape_id: ffi::b2ShapeId,
    ctx: *mut core::ffi::c_void,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let ctx = unsafe { &mut *(ctx as *mut VisitShapeIdCtx<'_, F>) };
    ctx.visit(ShapeId::from_raw(shape_id))
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
        shape_id: ShapeId::from_raw(shape_id),
        point: Vec2::from_raw(point),
        normal: Vec2::from_raw(normal),
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
        shape_id: ShapeId::from_raw(shape_id),
        plane: Plane::from_raw(plane.plane),
        point: Vec2::from_raw(plane.point),
        hit: plane.hit,
    })
}

fn make_capsule<V1: Into<Vec2>, V2: Into<Vec2>>(c1: V1, c2: V2, radius: f32) -> ffi::b2Capsule {
    crate::shapes::Capsule::new(c1, c2, radius).into_raw()
}

fn visit_overlap_aabb_impl<F>(
    world: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let mut ctx = VisitShapeIdCtx::new(visit);
    unsafe {
        let _ = ffi::b2World_OverlapAABB(
            world,
            aabb.into_raw(),
            filter.0,
            Some(visit_shape_id_cb::<F>),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish()
}

fn overlap_aabb_into_impl(
    world: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    out.clear();
    let mut collect = |shape_id| {
        out.push(shape_id);
        true
    };
    let _ = visit_overlap_aabb_impl(world, aabb, filter, &mut collect);
}

fn overlap_aabb_impl(world: ffi::b2WorldId, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_aabb_into_impl(world, aabb, filter, &mut out);
    out
}

fn visit_overlap_shape_proxy_impl<F>(
    world: ffi::b2WorldId,
    proxy: &ffi::b2ShapeProxy,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let mut ctx = VisitShapeIdCtx::new(visit);
    unsafe {
        let _ = ffi::b2World_OverlapShape(
            world,
            proxy,
            filter.0,
            Some(visit_shape_id_cb::<F>),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish()
}

fn cast_ray_closest_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    world: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> RayResult {
    let o: ffi::b2Vec2 = origin.into().into_raw();
    let t: ffi::b2Vec2 = translation.into().into_raw();
    let raw = unsafe { ffi::b2World_CastRayClosest(world, o, t, filter.0) };
    RayResult::from_raw(raw)
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
    let o: ffi::b2Vec2 = origin.into().into_raw();
    let t: ffi::b2Vec2 = translation.into().into_raw();
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

fn overlap_polygon_points_into_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    out.clear();
    let mut collect = |shape_id| {
        out.push(shape_id);
        true
    };
    let _ = visit_overlap_polygon_points_impl(world, points, radius, filter, &mut collect);
}

fn visit_overlap_polygon_points_impl<F>(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let Some(proxy) = make_proxy_from_points(points, radius) else {
        return true;
    };
    visit_overlap_shape_proxy_impl(world, &proxy, filter, visit)
}

fn overlap_polygon_points_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_polygon_points_into_impl(world, points, radius, filter, &mut out);
    out
}

fn cast_shape_points_into_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    translation: Vec2,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    out.clear();
    let Some(proxy) = make_proxy_from_points(points, radius) else {
        return;
    };
    let mut ctx = CollectCtx::from_cleared(out);
    let t = translation.into_raw();
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

fn cast_shape_points_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> Vec<RayResult> {
    let mut out = Vec::new();
    cast_shape_points_into_impl(world, points, radius, translation, filter, &mut out);
    out
}

fn cast_mover_impl(
    world: ffi::b2WorldId,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> f32 {
    let cap = make_capsule(c1, c2, radius);
    let t = translation.into_raw();
    unsafe { ffi::b2World_CastMover(world, &cap, t, filter.0) }
}

fn collide_mover_into_impl(
    world: ffi::b2WorldId,
    c1: Vec2,
    c2: Vec2,
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

fn collide_mover_impl(
    world: ffi::b2WorldId,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    filter: QueryFilter,
) -> Vec<MoverPlaneResult> {
    let mut out = Vec::new();
    collide_mover_into_impl(world, c1, c2, radius, filter, &mut out);
    out
}

fn overlap_polygon_points_with_offset_into_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    out.clear();
    let mut collect = |shape_id| {
        out.push(shape_id);
        true
    };
    let _ = visit_overlap_polygon_points_with_offset_impl(
        world,
        points,
        radius,
        position,
        angle_radians,
        filter,
        &mut collect,
    );
}

fn visit_overlap_polygon_points_with_offset_impl<F>(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let Some(proxy) = make_offset_proxy_from_points(points, radius, position, angle_radians) else {
        return true;
    };
    visit_overlap_shape_proxy_impl(world, &proxy, filter, visit)
}

fn overlap_polygon_points_with_offset_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    filter: QueryFilter,
) -> Vec<ShapeId> {
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

fn cast_shape_points_with_offset_into_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    translation: Vec2,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    out.clear();
    let Some(proxy) = make_offset_proxy_from_points(points, radius, position, angle_radians) else {
        return;
    };
    let mut ctx = CollectCtx::from_cleared(out);
    let t = translation.into_raw();
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

fn cast_shape_points_with_offset_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> Vec<RayResult> {
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
fn assert_query_solver_collision_plane_valid(plane: &CollisionPlane) {
    assert!(
        check_query_solver_collision_plane_valid(plane).is_ok(),
        "collision plane must be solver-valid, got {:?}",
        plane
    );
}

#[inline]
fn check_query_solver_collision_plane_valid(plane: &CollisionPlane) -> ApiResult<()> {
    if !plane.plane.is_valid() {
        return Err(crate::error::ApiError::InvalidArgument);
    }
    check_query_non_negative_finite_scalar(plane.push_limit)
}

#[inline]
fn assert_query_collision_plane_valid(plane: &CollisionPlane) {
    assert!(
        check_query_collision_plane_valid(plane).is_ok(),
        "collision plane must be valid, got {:?}",
        plane
    );
}

#[inline]
fn check_query_collision_plane_valid(plane: &CollisionPlane) -> ApiResult<()> {
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

#[inline]
fn checked_query_impl<R>(f: impl FnOnce() -> R) -> R {
    crate::core::callback_state::assert_not_in_callback();
    f()
}

#[inline]
fn try_checked_query_result_impl<R>(f: impl FnOnce() -> ApiResult<R>) -> ApiResult<R> {
    crate::core::callback_state::check_not_in_callback()?;
    f()
}

fn overlap_aabb_checked_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    checked_query_impl(|| {
        assert_query_aabb_valid(aabb);
        overlap_aabb_impl(raw_world_id, aabb, filter)
    })
}

fn visit_overlap_aabb_checked_impl<F>(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    checked_query_impl(|| {
        assert_query_aabb_valid(aabb);
        visit_overlap_aabb_impl(raw_world_id, aabb, filter, visit)
    })
}

fn overlap_aabb_into_checked_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    checked_query_impl(|| {
        assert_query_aabb_valid(aabb);
        overlap_aabb_into_impl(raw_world_id, aabb, filter, out);
    });
}

fn try_overlap_aabb_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>> {
    try_checked_query_result_impl(|| {
        check_query_aabb_valid(aabb)?;
        Ok(overlap_aabb_impl(raw_world_id, aabb, filter))
    })
}

fn try_visit_overlap_aabb_impl<F>(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    visit: &mut F,
) -> ApiResult<bool>
where
    F: FnMut(ShapeId) -> bool,
{
    try_checked_query_result_impl(|| {
        check_query_aabb_valid(aabb)?;
        Ok(visit_overlap_aabb_impl(raw_world_id, aabb, filter, visit))
    })
}

fn try_overlap_aabb_into_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    try_checked_query_result_impl(|| {
        check_query_aabb_valid(aabb)?;
        overlap_aabb_into_impl(raw_world_id, aabb, filter, out);
        Ok(())
    })
}

fn cast_ray_closest_checked_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> RayResult {
    checked_query_impl(|| {
        let origin = origin.into();
        let translation = translation.into();
        assert_query_vec2_valid("origin", origin);
        assert_query_vec2_valid("translation", translation);
        cast_ray_closest_impl(raw_world_id, origin, translation, filter)
    })
}

fn try_cast_ray_closest_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<RayResult> {
    try_checked_query_result_impl(|| {
        let origin = origin.into();
        let translation = translation.into();
        check_query_vec2_valid(origin)?;
        check_query_vec2_valid(translation)?;
        Ok(cast_ray_closest_impl(
            raw_world_id,
            origin,
            translation,
            filter,
        ))
    })
}

fn cast_ray_all_checked_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult> {
    checked_query_impl(|| {
        let origin = origin.into();
        let translation = translation.into();
        assert_query_vec2_valid("origin", origin);
        assert_query_vec2_valid("translation", translation);
        cast_ray_all_impl(raw_world_id, origin, translation, filter)
    })
}

fn cast_ray_all_into_checked_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    checked_query_impl(|| {
        let origin = origin.into();
        let translation = translation.into();
        assert_query_vec2_valid("origin", origin);
        assert_query_vec2_valid("translation", translation);
        cast_ray_all_into_impl(raw_world_id, origin, translation, filter, out);
    });
}

fn try_cast_ray_all_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<Vec<RayResult>> {
    try_checked_query_result_impl(|| {
        let origin = origin.into();
        let translation = translation.into();
        check_query_vec2_valid(origin)?;
        check_query_vec2_valid(translation)?;
        Ok(cast_ray_all_impl(raw_world_id, origin, translation, filter))
    })
}

fn try_cast_ray_all_into_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) -> ApiResult<()> {
    try_checked_query_result_impl(|| {
        let origin = origin.into();
        let translation = translation.into();
        check_query_vec2_valid(origin)?;
        check_query_vec2_valid(translation)?;
        cast_ray_all_into_impl(raw_world_id, origin, translation, filter, out);
        Ok(())
    })
}

fn overlap_polygon_points_checked_impl<I, P>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
) -> Vec<ShapeId>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    checked_query_impl(|| {
        assert_query_non_negative_finite_scalar("radius", radius);
        let points = collect_asserted_proxy_points(points);
        overlap_polygon_points_impl(raw_world_id, &points, radius, filter)
    })
}

fn visit_overlap_polygon_points_checked_impl<I, P, F>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    F: FnMut(ShapeId) -> bool,
{
    checked_query_impl(|| {
        assert_query_non_negative_finite_scalar("radius", radius);
        let points = collect_asserted_proxy_points(points);
        visit_overlap_polygon_points_impl(raw_world_id, &points, radius, filter, visit)
    })
}

fn overlap_polygon_points_into_checked_impl<I, P>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    checked_query_impl(|| {
        assert_query_non_negative_finite_scalar("radius", radius);
        let points = collect_asserted_proxy_points(points);
        overlap_polygon_points_into_impl(raw_world_id, &points, radius, filter, out)
    });
}

fn try_overlap_polygon_points_impl<I, P>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    try_checked_query_result_impl(|| {
        check_query_non_negative_finite_scalar(radius)?;
        let points = try_collect_proxy_points(points)?;
        Ok(overlap_polygon_points_impl(
            raw_world_id,
            &points,
            radius,
            filter,
        ))
    })
}

fn try_visit_overlap_polygon_points_impl<I, P, F>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
    visit: &mut F,
) -> ApiResult<bool>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    F: FnMut(ShapeId) -> bool,
{
    try_checked_query_result_impl(|| {
        check_query_non_negative_finite_scalar(radius)?;
        let points = try_collect_proxy_points(points)?;
        Ok(visit_overlap_polygon_points_impl(
            raw_world_id,
            &points,
            radius,
            filter,
            visit,
        ))
    })
}

fn try_overlap_polygon_points_into_impl<I, P>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    try_checked_query_result_impl(|| {
        check_query_non_negative_finite_scalar(radius)?;
        let points = try_collect_proxy_points(points)?;
        overlap_polygon_points_into_impl(raw_world_id, &points, radius, filter, out);
        Ok(())
    })
}

fn cast_shape_points_checked_impl<I, P, VT>(
    raw_world_id: ffi::b2WorldId,
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
    checked_query_impl(|| {
        let translation = translation.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("translation", translation);
        let points = collect_asserted_proxy_points(points);
        cast_shape_points_impl(raw_world_id, &points, radius, translation, filter)
    })
}

fn cast_shape_points_into_checked_impl<I, P, VT>(
    raw_world_id: ffi::b2WorldId,
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
    checked_query_impl(|| {
        let translation = translation.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("translation", translation);
        let points = collect_asserted_proxy_points(points);
        cast_shape_points_into_impl(raw_world_id, &points, radius, translation, filter, out)
    });
}

fn try_cast_shape_points_impl<I, P, VT>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<Vec<RayResult>>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    VT: Into<Vec2>,
{
    try_checked_query_result_impl(|| {
        let translation = translation.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(translation)?;
        let points = try_collect_proxy_points(points)?;
        Ok(cast_shape_points_impl(
            raw_world_id,
            &points,
            radius,
            translation,
            filter,
        ))
    })
}

fn try_cast_shape_points_into_impl<I, P, VT>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) -> ApiResult<()>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    VT: Into<Vec2>,
{
    try_checked_query_result_impl(|| {
        let translation = translation.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(translation)?;
        let points = try_collect_proxy_points(points)?;
        cast_shape_points_into_impl(raw_world_id, &points, radius, translation, filter, out);
        Ok(())
    })
}

fn cast_mover_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
) -> f32 {
    checked_query_impl(|| {
        let c1 = c1.into();
        let c2 = c2.into();
        let translation = translation.into();
        assert_query_vec2_valid("c1", c1);
        assert_query_vec2_valid("c2", c2);
        assert_query_vec2_valid("translation", translation);
        assert_query_mover_radius_valid(radius);
        cast_mover_impl(raw_world_id, c1, c2, radius, translation, filter)
    })
}

fn try_cast_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<f32> {
    try_checked_query_result_impl(|| {
        let c1 = c1.into();
        let c2 = c2.into();
        let translation = translation.into();
        check_query_vec2_valid(c1)?;
        check_query_vec2_valid(c2)?;
        check_query_vec2_valid(translation)?;
        check_query_mover_radius_valid(radius)?;
        Ok(cast_mover_impl(
            raw_world_id,
            c1,
            c2,
            radius,
            translation,
            filter,
        ))
    })
}

fn collide_mover_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    filter: QueryFilter,
) -> Vec<MoverPlaneResult> {
    checked_query_impl(|| {
        let c1 = c1.into();
        let c2 = c2.into();
        assert_query_vec2_valid("c1", c1);
        assert_query_vec2_valid("c2", c2);
        assert_query_mover_radius_valid(radius);
        collide_mover_impl(raw_world_id, c1, c2, radius, filter)
    })
}

fn collide_mover_into_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<MoverPlaneResult>,
) {
    checked_query_impl(|| {
        let c1 = c1.into();
        let c2 = c2.into();
        assert_query_vec2_valid("c1", c1);
        assert_query_vec2_valid("c2", c2);
        assert_query_mover_radius_valid(radius);
        collide_mover_into_impl(raw_world_id, c1, c2, radius, filter, out);
    });
}

fn try_collide_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    filter: QueryFilter,
) -> ApiResult<Vec<MoverPlaneResult>> {
    try_checked_query_result_impl(|| {
        let c1 = c1.into();
        let c2 = c2.into();
        check_query_vec2_valid(c1)?;
        check_query_vec2_valid(c2)?;
        check_query_mover_radius_valid(radius)?;
        Ok(collide_mover_impl(raw_world_id, c1, c2, radius, filter))
    })
}

fn try_collide_mover_into_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    c1: V1,
    c2: V2,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<MoverPlaneResult>,
) -> ApiResult<()> {
    try_checked_query_result_impl(|| {
        let c1 = c1.into();
        let c2 = c2.into();
        check_query_vec2_valid(c1)?;
        check_query_vec2_valid(c2)?;
        check_query_mover_radius_valid(radius)?;
        collide_mover_into_impl(raw_world_id, c1, c2, radius, filter, out);
        Ok(())
    })
}

fn overlap_polygon_points_with_offset_checked_impl<I, P, V, A>(
    raw_world_id: ffi::b2WorldId,
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
    checked_query_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("position", position);
        assert_query_angle_valid(angle_radians);
        let points = collect_asserted_proxy_points(points);
        overlap_polygon_points_with_offset_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            filter,
        )
    })
}

fn visit_overlap_polygon_points_with_offset_checked_impl<I, P, V, A, F>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
    F: FnMut(ShapeId) -> bool,
{
    checked_query_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("position", position);
        assert_query_angle_valid(angle_radians);
        let points = collect_asserted_proxy_points(points);
        visit_overlap_polygon_points_with_offset_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            visit,
        )
    })
}

fn overlap_polygon_points_with_offset_into_checked_impl<I, P, V, A>(
    raw_world_id: ffi::b2WorldId,
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
    checked_query_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("position", position);
        assert_query_angle_valid(angle_radians);
        let points = collect_asserted_proxy_points(points);
        overlap_polygon_points_with_offset_into_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        )
    });
}

fn try_overlap_polygon_points_with_offset_impl<I, P, V, A>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
{
    try_checked_query_result_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(position)?;
        check_query_angle_valid(angle_radians)?;
        let points = try_collect_proxy_points(points)?;
        Ok(overlap_polygon_points_with_offset_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            filter,
        ))
    })
}

fn try_visit_overlap_polygon_points_with_offset_impl<I, P, V, A, F>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    filter: QueryFilter,
    visit: &mut F,
) -> ApiResult<bool>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
    F: FnMut(ShapeId) -> bool,
{
    try_checked_query_result_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(position)?;
        check_query_angle_valid(angle_radians)?;
        let points = try_collect_proxy_points(points)?;
        Ok(visit_overlap_polygon_points_with_offset_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            visit,
        ))
    })
}

fn try_overlap_polygon_points_with_offset_into_impl<I, P, V, A>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
{
    try_checked_query_result_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(position)?;
        check_query_angle_valid(angle_radians)?;
        let points = try_collect_proxy_points(points)?;
        overlap_polygon_points_with_offset_into_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
        Ok(())
    })
}

fn cast_shape_points_with_offset_checked_impl<I, P, V, A, VT>(
    raw_world_id: ffi::b2WorldId,
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
    checked_query_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        let translation = translation.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("position", position);
        assert_query_angle_valid(angle_radians);
        assert_query_vec2_valid("translation", translation);
        let points = collect_asserted_proxy_points(points);
        cast_shape_points_with_offset_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
    })
}

fn cast_shape_points_with_offset_into_checked_impl<I, P, V, A, VT>(
    raw_world_id: ffi::b2WorldId,
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
    checked_query_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        let translation = translation.into();
        assert_query_non_negative_finite_scalar("radius", radius);
        assert_query_vec2_valid("position", position);
        assert_query_angle_valid(angle_radians);
        assert_query_vec2_valid("translation", translation);
        let points = collect_asserted_proxy_points(points);
        cast_shape_points_with_offset_into_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        )
    });
}

fn try_cast_shape_points_with_offset_impl<I, P, V, A, VT>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<Vec<RayResult>>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
    VT: Into<Vec2>,
{
    try_checked_query_result_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        let translation = translation.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(position)?;
        check_query_angle_valid(angle_radians)?;
        check_query_vec2_valid(translation)?;
        let points = try_collect_proxy_points(points)?;
        Ok(cast_shape_points_with_offset_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        ))
    })
}

fn try_cast_shape_points_with_offset_into_impl<I, P, V, A, VT>(
    raw_world_id: ffi::b2WorldId,
    points: I,
    radius: f32,
    position: V,
    angle_radians: A,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) -> ApiResult<()>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
    V: Into<Vec2>,
    A: Into<f32>,
    VT: Into<Vec2>,
{
    try_checked_query_result_impl(|| {
        let position = position.into();
        let angle_radians = angle_radians.into();
        let translation = translation.into();
        check_query_non_negative_finite_scalar(radius)?;
        check_query_vec2_valid(position)?;
        check_query_angle_valid(angle_radians)?;
        check_query_vec2_valid(translation)?;
        let points = try_collect_proxy_points(points)?;
        cast_shape_points_with_offset_into_impl(
            raw_world_id,
            &points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        );
        Ok(())
    })
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
        overlap_aabb_checked_impl(self.raw(), aabb, filter)
    }

    /// Overlap test for all shapes in an AABB and write matching shape ids into `out`.
    ///
    /// `out` is cleared before new hits are appended so its allocation can be reused across frames.
    pub fn overlap_aabb_into(&self, aabb: Aabb, filter: QueryFilter, out: &mut Vec<ShapeId>) {
        overlap_aabb_into_checked_impl(self.raw(), aabb, filter, out);
    }

    /// Visit matching shape ids in an AABB without allocating a result container.
    ///
    /// Return `true` from the visitor to continue, or `false` to stop early.
    /// Returns `true` if all hits were visited, or `false` if the visitor stopped early.
    pub fn visit_overlap_aabb<F>(&self, aabb: Aabb, filter: QueryFilter, mut visit: F) -> bool
    where
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_aabb_checked_impl(self.raw(), aabb, filter, &mut visit)
    }

    pub fn try_overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> ApiResult<Vec<ShapeId>> {
        try_overlap_aabb_impl(self.raw(), aabb, filter)
    }

    pub fn try_overlap_aabb_into(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()> {
        try_overlap_aabb_into_impl(self.raw(), aabb, filter, out)
    }

    pub fn try_visit_overlap_aabb<F>(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_aabb_impl(self.raw(), aabb, filter, &mut visit)
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
        cast_ray_closest_checked_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<RayResult> {
        try_cast_ray_closest_impl(self.raw(), origin, translation, filter)
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
        cast_ray_all_checked_impl(self.raw(), origin, translation, filter)
    }

    /// Cast a ray and append all hits into `out`, reusing the caller-owned allocation.
    pub fn cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        cast_ray_all_into_checked_impl(self.raw(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        try_cast_ray_all_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()> {
        try_cast_ray_all_into_impl(self.raw(), origin, translation, filter, out)
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
        overlap_polygon_points_checked_impl(self.raw(), points, radius, filter)
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
        overlap_polygon_points_into_checked_impl(self.raw(), points, radius, filter, out);
    }

    /// Visit matching shape ids for a temporary polygon proxy without allocating a result container.
    ///
    /// Return `true` from the visitor to continue, or `false` to stop early.
    /// Returns `true` if all hits were visited, or `false` if the visitor stopped early.
    pub fn visit_overlap_polygon_points<I, P, F>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_polygon_points_checked_impl(self.raw(), points, radius, filter, &mut visit)
    }

    pub fn try_overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_impl(self.raw(), points, radius, filter)
    }

    pub fn try_overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out)
    }

    pub fn try_visit_overlap_polygon_points<I, P, F>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_polygon_points_impl(self.raw(), points, radius, filter, &mut visit)
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
        cast_shape_points_checked_impl(self.raw(), points, radius, translation, filter)
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
        cast_shape_points_into_checked_impl(self.raw(), points, radius, translation, filter, out);
    }

    pub fn try_cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_impl(self.raw(), points, radius, translation, filter)
    }

    pub fn try_cast_shape_points_into<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out)
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
        cast_mover_checked_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn try_cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<f32> {
        try_cast_mover_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    /// Collect collision planes for a capsule mover at its current position.
    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        collide_mover_checked_impl(self.raw(), c1, c2, radius, filter)
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
        collide_mover_into_checked_impl(self.raw(), c1, c2, radius, filter, out);
    }

    pub fn try_collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<MoverPlaneResult>> {
        try_collide_mover_impl(self.raw(), c1, c2, radius, filter)
    }

    pub fn try_collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) -> ApiResult<()> {
        try_collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out)
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
        overlap_polygon_points_with_offset_checked_impl(
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
        overlap_polygon_points_with_offset_into_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
    }

    /// Visit matching shape ids for an offset temporary polygon proxy without allocating a result container.
    ///
    /// Return `true` from the visitor to continue, or `false` to stop early.
    /// Returns `true` if all hits were visited, or `false` if the visitor stopped early.
    pub fn visit_overlap_polygon_points_with_offset<I, P, V, A, F>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_polygon_points_with_offset_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
    }

    pub fn try_overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        try_overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn try_overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        try_overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        )
    }

    pub fn try_visit_overlap_polygon_points_with_offset<I, P, V, A, F>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
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
        cast_shape_points_with_offset_checked_impl(
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
        cast_shape_points_with_offset_into_checked_impl(
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
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
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
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        )
    }
}

impl WorldHandle {
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        overlap_aabb_checked_impl(self.raw(), aabb, filter)
    }

    pub fn overlap_aabb_into(&self, aabb: Aabb, filter: QueryFilter, out: &mut Vec<ShapeId>) {
        overlap_aabb_into_checked_impl(self.raw(), aabb, filter, out);
    }

    pub fn visit_overlap_aabb<F>(&self, aabb: Aabb, filter: QueryFilter, mut visit: F) -> bool
    where
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_aabb_checked_impl(self.raw(), aabb, filter, &mut visit)
    }

    pub fn try_overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> ApiResult<Vec<ShapeId>> {
        try_overlap_aabb_impl(self.raw(), aabb, filter)
    }

    pub fn try_overlap_aabb_into(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()> {
        try_overlap_aabb_into_impl(self.raw(), aabb, filter, out)
    }

    pub fn try_visit_overlap_aabb<F>(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_aabb_impl(self.raw(), aabb, filter, &mut visit)
    }

    pub fn cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> RayResult {
        cast_ray_closest_checked_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<RayResult> {
        try_cast_ray_closest_impl(self.raw(), origin, translation, filter)
    }

    pub fn cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        cast_ray_all_checked_impl(self.raw(), origin, translation, filter)
    }

    pub fn cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        cast_ray_all_into_checked_impl(self.raw(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        try_cast_ray_all_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()> {
        try_cast_ray_all_into_impl(self.raw(), origin, translation, filter, out)
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
        overlap_polygon_points_checked_impl(self.raw(), points, radius, filter)
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
        overlap_polygon_points_into_checked_impl(self.raw(), points, radius, filter, out);
    }

    pub fn visit_overlap_polygon_points<I, P, F>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_polygon_points_checked_impl(self.raw(), points, radius, filter, &mut visit)
    }

    pub fn try_overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_impl(self.raw(), points, radius, filter)
    }

    pub fn try_overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out)
    }

    pub fn try_visit_overlap_polygon_points<I, P, F>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_polygon_points_impl(self.raw(), points, radius, filter, &mut visit)
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
        cast_shape_points_checked_impl(self.raw(), points, radius, translation, filter)
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
        cast_shape_points_into_checked_impl(self.raw(), points, radius, translation, filter, out);
    }

    pub fn try_cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_impl(self.raw(), points, radius, translation, filter)
    }

    pub fn try_cast_shape_points_into<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out)
    }

    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        cast_mover_checked_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn try_cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<f32> {
        try_cast_mover_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        collide_mover_checked_impl(self.raw(), c1, c2, radius, filter)
    }

    pub fn collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) {
        collide_mover_into_checked_impl(self.raw(), c1, c2, radius, filter, out);
    }

    pub fn try_collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<MoverPlaneResult>> {
        try_collide_mover_impl(self.raw(), c1, c2, radius, filter)
    }

    pub fn try_collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) -> ApiResult<()> {
        try_collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out)
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
        overlap_polygon_points_with_offset_checked_impl(
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
        overlap_polygon_points_with_offset_into_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
    }

    pub fn visit_overlap_polygon_points_with_offset<I, P, V, A, F>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_polygon_points_with_offset_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
    }

    pub fn try_overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        try_overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn try_overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        try_overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        )
    }

    pub fn try_visit_overlap_polygon_points_with_offset<I, P, V, A, F>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
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
        cast_shape_points_with_offset_checked_impl(
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
        cast_shape_points_with_offset_into_checked_impl(
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
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
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
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        )
    }
}
