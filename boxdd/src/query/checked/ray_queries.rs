use super::*;

pub(crate) fn cast_ray_closest_checked_impl<VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> RayResult {
    checked_query_impl(|| {
        let translation = translation.into();
        assert_query_position_valid("origin", origin);
        assert_query_vec2_valid("translation", translation);
        cast_ray_closest_impl(raw_world_id, origin, translation, filter)
    })
}

pub(crate) fn try_cast_ray_closest_impl<VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<RayResult> {
    try_checked_query_result_impl(|| {
        let translation = translation.into();
        check_query_position_valid(origin)?;
        check_query_vec2_valid(translation)?;
        Ok(cast_ray_closest_impl(
            raw_world_id,
            origin,
            translation,
            filter,
        ))
    })
}

pub(crate) fn cast_ray_all_checked_impl<VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult> {
    checked_query_impl(|| {
        let translation = translation.into();
        assert_query_position_valid("origin", origin);
        assert_query_vec2_valid("translation", translation);
        cast_ray_all_impl(raw_world_id, origin, translation, filter)
    })
}

pub(crate) fn cast_ray_all_into_checked_impl<VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    checked_query_impl(|| {
        let translation = translation.into();
        assert_query_position_valid("origin", origin);
        assert_query_vec2_valid("translation", translation);
        cast_ray_all_into_impl(raw_world_id, origin, translation, filter, out);
    });
}

pub(crate) fn try_cast_ray_all_impl<VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<Vec<RayResult>> {
    try_checked_query_result_impl(|| {
        let translation = translation.into();
        check_query_position_valid(origin)?;
        check_query_vec2_valid(translation)?;
        Ok(cast_ray_all_impl(raw_world_id, origin, translation, filter))
    })
}

pub(crate) fn try_cast_ray_all_into_impl<VT: Into<Vec2>>(
    raw_world_id: ffi::b2WorldId,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) -> ApiResult<()> {
    try_checked_query_result_impl(|| {
        let translation = translation.into();
        check_query_position_valid(origin)?;
        check_query_vec2_valid(translation)?;
        cast_ray_all_into_impl(raw_world_id, origin, translation, filter, out);
        Ok(())
    })
}
