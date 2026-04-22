use super::*;

pub(crate) fn cast_mover_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
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

pub(crate) fn try_cast_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
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

pub(crate) fn collide_mover_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(crate) fn collide_mover_into_checked_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(crate) fn try_collide_mover_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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

pub(crate) fn try_collide_mover_into_impl<V1: Into<Vec2>, V2: Into<Vec2>>(
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
