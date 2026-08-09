//! Verify that a panic in a real native callback aborts an abort-profile subprocess.

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_callback_panic_aborts_without_returning_to_box2d() {
    use std::path::PathBuf;
    use std::process::Command;

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("boxdd crate must have a workspace parent")
        .to_owned();
    let manifest = workspace_root.join("tools/panic-abort-probe/Cargo.toml");
    let target_dir = workspace_root.join("target");
    let target_dir = target_dir.join(if cfg!(feature = "double-precision") {
        "panic-abort-probe-double"
    } else {
        "panic-abort-probe-single"
    });

    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args(["run", "--quiet", "--locked", "--offline", "--manifest-path"])
        .arg(&manifest)
        .args([
            "--profile",
            "panic-abort-probe",
            "--bin",
            "boxdd-panic-abort-probe",
        ])
        .env("CARGO_TARGET_DIR", target_dir);
    if cfg!(feature = "double-precision") {
        command.args(["--features", "double-precision"]);
    }

    let output = command
        .output()
        .expect("panic=abort probe cargo process must start");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "panic=abort callback probe unexpectedly returned successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr
    );
    assert!(
        stderr.contains("boxdd-panic-abort-probe: callback-entered"),
        "the native query did not reach the Rust callback\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr
    );
    assert!(
        !stderr.contains("boxdd-panic-abort-probe: after-query"),
        "native execution continued after the Rust callback panic\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr
    );
}
