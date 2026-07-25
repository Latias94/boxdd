#![allow(
    clippy::too_many_arguments,
    reason = "raw helpers preserve Box2D query arguments and the explicit world origin"
)]

use crate::error::ApiResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::types::ShapeId;
use crate::types::{Position, Vec2};
use boxdd_sys::ffi;
#[cfg(not(target_arch = "wasm32"))]
use smallvec::SmallVec;

use super::QueryTarget;
use super::types::*;

#[cfg(not(target_arch = "wasm32"))]
const MAX_PROXY_POINTS: usize = ffi::B2_MAX_POLYGON_VERTICES as usize;
#[cfg(not(target_arch = "wasm32"))]
type ProxyPoints = SmallVec<[ffi::b2Vec2; MAX_PROXY_POINTS]>;

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn make_proxy_from_points(
    points: &ProxyPoints,
    radius: f32,
) -> Option<ffi::b2ShapeProxy> {
    if points.is_empty() {
        None
    } else {
        Some(unsafe { ffi::b2MakeProxy(points.as_ptr(), points.len() as i32, radius) })
    }
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn make_offset_proxy_from_points(
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
) -> Option<ffi::b2ShapeProxy> {
    if points.is_empty() {
        None
    } else {
        let (s, c) = angle_radians.sin_cos();
        Some(unsafe {
            ffi::b2MakeOffsetProxy(
                points.as_ptr(),
                points.len() as i32,
                radius,
                position.into_raw(),
                ffi::b2Rot { c, s },
            )
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct CollectCtx<'a, T> {
    out: &'a mut Vec<T>,
    brand: crate::id::IdBrand,
    panic: crate::core::callback_state::PanicSlot,
    invalid_output: Option<crate::error::ApiError>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, T> CollectCtx<'a, T> {
    fn from_cleared(out: &'a mut Vec<T>, brand: crate::id::IdBrand) -> Self {
        Self {
            out,
            brand,
            panic: crate::core::callback_state::PanicSlot::default(),
            invalid_output: None,
        }
    }

    fn shape(&mut self, raw: ffi::b2ShapeId) -> Option<ShapeId> {
        match self.brand.try_shape(raw) {
            ::std::result::Result::Ok(id) => ::std::option::Option::Some(id),
            ::std::result::Result::Err(error) => {
                self.invalid_output = ::std::option::Option::Some(error);
                ::std::option::Option::None
            }
        }
    }

    fn push(&mut self, value: T) -> bool {
        crate::core::callback_state::invoke_owner_callback(&mut self.panic, false, || {
            self.out.push(value);
            true
        })
    }

    fn finish(mut self) {
        if !crate::core::callback_state::PanicSlot::has_panicked(&self.panic)
            && let ::std::option::Option::Some(error) = self.invalid_output
        {
            crate::core::callback_state::PanicSlot::run_cleanup(&mut self.panic, || {
                panic!("Box2D returned an invalid query shape id: {error}")
            });
        }
        crate::core::callback_state::PanicSlot::resume_or_forget(self.panic);
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct VisitShapeIdCtx<'a, F> {
    visit: &'a mut F,
    brand: crate::id::IdBrand,
    stopped_early: bool,
    panic: crate::core::callback_state::PanicSlot,
    invalid_output: Option<crate::error::ApiError>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, F> VisitShapeIdCtx<'a, F>
where
    F: FnMut(ShapeId) -> bool,
{
    fn new(visit: &'a mut F, brand: crate::id::IdBrand) -> Self {
        Self {
            visit,
            brand,
            stopped_early: false,
            panic: crate::core::callback_state::PanicSlot::default(),
            invalid_output: None,
        }
    }

    fn shape(&mut self, raw: ffi::b2ShapeId) -> Option<ShapeId> {
        match self.brand.try_shape(raw) {
            Ok(id) => Some(id),
            Err(error) => {
                self.invalid_output = Some(error);
                None
            }
        }
    }

    fn visit(&mut self, shape_id: ShapeId) -> bool {
        if self.stopped_early || self.panic.has_panicked() {
            return false;
        }
        let result =
            crate::core::callback_state::invoke_owner_callback(&mut self.panic, false, || {
                (self.visit)(shape_id)
            });
        if result {
            true
        } else {
            if !self.panic.has_panicked() {
                self.stopped_early = true;
            }
            false
        }
    }

    fn finish(mut self) -> bool {
        let completed =
            !self.stopped_early && !self.panic.has_panicked() && self.invalid_output.is_none();
        if !self.panic.has_panicked()
            && let Some(error) = self.invalid_output
        {
            self.panic
                .run_cleanup(|| panic!("Box2D returned an invalid query shape id: {error}"));
        }
        self.panic.resume_or_forget();
        completed
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn visit_shape_id_cb<F>(
    shape_id: ffi::b2ShapeId,
    ctx: *mut core::ffi::c_void,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let ctx = unsafe { &mut *(ctx as *mut VisitShapeIdCtx<'_, F>) };
    let Some(shape_id) = ctx.shape(shape_id) else {
        return false;
    };
    ctx.visit(shape_id)
}

#[allow(clippy::unnecessary_cast)]
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn collect_ray_result_cb(
    shape_id: ffi::b2ShapeId,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    fraction: f32,
    ctx: *mut core::ffi::c_void,
) -> f32 {
    let ctx = unsafe { &mut *(ctx as *mut CollectCtx<'_, RayResult>) };
    let Some(shape_id) = ctx.shape(shape_id) else {
        return 0.0;
    };
    if ctx.push(RayResult {
        shape_id,
        point: Position::from_raw(point),
        normal: Vec2::from_raw(normal),
        fraction,
        hit: true,
    }) {
        1.0f32
    } else {
        0.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn collect_mover_plane_result_cb(
    shape_id: ffi::b2ShapeId,
    plane: *const ffi::b2PlaneResult,
    ctx: *mut core::ffi::c_void,
) -> bool {
    let ctx = unsafe { &mut *(ctx as *mut CollectCtx<'_, MoverPlaneResult>) };
    let plane = unsafe { *plane };
    let ::std::option::Option::Some(shape_id) = ctx.shape(shape_id) else {
        return false;
    };
    ctx.push(MoverPlaneResult {
        shape_id,
        plane: Plane::from_raw(plane.plane),
        point: Vec2::from_raw(plane.point),
        hit: plane.hit,
    })
}

pub(super) fn make_capsule(c1: Vec2, c2: Vec2, radius: f32) -> ffi::b2Capsule {
    crate::shapes::Capsule::new(c1, c2, radius).into_raw()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn visit_overlap_aabb_impl<F>(
    target: &QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let mut ctx = VisitShapeIdCtx::new(visit, target.brand());
    unsafe {
        let _ = ffi::b2World_OverlapAABB(
            target.raw(),
            origin.into_raw(),
            aabb.into_raw(),
            filter.0,
            Some(visit_shape_id_cb::<F>),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_aabb_into_impl(
    target: &QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    out.clear();
    let mut collect = |shape_id| {
        out.push(shape_id);
        true
    };
    let _ = visit_overlap_aabb_impl(target, origin, aabb, filter, &mut collect);
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_aabb_impl(
    target: &QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_aabb_into_impl(target, origin, aabb, filter, &mut out);
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn visit_overlap_shape_proxy_impl<F>(
    target: &QueryTarget,
    origin: Position,
    proxy: &ffi::b2ShapeProxy,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    let mut ctx = VisitShapeIdCtx::new(visit, target.brand());
    unsafe {
        let _ = ffi::b2World_OverlapShape(
            target.raw(),
            origin.into_raw(),
            proxy,
            filter.0,
            Some(visit_shape_id_cb::<F>),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish()
}

pub(super) fn cast_ray_closest_with_stats_impl(
    target: &QueryTarget,
    origin: Position,
    translation: Vec2,
    filter: QueryFilter,
) -> ApiResult<ClosestRayCastResult> {
    let o = origin.into_raw();
    let t = translation.into_raw();
    let raw = unsafe { ffi::b2World_CastRayClosest(target.raw(), o, t, filter.0) };
    ClosestRayCastResult::from_raw_in(target.brand(), raw)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_ray_all_impl(
    target: &QueryTarget,
    origin: Position,
    translation: Vec2,
    filter: QueryFilter,
) -> Vec<RayResult> {
    let mut out = Vec::new();
    cast_ray_all_into_impl(target, origin, translation, filter, &mut out);
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_ray_all_into_impl(
    target: &QueryTarget,
    origin: Position,
    translation: Vec2,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    out.clear();
    let mut ctx = CollectCtx::from_cleared(out, target.brand());
    let o = origin.into_raw();
    let t = translation.into_raw();
    unsafe {
        let _ = ffi::b2World_CastRay(
            target.raw(),
            o,
            t,
            filter.0,
            Some(collect_ray_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish();
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_polygon_points_into_impl(
    target: &QueryTarget,
    origin: Position,
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
    let _ = visit_overlap_polygon_points_impl(target, origin, points, radius, filter, &mut collect);
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn visit_overlap_polygon_points_impl<F>(
    target: &QueryTarget,
    origin: Position,
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
    visit_overlap_shape_proxy_impl(target, origin, &proxy, filter, visit)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_polygon_points_impl(
    target: &QueryTarget,
    origin: Position,
    points: &ProxyPoints,
    radius: f32,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_polygon_points_into_impl(target, origin, points, radius, filter, &mut out);
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_shape_points_into_impl(
    target: &QueryTarget,
    origin: Position,
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
    let mut ctx = CollectCtx::from_cleared(out, target.brand());
    let t = translation.into_raw();
    unsafe {
        let _ = ffi::b2World_CastShape(
            target.raw(),
            origin.into_raw(),
            &proxy,
            t,
            filter.0,
            Some(collect_ray_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish();
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_shape_points_impl(
    target: &QueryTarget,
    origin: Position,
    points: &ProxyPoints,
    radius: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> Vec<RayResult> {
    let mut out = Vec::new();
    cast_shape_points_into_impl(
        target,
        origin,
        points,
        radius,
        translation,
        filter,
        &mut out,
    );
    out
}

pub(super) fn cast_mover_impl(
    target: &QueryTarget,
    origin: Position,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> f32 {
    let cap = make_capsule(c1, c2, radius);
    let t = translation.into_raw();
    unsafe { ffi::b2World_CastMover(target.raw(), origin.into_raw(), &cap, t, filter.0) }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn collide_mover_into_impl(
    target: &QueryTarget,
    origin: Position,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<MoverPlaneResult>,
) {
    out.clear();
    let cap = make_capsule(c1, c2, radius);
    let mut ctx = CollectCtx::from_cleared(out, target.brand());
    unsafe {
        ffi::b2World_CollideMover(
            target.raw(),
            origin.into_raw(),
            &cap,
            filter.0,
            ::std::option::Option::Some(collect_mover_plane_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish();
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn collide_mover_impl(
    target: &QueryTarget,
    origin: Position,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    filter: QueryFilter,
) -> Vec<MoverPlaneResult> {
    let mut out = Vec::new();
    collide_mover_into_impl(target, origin, c1, c2, radius, filter, &mut out);
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_polygon_points_with_offset_into_impl(
    target: &QueryTarget,
    origin: Position,
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
        target,
        origin,
        points,
        radius,
        position,
        angle_radians,
        filter,
        &mut collect,
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn visit_overlap_polygon_points_with_offset_impl<F>(
    target: &QueryTarget,
    origin: Position,
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
    visit_overlap_shape_proxy_impl(target, origin, &proxy, filter, visit)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_polygon_points_with_offset_impl(
    target: &QueryTarget,
    origin: Position,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    let mut out = Vec::new();
    overlap_polygon_points_with_offset_into_impl(
        target,
        origin,
        points,
        radius,
        position,
        angle_radians,
        filter,
        &mut out,
    );
    out
}

#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_shape_points_with_offset_into_impl(
    target: &QueryTarget,
    origin: Position,
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
    let mut ctx = CollectCtx::from_cleared(out, target.brand());
    let t = translation.into_raw();
    unsafe {
        let _ = ffi::b2World_CastShape(
            target.raw(),
            origin.into_raw(),
            &proxy,
            t,
            filter.0,
            Some(collect_ray_result_cb),
            &mut ctx as *mut _ as *mut _,
        );
    }
    ctx.finish();
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_shape_points_with_offset_impl(
    target: &QueryTarget,
    origin: Position,
    points: &ProxyPoints,
    radius: f32,
    position: Vec2,
    angle_radians: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> Vec<RayResult> {
    let mut out = Vec::new();
    cast_shape_points_with_offset_into_impl(
        target,
        origin,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callbacks_match_box2d_32_typedefs() {
        let _: ffi::b2CastResultFcn = Some(collect_ray_result_cb);
        let _: ffi::b2PlaneResultFcn = Some(collect_mover_plane_result_cb);
        let _: ffi::b2OverlapResultFcn = Some(visit_shape_id_cb::<fn(ShapeId) -> bool>);
    }

    #[test]
    fn world_queries_match_origin_aware_box2d_32_signatures() {
        type OverlapAabb = unsafe extern "C" fn(
            ffi::b2WorldId,
            ffi::b2Pos,
            ffi::b2AABB,
            ffi::b2QueryFilter,
            ffi::b2OverlapResultFcn,
            *mut core::ffi::c_void,
        ) -> ffi::b2TreeStats;
        type OverlapShape = unsafe extern "C" fn(
            ffi::b2WorldId,
            ffi::b2Pos,
            *const ffi::b2ShapeProxy,
            ffi::b2QueryFilter,
            ffi::b2OverlapResultFcn,
            *mut core::ffi::c_void,
        ) -> ffi::b2TreeStats;
        type CastShape = unsafe extern "C" fn(
            ffi::b2WorldId,
            ffi::b2Pos,
            *const ffi::b2ShapeProxy,
            ffi::b2Vec2,
            ffi::b2QueryFilter,
            ffi::b2CastResultFcn,
            *mut core::ffi::c_void,
        ) -> ffi::b2TreeStats;
        type CastMover = unsafe extern "C" fn(
            ffi::b2WorldId,
            ffi::b2Pos,
            *const ffi::b2Capsule,
            ffi::b2Vec2,
            ffi::b2QueryFilter,
        ) -> f32;
        type CollideMover = unsafe extern "C" fn(
            ffi::b2WorldId,
            ffi::b2Pos,
            *const ffi::b2Capsule,
            ffi::b2QueryFilter,
            ffi::b2PlaneResultFcn,
            *mut core::ffi::c_void,
        );

        let _: OverlapAabb = ffi::b2World_OverlapAABB;
        let _: OverlapShape = ffi::b2World_OverlapShape;
        let _: CastShape = ffi::b2World_CastShape;
        let _: CastMover = ffi::b2World_CastMover;
        let _: CollideMover = ffi::b2World_CollideMover;
    }

    #[test]
    fn panicking_overlap_visitor_returns_stop_sentinel() {
        fn panic_visitor(_: ShapeId) -> bool {
            panic!("overlap visitor test panic");
        }

        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let body = world.create_body_id(
            crate::BodyBuilder::new()
                .body_type(crate::BodyType::Dynamic)
                .build(),
        );
        let shape = world.create_polygon_shape_for(
            body,
            &crate::ShapeDef::builder().density(1.0).build(),
            &crate::shapes::box_polygon(0.5, 0.5),
        );
        let mut visitor: fn(ShapeId) -> bool = panic_visitor;
        let mut context = VisitShapeIdCtx::new(&mut visitor, world.brand());
        let context_ptr = core::ptr::from_mut(&mut context).cast::<core::ffi::c_void>();

        // SAFETY: `context_ptr` refers to the live context and `shape` belongs to its brand.
        let first = unsafe {
            visit_shape_id_cb::<fn(ShapeId) -> bool>(shape.unbind().into_ffi(), context_ptr)
        };
        let second = unsafe {
            visit_shape_id_cb::<fn(ShapeId) -> bool>(shape.unbind().into_ffi(), context_ptr)
        };
        assert!(!first);
        assert!(!second);
        assert!(context.panic.has_panicked());
    }
}
