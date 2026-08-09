use super::*;
use crate::types::{Position, WorldCastOutput};

#[inline]
pub(crate) fn raw_shape_id(id: ShapeId) -> ffi::b2ShapeId {
    id.into_raw()
}

#[inline]
fn bind_shape_parent_chain_output(
    resolver: &crate::core::identity_registry::OutputIdentityResolver<'_>,
    raw: ffi::b2ChainId,
) -> Result<Option<ChainId>> {
    if raw.index1 == 0 {
        return Ok(None);
    }
    resolver.active_chain(raw).map(Some)
}

#[inline]
pub(crate) fn shape_parent_chain_id_in_impl(
    shape: crate::world::ShapeCall<'_>,
) -> Result<Option<ChainId>> {
    let raw = unsafe { ffi::b2Shape_GetParentChain(raw_shape_id(shape.id())) };
    shape.with_output_identity_resolver(|resolver| bind_shape_parent_chain_output(resolver, raw))
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
pub(crate) fn shape_body_id_in_impl(shape: crate::world::ShapeCall<'_>) -> Result<BodyId> {
    let raw = unsafe { ffi::b2Shape_GetBody(raw_shape_id(shape.id())) };
    shape.with_output_identity_resolver(|resolver| resolver.active_body(raw))
}

#[inline]
pub(crate) fn shape_circle_impl(id: ShapeId) -> Result<Circle> {
    Circle::from_raw(unsafe { ffi::b2Shape_GetCircle(raw_shape_id(id)) }).map_err(|_| {
        Error::InvalidNativeOutput {
            operation: "Shape::circle",
            output: "circle",
            constraint: "valid circle geometry",
        }
    })
}

#[inline]
pub(crate) fn shape_segment_impl(id: ShapeId) -> Result<Segment> {
    Segment::from_raw(unsafe { ffi::b2Shape_GetSegment(raw_shape_id(id)) }).map_err(|_| {
        Error::InvalidNativeOutput {
            operation: "Shape::segment",
            output: "segment",
            constraint: "valid non-degenerate segment geometry",
        }
    })
}

#[inline]
pub(crate) fn shape_chain_segment_impl(id: ShapeId) -> Result<ChainSegment> {
    ChainSegment::from_raw(unsafe { ffi::b2Shape_GetChainSegment(raw_shape_id(id)) }).map_err(
        |_| Error::InvalidNativeOutput {
            operation: "Shape::chain_segment",
            output: "chain_segment",
            constraint: "valid chain-segment geometry",
        },
    )
}

#[inline]
pub(crate) fn shape_capsule_impl(id: ShapeId) -> Result<Capsule> {
    Capsule::from_raw(unsafe { ffi::b2Shape_GetCapsule(raw_shape_id(id)) }).map_err(|_| {
        Error::InvalidNativeOutput {
            operation: "Shape::capsule",
            output: "capsule",
            constraint: "valid capsule geometry",
        }
    })
}

#[inline]
pub(crate) fn shape_polygon_impl(id: ShapeId) -> Result<Polygon> {
    Polygon::from_raw(unsafe { ffi::b2Shape_GetPolygon(raw_shape_id(id)) }).map_err(|_| {
        Error::InvalidNativeOutput {
            operation: "Shape::polygon",
            output: "polygon",
            constraint: "valid convex polygon geometry",
        }
    })
}

#[inline]
pub(crate) fn shape_closest_point_impl<P: Into<Position>>(
    id: ShapeId,
    target: P,
) -> Result<Position> {
    let target: ffi::b2Pos = target.into().into_raw();
    crate::body::check_valid_native_body_position(
        "Shape::closest_point",
        "closest_point",
        Position::from_raw(unsafe { ffi::b2Shape_GetClosestPoint(raw_shape_id(id), target) }),
    )
}

#[inline]
pub(crate) fn shape_aabb_impl(id: ShapeId) -> Result<Aabb> {
    Aabb::from_raw(unsafe { ffi::b2Shape_GetAABB(raw_shape_id(id)) }).map_err(|_| {
        Error::InvalidNativeOutput {
            operation: "Shape::aabb",
            output: "aabb",
            constraint: "finite ordered lower and upper bounds",
        }
    })
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
) -> Result<WorldCastOutput> {
    let origin = origin.into_raw();
    let translation = translation.into_raw();
    WorldCastOutput::from_native("Shape::ray_cast", unsafe {
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
pub(crate) fn shape_mass_data_impl(id: ShapeId) -> Result<MassData> {
    MassData::from_native("Shape::mass_data", unsafe {
        ffi::b2Shape_ComputeMassData(raw_shape_id(id))
    })
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
pub(crate) fn shape_density_impl(id: ShapeId) -> Result<f32> {
    check_native_shape_non_negative_scalar("Shape::density", "density", unsafe {
        ffi::b2Shape_GetDensity(raw_shape_id(id))
    })
}

#[inline]
pub(crate) fn shape_set_friction_impl(id: ShapeId, friction: f32) {
    unsafe { ffi::b2Shape_SetFriction(raw_shape_id(id), friction) }
}

#[inline]
pub(crate) fn shape_friction_impl(id: ShapeId) -> Result<f32> {
    check_native_shape_non_negative_scalar("Shape::friction", "friction", unsafe {
        ffi::b2Shape_GetFriction(raw_shape_id(id))
    })
}

#[inline]
pub(crate) fn shape_set_restitution_impl(id: ShapeId, restitution: f32) {
    unsafe { ffi::b2Shape_SetRestitution(raw_shape_id(id), restitution) }
}

#[inline]
pub(crate) fn shape_restitution_impl(id: ShapeId) -> Result<f32> {
    check_native_shape_non_negative_scalar("Shape::restitution", "restitution", unsafe {
        ffi::b2Shape_GetRestitution(raw_shape_id(id))
    })
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
pub(crate) fn shape_surface_material_impl(id: ShapeId) -> Result<SurfaceMaterial> {
    SurfaceMaterial::from_raw(unsafe { ffi::b2Shape_GetSurfaceMaterial(raw_shape_id(id)) }).map_err(
        |_| Error::InvalidNativeOutput {
            operation: "Shape::surface_material",
            output: "surface_material",
            constraint: "a valid finite surface material",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Err(Error::InvalidNativeShapeType { raw })
        );
    }

    #[test]
    fn shape_acquisition_reports_unknown_type_once_then_stops_before_get_type() {
        let raw = ffi::b2ShapeType_b2_shapeTypeCount;
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape = world
            .body(body)
            .unwrap()
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        let get_type = ShapeGetTypeOverride::install(raw);

        assert_eq!(
            world.shape(shape).err(),
            Some(Error::InvalidNativeShapeType { raw })
        );
        assert_eq!(get_type.calls(), 1);
        assert_eq!(world.shape(shape).err(), Some(Error::WorldPoisoned));
        assert_eq!(get_type.calls(), 1);
    }

    #[test]
    fn shape_type_is_authenticated_once_per_capability() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape = world
            .body(body)
            .unwrap()
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        let get_type = ShapeGetTypeOverride::install(ShapeType::Circle.into_raw());
        let shape = world.shape(shape).unwrap();

        assert_eq!(get_type.calls(), 1);
        assert_eq!(shape.shape_type(), Ok(ShapeType::Circle));
        assert_eq!(
            shape.circle(),
            Ok(crate::shapes::circle([0.0, 0.0], 0.5).unwrap())
        );
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
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let world0 = world.core().brand().world0();

        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| {
                    resolver.active_body(ffi::b2BodyId {
                        index1: 0,
                        world0,
                        generation: 0,
                    })
                })
                .unwrap_err(),
            Error::InvalidBodyId
        );
        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| {
                    resolver.active_body(ffi::b2BodyId {
                        index1: 1,
                        world0: world0.wrapping_add(1),
                        generation: 0,
                    })
                })
                .unwrap_err(),
            Error::WrongWorld
        );
    }

    #[test]
    fn shape_parent_chain_output_distinguishes_absent_and_foreign_ids() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let world0 = world.core().brand().world0();

        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| {
                    bind_shape_parent_chain_output(
                        resolver,
                        ffi::b2ChainId {
                            index1: 0,
                            world0: 0,
                            generation: 0,
                        },
                    )
                })
                .unwrap(),
            None
        );
        assert_eq!(
            world
                .core()
                .with_output_identity_resolver(|resolver| {
                    bind_shape_parent_chain_output(
                        resolver,
                        ffi::b2ChainId {
                            index1: 1,
                            world0: world0.wrapping_add(1),
                            generation: 0,
                        },
                    )
                })
                .unwrap_err(),
            Error::WrongWorld
        );
    }
}
