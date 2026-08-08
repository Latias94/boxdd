//! Small, deterministic Git command helpers used by repository tooling.

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use tempfile::TempDir;

use crate::subprocess_policy::run_output;

static EMPTY_HOOKS_DIRECTORY: OnceLock<Result<TempDir, String>> = OnceLock::new();

pub(crate) fn isolated_git_command() -> Result<Command, String> {
    let hooks = EMPTY_HOOKS_DIRECTORY
        .get_or_init(|| tempfile::tempdir().map_err(|error| error.to_string()))
        .as_ref()
        .map_err(Clone::clone)?;
    let mut command = Command::new("git");
    remove_matching_environment(&mut command, is_git_environment_key);
    remove_process_injection_environment(&mut command);
    isolate_git_configuration(&mut command);
    command
        .arg("--no-replace-objects")
        .args(["-c", "core.fsmonitor=false"])
        .arg("-c")
        .arg(format!("core.hooksPath={}", hooks.path().display()));
    Ok(command)
}

pub(crate) fn repository_lock_path(root: &Path, file_name: &Path) -> Result<PathBuf, String> {
    let mut command = isolated_git_command()?;
    command
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"]);
    let output = run_output(&mut command, "query the Git common directory")?;
    if !output.status.success() {
        return Err(format!(
            "failed to query the Git common directory: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let common_dir = PathBuf::from(
        String::from_utf8(output.stdout)
            .map_err(|error| format!("Git common directory is not UTF-8: {error}"))?
            .trim(),
    );
    if !common_dir.is_absolute() {
        return Err(format!(
            "Git returned an invalid common directory: {}",
            common_dir.display()
        ));
    }
    lock_path_in_common_dir(&common_dir, file_name)
}

fn lock_path_in_common_dir(common_dir: &Path, relative: &Path) -> Result<PathBuf, String> {
    let common_dir = canonical_real_directory(common_dir, "Git common directory")?;
    validate_normal_relative_path(relative, "repository-owned lock path")?;
    let path = common_dir.join(relative);
    let lexical_relative = path.strip_prefix(&common_dir).map_err(|_| {
        format!(
            "repository-owned lock path escapes Git common directory {}: {}",
            common_dir.display(),
            path.display()
        )
    })?;
    if lexical_relative != relative {
        return Err(format!(
            "repository-owned lock path is not lexically contained by Git common directory {}: {}",
            common_dir.display(),
            path.display()
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "repository-owned lock path has no parent directory: {}",
            path.display()
        )
    })?;
    let canonical_parent = canonical_real_directory(parent, "repository-owned lock parent")?;
    if !canonical_parent.starts_with(&common_dir) {
        return Err(format!(
            "repository-owned lock parent escapes Git common directory {}: {}",
            common_dir.display(),
            canonical_parent.display()
        ));
    }
    let file_name = path.file_name().ok_or_else(|| {
        format!(
            "repository-owned lock path has no file name: {}",
            path.display()
        )
    })?;
    let path = canonical_parent.join(file_name);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if !metadata.file_type().is_file() || metadata.file_type().is_symlink() => {
            Err(format!(
                "repository-owned lock must be a regular non-symlink file: {}",
                path.display()
            ))
        }
        Ok(_) => {
            let canonical = fs::canonicalize(&path).map_err(|error| {
                format!(
                    "failed to resolve repository-owned lock {}: {error}",
                    path.display()
                )
            })?;
            if !canonical.starts_with(&common_dir) {
                return Err(format!(
                    "repository-owned lock escapes Git common directory {}: {}",
                    common_dir.display(),
                    canonical.display()
                ));
            }
            Ok(canonical)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(format!(
            "failed to inspect repository-owned lock {}: {error}",
            path.display()
        )),
    }
}

fn canonical_real_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must be a real directory: {}",
            path.display()
        ));
    }
    fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
}

fn validate_normal_relative_path(path: &Path, label: &str) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(format!(
            "{label} must be a non-empty normalized relative path: {}",
            path.display()
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn remove_process_injection_environment(command: &mut Command) {
    remove_matching_environment(command, is_process_injection_environment_key);
}

pub(crate) fn is_process_injection_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("LD_")
        || key.starts_with("DYLD_")
        || matches!(key.as_str(), "LIBPATH" | "SHLIB_PATH" | "BASH_ENV" | "ENV")
}

pub(crate) fn isolate_git_configuration(command: &mut Command) {
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_OPTIONAL_LOCKS", "0");
}

pub(crate) fn is_git_environment_key(key: &OsStr) -> bool {
    key.to_string_lossy()
        .to_ascii_uppercase()
        .starts_with("GIT_")
}

pub(crate) fn remove_matching_environment(
    command: &mut Command,
    predicate: impl Fn(&OsStr) -> bool,
) {
    let configured = command
        .get_envs()
        .map(|(key, _)| key.to_owned())
        .collect::<Vec<_>>();
    for key in env::vars_os().map(|(key, _)| key).chain(configured) {
        if predicate(&key) {
            command.env_remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn command_ignores_repository_redirection() {
        let mut command = isolated_git_command().unwrap();
        command.env("GIT_DIR", OsString::from("elsewhere"));
        remove_matching_environment(&mut command, is_git_environment_key);
        assert!(
            command
                .get_envs()
                .any(|(name, value)| { name == OsStr::new("GIT_DIR") && value.is_none() })
        );
    }

    #[test]
    fn process_injection_environment_is_removed_from_commands() {
        let mut command = Command::new("ignored");
        for key in ["LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "BASH_ENV"] {
            command.env(key, "payload");
        }
        remove_process_injection_environment(&mut command);
        for key in ["LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "BASH_ENV"] {
            assert!(
                command
                    .get_envs()
                    .any(|(name, value)| name == OsStr::new(key) && value.is_none()),
                "{key} was not removed"
            );
        }
    }

    #[test]
    fn lock_path_uses_the_repository_common_directory() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let lock = repository_lock_path(root, Path::new("boxdd-test.lock")).unwrap();
        assert_eq!(lock.file_name(), Some(OsStr::new("boxdd-test.lock")));
        assert!(lock.parent().is_some_and(Path::is_dir));
    }

    #[cfg(unix)]
    #[test]
    fn lock_paths_reject_symlinked_files_and_parents() {
        use std::os::unix::fs::symlink;

        let common = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), common.path().join("nested")).unwrap();
        symlink(outside.path().join("victim"), common.path().join("lock")).unwrap();

        assert!(lock_path_in_common_dir(common.path(), Path::new("nested/lock")).is_err());
        assert!(lock_path_in_common_dir(common.path(), Path::new("../lock")).is_err());
        assert!(lock_path_in_common_dir(common.path(), Path::new("lock")).is_err());
    }
}
