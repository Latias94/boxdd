use super::*;

#[inline]
fn world_gravity_impl(world: ffi::b2WorldId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2World_GetGravity(world) })
}

#[inline]
fn world_counters_impl(world: ffi::b2WorldId) -> Counters {
    Counters::from_raw(unsafe { ffi::b2World_GetCounters(world) })
}

#[inline]
fn world_profile_impl(world: ffi::b2WorldId) -> Profile {
    Profile::from_raw(unsafe { ffi::b2World_GetProfile(world) })
}

#[inline]
fn world_awake_body_count_impl(world: ffi::b2WorldId) -> i32 {
    unsafe { ffi::b2World_GetAwakeBodyCount(world) }
}

#[inline]
fn world_is_sleeping_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsSleepingEnabled(world) }
}

#[inline]
fn world_is_continuous_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsContinuousEnabled(world) }
}

#[inline]
fn world_is_warm_starting_enabled_impl(world: ffi::b2WorldId) -> bool {
    unsafe { ffi::b2World_IsWarmStartingEnabled(world) }
}

#[inline]
fn world_restitution_threshold_impl(world: ffi::b2WorldId) -> f32 {
    unsafe { ffi::b2World_GetRestitutionThreshold(world) }
}

#[inline]
fn world_hit_event_threshold_impl(world: ffi::b2WorldId) -> f32 {
    unsafe { ffi::b2World_GetHitEventThreshold(world) }
}

#[inline]
fn world_maximum_linear_speed_impl(world: ffi::b2WorldId) -> f32 {
    unsafe { ffi::b2World_GetMaximumLinearSpeed(world) }
}

#[inline]
fn checked_world_read_impl<R>(f: impl FnOnce() -> R) -> R {
    crate::core::callback_state::assert_not_in_callback();
    f()
}

#[inline]
fn try_checked_world_read_impl<R>(f: impl FnOnce() -> R) -> crate::error::ApiResult<R> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(f())
}

pub(crate) fn world_gravity_checked_impl(world: ffi::b2WorldId) -> Vec2 {
    checked_world_read_impl(|| world_gravity_impl(world))
}

pub(crate) fn try_world_gravity_impl(world: ffi::b2WorldId) -> crate::error::ApiResult<Vec2> {
    try_checked_world_read_impl(|| world_gravity_impl(world))
}

pub(crate) fn world_counters_checked_impl(world: ffi::b2WorldId) -> Counters {
    checked_world_read_impl(|| world_counters_impl(world))
}

pub(crate) fn try_world_counters_impl(world: ffi::b2WorldId) -> crate::error::ApiResult<Counters> {
    try_checked_world_read_impl(|| world_counters_impl(world))
}

pub(crate) fn world_profile_checked_impl(world: ffi::b2WorldId) -> Profile {
    checked_world_read_impl(|| world_profile_impl(world))
}

pub(crate) fn try_world_profile_impl(world: ffi::b2WorldId) -> crate::error::ApiResult<Profile> {
    try_checked_world_read_impl(|| world_profile_impl(world))
}

pub(crate) fn world_awake_body_count_checked_impl(world: ffi::b2WorldId) -> i32 {
    checked_world_read_impl(|| world_awake_body_count_impl(world))
}

pub(crate) fn try_world_awake_body_count_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<i32> {
    try_checked_world_read_impl(|| world_awake_body_count_impl(world))
}

pub(crate) fn world_is_sleeping_enabled_checked_impl(world: ffi::b2WorldId) -> bool {
    checked_world_read_impl(|| world_is_sleeping_enabled_impl(world))
}

pub(crate) fn try_world_is_sleeping_enabled_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<bool> {
    try_checked_world_read_impl(|| world_is_sleeping_enabled_impl(world))
}

pub(crate) fn world_is_continuous_enabled_checked_impl(world: ffi::b2WorldId) -> bool {
    checked_world_read_impl(|| world_is_continuous_enabled_impl(world))
}

pub(crate) fn try_world_is_continuous_enabled_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<bool> {
    try_checked_world_read_impl(|| world_is_continuous_enabled_impl(world))
}

pub(crate) fn world_is_warm_starting_enabled_checked_impl(world: ffi::b2WorldId) -> bool {
    checked_world_read_impl(|| world_is_warm_starting_enabled_impl(world))
}

pub(crate) fn try_world_is_warm_starting_enabled_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<bool> {
    try_checked_world_read_impl(|| world_is_warm_starting_enabled_impl(world))
}

pub(crate) fn world_restitution_threshold_checked_impl(world: ffi::b2WorldId) -> f32 {
    checked_world_read_impl(|| world_restitution_threshold_impl(world))
}

pub(crate) fn try_world_restitution_threshold_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<f32> {
    try_checked_world_read_impl(|| world_restitution_threshold_impl(world))
}

pub(crate) fn world_hit_event_threshold_checked_impl(world: ffi::b2WorldId) -> f32 {
    checked_world_read_impl(|| world_hit_event_threshold_impl(world))
}

pub(crate) fn try_world_hit_event_threshold_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<f32> {
    try_checked_world_read_impl(|| world_hit_event_threshold_impl(world))
}

pub(crate) fn world_maximum_linear_speed_checked_impl(world: ffi::b2WorldId) -> f32 {
    checked_world_read_impl(|| world_maximum_linear_speed_impl(world))
}

pub(crate) fn try_world_maximum_linear_speed_impl(
    world: ffi::b2WorldId,
) -> crate::error::ApiResult<f32> {
    try_checked_world_read_impl(|| world_maximum_linear_speed_impl(world))
}
