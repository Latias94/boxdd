use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct CallbackGuard;

impl CallbackGuard {
    pub(crate) fn enter() -> Self {
        DEPTH.with(|d| d.set(d.get().saturating_add(1)));
        Self
    }
}

impl Drop for CallbackGuard {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

#[inline]
pub(crate) fn in_callback() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

#[inline]
#[track_caller]
pub(crate) fn assert_not_in_callback() {
    assert!(
        !in_callback(),
        "boxdd API called from a Box2D callback; call is not allowed because Box2D world is locked"
    );
}

#[inline]
pub(crate) fn check_not_in_callback() -> crate::error::ApiResult<()> {
    if in_callback() {
        Err(crate::error::ApiError::InCallback)
    } else {
        Ok(())
    }
}
