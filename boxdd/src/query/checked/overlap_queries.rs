#![allow(
    clippy::too_many_arguments,
    reason = "query geometry and its absolute world origin are deliberately explicit"
)]

use super::*;

pub(crate) fn overlap_aabb_checked_impl(
    target: QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    checked_query_preflight(&target);
    assert_query_position_valid("origin", origin);
    assert_query_aabb_valid(aabb);
    checked_query_impl(&target, || overlap_aabb_impl(&target, origin, aabb, filter))
}

pub(crate) fn visit_overlap_aabb_checked_impl<F>(
    target: QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    visit: &mut F,
) -> bool
where
    F: FnMut(ShapeId) -> bool,
{
    checked_query_preflight(&target);
    assert_query_position_valid("origin", origin);
    assert_query_aabb_valid(aabb);
    checked_query_impl(&target, || {
        visit_overlap_aabb_impl(&target, origin, aabb, filter, visit)
    })
}

pub(crate) fn overlap_aabb_into_checked_impl(
    target: QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) {
    checked_query_preflight(&target);
    assert_query_position_valid("origin", origin);
    assert_query_aabb_valid(aabb);
    checked_query_impl(&target, || {
        overlap_aabb_into_impl(&target, origin, aabb, filter, out);
    });
}

pub(crate) fn try_overlap_aabb_impl(
    target: QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>> {
    try_checked_query_preflight(&target)?;
    check_query_position_valid(origin)?;
    check_query_aabb_valid(aabb)?;
    try_checked_query_result_impl(&target, || {
        Ok(overlap_aabb_impl(&target, origin, aabb, filter))
    })
}

pub(crate) fn try_visit_overlap_aabb_impl<F>(
    target: QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    visit: &mut F,
) -> ApiResult<bool>
where
    F: FnMut(ShapeId) -> bool,
{
    try_checked_query_preflight(&target)?;
    check_query_position_valid(origin)?;
    check_query_aabb_valid(aabb)?;
    try_checked_query_result_impl(&target, || {
        Ok(visit_overlap_aabb_impl(
            &target, origin, aabb, filter, visit,
        ))
    })
}

pub(crate) fn try_overlap_aabb_into_impl(
    target: QueryTarget,
    origin: Position,
    aabb: Aabb,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    try_checked_query_preflight(&target)?;
    check_query_position_valid(origin)?;
    check_query_aabb_valid(aabb)?;
    try_checked_query_result_impl(&target, || {
        overlap_aabb_into_impl(&target, origin, aabb, filter, out);
        Ok(())
    })
}

pub(crate) fn overlap_polygon_points_checked_impl<I, P>(
    target: QueryTarget,
    origin: Position,
    points: I,
    radius: f32,
    filter: QueryFilter,
) -> Vec<ShapeId>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    checked_query_preflight(&target);
    assert_query_position_valid("origin", origin);
    assert_query_non_negative_finite_scalar("radius", radius);
    let points = collect_asserted_proxy_points(points);
    checked_query_impl(&target, || {
        overlap_polygon_points_impl(&target, origin, &points, radius, filter)
    })
}

pub(crate) fn visit_overlap_polygon_points_checked_impl<I, P, F>(
    target: QueryTarget,
    origin: Position,
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
    checked_query_preflight(&target);
    assert_query_position_valid("origin", origin);
    assert_query_non_negative_finite_scalar("radius", radius);
    let points = collect_asserted_proxy_points(points);
    checked_query_impl(&target, || {
        visit_overlap_polygon_points_impl(&target, origin, &points, radius, filter, visit)
    })
}

pub(crate) fn overlap_polygon_points_into_checked_impl<I, P>(
    target: QueryTarget,
    origin: Position,
    points: I,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    checked_query_preflight(&target);
    assert_query_position_valid("origin", origin);
    assert_query_non_negative_finite_scalar("radius", radius);
    let points = collect_asserted_proxy_points(points);
    checked_query_impl(&target, || {
        overlap_polygon_points_into_impl(&target, origin, &points, radius, filter, out)
    });
}

pub(crate) fn try_overlap_polygon_points_impl<I, P>(
    target: QueryTarget,
    origin: Position,
    points: I,
    radius: f32,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    try_checked_query_preflight(&target)?;
    check_query_position_valid(origin)?;
    check_query_non_negative_finite_scalar(radius)?;
    let points = try_collect_proxy_points(points)?;
    try_checked_query_result_impl(&target, || {
        Ok(overlap_polygon_points_impl(
            &target, origin, &points, radius, filter,
        ))
    })
}

pub(crate) fn try_visit_overlap_polygon_points_impl<I, P, F>(
    target: QueryTarget,
    origin: Position,
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
    try_checked_query_preflight(&target)?;
    check_query_position_valid(origin)?;
    check_query_non_negative_finite_scalar(radius)?;
    let points = try_collect_proxy_points(points)?;
    try_checked_query_result_impl(&target, || {
        Ok(visit_overlap_polygon_points_impl(
            &target, origin, &points, radius, filter, visit,
        ))
    })
}

pub(crate) fn try_overlap_polygon_points_into_impl<I, P>(
    target: QueryTarget,
    origin: Position,
    points: I,
    radius: f32,
    filter: QueryFilter,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()>
where
    I: IntoIterator<Item = P>,
    P: Into<Vec2>,
{
    try_checked_query_preflight(&target)?;
    check_query_position_valid(origin)?;
    check_query_non_negative_finite_scalar(radius)?;
    let points = try_collect_proxy_points(points)?;
    try_checked_query_result_impl(&target, || {
        overlap_polygon_points_into_impl(&target, origin, &points, radius, filter, out);
        Ok(())
    })
}

pub(crate) fn overlap_polygon_points_with_offset_checked_impl<I, P, V, A>(
    target: QueryTarget,
    origin: Position,
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
    checked_query_preflight(&target);
    let position = position.into();
    let angle_radians = angle_radians.into();
    assert_query_position_valid("origin", origin);
    assert_query_non_negative_finite_scalar("radius", radius);
    assert_query_vec2_valid("position", position);
    assert_query_angle_valid(angle_radians);
    let points = collect_asserted_proxy_points(points);
    checked_query_impl(&target, || {
        overlap_polygon_points_with_offset_impl(
            &target,
            origin,
            &points,
            radius,
            position,
            angle_radians,
            filter,
        )
    })
}

pub(crate) fn visit_overlap_polygon_points_with_offset_checked_impl<I, P, V, A, F>(
    target: QueryTarget,
    origin: Position,
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
    checked_query_preflight(&target);
    let position = position.into();
    let angle_radians = angle_radians.into();
    assert_query_position_valid("origin", origin);
    assert_query_non_negative_finite_scalar("radius", radius);
    assert_query_vec2_valid("position", position);
    assert_query_angle_valid(angle_radians);
    let points = collect_asserted_proxy_points(points);
    checked_query_impl(&target, || {
        visit_overlap_polygon_points_with_offset_impl(
            &target,
            origin,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            visit,
        )
    })
}

pub(crate) fn overlap_polygon_points_with_offset_into_checked_impl<I, P, V, A>(
    target: QueryTarget,
    origin: Position,
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
    checked_query_preflight(&target);
    let position = position.into();
    let angle_radians = angle_radians.into();
    assert_query_position_valid("origin", origin);
    assert_query_non_negative_finite_scalar("radius", radius);
    assert_query_vec2_valid("position", position);
    assert_query_angle_valid(angle_radians);
    let points = collect_asserted_proxy_points(points);
    checked_query_impl(&target, || {
        overlap_polygon_points_with_offset_into_impl(
            &target,
            origin,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        )
    });
}

pub(crate) fn try_overlap_polygon_points_with_offset_impl<I, P, V, A>(
    target: QueryTarget,
    origin: Position,
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
    try_checked_query_preflight(&target)?;
    let position = position.into();
    let angle_radians = angle_radians.into();
    check_query_position_valid(origin)?;
    check_query_non_negative_finite_scalar(radius)?;
    check_query_vec2_valid(position)?;
    check_query_angle_valid(angle_radians)?;
    let points = try_collect_proxy_points(points)?;
    try_checked_query_result_impl(&target, || {
        Ok(overlap_polygon_points_with_offset_impl(
            &target,
            origin,
            &points,
            radius,
            position,
            angle_radians,
            filter,
        ))
    })
}

pub(crate) fn try_visit_overlap_polygon_points_with_offset_impl<I, P, V, A, F>(
    target: QueryTarget,
    origin: Position,
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
    try_checked_query_preflight(&target)?;
    let position = position.into();
    let angle_radians = angle_radians.into();
    check_query_position_valid(origin)?;
    check_query_non_negative_finite_scalar(radius)?;
    check_query_vec2_valid(position)?;
    check_query_angle_valid(angle_radians)?;
    let points = try_collect_proxy_points(points)?;
    try_checked_query_result_impl(&target, || {
        Ok(visit_overlap_polygon_points_with_offset_impl(
            &target,
            origin,
            &points,
            radius,
            position,
            angle_radians,
            filter,
            visit,
        ))
    })
}

pub(crate) fn try_overlap_polygon_points_with_offset_into_impl<I, P, V, A>(
    target: QueryTarget,
    origin: Position,
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
    try_checked_query_preflight(&target)?;
    let position = position.into();
    let angle_radians = angle_radians.into();
    check_query_position_valid(origin)?;
    check_query_non_negative_finite_scalar(radius)?;
    check_query_vec2_valid(position)?;
    check_query_angle_valid(angle_radians)?;
    let points = try_collect_proxy_points(points)?;
    try_checked_query_result_impl(&target, || {
        overlap_polygon_points_with_offset_into_impl(
            &target,
            origin,
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
