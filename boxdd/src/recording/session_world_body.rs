use super::RecordingSession;
use crate::{ExplosionDef, Result};

impl RecordingSession<'_> {
    /// Enable or disable sleeping and record the mutation.
    pub fn enable_sleeping(&mut self, flag: bool) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_enable_sleeping(world, flag)
        })
    }

    /// Enable or disable continuous collision detection and record the mutation.
    pub fn enable_continuous(&mut self, flag: bool) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_enable_continuous(world, flag)
        })
    }

    /// Enable or disable warm starting and record the mutation.
    pub fn enable_warm_starting(&mut self, flag: bool) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_enable_warm_starting(world, flag)
        })
    }

    /// Set the restitution velocity threshold and record the mutation.
    pub fn set_restitution_threshold(&mut self, threshold: f32) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_set_restitution_threshold(world, threshold)
        })
    }

    /// Set the hit-event velocity threshold and record the mutation.
    pub fn set_hit_event_threshold(&mut self, threshold: f32) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_set_hit_event_threshold(world, threshold)
        })
    }

    /// Apply an explosion and record the mutation.
    pub fn explode(&mut self, def: &ExplosionDef) -> Result<()> {
        crate::world::run_owner_call(self, |world| crate::world_extras::world_explode(world, def))
    }

    /// Configure contact softness and record the mutation.
    pub fn set_contact_tuning(
        &mut self,
        hertz: f32,
        damping_ratio: f32,
        push_speed: f32,
    ) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_set_contact_tuning(world, hertz, damping_ratio, push_speed)
        })
    }

    /// Set the contact recycling distance and record the mutation.
    pub fn set_contact_recycle_distance(&mut self, distance: f32) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_set_contact_recycle_distance(world, distance)
        })
    }

    /// Set the maximum linear speed and record the mutation.
    pub fn set_maximum_linear_speed(&mut self, speed: f32) -> Result<()> {
        crate::world::run_owner_call(self, |world| {
            crate::world::world_set_maximum_linear_speed(world, speed)
        })
    }
}
