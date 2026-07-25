use super::RecordingSession;
use crate::core::world_core::WorldAccess;
use crate::{
    ApiResult, BodyId, ExplosionDef, MassData, MotionLocks, Position, Vec2, WorldTransform,
};

const RECORDING: WorldAccess = WorldAccess::Recording;

impl RecordingSession<'_> {
    /// Enable or disable sleeping and record the mutation.
    pub fn enable_sleeping(&mut self, flag: bool) {
        self.try_enable_sleeping(flag)
            .expect("recording session could not configure sleeping")
    }

    pub fn try_enable_sleeping(&mut self, flag: bool) -> ApiResult<()> {
        crate::world::try_world_enable_sleeping_with_access(self.world, flag, RECORDING)
    }

    /// Enable or disable continuous collision detection and record the mutation.
    pub fn enable_continuous(&mut self, flag: bool) {
        self.try_enable_continuous(flag)
            .expect("recording session could not configure continuous collision detection")
    }

    pub fn try_enable_continuous(&mut self, flag: bool) -> ApiResult<()> {
        crate::world::try_world_enable_continuous_with_access(self.world, flag, RECORDING)
    }

    /// Enable or disable warm starting and record the mutation.
    pub fn enable_warm_starting(&mut self, flag: bool) {
        self.try_enable_warm_starting(flag)
            .expect("recording session could not configure warm starting")
    }

    pub fn try_enable_warm_starting(&mut self, flag: bool) -> ApiResult<()> {
        crate::world::try_world_enable_warm_starting_with_access(self.world, flag, RECORDING)
    }

    /// Set the restitution velocity threshold and record the mutation.
    pub fn set_restitution_threshold(&mut self, threshold: f32) {
        self.try_set_restitution_threshold(threshold)
            .expect("recording session received an invalid restitution threshold")
    }

    pub fn try_set_restitution_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        crate::world::try_world_set_restitution_threshold_with_access(
            self.world, threshold, RECORDING,
        )
    }

    /// Set the hit-event velocity threshold and record the mutation.
    pub fn set_hit_event_threshold(&mut self, threshold: f32) {
        self.try_set_hit_event_threshold(threshold)
            .expect("recording session received an invalid hit-event threshold")
    }

    pub fn try_set_hit_event_threshold(&mut self, threshold: f32) -> ApiResult<()> {
        crate::world::try_world_set_hit_event_threshold_with_access(
            self.world, threshold, RECORDING,
        )
    }

    /// Apply an explosion and record the mutation.
    pub fn explode(&mut self, def: &ExplosionDef) {
        self.try_explode(def)
            .expect("recording session could not apply an explosion")
    }

    pub fn try_explode(&mut self, def: &ExplosionDef) -> ApiResult<()> {
        crate::world_extras::try_world_explode_with_access(self.world, def, RECORDING)
    }

    /// Configure contact softness and record the mutation.
    pub fn set_contact_tuning(&mut self, hertz: f32, damping_ratio: f32, push_speed: f32) {
        self.try_set_contact_tuning(hertz, damping_ratio, push_speed)
            .expect("recording session received invalid contact tuning")
    }

    pub fn try_set_contact_tuning(
        &mut self,
        hertz: f32,
        damping_ratio: f32,
        push_speed: f32,
    ) -> ApiResult<()> {
        crate::world::try_world_set_contact_tuning_with_access(
            self.world,
            hertz,
            damping_ratio,
            push_speed,
            RECORDING,
        )
    }

    /// Set the contact recycling distance and record the mutation.
    pub fn set_contact_recycle_distance(&mut self, distance: f32) {
        self.try_set_contact_recycle_distance(distance)
            .expect("recording session received an invalid contact recycling distance")
    }

    pub fn try_set_contact_recycle_distance(&mut self, distance: f32) -> ApiResult<()> {
        crate::world::try_world_set_contact_recycle_distance_with_access(
            self.world, distance, RECORDING,
        )
    }

    /// Set the maximum linear speed and record the mutation.
    pub fn set_maximum_linear_speed(&mut self, speed: f32) {
        self.try_set_maximum_linear_speed(speed)
            .expect("recording session received an invalid maximum linear speed")
    }

    pub fn try_set_maximum_linear_speed(&mut self, speed: f32) -> ApiResult<()> {
        crate::world::try_world_set_maximum_linear_speed_with_access(self.world, speed, RECORDING)
    }

    /// Set a body's debug name and record the mutation.
    pub fn set_body_name(&mut self, body: BodyId, name: &str) {
        self.try_set_body_name(body, name)
            .expect("recording session received an invalid body name")
    }

    pub fn try_set_body_name(&mut self, body: BodyId, name: &str) -> ApiResult<()> {
        crate::world::try_body_set_name_with_access(self.world.core(), body, name, RECORDING)
    }

    /// Set a body's target transform and record the mutation.
    pub fn set_body_target_transform(
        &mut self,
        body: BodyId,
        target: WorldTransform,
        time_step: f32,
        wake: bool,
    ) {
        self.try_set_body_target_transform(body, target, time_step, wake)
            .expect("recording session received an invalid body target transform")
    }

    pub fn try_set_body_target_transform(
        &mut self,
        body: BodyId,
        target: WorldTransform,
        time_step: f32,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_set_body_target_transform_with_access(
            self.world.core(),
            body,
            target,
            time_step,
            wake,
            RECORDING,
        )
    }

    /// Apply a force at a world point and record the mutation.
    pub fn body_apply_force<F: Into<Vec2>, P: Into<Position>>(
        &mut self,
        body: BodyId,
        force: F,
        point: P,
        wake: bool,
    ) {
        self.try_body_apply_force(body, force, point, wake)
            .expect("recording session received an invalid body force")
    }

    pub fn try_body_apply_force<F: Into<Vec2>, P: Into<Position>>(
        &mut self,
        body: BodyId,
        force: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_body_apply_force_with_access(
            self.world.core(),
            body,
            force,
            point,
            wake,
            RECORDING,
        )
    }

    /// Apply a force at the center of mass and record the mutation.
    pub fn body_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        force: V,
        wake: bool,
    ) {
        self.try_body_apply_force_to_center(body, force, wake)
            .expect("recording session received an invalid body force")
    }

    pub fn try_body_apply_force_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        force: V,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_body_apply_force_to_center_with_access(
            self.world.core(),
            body,
            force,
            wake,
            RECORDING,
        )
    }

    /// Apply torque and record the mutation.
    pub fn body_apply_torque(&mut self, body: BodyId, torque: f32, wake: bool) {
        self.try_body_apply_torque(body, torque, wake)
            .expect("recording session received an invalid body torque")
    }

    pub fn try_body_apply_torque(
        &mut self,
        body: BodyId,
        torque: f32,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_body_apply_torque_with_access(
            self.world.core(),
            body,
            torque,
            wake,
            RECORDING,
        )
    }

    /// Apply a linear impulse at a world point and record the mutation.
    pub fn body_apply_linear_impulse<F: Into<Vec2>, P: Into<Position>>(
        &mut self,
        body: BodyId,
        impulse: F,
        point: P,
        wake: bool,
    ) {
        self.try_body_apply_linear_impulse(body, impulse, point, wake)
            .expect("recording session received an invalid body impulse")
    }

    pub fn try_body_apply_linear_impulse<F: Into<Vec2>, P: Into<Position>>(
        &mut self,
        body: BodyId,
        impulse: F,
        point: P,
        wake: bool,
    ) -> ApiResult<()> {
        crate::world::try_body_apply_linear_impulse_with_access(
            self.world.core(),
            body,
            impulse,
            point,
            wake,
            RECORDING,
        )
    }

    /// Override body mass data and record the mutation.
    pub fn set_body_mass_data(&mut self, body: BodyId, mass_data: MassData) {
        self.try_set_body_mass_data(body, mass_data)
            .expect("recording session received invalid body mass data")
    }

    pub fn try_set_body_mass_data(&mut self, body: BodyId, mass_data: MassData) -> ApiResult<()> {
        crate::world::try_body_set_mass_data_with_access(
            self.world.core(),
            body,
            mass_data,
            RECORDING,
        )
    }

    /// Recompute body mass from attached shapes and record the mutation.
    pub fn body_apply_mass_from_shapes(&mut self, body: BodyId) {
        self.try_body_apply_mass_from_shapes(body)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_apply_mass_from_shapes(&mut self, body: BodyId) -> ApiResult<()> {
        crate::world::try_body_apply_mass_from_shapes_with_access(
            self.world.core(),
            body,
            RECORDING,
        )
    }

    /// Set linear damping and record the mutation.
    pub fn set_body_linear_damping(&mut self, body: BodyId, damping: f32) {
        self.try_set_body_linear_damping(body, damping)
            .expect("recording session received invalid linear damping")
    }

    pub fn try_set_body_linear_damping(&mut self, body: BodyId, damping: f32) -> ApiResult<()> {
        crate::world::try_body_set_linear_damping_with_access(
            self.world.core(),
            body,
            damping,
            RECORDING,
        )
    }

    /// Set angular damping and record the mutation.
    pub fn set_body_angular_damping(&mut self, body: BodyId, damping: f32) {
        self.try_set_body_angular_damping(body, damping)
            .expect("recording session received invalid angular damping")
    }

    pub fn try_set_body_angular_damping(&mut self, body: BodyId, damping: f32) -> ApiResult<()> {
        crate::world::try_body_set_angular_damping_with_access(
            self.world.core(),
            body,
            damping,
            RECORDING,
        )
    }

    /// Set body gravity scale and record the mutation.
    pub fn set_body_gravity_scale(&mut self, body: BodyId, scale: f32) {
        self.try_set_body_gravity_scale(body, scale)
            .expect("recording session received an invalid gravity scale")
    }

    pub fn try_set_body_gravity_scale(&mut self, body: BodyId, scale: f32) -> ApiResult<()> {
        crate::world::try_body_set_gravity_scale_with_access(
            self.world.core(),
            body,
            scale,
            RECORDING,
        )
    }

    /// Set a body's awake state and record the mutation.
    pub fn set_body_awake(&mut self, body: BodyId, awake: bool) {
        self.try_set_body_awake(body, awake)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_set_body_awake(&mut self, body: BodyId, awake: bool) -> ApiResult<()> {
        crate::world::try_body_set_awake_with_access(self.world.core(), body, awake, RECORDING)
    }

    /// Wake bodies touching this body and record the mutation.
    pub fn body_wake_touching(&mut self, body: BodyId) {
        self.try_body_wake_touching(body)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_wake_touching(&mut self, body: BodyId) -> ApiResult<()> {
        crate::world::try_body_wake_touching_with_access(self.world.core(), body, RECORDING)
    }

    /// Enable or disable body sleeping and record the mutation.
    pub fn body_enable_sleep(&mut self, body: BodyId, flag: bool) {
        self.try_body_enable_sleep(body, flag)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_enable_sleep(&mut self, body: BodyId, flag: bool) -> ApiResult<()> {
        crate::world::try_body_enable_sleep_with_access(self.world.core(), body, flag, RECORDING)
    }

    /// Set the body sleep threshold and record the mutation.
    pub fn set_body_sleep_threshold(&mut self, body: BodyId, threshold: f32) {
        self.try_set_body_sleep_threshold(body, threshold)
            .expect("recording session received an invalid body sleep threshold")
    }

    pub fn try_set_body_sleep_threshold(&mut self, body: BodyId, threshold: f32) -> ApiResult<()> {
        crate::world::try_body_set_sleep_threshold_with_access(
            self.world.core(),
            body,
            threshold,
            RECORDING,
        )
    }

    /// Enable a body and record the mutation.
    pub fn enable_body(&mut self, body: BodyId) {
        self.try_enable_body(body)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_enable_body(&mut self, body: BodyId) -> ApiResult<()> {
        crate::world::try_body_enable_with_access(self.world.core(), body, RECORDING)
    }

    /// Disable a body and record the mutation.
    pub fn disable_body(&mut self, body: BodyId) {
        self.try_disable_body(body)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_disable_body(&mut self, body: BodyId) -> ApiResult<()> {
        crate::world::try_body_disable_with_access(self.world.core(), body, RECORDING)
    }

    /// Set per-axis body motion locks and record the mutation.
    pub fn set_body_motion_locks(&mut self, body: BodyId, locks: MotionLocks) {
        self.try_set_body_motion_locks(body, locks)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_set_body_motion_locks(&mut self, body: BodyId, locks: MotionLocks) -> ApiResult<()> {
        crate::world::try_body_set_motion_locks_with_access(
            self.world.core(),
            body,
            locks,
            RECORDING,
        )
    }

    /// Set the bullet flag and record the mutation.
    pub fn set_body_bullet(&mut self, body: BodyId, flag: bool) {
        self.try_set_body_bullet(body, flag)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_set_body_bullet(&mut self, body: BodyId, flag: bool) -> ApiResult<()> {
        crate::world::try_body_set_bullet_with_access(self.world.core(), body, flag, RECORDING)
    }

    /// Enable contact recycling for future contacts and record the mutation.
    pub fn body_enable_contact_recycling(&mut self, body: BodyId, flag: bool) {
        self.try_body_enable_contact_recycling(body, flag)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_enable_contact_recycling(&mut self, body: BodyId, flag: bool) -> ApiResult<()> {
        crate::world::try_body_enable_contact_recycling_with_access(
            self.world.core(),
            body,
            flag,
            RECORDING,
        )
    }

    /// Enable body contact events and record the mutation.
    pub fn body_enable_contact_events(&mut self, body: BodyId, flag: bool) {
        self.try_body_enable_contact_events(body, flag)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_enable_contact_events(&mut self, body: BodyId, flag: bool) -> ApiResult<()> {
        crate::world::try_body_enable_contact_events_with_access(
            self.world.core(),
            body,
            flag,
            RECORDING,
        )
    }

    /// Enable body hit events and record the mutation.
    pub fn body_enable_hit_events(&mut self, body: BodyId, flag: bool) {
        self.try_body_enable_hit_events(body, flag)
            .expect("recording session received an invalid BodyId")
    }

    pub fn try_body_enable_hit_events(&mut self, body: BodyId, flag: bool) -> ApiResult<()> {
        crate::world::try_body_enable_hit_events_with_access(
            self.world.core(),
            body,
            flag,
            RECORDING,
        )
    }
}
