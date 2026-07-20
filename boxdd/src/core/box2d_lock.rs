use std::sync::{Mutex, MutexGuard, OnceLock};

static BOX2D_GLOBAL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn lock<'a>() -> MutexGuard<'a, ()> {
    BOX2D_GLOBAL_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
