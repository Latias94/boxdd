use std::any::Any;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::panic::resume_unwind;
#[cfg(any(test, not(target_arch = "wasm32")))]
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

pub(crate) type PanicPayload = Box<dyn Any + Send + 'static>;

/// Owns a user value until an operation crosses every fallible preflight.
///
/// Rejected inputs are destroyed behind the same cleanup boundary used for retired callbacks and
/// user data. Ordinary calls still resume a destructor panic, while a value rejected during an
/// outer unwind cannot replace the primary panic or abort the process.
#[must_use = "pending user values must be committed or released through their cleanup boundary"]
pub(crate) struct PendingUserValue<T>(Option<T>);

impl<T> PendingUserValue<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Some(value))
    }

    pub(crate) fn into_inner(mut self) -> T {
        self.0
            .take()
            .expect("a pending user value may be committed only once")
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn as_mut(&mut self) -> &mut T {
        self.0
            .as_mut()
            .expect("a committed user value cannot be borrowed again")
    }
}

impl<T> Drop for PendingUserValue<T> {
    fn drop(&mut self) {
        let Some(value) = self.0.take() else {
            return;
        };
        let mut panic = PanicSlot::default();
        panic.run_cleanup(|| drop(value));
        panic.resume_or_forget();
    }
}

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
    failed: AtomicBool,
    panics: Mutex<Vec<PanicPayload>>,
    error: Mutex<Option<crate::Error>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WorkerCallbackState {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            failed: AtomicBool::new(false),
            panics: Mutex::new(Vec::new()),
            error: Mutex::new(None),
        })
    }

    #[inline]
    pub(crate) fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    pub(crate) fn record_panic(&self, payload: PanicPayload) {
        self.failed.store(true, Ordering::Release);
        self.panics
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(payload);
    }

    pub(crate) fn record_error(&self, error: crate::Error) {
        self.failed.store(true, Ordering::Release);
        let mut first = self
            .error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if first.is_none() {
            *first = Some(error);
        }
    }

    pub(crate) fn take_error(&self) -> Option<crate::Error> {
        self.error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    /// Prepare a new native call after the previous call has joined every worker.
    ///
    /// A correctly completed owner boundary has already drained the queue. If an exceptional path
    /// left a payload behind, resume it here instead of silently discarding the original failure.
    pub(crate) fn begin_call(&self) -> crate::Result<()> {
        let error = self.take_error();
        let mut panic = PanicSlot::default();
        self.drain_panics(&mut panic);
        self.failed.store(false, Ordering::Release);
        panic.resume_or_forget();
        match error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(crate) fn drain_panics(&self, target: &mut PanicSlot) {
        let payloads = core::mem::take(
            &mut *self
                .panics
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for payload in payloads {
            target.capture(payload);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct CustomFilterCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) shapes: Arc<crate::core::identity_registry::StepShapeResolver>,
    pub(crate) cb: Arc<CustomFilterCb>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct PreSolveCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) shapes: Arc<crate::core::identity_registry::StepShapeResolver>,
    pub(crate) cb: Arc<PreSolveCb>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct MaterialMixCtx {
    pub(crate) worker: Arc<WorkerCallbackState>,
    pub(crate) cb: Box<MaterialMixCb>,
}

/// Produce a worker-visible context pointer only after proving the context can be shared safely.
///
/// Native APIs spell callback context as mutable `void*`, but boxdd callbacks only read through
/// the pointer while the boxed owner keeps the allocation alive through worker join.
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub(crate) fn worker_context_ptr<T: Send + Sync>(context: &T) -> *mut T {
    core::ptr::from_ref(context).cast_mut()
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static OWNER_FRAMES: RefCell<Vec<OwnerFrame>> = const { RefCell::new(Vec::new()) };
    static NEXT_OWNER_FRAME_GENERATION: Cell<u64> = const { Cell::new(1) };
}

#[cfg(test)]
pub(crate) fn owner_frame_count_for_test() -> usize {
    OWNER_FRAMES.with(|frames| frames.borrow().len())
}

struct OwnerFrame {
    generation: OwnerFrameGeneration,
    owner: CallbackOwnerToken,
    cleanups: Vec<OwnedBoundaryCleanup>,
    worlds: Vec<OwnedWorld>,
    panic: PanicSlot,
}

type BoundaryCleanup = Box<dyn FnOnce() + 'static>;

struct OwnedBoundaryCleanup {
    owner: CallbackOwnerToken,
    cleanup: BoundaryCleanup,
}

struct OwnedWorld {
    owner: CallbackOwnerToken,
    core: Pin<Box<crate::core::world_core::WorldCore>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct OwnerFrameGeneration(NonZeroU64);

impl OwnerFrameGeneration {
    fn next() -> Self {
        NEXT_OWNER_FRAME_GENERATION.with(|next| {
            let generation = NonZeroU64::new(next.get())
                .expect("owner callback frame generation must remain non-zero");
            next.set(
                generation
                    .get()
                    .checked_add(1)
                    .expect("owner callback frame generation overflowed"),
            );
            Self(generation)
        })
    }
}

/// Stable identity of the native owner whose call may synchronously invoke Rust.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CallbackOwnerToken {
    World(crate::id::WorldToken),
    DynamicTree(NonZeroU64),
    #[cfg(test)]
    Test(NonZeroU64),
}

impl CallbackOwnerToken {
    pub(crate) const fn world(token: crate::id::WorldToken) -> Self {
        Self::World(token)
    }

    pub(crate) const fn dynamic_tree(token: NonZeroU64) -> Self {
        Self::DynamicTree(token)
    }

    #[cfg(test)]
    fn test(token: u64) -> Self {
        Self::Test(NonZeroU64::new(token).expect("test owner token must be non-zero"))
    }
}

impl OwnerFrame {
    fn new(generation: OwnerFrameGeneration, owner: CallbackOwnerToken) -> Self {
        Self {
            generation,
            owner,
            cleanups: Vec::new(),
            worlds: Vec::new(),
            panic: PanicSlot::default(),
        }
    }

    fn merge(&mut self, mut nested: Self) -> Option<PanicPayload> {
        self.cleanups.append(&mut nested.cleanups);
        self.worlds.append(&mut nested.worlds);
        self.panic.absorb_deferred(nested.panic)
    }

    fn merge_resources(&mut self, mut nested: Self) {
        debug_assert!(!nested.panic.has_panicked());
        self.cleanups.append(&mut nested.cleanups);
        self.worlds.append(&mut nested.worlds);
    }
}

#[cfg(any(test, not(target_arch = "wasm32")))]
pub(crate) struct CallbackGuard {
    _token: CallbackGuardToken,
    _owner_thread: PhantomData<Rc<()>>,
}

#[cfg(any(test, not(target_arch = "wasm32")))]
struct CallbackGuardToken;

#[cfg(any(test, not(target_arch = "wasm32")))]
impl CallbackGuard {
    pub(crate) fn enter() -> Self {
        DEPTH.with(|d| {
            d.set(
                d.get()
                    .checked_add(1)
                    .expect("Box2D callback nesting depth overflowed"),
            )
        });
        Self {
            _token: CallbackGuardToken,
            _owner_thread: PhantomData,
        }
    }
}

#[cfg(any(test, not(target_arch = "wasm32")))]
impl Drop for CallbackGuard {
    fn drop(&mut self) {
        DEPTH.with(|d| {
            let depth = d.get();
            debug_assert!(depth > 0, "callback guard depth underflow");
            d.set(depth.saturating_sub(1));
        });
    }
}

#[inline]
pub(crate) fn in_callback() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

#[inline]
pub(crate) fn check_not_in_callback() -> crate::error::Result<()> {
    if in_callback() {
        Err(crate::error::Error::InCallback)
    } else {
        Ok(())
    }
}

/// Stores the first panic owned by a Rust call boundary.
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
            drop_suppressed_panic_payload(payload);
        }
    }

    pub(crate) fn absorb(&mut self, other: Self) {
        if let Some(payload) = self.absorb_deferred(other) {
            drop_suppressed_panic_payload(payload);
        }
    }

    /// Merge another slot without running an arbitrary suppressed-payload destructor.
    ///
    /// Callers holding owner-frame or registry borrows must release them before dropping the
    /// returned payload because its destructor may re-enter another callback boundary.
    fn absorb_deferred(&mut self, mut other: Self) -> Option<PanicPayload> {
        let payload = other.first.take()?;
        if self.first.is_none() {
            self.first = Some(payload);
            None
        } else {
            Some(payload)
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

fn drop_suppressed_panic_payload(payload: PanicPayload) {
    if let Err(secondary) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(payload)))
    {
        // The original payload and its resources have completed their isolated destructor. The
        // replacement payload produced by a panicking destructor has no remaining safe boundary:
        // dropping it could recurse forever, while resuming it would replace the primary panic.
        std::mem::forget(secondary);
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
    if worker.has_failed() {
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
struct OwnerCallScope {
    generation: OwnerFrameGeneration,
    active: bool,
    _owner_thread: PhantomData<Rc<()>>,
}

impl OwnerCallScope {
    fn enter(owner: CallbackOwnerToken) -> Self {
        assert!(
            !in_callback(),
            "an owner call cannot begin from a Box2D callback"
        );
        let generation = OwnerFrameGeneration::next();
        OWNER_FRAMES.with(|frames| {
            frames.borrow_mut().push(OwnerFrame::new(generation, owner));
        });
        Self {
            generation,
            active: true,
            _owner_thread: PhantomData,
        }
    }

    #[cfg(test)]
    pub(crate) fn finish<T>(self, call: std::thread::Result<T>) -> T {
        let mut panic = PanicSlot::default();
        let value = panic.capture_result(call);
        self.finish_captured(value, panic)
    }

    /// Finish an owner call whose native protocol produced a usable value even when a callback
    /// panicked.
    ///
    /// Keeping that value separate from the panic payload lets a destructor finish while an outer
    /// panic already owns the thread. Ordinary calls still resume the callback panic after every
    /// deferred owner cleanup has run.
    pub(crate) fn finish_captured<T>(mut self, value: Option<T>, mut panic: PanicSlot) -> T {
        let mut frame = self.pop_frame();
        frame.panic.absorb(core::mem::take(&mut panic));

        let mut nested_panic = None;
        let mut suppressed_panic = None;
        let root = OWNER_FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            let Some(parent) = frames.last_mut() else {
                return Some(frame);
            };
            if value.is_some() {
                suppressed_panic = parent.merge(frame);
            } else {
                nested_panic = Some(core::mem::take(&mut frame.panic));
                parent.merge_resources(frame);
            }
            None
        });
        if let Some(payload) = suppressed_panic {
            drop_suppressed_panic_payload(payload);
        }

        if let Some(mut frame) = root {
            if in_callback() {
                // Finishing while a callback guard is still active would re-enter Box2D during
                // teardown. Retain the complete frame if an internal invariant is ever violated.
                core::mem::forget(frame);
            } else {
                let mut panic = core::mem::take(&mut frame.panic);
                drain_owner_frame(frame, &mut panic);
                match value {
                    ::std::option::Option::Some(value) => {
                        panic.resume_or_forget();
                        return value;
                    }
                    ::std::option::Option::None => {
                        panic.resume_without_fallback();
                        unreachable!("a captured owner-call panic must resume")
                    }
                }
            }
        }

        if let Some(panic) = nested_panic {
            panic.resume_without_fallback();
            unreachable!("a captured nested owner-call panic must resume")
        }

        value.expect("a nested owner boundary without a value must carry a panic")
    }

    fn pop_frame(&mut self) -> OwnerFrame {
        self.active = false;
        OWNER_FRAMES.with(|frames| {
            let frame = frames
                .borrow_mut()
                .pop()
                .expect("owner callback frame must be balanced");
            assert_eq!(
                frame.generation, self.generation,
                "owner callback frames must finish in LIFO order"
            );
            frame
        })
    }
}

impl Drop for OwnerCallScope {
    fn drop(&mut self) {
        if self.active {
            let frame = self.pop_frame();
            let mut suppressed_panic = None;
            let root = OWNER_FRAMES.with(|frames| {
                let mut frames = frames.borrow_mut();
                if let Some(parent) = frames.last_mut() {
                    suppressed_panic = parent.merge(frame);
                    None
                } else {
                    Some(frame)
                }
            });
            if let Some(payload) = suppressed_panic {
                drop_suppressed_panic_payload(payload);
            }
            let Some(mut frame) = root else { return };
            if in_callback() {
                // There is no Rust boundary at which native teardown can safely run. Retain every
                // captured owner and lease instead of re-entering Box2D.
                core::mem::forget(frame);
                return;
            }
            let mut panic = core::mem::take(&mut frame.panic);
            drain_owner_frame(frame, &mut panic);
            panic.resume_or_forget();
        }
    }
}

macro_rules! define_callback_boundaries {
    ($($(#[$meta:meta])* $variant:ident => $label:literal => $runner:ident),+ $(,)?) => {
        /// Named callback-capable native boundaries covered by the shared owner scope.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        enum CallbackBoundary {
            $($(#[$meta])* $variant,)+
            #[cfg(test)]
            TestOnly,
        }

        impl CallbackBoundary {
            #[cfg(test)]
            const ALL: &'static [Self] = &[$($(#[$meta])* Self::$variant),+];

            #[cfg(test)]
            const fn as_str(self) -> &'static str {
                match self {
                    $($(#[$meta])* Self::$variant => $label,)+
                    Self::TestOnly => "test-only",
                }
            }
        }

        $(
            $(#[$meta])*
            #[inline]
            pub(crate) fn $runner<Native, Output>(
                owner: CallbackOwnerToken,
                invoke: impl FnOnce() -> Native,
                complete: impl FnOnce(Option<Native>, &mut PanicSlot) -> Option<Output>,
            ) -> Output {
                run_owner_callback_boundary(CallbackBoundary::$variant, owner, invoke, complete)
            }
        )+
    };
}

define_callback_boundaries! {
    WorldStep => "world-step" => run_world_step_boundary,
    Query => "query" => run_query_boundary,
    #[cfg(not(target_arch = "wasm32"))]
    DebugDraw => "debug-draw" => run_debug_draw_boundary,
    #[cfg(not(target_arch = "wasm32"))]
    DynamicTreeQuery => "dynamic-tree-query" => run_dynamic_tree_query_boundary,
    #[cfg(not(target_arch = "wasm32"))]
    DynamicTreeQueryAll => "dynamic-tree-query-all" => run_dynamic_tree_query_all_boundary,
    #[cfg(not(target_arch = "wasm32"))]
    DynamicTreeRayCast => "dynamic-tree-ray-cast" => run_dynamic_tree_ray_cast_boundary,
    #[cfg(not(target_arch = "wasm32"))]
    DynamicTreeBoxCast => "dynamic-tree-box-cast" => run_dynamic_tree_box_cast_boundary,
    ReplayStep => "replay-step" => run_replay_step_boundary,
    ReplaySeek => "replay-seek" => run_replay_seek_boundary,
    ReplayRestart => "replay-restart" => run_replay_restart_boundary,
    #[cfg(not(target_arch = "wasm32"))]
    ReplayDraw => "replay-draw" => run_replay_draw_boundary,
}

/// Run one complete callback-capable native invocation under the shared owner boundary.
///
/// `complete` runs after the native stack has returned, even when `invoke` panics. It can unpublish
/// callback contexts, merge worker or owner callback panics, and decide whether a usable value was
/// produced. Only this function can create the scope used by production callback-capable paths.
fn run_owner_callback_boundary<Native, Output>(
    _boundary: CallbackBoundary,
    owner: CallbackOwnerToken,
    invoke: impl FnOnce() -> Native,
    complete: impl FnOnce(Option<Native>, &mut PanicSlot) -> Option<Output>,
) -> Output {
    let owner_scope = OwnerCallScope::enter(owner);
    let mut panic = PanicSlot::default();
    let native = panic.capture_result(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        invoke,
    )));
    let completed = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        complete(native, &mut panic)
    })) {
        Ok(completed) => completed,
        Err(payload) => {
            panic.capture(payload);
            None
        }
    };
    owner_scope.finish_captured(completed, panic)
}

#[cfg(test)]
pub(crate) fn run_test_owner_callback_boundary<Native, Output>(
    owner: CallbackOwnerToken,
    invoke: impl FnOnce() -> Native,
    complete: impl FnOnce(Option<Native>, &mut PanicSlot) -> Option<Output>,
) -> Output {
    run_owner_callback_boundary(CallbackBoundary::TestOnly, owner, invoke, complete)
}

/// Run a native-owner cleanup after the active owner callback returns to Rust.
///
/// A callback without an owner frame has no safe local drain boundary. In that case the boxed
/// cleanup is deliberately leaked, retaining every native owner and foundation lease it captures.
pub(crate) fn defer_callback_cleanup_or_forget(
    owner: CallbackOwnerToken,
    cleanup: impl FnOnce() + 'static,
) {
    let mut cleanup: Option<BoundaryCleanup> = Some(Box::new(cleanup));
    let registered = OWNER_FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        if let Some(frame) = frames.last_mut() {
            frame.cleanups.push(OwnedBoundaryCleanup {
                owner,
                cleanup: cleanup.take().expect("cleanup is registered at most once"),
            });
            true
        } else {
            false
        }
    });

    if !registered {
        core::mem::forget(cleanup.expect("an unregistered cleanup remains owned"));
    }
}

/// Transfer the sole native-world owner to the active callback boundary.
///
/// A callback without an owner frame has no safe local drain boundary. In that case the complete
/// owner is deliberately leaked, retaining the native world and its foundation lease rather than
/// re-entering Box2D or exposing a dangling callback context.
pub(crate) fn defer_world_owner_or_forget(core: Pin<Box<crate::core::world_core::WorldCore>>) {
    let owner_token = CallbackOwnerToken::world(core.brand.token());
    let mut owner = Some(core);
    let registered = OWNER_FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        if let Some(frame) = frames.last_mut() {
            frame.worlds.push(OwnedWorld {
                owner: owner_token,
                core: owner
                    .take()
                    .expect("world owner is registered at most once"),
            });
            true
        } else {
            false
        }
    });

    if !registered {
        core::mem::forget(owner.expect("an unregistered world owner remains owned"));
    }
}

fn drain_owner_frame(mut frame: OwnerFrame, panic: &mut PanicSlot) {
    let _root_owner = frame.owner;
    for cleanup in frame.cleanups.drain(..) {
        let _cleanup_owner = cleanup.owner;
        panic.run_cleanup(cleanup.cleanup);
    }
    for world in frame.worlds.drain(..) {
        let _world_owner = world.owner;
        panic.run_cleanup(move || {
            world.core.shutdown_native();
            drop(world.core);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use boxdd_sys::ffi;
    use std::sync::Arc;
    #[cfg(not(target_arch = "wasm32"))]
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[cfg(not(target_arch = "wasm32"))]
    use std::thread;

    #[cfg(not(target_arch = "wasm32"))]
    static_assertions::assert_impl_all!(CustomFilterCtx: Send, Sync);
    #[cfg(not(target_arch = "wasm32"))]
    static_assertions::assert_impl_all!(PreSolveCtx: Send, Sync);
    #[cfg(not(target_arch = "wasm32"))]
    static_assertions::assert_impl_all!(MaterialMixCtx: Send, Sync);
    static_assertions::assert_not_impl_any!(CallbackGuard: Send, Sync);
    static_assertions::assert_not_impl_any!(OwnerCallScope: Send, Sync);

    #[test]
    fn callback_boundary_catalog_is_exact() {
        let boundaries = CallbackBoundary::ALL
            .iter()
            .copied()
            .map(CallbackBoundary::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            boundaries,
            [
                "world-step",
                "query",
                "debug-draw",
                "dynamic-tree-query",
                "dynamic-tree-query-all",
                "dynamic-tree-ray-cast",
                "dynamic-tree-box-cast",
                "replay-step",
                "replay-seek",
                "replay-restart",
                "replay-draw",
            ]
        );
    }

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
    fn rejected_pending_user_value_resumes_its_destructor_panic() {
        struct PanicOnDrop;

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                std::panic::panic_any("pending user value drop panic");
            }
        }

        let panic = std::panic::catch_unwind(|| drop(PendingUserValue::new(PanicOnDrop)))
            .expect_err("an ordinary rejected input must resume its destructor panic");
        assert_eq!(
            panic.downcast_ref::<&'static str>(),
            Some(&"pending user value drop panic")
        );
    }

    struct ReentrantPayload {
        drops: Arc<AtomicUsize>,
        completed_boundaries: Arc<AtomicUsize>,
    }

    impl Drop for ReentrantPayload {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
            let scope = OwnerCallScope::enter(CallbackOwnerToken::test(99));
            scope.finish(Ok(()));
            self.completed_boundaries.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn owner_cleanup_runs_only_after_the_callback_returns() {
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = CallbackOwnerToken::test(1);
        let owner_scope = OwnerCallScope::enter(owner);

        {
            let _callback = CallbackGuard::enter();
            let cleanup_calls = Arc::clone(&calls);
            let probe = DropProbe(Arc::clone(&drops));
            defer_callback_cleanup_or_forget(owner, move || {
                cleanup_calls.fetch_add(1, Ordering::SeqCst);
                drop(probe);
            });
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }

        owner_scope.finish(Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nested_owner_frames_merge_cleanup_into_the_root_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let outer = OwnerCallScope::enter(CallbackOwnerToken::test(1));
        let inner_owner = CallbackOwnerToken::test(2);
        let inner = OwnerCallScope::enter(inner_owner);
        {
            let _callback = CallbackGuard::enter();
            let calls = Arc::clone(&calls);
            defer_callback_cleanup_or_forget(inner_owner, move || {
                calls.fetch_add(1, Ordering::SeqCst);
            });
        }

        inner.finish(Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        OWNER_FRAMES.with(|frames| assert_eq!(frames.borrow().len(), 1));
        outer.finish(Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        OWNER_FRAMES.with(|frames| assert!(frames.borrow().is_empty()));
    }

    #[test]
    fn nested_callback_panic_resumes_only_after_the_root_drains() {
        let outer = OwnerCallScope::enter(CallbackOwnerToken::test(1));
        let inner = OwnerCallScope::enter(CallbackOwnerToken::test(2));
        let mut panic = PanicSlot::default();
        panic.capture(Box::new("nested callback panic"));

        inner.finish_captured(Some(()), panic);
        OWNER_FRAMES.with(|frames| assert_eq!(frames.borrow().len(), 1));
        let resumed = catch_unwind(AssertUnwindSafe(|| outer.finish(Ok(()))))
            .expect_err("the root boundary must resume the nested callback panic");
        assert_eq!(
            resumed.downcast_ref::<&str>(),
            Some(&"nested callback panic")
        );
        OWNER_FRAMES.with(|frames| assert!(frames.borrow().is_empty()));
    }

    #[test]
    fn suppressed_nested_panic_payload_drops_after_owner_frame_borrow_is_released() {
        let drops = Arc::new(AtomicUsize::new(0));
        let completed_boundaries = Arc::new(AtomicUsize::new(0));
        let outer = OwnerCallScope::enter(CallbackOwnerToken::test(1));

        let first = OwnerCallScope::enter(CallbackOwnerToken::test(2));
        let mut first_panic = PanicSlot::default();
        first_panic.capture(Box::new("first nested callback panic"));
        first.finish_captured(Some(()), first_panic);

        let second = OwnerCallScope::enter(CallbackOwnerToken::test(3));
        let mut second_panic = PanicSlot::default();
        second_panic.capture(Box::new(ReentrantPayload {
            drops: Arc::clone(&drops),
            completed_boundaries: Arc::clone(&completed_boundaries),
        }));
        second.finish_captured(Some(()), second_panic);

        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(completed_boundaries.load(Ordering::SeqCst), 1);
        OWNER_FRAMES.with(|frames| assert_eq!(frames.borrow().len(), 1));

        let resumed = catch_unwind(AssertUnwindSafe(|| outer.finish(Ok(()))))
            .expect_err("the first callback panic must remain authoritative");
        assert_eq!(
            resumed.downcast_ref::<&str>(),
            Some(&"first nested callback panic")
        );
        OWNER_FRAMES.with(|frames| assert!(frames.borrow().is_empty()));
    }

    #[test]
    fn owner_callback_panic_payload_is_not_dropped_on_the_native_stack() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = CallbackOwnerToken::test(1);
        let panic = catch_unwind(AssertUnwindSafe(|| {
            run_owner_callback_boundary(
                CallbackBoundary::TestOnly,
                owner,
                || {
                    let mut callback_panic = PanicSlot::default();
                    invoke_owner_callback(&mut callback_panic, (), || {
                        std::panic::panic_any(DropProbe(Arc::clone(&drops)));
                    });
                    assert_eq!(
                        drops.load(Ordering::SeqCst),
                        0,
                        "the native callback stack must retain its panic payload",
                    );
                    callback_panic
                },
                |callback_panic, panic| {
                    callback_panic.map(|callback_panic| {
                        panic.absorb(callback_panic);
                        assert_eq!(
                            drops.load(Ordering::SeqCst),
                            0,
                            "postflight must transfer rather than drop the panic payload",
                        );
                    })
                },
            );
        }))
        .expect_err("the owner boundary must resume the callback panic");

        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(panic);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn owner_actions_run_before_affected_world_teardown() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .expect("test world should be created");
        let raw = world.raw();
        let world_owner = CallbackOwnerToken::world(world.core().brand.token());
        let owner_scope = OwnerCallScope::enter(world_owner);

        {
            let _callback = CallbackGuard::enter();
            // Dropping the world on a callback stack transfers its core into the active frame.
            // The deferred action needs that native owner to remain valid until it has detached.
            drop(world);
            defer_callback_cleanup_or_forget(world_owner, move || {
                assert!(unsafe { ffi::b2World_IsValid(raw) });
            });
        }

        owner_scope.finish(Ok(()));
        assert!(!unsafe { ffi::b2World_IsValid(raw) });
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn owner_world_teardown_finishes_before_the_original_panic_resumes() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .expect("test world should be created");
        let raw = world.raw();
        let cleanup_calls = Arc::new(AtomicUsize::new(0));
        let world_owner = CallbackOwnerToken::world(world.core().brand.token());
        let owner_scope = OwnerCallScope::enter(world_owner);

        {
            let _callback = CallbackGuard::enter();
            drop(world);
            let cleanup_calls = Arc::clone(&cleanup_calls);
            defer_callback_cleanup_or_forget(world_owner, move || {
                assert!(unsafe { ffi::b2World_IsValid(raw) });
                cleanup_calls.fetch_add(1, Ordering::SeqCst);
            });
        }

        let panic = catch_unwind(AssertUnwindSafe(|| {
            owner_scope.finish::<()>(Err(Box::new("original callback panic") as PanicPayload));
        }));

        let payload = panic.expect_err("the original callback panic must resume");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"original callback panic")
        );
        assert_eq!(cleanup_calls.load(Ordering::SeqCst), 1);
        assert!(!unsafe { ffi::b2World_IsValid(raw) });
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn non_callback_world_drop_destroys_native_with_an_active_lease() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .expect("test world should be created");
        let raw = world.raw();
        let activity = world.core().begin_restore_activity().unwrap();

        drop(world);

        assert!(!unsafe { ffi::b2World_IsValid(raw) });
        drop(activity);
    }

    #[test]
    fn callback_without_owner_frame_retains_cleanup_without_running_or_dropping_it() {
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));

        {
            let _callback = CallbackGuard::enter();
            let calls = Arc::clone(&calls);
            let probe = DropProbe(Arc::clone(&drops));
            defer_callback_cleanup_or_forget(CallbackOwnerToken::test(1), move || {
                calls.fetch_add(1, Ordering::SeqCst);
                drop(probe);
            });
        }

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn owner_call_entry_is_rejected_from_a_callback() {
        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _callback = CallbackGuard::enter();
            let _owner_scope = OwnerCallScope::enter(CallbackOwnerToken::test(1));
        }));

        assert!(panic.is_err());
        OWNER_FRAMES.with(|frames| assert!(frames.borrow().is_empty()));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn begin_call_resumes_an_undrained_worker_panic() {
        let worker = WorkerCallbackState::new();
        worker.record_panic(Box::new("undrained worker panic"));

        let panic = catch_unwind(AssertUnwindSafe(|| worker.begin_call()))
            .expect_err("an undrained worker panic must not be discarded");
        assert_eq!(
            panic.downcast_ref::<&str>(),
            Some(&"undrained worker panic")
        );
        assert!(!worker.has_failed());
        let mut remaining = PanicSlot::default();
        worker.drain_panics(&mut remaining);
        assert!(!remaining.has_panicked());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn concurrent_worker_panics_drop_every_payload_only_on_the_owner_boundary() {
        let worker = WorkerCallbackState::new();
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

        assert_eq!(drops.load(Ordering::SeqCst), 0);
        let mut panic = PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert_eq!(drops.load(Ordering::SeqCst), THREADS - 1);
        let winner = panic
            .into_result(())
            .expect_err("one panic payload must win");
        assert!(winner.downcast_ref::<CompetingPayload>().is_some());
        let winner_drop = catch_unwind(AssertUnwindSafe(|| drop(winner)));
        assert!(winner_drop.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), THREADS);
        worker.begin_call().unwrap();
        assert!(!worker.has_failed());
    }
}
