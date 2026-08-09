use super::*;

#[inline]
fn check_native_world_vec2(
    operation: &'static str,
    output: &'static str,
    value: Vec2,
) -> crate::error::Result<Vec2> {
    if value.is_valid() {
        Ok(value)
    } else {
        Err(crate::error::Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite vector",
        })
    }
}

#[inline]
fn check_native_world_non_negative(
    operation: &'static str,
    output: &'static str,
    value: f32,
) -> crate::error::Result<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(crate::error::Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite non-negative value",
        })
    }
}

#[inline]
fn check_native_world_positive(
    operation: &'static str,
    output: &'static str,
    value: f32,
) -> crate::error::Result<f32> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(crate::error::Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite positive value",
        })
    }
}

#[inline]
fn check_native_world_count(
    operation: &'static str,
    output: &'static str,
    value: i32,
) -> crate::error::Result<i32> {
    if value >= 0 {
        Ok(value)
    } else {
        Err(crate::error::Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a non-negative native int",
        })
    }
}

#[inline]
pub(super) fn world_gravity_impl(world: ffi::b2WorldId) -> crate::error::Result<Vec2> {
    check_native_world_vec2(
        "World::gravity",
        "gravity",
        Vec2::from_raw(unsafe { ffi::b2World_GetGravity(world) }),
    )
}

#[inline]
pub(super) fn world_counters_impl(world: ffi::b2WorldId) -> crate::error::Result<Counters> {
    Counters::from_native("World::counters", unsafe {
        ffi::b2World_GetCounters(world)
    })
}

#[inline]
pub(super) fn world_profile_impl(world: ffi::b2WorldId) -> crate::error::Result<Profile> {
    Profile::from_native("World::profile", unsafe { ffi::b2World_GetProfile(world) })
}

#[inline]
pub(super) fn world_awake_body_count_impl(world: ffi::b2WorldId) -> crate::error::Result<i32> {
    check_native_world_count("World::awake_body_count", "awake_body_count", unsafe {
        ffi::b2World_GetAwakeBodyCount(world)
    })
}

#[inline]
pub(super) fn world_is_sleeping_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsSleepingEnabled(world) }
}

#[inline]
pub(super) fn world_is_continuous_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsContinuousEnabled(world) }
}

#[inline]
pub(super) fn world_is_warm_starting_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsWarmStartingEnabled(world) }
}

#[inline]
pub(super) fn world_restitution_threshold_impl(world: ffi::b2WorldId) -> crate::error::Result<f32> {
    check_native_world_non_negative(
        "World::restitution_threshold",
        "restitution_threshold",
        unsafe { ffi::b2World_GetRestitutionThreshold(world) },
    )
}

#[inline]
pub(super) fn world_hit_event_threshold_impl(world: ffi::b2WorldId) -> crate::error::Result<f32> {
    check_native_world_non_negative(
        "World::hit_event_threshold",
        "hit_event_threshold",
        unsafe { ffi::b2World_GetHitEventThreshold(world) },
    )
}

#[inline]
pub(super) fn world_maximum_linear_speed_impl(world: ffi::b2WorldId) -> crate::error::Result<f32> {
    check_native_world_positive(
        "World::maximum_linear_speed",
        "maximum_linear_speed",
        unsafe { ffi::b2World_GetMaximumLinearSpeed(world) },
    )
}

#[inline]
pub(super) fn world_bounds_impl(world: ffi::b2WorldId) -> crate::error::Result<Aabb> {
    Aabb::from_raw(unsafe { ffi::b2World_GetBounds(world) }).map_err(|_| {
        crate::error::Error::InvalidNativeOutput {
            operation: "World::bounds",
            output: "bounds",
            constraint: "finite ordered lower and upper bounds",
        }
    })
}

#[inline]
pub(super) fn world_max_capacity_impl(
    world: ffi::b2WorldId,
) -> crate::error::Result<WorldCapacity> {
    WorldCapacity::from_raw(unsafe { ffi::b2World_GetMaxCapacity(world) })
}

#[inline]
pub(super) fn world_contact_recycle_distance_impl(
    world: ffi::b2WorldId,
) -> crate::error::Result<f32> {
    check_native_world_non_negative(
        "World::contact_recycle_distance",
        "contact_recycle_distance",
        unsafe { ffi::b2World_GetContactRecycleDistance(world) },
    )
}

#[inline]
pub(super) fn world_worker_count_impl(world: ffi::b2WorldId) -> crate::error::Result<WorkerCount> {
    WorkerCount::from_native(unsafe { ffi::b2World_GetWorkerCount(world) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_world_scalar_vector_and_count_checks_fail_closed() {
        assert_eq!(
            check_native_world_vec2("World::gravity", "gravity", Vec2::new(f32::NAN, 0.0)),
            Err(crate::Error::InvalidNativeOutput {
                operation: "World::gravity",
                output: "gravity",
                constraint: "a finite vector",
            })
        );
        assert_eq!(
            check_native_world_non_negative(
                "World::contact_recycle_distance",
                "contact_recycle_distance",
                -1.0,
            ),
            Err(crate::Error::InvalidNativeOutput {
                operation: "World::contact_recycle_distance",
                output: "contact_recycle_distance",
                constraint: "a finite non-negative value",
            })
        );
        assert_eq!(
            check_native_world_positive("World::maximum_linear_speed", "maximum_linear_speed", 0.0,),
            Err(crate::Error::InvalidNativeOutput {
                operation: "World::maximum_linear_speed",
                output: "maximum_linear_speed",
                constraint: "a finite positive value",
            })
        );
        assert_eq!(
            check_native_world_count("World::awake_body_count", "awake_body_count", -1),
            Err(crate::Error::InvalidNativeOutput {
                operation: "World::awake_body_count",
                output: "awake_body_count",
                constraint: "a non-negative native int",
            })
        );
    }
}
