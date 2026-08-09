//! Xtask-owned bounded snapshots and create-new publication helpers.
//!
//! Similar primitives exist in `boxdd-sys` build support. They deliberately remain separate:
//! the published sys package must keep its build script self-contained, while this repository-only
//! crate must not source-link implementation modules from another package.

use sha2::{Digest as _, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
};

/// One bounded byte sequence read from a regular, non-symlink file.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct VerifiedFileSnapshot {
    path: PathBuf,
    bytes: Vec<u8>,
    sha256: String,
}

impl VerifiedFileSnapshot {
    pub(crate) fn read(path: &Path, maximum_bytes: u64, label: &str) -> Result<Self, String> {
        let metadata = regular_file_metadata(path, label)?;
        if metadata.len() > maximum_bytes {
            return Err(format!(
                "{label} {} exceeds the {maximum_bytes} byte limit (found {} bytes)",
                path.display(),
                metadata.len()
            ));
        }

        let bytes = fs::read(path)
            .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
        let actual_bytes = u64::try_from(bytes.len())
            .map_err(|_| format!("{label} {} is too large", path.display()))?;
        if actual_bytes > maximum_bytes {
            return Err(format!(
                "{label} {} exceeds the {maximum_bytes} byte limit (found {actual_bytes} bytes)",
                path.display()
            ));
        }

        let completed = regular_file_metadata(path, label)?;
        if completed.len() != actual_bytes {
            return Err(format!(
                "{label} {} changed while being read",
                path.display()
            ));
        }

        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        Ok(Self {
            path: path.to_path_buf(),
            bytes,
            sha256,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn verify_sha256(&self, expected_sha256: &str, label: &str) -> Result<(), String> {
        if !is_canonical_sha256(expected_sha256) {
            return Err(format!(
                "{label} expected SHA-256 must be 64 lowercase hexadecimal bytes"
            ));
        }
        if self.sha256 == expected_sha256 {
            Ok(())
        } else {
            Err(format!(
                "{label} SHA-256 mismatch for {}: expected {expected_sha256}, found {}",
                self.path.display(),
                self.sha256
            ))
        }
    }

    pub(crate) fn verify_exact(
        &self,
        expected_bytes: &[u8],
        expected_sha256: &str,
        label: &str,
    ) -> Result<(), String> {
        self.verify_sha256(expected_sha256, label)?;
        if self.bytes == expected_bytes {
            Ok(())
        } else {
            Err(format!(
                "{label} {} does not contain the expected exact bytes",
                self.path.display()
            ))
        }
    }

    pub(crate) fn revalidate(&self, label: &str) -> Result<(), String> {
        verify_exact_file(&self.path, &self.sha256, &self.bytes, label)
    }

    fn rebind_path(&mut self, path: &Path) {
        self.path = path.to_path_buf();
    }
}

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

fn regular_file_metadata(path: &Path, label: &str) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!(
            "{label} must identify a regular non-symlink file: {}",
            path.display()
        ));
    }
    Ok(metadata)
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn verify_exact_file(
    path: &Path,
    expected_sha256: &str,
    bytes: &[u8],
    label: &str,
) -> Result<(), String> {
    let maximum_bytes = u64::try_from(bytes.len())
        .map_err(|_| format!("verified {label} length does not fit in u64"))?;
    let snapshot = VerifiedFileSnapshot::read(path, maximum_bytes, &format!("verified {label}"))?;
    snapshot.verify_exact(bytes, expected_sha256, &format!("verified {label}"))
}
