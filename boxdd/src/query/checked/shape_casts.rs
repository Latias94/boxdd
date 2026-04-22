use super::*;

pub(crate) fn cast_shape_points_checked_impl<I, P, VT>(
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

pub(crate) fn cast_shape_points_into_checked_impl<I, P, VT>(
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

pub(crate) fn try_cast_shape_points_impl<I, P, VT>(
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

pub(crate) fn try_cast_shape_points_into_impl<I, P, VT>(
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

pub(crate) fn cast_shape_points_with_offset_checked_impl<I, P, V, A, VT>(
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_shape_points_with_offset_into_checked_impl<I, P, V, A, VT>(
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

pub(crate) fn try_cast_shape_points_with_offset_impl<I, P, V, A, VT>(
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_cast_shape_points_with_offset_into_impl<I, P, V, A, VT>(
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
