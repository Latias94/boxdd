use std::{fs, io::Write as _, path::Path};

use serde::{Serialize, de::DeserializeOwned};
use tempfile::NamedTempFile;

use crate::{Error, Result};

pub const RECORDING_WIRE_SCHEMA: u32 = 5;

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
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;

    let permissions = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Some(metadata.permissions()),
        Ok(_) => {
            return Err(Error::message(format!(
                "{} is not a regular file and cannot be replaced",
                path.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(Error::io(path, error)),
    };

    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    if let Some(permissions) = permissions {
        temporary
            .as_file()
            .set_permissions(permissions)
            .map_err(|source| Error::io(temporary.path(), source))?;
    }
    temporary
        .write_all(content)
        .map_err(|source| Error::io(temporary.path(), source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(temporary.path(), source))?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| Error::io(path, error.error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_and_replaces_regular_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/value.txt");

        write_atomic(&path, "first").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");

        write_atomic(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn atomic_write_rejects_non_file_targets() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("target");
        fs::create_dir(&path).unwrap();

        let error = write_atomic(&path, "value").unwrap_err();
        assert!(error.to_string().contains("not a regular file"));
    }
}
