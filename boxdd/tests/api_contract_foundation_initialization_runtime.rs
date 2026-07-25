#[test]
fn foundation_initialization_installs_scale_and_hooks() {
    run_foundation_initialization_contract();
}

#[cfg(not(target_arch = "wasm32"))]
const ASSERT_HOOK_MARKER: &str = "boxdd-contract-assert-hook-ran";
#[cfg(not(target_arch = "wasm32"))]
const ASSERT_RETURNED_MARKER: &str = "boxdd-contract-assert-returned";
#[cfg(not(target_arch = "wasm32"))]
const LOG_HOOK_MARKER: &str = "boxdd-contract-log-hook-ran";
#[cfg(not(target_arch = "wasm32"))]
const LOG_RETURNED_MARKER: &str = "boxdd-contract-log-returned";

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" {
    fn b2Log(format: *const std::ffi::c_char, ...);
}

#[cfg(not(target_arch = "wasm32"))]
fn run_foundation_initialization_contract() {
    use std::{process::Command, sync::Arc};

    let assert_hook: Arc<boxdd::FoundationAssertHook> = Arc::new(|_, _, _| {});
    let log_hook: Arc<boxdd::FoundationLogHook> = Arc::new(|_| {});
    let config = boxdd::FoundationConfig::new(2.5)
        .with_assert_hook(assert_hook)
        .with_log_hook(log_hook);

    let foundation = boxdd::initialize_foundation(config)
        .expect("the first valid foundation configuration must initialize");

    assert_eq!(foundation.config().length_units_per_meter(), 2.5);
    assert!(foundation.config().has_assert_hook());
    assert!(foundation.config().has_log_hook());
    // SAFETY: foundation initialization completed above, so this read observes the frozen native
    // process configuration without racing a global mutation.
    assert_eq!(unsafe { boxdd_sys::ffi::b2GetLengthUnitsPerMeter() }, 2.5);

    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--ignored",
            "--exact",
            "foundation_native_hooks_child",
            "--nocapture",
        ])
        .output()
        .expect("run isolated native hook probe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "native assertion unexpectedly returned\nstdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains(LOG_HOOK_MARKER) && stderr.contains("native log probe"),
        "Box2D did not invoke the configured log hook\nstdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains(LOG_RETURNED_MARKER),
        "native log callback did not return normally\nstdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains(ASSERT_HOOK_MARKER),
        "Box2D did not invoke the configured assertion hook\nstdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !stderr.contains(ASSERT_RETURNED_MARKER),
        "native execution continued after the assertion hook requested a trap"
    );
}

#[cfg(target_arch = "wasm32")]
fn run_foundation_initialization_contract() {
    let foundation = boxdd::initialize_foundation(boxdd::FoundationConfig::new(2.5))
        .expect("the first valid foundation configuration must initialize");

    assert_eq!(foundation.config().length_units_per_meter(), 2.5);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "run only in an isolated child process"]
fn foundation_native_hooks_child() {
    use std::{ffi::CString, sync::Arc};

    let assert_hook: Arc<boxdd::FoundationAssertHook> = Arc::new(|condition, file_name, line| {
        eprintln!("{ASSERT_HOOK_MARKER}: {condition}, {file_name}:{line}");
    });
    let log_hook: Arc<boxdd::FoundationLogHook> = Arc::new(|message| {
        eprintln!("{LOG_HOOK_MARKER}: {message}");
    });
    boxdd::initialize_foundation(
        boxdd::FoundationConfig::default()
            .with_assert_hook(assert_hook)
            .with_log_hook(log_hook),
    )
    .expect("initialize isolated hook probe");

    let log_message = CString::new("native log probe").expect("log probe CString");
    // SAFETY: the format string is null-terminated, contains no conversion specifiers, and remains
    // valid for the duration of this synchronous native call.
    unsafe { b2Log(log_message.as_ptr()) };
    eprintln!("{LOG_RETURNED_MARKER}");

    let condition = CString::new("native assertion probe").expect("assert condition CString");
    let file_name = CString::new("api_contract_foundation_initialization_runtime.rs")
        .expect("assert file CString");
    // SAFETY: both pointers are valid null-terminated strings for the synchronous call. The child
    // process exists specifically to verify that the configured nonzero trampoline traps.
    unsafe {
        boxdd_sys::ffi::b2InternalAssert(condition.as_ptr(), file_name.as_ptr(), line!() as i32);
    }
    eprintln!("{ASSERT_RETURNED_MARKER}");
}
