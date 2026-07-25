#[cfg(not(target_arch = "wasm32"))]
use boxdd::{FoundationAssertHook, FoundationConfig, FoundationInitError, initialize_foundation};
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::CString;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Barrier};

#[cfg(not(target_arch = "wasm32"))]
const ASSERT_CHILD_ENV: &str = "BOXDD_FOUNDATION_ASSERT_CHILD";
#[cfg(not(target_arch = "wasm32"))]
const ASSERT_HOOK_MARKER: &str = "boxdd-foundation-assert-hook-ran";
#[cfg(not(target_arch = "wasm32"))]
const ASSERT_RETURNED_MARKER: &str = "boxdd-foundation-assert-returned";

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn foundation_configuration_and_assert_trap() {
    match std::env::var(ASSERT_CHILD_ENV).ok().as_deref() {
        Some("assert") => run_assert_trap_child(),
        Some("lazy") => {
            run_lazy_default_child();
            return;
        }
        Some("conflict") => {
            run_conflicting_initialization_child();
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
            initialize_foundation(config)
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
    let second = initialize_foundation(config).unwrap();

    assert!(core::ptr::eq(first, second));
    assert_eq!(first.config().length_units_per_meter(), 2.5);
    assert_eq!(unsafe { boxdd_sys::ffi::b2GetLengthUnitsPerMeter() }, 2.5);
    assert_eq!(
        initialize_foundation(FoundationConfig::new(3.0)).unwrap_err(),
        FoundationInitError::ConfigurationConflict
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

    let lazy = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "foundation_configuration_and_assert_trap",
            "--nocapture",
        ])
        .env(ASSERT_CHILD_ENV, "lazy")
        .output()
        .unwrap();
    assert!(
        lazy.status.success(),
        "lazy default initialization child failed: {}",
        String::from_utf8_lossy(&lazy.stderr)
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
}

#[cfg(not(target_arch = "wasm32"))]
fn run_assert_trap_child() -> ! {
    let hook: Arc<FoundationAssertHook> = Arc::new(|condition, file_name, line_number| {
        eprintln!("{ASSERT_HOOK_MARKER}: {condition}, {file_name}:{line_number}");
    });
    initialize_foundation(FoundationConfig::default().with_assert_hook(hook)).unwrap();

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
fn run_lazy_default_child() {
    let version = boxdd::version();
    assert!(version.major >= 3);

    let first = boxdd::foundation();
    let second = boxdd::foundation();
    let explicit = initialize_foundation(FoundationConfig::default()).unwrap();

    assert!(core::ptr::eq(first, second));
    assert!(core::ptr::eq(first, explicit));
    assert_eq!(first.config().length_units_per_meter(), 1.0);
    assert_eq!(
        initialize_foundation(FoundationConfig::new(2.0)).unwrap_err(),
        FoundationInitError::ConfigurationConflict
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
            initialize_foundation(FoundationConfig::new(scale))
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
            .filter(|result| { matches!(result, Err(FoundationInitError::ConfigurationConflict)) })
            .count(),
        1
    );
    let winning_scale = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(
        unsafe { boxdd_sys::ffi::b2GetLengthUnitsPerMeter() },
        winning_scale
    );
}
