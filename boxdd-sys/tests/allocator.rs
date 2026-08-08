const NULL_ALLOCATOR_CHILD: &str = "BOXDD_TEST_NULL_ALLOCATOR_CHILD";
const CONTROL_ABORT_CHILD: &str = "BOXDD_TEST_CONTROL_ABORT_CHILD";

unsafe extern "C" fn return_null(
    _size: usize,
    _alignment: std::os::raw::c_int,
) -> *mut std::os::raw::c_void {
    std::ptr::null_mut()
}

unsafe extern "C" fn ignore_free(_memory: *mut std::os::raw::c_void, _size: usize) {}

#[test]
fn allocator_failure_aborts_before_native_dereference() {
    if std::env::var_os(CONTROL_ABORT_CHILD).is_some() {
        std::process::abort();
    }
    if std::env::var_os(NULL_ALLOCATOR_CHILD).is_some() {
        unsafe {
            boxdd_sys::ffi::b2SetAllocator(Some(return_null), Some(ignore_free));
            let _ = boxdd_sys::ffi::b2DynamicTree_Create(1);
        }
        std::process::exit(99);
    }

    let run_child = |marker| {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "allocator_failure_aborts_before_native_dereference",
                "--nocapture",
            ])
            .env(marker, "1")
            .output()
            .unwrap()
    };

    let control = run_child(CONTROL_ABORT_CHILD);
    assert!(!control.status.success());
    let allocator = run_child(NULL_ALLOCATOR_CHILD);
    assert_ne!(allocator.status.code(), Some(99));
    assert_eq!(
        allocator.status,
        control.status,
        "allocator failure must terminate through abort, not a later invalid dereference; stderr={}",
        String::from_utf8_lossy(&allocator.stderr)
    );
}
