use super::*;

fn finish_shape_creation(
    core: &crate::core::world_core::WorldCore,
    raw: ffi::b2ShapeId,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ShapeId> {
    core.finish_created_shape_with_access(raw, access)
}

#[derive(Copy, Clone)]
struct ShapeCreationTarget<'a> {
    core: &'a crate::core::world_core::WorldCore,
    body: BodyId,
    access: crate::core::world_core::WorldAccess,
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
    crate::core::callback_state::assert_not_in_callback();
    core.check_body(body).expect("invalid or foreign BodyId");
    assert_shape_def_valid(def);
    assert_geometry_valid(geometry);
    let raw = into_raw(geometry);
    let raw_id = create_raw(body.into_raw(), &def.0, &raw);
    finish_shape_creation(core, raw_id, crate::core::world_core::WorldAccess::Idle)
        .expect("Box2D returned an invalid ShapeId")
}

fn try_create_body_attached_shape_id_with_access<G, R>(
    target: ShapeCreationTarget<'_>,
    def: &ShapeDef,
    geometry: &G,
    check_geometry_valid: impl FnOnce(&G) -> ApiResult<()>,
    into_raw: impl FnOnce(&G) -> R,
    create_raw: impl FnOnce(ffi::b2BodyId, &ffi::b2ShapeDef, &R) -> ffi::b2ShapeId,
) -> ApiResult<ShapeId> {
    crate::core::callback_state::check_not_in_callback()?;
    target
        .core
        .check_body_with_access(target.body, target.access)?;
    check_shape_def_valid(def)?;
    check_geometry_valid(geometry)?;
    let raw = into_raw(geometry);
    let raw_id = create_raw(target.body.into_raw(), &def.0, &raw);
    finish_shape_creation(target.core, raw_id, target.access)
}

pub(crate) fn create_body_attached_shape_handle<T, G>(
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    geometry: &G,
    create: impl FnOnce(&crate::core::world_core::WorldCore, BodyId, &ShapeDef, &G) -> ShapeId,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> T {
    let id = create(core.as_ref(), body, def, geometry);
    wrap(Rc::clone(core), id)
}

pub(crate) fn try_create_body_attached_shape_handle<T, G>(
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    geometry: &G,
    create: impl FnOnce(
        &crate::core::world_core::WorldCore,
        BodyId,
        &ShapeDef,
        &G,
    ) -> ApiResult<ShapeId>,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
) -> ApiResult<T> {
    let id = create(core.as_ref(), body, def, geometry)?;
    Ok(wrap(Rc::clone(core), id))
}

pub(crate) fn create_body_attached_box_shape_handle<T>(
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    half_w: f32,
    half_h: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    half_w: f32,
    half_h: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    radius: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    radius: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    p1: V,
    p2: V,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    p1: V,
    p2: V,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    c1: V,
    c2: V,
    radius: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    c1: V,
    c2: V,
    radius: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    points: I,
    radius: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ShapeDef,
    points: I,
    radius: f32,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ShapeId) -> T,
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
    try_create_segment_shape_for_body_with_access(
        core,
        body,
        def,
        segment,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_create_segment_shape_for_body_with_access(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    segment: &Segment,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_with_access(
        ShapeCreationTarget { core, body, access },
        def,
        segment,
        check_segment_geometry_valid,
        |segment| segment.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateSegmentShape(body, def, raw) },
    )
}

pub(crate) fn create_chain_segment_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    chain_segment: &ChainSegment,
) -> ShapeId {
    create_body_attached_shape_id_impl(
        core,
        body,
        def,
        chain_segment,
        assert_chain_segment_geometry_valid,
        |chain_segment| chain_segment.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateChainSegmentShape(body, def, raw) },
    )
}

pub(crate) fn try_create_chain_segment_shape_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    chain_segment: &ChainSegment,
) -> ApiResult<ShapeId> {
    try_create_chain_segment_shape_for_body_with_access(
        core,
        body,
        def,
        chain_segment,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_create_chain_segment_shape_for_body_with_access(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    chain_segment: &ChainSegment,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_with_access(
        ShapeCreationTarget { core, body, access },
        def,
        chain_segment,
        check_chain_segment_geometry_valid,
        |chain_segment| chain_segment.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateChainSegmentShape(body, def, raw) },
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
    try_create_capsule_shape_for_body_with_access(
        core,
        body,
        def,
        capsule,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_create_capsule_shape_for_body_with_access(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    capsule: &Capsule,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_with_access(
        ShapeCreationTarget { core, body, access },
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
    try_create_polygon_shape_for_body_with_access(
        core,
        body,
        def,
        polygon,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_create_polygon_shape_for_body_with_access(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    polygon: &Polygon,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_with_access(
        ShapeCreationTarget { core, body, access },
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
    try_create_circle_shape_for_body_with_access(
        core,
        body,
        def,
        circle,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_create_circle_shape_for_body_with_access(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ShapeDef,
    circle: &Circle,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ShapeId> {
    try_create_body_attached_shape_id_with_access(
        ShapeCreationTarget { core, body, access },
        def,
        circle,
        check_circle_geometry_valid,
        |circle| circle.into_raw(),
        |body, def, raw| unsafe { ffi::b2CreateCircleShape(body, def, raw) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn shape_creation_registers_identity_before_returning() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let body = world.create_body_id(crate::BodyBuilder::new().build());
        let core = world.core();
        let def = ShapeDef::default();
        let shape =
            try_create_circle_shape_for_body_impl(core, body, &def, &circle([0.0_f32, 0.0], 0.5))
                .unwrap();

        assert_eq!(core.check_shape(shape), Ok(()));
        assert_eq!(
            core.finish_created_shape_with_access(
                shape.into_raw(),
                crate::core::world_core::WorldAccess::Idle,
            ),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(core.check_available(), Err(ApiError::WorldPoisoned));
    }

    #[test]
    fn shape_creation_checks_the_target_body_before_ffi() {
        let mut source = World::new(crate::WorldDef::default()).unwrap();
        let foreign_body = source.create_body_id(crate::BodyBuilder::new().build());
        let target = World::new(crate::WorldDef::default()).unwrap();
        let called = Cell::new(false);

        let result = try_create_body_attached_shape_id_with_access(
            ShapeCreationTarget {
                core: target.core(),
                body: foreign_body,
                access: crate::core::world_core::WorldAccess::Idle,
            },
            &ShapeDef::default(),
            &circle([0.0_f32, 0.0], 0.5),
            check_circle_geometry_valid,
            |circle| circle.into_raw(),
            |_, _, _| {
                called.set(true);
                ffi::b2ShapeId {
                    index1: 0,
                    world0: 0,
                    generation: 0,
                }
            },
        );

        assert_eq!(result, Err(ApiError::WrongWorld));
        assert!(!called.get());
    }
}
