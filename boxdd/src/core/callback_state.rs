use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::resume_unwind;
#[cfg(any(test, not(target_arch = "wasm32")))]
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use boxdd_sys::ffi;

pub(crate) type PanicPayload = Box<dyn Any + Send + 'static>;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type CustomFilterCb =
    dyn Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type PreSolveCb = dyn Fn(
        crate::types::ShapeId,
        crate::types::ShapeId,
        crate::types::Position,
        crate::types::Vec2,
    ) -> bool
    + Send
    + Sync
    + 'static;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type MaterialMixCb = dyn Fn(crate::world::MaterialMixInput, crate::world::MaterialMixInput) -> f32
    + Send
    + Sync
    + 'static;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct WorkerCallbackState {
    identities: Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    panicked: AtomicBool,
    panic: Mutex<Option<PanicPayload>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WorkerCallbackState {
    pub(crate) fn new(
        brand: crate::id::IdBrand,
        identities: Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    ) -> Arc<Self> {
        debug_assert_eq!(identities.brand(), brand);
        Arc::new(Self {
            identities,
            panicked: AtomicBool::new(false),
            panic: Mutex::new(None),
        })
    }

    #[inline]
    pub(crate) fn shape(&self, raw: ffi::b2ShapeId) -> crate::types::ShapeId {
        // Resolution copies the active nonce and releases the registry lock before the caller
        // enters arbitrary user code.
        self.identities
            .resolve_shape(raw)
            .unwrap_or_else(|error| panic!("Box2D callback returned an invalid shape id: {error}"))
    }

    #[inline]
    pub(crate) fn has_panicked(&self) -> bool {
        self.panicked.load(Ordering::Acquire)
    }

    pub(crate) fn record_panic(&self, payload: PanicPayload) {
        if self.panicked.swap(true, Ordering::AcqRel) {
            // A competing callback panic has no unique owner boundary. Forgetting this exceptional
            // payload prevents its destructor from panicking across the C callback boundary.
            std::mem::forget(payload);
            return;
        }

        let mut first = self
            .panic
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if first.is_none() {
            *first = Some(payload);
        } else {
            std::mem::forget(payload);
        }
    }

    pub(crate) fn clear_panic(&self) {
        *self
            .panic
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        self.panicked.store(false, Ordering::Release);
    }

    pub(crate) fn take_panic(&self) -> Option<PanicPayload> {
        self.panic
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct CustomFilterCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<CustomFilterCb>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct PreSolveCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<PreSolveCb>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct MaterialMixCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<MaterialMixCb>,
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static OWNER_FRAMES: RefCell<Vec<OwnerFrame>> = const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
struct OwnerFrame {
    affected: Vec<Rc<crate::core::world_core::WorldCore>>,
    cleanups: Vec<BoundaryCleanup>,
}

type BoundaryCleanup = Box<dyn FnOnce() + 'static>;

#[cfg(any(test, not(target_arch = "wasm32")))]
pub(crate) struct CallbackGuard;

#[cfg(any(test, not(target_arch = "wasm32")))]
impl CallbackGuard {
    pub(crate) fn enter() -> Self {
        DEPTH.with(|d| d.set(d.get().saturating_add(1)));
        Self
    }
}

#[cfg(any(test, not(target_arch = "wasm32")))]
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

/// Stores the first panic owned by a Rust call boundary.
///
/// A later payload is intentionally leaked: dropping arbitrary panic payloads while another panic
/// is already pending can execute a second panicking destructor and abort the process.
#[derive(Default)]
pub(crate) struct PanicSlot {
    first: Option<PanicPayload>,
}

impl PanicSlot {
    pub(crate) fn has_panicked(&self) -> bool {
        self.first.is_some()
    }

    pub(crate) fn capture(&mut self, payload: PanicPayload) {
        if self.first.is_none() {
            self.first = Some(payload);
        } else {
            std::mem::forget(payload);
        }
    }

    pub(crate) fn capture_result<T>(&mut self, result: std::thread::Result<T>) -> Option<T> {
        match result {
            ::std::result::Result::Ok(value) => ::std::option::Option::Some(value),
            ::std::result::Result::Err(payload) => {
                PanicSlot::capture(self, payload);
                ::std::option::Option::None
            }
        }
    }

    pub(crate) fn run_cleanup(&mut self, cleanup: impl FnOnce()) {
        if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(cleanup)) {
            PanicSlot::capture(self, payload);
        }
    }

    /// Resume a panic for a Rust closure that did not produce any fallback value.
    ///
    /// FFI callback protocols must use [`Self::resume_or_forget`] with their native fallback.
    pub(crate) fn resume_without_fallback(self) {
        if let Some(payload) = self.first {
            resume_unwind(payload);
        }
    }

    /// Resume the first captured panic unless this thread is already unwinding.
    ///
    /// Cleanup paths can run from `Drop`. A second unwind would abort the process, and dropping an
    /// arbitrary panic payload is itself allowed to panic, so the payload is intentionally leaked
    /// when an earlier panic already owns the thread boundary.
    pub(crate) fn resume_or_forget(self) {
        if let Some(payload) = self.first {
            if std::thread::panicking() {
                std::mem::forget(payload);
            } else {
                resume_unwind(payload);
            }
        }
    }

    pub(crate) fn into_result<T>(self, value: T) -> std::thread::Result<T> {
        match self.first {
            Some(payload) => Err(payload),
            None => Ok(value),
        }
    }
}

#[cfg(any(test, not(target_arch = "wasm32")))]
pub(crate) fn invoke_owner_callback<T: Copy>(
    panic: &mut PanicSlot,
    fallback: T,
    callback: impl FnOnce() -> T,
) -> T {
    if PanicSlot::has_panicked(panic) {
        return fallback;
    }
    match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
        let _callback_guard = CallbackGuard::enter();
        callback()
    })) {
        ::std::result::Result::Ok(value) => value,
        ::std::result::Result::Err(payload) => {
            PanicSlot::capture(panic, payload);
            fallback
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn invoke_worker_callback<T: Copy>(
    worker: &WorkerCallbackState,
    fallback: T,
    callback: impl FnOnce() -> T,
) -> T {
    if worker.has_panicked() {
        return fallback;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let _callback_guard = CallbackGuard::enter();
        callback()
    })) {
        Ok(value) => value,
        Err(payload) => {
            worker.record_panic(payload);
            fallback
        }
    }
}

/// Owns the Rust side of one synchronous native call that may invoke owner-thread callbacks.
pub(crate) struct OwnerCallScope {
    active: bool,
}

impl OwnerCallScope {
    pub(crate) fn enter() -> Self {
        OWNER_FRAMES.with(|frames| frames.borrow_mut().push(OwnerFrame::default()));
        Self { active: true }
    }

    pub(crate) fn finish<T>(
        self,
        call: std::thread::Result<T>,
        explicit: impl IntoIterator<Item = Rc<crate::core::world_core::WorldCore>>,
    ) -> T {
        let mut panic = PanicSlot::default();
        let value = panic.capture_result(call);
        self.finish_captured(value, panic, explicit)
    }

    /// Finish an owner call whose native protocol produced a usable value even when a callback
    /// panicked.
    ///
    /// Keeping that value separate from the panic payload lets a destructor finish while an outer
    /// panic already owns the thread. Ordinary calls still resume the callback panic after every
    /// deferred owner cleanup has run.
    pub(crate) fn finish_captured<T>(
        mut self,
        value: Option<T>,
        mut panic: PanicSlot,
        explicit: impl IntoIterator<Item = Rc<crate::core::world_core::WorldCore>>,
    ) -> T {
        let mut frame = self.pop_frame();
        for core in explicit {
            push_unique_core(&mut frame.affected, core);
        }

        let merged_into_parent = OWNER_FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            if let Some(parent) = frames.last_mut() {
                for core in frame.affected.drain(..) {
                    push_unique_core(&mut parent.affected, core);
                }
                parent.cleanups.append(&mut frame.cleanups);
                true
            } else {
                false
            }
        });

        if !merged_into_parent {
            if in_callback() {
                // This scope was itself entered from a callback without an owner frame on this
                // thread (for example a process hook invoking a standalone tree query). Returning
                // from the nested native call is not the outer callback boundary, so retain every
                // queued native owner and lease rather than re-entering Box2D here.
                core::mem::forget(frame);
            } else {
                drain_owner_frame(frame, &mut panic);
            }
        }

        match value {
            ::std::option::Option::Some(value) => {
                panic.resume_or_forget();
                value
            }
            ::std::option::Option::None => {
                // An arbitrary Rust closure did not produce a value. Unlike a native callback
                // protocol, this path has no sound fallback value to return.
                panic.resume_without_fallback();
                unreachable!("a captured owner-call panic must resume")
            }
        }
    }

    fn pop_frame(&mut self) -> OwnerFrame {
        self.active = false;
        OWNER_FRAMES.with(|frames| {
            frames
                .borrow_mut()
                .pop()
                .expect("owner callback frame stack must be balanced")
        })
    }
}

impl Drop for OwnerCallScope {
    fn drop(&mut self) {
        if self.active {
            let mut frame = self.pop_frame();
            let merged_into_parent = OWNER_FRAMES.with(|frames| {
                let mut frames = frames.borrow_mut();
                if let Some(parent) = frames.last_mut() {
                    for core in frame.affected.drain(..) {
                        push_unique_core(&mut parent.affected, core);
                    }
                    parent.cleanups.append(&mut frame.cleanups);
                    true
                } else {
                    false
                }
            });

            if !merged_into_parent {
                if in_callback() {
                    // There is no Rust boundary at which native teardown can safely run. Retain
                    // every captured owner and lease instead of re-entering Box2D or allowing
                    // replay to start over leaked native state.
                    core::mem::forget(frame);
                } else {
                    let mut panic = PanicSlot::default();
                    drain_owner_frame(frame, &mut panic);
                    panic.resume_or_forget();
                }
            }
        }
    }
}

pub(crate) fn register_deferred_core(
    core: Rc<crate::core::world_core::WorldCore>,
) -> Result<(), Rc<crate::core::world_core::WorldCore>> {
    OWNER_FRAMES.with(|frames| {
        if let Some(frame) = frames.borrow_mut().last_mut() {
            push_unique_core(&mut frame.affected, core);
            Ok(())
        } else {
            Err(core)
        }
    })
}

/// Run a native-owner cleanup after the outermost owner callback returns to Rust.
///
/// A callback without an owner frame has no safe local drain boundary. In that case the boxed
/// cleanup is deliberately leaked, retaining every native owner and foundation lease it captures.
pub(crate) fn defer_callback_cleanup_or_forget(cleanup: impl FnOnce() + 'static) {
    let mut cleanup: Option<BoundaryCleanup> = Some(Box::new(cleanup));
    let registered = OWNER_FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        if let Some(frame) = frames.last_mut() {
            frame
                .cleanups
                .push(cleanup.take().expect("cleanup is registered at most once"));
            true
        } else {
            false
        }
    });

    if !registered {
        core::mem::forget(cleanup.expect("an unregistered cleanup remains owned"));
    }
}

fn drain_owner_frame(frame: OwnerFrame, panic: &mut PanicSlot) {
    for core in &frame.affected {
        panic.run_cleanup(|| core.process_deferred_destroys());
    }
    for cleanup in frame.cleanups {
        panic.run_cleanup(cleanup);
    }
}

fn push_unique_core(
    cores: &mut Vec<Rc<crate::core::world_core::WorldCore>>,
    core: Rc<crate::core::world_core::WorldCore>,
) {
    if !cores.iter().any(|current| Rc::ptr_eq(current, &core)) {
        cores.push(core);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[cfg(not(target_arch = "wasm32"))]
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[cfg(not(target_arch = "wasm32"))]
    use std::thread;

    #[cfg(not(target_arch = "wasm32"))]
    struct CompetingPayload {
        drops: Arc<AtomicUsize>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl Drop for CompetingPayload {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
            panic!("competing panic payload destructor");
        }
    }

    struct DropProbe(Arc<AtomicUsize>);

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn owner_cleanup_runs_only_after_the_outer_callback_returns() {
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let owner_scope = OwnerCallScope::enter();

        {
            let _callback = CallbackGuard::enter();
            let cleanup_calls = Arc::clone(&calls);
            let probe = DropProbe(Arc::clone(&drops));
            defer_callback_cleanup_or_forget(move || {
                cleanup_calls.fetch_add(1, Ordering::SeqCst);
                drop(probe);
            });
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }

        owner_scope.finish(
            Ok(()),
            std::iter::empty::<Rc<crate::core::world_core::WorldCore>>(),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn callback_without_owner_frame_retains_cleanup_without_running_or_dropping_it() {
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));

        {
            let _callback = CallbackGuard::enter();
            let calls = Arc::clone(&calls);
            let probe = DropProbe(Arc::clone(&drops));
            defer_callback_cleanup_or_forget(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                drop(probe);
            });
        }

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn nested_owner_scope_without_outer_frame_retains_cleanup_at_unsafe_boundary() {
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));

        {
            let _outer_callback = CallbackGuard::enter();
            let owner_scope = OwnerCallScope::enter();
            let calls = Arc::clone(&calls);
            let probe = DropProbe(Arc::clone(&drops));
            defer_callback_cleanup_or_forget(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                drop(probe);
            });
            owner_scope.finish(
                Ok(()),
                std::iter::empty::<Rc<crate::core::world_core::WorldCore>>(),
            );
        }

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn concurrent_worker_panics_keep_one_payload_and_leak_losers() {
        let token = crate::id::WorldToken::allocate().expect("test world token");
        let brand = crate::id::IdBrand::new(
            ffi::b2WorldId {
                index1: 1,
                generation: 0,
            },
            token,
        )
        .expect("test world brand");
        let identities = crate::core::identity_registry::ActiveIdentityRegistry::new(brand);
        let worker = WorkerCallbackState::new(brand, identities);
        let drops = Arc::new(AtomicUsize::new(0));
        const THREADS: usize = 32;
        let ready = Arc::new(Barrier::new(THREADS));

        thread::scope(|scope| {
            for _ in 0..THREADS {
                let worker = Arc::clone(&worker);
                let drops = Arc::clone(&drops);
                let ready = Arc::clone(&ready);
                scope.spawn(move || {
                    ready.wait();
                    worker.record_panic(Box::new(CompetingPayload { drops }));
                });
            }
        });

        let winner = worker.take_panic().expect("one panic payload must win");
        assert!(winner.downcast_ref::<CompetingPayload>().is_some());
        let winner_drop = catch_unwind(AssertUnwindSafe(|| drop(winner)));
        assert!(winner_drop.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        worker.clear_panic();
        assert!(!worker.has_panicked());
    }
}
