use super::*;

pub(crate) fn overlap_aabb_checked_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> Vec<ShapeId> {
    checked_query_impl(|| {
        assert_query_aabb_valid(aabb);
        overlap_aabb_impl(raw_world_id, aabb, filter)
    })
}

pub(crate) fn visit_overlap_aabb_checked_impl<F>(
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

pub(crate) fn overlap_aabb_into_checked_impl(
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

pub(crate) fn try_overlap_aabb_impl(
    raw_world_id: ffi::b2WorldId,
    aabb: Aabb,
    filter: QueryFilter,
) -> ApiResult<Vec<ShapeId>> {
    try_checked_query_result_impl(|| {
        check_query_aabb_valid(aabb)?;
        Ok(overlap_aabb_impl(raw_world_id, aabb, filter))
    })
}

pub(crate) fn try_visit_overlap_aabb_impl<F>(
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

pub(crate) fn try_overlap_aabb_into_impl(
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

pub(crate) fn overlap_polygon_points_checked_impl<I, P>(
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

pub(crate) fn visit_overlap_polygon_points_checked_impl<I, P, F>(
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

pub(crate) fn overlap_polygon_points_into_checked_impl<I, P>(
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

pub(crate) fn try_overlap_polygon_points_impl<I, P>(
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

pub(crate) fn try_visit_overlap_polygon_points_impl<I, P, F>(
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

pub(crate) fn try_overlap_polygon_points_into_impl<I, P>(
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

pub(crate) fn overlap_polygon_points_with_offset_checked_impl<I, P, V, A>(
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

pub(crate) fn visit_overlap_polygon_points_with_offset_checked_impl<I, P, V, A, F>(
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

pub(crate) fn overlap_polygon_points_with_offset_into_checked_impl<I, P, V, A>(
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

pub(crate) fn try_overlap_polygon_points_with_offset_impl<I, P, V, A>(
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

pub(crate) fn try_visit_overlap_polygon_points_with_offset_impl<I, P, V, A, F>(
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

pub(crate) fn try_overlap_polygon_points_with_offset_into_impl<I, P, V, A>(
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
