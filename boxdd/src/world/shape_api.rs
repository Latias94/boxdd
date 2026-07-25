use super::*;

mod control;
mod reads;
mod sensor_queries;

#[inline]
fn assert_shape_target(core: &WorldCore, shape: ShapeId) {
    crate::core::callback_state::assert_not_in_callback();
    core.check_shape(shape).expect("invalid or foreign ShapeId");
}

#[inline]
fn check_shape_target(core: &WorldCore, shape: ShapeId) -> crate::error::ApiResult<()> {
    check_shape_target_with_access(core, shape, crate::core::world_core::WorldAccess::Idle)
}

pub(crate) fn check_shape_target_with_access(
    core: &WorldCore,
    shape: ShapeId,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    core.check_shape_with_access(shape, access)
}

pub(crate) use control::{
    try_world_shape_set_capsule_with_access, try_world_shape_set_circle_with_access,
    try_world_shape_set_polygon_with_access, try_world_shape_set_segment_with_access,
    try_world_shape_set_surface_material_with_access,
};
