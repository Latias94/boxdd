//! Shapes API
//!
//! Safe wrappers around Box2D shapes. Shapes are attached to bodies and can be
//! modified at runtime. Use `ShapeDef` and `Body::create_*_shape` helpers to
//! create shapes.
use std::marker::PhantomData;
pub mod chain;
pub mod geometry;
pub mod helpers;

use crate::body::Body;
use crate::error::ApiResult;
use crate::filter::Filter;
use crate::types::{BodyId, ChainId, ContactData, ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::os::raw::c_void;
use std::rc::Rc;
use std::sync::Arc;

pub use geometry::{
    Capsule, ChainSegment, Circle, MAX_POLYGON_VERTICES, Polygon, Segment, box_polygon, capsule,
    chain_segment, circle, polygon_from_points, segment,
};

/// Shape kinds reported by Box2D.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ShapeType {
    Circle,
    Capsule,
    Segment,
    Polygon,
    ChainSegment,
}

impl ShapeType {
    #[inline]
    pub const fn from_raw(raw: ffi::b2ShapeType) -> Option<Self> {
        match raw {
            ffi::b2ShapeType_b2_circleShape => Some(Self::Circle),
            ffi::b2ShapeType_b2_capsuleShape => Some(Self::Capsule),
            ffi::b2ShapeType_b2_segmentShape => Some(Self::Segment),
            ffi::b2ShapeType_b2_polygonShape => Some(Self::Polygon),
            ffi::b2ShapeType_b2_chainSegmentShape => Some(Self::ChainSegment),
            _ => None,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2ShapeType {
        match self {
            Self::Circle => ffi::b2ShapeType_b2_circleShape,
            Self::Capsule => ffi::b2ShapeType_b2_capsuleShape,
            Self::Segment => ffi::b2ShapeType_b2_segmentShape,
            Self::Polygon => ffi::b2ShapeType_b2_polygonShape,
            Self::ChainSegment => ffi::b2ShapeType_b2_chainSegmentShape,
        }
    }
}

impl From<ShapeType> for ffi::b2ShapeType {
    #[inline]
    fn from(value: ShapeType) -> Self {
        value.into_raw()
    }
}

impl TryFrom<ffi::b2ShapeType> for ShapeType {
    type Error = ffi::b2ShapeType;

    #[inline]
    fn try_from(value: ffi::b2ShapeType) -> Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}

#[inline]
fn shape_type_from_ffi(raw: ffi::b2ShapeType) -> ShapeType {
    ShapeType::from_raw(raw).expect("Box2D returned an unknown shape type")
}

/// A scoped shape handle tied to a mutable borrow of the world.
pub struct Shape<'w> {
    pub(crate) id: ShapeId,
    #[allow(dead_code)]
    pub(crate) core: Arc<crate::core::world_core::WorldCore>,
    _world: PhantomData<&'w World>,
}

/// A RAII-owned shape that is destroyed on drop.
pub struct OwnedShape {
    id: ShapeId,
    core: Arc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
    update_body_mass_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

fn retain_valid_shape_ids(ids: &mut Vec<ShapeId>) {
    ids.retain(|sid| unsafe { ffi::b2Shape_IsValid(*sid) });
}

fn shape_contact_capacity(id: ShapeId) -> usize {
    unsafe { ffi::b2Shape_GetContactCapacity(id) }.max(0) as usize
}

fn shape_contact_data_into_impl(id: ShapeId, out: &mut Vec<ContactData>) {
    let cap = shape_contact_capacity(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        });
    }
}

fn shape_contact_data_impl(id: ShapeId) -> Vec<ContactData> {
    let cap = shape_contact_capacity(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi::<ContactData>(cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr.cast::<ffi::b2ContactData>(), cap)
        })
    }
}

fn shape_contact_data_into_raw_impl(id: ShapeId, out: &mut Vec<ffi::b2ContactData>) {
    let cap = shape_contact_capacity(id);
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        });
    }
}

fn shape_contact_data_raw_impl(id: ShapeId) -> Vec<ffi::b2ContactData> {
    let cap = shape_contact_capacity(id);
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Shape_GetContactData(id, ptr, cap)
        })
    }
}

fn shape_sensor_overlaps_into_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr, cap)
        });
    }
}

fn shape_sensor_overlaps_impl(id: ShapeId) -> Vec<ShapeId> {
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::read_from_ffi(cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr, cap)
        })
    }
}

fn shape_sensor_overlaps_valid_into_impl(id: ShapeId, out: &mut Vec<ShapeId>) {
    shape_sensor_overlaps_into_impl(id, out);
    retain_valid_shape_ids(out);
}

fn shape_sensor_overlaps_valid_impl(id: ShapeId) -> Vec<ShapeId> {
    let mut ids = shape_sensor_overlaps_impl(id);
    retain_valid_shape_ids(&mut ids);
    ids
}

#[inline]
fn shape_world_id_impl(id: ShapeId) -> ffi::b2WorldId {
    unsafe { ffi::b2Shape_GetWorld(id) }
}

#[inline]
fn shape_parent_chain_id_impl(id: ShapeId) -> Option<ChainId> {
    let chain_id = unsafe { ffi::b2Shape_GetParentChain(id) };
    if unsafe { ffi::b2Chain_IsValid(chain_id) } {
        Some(chain_id)
    } else {
        None
    }
}

#[inline]
fn shape_type_raw_impl(id: ShapeId) -> ffi::b2ShapeType {
    unsafe { ffi::b2Shape_GetType(id) }
}

#[inline]
fn shape_type_impl(id: ShapeId) -> ShapeType {
    shape_type_from_ffi(shape_type_raw_impl(id))
}

#[inline]
fn shape_body_id_impl(id: ShapeId) -> BodyId {
    unsafe { ffi::b2Shape_GetBody(id) }
}

#[inline]
fn shape_circle_impl(id: ShapeId) -> Circle {
    Circle::from_raw(unsafe { ffi::b2Shape_GetCircle(id) })
}

#[inline]
fn shape_segment_impl(id: ShapeId) -> Segment {
    Segment::from_raw(unsafe { ffi::b2Shape_GetSegment(id) })
}

#[inline]
fn shape_chain_segment_impl(id: ShapeId) -> ChainSegment {
    ChainSegment::from_raw(unsafe { ffi::b2Shape_GetChainSegment(id) })
}

#[inline]
fn shape_capsule_impl(id: ShapeId) -> Capsule {
    Capsule::from_raw(unsafe { ffi::b2Shape_GetCapsule(id) })
}

#[inline]
fn shape_polygon_impl(id: ShapeId) -> Polygon {
    Polygon::from_raw(unsafe { ffi::b2Shape_GetPolygon(id) })
}

#[inline]
fn shape_closest_point_impl<V: Into<Vec2>>(id: ShapeId, target: V) -> Vec2 {
    let target: ffi::b2Vec2 = target.into().into();
    Vec2::from(unsafe { ffi::b2Shape_GetClosestPoint(id, target) })
}

#[inline]
fn shape_apply_wind_impl<V: Into<Vec2>>(id: ShapeId, wind: V, drag: f32, lift: f32, wake: bool) {
    let wind: ffi::b2Vec2 = wind.into().into();
    unsafe { ffi::b2Shape_ApplyWind(id, wind, drag, lift, wake) }
}

#[inline]
fn shape_set_circle_impl(id: ShapeId, circle: &Circle) {
    let raw = circle.into_raw();
    unsafe { ffi::b2Shape_SetCircle(id, &raw) }
}

#[inline]
fn shape_set_segment_impl(id: ShapeId, segment: &Segment) {
    let raw = segment.into_raw();
    unsafe { ffi::b2Shape_SetSegment(id, &raw) }
}

#[inline]
fn shape_set_capsule_impl(id: ShapeId, capsule: &Capsule) {
    let raw = capsule.into_raw();
    unsafe { ffi::b2Shape_SetCapsule(id, &raw) }
}

#[inline]
fn shape_set_polygon_impl(id: ShapeId, polygon: &Polygon) {
    let raw = polygon.into_raw();
    unsafe { ffi::b2Shape_SetPolygon(id, &raw) }
}

#[inline]
fn shape_filter_impl(id: ShapeId) -> Filter {
    Filter::from_raw(unsafe { ffi::b2Shape_GetFilter(id) })
}

#[inline]
fn shape_set_filter_impl(id: ShapeId, filter: Filter) {
    unsafe { ffi::b2Shape_SetFilter(id, filter.into_raw()) }
}

#[inline]
fn shape_is_sensor_impl(id: ShapeId) -> bool {
    unsafe { ffi::b2Shape_IsSensor(id) }
}

#[inline]
fn shape_set_density_impl(id: ShapeId, density: f32, update_body_mass: bool) {
    unsafe { ffi::b2Shape_SetDensity(id, density, update_body_mass) }
}

#[inline]
fn shape_density_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetDensity(id) }
}

#[inline]
fn shape_set_friction_impl(id: ShapeId, friction: f32) {
    unsafe { ffi::b2Shape_SetFriction(id, friction) }
}

#[inline]
fn shape_friction_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetFriction(id) }
}

#[inline]
fn shape_set_restitution_impl(id: ShapeId, restitution: f32) {
    unsafe { ffi::b2Shape_SetRestitution(id, restitution) }
}

#[inline]
fn shape_restitution_impl(id: ShapeId) -> f32 {
    unsafe { ffi::b2Shape_GetRestitution(id) }
}

#[inline]
fn shape_set_user_material_impl(id: ShapeId, material: u64) {
    unsafe { ffi::b2Shape_SetUserMaterial(id, material) }
}

#[inline]
fn shape_user_material_impl(id: ShapeId) -> u64 {
    unsafe { ffi::b2Shape_GetUserMaterial(id) }
}

#[inline]
fn shape_set_surface_material_impl(id: ShapeId, material: &SurfaceMaterial) {
    unsafe { ffi::b2Shape_SetSurfaceMaterial(id, &material.0) }
}

#[inline]
fn shape_surface_material_impl(id: ShapeId) -> SurfaceMaterial {
    SurfaceMaterial(unsafe { ffi::b2Shape_GetSurfaceMaterial(id) })
}

#[inline]
fn shape_sensor_capacity_impl(id: ShapeId) -> i32 {
    unsafe { ffi::b2Shape_GetSensorCapacity(id) }
}

impl OwnedShape {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: ShapeId) -> Self {
        core.owned_shapes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            core,
            destroy_on_drop: true,
            update_body_mass_on_drop: true,
            _not_send: PhantomData,
        }
    }

    pub fn id(&self) -> ShapeId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        shape_world_id_impl(self.id)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(shape_world_id_impl(self.id))
    }

    pub fn parent_chain_id(&self) -> Option<ChainId> {
        self.assert_valid();
        shape_parent_chain_id_impl(self.id)
    }

    pub fn try_parent_chain_id(&self) -> ApiResult<Option<ChainId>> {
        self.check_valid()?;
        Ok(shape_parent_chain_id_impl(self.id))
    }

    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Shape_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Shape_IsValid(self.id) })
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_shape_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(self.id)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> ShapeId {
        self.id
    }

    pub fn is_sensor(&self) -> bool {
        self.assert_valid();
        shape_is_sensor_impl(self.id)
    }

    pub fn try_is_sensor(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_is_sensor_impl(self.id))
    }

    pub fn shape_type(&self) -> ShapeType {
        self.assert_valid();
        shape_type_impl(self.id)
    }

    pub fn try_shape_type(&self) -> ApiResult<ShapeType> {
        self.check_valid()?;
        Ok(shape_type_impl(self.id))
    }

    pub fn shape_type_raw(&self) -> ffi::b2ShapeType {
        self.assert_valid();
        shape_type_raw_impl(self.id)
    }

    pub fn try_shape_type_raw(&self) -> ApiResult<ffi::b2ShapeType> {
        self.check_valid()?;
        Ok(shape_type_raw_impl(self.id))
    }

    pub fn body_id(&self) -> BodyId {
        self.assert_valid();
        shape_body_id_impl(self.id)
    }

    pub fn try_body_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(shape_body_id_impl(self.id))
    }

    // Geometry
    pub fn circle(&self) -> Circle {
        self.assert_valid();
        shape_circle_impl(self.id)
    }
    pub fn segment(&self) -> Segment {
        self.assert_valid();
        shape_segment_impl(self.id)
    }
    pub fn chain_segment(&self) -> ChainSegment {
        self.assert_valid();
        shape_chain_segment_impl(self.id)
    }
    pub fn capsule(&self) -> Capsule {
        self.assert_valid();
        shape_capsule_impl(self.id)
    }
    pub fn polygon(&self) -> Polygon {
        self.assert_valid();
        shape_polygon_impl(self.id)
    }

    /// Return the closest point on this shape to `target` (in world coordinates).
    pub fn closest_point<V: Into<Vec2>>(&self, target: V) -> Vec2 {
        self.assert_valid();
        shape_closest_point_impl(self.id, target)
    }

    pub fn try_closest_point<V: Into<Vec2>>(&self, target: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(shape_closest_point_impl(self.id, target))
    }

    /// Apply wind force/torque approximation to the shape.
    pub fn apply_wind<V: Into<Vec2>>(&mut self, wind: V, drag: f32, lift: f32, wake: bool) {
        self.assert_valid();
        shape_apply_wind_impl(self.id, wind, drag, lift, wake)
    }

    pub fn try_apply_wind<V: Into<Vec2>>(
        &mut self,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        shape_apply_wind_impl(self.id, wind, drag, lift, wake);
        Ok(())
    }

    pub fn set_circle(&mut self, c: &Circle) {
        self.assert_valid();
        shape_set_circle_impl(self.id, c)
    }
    pub fn try_set_circle(&mut self, c: &Circle) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_circle_impl(self.id, c);
        Ok(())
    }
    pub fn set_segment(&mut self, s: &Segment) {
        self.assert_valid();
        shape_set_segment_impl(self.id, s)
    }
    pub fn try_set_segment(&mut self, s: &Segment) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_segment_impl(self.id, s);
        Ok(())
    }
    pub fn set_capsule(&mut self, c: &Capsule) {
        self.assert_valid();
        shape_set_capsule_impl(self.id, c)
    }
    pub fn try_set_capsule(&mut self, c: &Capsule) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_capsule_impl(self.id, c);
        Ok(())
    }
    pub fn set_polygon(&mut self, p: &Polygon) {
        self.assert_valid();
        shape_set_polygon_impl(self.id, p)
    }
    pub fn try_set_polygon(&mut self, p: &Polygon) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_polygon_impl(self.id, p);
        Ok(())
    }

    pub fn filter(&self) -> Filter {
        self.assert_valid();
        shape_filter_impl(self.id)
    }
    pub fn try_filter(&self) -> ApiResult<Filter> {
        self.check_valid()?;
        Ok(shape_filter_impl(self.id))
    }
    pub fn set_filter(&mut self, f: Filter) {
        self.assert_valid();
        shape_set_filter_impl(self.id, f)
    }
    pub fn try_set_filter(&mut self, f: Filter) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_filter_impl(self.id, f);
        Ok(())
    }

    pub fn set_density(&mut self, density: f32, update_body_mass: bool) {
        self.assert_valid();
        shape_set_density_impl(self.id, density, update_body_mass)
    }
    pub fn try_set_density(&mut self, density: f32, update_body_mass: bool) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_density_impl(self.id, density, update_body_mass);
        Ok(())
    }
    pub fn density(&self) -> f32 {
        self.assert_valid();
        shape_density_impl(self.id)
    }
    pub fn try_density(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_density_impl(self.id))
    }

    pub fn set_friction(&mut self, friction: f32) {
        self.assert_valid();
        shape_set_friction_impl(self.id, friction)
    }
    pub fn try_set_friction(&mut self, friction: f32) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_friction_impl(self.id, friction);
        Ok(())
    }
    pub fn friction(&self) -> f32 {
        self.assert_valid();
        shape_friction_impl(self.id)
    }
    pub fn try_friction(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_friction_impl(self.id))
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        self.assert_valid();
        shape_set_restitution_impl(self.id, restitution)
    }
    pub fn try_set_restitution(&mut self, restitution: f32) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_restitution_impl(self.id, restitution);
        Ok(())
    }
    pub fn restitution(&self) -> f32 {
        self.assert_valid();
        shape_restitution_impl(self.id)
    }
    pub fn try_restitution(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_restitution_impl(self.id))
    }

    pub fn set_user_material(&mut self, material: u64) {
        self.assert_valid();
        shape_set_user_material_impl(self.id, material)
    }
    pub fn try_set_user_material(&mut self, material: u64) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_user_material_impl(self.id, material);
        Ok(())
    }
    pub fn user_material(&self) -> u64 {
        self.assert_valid();
        shape_user_material_impl(self.id)
    }
    pub fn try_user_material(&self) -> ApiResult<u64> {
        self.check_valid()?;
        Ok(shape_user_material_impl(self.id))
    }

    pub fn set_surface_material(&mut self, material: &SurfaceMaterial) {
        self.assert_valid();
        shape_set_surface_material_impl(self.id, material)
    }
    pub fn try_set_surface_material(&mut self, material: &SurfaceMaterial) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_surface_material_impl(self.id, material);
        Ok(())
    }
    pub fn surface_material(&self) -> SurfaceMaterial {
        self.assert_valid();
        shape_surface_material_impl(self.id)
    }
    pub fn try_surface_material(&self) -> ApiResult<SurfaceMaterial> {
        self.check_valid()?;
        Ok(shape_surface_material_impl(self.id))
    }

    pub fn contact_data(&self) -> Vec<ContactData> {
        self.assert_valid();
        shape_contact_data_impl(self.id)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        self.assert_valid();
        shape_contact_data_into_impl(self.id, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        self.check_valid()?;
        Ok(shape_contact_data_impl(self.id))
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        shape_contact_data_into_impl(self.id, out);
        Ok(())
    }

    pub fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        self.assert_valid();
        shape_contact_data_raw_impl(self.id)
    }

    pub fn contact_data_into_raw(&self, out: &mut Vec<ffi::b2ContactData>) {
        self.assert_valid();
        shape_contact_data_into_raw_impl(self.id, out);
    }

    pub fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        self.check_valid()?;
        Ok(shape_contact_data_raw_impl(self.id))
    }

    pub fn try_contact_data_into_raw(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        shape_contact_data_into_raw_impl(self.id, out);
        Ok(())
    }

    /// Get the maximum capacity required for retrieving all overlapped shapes on this sensor shape.
    pub fn sensor_capacity(&self) -> i32 {
        self.assert_valid();
        shape_sensor_capacity_impl(self.id)
    }

    pub fn try_sensor_capacity(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(shape_sensor_capacity_impl(self.id))
    }

    pub fn sensor_overlaps(&self) -> Vec<ShapeId> {
        self.assert_valid();
        shape_sensor_overlaps_impl(self.id)
    }

    pub fn sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        shape_sensor_overlaps_into_impl(self.id, out);
    }

    pub fn try_sensor_overlaps(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        Ok(shape_sensor_overlaps_impl(self.id))
    }

    pub fn try_sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        shape_sensor_overlaps_into_impl(self.id, out);
        Ok(())
    }

    pub fn sensor_overlaps_valid(&self) -> Vec<ShapeId> {
        self.assert_valid();
        shape_sensor_overlaps_valid_impl(self.id)
    }

    pub fn sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        shape_sensor_overlaps_valid_into_impl(self.id, out);
    }

    pub fn try_sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        shape_sensor_overlaps_valid_into_impl(self.id, out);
        Ok(())
    }

    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as the engine may
    /// read it and that any aliasing/lifetime constraints are upheld. Box2D stores this
    /// pointer and may access it during simulation callbacks.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut c_void) {
        self.assert_valid();
        let _ = self.core.clear_shape_user_data(self.id);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) }
    }
    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr(&mut self, p: *mut c_void) -> ApiResult<()> {
        self.check_valid()?;
        let _ = self.core.clear_shape_user_data(self.id);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) }
        Ok(())
    }
    pub fn user_data_ptr(&self) -> *mut c_void {
        self.assert_valid();
        unsafe { ffi::b2Shape_GetUserData(self.id) }
    }

    pub fn try_user_data_ptr(&self) -> ApiResult<*mut c_void> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Shape_GetUserData(self.id) })
    }

    /// Set typed user data on this shape.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the shape is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        self.assert_valid();
        let p = self.core.set_shape_user_data(self.id, value);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        self.check_valid()?;
        let p = self.core.set_shape_user_data(self.id, value);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) };
        Ok(())
    }

    /// Clear typed user data on this shape. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        self.assert_valid();
        let had = self.core.clear_shape_user_data(self.id);
        if had {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        had
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        self.check_valid()?;
        let had = self.core.clear_shape_user_data(self.id);
        if had {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_shape_user_data(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_shape_user_data(self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_shape_user_data_mut(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_shape_user_data_mut(self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        self.assert_valid();
        let v = self
            .core
            .take_shape_user_data::<T>(self.id)
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        self.check_valid()?;
        let v = self.core.take_shape_user_data::<T>(self.id)?;
        if v.is_some() {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(v)
    }

    pub fn update_body_mass_on_drop(mut self, flag: bool) -> Self {
        self.update_body_mass_on_drop = flag;
        self
    }

    /// Disarm RAII and return the raw id for manual lifetime management.
    pub fn into_id(mut self) -> ShapeId {
        self.destroy_on_drop = false;
        self.id
    }

    /// Destroy the shape immediately and disarm drop.
    pub fn destroy(mut self, update_body_mass: bool) {
        if self.destroy_on_drop && unsafe { ffi::b2Shape_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Shape {
                        id: self.id,
                        update_body_mass,
                    });
            } else {
                unsafe { ffi::b2DestroyShape(self.id, update_body_mass) };
                let _ = self.core.clear_shape_user_data(self.id);
                #[cfg(feature = "serialize")]
                self.core.remove_shape_flags(self.id);
            }
        }
        self.destroy_on_drop = false;
    }
}

impl Drop for OwnedShape {
    fn drop(&mut self) {
        let _ = self.core.id;
        let prev = self
            .core
            .owned_shapes
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug_assert!(prev > 0, "owned shape counter underflow");
        if self.destroy_on_drop && unsafe { ffi::b2Shape_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Shape {
                        id: self.id,
                        update_body_mass: self.update_body_mass_on_drop,
                    });
            } else {
                unsafe { ffi::b2DestroyShape(self.id, self.update_body_mass_on_drop) };
                let _ = self.core.clear_shape_user_data(self.id);
                #[cfg(feature = "serialize")]
                self.core.remove_shape_flags(self.id);
            }
        }
    }
}

impl<'w> Shape<'w> {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: ShapeId) -> Self {
        Self {
            id,
            core,
            _world: PhantomData,
        }
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_shape_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(self.id)
    }

    pub fn id(&self) -> ShapeId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        shape_world_id_impl(self.id)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(shape_world_id_impl(self.id))
    }

    pub fn parent_chain_id(&self) -> Option<ChainId> {
        self.assert_valid();
        shape_parent_chain_id_impl(self.id)
    }

    pub fn try_parent_chain_id(&self) -> ApiResult<Option<ChainId>> {
        self.check_valid()?;
        Ok(shape_parent_chain_id_impl(self.id))
    }

    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Shape_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Shape_IsValid(self.id) })
    }

    pub fn shape_type(&self) -> ShapeType {
        self.assert_valid();
        shape_type_impl(self.id)
    }

    pub fn try_shape_type(&self) -> ApiResult<ShapeType> {
        self.check_valid()?;
        Ok(shape_type_impl(self.id))
    }

    pub fn shape_type_raw(&self) -> ffi::b2ShapeType {
        self.assert_valid();
        shape_type_raw_impl(self.id)
    }

    pub fn try_shape_type_raw(&self) -> ApiResult<ffi::b2ShapeType> {
        self.check_valid()?;
        Ok(shape_type_raw_impl(self.id))
    }

    pub fn body_id(&self) -> BodyId {
        self.assert_valid();
        shape_body_id_impl(self.id)
    }

    pub fn try_body_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        Ok(shape_body_id_impl(self.id))
    }

    // Getters
    pub fn circle(&self) -> Circle {
        self.assert_valid();
        shape_circle_impl(self.id)
    }
    pub fn segment(&self) -> Segment {
        self.assert_valid();
        shape_segment_impl(self.id)
    }
    pub fn chain_segment(&self) -> ChainSegment {
        self.assert_valid();
        shape_chain_segment_impl(self.id)
    }
    pub fn capsule(&self) -> Capsule {
        self.assert_valid();
        shape_capsule_impl(self.id)
    }
    pub fn polygon(&self) -> Polygon {
        self.assert_valid();
        shape_polygon_impl(self.id)
    }

    /// Return the closest point on this shape to `target` (in world coordinates).
    pub fn closest_point<V: Into<Vec2>>(&self, target: V) -> Vec2 {
        self.assert_valid();
        shape_closest_point_impl(self.id, target)
    }

    pub fn try_closest_point<V: Into<Vec2>>(&self, target: V) -> ApiResult<Vec2> {
        self.check_valid()?;
        Ok(shape_closest_point_impl(self.id, target))
    }

    /// Apply wind force/torque approximation to the shape.
    pub fn apply_wind<V: Into<Vec2>>(&mut self, wind: V, drag: f32, lift: f32, wake: bool) {
        self.assert_valid();
        shape_apply_wind_impl(self.id, wind, drag, lift, wake)
    }

    pub fn try_apply_wind<V: Into<Vec2>>(
        &mut self,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> ApiResult<()> {
        self.check_valid()?;
        shape_apply_wind_impl(self.id, wind, drag, lift, wake);
        Ok(())
    }

    // Setters
    pub fn set_circle(&mut self, c: &Circle) {
        self.assert_valid();
        shape_set_circle_impl(self.id, c)
    }
    pub fn try_set_circle(&mut self, c: &Circle) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_circle_impl(self.id, c);
        Ok(())
    }
    pub fn set_segment(&mut self, s: &Segment) {
        self.assert_valid();
        shape_set_segment_impl(self.id, s)
    }
    pub fn try_set_segment(&mut self, s: &Segment) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_segment_impl(self.id, s);
        Ok(())
    }
    pub fn set_capsule(&mut self, c: &Capsule) {
        self.assert_valid();
        shape_set_capsule_impl(self.id, c)
    }
    pub fn try_set_capsule(&mut self, c: &Capsule) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_capsule_impl(self.id, c);
        Ok(())
    }
    pub fn set_polygon(&mut self, p: &Polygon) {
        self.assert_valid();
        shape_set_polygon_impl(self.id, p)
    }
    pub fn try_set_polygon(&mut self, p: &Polygon) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_polygon_impl(self.id, p);
        Ok(())
    }

    pub fn filter(&self) -> Filter {
        self.assert_valid();
        shape_filter_impl(self.id)
    }
    pub fn try_filter(&self) -> ApiResult<Filter> {
        self.check_valid()?;
        Ok(shape_filter_impl(self.id))
    }
    pub fn set_filter(&mut self, f: Filter) {
        self.assert_valid();
        shape_set_filter_impl(self.id, f)
    }
    pub fn try_set_filter(&mut self, f: Filter) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_filter_impl(self.id, f);
        Ok(())
    }

    // Material and physical properties
    pub fn is_sensor(&self) -> bool {
        self.assert_valid();
        shape_is_sensor_impl(self.id)
    }
    pub fn try_is_sensor(&self) -> ApiResult<bool> {
        self.check_valid()?;
        Ok(shape_is_sensor_impl(self.id))
    }
    pub fn set_density(&mut self, density: f32, update_body_mass: bool) {
        self.assert_valid();
        shape_set_density_impl(self.id, density, update_body_mass)
    }
    pub fn try_set_density(&mut self, density: f32, update_body_mass: bool) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_density_impl(self.id, density, update_body_mass);
        Ok(())
    }
    pub fn density(&self) -> f32 {
        self.assert_valid();
        shape_density_impl(self.id)
    }
    pub fn try_density(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_density_impl(self.id))
    }
    pub fn set_friction(&mut self, friction: f32) {
        self.assert_valid();
        shape_set_friction_impl(self.id, friction)
    }
    pub fn try_set_friction(&mut self, friction: f32) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_friction_impl(self.id, friction);
        Ok(())
    }
    pub fn friction(&self) -> f32 {
        self.assert_valid();
        shape_friction_impl(self.id)
    }
    pub fn try_friction(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_friction_impl(self.id))
    }
    pub fn set_restitution(&mut self, restitution: f32) {
        self.assert_valid();
        shape_set_restitution_impl(self.id, restitution)
    }
    pub fn try_set_restitution(&mut self, restitution: f32) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_restitution_impl(self.id, restitution);
        Ok(())
    }
    pub fn restitution(&self) -> f32 {
        self.assert_valid();
        shape_restitution_impl(self.id)
    }
    pub fn try_restitution(&self) -> ApiResult<f32> {
        self.check_valid()?;
        Ok(shape_restitution_impl(self.id))
    }
    pub fn set_user_material(&mut self, material: u64) {
        self.assert_valid();
        shape_set_user_material_impl(self.id, material)
    }
    pub fn try_set_user_material(&mut self, material: u64) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_user_material_impl(self.id, material);
        Ok(())
    }
    pub fn user_material(&self) -> u64 {
        self.assert_valid();
        shape_user_material_impl(self.id)
    }
    pub fn try_user_material(&self) -> ApiResult<u64> {
        self.check_valid()?;
        Ok(shape_user_material_impl(self.id))
    }
    pub fn set_surface_material(&mut self, material: &SurfaceMaterial) {
        self.assert_valid();
        shape_set_surface_material_impl(self.id, material)
    }
    pub fn try_set_surface_material(&mut self, material: &SurfaceMaterial) -> ApiResult<()> {
        self.check_valid()?;
        shape_set_surface_material_impl(self.id, material);
        Ok(())
    }
    pub fn surface_material(&self) -> SurfaceMaterial {
        self.assert_valid();
        shape_surface_material_impl(self.id)
    }
    pub fn try_surface_material(&self) -> ApiResult<SurfaceMaterial> {
        self.check_valid()?;
        Ok(shape_surface_material_impl(self.id))
    }

    // Opaque user pointer (engine-owned)
    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as the engine may
    /// read it and that any aliasing/lifetime constraints are upheld. Box2D stores this
    /// pointer and may access it during simulation callbacks.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut core::ffi::c_void) {
        self.assert_valid();
        let _ = self.core.clear_shape_user_data(self.id);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) }
    }
    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr(&mut self, p: *mut core::ffi::c_void) -> ApiResult<()> {
        self.check_valid()?;
        let _ = self.core.clear_shape_user_data(self.id);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) }
        Ok(())
    }
    pub fn user_data_ptr(&self) -> *mut core::ffi::c_void {
        self.assert_valid();
        unsafe { ffi::b2Shape_GetUserData(self.id) }
    }

    pub fn try_user_data_ptr(&self) -> ApiResult<*mut core::ffi::c_void> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Shape_GetUserData(self.id) })
    }

    /// Set typed user data on this shape.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the shape is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        self.assert_valid();
        let p = self.core.set_shape_user_data(self.id, value);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) };
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        self.check_valid()?;
        let p = self.core.set_shape_user_data(self.id, value);
        unsafe { ffi::b2Shape_SetUserData(self.id, p) };
        Ok(())
    }

    /// Clear typed user data on this shape. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        self.assert_valid();
        let had = self.core.clear_shape_user_data(self.id);
        if had {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        had
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        self.check_valid()?;
        let had = self.core.clear_shape_user_data(self.id);
        if had {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(had)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_shape_user_data(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_shape_user_data(self.id, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_valid();
        self.core
            .try_with_shape_user_data_mut(self.id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        self.check_valid()?;
        self.core.try_with_shape_user_data_mut(self.id, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        self.assert_valid();
        let v = self
            .core
            .take_shape_user_data::<T>(self.id)
            .expect("user data type mismatch");
        if v.is_some() {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        v
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        self.check_valid()?;
        let v = self.core.take_shape_user_data::<T>(self.id)?;
        if v.is_some() {
            unsafe { ffi::b2Shape_SetUserData(self.id, core::ptr::null_mut()) };
        }
        Ok(v)
    }

    pub fn contact_data(&self) -> Vec<ContactData> {
        self.assert_valid();
        shape_contact_data_impl(self.id)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        self.assert_valid();
        shape_contact_data_into_impl(self.id, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        self.check_valid()?;
        Ok(shape_contact_data_impl(self.id))
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        shape_contact_data_into_impl(self.id, out);
        Ok(())
    }

    pub fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        self.assert_valid();
        shape_contact_data_raw_impl(self.id)
    }

    pub fn contact_data_into_raw(&self, out: &mut Vec<ffi::b2ContactData>) {
        self.assert_valid();
        shape_contact_data_into_raw_impl(self.id, out);
    }

    pub fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        self.check_valid()?;
        Ok(shape_contact_data_raw_impl(self.id))
    }

    pub fn try_contact_data_into_raw(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        shape_contact_data_into_raw_impl(self.id, out);
        Ok(())
    }

    /// Get the maximum capacity required for retrieving all the overlapped shapes on this sensor shape.
    /// Returns 0 if this shape is not a sensor.
    pub fn sensor_capacity(&self) -> i32 {
        self.assert_valid();
        shape_sensor_capacity_impl(self.id)
    }

    pub fn try_sensor_capacity(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(shape_sensor_capacity_impl(self.id))
    }

    /// Get overlapped shapes for this sensor shape. If this is not a sensor, returns empty.
    /// Note: overlaps may contain destroyed shapes; use `sensor_overlaps_valid` to filter.
    pub fn sensor_overlaps(&self) -> Vec<ShapeId> {
        self.assert_valid();
        shape_sensor_overlaps_impl(self.id)
    }

    pub fn sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        shape_sensor_overlaps_into_impl(self.id, out);
    }

    pub fn try_sensor_overlaps(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        Ok(shape_sensor_overlaps_impl(self.id))
    }

    pub fn try_sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        shape_sensor_overlaps_into_impl(self.id, out);
        Ok(())
    }

    /// Get overlapped shapes and filter out invalid (destroyed) shape ids.
    pub fn sensor_overlaps_valid(&self) -> Vec<ShapeId> {
        self.assert_valid();
        shape_sensor_overlaps_valid_impl(self.id)
    }

    pub fn sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        shape_sensor_overlaps_valid_into_impl(self.id, out);
    }

    pub fn try_sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        shape_sensor_overlaps_valid_into_impl(self.id, out);
        Ok(())
    }

    /// Destroy this shape immediately.
    ///
    /// After destruction, any previously stored `ShapeId` referring to this shape becomes invalid.
    pub fn destroy(self, update_body_mass: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Shape_IsValid(self.id) } {
            unsafe { ffi::b2DestroyShape(self.id, update_body_mass) };
            let _ = self.core.clear_shape_user_data(self.id);
            #[cfg(feature = "serialize")]
            self.core.remove_shape_flags(self.id);
        }
    }

    pub fn try_destroy(self, update_body_mass: bool) -> ApiResult<()> {
        self.check_valid()?;
        if unsafe { ffi::b2Shape_IsValid(self.id) } {
            unsafe { ffi::b2DestroyShape(self.id, update_body_mass) };
            let _ = self.core.clear_shape_user_data(self.id);
            #[cfg(feature = "serialize")]
            self.core.remove_shape_flags(self.id);
        }
        Ok(())
    }
}

/// Shape surface material parameters.
#[derive(Clone, Debug)]
pub struct SurfaceMaterial(pub(crate) ffi::b2SurfaceMaterial);

impl Default for SurfaceMaterial {
    fn default() -> Self {
        Self(unsafe { ffi::b2DefaultSurfaceMaterial() })
    }
}

impl SurfaceMaterial {
    pub fn friction(mut self, v: f32) -> Self {
        self.0.friction = v;
        self
    }
    pub fn restitution(mut self, v: f32) -> Self {
        self.0.restitution = v;
        self
    }
    pub fn rolling_resistance(mut self, v: f32) -> Self {
        self.0.rollingResistance = v;
        self
    }
    pub fn tangent_speed(mut self, v: f32) -> Self {
        self.0.tangentSpeed = v;
        self
    }
    pub fn user_material_id(mut self, v: u64) -> Self {
        self.0.userMaterialId = v;
        self
    }
    pub fn custom_color(mut self, rgba: u32) -> Self {
        self.0.customColor = rgba;
        self
    }
}

/// Shape definition with Builder pattern.
#[doc(alias = "shape_def")]
#[doc(alias = "shapedef")]
#[derive(Clone, Debug)]
pub struct ShapeDef(pub(crate) ffi::b2ShapeDef);

impl Default for ShapeDef {
    fn default() -> Self {
        Self(unsafe { ffi::b2DefaultShapeDef() })
    }
}

impl ShapeDef {
    pub fn builder() -> ShapeDefBuilder {
        ShapeDefBuilder {
            def: Self::default(),
        }
    }
}

#[doc(alias = "shape_builder")]
#[doc(alias = "shapebuilder")]
#[derive(Clone, Debug)]
pub struct ShapeDefBuilder {
    def: ShapeDef,
}

impl ShapeDefBuilder {
    /// Set the surface material (friction, restitution, etc.).
    pub fn material(mut self, mat: SurfaceMaterial) -> Self {
        self.def.0.material = mat.0;
        self
    }
    /// Density in kg/m². Affects mass.
    pub fn density(mut self, v: f32) -> Self {
        self.def.0.density = v;
        self
    }
    /// Collision filter (category/mask/group).
    pub fn filter(mut self, f: Filter) -> Self {
        self.def.0.filter = f.into_raw();
        self
    }
    /// Raw Box2D filter escape hatch.
    pub fn filter_raw(mut self, f: ffi::b2Filter) -> Self {
        self.def.0.filter = f;
        self
    }
    /// Enable user-provided filtering callback.
    ///
    /// Note: To receive custom filter calls you must also register a world-level
    /// callback via `World::set_custom_filter_callback` or `World::set_custom_filter_with_ctx`.
    pub fn enable_custom_filtering(mut self, flag: bool) -> Self {
        self.def.0.enableCustomFiltering = flag;
        self
    }
    /// Mark as sensor (no collision response).
    pub fn sensor(mut self, flag: bool) -> Self {
        self.def.0.isSensor = flag;
        self
    }
    /// Emit sensor begin/end touch events.
    pub fn enable_sensor_events(mut self, flag: bool) -> Self {
        self.def.0.enableSensorEvents = flag;
        self
    }
    /// Emit contact begin/end events.
    pub fn enable_contact_events(mut self, flag: bool) -> Self {
        self.def.0.enableContactEvents = flag;
        self
    }
    /// Emit impact hit events when above threshold.
    pub fn enable_hit_events(mut self, flag: bool) -> Self {
        self.def.0.enableHitEvents = flag;
        self
    }
    /// Emit pre-solve events (advanced).
    ///
    /// Note: To receive pre-solve events you must also register a world-level
    /// callback via `World::set_pre_solve_callback` or `World::set_pre_solve_with_ctx`.
    pub fn enable_pre_solve_events(mut self, flag: bool) -> Self {
        self.def.0.enablePreSolveEvents = flag;
        self
    }
    /// Invoke user callback on contact creation.
    pub fn invoke_contact_creation(mut self, flag: bool) -> Self {
        self.def.0.invokeContactCreation = flag;
        self
    }
    /// Recompute body mass when adding/removing this shape.
    pub fn update_body_mass(mut self, flag: bool) -> Self {
        self.def.0.updateBodyMass = flag;
        self
    }
    #[must_use]
    pub fn build(self) -> ShapeDef {
        self.def
    }
}

// serde for SurfaceMaterial and ShapeDef via lightweight representations
#[cfg(feature = "serde")]
impl serde::Serialize for SurfaceMaterial {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            friction: f32,
            restitution: f32,
            rolling_resistance: f32,
            tangent_speed: f32,
            user_material_id: u64,
            custom_color: u32,
        }
        let r = Repr {
            friction: self.0.friction,
            restitution: self.0.restitution,
            rolling_resistance: self.0.rollingResistance,
            tangent_speed: self.0.tangentSpeed,
            user_material_id: self.0.userMaterialId,
            custom_color: self.0.customColor,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SurfaceMaterial {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            friction: f32,
            #[serde(default)]
            restitution: f32,
            #[serde(default)]
            rolling_resistance: f32,
            #[serde(default)]
            tangent_speed: f32,
            #[serde(default)]
            user_material_id: u64,
            #[serde(default)]
            custom_color: u32,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut sm = SurfaceMaterial::default();
        sm = sm
            .friction(r.friction)
            .restitution(r.restitution)
            .rolling_resistance(r.rolling_resistance)
            .tangent_speed(r.tangent_speed)
            .user_material_id(r.user_material_id)
            .custom_color(r.custom_color);
        Ok(sm)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ShapeDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            material: SurfaceMaterial,
            density: f32,
            filter: Filter,
            enable_custom_filtering: bool,
            is_sensor: bool,
            enable_sensor_events: bool,
            enable_contact_events: bool,
            enable_hit_events: bool,
            enable_pre_solve_events: bool,
            invoke_contact_creation: bool,
            update_body_mass: bool,
        }
        let r = Repr {
            material: SurfaceMaterial(self.0.material),
            density: self.0.density,
            filter: Filter::from_raw(self.0.filter),
            enable_custom_filtering: self.0.enableCustomFiltering,
            is_sensor: self.0.isSensor,
            enable_sensor_events: self.0.enableSensorEvents,
            enable_contact_events: self.0.enableContactEvents,
            enable_hit_events: self.0.enableHitEvents,
            enable_pre_solve_events: self.0.enablePreSolveEvents,
            invoke_contact_creation: self.0.invokeContactCreation,
            update_body_mass: self.0.updateBodyMass,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ShapeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            material: Option<SurfaceMaterial>,
            #[serde(default)]
            density: f32,
            #[serde(default)]
            filter: Option<Filter>,
            #[serde(default)]
            enable_custom_filtering: bool,
            #[serde(default)]
            is_sensor: bool,
            #[serde(default)]
            enable_sensor_events: bool,
            #[serde(default)]
            enable_contact_events: bool,
            #[serde(default)]
            enable_hit_events: bool,
            #[serde(default)]
            enable_pre_solve_events: bool,
            #[serde(default)]
            invoke_contact_creation: bool,
            #[serde(default)]
            update_body_mass: bool,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut b = ShapeDef::builder();
        if let Some(mat) = r.material {
            b = b.material(mat);
        }
        if let Some(f) = r.filter {
            b = b.filter(f);
        }
        b = b
            .density(r.density)
            .enable_custom_filtering(r.enable_custom_filtering)
            .sensor(r.is_sensor)
            .enable_sensor_events(r.enable_sensor_events)
            .enable_contact_events(r.enable_contact_events)
            .enable_hit_events(r.enable_hit_events)
            .enable_pre_solve_events(r.enable_pre_solve_events)
            .invoke_contact_creation(r.invoke_contact_creation)
            .update_body_mass(r.update_body_mass);
        Ok(b.build())
    }
}

impl<'w> Body<'w> {
    pub fn create_circle_shape(&mut self, def: &ShapeDef, c: &Circle) -> Shape<'w> {
        crate::core::debug_checks::assert_body_valid(self.id);
        let raw = c.into_raw();
        let id = unsafe { ffi::b2CreateCircleShape(self.id, &def.0, &raw) };
        #[cfg(feature = "serialize")]
        self.core.record_shape_flags(id, &def.0);
        Shape::new(Arc::clone(&self.core), id)
    }
    pub fn create_segment_shape(&mut self, def: &ShapeDef, s: &Segment) -> Shape<'w> {
        crate::core::debug_checks::assert_body_valid(self.id);
        let raw = s.into_raw();
        let id = unsafe { ffi::b2CreateSegmentShape(self.id, &def.0, &raw) };
        #[cfg(feature = "serialize")]
        self.core.record_shape_flags(id, &def.0);
        Shape::new(Arc::clone(&self.core), id)
    }
    pub fn create_capsule_shape(&mut self, def: &ShapeDef, c: &Capsule) -> Shape<'w> {
        crate::core::debug_checks::assert_body_valid(self.id);
        let raw = c.into_raw();
        let id = unsafe { ffi::b2CreateCapsuleShape(self.id, &def.0, &raw) };
        #[cfg(feature = "serialize")]
        self.core.record_shape_flags(id, &def.0);
        Shape::new(Arc::clone(&self.core), id)
    }
    pub fn create_polygon_shape(&mut self, def: &ShapeDef, p: &Polygon) -> Shape<'w> {
        crate::core::debug_checks::assert_body_valid(self.id);
        let raw = p.into_raw();
        let id = unsafe { ffi::b2CreatePolygonShape(self.id, &def.0, &raw) };
        #[cfg(feature = "serialize")]
        self.core.record_shape_flags(id, &def.0);
        Shape::new(Arc::clone(&self.core), id)
    }

    // Convenience creators
    pub fn create_box(&mut self, def: &ShapeDef, half_w: f32, half_h: f32) -> Shape<'w> {
        self.create_polygon_shape(def, &box_polygon(half_w, half_h))
    }
    pub fn create_circle_simple(&mut self, def: &ShapeDef, radius: f32) -> Shape<'w> {
        let c = circle([0.0_f32, 0.0], radius);
        self.create_circle_shape(def, &c)
    }
    pub fn create_segment_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> Shape<'w> {
        let seg = segment(p1, p2);
        self.create_segment_shape(def, &seg)
    }
    pub fn create_capsule_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> Shape<'w> {
        let cap = capsule(c1, c2, radius);
        self.create_capsule_shape(def, &cap)
    }
    pub fn create_polygon_from_points<I, P>(
        &mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> Option<Shape<'w>>
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        let poly = crate::shapes::polygon_from_points(points, radius)?;
        Some(self.create_polygon_shape(def, &poly))
    }
}
// Shapes: module note moved to top-level doc above.
