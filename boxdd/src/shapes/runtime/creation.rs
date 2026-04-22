use super::*;

#[inline]
pub(crate) fn record_shape_flags_on_create(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
    def: &ShapeDef,
) {
    #[cfg(feature = "serialize")]
    core.record_shape_flags(id, &def.0);
    #[cfg(not(feature = "serialize"))]
    let _ = (core, id, def);
}

pub(crate) fn create_body_attached_shape_id_impl<G, R>(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    geometry: &G,
    assert_geometry_valid: impl FnOnce(&G),
    into_raw: impl FnOnce(&G) -> R,
    create_raw: impl FnOnce(ffi::b2BodyId, &ffi::b2ShapeDef, &R) -> ffi::b2ShapeId,
) -> ShapeId {
    crate::core::debug_checks::assert_body_valid(body);
    assert_shape_def_valid(def);
    assert_geometry_valid(geometry);
    let raw = into_raw(geometry);
    let id = ShapeId::from_raw(create_raw(body.into_raw(), &def.0, &raw));
    record_shape_flags_on_create(core, id, def);
    id
}

pub(crate) fn try_create_body_attached_shape_id_impl<G, R>(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    geometry: &G,
    check_geometry_valid: impl FnOnce(&G) -> ApiResult<()>,
    into_raw: impl FnOnce(&G) -> R,
    create_raw: impl FnOnce(ffi::b2BodyId, &ffi::b2ShapeDef, &R) -> ffi::b2ShapeId,
) -> ApiResult<ShapeId> {
    crate::core::debug_checks::check_body_valid(body)?;
    check_shape_def_valid(def)?;
    check_geometry_valid(geometry)?;
    let raw = into_raw(geometry);
    let id = ShapeId::from_raw(create_raw(body.into_raw(), &def.0, &raw));
    record_shape_flags_on_create(core, id, def);
    Ok(id)
}

pub(crate) fn create_body_attached_shape_handle<T, G>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    geometry: &G,
    create: impl FnOnce(&crate::core::world_core::WorldCore, BodyId, &ShapeDef, &G) -> ShapeId,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> T {
    let id = create(core.as_ref(), body, def, geometry);
    wrap(Arc::clone(core), id)
}

pub(crate) fn try_create_body_attached_shape_handle<T, G>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    geometry: &G,
    create: impl FnOnce(
        &crate::core::world_core::WorldCore,
        BodyId,
        &ShapeDef,
        &G,
    ) -> ApiResult<ShapeId>,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T> {
    let id = create(core.as_ref(), body, def, geometry)?;
    Ok(wrap(Arc::clone(core), id))
}

pub(crate) fn create_body_attached_box_shape_handle<T>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    half_w: f32,
    half_h: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> T {
    let polygon = box_polygon(half_w, half_h);
    create_body_attached_shape_handle(
        core,
        body,
        def,
        &polygon,
        create_polygon_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn try_create_body_attached_box_shape_handle<T>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    half_w: f32,
    half_h: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T> {
    let polygon = crate::shapes::try_box_polygon(half_w, half_h)?;
    try_create_body_attached_shape_handle(
        core,
        body,
        def,
        &polygon,
        try_create_polygon_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn create_body_attached_circle_simple_shape_handle<T>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    radius: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> T {
    let circle = circle([0.0_f32, 0.0], radius);
    create_body_attached_shape_handle(
        core,
        body,
        def,
        &circle,
        create_circle_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn try_create_body_attached_circle_simple_shape_handle<T>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    radius: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T> {
    let circle = circle([0.0_f32, 0.0], radius);
    try_create_body_attached_shape_handle(
        core,
        body,
        def,
        &circle,
        try_create_circle_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn create_body_attached_segment_simple_shape_handle<T, V: Into<crate::types::Vec2>>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    p1: V,
    p2: V,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> T {
    let segment = segment(p1, p2);
    create_body_attached_shape_handle(
        core,
        body,
        def,
        &segment,
        create_segment_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn try_create_body_attached_segment_simple_shape_handle<
    T,
    V: Into<crate::types::Vec2>,
>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    p1: V,
    p2: V,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T> {
    let segment = segment(p1, p2);
    try_create_body_attached_shape_handle(
        core,
        body,
        def,
        &segment,
        try_create_segment_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn create_body_attached_capsule_simple_shape_handle<T, V: Into<crate::types::Vec2>>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    c1: V,
    c2: V,
    radius: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> T {
    let capsule = capsule(c1, c2, radius);
    create_body_attached_shape_handle(
        core,
        body,
        def,
        &capsule,
        create_capsule_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn try_create_body_attached_capsule_simple_shape_handle<
    T,
    V: Into<crate::types::Vec2>,
>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    c1: V,
    c2: V,
    radius: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T> {
    let capsule = capsule(c1, c2, radius);
    try_create_body_attached_shape_handle(
        core,
        body,
        def,
        &capsule,
        try_create_capsule_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn create_body_attached_polygon_from_points_shape_handle<T, I, P>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    points: I,
    radius: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> Option<T>
where
    I: IntoIterator<Item = P>,
    P: Into<crate::types::Vec2>,
{
    let polygon = crate::shapes::polygon_from_points(points, radius)?;
    Some(create_body_attached_shape_handle(
        core,
        body,
        def,
        &polygon,
        create_polygon_shape_for_body_impl,
        wrap,
    ))
}

pub(crate) fn try_create_body_attached_polygon_from_points_shape_handle<T, I, P>(
    core: &Arc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    points: I,
    radius: f32,
    wrap: impl FnOnce(Arc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T>
where
    I: IntoIterator<Item = P>,
    P: Into<crate::types::Vec2>,
{
    let polygon = crate::shapes::try_polygon_from_points(points, radius)?;
    try_create_body_attached_shape_handle(
        core,
        body,
        def,
        &polygon,
        try_create_polygon_shape_for_body_impl,
        wrap,
    )
}

pub(crate) fn create_segment_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    segment: &Segment,
) -> ShapeId {
    create_body_attached_shape_id_impl(
        core,
        body,
        def,
        segment,
        assert_segment_geometry_valid,
        |segment| segment.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateSegmentShape(body, def, raw) },
    )
}

pub(crate) fn try_create_segment_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    segment: &Segment,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_impl(
        core,
        body,
        def,
        segment,
        check_segment_geometry_valid,
        |segment| segment.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateSegmentShape(body, def, raw) },
    )
}

pub(crate) fn create_capsule_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    capsule: &Capsule,
) -> ShapeId {
    create_body_attached_shape_id_impl(
        core,
        body,
        def,
        capsule,
        assert_capsule_geometry_valid,
        |capsule| capsule.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateCapsuleShape(body, def, raw) },
    )
}

pub(crate) fn try_create_capsule_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    capsule: &Capsule,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_impl(
        core,
        body,
        def,
        capsule,
        check_capsule_geometry_valid,
        |capsule| capsule.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateCapsuleShape(body, def, raw) },
    )
}

pub(crate) fn create_polygon_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    polygon: &Polygon,
) -> ShapeId {
    create_body_attached_shape_id_impl(
        core,
        body,
        def,
        polygon,
        assert_polygon_geometry_valid,
        |polygon| polygon.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreatePolygonShape(body, def, raw) },
    )
}

pub(crate) fn try_create_polygon_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    polygon: &Polygon,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_impl(
        core,
        body,
        def,
        polygon,
        check_polygon_geometry_valid,
        |polygon| polygon.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreatePolygonShape(body, def, raw) },
    )
}

pub(crate) fn create_circle_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    circle: &Circle,
) -> ShapeId {
    create_body_attached_shape_id_impl(
        core,
        body,
        def,
        circle,
        assert_circle_geometry_valid,
        |circle| circle.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateCircleShape(body, def, raw) },
    )
}

pub(crate) fn try_create_circle_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    circle: &Circle,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_impl(
        core,
        body,
        def,
        circle,
        check_circle_geometry_valid,
        |circle| circle.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateCircleShape(body, def, raw) },
    )
}
