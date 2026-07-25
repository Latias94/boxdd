use super::*;

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
    let owner_scope = crate::core::callback_state::OwnerCallScope::enter();
    let mut panic = crate::core::callback_state::PanicSlot::default();
    let value = panic.capture_result(::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(
        f,
    )));
    panic.run_cleanup(|| drop(native_call));
    owner_scope.finish_captured(value, panic, [target.core_rc()])
}

#[inline]
pub(crate) fn try_checked_query_result_impl<R>(
    target: &QueryTarget,
    f: impl FnOnce() -> ApiResult<R>,
) -> ApiResult<R> {
    crate::core::callback_state::check_not_in_callback()?;
    let native_call = target.begin_native_call()?;
    let owner_scope = crate::core::callback_state::OwnerCallScope::enter();
    let mut panic = crate::core::callback_state::PanicSlot::default();
    let value = panic.capture_result(::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(
        f,
    )));
    panic.run_cleanup(|| drop(native_call));
    owner_scope.finish_captured(value, panic, [target.core_rc()])
}
