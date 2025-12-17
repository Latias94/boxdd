use std::sync::{Mutex, MutexGuard, OnceLock};

static BOX2D_GLOBAL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn lock<'a>() -> MutexGuard<'a, ()> {
    BOX2D_GLOBAL_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("box2d global lock poisoned")
}
