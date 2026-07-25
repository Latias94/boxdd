use super::*;
use crate::types::{Position, WorldCastOutput};

#[inline]
pub(crate) fn raw_shape_id(id: ShapeId) -> ffi::b2ShapeId {
    id.into_raw()
}

#[inline]
pub(crate) fn shape_world_id_impl(id: ShapeId) -> ffi::b2WorldId {
    unsafe { ffi::b2Shape_GetWorld(raw_shape_id(id)) }
}

#[inline]
fn try_bind_shape_parent_chain_output(
    core: &crate::core::world_core::WorldCore,
    raw: ffi::b2ChainId,
) -> ApiResult<Option<ChainId>> {
    if raw.index1 == 0 {
        return Ok(None);
    }
    let id = crate::core::world_core::WorldCore::brand(core).try_chain(raw)?;
    core.check_chain(id)?;
    Ok(Some(id))
}

#[inline]
pub(crate) fn shape_parent_chain_id_in_impl(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<Option<ChainId>> {
    let raw = unsafe { ffi::b2Shape_GetParentChain(raw_shape_id(id)) };
    try_bind_shape_parent_chain_output(core, raw)
}

#[inline]
pub(crate) fn shape_type_raw_impl(id: ShapeId) -> ffi::b2ShapeType {
    #[cfg(test)]
    {
        SHAPE_GET_TYPE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        if let Some(raw) = SHAPE_GET_TYPE_OVERRIDE.with(core::cell::Cell::get) {
            return raw;
        }
    }
    unsafe { ffi::b2Shape_GetType(raw_shape_id(id)) }
}

#[cfg(test)]
thread_local! {
    static SHAPE_GET_TYPE_OVERRIDE: core::cell::Cell<Option<ffi::b2ShapeType>> = const {
        core::cell::Cell::new(None)
    };
    static SHAPE_GET_TYPE_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

#[inline]
pub(crate) fn resolve_shape_type_output(
    core: &crate::core::world_core::WorldCore,
    raw: ffi::b2ShapeType,
) -> ApiResult<ShapeType> {
    ShapeType::decode_native(raw).inspect_err(|_| core.poison())
}

#[inline]
pub(crate) fn try_shape_type_impl(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<ShapeType> {
    resolve_shape_type_output(core, shape_type_raw_impl(id))
}

#[inline]
fn try_bind_shape_body_output(
    core: &crate::core::world_core::WorldCore,
    raw: ffi::b2BodyId,
) -> ApiResult<BodyId> {
    let id = core.brand().try_body(raw)?;
    core.check_body(id)?;
    Ok(id)
}

#[inline]
pub(crate) fn shape_body_id_in_impl(
    core: &crate::core::world_core::WorldCore,
    id: ShapeId,
) -> ApiResult<BodyId> {
    try_bind_shape_body_output(core, unsafe { ffi::b2Shape_GetBody(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_body_id_impl(id: ShapeId) -> BodyId {
    let raw = unsafe { ffi::b2Shape_GetBody(raw_shape_id(id)) };
    let body = id
        .brand()
        .try_body(raw)
        .expect("Box2D returned an invalid body id for a validated shape");
    assert!(
        crate::core::identity_registry::body_is_active(body) && unsafe { ffi::b2Body_IsValid(raw) },
        "Box2D returned a non-live body id for a validated shape"
    );
    body
}

#[inline]
pub(crate) fn shape_circle_impl(id: ShapeId) -> Circle {
    Circle::from_raw(unsafe { ffi::b2Shape_GetCircle(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_segment_impl(id: ShapeId) -> Segment {
    Segment::from_raw(unsafe { ffi::b2Shape_GetSegment(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_chain_segment_impl(id: ShapeId) -> ChainSegment {
    ChainSegment::from_raw(unsafe { ffi::b2Shape_GetChainSegment(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_capsule_impl(id: ShapeId) -> Capsule {
    Capsule::from_raw(unsafe { ffi::b2Shape_GetCapsule(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_polygon_impl(id: ShapeId) -> Polygon {
    // SAFETY: the checked runtime handle refers to a live native polygon shape, and Box2D returns
    // its complete canonical polygon value.
    unsafe { Polygon::from_raw(ffi::b2Shape_GetPolygon(raw_shape_id(id))) }
}

#[inline]
pub(crate) fn shape_closest_point_impl<P: Into<Position>>(id: ShapeId, target: P) -> Position {
    let target: ffi::b2Pos = target.into().into_raw();
    Position::from_raw(unsafe { ffi::b2Shape_GetClosestPoint(raw_shape_id(id), target) })
}

#[inline]
pub(crate) fn shape_aabb_impl(id: ShapeId) -> Aabb {
    Aabb::from_raw(unsafe { ffi::b2Shape_GetAABB(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_test_point_impl<P: Into<Position>>(id: ShapeId, point: P) -> bool {
    let point: ffi::b2Pos = point.into().into_raw();
    unsafe { ffi::b2Shape_TestPoint(raw_shape_id(id), point) }
}

#[inline]
pub(crate) fn shape_ray_cast_impl(
    id: ShapeId,
    origin: Position,
    translation: Vec2,
) -> WorldCastOutput {
    let origin = origin.into_raw();
    let translation = translation.into_raw();
    WorldCastOutput::from_raw(unsafe {
        ffi::b2Shape_RayCast(raw_shape_id(id), origin, translation)
    })
}

#[inline]
pub(crate) fn shape_apply_wind_impl(id: ShapeId, wind: Vec2, drag: f32, lift: f32, wake: bool) {
    let wind: ffi::b2Vec2 = wind.into_raw();
    unsafe { ffi::b2Shape_ApplyWind(raw_shape_id(id), wind, drag, lift, wake) }
}

#[inline]
pub(crate) fn shape_set_circle_impl(id: ShapeId, circle: &Circle) {
    let raw = circle.into_raw();
    unsafe { ffi::b2Shape_SetCircle(raw_shape_id(id), &raw) }
}

#[inline]
pub(crate) fn shape_set_segment_impl(id: ShapeId, segment: &Segment) {
    let raw = segment.into_raw();
    unsafe { ffi::b2Shape_SetSegment(raw_shape_id(id), &raw) }
}

#[inline]
pub(crate) fn shape_set_capsule_impl(id: ShapeId, capsule: &Capsule) {
    let raw = capsule.into_raw();
    unsafe { ffi::b2Shape_SetCapsule(raw_shape_id(id), &raw) }
}

#[inline]
pub(crate) fn shape_set_polygon_impl(id: ShapeId, polygon: &Polygon) {
    let raw = polygon.into_raw();
    unsafe { ffi::b2Shape_SetPolygon(raw_shape_id(id), &raw) }
}

#[inline]
pub(crate) fn shape_filter_impl(id: ShapeId) -> Filter {
    Filter::from_raw(unsafe { ffi::b2Shape_GetFilter(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_set_filter_impl(id: ShapeId, filter: Filter) {
    unsafe { ffi::b2Shape_SetFilter(raw_shape_id(id), filter.into_raw()) }
}

#[inline]
pub(crate) fn shape_is_sensor_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_IsSensor(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_mass_data_impl(id: ShapeId) -> MassData {
    MassData::from_raw(unsafe { ffi::b2Shape_ComputeMassData(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_enable_sensor_events_impl(id: ShapeId, flag: bool) {
    unsafe { ffi::b2Shape_EnableSensorEvents(raw_shape_id(id), flag) }
}

#[inline]
pub(crate) fn shape_sensor_events_enabled_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_AreSensorEventsEnabled(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_enable_contact_events_impl(id: ShapeId, flag: bool) {
    unsafe { ffi::b2Shape_EnableContactEvents(raw_shape_id(id), flag) }
}

#[inline]
pub(crate) fn shape_contact_events_enabled_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_AreContactEventsEnabled(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_enable_pre_solve_events_impl(id: ShapeId, flag: bool) {
    unsafe { ffi::b2Shape_EnablePreSolveEvents(raw_shape_id(id), flag) }
}

#[inline]
pub(crate) fn shape_pre_solve_events_enabled_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_ArePreSolveEventsEnabled(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_enable_hit_events_impl(id: ShapeId, flag: bool) {
    unsafe { ffi::b2Shape_EnableHitEvents(raw_shape_id(id), flag) }
}

#[inline]
pub(crate) fn shape_hit_events_enabled_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_AreHitEventsEnabled(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_set_density_impl(id: ShapeId, density: f32, update_body_mass: bool) {
    unsafe { ffi::b2Shape_SetDensity(raw_shape_id(id), density, update_body_mass) }
}

#[inline]
pub(crate) fn shape_density_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetDensity(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_set_friction_impl(id: ShapeId, friction: f32) {
    unsafe { ffi::b2Shape_SetFriction(raw_shape_id(id), friction) }
}

#[inline]
pub(crate) fn shape_friction_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetFriction(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_set_restitution_impl(id: ShapeId, restitution: f32) {
    unsafe { ffi::b2Shape_SetRestitution(raw_shape_id(id), restitution) }
}

#[inline]
pub(crate) fn shape_restitution_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetRestitution(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_set_user_material_impl(id: ShapeId, material: u64) {
    unsafe { ffi::b2Shape_SetUserMaterial(raw_shape_id(id), material) }
}

#[inline]
pub(crate) fn shape_user_material_impl(id: ShapeId) -> u64 {
    unsafe { ffi::b2Shape_GetUserMaterial(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_set_surface_material_impl(id: ShapeId, material: &SurfaceMaterial) {
    unsafe { ffi::b2Shape_SetSurfaceMaterial(raw_shape_id(id), &material.0) }
}

#[inline]
pub(crate) fn shape_surface_material_impl(id: ShapeId) -> SurfaceMaterial {
    SurfaceMaterial::from_raw(unsafe { ffi::b2Shape_GetSurfaceMaterial(raw_shape_id(id)) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct ShapeGetTypeOverride;

    impl ShapeGetTypeOverride {
        fn install(raw: ffi::b2ShapeType) -> Self {
            SHAPE_GET_TYPE_OVERRIDE.with(|current| {
                assert_eq!(current.replace(Some(raw)), None);
            });
            SHAPE_GET_TYPE_CALLS.with(|calls| calls.set(0));
            Self
        }

        fn calls(&self) -> usize {
            SHAPE_GET_TYPE_CALLS.with(core::cell::Cell::get)
        }
    }

    impl Drop for ShapeGetTypeOverride {
        fn drop(&mut self) {
            SHAPE_GET_TYPE_OVERRIDE.with(|current| current.set(None));
            SHAPE_GET_TYPE_CALLS.with(|calls| calls.set(0));
        }
    }

    #[test]
    fn shape_type_native_decoder_preserves_known_values_and_reports_the_raw_unknown() {
        for expected in [
            ShapeType::Circle,
            ShapeType::Capsule,
            ShapeType::Segment,
            ShapeType::Polygon,
            ShapeType::ChainSegment,
        ] {
            assert_eq!(ShapeType::decode_native(expected.into_raw()), Ok(expected));
        }

        let raw = ffi::b2ShapeType_b2_shapeTypeCount;
        assert_eq!(
            ShapeType::decode_native(raw),
            Err(ApiError::InvalidNativeShapeType { raw })
        );
    }

    #[test]
    fn all_public_shape_type_getters_report_unknown_once_then_stop_before_get_type() {
        let raw = ffi::b2ShapeType_b2_shapeTypeCount;

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let body = world.create_body_id(crate::BodyBuilder::new().build());
            let shape = world.create_circle_shape_for_owned(
                body,
                &ShapeDef::default(),
                &crate::shapes::circle(crate::Vec2::ZERO, 0.5),
            );
            let get_type = ShapeGetTypeOverride::install(raw);

            assert_eq!(
                shape.try_shape_type(),
                Err(ApiError::InvalidNativeShapeType { raw })
            );
            assert_eq!(get_type.calls(), 1);
            assert_eq!(shape.try_shape_type(), Err(ApiError::WorldPoisoned));
            assert_eq!(shape.try_shape_type_raw(), Err(ApiError::WorldPoisoned));
            assert_eq!(get_type.calls(), 1);
        }

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let body = world.create_body_id(crate::BodyBuilder::new().build());
            let shape = world.create_circle_shape_for(
                body,
                &ShapeDef::default(),
                &crate::shapes::circle(crate::Vec2::ZERO, 0.5),
            );
            let shape = world.shape(shape).unwrap();
            let get_type = ShapeGetTypeOverride::install(raw);

            assert_eq!(
                shape.try_shape_type(),
                Err(ApiError::InvalidNativeShapeType { raw })
            );
            assert_eq!(get_type.calls(), 1);
            assert_eq!(shape.try_shape_type(), Err(ApiError::WorldPoisoned));
            assert_eq!(shape.try_shape_type_raw(), Err(ApiError::WorldPoisoned));
            assert_eq!(get_type.calls(), 1);
        }
    }

    #[test]
    fn infallible_shape_type_poisoning_precedes_its_unknown_native_panic() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let body = world.create_body_id(crate::BodyBuilder::new().build());
        let shape = world.create_circle_shape_for_owned(
            body,
            &ShapeDef::default(),
            &crate::shapes::circle(crate::Vec2::ZERO, 0.5),
        );
        let raw = ffi::b2ShapeType_b2_shapeTypeCount;
        let get_type = ShapeGetTypeOverride::install(raw);

        assert!(catch_unwind(AssertUnwindSafe(|| shape.shape_type())).is_err());
        assert_eq!(get_type.calls(), 1);
        assert_eq!(shape.try_shape_type(), Err(ApiError::WorldPoisoned));
        assert_eq!(get_type.calls(), 1);
    }

    #[test]
    fn shape_world_queries_match_box2d_32_signatures() {
        type PointQuery = unsafe extern "C" fn(ffi::b2ShapeId, ffi::b2Pos) -> bool;
        type ClosestPoint = unsafe extern "C" fn(ffi::b2ShapeId, ffi::b2Pos) -> ffi::b2Pos;
        type RayCast =
            unsafe extern "C" fn(ffi::b2ShapeId, ffi::b2Pos, ffi::b2Vec2) -> ffi::b2WorldCastOutput;

        let _: PointQuery = ffi::b2Shape_TestPoint;
        let _: ClosestPoint = ffi::b2Shape_GetClosestPoint;
        let _: RayCast = ffi::b2Shape_RayCast;
    }

    #[test]
    fn shape_body_output_rejects_null_and_foreign_native_ids() {
        let world = crate::World::new(crate::WorldDef::default()).unwrap();
        let world0 = world.core().brand().world0();

        assert_eq!(
            try_bind_shape_body_output(
                world.core(),
                ffi::b2BodyId {
                    index1: 0,
                    world0,
                    generation: 0,
                },
            )
            .unwrap_err(),
            ApiError::InvalidBodyId
        );
        assert_eq!(
            try_bind_shape_body_output(
                world.core(),
                ffi::b2BodyId {
                    index1: 1,
                    world0: world0.wrapping_add(1),
                    generation: 0,
                },
            )
            .unwrap_err(),
            ApiError::WrongWorld
        );
    }

    #[test]
    fn shape_parent_chain_output_distinguishes_absent_and_foreign_ids() {
        let world = crate::World::new(crate::WorldDef::default()).unwrap();
        let world0 = world.core().brand().world0();

        assert_eq!(
            try_bind_shape_parent_chain_output(
                world.core(),
                ffi::b2ChainId {
                    index1: 0,
                    world0: 0,
                    generation: 0,
                },
            )
            .unwrap(),
            None
        );
        assert_eq!(
            try_bind_shape_parent_chain_output(
                world.core(),
                ffi::b2ChainId {
                    index1: 1,
                    world0: world0.wrapping_add(1),
                    generation: 0,
                },
            )
            .unwrap_err(),
            ApiError::WrongWorld
        );
    }
}
