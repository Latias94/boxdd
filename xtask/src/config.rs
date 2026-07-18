use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Serialize, de::DeserializeOwned};
use tempfile::NamedTempFile;

use crate::{Error, Result};

pub const API_CONTRACT_SCHEMA: u32 = 4;
pub const UPSTREAM_MANIFEST_SCHEMA: u32 = 3;
pub const RECORDING_WIRE_SCHEMA: u32 = 4;

pub fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let source = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    toml::from_str(&source)
        .map_err(|error| Error::message(format!("{}: invalid TOML: {error}", path.display())))
}

pub fn render_toml<T: Serialize>(value: &T) -> Result<String> {
    toml::to_string_pretty(value)
        .map_err(|error| Error::message(format!("could not serialize TOML: {error}")))
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    write_atomic_bytes(path, content.as_bytes())
}

pub fn write_atomic_bytes(path: &Path, content: &[u8]) -> Result<()> {
    write_atomic_bytes_with(path, content, || Ok(()))
}

pub(crate) fn write_atomic_bytes_with<F>(
    path: &Path,
    content: &[u8],
    before_commit: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(Error::message(format!(
                "{} is not a regular file and cannot be replaced",
                path.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io(path, error)),
    }

    let target_permissions = fs::metadata(path)
        .map(|metadata| Some(metadata.permissions()))
        .or_else(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(default_file_permissions())
            } else {
                Err(error)
            }
        })
        .map_err(|source| Error::io(path, source))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    if let Some(target_permissions) = target_permissions {
        temporary
            .as_file()
            .set_permissions(target_permissions)
            .map_err(|source| Error::io(temporary.path(), source))?;
    }
    temporary
        .as_file_mut()
        .write_all(content)
        .map_err(|source| Error::io(path, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(path, source))?;
    before_commit()?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| Error::io(path, error.error))
}

pub(crate) fn write_new_bytes_noclobber(
    path: &Path,
    content: &[u8],
    permissions: Option<fs::Permissions>,
) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(Error::message(format!(
                "{} already exists; refusing to overwrite a concurrent state",
                path.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io(path, error)),
    }
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    if let Some(permissions) = permissions.or_else(default_file_permissions) {
        temporary
            .as_file()
            .set_permissions(permissions)
            .map_err(|source| Error::io(temporary.path(), source))?;
    }
    temporary
        .as_file_mut()
        .write_all(content)
        .map_err(|source| Error::io(path, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(path, source))?;
    temporary
        .persist_noclobber(path)
        .map(|_| ())
        .map_err(|error| Error::io(path, error.error))
}

#[cfg(unix)]
fn default_file_permissions() -> Option<fs::Permissions> {
    use std::os::unix::fs::PermissionsExt as _;

    Some(fs::Permissions::from_mode(0o644))
}

#[cfg(not(unix))]
fn default_file_permissions() -> Option<fs::Permissions> {
    None
}

pub fn normalized(path: impl Into<PathBuf>) -> PathBuf {
    path.into().components().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_an_existing_file() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-write-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("contract.toml");
        fs::write(&path, "old").expect("old fixture");

        write_atomic(&path, "new").expect("replace existing file");

        assert_eq!(fs::read_to_string(&path).expect("updated fixture"), "new");
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0, "temporary files must be cleaned up");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn failed_precommit_preserves_original_and_cleans_staging_file() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-failure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("contract.toml");
        fs::write(&path, "old").expect("old fixture");

        let error = write_atomic_bytes_with(&path, b"new", || {
            Err(Error::message("injected failure before atomic commit"))
        })
        .expect_err("precommit failure");

        assert!(error.to_string().contains("injected failure"));
        assert_eq!(fs::read_to_string(&path).expect("preserved fixture"), "old");
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0, "staging file must be discarded");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn atomic_write_rejects_non_regular_targets_without_mutation() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-nonregular-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let target = directory.join("target");
        fs::create_dir_all(&target).expect("directory target");

        let error = write_atomic(&target, "new").expect_err("directory target must fail");

        assert!(error.to_string().contains("not a regular file"));
        assert!(target.is_dir());
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn failed_atomic_replacement_cleans_staging_file() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-replace-failure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("contract.toml");
        fs::write(&path, "old").expect("old fixture");

        let error = write_atomic_bytes_with(&path, b"new", || {
            fs::remove_file(&path).map_err(|source| Error::io(&path, source))?;
            fs::create_dir(&path).map_err(|source| Error::io(&path, source))?;
            Ok(())
        })
        .expect_err("replacing a directory must fail");

        assert!(error.to_string().contains("contract.toml"));
        assert!(path.is_dir());
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0, "failed persist must discard its staging file");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replacement_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-permissions-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("artifact");
        fs::write(&path, "old").expect("old fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).expect("fixture permissions");

        write_atomic(&path, "new").expect("atomic replacement");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o754);
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn noclobber_write_preserves_a_concurrent_target_and_cleans_staging() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-noclobber-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("artifact");
        fs::write(&path, "concurrent").expect("concurrent target");

        let error = write_new_bytes_noclobber(&path, b"transaction", None)
            .expect_err("noclobber write must reject an existing path");

        assert!(error.to_string().contains("refusing to overwrite"));
        assert_eq!(
            fs::read_to_string(&path).expect("preserved target"),
            "concurrent"
        );
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0);
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }
}
