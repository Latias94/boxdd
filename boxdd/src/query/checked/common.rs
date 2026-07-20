use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

#[inline]
#[track_caller]
pub(crate) fn checked_query_preflight(target: &QueryTarget) {
    crate::core::callback_state::assert_not_in_callback();
    target
        .check_available()
        .expect("world must be idle, live, and not poisoned");
}

#[inline]
pub(crate) fn try_checked_query_preflight(target: &QueryTarget) -> ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    target.check_available()
}

pub(crate) fn checked_query_impl<R>(target: &QueryTarget, f: impl FnOnce() -> R) -> R {
    crate::core::callback_state::assert_not_in_callback();
    let native_call = target
        .begin_native_call()
        .expect("world must be idle, live, and not poisoned");
    let result = catch_unwind(AssertUnwindSafe(f));
    drop(native_call);
    target.process_deferred_destroys();

    match result {
        Ok(value) => value,
        Err(payload) => resume_unwind(payload),
    }
}

#[inline]
pub(crate) fn try_checked_query_result_impl<R>(
    target: &QueryTarget,
    f: impl FnOnce() -> ApiResult<R>,
) -> ApiResult<R> {
    crate::core::callback_state::check_not_in_callback()?;
    let native_call = target.begin_native_call()?;
    let result = catch_unwind(AssertUnwindSafe(f));
    drop(native_call);
    target.process_deferred_destroys();

    match result {
        Ok(value) => value,
        Err(payload) => resume_unwind(payload),
    }
}
