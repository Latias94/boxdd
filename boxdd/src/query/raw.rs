#![allow(
    clippy::too_many_arguments,
    reason = "raw helpers preserve Box2D query arguments and the explicit world origin"
)]

use boxdd_sys::ffi;

#[cfg(not(target_arch = "wasm32"))]
use crate::error::{Error, Result};
use crate::types::{Position, Vec2};

#[cfg(not(target_arch = "wasm32"))]
use super::buffers::{
    MoverQueryBuffer, RawMoverPlane, RawRayHit, RayQueryBuffer, ShapeQueryBuffer,
};
use super::types::*;
use crate::world::QueryCall;

#[cfg(not(target_arch = "wasm32"))]
struct CollectCtx<'a, B> {
    buffer: &'a mut B,
    error: Option<Error>,
    panic: crate::core::callback_state::PanicSlot,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, B> CollectCtx<'a, B> {
    fn new(buffer: &'a mut B) -> Self {
        Self {
            buffer,
            error: None,
            panic: crate::core::callback_state::PanicSlot::default(),
        }
    }

    fn push(&mut self, push: impl FnOnce(&mut B) -> Result<()>) -> bool {
        if self.error.is_some() || self.panic.has_panicked() {
            return false;
        }
        let result =
            crate::core::callback_state::invoke_owner_callback(&mut self.panic, None, || {
                Some(push(self.buffer))
            });
        match result {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                self.error = Some(error);
                false
            }
            None => false,
        }
    }

    fn finish(self) -> Result<()> {
        let Self { error, panic, .. } = self;
        panic.resume_or_forget();
        error.map_or(Ok(()), Err)
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn collect_shape_id_cb(
    shape_id: ffi::b2ShapeId,
    context: *mut core::ffi::c_void,
) -> bool {
    let context = context.cast::<CollectCtx<'_, ShapeQueryBuffer>>();
    if context.is_null() || !context.is_aligned() {
        return false;
    }
    let context = unsafe { &mut *context };
    context.push(|buffer| buffer.push_raw(shape_id))
}

#[allow(clippy::unnecessary_cast)]
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn collect_ray_result_cb(
    shape_id: ffi::b2ShapeId,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    fraction: f32,
    context: *mut core::ffi::c_void,
) -> f32 {
    let context = context.cast::<CollectCtx<'_, RayQueryBuffer>>();
    if context.is_null() || !context.is_aligned() {
        return 0.0;
    }
    let context = unsafe { &mut *context };
    if context.push(|buffer| {
        buffer.push_raw(RawRayHit {
            shape_id,
            point,
            normal,
            fraction,
        })
    }) {
        1.0f32
    } else {
        0.0f32
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn collect_mover_plane_result_cb(
    shape_id: ffi::b2ShapeId,
    plane: *const ffi::b2PlaneResult,
    context: *mut core::ffi::c_void,
) -> bool {
    let context = context.cast::<CollectCtx<'_, MoverQueryBuffer>>();
    if context.is_null() || !context.is_aligned() {
        return false;
    }
    let context = unsafe { &mut *context };
    if plane.is_null() || !plane.is_aligned() {
        context.error = ::core::option::Option::Some(Error::InvalidNativeOutput {
            operation: "Query::collide_mover",
            output: "plane",
            constraint: "a non-null aligned mover-plane pointer",
        });
        return false;
    }
    let plane = unsafe { *plane };
    context.push(|buffer| buffer.push_raw(RawMoverPlane { shape_id, plane }))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_aabb(
    call: &QueryCall<'_>,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    buffer: &mut ShapeQueryBuffer,
) -> Result<()> {
    let mut context = CollectCtx::new(buffer);
    unsafe {
        let _ = ffi::b2World_OverlapAABB(
            call.raw_world(),
            origin.into_raw(),
            aabb.into_raw(),
            filter.0,
            Some(collect_shape_id_cb),
            &mut context as *mut _ as *mut _,
        );
    }
    context.finish()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn overlap_shape(
    call: &QueryCall<'_>,
    origin: Position,
    proxy: &ffi::b2ShapeProxy,
    filter: QueryFilter,
    buffer: &mut ShapeQueryBuffer,
) -> Result<()> {
    let mut context = CollectCtx::new(buffer);
    unsafe {
        let _ = ffi::b2World_OverlapShape(
            call.raw_world(),
            origin.into_raw(),
            proxy,
            filter.0,
            Some(collect_shape_id_cb),
            &mut context as *mut _ as *mut _,
        );
    }
    context.finish()
}

pub(super) fn cast_ray_closest(
    call: &QueryCall<'_>,
    origin: Position,
    translation: Vec2,
    filter: QueryFilter,
) -> ffi::b2RayResult {
    unsafe {
        ffi::b2World_CastRayClosest(
            call.raw_world(),
            origin.into_raw(),
            translation.into_raw(),
            filter.0,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_ray_all(
    call: &QueryCall<'_>,
    origin: Position,
    translation: Vec2,
    filter: QueryFilter,
    buffer: &mut RayQueryBuffer,
) -> Result<()> {
    let mut context = CollectCtx::new(buffer);
    unsafe {
        let _ = ffi::b2World_CastRay(
            call.raw_world(),
            origin.into_raw(),
            translation.into_raw(),
            filter.0,
            Some(collect_ray_result_cb),
            &mut context as *mut _ as *mut _,
        );
    }
    context.finish()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cast_shape(
    call: &QueryCall<'_>,
    origin: Position,
    proxy: &ffi::b2ShapeProxy,
    translation: Vec2,
    filter: QueryFilter,
    buffer: &mut RayQueryBuffer,
) -> Result<()> {
    let mut context = CollectCtx::new(buffer);
    unsafe {
        let _ = ffi::b2World_CastShape(
            call.raw_world(),
            origin.into_raw(),
            proxy,
            translation.into_raw(),
            filter.0,
            Some(collect_ray_result_cb),
            &mut context as *mut _ as *mut _,
        );
    }
    context.finish()
}

pub(super) fn make_capsule(c1: Vec2, c2: Vec2, radius: f32) -> ffi::b2Capsule {
    ffi::b2Capsule {
        center1: c1.into_raw(),
        center2: c2.into_raw(),
        radius,
    }
}

pub(super) fn cast_mover(
    call: &QueryCall<'_>,
    origin: Position,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    translation: Vec2,
    filter: QueryFilter,
) -> f32 {
    let capsule = make_capsule(c1, c2, radius);
    unsafe {
        ffi::b2World_CastMover(
            call.raw_world(),
            origin.into_raw(),
            &capsule,
            translation.into_raw(),
            filter.0,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn collide_mover(
    call: &QueryCall<'_>,
    origin: Position,
    c1: Vec2,
    c2: Vec2,
    radius: f32,
    filter: QueryFilter,
    buffer: &mut MoverQueryBuffer,
) -> Result<()> {
    let capsule = make_capsule(c1, c2, radius);
    let mut context = CollectCtx::new(buffer);
    unsafe {
        ffi::b2World_CollideMover(
            call.raw_world(),
            origin.into_raw(),
            &capsule,
            filter.0,
            ::core::option::Option::Some(collect_mover_plane_result_cb),
            &mut context as *mut _ as *mut _,
        );
    }
    context.finish()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn callbacks_match_box2d_typedefs() {
        let _: ffi::b2CastResultFcn = Some(collect_ray_result_cb);
        let _: ffi::b2PlaneResultFcn = Some(collect_mover_plane_result_cb);
        let _: ffi::b2OverlapResultFcn = Some(collect_shape_id_cb);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn query_callback_trampolines_fail_closed_on_invalid_native_pointers() {
        let shape_id: ffi::b2ShapeId = unsafe { core::mem::zeroed() };
        assert!(!unsafe { collect_shape_id_cb(shape_id, core::ptr::null_mut()) });
        assert_eq!(
            unsafe {
                collect_ray_result_cb(
                    shape_id,
                    Position::ZERO.into_raw(),
                    Vec2::ZERO.into_raw(),
                    0.0,
                    core::ptr::null_mut(),
                )
            },
            0.0
        );
        assert!(!unsafe {
            collect_mover_plane_result_cb(shape_id, core::ptr::null(), core::ptr::null_mut())
        });

        let mut buffer = MoverQueryBuffer::new();
        let mut context = CollectCtx::new(&mut buffer);
        let context_pointer = core::ptr::from_mut(&mut context).cast::<core::ffi::c_void>();
        assert!(!unsafe {
            collect_mover_plane_result_cb(shape_id, core::ptr::null(), context_pointer)
        });
        assert_eq!(
            context.error,
            Some(Error::InvalidNativeOutput {
                operation: "Query::collide_mover",
                output: "plane",
                constraint: "a non-null aligned mover-plane pointer",
            })
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn world_queries_match_origin_aware_box2d_signatures() {
        type OverlapAabb = unsafe extern "C" fn(
            ffi::b2WorldId,
            ffi::b2Pos,
            ffi::b2AABB,
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

        let _: OverlapAabb = ffi::b2World_OverlapAABB;
        let _: CastShape = ffi::b2World_CastShape;
    }
}
