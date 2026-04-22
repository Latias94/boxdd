use super::*;

#[inline]
fn assert_world_step_args_valid(time_step: f32, sub_steps: i32) {
    assert!(
        crate::is_valid_float(time_step),
        "time_step must be finite, got {time_step}"
    );
    assert!(sub_steps > 0, "sub_steps must be > 0, got {sub_steps}");
}

#[inline]
fn check_world_step_args_valid(time_step: f32, sub_steps: i32) -> crate::error::ApiResult<()> {
    if crate::is_valid_float(time_step) && sub_steps > 0 {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

impl World {
    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    pub fn step(&mut self, time_step: f32, sub_steps: i32) {
        crate::core::callback_state::assert_not_in_callback();
        assert_world_step_args_valid(time_step, sub_steps);
        // Prepare panic forwarding for callbacks invoked during the FFI call.
        self.core
            .callback_panicked
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self
            .core
            .callback_panic
            .lock()
            .expect("callback_panic mutex poisoned") = None;
        // SAFETY: valid world id managed by RAII
        unsafe { ffi::b2World_Step(self.raw(), time_step, sub_steps) };

        // Flush deferred destroys scheduled from callbacks.
        self.core.process_deferred_destroys();

        if self
            .core
            .callback_panicked
            .load(std::sync::atomic::Ordering::Relaxed)
            && let Some(payload) = self
                .core
                .callback_panic
                .lock()
                .expect("callback_panic mutex poisoned")
                .take()
        {
            std::panic::resume_unwind(payload);
        }
    }

    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_step(&mut self, time_step: f32, sub_steps: i32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        check_world_step_args_valid(time_step, sub_steps)?;
        self.step(time_step, sub_steps);
        Ok(())
    }

    /// Flush deferred destroys scheduled from Box2D callbacks.
    ///
    /// Most users don't need to call this because `World::step`, event view helpers
    /// (`with_*_events_view`), and debug draw helpers flush automatically. This is useful if you
    /// drop `Owned*` handles during callbacks but want to reclaim resources without stepping the
    /// simulation again.
    pub fn flush_deferred_destroys(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.core.process_deferred_destroys();
    }

    /// Flush deferred destroys scheduled from Box2D callbacks.
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_flush_deferred_destroys(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.flush_deferred_destroys();
        Ok(())
    }

    /// Set gravity vector.
    pub fn set_gravity<V: Into<Vec2>>(&mut self, g: V) {
        crate::core::callback_state::assert_not_in_callback();
        let gravity = g.into();
        assert_world_gravity_valid(gravity);
        let gv: ffi::b2Vec2 = gravity.into_raw();
        unsafe { ffi::b2World_SetGravity(self.raw(), gv) };
    }

    pub fn try_set_gravity<V: Into<Vec2>>(&mut self, g: V) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        let gravity = g.into();
        check_world_gravity_valid(gravity)?;
        let gv: ffi::b2Vec2 = gravity.into_raw();
        unsafe { ffi::b2World_SetGravity(self.raw(), gv) };
        Ok(())
    }

    /// Get current gravity vector.
    pub fn gravity(&self) -> Vec2 {
        world_gravity_checked_impl(self.raw())
    }

    pub fn try_gravity(&self) -> crate::error::ApiResult<Vec2> {
        try_world_gravity_impl(self.raw())
    }

    /// World counters snapshot (sizes, tree heights, etc.).
    pub fn counters(&self) -> Counters {
        world_counters_checked_impl(self.raw())
    }

    pub fn try_counters(&self) -> crate::error::ApiResult<Counters> {
        try_world_counters_impl(self.raw())
    }

    /// World profile snapshot with per-stage timing in milliseconds from the last completed step.
    pub fn profile(&self) -> Profile {
        world_profile_checked_impl(self.raw())
    }

    pub fn try_profile(&self) -> crate::error::ApiResult<Profile> {
        try_world_profile_impl(self.raw())
    }

    /// Get number of awake bodies.
    pub fn awake_body_count(&self) -> i32 {
        world_awake_body_count_checked_impl(self.raw())
    }

    pub fn try_awake_body_count(&self) -> crate::error::ApiResult<i32> {
        try_world_awake_body_count_impl(self.raw())
    }

    // Runtime configuration helpers mirroring WorldDef fields
    pub fn enable_sleeping(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableSleeping(self.raw(), flag) }
    }

    pub fn try_enable_sleeping(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableSleeping(self.raw(), flag) }
        Ok(())
    }

    pub fn enable_continuous(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableContinuous(self.raw(), flag) }
    }

    pub fn try_enable_continuous(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableContinuous(self.raw(), flag) }
        Ok(())
    }

    /// Enable or disable constraint warm starting at runtime.
    ///
    /// Warm starting seeds the solver with accumulated impulses from the previous
    /// step to improve stability and convergence. Disabling this is only useful
    /// for experiments and will significantly reduce stability in most scenes.
    pub fn enable_warm_starting(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableWarmStarting(self.raw(), flag) }
    }

    pub fn try_enable_warm_starting(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableWarmStarting(self.raw(), flag) }
        Ok(())
    }

    pub fn set_restitution_threshold(&mut self, value: f32) {
        crate::core::callback_state::assert_not_in_callback();
        assert_non_negative_finite_world_scalar("restitution_threshold", value);
        unsafe { ffi::b2World_SetRestitutionThreshold(self.raw(), value) }
    }

    pub fn try_set_restitution_threshold(&mut self, value: f32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        check_non_negative_finite_world_scalar(value)?;
        unsafe { ffi::b2World_SetRestitutionThreshold(self.raw(), value) }
        Ok(())
    }

    pub fn set_hit_event_threshold(&mut self, value: f32) {
        crate::core::callback_state::assert_not_in_callback();
        assert_non_negative_finite_world_scalar("hit_event_threshold", value);
        unsafe { ffi::b2World_SetHitEventThreshold(self.raw(), value) }
    }

    pub fn try_set_hit_event_threshold(&mut self, value: f32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        check_non_negative_finite_world_scalar(value)?;
        unsafe { ffi::b2World_SetHitEventThreshold(self.raw(), value) }
        Ok(())
    }

    pub fn set_contact_tuning(&mut self, hertz: f32, damping_ratio: f32, push_speed: f32) {
        crate::core::callback_state::assert_not_in_callback();
        assert_non_negative_finite_world_scalar("contact_hertz", hertz);
        assert_non_negative_finite_world_scalar("contact_damping_ratio", damping_ratio);
        assert_non_negative_finite_world_scalar("contact_speed", push_speed);
        unsafe { ffi::b2World_SetContactTuning(self.raw(), hertz, damping_ratio, push_speed) }
    }

    pub fn try_set_contact_tuning(
        &mut self,
        hertz: f32,
        damping_ratio: f32,
        push_speed: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        check_non_negative_finite_world_scalar(hertz)?;
        check_non_negative_finite_world_scalar(damping_ratio)?;
        check_non_negative_finite_world_scalar(push_speed)?;
        unsafe { ffi::b2World_SetContactTuning(self.raw(), hertz, damping_ratio, push_speed) }
        Ok(())
    }

    /// Enable or disable speculative collision handling at runtime.
    pub fn enable_speculative(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableSpeculative(self.raw(), flag) }
    }

    pub fn try_enable_speculative(&mut self, flag: bool) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_EnableSpeculative(self.raw(), flag) }
        Ok(())
    }

    pub fn set_maximum_linear_speed(&mut self, v: f32) {
        crate::core::callback_state::assert_not_in_callback();
        assert_positive_finite_world_scalar("maximum_linear_speed", v);
        unsafe { ffi::b2World_SetMaximumLinearSpeed(self.raw(), v) }
    }

    pub fn try_set_maximum_linear_speed(&mut self, v: f32) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        check_positive_finite_world_scalar(v)?;
        unsafe { ffi::b2World_SetMaximumLinearSpeed(self.raw(), v) }
        Ok(())
    }

    pub fn is_sleeping_enabled(&self) -> bool {
        world_is_sleeping_enabled_checked_impl(self.raw())
    }

    pub fn try_is_sleeping_enabled(&self) -> crate::error::ApiResult<bool> {
        try_world_is_sleeping_enabled_impl(self.raw())
    }

    pub fn is_continuous_enabled(&self) -> bool {
        world_is_continuous_enabled_checked_impl(self.raw())
    }

    pub fn try_is_continuous_enabled(&self) -> crate::error::ApiResult<bool> {
        try_world_is_continuous_enabled_impl(self.raw())
    }

    /// Returns true if constraint warm starting is enabled.
    pub fn is_warm_starting_enabled(&self) -> bool {
        world_is_warm_starting_enabled_checked_impl(self.raw())
    }

    pub fn try_is_warm_starting_enabled(&self) -> crate::error::ApiResult<bool> {
        try_world_is_warm_starting_enabled_impl(self.raw())
    }

    pub fn restitution_threshold(&self) -> f32 {
        world_restitution_threshold_checked_impl(self.raw())
    }

    pub fn try_restitution_threshold(&self) -> crate::error::ApiResult<f32> {
        try_world_restitution_threshold_impl(self.raw())
    }

    pub fn hit_event_threshold(&self) -> f32 {
        world_hit_event_threshold_checked_impl(self.raw())
    }

    pub fn try_hit_event_threshold(&self) -> crate::error::ApiResult<f32> {
        try_world_hit_event_threshold_impl(self.raw())
    }

    pub fn maximum_linear_speed(&self) -> f32 {
        world_maximum_linear_speed_checked_impl(self.raw())
    }

    pub fn try_maximum_linear_speed(&self) -> crate::error::ApiResult<f32> {
        try_world_maximum_linear_speed_impl(self.raw())
    }
}
