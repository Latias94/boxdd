use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use crate::{Error, Result};

const PAGES_WASM_PROFILE_ENV: &str = "BOXDD_PAGES_WASM_PROFILE";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";

#[derive(Copy, Clone)]
pub(super) enum BuildProfile {
    Debug,
    Release,
    WasmRelease,
}

impl BuildProfile {
    pub(super) fn for_provider_smoke() -> Result<Self> {
        Self::from_env_or(Self::Debug)
    }

    pub(super) fn for_pages() -> Result<Self> {
        Self::from_env_or(Self::WasmRelease)
    }

    fn from_env_or(default: Self) -> Result<Self> {
        match env::var(PAGES_WASM_PROFILE_ENV) {
            Ok(value) => Self::parse(&value).ok_or_else(|| {
                Error::Message(format!(
                    "invalid {PAGES_WASM_PROFILE_ENV} value `{value}`; expected debug, release, or wasm-release"
                ))
            }),
            Err(_) => Ok(default),
        }
    }

    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "debug" | "Debug" | "DEBUG" => Some(Self::Debug),
            "release" | "Release" | "RELEASE" => Some(Self::Release),
            "wasm-release" | "WASM-RELEASE" | "wasm_release" | "WASM_RELEASE" => {
                Some(Self::WasmRelease)
            }
            _ => None,
        }
    }

    pub(super) const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Release => &["--release"],
            Self::WasmRelease => &["--profile", "wasm-release"],
        }
    }

    pub(super) const fn target_dir(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
            Self::WasmRelease => "wasm-release",
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
            Self::WasmRelease => "wasm-release",
        }
    }
}

pub(super) fn add_wasm_app_link_args(command: &mut Command, export_groups: &[&[&str]]) {
    command.arg("--").arg("-C").arg("link-arg=--import-memory");
    for export in export_groups.iter().flat_map(|exports| exports.iter()) {
        command.arg("-C").arg(format!("link-arg=--export={export}"));
    }
}

pub(super) fn run_command(command: &mut Command, label: &str) -> Result<()> {
    let status = command
        .status()
        .map_err(|source| Error::io(label, source))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} failed with status {status}"
        )))
    }
}

pub(super) fn cargo_target_dir(root: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|source| Error::io("cargo metadata", source))?;
    if !output.status.success() {
        return Err(Error::Message(format!(
            "cargo metadata failed while resolving target_directory: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    parse_cargo_target_dir(&output.stdout)
}

fn parse_cargo_target_dir(metadata: &[u8]) -> Result<PathBuf> {
    let value: serde_json::Value = serde_json::from_slice(metadata).map_err(|error| {
        Error::Message(format!(
            "cargo metadata returned invalid JSON while resolving target_directory: {error}"
        ))
    })?;
    let target_directory = value
        .get("target_directory")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| Error::Message("cargo metadata omitted target_directory".to_owned()))?;
    Ok(PathBuf::from(target_directory))
}

pub(super) fn replace_dir_under(dir: &Path, allowed_root: &Path) -> Result<()> {
    fs::create_dir_all(allowed_root).map_err(|source| Error::io(allowed_root, source))?;
    require_real_directory(allowed_root, "allowed replacement root")?;
    let canonical_root = allowed_root
        .canonicalize()
        .map_err(|source| Error::io(allowed_root, source))?;
    let relative = dir.strip_prefix(allowed_root).map_err(|_| {
        Error::Message(format!(
            "refusing to replace directory outside {}: {}",
            allowed_root.display(),
            dir.display()
        ))
    })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Message(format!(
            "replacement directory must be a canonical child of {}: {}",
            allowed_root.display(),
            dir.display()
        )));
    }

    let parent = dir.parent().ok_or_else(|| {
        Error::Message(format!(
            "replacement directory has no parent: {}",
            dir.display()
        ))
    })?;
    ensure_real_directory_tree(allowed_root, parent, &canonical_root)?;

    match fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            let canonical_dir = dir
                .canonicalize()
                .map_err(|source| Error::io(dir, source))?;
            if !canonical_dir.starts_with(&canonical_root) {
                return Err(Error::Message(format!(
                    "refusing to remove directory outside {}: {}",
                    canonical_root.display(),
                    canonical_dir.display()
                )));
            }
            fs::remove_dir_all(dir).map_err(|source| Error::io(dir, source))?;
        }
        Ok(_) => {
            return Err(Error::Message(format!(
                "replacement target must be a real directory: {}",
                dir.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(Error::io(dir, source)),
    }
    fs::create_dir(dir).map_err(|source| Error::io(dir, source))
}

fn ensure_real_directory_tree(root: &Path, dir: &Path, canonical_root: &Path) -> Result<()> {
    let relative = dir.strip_prefix(root).map_err(|_| {
        Error::Message(format!(
            "directory must remain under {}: {}",
            root.display(),
            dir.display()
        ))
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(Error::Message(format!(
                "directory path is not canonical: {}",
                dir.display()
            )));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            }
            Ok(_) => {
                return Err(Error::Message(format!(
                    "directory tree contains a symlink or non-directory: {}",
                    current.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| Error::io(&current, source))?;
            }
            Err(source) => return Err(Error::io(&current, source)),
        }
        let canonical = current
            .canonicalize()
            .map_err(|source| Error::io(&current, source))?;
        if !canonical.starts_with(canonical_root) {
            return Err(Error::Message(format!(
                "directory escaped {}: {}",
                canonical_root.display(),
                canonical.display()
            )));
        }
    }
    Ok(())
}

fn require_real_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} must be a real directory: {}",
            path.display()
        )))
    }
}

pub(super) fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    fs::copy(from, to).map_err(|source| Error::io(to, source))?;
    Ok(())
}

pub(super) fn ensure_file(path: &Path, label: &str) -> Result<PathBuf> {
    if path.is_file() {
        Ok(path.to_path_buf())
    } else {
        Err(Error::Message(format!(
            "{label} not found: {}",
            path.display()
        )))
    }
}
pub(super) fn ensure_runnable_tool(tool: &str, version_arg: &str, message: &str) -> Result<()> {
    if runnable_tool(tool, version_arg).is_some() {
        Ok(())
    } else {
        Err(Error::Message(message.to_owned()))
    }
}

pub(super) fn runnable_tool(tool: &str, version_arg: &str) -> Option<PathBuf> {
    runnable_path(Path::new(tool), version_arg).map(|_| PathBuf::from(tool))
}

pub(super) fn runnable_path(path: &Path, version_arg: &str) -> Option<PathBuf> {
    Command::new(path)
        .arg(version_arg)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_profile_parses_supported_values() {
        assert!(matches!(
            BuildProfile::parse("debug"),
            Some(BuildProfile::Debug)
        ));
        assert!(matches!(
            BuildProfile::parse("release"),
            Some(BuildProfile::Release)
        ));
        assert!(matches!(
            BuildProfile::parse("wasm-release"),
            Some(BuildProfile::WasmRelease)
        ));
        assert!(matches!(
            BuildProfile::parse("WASM_RELEASE"),
            Some(BuildProfile::WasmRelease)
        ));
        assert!(BuildProfile::parse("fast").is_none());
    }

    #[test]
    fn wasm_release_profile_uses_custom_cargo_profile() {
        assert_eq!(
            BuildProfile::WasmRelease.cargo_args(),
            &["--profile", "wasm-release"]
        );
        assert_eq!(BuildProfile::WasmRelease.target_dir(), "wasm-release");
    }

    #[test]
    fn cargo_metadata_target_directory_is_required_and_structured() {
        assert_eq!(
            parse_cargo_target_dir(br#"{"target_directory":"/tmp/boxdd-target"}"#).unwrap(),
            PathBuf::from("/tmp/boxdd-target")
        );
        assert!(parse_cargo_target_dir(br#"{"packages":[]}"#).is_err());
        assert!(parse_cargo_target_dir(b"not-json").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn replace_dir_rejects_a_symlink_without_removing_its_target() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::tempdir().unwrap();
        let allowed = fixture.path().join("pages");
        let outside = fixture.path().join("outside");
        fs::create_dir_all(&allowed).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("preserved.txt"), b"preserved").unwrap();
        let target = allowed.join("generated");
        symlink(&outside, &target).unwrap();

        assert!(replace_dir_under(&target, &allowed).is_err());
        assert_eq!(
            fs::read(outside.join("preserved.txt")).unwrap(),
            b"preserved"
        );
        assert!(
            fs::symlink_metadata(target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
