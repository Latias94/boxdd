use crate::error::ApiResult;
use crate::types::{ShapeId, Vec2};
use boxdd_sys::ffi;

use super::raw::*;
use super::types::*;

pub(super) fn checked_query_impl<R>(f: impl FnOnce() -> R) -> R {
    crate::core::callback_state::assert_not_in_callback();
    f()
}

#[inline]
pub(super) fn try_checked_query_result_impl<R>(f: impl FnOnce() -> ApiResult<R>) -> ApiResult<R> {
    crate::core::callback_state::check_not_in_callback()?;
    f()
}

pub(super) fn overlap_aabb_checked_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    checked_query_impl(|| {
        assert_query_aabb_valid(aabb);
        overlap_aabb_impl(raw_world_id, aabb, filter)
    })
}

pub(super) fn visit_overlap_aabb_checked_impl<F>(
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

pub(super) fn overlap_aabb_into_checked_impl(
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

pub(super) fn try_overlap_aabb_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>> {
    try_checked_query_result_impl(|| {
        check_query_aabb_valid(aabb)?;
        Ok(overlap_aabb_impl(raw_world_id, aabb, filter))
    })
}

pub(super) fn try_visit_overlap_aabb_impl<F>(
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

pub(super) fn try_overlap_aabb_into_impl(
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

pub(super) fn cast_ray_closest_checked_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn try_cast_ray_closest_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn cast_ray_all_checked_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn cast_ray_all_into_checked_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn try_cast_ray_all_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn try_cast_ray_all_into_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn overlap_polygon_points_checked_impl<I, P>(
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

pub(super) fn visit_overlap_polygon_points_checked_impl<I, P, F>(
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

pub(super) fn overlap_polygon_points_into_checked_impl<I, P>(
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

pub(super) fn try_overlap_polygon_points_impl<I, P>(
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

pub(super) fn try_visit_overlap_polygon_points_impl<I, P, F>(
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

pub(super) fn try_overlap_polygon_points_into_impl<I, P>(
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

pub(super) fn cast_shape_points_checked_impl<I, P, VT>(
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

pub(super) fn cast_shape_points_into_checked_impl<I, P, VT>(
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

pub(super) fn try_cast_shape_points_impl<I, P, VT>(
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

pub(super) fn try_cast_shape_points_into_impl<I, P, VT>(
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

pub(super) fn cast_mover_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn try_cast_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
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

pub(super) fn collide_mover_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(super) fn collide_mover_into_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(super) fn try_collide_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(super) fn try_collide_mover_into_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(super) fn overlap_polygon_points_with_offset_checked_impl<I, P, V, A>(
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

pub(super) fn visit_overlap_polygon_points_with_offset_checked_impl<I, P, V, A, F>(
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

pub(super) fn overlap_polygon_points_with_offset_into_checked_impl<I, P, V, A>(
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

pub(super) fn try_overlap_polygon_points_with_offset_impl<I, P, V, A>(
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

pub(super) fn try_visit_overlap_polygon_points_with_offset_impl<I, P, V, A, F>(
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

pub(super) fn try_overlap_polygon_points_with_offset_into_impl<I, P, V, A>(
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

pub(super) fn cast_shape_points_with_offset_checked_impl<I, P, V, A, VT>(
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

pub(super) fn cast_shape_points_with_offset_into_checked_impl<I, P, V, A, VT>(
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

pub(super) fn try_cast_shape_points_with_offset_impl<I, P, V, A, VT>(
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

pub(super) fn try_cast_shape_points_with_offset_into_impl<I, P, V, A, VT>(
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
