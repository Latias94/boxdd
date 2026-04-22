use super::*;

pub(crate) fn checked_query_impl<R>(f: impl FnOnce() -> R) -> R {
    crate::core::callback_state::assert_not_in_callback();
    f()
}

#[inline]
pub(crate) fn try_checked_query_result_impl<R>(f: impl FnOnce() -> ApiResult<R>) -> ApiResult<R> {
    crate::core::callback_state::check_not_in_callback()?;
    f()
}
