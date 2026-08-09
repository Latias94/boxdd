//! Xtask-owned subprocess wrappers.

use std::{
    io::Write as _,
    process::{Child, Command, ExitStatus, Output, Stdio},
};

fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) fn run_status(command: &mut Command, label: &str) -> Result<ExitStatus, String> {
    command
        .status()
        .map_err(|error| format!("failed to run {label}: {error}"))
}

pub(crate) fn run_output(command: &mut Command, label: &str) -> Result<Output, String> {
    command
        .output()
        .map_err(|error| format!("failed to run {label}: {error}"))
}

pub(crate) fn run_output_with_input(
    command: &mut Command,
    input: &[u8],
    label: &str,
) -> Result<Output, String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start {label}: {error}"))?;
    let Some(mut stdin) = child.stdin.take() else {
        terminate_and_reap(&mut child);
        return Err(format!("failed to open {label} stdin"));
    };
    if let Err(error) = stdin.write_all(input) {
        drop(stdin);
        terminate_and_reap(&mut child);
        return Err(format!("failed to write {label} stdin: {error}"));
    }
    drop(stdin);
    child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for {label}: {error}"))
}
