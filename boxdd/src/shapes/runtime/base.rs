use super::*;

fn shape_type_from_ffi(raw: ffi::b2ShapeType) -> ShapeType {
    ShapeType::from_raw(raw).expect("Box2D returned an unknown shape type")
}

#[inline]
pub(crate) fn raw_shape_id(id: ShapeId) -> ffi::b2ShapeId {
    id.into_raw()
}

#[inline]
pub(crate) fn raw_chain_id(id: ChainId) -> ffi::b2ChainId {
    id.into_raw()
}

#[inline]
pub(crate) fn shape_world_id_impl(id: ShapeId) -> ffi::b2WorldId {
    unsafe { ffi::b2Shape_GetWorld(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_parent_chain_id_impl(id: ShapeId) -> Option<ChainId> {
    let chain_id = ChainId::from_raw(unsafe { ffi::b2Shape_GetParentChain(raw_shape_id(id)) });
    if unsafe { ffi::b2Chain_IsValid(raw_chain_id(chain_id)) } {
        Some(chain_id)
    } else {
        None
    }
}

#[inline]
pub(crate) fn shape_is_valid_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_IsValid(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_type_raw_impl(id: ShapeId) -> ffi::b2ShapeType {
    unsafe { ffi::b2Shape_GetType(raw_shape_id(id)) }
}

#[inline]
pub(crate) fn shape_type_impl(id: ShapeId) -> ShapeType {
    shape_type_from_ffi(shape_type_raw_impl(id))
}

#[inline]
pub(crate) fn shape_body_id_impl(id: ShapeId) -> BodyId {
    BodyId::from_raw(unsafe { ffi::b2Shape_GetBody(raw_shape_id(id)) })
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
    Polygon::from_raw(unsafe { ffi::b2Shape_GetPolygon(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_closest_point_impl<V: Into<Vec2>>(id: ShapeId, target: V) -> Vec2 {
    let target: ffi::b2Vec2 = target.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Shape_GetClosestPoint(raw_shape_id(id), target) })
}

#[inline]
pub(crate) fn shape_aabb_impl(id: ShapeId) -> Aabb {
    Aabb::from_raw(unsafe { ffi::b2Shape_GetAABB(raw_shape_id(id)) })
}

#[inline]
pub(crate) fn shape_test_point_impl<V: Into<Vec2>>(id: ShapeId, point: V) -> bool {
    let point: ffi::b2Vec2 = point.into().into_raw();
    unsafe { ffi::b2Shape_TestPoint(raw_shape_id(id), point) }
}

#[inline]
fn make_shape_ray_input<VO: Into<Vec2>, VT: Into<Vec2>>(
    origin: VO,
    translation: VT,
) -> ffi::b2RayCastInput {
    ffi::b2RayCastInput {
        origin: origin.into().into_raw(),
        translation: translation.into().into_raw(),
        maxFraction: 1.0,
    }
}

#[inline]
pub(crate) fn shape_ray_cast_impl<VO: Into<Vec2>, VT: Into<Vec2>>(
    id: ShapeId,
    origin: VO,
    translation: VT,
) -> CastOutput {
    let input = make_shape_ray_input(origin, translation);
    CastOutput::from_raw(unsafe { ffi::b2Shape_RayCast(raw_shape_id(id), &input) })
}

#[inline]
pub(crate) fn shape_apply_wind_impl<V: Into<Vec2>>(
    id: ShapeId,
    wind: V,
    drag: f32,
    lift: f32,
    wake: bool,
) {
    let wind: ffi::b2Vec2 = wind.into().into_raw();
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
