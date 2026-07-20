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
    crate::core::callback_state::check_not_in_callback()?;
    core.check_shape(shape)
}
