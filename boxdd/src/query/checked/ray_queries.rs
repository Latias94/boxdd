use super::*;

pub(crate) fn cast_ray_closest_with_stats_checked_impl<VT: Into<Vec2>>(
    target: QueryTarget,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> ClosestRayCastResult {
    checked_query_preflight(&target);
    let translation = translation.into();
    assert_query_position_valid("origin", origin);
    assert_query_vec2_valid("translation", translation);
    checked_query_impl(&target, || {
        crate::query::raw::cast_ray_closest_with_stats_impl(&target, origin, translation, filter)
    })
    .expect("Box2D returned an invalid closest-ray shape id")
}

pub(crate) fn try_cast_ray_closest_with_stats_impl<VT: Into<Vec2>>(
    target: QueryTarget,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<ClosestRayCastResult> {
    try_checked_query_preflight(&target)?;
    let translation = translation.into();
    check_query_position_valid(origin)?;
    check_query_vec2_valid(translation)?;
    try_checked_query_result_impl(&target, || {
        crate::query::raw::cast_ray_closest_with_stats_impl(&target, origin, translation, filter)
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn cast_ray_all_checked_impl<VT: Into<Vec2>>(
    target: QueryTarget,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> Vec<RayResult> {
    checked_query_preflight(&target);
    let translation = translation.into();
    assert_query_position_valid("origin", origin);
    assert_query_vec2_valid("translation", translation);
    checked_query_impl(&target, || {
        crate::query::raw::cast_ray_all_impl(&target, origin, translation, filter)
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn cast_ray_all_into_checked_impl<VT: Into<Vec2>>(
    target: QueryTarget,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) {
    checked_query_preflight(&target);
    let translation = translation.into();
    assert_query_position_valid("origin", origin);
    assert_query_vec2_valid("translation", translation);
    checked_query_impl(&target, || {
        crate::query::raw::cast_ray_all_into_impl(&target, origin, translation, filter, out);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn try_cast_ray_all_impl<VT: Into<Vec2>>(
    target: QueryTarget,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
) -> ApiResult<Vec<RayResult>> {
    try_checked_query_preflight(&target)?;
    let translation = translation.into();
    check_query_position_valid(origin)?;
    check_query_vec2_valid(translation)?;
    try_checked_query_result_impl(&target, || {
        Ok(crate::query::raw::cast_ray_all_impl(
            &target,
            origin,
            translation,
            filter,
        ))
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn try_cast_ray_all_into_impl<VT: Into<Vec2>>(
    target: QueryTarget,
    origin: Position,
    translation: VT,
    filter: QueryFilter,
    out: &mut Vec<RayResult>,
) -> ApiResult<()> {
    try_checked_query_preflight(&target)?;
    let translation = translation.into();
    check_query_position_valid(origin)?;
    check_query_vec2_valid(translation)?;
    try_checked_query_result_impl(&target, || {
        crate::query::raw::cast_ray_all_into_impl(&target, origin, translation, filter, out);
        Ok(())
    })
}
