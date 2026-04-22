use crate::error::ApiResult;
use crate::types::{ShapeId, Vec2};
use boxdd_sys::ffi;
use smallvec::SmallVec;
use std::any::Any;

use super::types::*;

const MAX_PROXY_POINTS: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;
type ProxyPoints = SmallVec<[ffi::b2Vec2; MAX_PROXY_POINTS]>;
type PanicPayload = Box<dyn Any + Send + 'static>;

pub(super) fn collect_asserted_proxy_points<I, P>(points: I) -> ProxyPoints
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

pub(super) fn try_collect_proxy_points<I, P>(points: I) -> ApiResult<ProxyPoints>
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
pub(super) fn make_proxy_from_points(
    points: &ProxyPoints,
    radius: f32,
) -> Option<ffi::b2ShapeProxy> {
    (!points.is_empty())
        .then(|| unsafe { ffi::b2MakeProxy(points.as_ptr(), points.len() as i32, radius) })
}

#[inline]
pub(super) fn make_offset_proxy_from_points(
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

pub(super) fn make_capsule<V1: Into<Vec2>, V2: Into<Vec2>>(
    c1: V1,
    c2: V2,
    radius: f32,
) -> ffi::b2Capsule {
    crate::shapes::Capsule::new(c1, c2, radius).into_raw()
}

pub(super) fn visit_overlap_aabb_impl<F>(
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

pub(super) fn overlap_aabb_into_impl(
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

pub(super) fn overlap_aabb_impl(
    world: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_aabb_into_impl(world, aabb, filter, &mut out);
    out
}

pub(super) fn visit_overlap_shape_proxy_impl<F>(
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

pub(super) fn cast_ray_closest_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn cast_ray_all_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    world: ffi::b2WorldId,
    origin: VO,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult> {
    let mut out = Vec::new();
    cast_ray_all_into_impl(world, origin, translation, filter, &mut out);
    out
}

pub(super) fn cast_ray_all_into_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn overlap_polygon_points_into_impl(
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

pub(super) fn visit_overlap_polygon_points_impl<F>(
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

pub(super) fn overlap_polygon_points_impl(
    world: ffi::b2WorldId,
    points: &ProxyPoints,
    radius: f32,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_polygon_points_into_impl(world, points, radius, filter, &mut out);
    out
}

pub(super) fn cast_shape_points_into_impl(
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

pub(super) fn cast_shape_points_impl(
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

pub(super) fn cast_mover_impl(
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

pub(super) fn collide_mover_into_impl(
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

pub(super) fn collide_mover_impl(
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

pub(super) fn overlap_polygon_points_with_offset_into_impl(
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

pub(super) fn visit_overlap_polygon_points_with_offset_impl<F>(
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

pub(super) fn overlap_polygon_points_with_offset_impl(
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

pub(super) fn cast_shape_points_with_offset_into_impl(
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

pub(super) fn cast_shape_points_with_offset_impl(
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
