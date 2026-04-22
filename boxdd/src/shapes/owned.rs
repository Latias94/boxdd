use super::*;
use std::rc::Rc;

/// A RAII-owned shape that is destroyed on drop.
pub struct OwnedShape {
    id: ShapeId,
    core: Arc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
    update_body_mass_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

impl ShapeRuntimeHandle for OwnedShape {
    fn shape_id(&self) -> ShapeId {
        self.id
    }

    fn shape_world_core(&self) -> &crate::core::world_core::WorldCore {
        self.core.as_ref()
    }
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
        ShapeRuntimeHandle::world_id_raw(self)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        ShapeRuntimeHandle::try_world_id_raw(self)
    }

    pub fn parent_chain_id(&self) -> Option<ChainId> {
        ShapeRuntimeHandle::parent_chain_id(self)
    }

    pub fn try_parent_chain_id(&self) -> ApiResult<Option<ChainId>> {
        ShapeRuntimeHandle::try_parent_chain_id(self)
    }

    pub fn is_valid(&self) -> bool {
        ShapeRuntimeHandle::is_valid(self)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_is_valid(self)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> ShapeId {
        self.id
    }

    pub fn is_sensor(&self) -> bool {
        ShapeRuntimeHandle::is_sensor(self)
    }

    pub fn try_is_sensor(&self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_is_sensor(self)
    }

    pub fn enable_sensor_events(&mut self, flag: bool) {
        ShapeRuntimeHandle::enable_sensor_events(self, flag)
    }

    pub fn try_enable_sensor_events(&mut self, flag: bool) -> ApiResult<()> {
        ShapeRuntimeHandle::try_enable_sensor_events(self, flag)
    }

    pub fn sensor_events_enabled(&self) -> bool {
        ShapeRuntimeHandle::sensor_events_enabled(self)
    }

    pub fn try_sensor_events_enabled(&self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_sensor_events_enabled(self)
    }

    pub fn enable_contact_events(&mut self, flag: bool) {
        ShapeRuntimeHandle::enable_contact_events(self, flag)
    }

    pub fn try_enable_contact_events(&mut self, flag: bool) -> ApiResult<()> {
        ShapeRuntimeHandle::try_enable_contact_events(self, flag)
    }

    pub fn contact_events_enabled(&self) -> bool {
        ShapeRuntimeHandle::contact_events_enabled(self)
    }

    pub fn try_contact_events_enabled(&self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_contact_events_enabled(self)
    }

    pub fn enable_pre_solve_events(&mut self, flag: bool) {
        ShapeRuntimeHandle::enable_pre_solve_events(self, flag)
    }

    pub fn try_enable_pre_solve_events(&mut self, flag: bool) -> ApiResult<()> {
        ShapeRuntimeHandle::try_enable_pre_solve_events(self, flag)
    }

    pub fn pre_solve_events_enabled(&self) -> bool {
        ShapeRuntimeHandle::pre_solve_events_enabled(self)
    }

    pub fn try_pre_solve_events_enabled(&self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_pre_solve_events_enabled(self)
    }

    pub fn enable_hit_events(&mut self, flag: bool) {
        ShapeRuntimeHandle::enable_hit_events(self, flag)
    }

    pub fn try_enable_hit_events(&mut self, flag: bool) -> ApiResult<()> {
        ShapeRuntimeHandle::try_enable_hit_events(self, flag)
    }

    pub fn hit_events_enabled(&self) -> bool {
        ShapeRuntimeHandle::hit_events_enabled(self)
    }

    pub fn try_hit_events_enabled(&self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_hit_events_enabled(self)
    }

    pub fn shape_type(&self) -> ShapeType {
        ShapeRuntimeHandle::shape_type(self)
    }

    pub fn try_shape_type(&self) -> ApiResult<ShapeType> {
        ShapeRuntimeHandle::try_shape_type(self)
    }

    pub fn shape_type_raw(&self) -> ffi::b2ShapeType {
        ShapeRuntimeHandle::shape_type_raw(self)
    }

    pub fn try_shape_type_raw(&self) -> ApiResult<ffi::b2ShapeType> {
        ShapeRuntimeHandle::try_shape_type_raw(self)
    }

    pub fn body_id(&self) -> BodyId {
        ShapeRuntimeHandle::body_id(self)
    }

    pub fn try_body_id(&self) -> ApiResult<BodyId> {
        ShapeRuntimeHandle::try_body_id(self)
    }

    // Geometry
    pub fn circle(&self) -> Circle {
        ShapeRuntimeHandle::circle(self)
    }
    pub fn segment(&self) -> Segment {
        ShapeRuntimeHandle::segment(self)
    }
    pub fn chain_segment(&self) -> ChainSegment {
        ShapeRuntimeHandle::chain_segment(self)
    }
    pub fn capsule(&self) -> Capsule {
        ShapeRuntimeHandle::capsule(self)
    }
    pub fn polygon(&self) -> Polygon {
        ShapeRuntimeHandle::polygon(self)
    }

    /// Return the closest point on this shape to `target` (in world coordinates).
    pub fn closest_point<V: Into<Vec2>>(&self, target: V) -> Vec2 {
        ShapeRuntimeHandle::closest_point(self, target)
    }

    pub fn try_closest_point<V: Into<Vec2>>(&self, target: V) -> ApiResult<Vec2> {
        ShapeRuntimeHandle::try_closest_point(self, target)
    }

    pub fn aabb(&self) -> Aabb {
        ShapeRuntimeHandle::aabb(self)
    }

    pub fn try_aabb(&self) -> ApiResult<Aabb> {
        ShapeRuntimeHandle::try_aabb(self)
    }

    pub fn test_point<V: Into<Vec2>>(&self, point: V) -> bool {
        ShapeRuntimeHandle::test_point(self, point)
    }

    pub fn try_test_point<V: Into<Vec2>>(&self, point: V) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_test_point(self, point)
    }

    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
    ) -> CastOutput {
        ShapeRuntimeHandle::ray_cast(self, origin, translation)
    }

    pub fn try_ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
    ) -> ApiResult<CastOutput> {
        ShapeRuntimeHandle::try_ray_cast(self, origin, translation)
    }

    /// Apply wind force/torque approximation to the shape.
    pub fn apply_wind<V: Into<Vec2>>(&mut self, wind: V, drag: f32, lift: f32, wake: bool) {
        ShapeRuntimeHandle::apply_wind(self, wind, drag, lift, wake)
    }

    pub fn try_apply_wind<V: Into<Vec2>>(
        &mut self,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> ApiResult<()> {
        ShapeRuntimeHandle::try_apply_wind(self, wind, drag, lift, wake)
    }

    pub fn set_circle(&mut self, c: &Circle) {
        ShapeRuntimeHandle::set_circle(self, c)
    }
    pub fn try_set_circle(&mut self, c: &Circle) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_circle(self, c)
    }
    pub fn set_segment(&mut self, s: &Segment) {
        ShapeRuntimeHandle::set_segment(self, s)
    }
    pub fn try_set_segment(&mut self, s: &Segment) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_segment(self, s)
    }
    pub fn set_capsule(&mut self, c: &Capsule) {
        ShapeRuntimeHandle::set_capsule(self, c)
    }
    pub fn try_set_capsule(&mut self, c: &Capsule) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_capsule(self, c)
    }
    pub fn set_polygon(&mut self, p: &Polygon) {
        ShapeRuntimeHandle::set_polygon(self, p)
    }
    pub fn try_set_polygon(&mut self, p: &Polygon) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_polygon(self, p)
    }

    pub fn filter(&self) -> Filter {
        ShapeRuntimeHandle::filter(self)
    }
    pub fn try_filter(&self) -> ApiResult<Filter> {
        ShapeRuntimeHandle::try_filter(self)
    }
    pub fn set_filter(&mut self, f: Filter) {
        ShapeRuntimeHandle::set_filter(self, f)
    }
    pub fn try_set_filter(&mut self, f: Filter) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_filter(self, f)
    }

    pub fn set_density(&mut self, density: f32, update_body_mass: bool) {
        ShapeRuntimeHandle::set_density(self, density, update_body_mass)
    }
    pub fn try_set_density(&mut self, density: f32, update_body_mass: bool) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_density(self, density, update_body_mass)
    }
    pub fn density(&self) -> f32 {
        ShapeRuntimeHandle::density(self)
    }
    pub fn try_density(&self) -> ApiResult<f32> {
        ShapeRuntimeHandle::try_density(self)
    }

    pub fn mass_data(&self) -> MassData {
        ShapeRuntimeHandle::mass_data(self)
    }

    pub fn try_mass_data(&self) -> ApiResult<MassData> {
        ShapeRuntimeHandle::try_mass_data(self)
    }

    pub fn set_friction(&mut self, friction: f32) {
        ShapeRuntimeHandle::set_friction(self, friction)
    }
    pub fn try_set_friction(&mut self, friction: f32) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_friction(self, friction)
    }
    pub fn friction(&self) -> f32 {
        ShapeRuntimeHandle::friction(self)
    }
    pub fn try_friction(&self) -> ApiResult<f32> {
        ShapeRuntimeHandle::try_friction(self)
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        ShapeRuntimeHandle::set_restitution(self, restitution)
    }
    pub fn try_set_restitution(&mut self, restitution: f32) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_restitution(self, restitution)
    }
    pub fn restitution(&self) -> f32 {
        ShapeRuntimeHandle::restitution(self)
    }
    pub fn try_restitution(&self) -> ApiResult<f32> {
        ShapeRuntimeHandle::try_restitution(self)
    }

    pub fn set_user_material(&mut self, material: u64) {
        ShapeRuntimeHandle::set_user_material(self, material)
    }
    pub fn try_set_user_material(&mut self, material: u64) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_user_material(self, material)
    }
    pub fn user_material(&self) -> u64 {
        ShapeRuntimeHandle::user_material(self)
    }
    pub fn try_user_material(&self) -> ApiResult<u64> {
        ShapeRuntimeHandle::try_user_material(self)
    }

    pub fn set_surface_material(&mut self, material: &SurfaceMaterial) {
        ShapeRuntimeHandle::set_surface_material(self, material)
    }
    pub fn try_set_surface_material(&mut self, material: &SurfaceMaterial) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_surface_material(self, material)
    }
    pub fn surface_material(&self) -> SurfaceMaterial {
        ShapeRuntimeHandle::surface_material(self)
    }
    pub fn try_surface_material(&self) -> ApiResult<SurfaceMaterial> {
        ShapeRuntimeHandle::try_surface_material(self)
    }

    pub fn contact_data(&self) -> Vec<ContactData> {
        ShapeRuntimeHandle::contact_data(self)
    }

    pub fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        ShapeRuntimeHandle::contact_data_into(self, out);
    }

    pub fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        ShapeRuntimeHandle::try_contact_data(self)
    }

    pub fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        ShapeRuntimeHandle::try_contact_data_into(self, out)
    }

    pub fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        ShapeRuntimeHandle::contact_data_raw(self)
    }

    pub fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        ShapeRuntimeHandle::contact_data_raw_into(self, out);
    }

    pub fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        ShapeRuntimeHandle::try_contact_data_raw(self)
    }

    pub fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        ShapeRuntimeHandle::try_contact_data_raw_into(self, out)
    }

    /// Get the maximum capacity required for retrieving all overlapped shapes on this sensor shape.
    pub fn sensor_capacity(&self) -> i32 {
        ShapeRuntimeHandle::sensor_capacity(self)
    }

    pub fn try_sensor_capacity(&self) -> ApiResult<i32> {
        ShapeRuntimeHandle::try_sensor_capacity(self)
    }

    /// Get overlapped shapes for this sensor shape. If this is not a sensor, returns empty.
    /// Note: overlaps may contain destroyed shapes; use `sensor_overlaps_valid` to filter.
    pub fn sensor_overlaps(&self) -> Vec<ShapeId> {
        ShapeRuntimeHandle::sensor_overlaps(self)
    }

    pub fn sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) {
        ShapeRuntimeHandle::sensor_overlaps_into(self, out);
    }

    pub fn try_sensor_overlaps(&self) -> ApiResult<Vec<ShapeId>> {
        ShapeRuntimeHandle::try_sensor_overlaps(self)
    }

    pub fn try_sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        ShapeRuntimeHandle::try_sensor_overlaps_into(self, out)
    }

    /// Get overlapped shapes and filter out invalid (destroyed) shape ids.
    pub fn sensor_overlaps_valid(&self) -> Vec<ShapeId> {
        ShapeRuntimeHandle::sensor_overlaps_valid(self)
    }

    pub fn try_sensor_overlaps_valid(&self) -> ApiResult<Vec<ShapeId>> {
        ShapeRuntimeHandle::try_sensor_overlaps_valid(self)
    }

    pub fn sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) {
        ShapeRuntimeHandle::sensor_overlaps_valid_into(self, out);
    }

    pub fn try_sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        ShapeRuntimeHandle::try_sensor_overlaps_valid_into(self, out)
    }

    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as the engine may
    /// read it and that any aliasing/lifetime constraints are upheld. Box2D stores this
    /// pointer and may access it during simulation callbacks.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn set_user_data_ptr_raw(&mut self, p: *mut c_void) {
        unsafe { ShapeRuntimeHandle::set_user_data_ptr_raw(self, p) }
    }
    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// Same safety contract as `set_user_data_ptr_raw`.
    ///
    /// If typed user data was previously set via `set_user_data`, it will be cleared and dropped.
    pub unsafe fn try_set_user_data_ptr_raw(&mut self, p: *mut c_void) -> ApiResult<()> {
        unsafe { ShapeRuntimeHandle::try_set_user_data_ptr_raw(self, p) }
    }
    pub fn user_data_ptr_raw(&self) -> *mut c_void {
        ShapeRuntimeHandle::user_data_ptr_raw(self)
    }

    pub fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        ShapeRuntimeHandle::try_user_data_ptr_raw(self)
    }

    /// Set typed user data on this shape.
    ///
    /// This stores a `Box<T>` internally and sets Box2D's user data pointer to it. The allocation
    /// is automatically freed when cleared or when the shape is destroyed.
    pub fn set_user_data<T: 'static>(&mut self, value: T) {
        ShapeRuntimeHandle::set_user_data(self, value);
    }

    pub fn try_set_user_data<T: 'static>(&mut self, value: T) -> ApiResult<()> {
        ShapeRuntimeHandle::try_set_user_data(self, value)
    }

    /// Clear typed user data on this shape. Returns whether any typed data was present.
    pub fn clear_user_data(&mut self) -> bool {
        ShapeRuntimeHandle::clear_user_data(self)
    }

    pub fn try_clear_user_data(&mut self) -> ApiResult<bool> {
        ShapeRuntimeHandle::try_clear_user_data(self)
    }

    pub fn with_user_data<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        ShapeRuntimeHandle::with_user_data(self, f)
    }

    pub fn try_with_user_data<T: 'static, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> ApiResult<Option<R>> {
        ShapeRuntimeHandle::try_with_user_data(self, f)
    }

    pub fn with_user_data_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        ShapeRuntimeHandle::with_user_data_mut(self, f)
    }

    pub fn try_with_user_data_mut<T: 'static, R>(
        &mut self,
        f: impl FnOnce(&mut T) -> R,
    ) -> ApiResult<Option<R>> {
        ShapeRuntimeHandle::try_with_user_data_mut(self, f)
    }

    pub fn take_user_data<T: 'static>(&mut self) -> Option<T> {
        ShapeRuntimeHandle::take_user_data(self)
    }

    pub fn try_take_user_data<T: 'static>(&mut self) -> ApiResult<Option<T>> {
        ShapeRuntimeHandle::try_take_user_data(self)
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
        if self.destroy_on_drop && unsafe { ffi::b2Shape_IsValid(raw_shape_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Shape {
                        id: self.id,
                        update_body_mass,
                    });
            } else {
                unsafe { ffi::b2DestroyShape(raw_shape_id(self.id), update_body_mass) };
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
        if self.destroy_on_drop && unsafe { ffi::b2Shape_IsValid(raw_shape_id(self.id)) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Shape {
                        id: self.id,
                        update_body_mass: self.update_body_mass_on_drop,
                    });
            } else {
                unsafe {
                    ffi::b2DestroyShape(raw_shape_id(self.id), self.update_body_mass_on_drop)
                };
                let _ = self.core.clear_shape_user_data(self.id);
                #[cfg(feature = "serialize")]
                self.core.remove_shape_flags(self.id);
            }
        }
    }
}
