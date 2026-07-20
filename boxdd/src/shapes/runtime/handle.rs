use super::*;

pub(crate) trait ShapeRuntimeHandle {
    fn shape_id(&self) -> ShapeId;
    fn shape_world_core(&self) -> &crate::core::world_core::WorldCore;

    #[inline]
    #[track_caller]
    fn assert_valid(&self) {
        self.check_valid()
            .expect("shape handle is unavailable, foreign, or invalid");
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.shape_world_core().check_shape(self.shape_id())
    }

    fn world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        shape_world_id_impl(self.shape_id())
    }

    fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(shape_world_id_impl(self.shape_id()))
    }

    fn parent_chain_id(&self) -> Option<ChainId> {
        self.assert_valid();
        shape_parent_chain_id_in_impl(self.shape_world_core(), self.shape_id())
            .expect("Box2D returned an invalid parent chain id for a validated shape")
    }

    fn try_parent_chain_id(&self) -> ApiResult<Option<ChainId>> {
        self.check_valid()?;
        shape_parent_chain_id_in_impl(self.shape_world_core(), self.shape_id())
    }

    fn is_valid(&self) -> bool {
        self.try_is_valid()
            .expect("shape handle is unavailable or foreign")
    }

    fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        let core = self.shape_world_core();
        core.check_available()?;
        let id = self.shape_id();
        if id.brand() != core.brand() {
            return Err(ApiError::WrongWorld);
        }
        Ok(shape_is_valid_impl(id))
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
        shape_user_data_ptr_raw_checked_impl(self.shape_world_core(), self.shape_id())
    }

    fn try_user_data_ptr_raw(&self) -> ApiResult<*mut c_void> {
        try_shape_user_data_ptr_raw_impl(self.shape_world_core(), self.shape_id())
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
        self.assert_valid();
        shape_contact_data_in_impl(self.shape_world_core(), self.shape_id())
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    fn contact_data_into(&self, out: &mut Vec<ContactData>) {
        self.assert_valid();
        shape_contact_data_into_in_impl(self.shape_world_core(), self.shape_id(), out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    fn try_contact_data(&self) -> ApiResult<Vec<ContactData>> {
        self.check_valid()?;
        shape_contact_data_in_impl(self.shape_world_core(), self.shape_id())
    }

    fn try_contact_data_into(&self, out: &mut Vec<ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        shape_contact_data_into_in_impl(self.shape_world_core(), self.shape_id(), out)
    }

    fn contact_data_raw(&self) -> Vec<ffi::b2ContactData> {
        self.assert_valid();
        shape_contact_data_raw_impl(self.shape_id()).expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    fn contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) {
        self.assert_valid();
        shape_contact_data_raw_into_impl(self.shape_id(), out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    fn try_contact_data_raw(&self) -> ApiResult<Vec<ffi::b2ContactData>> {
        self.check_valid()?;
        shape_contact_data_raw_impl(self.shape_id())
    }

    fn try_contact_data_raw_into(&self, out: &mut Vec<ffi::b2ContactData>) -> ApiResult<()> {
        self.check_valid()?;
        shape_contact_data_raw_into_impl(self.shape_id(), out)
    }

    fn sensor_capacity(&self) -> i32 {
        self.assert_valid();
        shape_sensor_capacity_impl(self.shape_id())
    }

    fn try_sensor_capacity(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(shape_sensor_capacity_impl(self.shape_id()))
    }

    fn sensor_overlaps(&self) -> Vec<ShapeId> {
        self.assert_valid();
        shape_sensor_overlaps_in_impl(self.shape_world_core().brand(), self.shape_id())
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    fn sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        shape_sensor_overlaps_into_in_impl(self.shape_world_core().brand(), self.shape_id(), out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    fn try_sensor_overlaps(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        shape_sensor_overlaps_in_impl(self.shape_world_core().brand(), self.shape_id())
    }

    fn try_sensor_overlaps_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        shape_sensor_overlaps_into_in_impl(self.shape_world_core().brand(), self.shape_id(), out)
    }

    fn sensor_overlaps_valid(&self) -> Vec<ShapeId> {
        self.assert_valid();
        shape_sensor_overlaps_valid_in_impl(self.shape_world_core().brand(), self.shape_id())
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    fn try_sensor_overlaps_valid(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        shape_sensor_overlaps_valid_in_impl(self.shape_world_core().brand(), self.shape_id())
    }

    fn sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        shape_sensor_overlaps_valid_into_in_impl(
            self.shape_world_core().brand(),
            self.shape_id(),
            out,
        )
        .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    fn try_sensor_overlaps_valid_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        shape_sensor_overlaps_valid_into_in_impl(
            self.shape_world_core().brand(),
            self.shape_id(),
            out,
        )
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
        shape_body_id_in_impl(self.shape_world_core(), self.shape_id())
            .expect("Box2D returned an invalid body id for a validated shape")
    }

    fn try_body_id(&self) -> ApiResult<BodyId> {
        self.check_valid()?;
        shape_body_id_in_impl(self.shape_world_core(), self.shape_id())
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

    fn closest_point(&self, target: Position) -> Position {
        self.assert_valid();
        assert_shape_world_point_in_local_range("target", self.shape_id(), target);
        shape_closest_point_impl(self.shape_id(), target)
    }

    fn try_closest_point(&self, target: Position) -> ApiResult<Position> {
        self.check_valid()?;
        check_shape_world_point_in_local_range(self.shape_id(), target)?;
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

    fn test_point(&self, point: Position) -> bool {
        self.assert_valid();
        assert_shape_world_point_in_local_range("point", self.shape_id(), point);
        shape_test_point_impl(self.shape_id(), point)
    }

    fn try_test_point(&self, point: Position) -> ApiResult<bool> {
        self.check_valid()?;
        check_shape_world_point_in_local_range(self.shape_id(), point)?;
        Ok(shape_test_point_impl(self.shape_id(), point))
    }

    fn ray_cast<VT: Into<Vec2>>(&self, origin: Position, translation: VT) -> WorldCastOutput {
        self.assert_valid();
        let origin = crate::body::assert_valid_body_position("origin", origin);
        let translation = translation.into();
        assert_shape_vec2_valid("translation", translation);
        self.assert_valid();
        assert_shape_world_point_in_local_range("origin", self.shape_id(), origin);
        shape_ray_cast_impl(self.shape_id(), origin, translation)
    }

    fn try_ray_cast<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
    ) -> ApiResult<WorldCastOutput> {
        self.check_valid()?;
        let origin = crate::body::check_valid_body_position(origin)?;
        let translation = translation.into();
        check_shape_vec2_valid(translation)?;
        self.check_valid()?;
        check_shape_world_point_in_local_range(self.shape_id(), origin)?;
        Ok(shape_ray_cast_impl(self.shape_id(), origin, translation))
    }

    fn apply_wind<V: Into<Vec2>>(&mut self, wind: V, drag: f32, lift: f32, wake: bool) {
        self.assert_valid();
        let wind = wind.into();
        assert_shape_vec2_valid("wind", wind);
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
        let wind = wind.into();
        check_shape_vec2_valid(wind)?;
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
        self.assert_valid();
        assert_non_negative_finite_shape_scalar("density", density);
        shape_set_density_impl(self.shape_id(), density, update_body_mass)
    }

    fn try_set_density(&mut self, density: f32, update_body_mass: bool) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_shape_scalar(density)?;
        shape_set_density_impl(self.shape_id(), density, update_body_mass);
        Ok(())
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
        self.assert_valid();
        assert_non_negative_finite_shape_scalar("friction", friction);
        shape_set_friction_impl(self.shape_id(), friction)
    }

    fn try_set_friction(&mut self, friction: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_shape_scalar(friction)?;
        shape_set_friction_impl(self.shape_id(), friction);
        Ok(())
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
        self.assert_valid();
        assert_non_negative_finite_shape_scalar("restitution", restitution);
        shape_set_restitution_impl(self.shape_id(), restitution)
    }

    fn try_set_restitution(&mut self, restitution: f32) -> ApiResult<()> {
        self.check_valid()?;
        check_non_negative_finite_shape_scalar(restitution)?;
        shape_set_restitution_impl(self.shape_id(), restitution);
        Ok(())
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
