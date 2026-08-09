use super::reads::*;
use super::*;

#[inline]
fn check_world_step_args_valid(time_step: f32, sub_steps: i32) -> crate::error::Result<()> {
    if !crate::is_valid_float(time_step) || time_step < 0.0 {
        return Err(crate::error::Error::invalid_argument(
            "World::step",
            "time_step",
            "a finite value greater than or equal to zero",
        ));
    }
    if sub_steps <= 0 {
        return Err(crate::error::Error::invalid_argument(
            "World::step",
            "sub_steps",
            "an integer greater than zero",
        ));
    }
    Ok(())
}

pub(crate) fn world_set_gravity<V: Into<Vec2>>(
    world: crate::world::WorldCall<'_>,
    gravity: V,
) -> crate::error::Result<()> {
    let gravity = gravity.into();
    check_world_gravity_valid("World::set_gravity", "gravity", gravity)?;
    unsafe { ffi::b2World_SetGravity(world.raw_world(), gravity.into_raw()) };
    Ok(())
}

pub(crate) fn world_enable_sleeping(
    world: crate::world::WorldCall<'_>,
    flag: bool,
) -> crate::error::Result<()> {
    unsafe { ffi::b2World_EnableSleeping(world.raw_world(), flag) };
    Ok(())
}

pub(crate) fn world_enable_continuous(
    world: crate::world::WorldCall<'_>,
    flag: bool,
) -> crate::error::Result<()> {
    unsafe { ffi::b2World_EnableContinuous(world.raw_world(), flag) };
    Ok(())
}

pub(crate) fn world_enable_warm_starting(
    world: crate::world::WorldCall<'_>,
    flag: bool,
) -> crate::error::Result<()> {
    unsafe { ffi::b2World_EnableWarmStarting(world.raw_world(), flag) };
    Ok(())
}

pub(crate) fn world_set_restitution_threshold(
    world: crate::world::WorldCall<'_>,
    value: f32,
) -> crate::error::Result<()> {
    check_non_negative_finite_world_scalar("World::set_restitution_threshold", "value", value)?;
    unsafe { ffi::b2World_SetRestitutionThreshold(world.raw_world(), value) };
    Ok(())
}

pub(crate) fn world_set_hit_event_threshold(
    world: crate::world::WorldCall<'_>,
    value: f32,
) -> crate::error::Result<()> {
    check_non_negative_finite_world_scalar("World::set_hit_event_threshold", "value", value)?;
    unsafe { ffi::b2World_SetHitEventThreshold(world.raw_world(), value) };
    Ok(())
}

pub(crate) fn world_set_contact_tuning(
    world: crate::world::WorldCall<'_>,
    hertz: f32,
    damping_ratio: f32,
    push_speed: f32,
) -> crate::error::Result<()> {
    check_non_negative_finite_world_scalar("World::set_contact_tuning", "hertz", hertz)?;
    check_non_negative_finite_world_scalar(
        "World::set_contact_tuning",
        "damping_ratio",
        damping_ratio,
    )?;
    check_non_negative_finite_world_scalar("World::set_contact_tuning", "push_speed", push_speed)?;
    unsafe { ffi::b2World_SetContactTuning(world.raw_world(), hertz, damping_ratio, push_speed) };
    Ok(())
}

pub(crate) fn world_set_contact_recycle_distance(
    world: crate::world::WorldCall<'_>,
    distance: f32,
) -> crate::error::Result<()> {
    check_non_negative_finite_world_scalar(
        "World::set_contact_recycle_distance",
        "distance",
        distance,
    )?;
    unsafe { ffi::b2World_SetContactRecycleDistance(world.raw_world(), distance) };
    Ok(())
}

pub(crate) fn world_set_maximum_linear_speed(
    world: crate::world::WorldCall<'_>,
    value: f32,
) -> crate::error::Result<()> {
    check_positive_finite_world_linear_speed("World::set_maximum_linear_speed", "value", value)?;
    unsafe { ffi::b2World_SetMaximumLinearSpeed(world.raw_world(), value) };
    Ok(())
}

impl World {
    fn step_validated<'world>(
        &'world mut self,
        time_step: f32,
        sub_steps: i32,
    ) -> crate::error::Result<(
        crate::events::PendingStepPublication<'world>,
        Option<crate::Error>,
    )> {
        // Surface any failure left by an exceptional prior worker boundary before changing the
        // contact epoch or completed-step publication state.
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.core.worker_callbacks.begin_call()?;
        }

        // Prepare every worker-visible callback artifact before advancing any Rust epoch or
        // mutating native state. A failed identity snapshot therefore leaves the complete step
        // transaction untouched. Worlds without shape callbacks skip the snapshot entirely.
        #[cfg(not(target_arch = "wasm32"))]
        let prepared_callbacks = self.prepare_step_callbacks()?;

        // Advance before entering Box2D so every contact produced by this step receives one epoch.
        // Exhaustion poisons the world and returns before any native mutation occurs.
        let contact_epoch = crate::core::world_core::WorldCore::advance_contact_epoch(&self.core)?;
        self.completed_step_state().begin_step();
        let world = self.raw();
        let publication = crate::core::callback_state::run_world_step_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(self.core.brand.token()),
            || {
                #[cfg(not(target_arch = "wasm32"))]
                let active_callbacks = prepared_callbacks.install(world);
                unsafe { ffi::b2World_Step(world, time_step, sub_steps) };
                // Box2D has synchronously finished every task before returning. Unpublish the
                // native pointers before owner-side cleanup or any later world mutation can begin.
                #[cfg(not(target_arch = "wasm32"))]
                active_callbacks.finish();

                // Publish before owner cleanup so callback-time destruction of primary-world
                // objects is deferred until the completed-step capability ends. The guard retires
                // the publication if cleanup resumes a panic before a capability can be returned.
                self.completed_step_state()
                    .publish_pending(&self.core, contact_epoch)
            },
            |publication, _panic| {
                #[cfg(not(target_arch = "wasm32"))]
                self.core.worker_callbacks.drain_panics(_panic);
                publication
            },
        );
        #[cfg(not(target_arch = "wasm32"))]
        let post_step_error = self.core.worker_callbacks.take_error();
        #[cfg(target_arch = "wasm32")]
        let post_step_error = None;
        ::core::result::Result::Ok((publication, post_step_error))
    }

    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    ///
    /// The returned capability proves native simulation advanced and keeps the world exclusively
    /// borrowed. Native event getters and ID mapping run only for event families requested through
    /// that capability. Inspect [`crate::CompletedStep::post_step_error`] for callback or task
    /// failures discovered after native advancement; an outer error means Box2D was not called.
    pub fn step(
        &mut self,
        time_step: f32,
        sub_steps: i32,
    ) -> crate::error::Result<crate::events::CompletedStep<'_>> {
        check_world_available(self)?;
        check_world_step_args_valid(time_step, sub_steps)?;
        let (publication, post_step_error) = self.step_validated(time_step, sub_steps)?;
        let contact_epoch = publication.commit();
        core::result::Result::Ok(crate::events::CompletedStep::after_validated_step(
            self,
            contact_epoch,
            post_step_error,
        ))
    }

    pub(crate) fn step_while_recording_after_preflight(
        &mut self,
        time_step: f32,
        sub_steps: i32,
    ) -> crate::error::Result<(
        crate::events::PendingStepPublication<'_>,
        Option<crate::Error>,
    )> {
        check_world_step_args_valid(time_step, sub_steps)?;
        self.step_validated(time_step, sub_steps)
    }

    /// Set gravity vector.
    pub fn set_gravity<V: Into<Vec2>>(&mut self, g: V) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_set_gravity(world, g))
    }

    /// Get current gravity vector.
    pub fn gravity(&self) -> crate::error::Result<Vec2> {
        check_world_available(self)?;
        world_gravity_impl(self.raw())
    }

    /// World counters snapshot (sizes, tree heights, etc.).
    pub fn counters(&self) -> crate::error::Result<Counters> {
        check_world_available(self)?;
        world_counters_impl(self.raw())
    }

    /// World profile snapshot with per-stage timing in milliseconds from the last completed step.
    pub fn profile(&self) -> crate::error::Result<Profile> {
        check_world_available(self)?;
        world_profile_impl(self.raw())
    }

    /// Get number of awake bodies.
    pub fn awake_body_count(&self) -> crate::error::Result<i32> {
        check_world_available(self)?;
        world_awake_body_count_impl(self.raw())
    }

    // Runtime configuration helpers mirroring WorldDef fields
    pub fn enable_sleeping(&mut self, flag: bool) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_enable_sleeping(world, flag))
    }

    pub fn enable_continuous(&mut self, flag: bool) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_enable_continuous(world, flag))
    }

    /// Enable or disable constraint warm starting at runtime.
    ///
    /// Warm starting seeds the solver with accumulated impulses from the previous
    /// step to improve stability and convergence. Disabling this is only useful
    /// for experiments and will significantly reduce stability in most scenes.
    pub fn enable_warm_starting(&mut self, flag: bool) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_enable_warm_starting(world, flag))
    }

    pub fn set_restitution_threshold(&mut self, value: f32) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_set_restitution_threshold(world, value))
    }

    pub fn set_hit_event_threshold(&mut self, value: f32) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_set_hit_event_threshold(world, value))
    }

    pub fn set_contact_tuning(
        &mut self,
        hertz: f32,
        damping_ratio: f32,
        push_speed: f32,
    ) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| {
            world_set_contact_tuning(world, hertz, damping_ratio, push_speed)
        })
    }

    pub fn set_maximum_linear_speed(&mut self, v: f32) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| world_set_maximum_linear_speed(world, v))
    }

    pub fn is_sleeping_enabled(&self) -> crate::error::Result<bool> {
        check_world_available(self)?;
        Ok(world_is_sleeping_enabled_impl(self.raw()))
    }

    pub fn is_continuous_enabled(&self) -> crate::error::Result<bool> {
        check_world_available(self)?;
        Ok(world_is_continuous_enabled_impl(self.raw()))
    }

    /// Returns true if constraint warm starting is enabled.
    pub fn is_warm_starting_enabled(&self) -> crate::error::Result<bool> {
        check_world_available(self)?;
        Ok(world_is_warm_starting_enabled_impl(self.raw()))
    }

    pub fn restitution_threshold(&self) -> crate::error::Result<f32> {
        check_world_available(self)?;
        world_restitution_threshold_impl(self.raw())
    }

    pub fn hit_event_threshold(&self) -> crate::error::Result<f32> {
        check_world_available(self)?;
        world_hit_event_threshold_impl(self.raw())
    }

    pub fn maximum_linear_speed(&self) -> crate::error::Result<f32> {
        check_world_available(self)?;
        world_maximum_linear_speed_impl(self.raw())
    }

    /// Return the union of all current shape bounds in Box2D's local `f32` frame.
    pub fn bounds(&self) -> crate::error::Result<Aabb> {
        check_world_available(self)?;
        world_bounds_impl(self.raw())
    }

    /// Return the largest capacities observed by this world so far.
    pub fn maximum_capacity(&self) -> crate::error::Result<WorldCapacity> {
        check_world_available(self)?;
        world_max_capacity_impl(self.raw())
    }

    /// Return the contact point recycling threshold in meters. Zero disables recycling.
    pub fn contact_recycle_distance(&self) -> crate::error::Result<f32> {
        check_world_available(self)?;
        world_contact_recycle_distance_impl(self.raw())
    }

    /// Set the contact point recycling threshold in meters. Zero disables recycling.
    pub fn set_contact_recycle_distance(&mut self, distance: f32) -> crate::error::Result<()> {
        crate::world::run_owner_call(self, |world| {
            world_set_contact_recycle_distance(world, distance)
        })
    }

    /// Change the worker count at a step boundary.
    ///
    /// Box2D rebuilds its per-worker simulation contexts synchronously before
    /// this call returns. The scheduler wiring and number of native scheduler
    /// threads remain the ones selected when the world was created. In
    /// particular, a world created with one worker keeps the serial scheduler
    /// when its logical worker count is later increased. The `&mut self`
    /// receiver prevents a concurrent owner call.
    ///
    pub fn set_worker_count(&mut self, count: WorkerCount) -> crate::error::Result<()> {
        check_world_available(self)?;
        unsafe { ffi::b2World_SetWorkerCount(self.raw(), count.as_i32()) };
        Ok(())
    }

    pub fn worker_count(&self) -> crate::error::Result<WorkerCount> {
        check_world_available(self)?;
        world_worker_count_impl(self.raw())
    }
}

#[cfg(test)]
mod tests {
    use crate::Error;

    #[test]
    fn maximum_linear_speed_rejects_values_whose_square_overflows() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();

        assert_eq!(
            world.set_maximum_linear_speed(f32::MAX),
            Err(Error::invalid_argument(
                "World::set_maximum_linear_speed",
                "value",
                "a positive finite value whose square is finite",
            ))
        );
        assert_eq!(world.maximum_linear_speed().unwrap(), 400.0);
    }

    #[test]
    fn invalid_step_arguments_do_not_advance_the_contact_epoch() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let initial_epoch = world.core().contact_epoch();

        assert_eq!(
            world.step(f32::NAN, 4).unwrap_err(),
            Error::invalid_argument(
                "World::step",
                "time_step",
                "a finite value greater than or equal to zero",
            )
        );
        assert_eq!(world.core().contact_epoch(), initial_epoch);
        assert_eq!(
            world.step(1.0 / 60.0, 0).unwrap_err(),
            Error::invalid_argument("World::step", "sub_steps", "an integer greater than zero",)
        );
        assert_eq!(world.core().contact_epoch(), initial_epoch);

        world.step(-1.0 / 60.0, 4).unwrap_err();
        assert_eq!(world.core().contact_epoch(), initial_epoch);

        drop(world.step(1.0 / 60.0, 4).unwrap());
        assert_ne!(world.core().contact_epoch(), initial_epoch);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn shape_snapshot_failure_does_not_begin_the_step_transaction() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world.set_custom_filter(|_, _| true).unwrap();
        let completed = world.step(1.0 / 60.0, 4).unwrap();
        completed.body_events().unwrap();
        drop(completed);
        world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let initial_epoch = world.core().contact_epoch();
        let initial_getter_calls = world.event_storage().getter_calls_for_test();
        world.core().fail_next_step_shape_snapshot_for_test();

        assert_eq!(
            world.step(1.0 / 60.0, 4).unwrap_err(),
            Error::IdentityTrackingAllocationFailed
        );
        assert_eq!(world.core().contact_epoch(), initial_epoch);
        assert_eq!(
            world.event_storage().getter_calls_for_test(),
            initial_getter_calls
        );

        drop(world.step(1.0 / 60.0, 4).unwrap());
        assert_ne!(world.core().contact_epoch(), initial_epoch);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn undrained_worker_error_does_not_begin_the_step_transaction() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let initial_epoch = world.core().contact_epoch();
        let expected = Error::IdentityTrackingAllocationFailed;
        world.core().worker_callbacks.record_error(expected);

        assert_eq!(world.step(1.0 / 60.0, 4).unwrap_err(), expected);
        assert_eq!(world.core().contact_epoch(), initial_epoch);

        drop(world.step(1.0 / 60.0, 4).unwrap());
        assert_ne!(world.core().contact_epoch(), initial_epoch);
    }
}
