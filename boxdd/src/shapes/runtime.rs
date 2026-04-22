use super::*;

mod creation;
mod user_data;
mod validation;

pub(crate) use self::{creation::*, user_data::*, validation::*};

pub(crate) fn shape_type_from_ffi(raw: ffi::b2ShapeType) -> ShapeType {
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

pub(crate) fn retain_valid_shape_ids(ids: &mut Vec<ShapeId>) {
    ids.retain(|sid| unsafe { ffi::b2Shape_IsValid(raw_shape_id(*sid)) });
}

pub(crate) fn shape_contact_capacity(id: ShapeId) -> usize {
    unsafe { ffi::b2Shape_GetContactCapacity(raw_shape_id(id)) }.max(0) as usize
}

pub(crate) fn shape_contact_data_into_impl(id: ShapeId, out: &mut Vec<ContactData>) {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        });
    }
}

pub(crate) fn shape_contact_data_impl(id: ShapeId) -> Vec<ContactData> {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi::<ContactData>(cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        })
    }
}

pub(crate) fn shape_contact_data_raw_into_impl(id: ShapeId, out: &mut Vec<ffi::b2ContactData>) {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        });
    }
}

pub(crate) fn shape_contact_data_raw_impl(id: ShapeId) -> Vec<ffi::b2ContactData> {
    let cap = shape_contact_capacity(id);
    let id = raw_shape_id(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        })
    }
}

pub(crate) fn shape_contact_data_checked_impl(id: ShapeId) -> Vec<ContactData> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_impl(id)
}

pub(crate) fn shape_contact_data_into_checked_impl(id: ShapeId, out: &mut Vec<ContactData>) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_into_impl(id, out);
}

pub(crate) fn try_shape_contact_data_impl(id: ShapeId) -> ApiResult<Vec<ContactData>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_contact_data_impl(id))
}

pub(crate) fn try_shape_contact_data_into_impl(
    id: ShapeId,
    out: &mut Vec<ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_contact_data_into_impl(id, out);
    Ok(())
}

pub(crate) fn shape_contact_data_raw_checked_impl(id: ShapeId) -> Vec<ffi::b2ContactData> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_raw_impl(id)
}

pub(crate) fn shape_contact_data_raw_into_checked_impl(
    id: ShapeId,
    out: &mut Vec<ffi::b2ContactData>,
) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_contact_data_raw_into_impl(id, out);
}

pub(crate) fn try_shape_contact_data_raw_impl(id: ShapeId) -> ApiResult<Vec<ffi::b2ContactData>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_contact_data_raw_impl(id))
}

pub(crate) fn try_shape_contact_data_raw_into_impl(
    id: ShapeId,
    out: &mut Vec<ffi::b2ContactData>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_contact_data_raw_into_impl(id, out);
    Ok(())
}

pub(crate) fn shape_sensor_capacity_checked_impl(id: ShapeId) -> i32 {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_capacity_impl(id)
}

pub(crate) fn try_shape_sensor_capacity_impl(id: ShapeId) -> ApiResult<i32> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_sensor_capacity_impl(id))
}

pub(crate) fn shape_sensor_overlaps_checked_impl(id: ShapeId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_impl(id)
}

pub(crate) fn shape_sensor_overlaps_into_checked_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_into_impl(id, out);
}

pub(crate) fn try_shape_sensor_overlaps_impl(id: ShapeId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_sensor_overlaps_impl(id))
}

pub(crate) fn try_shape_sensor_overlaps_into_impl(
    id: ShapeId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_sensor_overlaps_into_impl(id, out);
    Ok(())
}

pub(crate) fn shape_sensor_overlaps_valid_checked_impl(id: ShapeId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_valid_impl(id)
}

pub(crate) fn try_shape_sensor_overlaps_valid_impl(id: ShapeId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_shape_valid(id)?;
    Ok(shape_sensor_overlaps_valid_impl(id))
}

pub(crate) fn shape_sensor_overlaps_valid_into_checked_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_shape_valid(id);
    shape_sensor_overlaps_valid_into_impl(id, out);
}

pub(crate) fn try_shape_sensor_overlaps_valid_into_impl(
    id: ShapeId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    shape_sensor_overlaps_valid_into_impl(id, out);
    Ok(())
}

pub(crate) fn shape_sensor_overlaps_into_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    let id = raw_shape_id(id);
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr.cast(), cap)
        });
    }
}

pub(crate) fn shape_sensor_overlaps_impl(id: ShapeId) -> Vec<ShapeId> {
    let id = raw_shape_id(id);
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr: *mut ShapeId, cap| {
            ffi::b2Shape_GetSensorData(id, ptr.cast(), cap)
        })
    }
}

pub(crate) fn shape_sensor_overlaps_valid_into_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    shape_sensor_overlaps_into_impl(id, out);
    retain_valid_shape_ids(out);
}

pub(crate) fn shape_sensor_overlaps_valid_impl(id: ShapeId) -> Vec<ShapeId> {
    let mut ids = shape_sensor_overlaps_impl(id);
    retain_valid_shape_ids(&mut ids);
    ids
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
pub(crate) fn make_shape_ray_input<VO: Into<Vec2>, VT: Into<Vec2>>(
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

#[inline]
pub(crate) fn shape_sensor_capacity_impl(id: ShapeId) -> i32 {
    unsafe { ffi::b2Shape_GetSensorCapacity(raw_shape_id(id)) }
}

pub(crate) trait ShapeRuntimeHandle {
    fn shape_id(&self) -> ShapeId;
    fn shape_world_core(&self) -> &crate::core::world_core::WorldCore;

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_shape_valid(self.shape_id());
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(self.shape_id())
    }

    fn world_id_raw(&self) -> ffi::b2WorldId {
        shape_world_id_checked_impl(self.shape_id())
    }

    fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        try_shape_world_id_raw_impl(self.shape_id())
    }

    fn parent_chain_id(&self) -> Option<ChainId> {
        shape_parent_chain_id_checked_impl(self.shape_id())
    }

    fn try_parent_chain_id(&self) -> ApiResult<Option<ChainId>> {
        try_shape_parent_chain_id_impl(self.shape_id())
    }

    fn is_valid(&self) -> bool {
        shape_is_valid_checked_impl(self.shape_id())
    }

    fn try_is_valid(&self) -> ApiResult<bool> {
        try_shape_is_valid_impl(self.shape_id())
    }

    unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe {
            shape_set_user_data_ptr_raw_checked_impl(self.shape_world_core(), self.shape_id(), p)
        }
    }

    unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { try_shape_set_user_data_ptr_raw_impl(self.shape_world_core(), self.shape_id(), p) }
    }

    fn user_data_ptr_raw(&self) -> *mut c_void {
        shape_user_data_ptr_raw_checked_impl(self.shape_id())
    }

    fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_shape_user_data_ptr_raw_impl(self.shape_id())
    }

    fn set_user_data<T: 'static>(&mut self, value: T) {
        shape_set_user_data_checked_impl(self.shape_world_core(), self.shape_id(), value);
    }

    fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        try_shape_set_user_data_checked_impl(self.shape_world_core(), self.shape_id(), value)
    }

    fn clear_user_data(&mut self) -> bool {
        shape_clear_user_data_checked_impl(self.shape_world_core(), self.shape_id())
    }

    fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        try_shape_clear_user_data_checked_impl(self.shape_world_core(), self.shape_id())
    }

    fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        shape_with_user_data_checked_impl(self.shape_world_core(), self.shape_id(), f)
    }

    fn try_with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> ApiResult<Option<R>> {
        try_shape_with_user_data_checked_impl(self.shape_world_core(), self.shape_id(), f)
    }

    fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        shape_with_user_data_mut_checked_impl(self.shape_world_core(), self.shape_id(), f)
    }

    fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        try_shape_with_user_data_mut_checked_impl(self.shape_world_core(), self.shape_id(), f)
    }

    fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        shape_take_user_data_checked_impl(self.shape_world_core(), self.shape_id())
    }

    fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        try_shape_take_user_data_checked_impl(self.shape_world_core(), self.shape_id())
    }

    fn contact_data(&self) -> Vec<ContactData> {
        shape_contact_data_checked_impl(self.shape_id())
    }

    fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        shape_contact_data_into_checked_impl(self.shape_id(), out);
    }

    fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        try_shape_contact_data_impl(self.shape_id())
    }

    fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        try_shape_contact_data_into_impl(self.shape_id(), out)
    }

    fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        shape_contact_data_raw_checked_impl(self.shape_id())
    }

    fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        shape_contact_data_raw_into_checked_impl(self.shape_id(), out);
    }

    fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        try_shape_contact_data_raw_impl(self.shape_id())
    }

    fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        try_shape_contact_data_raw_into_impl(self.shape_id(), out)
    }

    fn sensor_capacity(&self) -> i32 {
        shape_sensor_capacity_checked_impl(self.shape_id())
    }

    fn try_sensor_capacity(&self) -> ApiResult<i32> {
        try_shape_sensor_capacity_impl(self.shape_id())
    }

    fn sensor_overlaps(&self) -> Vec<ShapeId> {
        shape_sensor_overlaps_checked_impl(self.shape_id())
    }

    fn sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) {
        shape_sensor_overlaps_into_checked_impl(self.shape_id(), out);
    }

    fn try_sensor_overlaps(&self) -> ApiResult<Vec<ShapeId>> {
        try_shape_sensor_overlaps_impl(self.shape_id())
    }

    fn try_sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_shape_sensor_overlaps_into_impl(self.shape_id(), out)
    }

    fn sensor_overlaps_valid(&self) -> Vec<ShapeId> {
        shape_sensor_overlaps_valid_checked_impl(self.shape_id())
    }

    fn try_sensor_overlaps_valid(&self) -> ApiResult<Vec<ShapeId>> {
        try_shape_sensor_overlaps_valid_impl(self.shape_id())
    }

    fn sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) {
        shape_sensor_overlaps_valid_into_checked_impl(self.shape_id(), out);
    }

    fn try_sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_shape_sensor_overlaps_valid_into_impl(self.shape_id(), out)
    }

    fn is_sensor(&self) -> bool {
        self.assert_valid();
        shape_is_sensor_impl(self.shape_id())
    }

    fn try_is_sensor(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_is_sensor_impl(self.shape_id()))
    }

    fn enable_sensor_events(&mut self, flag: bool) {
        self.assert_valid();
        shape_enable_sensor_events_impl(self.shape_id(), flag)
    }

    fn try_enable_sensor_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        shape_enable_sensor_events_impl(self.shape_id(), flag);
        Ok(())
    }

    fn sensor_events_enabled(&self) -> bool {
        self.assert_valid();
        shape_sensor_events_enabled_impl(self.shape_id())
    }

    fn try_sensor_events_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_sensor_events_enabled_impl(self.shape_id()))
    }

    fn enable_contact_events(&mut self, flag: bool) {
        self.assert_valid();
        shape_enable_contact_events_impl(self.shape_id(), flag)
    }

    fn try_enable_contact_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        shape_enable_contact_events_impl(self.shape_id(), flag);
        Ok(())
    }

    fn contact_events_enabled(&self) -> bool {
        self.assert_valid();
        shape_contact_events_enabled_impl(self.shape_id())
    }

    fn try_contact_events_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_contact_events_enabled_impl(self.shape_id()))
    }

    fn enable_pre_solve_events(&mut self, flag: bool) {
        self.assert_valid();
        shape_enable_pre_solve_events_impl(self.shape_id(), flag)
    }

    fn try_enable_pre_solve_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        shape_enable_pre_solve_events_impl(self.shape_id(), flag);
        Ok(())
    }

    fn pre_solve_events_enabled(&self) -> bool {
        self.assert_valid();
        shape_pre_solve_events_enabled_impl(self.shape_id())
    }

    fn try_pre_solve_events_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_pre_solve_events_enabled_impl(self.shape_id()))
    }

    fn enable_hit_events(&mut self, flag: bool) {
        self.assert_valid();
        shape_enable_hit_events_impl(self.shape_id(), flag)
    }

    fn try_enable_hit_events(&mut self, flag: bool) -> ApiResult<()> {
        self.check_valid()?;
        shape_enable_hit_events_impl(self.shape_id(), flag);
        Ok(())
    }

    fn hit_events_enabled(&self) -> bool {
        self.assert_valid();
        shape_hit_events_enabled_impl(self.shape_id())
    }

    fn try_hit_events_enabled(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_hit_events_enabled_impl(self.shape_id()))
    }

    fn shape_type(&self) -> ShapeType {
        self.assert_valid();
        shape_type_impl(self.shape_id())
    }

    fn try_shape_type(&self) -> ApiResult<ShapeType> {
        self.check_valid()?;
        Ok(shape_type_impl(self.shape_id()))
    }

    fn shape_type_raw(&self) -> ffi::b2ShapeType {
        self.assert_valid();
        shape_type_raw_impl(self.shape_id())
    }

    fn try_shape_type_raw(&self) -> ApiResult<ffi::b2ShapeType> {
        self.check_valid()?;
        Ok(shape_type_raw_impl(self.shape_id()))
    }

    fn body_id(&self) -> BodyId {
        self.assert_valid();
        shape_body_id_impl(self.shape_id())
    }

    fn try_body_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(shape_body_id_impl(self.shape_id()))
    }

    fn circle(&self) -> Circle {
        self.assert_valid();
        shape_circle_impl(self.shape_id())
    }

    fn segment(&self) -> Segment {
        self.assert_valid();
        shape_segment_impl(self.shape_id())
    }

    fn chain_segment(&self) -> ChainSegment {
        self.assert_valid();
        shape_chain_segment_impl(self.shape_id())
    }

    fn capsule(&self) -> Capsule {
        self.assert_valid();
        shape_capsule_impl(self.shape_id())
    }

    fn polygon(&self) -> Polygon {
        self.assert_valid();
        shape_polygon_impl(self.shape_id())
    }

    fn closest_point<V: Into<Vec2>>(&self, target: V) -> Vec2 {
        self.assert_valid();
        shape_closest_point_impl(self.shape_id(), target)
    }

    fn try_closest_point<V: Into<Vec2>>(&self, target: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(shape_closest_point_impl(self.shape_id(), target))
    }

    fn aabb(&self) -> Aabb {
        self.assert_valid();
        shape_aabb_impl(self.shape_id())
    }

    fn try_aabb(&self) -> ApiResult<Aabb> {
        self.check_valid()?;
        Ok(shape_aabb_impl(self.shape_id()))
    }

    fn test_point<V: Into<Vec2>>(&self, point: V) -> bool {
        self.assert_valid();
        shape_test_point_impl(self.shape_id(), point)
    }

    fn try_test_point<V: Into<Vec2>>(&self, point: V) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_test_point_impl(self.shape_id(), point))
    }

    fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(&self, origin: VO, translation: VT) -> CastOutput {
        self.assert_valid();
        shape_ray_cast_impl(self.shape_id(), origin, translation)
    }

    fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        self.check_valid()?;
        Ok(shape_ray_cast_impl(self.shape_id(), origin, translation))
    }

    fn apply_wind<V: Into<Vec2>>(&mut self, wind: V, drag: f32, lift: f32, wake: bool) {
        self.assert_valid();
        shape_apply_wind_impl(self.shape_id(), wind, drag, lift, wake)
    }

    fn try_apply_wind<V: Into<Vec2>>(
        &mut self,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        shape_apply_wind_impl(self.shape_id(), wind, drag, lift, wake);
        Ok(())
    }

    fn set_circle(&mut self, circle: &Circle) {
        self.assert_valid();
        assert_circle_geometry_valid(circle);
        shape_set_circle_impl(self.shape_id(), circle)
    }

    fn try_set_circle(&mut self, circle: &Circle) -> ApiResult<()> {
        self.check_valid()?;
        check_circle_geometry_valid(circle)?;
        shape_set_circle_impl(self.shape_id(), circle);
        Ok(())
    }

    fn set_segment(&mut self, segment: &Segment) {
        self.assert_valid();
        assert_segment_geometry_valid(segment);
        shape_set_segment_impl(self.shape_id(), segment)
    }

    fn try_set_segment(&mut self, segment: &Segment) -> ApiResult<()> {
        self.check_valid()?;
        check_segment_geometry_valid(segment)?;
        shape_set_segment_impl(self.shape_id(), segment);
        Ok(())
    }

    fn set_capsule(&mut self, capsule: &Capsule) {
        self.assert_valid();
        assert_capsule_geometry_valid(capsule);
        shape_set_capsule_impl(self.shape_id(), capsule)
    }

    fn try_set_capsule(&mut self, capsule: &Capsule) -> ApiResult<()> {
        self.check_valid()?;
        check_capsule_geometry_valid(capsule)?;
        shape_set_capsule_impl(self.shape_id(), capsule);
        Ok(())
    }

    fn set_polygon(&mut self, polygon: &Polygon) {
        self.assert_valid();
        assert_polygon_geometry_valid(polygon);
        shape_set_polygon_impl(self.shape_id(), polygon)
    }

    fn try_set_polygon(&mut self, polygon: &Polygon) -> ApiResult<()> {
        self.check_valid()?;
        check_polygon_geometry_valid(polygon)?;
        shape_set_polygon_impl(self.shape_id(), polygon);
        Ok(())
    }

    fn filter(&self) -> Filter {
        self.assert_valid();
        shape_filter_impl(self.shape_id())
    }

    fn try_filter(&self) -> ApiResult<Filter> {
        self.check_valid()?;
        Ok(shape_filter_impl(self.shape_id()))
    }

    fn set_filter(&mut self, filter: Filter) {
        self.assert_valid();
        shape_set_filter_impl(self.shape_id(), filter)
    }

    fn try_set_filter(&mut self, filter: Filter) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_filter_impl(self.shape_id(), filter);
        Ok(())
    }

    fn set_density(&mut self, density: f32, update_body_mass: bool) {
        shape_set_density_checked_impl(self.shape_id(), density, update_body_mass)
    }

    fn try_set_density(&mut self, density: f32, update_body_mass: bool) -> ApiResult<()> {
        try_shape_set_density_checked_impl(self.shape_id(), density, update_body_mass)
    }

    fn density(&self) -> f32 {
        self.assert_valid();
        shape_density_impl(self.shape_id())
    }

    fn try_density(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_density_impl(self.shape_id()))
    }

    fn mass_data(&self) -> MassData {
        self.assert_valid();
        shape_mass_data_impl(self.shape_id())
    }

    fn try_mass_data(&self) -> ApiResult<MassData> {
        self.check_valid()?;
        Ok(shape_mass_data_impl(self.shape_id()))
    }

    fn set_friction(&mut self, friction: f32) {
        shape_set_friction_checked_impl(self.shape_id(), friction)
    }

    fn try_set_friction(&mut self, friction: f32) -> ApiResult<()> {
        try_shape_set_friction_checked_impl(self.shape_id(), friction)
    }

    fn friction(&self) -> f32 {
        self.assert_valid();
        shape_friction_impl(self.shape_id())
    }

    fn try_friction(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_friction_impl(self.shape_id()))
    }

    fn set_restitution(&mut self, restitution: f32) {
        shape_set_restitution_checked_impl(self.shape_id(), restitution)
    }

    fn try_set_restitution(&mut self, restitution: f32) -> ApiResult<()> {
        try_shape_set_restitution_checked_impl(self.shape_id(), restitution)
    }

    fn restitution(&self) -> f32 {
        self.assert_valid();
        shape_restitution_impl(self.shape_id())
    }

    fn try_restitution(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_restitution_impl(self.shape_id()))
    }

    fn set_user_material(&mut self, material: u64) {
        self.assert_valid();
        shape_set_user_material_impl(self.shape_id(), material)
    }

    fn try_set_user_material(&mut self, material: u64) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_user_material_impl(self.shape_id(), material);
        Ok(())
    }

    fn user_material(&self) -> u64 {
        self.assert_valid();
        shape_user_material_impl(self.shape_id())
    }

    fn try_user_material(&self) -> ApiResult<u64> {
        self.check_valid()?;
        Ok(shape_user_material_impl(self.shape_id()))
    }

    fn set_surface_material(&mut self, material: &SurfaceMaterial) {
        self.assert_valid();
        shape_set_surface_material_impl(self.shape_id(), material)
    }

    fn try_set_surface_material(&mut self, material: &SurfaceMaterial) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_surface_material_impl(self.shape_id(), material);
        Ok(())
    }

    fn surface_material(&self) -> SurfaceMaterial {
        self.assert_valid();
        shape_surface_material_impl(self.shape_id())
    }

    fn try_surface_material(&self) -> ApiResult<SurfaceMaterial> {
        self.check_valid()?;
        Ok(shape_surface_material_impl(self.shape_id()))
    }
}
