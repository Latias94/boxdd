use super::*;

mod control;
mod reads;

#[inline]
fn assert_body_target(core: &WorldCore, body: BodyId) {
    crate::core::callback_state::assert_not_in_callback();
    core.check_body(body)
        .expect("body must be live and belong to this world");
}

#[inline]
fn check_body_target(core: &WorldCore, body: BodyId) -> crate::error::ApiResult<()> {
    check_body_target_with_access(core, body, crate::core::world_core::WorldAccess::Idle)
}

pub(crate) fn check_body_target_with_access(
    core: &WorldCore,
    body: BodyId,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_body_with_access(body, access)
}

pub(crate) use control::{
    try_body_apply_angular_impulse_with_access, try_body_apply_force_to_center_with_access,
    try_body_apply_force_with_access, try_body_apply_linear_impulse_to_center_with_access,
    try_body_apply_linear_impulse_with_access, try_body_apply_mass_from_shapes_with_access,
    try_body_apply_torque_with_access, try_body_clear_forces_with_access,
    try_body_disable_with_access, try_body_enable_contact_events_with_access,
    try_body_enable_contact_recycling_with_access, try_body_enable_hit_events_with_access,
    try_body_enable_sleep_with_access, try_body_enable_with_access,
    try_body_set_angular_damping_with_access, try_body_set_awake_with_access,
    try_body_set_bullet_with_access, try_body_set_gravity_scale_with_access,
    try_body_set_linear_damping_with_access, try_body_set_mass_data_with_access,
    try_body_set_motion_locks_with_access, try_body_set_name_with_access,
    try_body_set_sleep_threshold_with_access, try_body_wake_touching_with_access,
    try_set_body_angular_velocity_with_access, try_set_body_linear_velocity_with_access,
    try_set_body_position_and_rotation_with_access, try_set_body_target_transform_with_access,
    try_set_body_type_with_access,
};
