use super::*;

impl World {
    pub fn set_body_mass_data(&mut self, body: BodyId, mass_data: MassData) {
        assert_body_target(&self.core, body);
        crate::body::assert_mass_data_valid(mass_data);
        unsafe { ffi::b2Body_SetMassData(raw_body_id(body), mass_data.into_raw()) };
    }

    pub fn try_set_body_mass_data(
        &mut self,
        body: BodyId,
        mass_data: MassData,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::check_mass_data_valid(mass_data)?;
        unsafe { ffi::b2Body_SetMassData(raw_body_id(body), mass_data.into_raw()) };
        Ok(())
    }

    pub fn body_apply_mass_from_shapes(&mut self, body: BodyId) {
        assert_body_target(&self.core, body);
        unsafe { ffi::b2Body_ApplyMassFromShapes(raw_body_id(body)) };
    }

    pub fn try_body_apply_mass_from_shapes(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        unsafe { ffi::b2Body_ApplyMassFromShapes(raw_body_id(body)) };
        Ok(())
    }

    pub fn set_body_target_transform(
        &mut self,
        body: BodyId,
        target: WorldTransform,
        time_step: f32,
        wake: bool,
    ) {
        assert_body_target(&self.core, body);
        let (target, time_step) = crate::body::assert_valid_body_target_motion(
            target,
            time_step,
            crate::body::body_world_center_of_mass_impl(body),
            crate::body::body_rotation_impl(body),
            crate::body::body_local_center_of_mass_impl(body),
        );
        unsafe {
            ffi::b2Body_SetTargetTransform(raw_body_id(body), target.into_raw(), time_step, wake)
        };
    }

    pub fn try_set_body_target_transform(
        &mut self,
        body: BodyId,
        target: WorldTransform,
        time_step: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let (target, time_step) = crate::body::check_valid_body_target_motion(
            target,
            time_step,
            crate::body::body_world_center_of_mass_impl(body),
            crate::body::body_rotation_impl(body),
            crate::body::body_local_center_of_mass_impl(body),
        )?;
        unsafe {
            ffi::b2Body_SetTargetTransform(raw_body_id(body), target.into_raw(), time_step, wake)
        };
        Ok(())
    }

    /// Set a body's world position and rotation (angle in radians) by id.
    pub fn set_body_position_and_rotation<V: Into<Position>>(
        &mut self,
        body: BodyId,
        p: V,
        angle_radians: f32,
    ) {
        assert_body_target(&self.core, body);
        let p = crate::body::assert_valid_body_position("position", p.into());
        let angle_radians = crate::body::assert_valid_body_float("angle_radians", angle_radians);
        assert_body_target(&self.core, body);
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Pos = p.into_raw();
        unsafe { ffi::b2Body_SetTransform(raw_body_id(body), pos, rot) };
    }

    pub fn try_set_body_position_and_rotation<V: Into<Position>>(
        &mut self,
        body: BodyId,
        p: V,
        angle_radians: f32,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let p = crate::body::check_valid_body_position(p.into())?;
        let angle_radians = crate::body::check_valid_body_float(angle_radians)?;
        check_body_target(&self.core, body)?;
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Pos = p.into_raw();
        unsafe { ffi::b2Body_SetTransform(raw_body_id(body), pos, rot) };
        Ok(())
    }

    /// Set a body's linear velocity by id.
    pub fn set_body_linear_velocity<V: Into<Vec2>>(&mut self, body: BodyId, v: V) {
        assert_body_target(&self.core, body);
        let v = crate::body::assert_valid_body_vec2("velocity", v.into());
        assert_body_target(&self.core, body);
        let vv: ffi::b2Vec2 = v.into_raw();
        unsafe { ffi::b2Body_SetLinearVelocity(raw_body_id(body), vv) }
    }

    pub fn try_set_body_linear_velocity<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        v: V,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let v = crate::body::check_valid_body_vec2(v.into())?;
        check_body_target(&self.core, body)?;
        let vv: ffi::b2Vec2 = v.into_raw();
        unsafe { ffi::b2Body_SetLinearVelocity(raw_body_id(body), vv) }
        Ok(())
    }

    /// Set a body's angular velocity by id.
    pub fn set_body_angular_velocity(&mut self, body: BodyId, w: f32) {
        assert_body_target(&self.core, body);
        let w = crate::body::assert_valid_body_float("angular_velocity", w);
        unsafe { ffi::b2Body_SetAngularVelocity(raw_body_id(body), w) }
    }

    pub fn try_set_body_angular_velocity(
        &mut self,
        body: BodyId,
        w: f32,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let w = crate::body::check_valid_body_float(w)?;
        unsafe { ffi::b2Body_SetAngularVelocity(raw_body_id(body), w) }
        Ok(())
    }

    pub fn body_enable_sleep(&mut self, body: BodyId, flag: bool) {
        assert_body_target(&self.core, body);
        crate::body::body_enable_sleep_impl(body, flag)
    }

    pub fn try_body_enable_sleep(
        &mut self,
        body: BodyId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_enable_sleep_impl(body, flag);
        Ok(())
    }

    pub fn body_is_sleep_enabled(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_sleep_enabled_impl(body)
    }

    pub fn try_body_is_sleep_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_sleep_enabled_impl(body))
    }

    pub fn body_sleep_threshold(&self, body: BodyId) -> f32 {
        assert_body_target(&self.core, body);
        crate::body::body_sleep_threshold_impl(body)
    }

    pub fn try_body_sleep_threshold(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_sleep_threshold_impl(body))
    }

    pub fn set_body_sleep_threshold(&mut self, body: BodyId, sleep_threshold: f32) {
        assert_body_target(&self.core, body);
        crate::body::assert_non_negative_finite_body_scalar("sleep_threshold", sleep_threshold);
        crate::body::body_set_sleep_threshold_impl(body, sleep_threshold)
    }

    pub fn try_set_body_sleep_threshold(
        &mut self,
        body: BodyId,
        sleep_threshold: f32,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::check_non_negative_finite_body_scalar(sleep_threshold)?;
        crate::body::body_set_sleep_threshold_impl(body, sleep_threshold);
        Ok(())
    }

    pub fn body_is_awake(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_awake_impl(body)
    }

    pub fn try_body_is_awake(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_awake_impl(body))
    }

    pub fn set_body_awake(&mut self, body: BodyId, awake: bool) {
        assert_body_target(&self.core, body);
        crate::body::body_set_awake_impl(body, awake)
    }

    pub fn try_set_body_awake(&mut self, body: BodyId, awake: bool) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_set_awake_impl(body, awake);
        Ok(())
    }

    pub fn body_is_enabled(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_enabled_impl(body)
    }

    pub fn try_body_is_enabled(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_enabled_impl(body))
    }

    pub fn body_enable_contact_events(&mut self, body: BodyId, flag: bool) {
        assert_body_target(&self.core, body);
        crate::body::body_enable_contact_events_impl(body, flag)
    }

    pub fn try_body_enable_contact_events(
        &mut self,
        body: BodyId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_enable_contact_events_impl(body, flag);
        Ok(())
    }

    pub fn body_enable_hit_events(&mut self, body: BodyId, flag: bool) {
        assert_body_target(&self.core, body);
        crate::body::body_enable_hit_events_impl(body, flag)
    }

    pub fn try_body_enable_hit_events(
        &mut self,
        body: BodyId,
        flag: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_enable_hit_events_impl(body, flag);
        Ok(())
    }

    /// Get the current motion locks for a body.
    pub fn body_motion_locks(&self, body: BodyId) -> MotionLocks {
        assert_body_target(&self.core, body);
        crate::body::body_motion_locks_impl(body)
    }

    pub fn try_body_motion_locks(&self, body: BodyId) -> crate::error::ApiResult<MotionLocks> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_motion_locks_impl(body))
    }

    /// Set motion locks (translation/rotation constraints) for a body.
    pub fn set_body_motion_locks(&mut self, body: BodyId, locks: MotionLocks) {
        assert_body_target(&self.core, body);
        unsafe { ffi::b2Body_SetMotionLocks(raw_body_id(body), locks.into_raw()) }
    }

    pub fn try_set_body_motion_locks(
        &mut self,
        body: BodyId,
        locks: MotionLocks,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        unsafe { ffi::b2Body_SetMotionLocks(raw_body_id(body), locks.into_raw()) }
        Ok(())
    }

    /// Apply a linear impulse to the center of mass of a body.
    pub fn body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) {
        assert_body_target(&self.core, body);
        let impulse = crate::body::assert_valid_body_vec2("impulse", impulse.into());
        assert_body_target(&self.core, body);
        let i: ffi::b2Vec2 = impulse.into_raw();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(raw_body_id(body), i, wake) };
    }

    pub fn try_body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let impulse = crate::body::check_valid_body_vec2(impulse.into())?;
        check_body_target(&self.core, body)?;
        let i: ffi::b2Vec2 = impulse.into_raw();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(raw_body_id(body), i, wake) };
        Ok(())
    }

    /// Apply an angular impulse to a body.
    pub fn body_apply_angular_impulse(&mut self, body: BodyId, impulse: f32, wake: bool) {
        assert_body_target(&self.core, body);
        let impulse = crate::body::assert_valid_body_float("impulse", impulse);
        unsafe { ffi::b2Body_ApplyAngularImpulse(raw_body_id(body), impulse, wake) };
    }

    pub fn try_body_apply_angular_impulse(
        &mut self,
        body: BodyId,
        impulse: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let impulse = crate::body::check_valid_body_float(impulse)?;
        unsafe { ffi::b2Body_ApplyAngularImpulse(raw_body_id(body), impulse, wake) };
        Ok(())
    }

    /// Clear accumulated forces and torque on a body (usually only needed before stepping).
    pub fn body_clear_forces(&mut self, body: BodyId) {
        assert_body_target(&self.core, body);
        unsafe { ffi::b2Body_ClearForces(raw_body_id(body)) };
    }

    pub fn try_body_clear_forces(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        unsafe { ffi::b2Body_ClearForces(raw_body_id(body)) };
        Ok(())
    }

    /// Wake all touching bodies.
    pub fn body_wake_touching(&mut self, body: BodyId) {
        assert_body_target(&self.core, body);
        unsafe { ffi::b2Body_WakeTouching(raw_body_id(body)) };
    }

    pub fn try_body_wake_touching(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        unsafe { ffi::b2Body_WakeTouching(raw_body_id(body)) };
        Ok(())
    }

    /// Set a body's type by id.
    pub fn set_body_type(&mut self, body: BodyId, t: BodyType) {
        assert_body_target(&self.core, body);
        unsafe { ffi::b2Body_SetType(raw_body_id(body), t.into_raw()) }
    }

    pub fn try_set_body_type(&mut self, body: BodyId, t: BodyType) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        unsafe { ffi::b2Body_SetType(raw_body_id(body), t.into_raw()) }
        Ok(())
    }

    /// Enable a body by id.
    pub fn enable_body(&mut self, body: BodyId) {
        assert_body_target(&self.core, body);
        crate::body::body_enable_impl(body)
    }

    pub fn try_enable_body(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_enable_impl(body);
        Ok(())
    }

    /// Disable a body by id.
    pub fn disable_body(&mut self, body: BodyId) {
        assert_body_target(&self.core, body);
        crate::body::body_disable_impl(body)
    }

    pub fn try_disable_body(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_disable_impl(body);
        Ok(())
    }

    pub fn body_is_bullet(&self, body: BodyId) -> bool {
        assert_body_target(&self.core, body);
        crate::body::body_is_bullet_impl(body)
    }

    pub fn try_body_is_bullet(&self, body: BodyId) -> crate::error::ApiResult<bool> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_is_bullet_impl(body))
    }

    pub fn set_body_bullet(&mut self, body: BodyId, bullet: bool) {
        assert_body_target(&self.core, body);
        crate::body::body_set_bullet_impl(body, bullet)
    }

    pub fn try_set_body_bullet(
        &mut self,
        body: BodyId,
        bullet: bool,
    ) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        crate::body::body_set_bullet_impl(body, bullet);
        Ok(())
    }

    /// Set a body's name by id.
    pub fn set_body_name(&mut self, body: BodyId, name: &str) {
        assert_body_target(&self.core, body);
        let cs = crate::body::assert_valid_body_name(name);
        crate::body::body_set_name_impl(body, cs.as_c_str())
    }

    pub fn try_set_body_name(&mut self, body: BodyId, name: &str) -> crate::error::ApiResult<()> {
        check_body_target(&self.core, body)?;
        let cs = crate::body::check_valid_body_name(name)?;
        crate::body::body_set_name_impl(body, cs.as_c_str());
        Ok(())
    }

    pub fn body_name(&self, body: BodyId) -> Option<String> {
        assert_body_target(&self.core, body);
        crate::body::body_name_impl(body)
    }

    pub fn try_body_name(&self, body: BodyId) -> crate::error::ApiResult<Option<String>> {
        check_body_target(&self.core, body)?;
        Ok(crate::body::body_name_impl(body))
    }
}
