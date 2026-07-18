use std::{
    env, fs,
    path::{Path, PathBuf},
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
pub(super) fn replace_dir_under(dir: &Path, allowed_root: &Path) -> Result<()> {
    fs::create_dir_all(allowed_root).map_err(|source| Error::io(allowed_root, source))?;
    if dir.exists() {
        let canonical_dir = dir
            .canonicalize()
            .map_err(|source| Error::io(dir, source))?;
        let canonical_root = allowed_root
            .canonicalize()
            .map_err(|source| Error::io(allowed_root, source))?;
        if !canonical_dir.starts_with(&canonical_root) {
            return Err(Error::Message(format!(
                "refusing to remove directory outside {}: {}",
                canonical_root.display(),
                canonical_dir.display()
            )));
        }
        fs::remove_dir_all(&canonical_dir).map_err(|source| Error::io(&canonical_dir, source))?;
    }
    fs::create_dir_all(dir).map_err(|source| Error::io(dir, source))
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
}
