//! Bounded snapshots of regular filesystem inputs.

use sha2::{Digest as _, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// One bounded byte sequence read from a regular, non-symlink file.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedFileSnapshot {
    path: PathBuf,
    bytes: Vec<u8>,
    sha256: String,
}

impl VerifiedFileSnapshot {
    pub fn read(path: &Path, maximum_bytes: u64, label: &str) -> Result<Self, String> {
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn verify_sha256(&self, expected_sha256: &str, label: &str) -> Result<(), String> {
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

    pub fn verify_exact(
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

    pub fn revalidate(&self, label: &str) -> Result<(), String> {
        verify_exact_file(&self.path, &self.sha256, &self.bytes, label)
    }

    pub(super) fn rebind_path(&mut self, path: &Path) {
        self.path = path.to_path_buf();
    }
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

pub(super) fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn verify_exact_file(
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
