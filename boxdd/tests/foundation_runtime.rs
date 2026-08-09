#[cfg(not(target_arch = "wasm32"))]
use boxdd::{
    Error, Foundation, FoundationActivityError, FoundationAssertHook, FoundationConfig,
    FoundationInitError, FoundationLogHook,
};
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::CString;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Barrier};

#[cfg(not(target_arch = "wasm32"))]
const ASSERT_CHILD_ENV: &str = "BOXDD_FOUNDATION_ASSERT_CHILD";
#[cfg(not(target_arch = "wasm32"))]
const ASSERT_HOOK_MARKER: &str = "boxdd-foundation-assert-hook-ran";
#[cfg(not(target_arch = "wasm32"))]
const ASSERT_RETURNED_MARKER: &str = "boxdd-foundation-assert-returned";
#[cfg(not(target_arch = "wasm32"))]
const UNINITIALIZED_COMPLETED_MARKER: &str = "boxdd-foundation-uninitialized-completed";

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn foundation_configuration_and_assert_trap() {
    match std::env::var(ASSERT_CHILD_ENV).ok().as_deref() {
        Some("assert") => run_assert_trap_child(),
        Some("default") => {
            run_explicit_default_child();
            return;
        }
        Some("conflict") => {
            run_conflicting_initialization_child();
            return;
        }
        Some("uninitialized") => {
            run_uninitialized_safe_call_child();
            return;
        }
        _ => {}
    }

    let assert_hook: Arc<FoundationAssertHook> = Arc::new(|_, _, _| {});
    let config = FoundationConfig::new(2.5).with_assert_hook(assert_hook);
    let barrier = Arc::new(Barrier::new(9));
    let mut initializers = Vec::new();
    for _ in 0..8 {
        let barrier = Arc::clone(&barrier);
        let config = config.clone();
        initializers.push(std::thread::spawn(move || {
            barrier.wait();
            Foundation::initialize(config)
        }));
    }
    barrier.wait();
    let initialized: Vec<_> = initializers
        .into_iter()
        .map(|thread| thread.join().unwrap().unwrap())
        .collect();
    let first = initialized[0];
    assert!(
        initialized
            .iter()
            .all(|foundation| core::ptr::eq(*foundation, first))
    );
    let second = Foundation::initialize(config).unwrap();

    assert!(core::ptr::eq(first, second));
    assert_eq!(first.config().length_units_per_meter(), 2.5);
    assert_eq!(unsafe { boxdd_sys::ffi::b2GetLengthUnitsPerMeter() }, 2.5);
    assert_eq!(
        Foundation::initialize(FoundationConfig::new(3.0)).unwrap_err(),
        Error::FoundationInitialization(FoundationInitError::ConfigurationConflict)
    );

    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "foundation_configuration_and_assert_trap",
            "--nocapture",
        ])
        .env(ASSERT_CHILD_ENV, "assert")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "native assertion unexpectedly returned"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(ASSERT_HOOK_MARKER),
        "configured assertion hook did not run before the native trap: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains(ASSERT_RETURNED_MARKER),
        "native execution continued after the assertion trampoline requested a trap"
    );

    let explicit_default = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "foundation_configuration_and_assert_trap",
            "--nocapture",
        ])
        .env(ASSERT_CHILD_ENV, "default")
        .output()
        .unwrap();
    assert!(
        explicit_default.status.success(),
        "explicit default initialization child failed: {}",
        String::from_utf8_lossy(&explicit_default.stderr)
    );

    let conflict = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "foundation_configuration_and_assert_trap",
            "--nocapture",
        ])
        .env(ASSERT_CHILD_ENV, "conflict")
        .output()
        .unwrap();
    assert!(
        conflict.status.success(),
        "conflicting initialization child failed: {}",
        String::from_utf8_lossy(&conflict.stderr)
    );

    let uninitialized = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "foundation_configuration_and_assert_trap",
            "--nocapture",
        ])
        .env(ASSERT_CHILD_ENV, "uninitialized")
        .output()
        .unwrap();
    assert!(
        uninitialized.status.success()
            && String::from_utf8_lossy(&uninitialized.stderr)
                .contains(UNINITIALIZED_COMPLETED_MARKER),
        "uninitialized safe-call child failed or ran no assertions: {}",
        String::from_utf8_lossy(&uninitialized.stderr)
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn run_uninitialized_safe_call_child() {
    assert!(Foundation::get().is_none());
    assert_eq!(
        boxdd::version(),
        Err(Error::FoundationActivity(
            FoundationActivityError::NotInitialized
        ))
    );
    assert!(Foundation::get().is_none());
    eprintln!("{UNINITIALIZED_COMPLETED_MARKER}");
}

#[cfg(not(target_arch = "wasm32"))]
fn run_assert_trap_child() -> ! {
    let hook: Arc<FoundationAssertHook> = Arc::new(|condition, file_name, line_number| {
        eprintln!("{ASSERT_HOOK_MARKER}: {condition}, {file_name}:{line_number}");
    });
    Foundation::initialize(FoundationConfig::default().with_assert_hook(hook)).unwrap();

    let condition = CString::new("intentional foundation assertion").unwrap();
    let file_name = CString::new("foundation_runtime.rs").unwrap();
    // SAFETY: both pointers are valid null-terminated strings for the duration of the call. The
    // child process exists specifically to verify that the configured nonzero trampoline traps.
    unsafe {
        boxdd_sys::ffi::b2InternalAssert(condition.as_ptr(), file_name.as_ptr(), line!() as i32);
    }
    eprintln!("{ASSERT_RETURNED_MARKER}");
    std::process::exit(86);
}

#[cfg(not(target_arch = "wasm32"))]
fn run_explicit_default_child() {
    let first = Foundation::initialize_default().unwrap();
    let version = boxdd::version().expect("Box2D version should be available");
    assert!(version.major >= 3);

    let second = Foundation::initialize_default().unwrap();
    let explicit = Foundation::initialize(FoundationConfig::default()).unwrap();

    assert!(core::ptr::eq(first, second));
    assert!(core::ptr::eq(first, explicit));
    assert_eq!(first.config().length_units_per_meter(), 1.0);
    assert_eq!(
        Foundation::initialize(FoundationConfig::new(2.0)).unwrap_err(),
        Error::FoundationInitialization(FoundationInitError::ConfigurationConflict)
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn run_conflicting_initialization_child() {
    let start = Arc::new(Barrier::new(3));
    let mut initializers = Vec::new();
    for scale in [2.0, 3.0] {
        let start = Arc::clone(&start);
        initializers.push(std::thread::spawn(move || {
            start.wait();
            Foundation::initialize(FoundationConfig::new(scale))
                .map(|foundation| foundation.config().length_units_per_meter())
        }));
    }
    start.wait();

    let results: Vec<_> = initializers
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(Error::FoundationInitialization(
                        FoundationInitError::ConfigurationConflict
                    ))
                )
            })
            .count(),
        1
    );
    let winning_scale = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(
        unsafe { boxdd_sys::ffi::b2GetLengthUnitsPerMeter() },
        winning_scale
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn foundation_hook_destructors_use_one_outer_unwind_boundary() {
    const CHILD_ENV: &str = "BOXDD_FOUNDATION_HOOK_DROP_CHILD";
    const TEST_NAME: &str = "foundation_hook_destructors_use_one_outer_unwind_boundary";
    const MARKER: &str = "boxdd-foundation-hook-drop: completed";
    const PRIMARY_PANIC: &str = "foundation hook cleanup outer unwind remains primary";

    struct PanicOnDrop(Arc<AtomicUsize>);

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
            panic!("intentional foundation hook destructor panic");
        }
    }

    struct InvokeOnDrop<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

    fn assert_hook(drops: &Arc<AtomicUsize>) -> Arc<FoundationAssertHook> {
        let probe = PanicOnDrop(Arc::clone(drops));
        Arc::new(move |_, _, _| {
            let _ = &probe;
        })
    }

    fn log_hook(drops: &Arc<AtomicUsize>) -> Arc<FoundationLogHook> {
        let probe = PanicOnDrop(Arc::clone(drops));
        Arc::new(move |_| {
            let _ = &probe;
        })
    }

    fn during_outer_unwind(invoke: impl FnOnce()) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _invoke = InvokeOnDrop(Some(invoke));
            std::panic::panic_any(PRIMARY_PANIC);
        }));
        let payload = result.expect_err("the outer panic must keep unwinding");
        assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
    }

    fn install_child_panic_hook() {
        std::panic::set_hook(Box::new(|info| {
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string panic payload");
            if let Some(location) = info.location() {
                eprintln!(
                    "boxdd-foundation-hook-drop panic at {}:{}:{}: {message}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            } else {
                eprintln!("boxdd-foundation-hook-drop panic: {message}");
            }
        }));
    }

    fn run_child() {
        let drops = Arc::new(AtomicUsize::new(0));
        let config = FoundationConfig::default()
            .with_assert_hook(assert_hook(&drops))
            .with_log_hook(log_hook(&drops));
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(config)));
        assert!(panic.is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 2);

        let drops = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicUsize::new(0));
        let observed_rejected = Arc::clone(&rejected);
        let config = FoundationConfig::new(f32::NAN)
            .with_assert_hook(assert_hook(&drops))
            .with_log_hook(log_hook(&drops));
        during_outer_unwind(move || {
            if matches!(
                Foundation::initialize(config),
                Err(Error::FoundationInitialization(
                    FoundationInitError::InvalidLengthUnitsPerMeter
                ))
            ) {
                observed_rejected.fetch_add(1, Ordering::SeqCst);
            }
        });
        assert_eq!(rejected.load(Ordering::SeqCst), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 2);

        let drops = Arc::new(AtomicUsize::new(0));
        let config = FoundationConfig::default().with_assert_hook(assert_hook(&drops));
        let replacement: Arc<FoundationAssertHook> = Arc::new(|_, _, _| {});
        let committed = std::rc::Rc::new(std::cell::RefCell::new(None));
        let committed_from_drop = std::rc::Rc::clone(&committed);
        during_outer_unwind(move || {
            committed_from_drop.replace(Some(config.with_assert_hook(replacement)));
        });
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        let config = committed
            .borrow_mut()
            .take()
            .expect("hook replacement must return the committed configuration");
        assert!(config.has_assert_hook());
        drop(config);
        eprintln!("{MARKER}");
    }

    if std::env::var_os(CHILD_ENV).is_some() {
        install_child_panic_hook();
        run_child();
        return;
    }

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("foundation hook cleanup child process must start");
    assert!(
        output.status.success(),
        "foundation hook cleanup child aborted\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(MARKER),
        "foundation hook cleanup child did not finish assertions\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
