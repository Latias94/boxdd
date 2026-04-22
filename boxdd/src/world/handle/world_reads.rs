use super::*;

impl WorldHandle {
    pub(crate) fn new(core: Arc<WorldCore>) -> Self {
        Self {
            core,
            _not_send_sync: core::marker::PhantomData,
        }
    }

    /// Expose raw world id for advanced use-cases.
    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.core.id
    }

    pub(crate) fn raw(&self) -> ffi::b2WorldId {
        self.world_id_raw()
    }

    pub fn gravity(&self) -> Vec2 {
        world_gravity_checked_impl(self.raw())
    }

    pub fn try_gravity(&self) -> crate::error::ApiResult<Vec2> {
        try_world_gravity_impl(self.raw())
    }

    pub fn counters(&self) -> Counters {
        world_counters_checked_impl(self.raw())
    }

    pub fn try_counters(&self) -> crate::error::ApiResult<Counters> {
        try_world_counters_impl(self.raw())
    }

    pub fn profile(&self) -> Profile {
        world_profile_checked_impl(self.raw())
    }

    pub fn try_profile(&self) -> crate::error::ApiResult<Profile> {
        try_world_profile_impl(self.raw())
    }

    pub fn awake_body_count(&self) -> i32 {
        world_awake_body_count_checked_impl(self.raw())
    }

    pub fn try_awake_body_count(&self) -> crate::error::ApiResult<i32> {
        try_world_awake_body_count_impl(self.raw())
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
