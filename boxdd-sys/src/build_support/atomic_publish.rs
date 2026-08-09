//! Small create-new and atomically visible helpers for build artifacts.
//!
//! Published files recover on the next invocation after interruption. They are not a power-loss
//! durability protocol, so this module intentionally does not fsync files or parent directories.

use sha2::{Digest as _, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::Path,
};

use super::verified_snapshot::{VerifiedFileSnapshot, is_canonical_sha256, verify_exact_file};

const MAX_ATOMIC_PUBLISH_ATTEMPTS: usize = 32;

pub(crate) fn snapshot_file_create_new(
    source: &Path,
    destination: &Path,
    maximum_bytes: u64,
    label: &str,
) -> Result<VerifiedFileSnapshot, String> {
    let mut snapshot = VerifiedFileSnapshot::read(source, maximum_bytes, label)?;
    if snapshot.is_empty() {
        return Err(format!(
            "{label} size is outside the accepted 1..={maximum_bytes} byte range"
        ));
    }
    write_create_new(destination, label, |output| {
        output
            .write_all(snapshot.bytes())
            .map_err(|error| format!("failed to write {label} snapshot: {error}"))
    })?;
    verify_exact_file(destination, snapshot.sha256(), snapshot.bytes(), label)?;
    snapshot.rebind_path(destination);
    Ok(snapshot)
}

pub(crate) fn generate_file_create_new<F>(
    destination: &Path,
    maximum_bytes: u64,
    label: &str,
    writer: F,
) -> Result<VerifiedFileSnapshot, String>
where
    F: FnOnce(&mut fs::File) -> Result<(), String>,
{
    write_create_new(destination, label, writer)?;
    match VerifiedFileSnapshot::read(destination, maximum_bytes, label) {
        Ok(snapshot) if !snapshot.is_empty() => Ok(snapshot),
        Ok(_) => {
            let _ = fs::remove_file(destination);
            Err(format!("generated {label} must not be empty"))
        }
        Err(error) => {
            let _ = fs::remove_file(destination);
            Err(error)
        }
    }
}

pub(crate) fn publish_verified_file(
    destination: &Path,
    expected_sha256: &str,
    bytes: &[u8],
    label: &str,
) -> Result<(), String> {
    require_sha256(expected_sha256, bytes, label)?;
    require_real_parent(destination, label)?;

    if verify_exact_file(destination, expected_sha256, bytes, label).is_ok() {
        return Ok(());
    }

    let parent = destination.parent().expect("validated parent above");
    if destination.file_name().is_none() {
        return Err(format!(
            "verified {label} destination {} has no file name",
            destination.display()
        ));
    }
    let mut temporary = tempfile::Builder::new()
        // Keep the temporary name independent of the destination name. On Windows, tempfile's
        // persist implementation passes this path directly to MoveFileExW, where a long bindings
        // digest name can otherwise push the temporary source beyond MAX_PATH.
        .prefix(".boxdd-tmp-")
        .tempfile_in(parent)
        .map_err(|error| {
            format!(
                "failed to reserve temporary verified {label} beside {}: {error}",
                destination.display()
            )
        })?;
    temporary.as_file_mut().write_all(bytes).map_err(|error| {
        format!(
            "failed to write temporary verified {label} {}: {error}",
            temporary.path().display()
        )
    })?;
    let mut last_error = None;
    for _ in 0..MAX_ATOMIC_PUBLISH_ATTEMPTS {
        match temporary.persist(destination) {
            Ok(_) => return verify_exact_file(destination, expected_sha256, bytes, label),
            Err(error) => {
                last_error = Some(error.error.to_string());
                temporary = error.file;
            }
        }

        // Windows may temporarily reject replacement while another publisher is validating the
        // destination. An exact winner is success; otherwise retry the atomic replacement. Never
        // remove the destination here: another publisher may have replaced the incomplete file
        // after our failed validation, and deleting that winner creates a transient missing path.
        if verify_exact_file(destination, expected_sha256, bytes, label).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    Err(format!(
        "failed to atomically publish verified {label} {} after {MAX_ATOMIC_PUBLISH_ATTEMPTS} attempts: {}",
        destination.display(),
        last_error.as_deref().unwrap_or("unknown publication error")
    ))
}

fn write_create_new<F>(destination: &Path, label: &str, writer: F) -> Result<(), String>
where
    F: FnOnce(&mut fs::File) -> Result<(), String>,
{
    require_real_parent(destination, label)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "refusing to replace existing {label} {}",
                    destination.display()
                )
            } else {
                format!(
                    "failed to create {label} {}: {error}",
                    destination.display()
                )
            }
        })?;
    if let Err(error) = writer(&mut output) {
        drop(output);
        let _ = fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

fn require_real_parent(destination: &Path, label: &str) -> Result<(), String> {
    let parent = destination.parent().ok_or_else(|| {
        format!(
            "{label} destination {} has no parent directory",
            destination.display()
        )
    })?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        format!(
            "failed to inspect {label} parent {}: {error}",
            parent.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!(
            "{label} parent {} must be a real directory",
            parent.display()
        ));
    }
    Ok(())
}

fn require_sha256(expected: &str, bytes: &[u8], label: &str) -> Result<(), String> {
    if !is_canonical_sha256(expected) {
        return Err(format!(
            "verified {label} expected SHA-256 must be 64 lowercase hexadecimal bytes"
        ));
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!(
            "verified {label} SHA-256 mismatch: expected {expected}, found {actual}"
        ));
    }
    Ok(())
}
