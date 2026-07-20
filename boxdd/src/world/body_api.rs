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
    crate::core::callback_state::check_not_in_callback()?;
    core.check_body(body)
}
