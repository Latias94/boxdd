//! Qualified Git commands and repository-owned lock paths.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use tempfile::TempDir;

const GIT_HOOKS_ISOLATION_FAILURE_OPTION: &str = "--boxdd-git-hooks-isolation-failed";

static PROCESS_GIT_HOOKS_DIRECTORY: OnceLock<Result<ProcessGitHooksDirectory, String>> =
    OnceLock::new();

struct ProcessGitHooksDirectory {
    directory: TempDir,
    canonical_path: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl ProcessGitHooksDirectory {
    fn create() -> Result<Self, String> {
        let directory = tempfile::Builder::new()
            .prefix("boxdd-git-hooks-")
            .tempdir()
            .map_err(|error| {
                format!("failed to create the isolated Git hooks directory: {error}")
            })?;
        restrict_directory_to_owner(directory.path())?;
        let canonical_path =
            canonical_real_directory(directory.path(), "isolated Git hooks directory")?;
        let metadata = fs::symlink_metadata(directory.path()).map_err(|error| {
            format!(
                "failed to inspect isolated Git hooks directory {}: {error}",
                directory.path().display()
            )
        })?;
        let qualified = Self {
            directory,
            canonical_path,
            #[cfg(unix)]
            device: directory_device(&metadata),
            #[cfg(unix)]
            inode: directory_inode(&metadata),
        };
        qualified.revalidate()?;
        Ok(qualified)
    }

    fn revalidate(&self) -> Result<&Path, String> {
        let path = self.directory.path();
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            format!(
                "failed to revalidate isolated Git hooks directory {}: {error}",
                path.display()
            )
        })?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "isolated Git hooks path must remain a real directory: {}",
                path.display()
            ));
        }
        validate_owner_only_directory_permissions(path, &metadata)?;
        validate_directory_identity(path, &metadata, self)?;
        let canonical = fs::canonicalize(path).map_err(|error| {
            format!(
                "failed to resolve isolated Git hooks directory {}: {error}",
                path.display()
            )
        })?;
        if canonical != self.canonical_path {
            return Err(format!(
                "isolated Git hooks directory changed after creation: expected {}, found {}",
                self.canonical_path.display(),
                canonical.display()
            ));
        }
        let mut entries = fs::read_dir(&canonical).map_err(|error| {
            format!(
                "failed to read isolated Git hooks directory {}: {error}",
                canonical.display()
            )
        })?;
        match entries.next() {
            None => Ok(&self.canonical_path),
            Some(Ok(entry)) => Err(format!(
                "isolated Git hooks directory must remain empty; found {}",
                entry.path().display()
            )),
            Some(Err(error)) => Err(format!(
                "failed to inspect an entry in isolated Git hooks directory {}: {error}",
                canonical.display()
            )),
        }
    }
}

#[cfg(unix)]
fn restrict_directory_to_owner(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        format!(
            "failed to restrict isolated Git hooks directory {} to mode 0700: {error}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn restrict_directory_to_owner(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn validate_owner_only_directory_permissions(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = metadata.permissions().mode() & 0o777;
    if mode == 0o700 {
        Ok(())
    } else {
        Err(format!(
            "isolated Git hooks directory must retain mode 0700; found {mode:04o} at {}",
            path.display()
        ))
    }
}

#[cfg(not(unix))]
fn validate_owner_only_directory_permissions(
    _path: &Path,
    _metadata: &fs::Metadata,
) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn directory_device(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    metadata.dev()
}

#[cfg(unix)]
fn directory_inode(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    metadata.ino()
}

#[cfg(unix)]
fn validate_directory_identity(
    path: &Path,
    metadata: &fs::Metadata,
    expected: &ProcessGitHooksDirectory,
) -> Result<(), String> {
    let device = directory_device(metadata);
    let inode = directory_inode(metadata);
    if device == expected.device && inode == expected.inode {
        Ok(())
    } else {
        Err(format!(
            "isolated Git hooks directory was replaced after creation: {}",
            path.display()
        ))
    }
}

#[cfg(not(unix))]
fn validate_directory_identity(
    _path: &Path,
    _metadata: &fs::Metadata,
    _expected: &ProcessGitHooksDirectory,
) -> Result<(), String> {
    Ok(())
}

fn qualified_git_hooks_directory() -> Result<&'static Path, String> {
    PROCESS_GIT_HOOKS_DIRECTORY
        .get_or_init(ProcessGitHooksDirectory::create)
        .as_ref()
        .map_err(|error| error.clone())?
        .revalidate()
}

pub(crate) fn qualified_git_executable() -> Result<PathBuf, String> {
    let candidates = system_git_candidates()?;
    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "Git must be available at a qualified system path; checked {}",
                candidates.join(", ")
            )
        })
        .and_then(|path| canonical_regular_file(path, "qualified system Git executable"))
}

pub(crate) fn qualified_git_command() -> Result<Command, String> {
    let git = qualified_git_executable()?;
    let hooks = qualified_git_hooks_directory()?;
    Ok(configured_git_command_with_hooks(
        &git,
        hooks,
        env::vars_os(),
    ))
}

pub(crate) fn configured_git_command(git: &Path) -> Command {
    match qualified_git_hooks_directory() {
        Ok(hooks) => configured_git_command_with_hooks(git, hooks, env::vars_os()),
        Err(_) => refused_git_command(git, env::vars_os()),
    }
}

fn configured_git_command_with_hooks(
    git: &Path,
    hooks: &Path,
    environment: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    let mut command = Command::new(git);
    configure_git_command_environment(&mut command, environment);
    configure_git_command_arguments(&mut command, hooks);
    command
}

fn configure_git_command_arguments(command: &mut Command, hooks: &Path) {
    let mut hooks_configuration = OsString::from("core.hooksPath=");
    hooks_configuration.push(hooks);
    command
        .arg("--no-replace-objects")
        .args(["-c", "core.fsmonitor=false"])
        .arg("-c")
        .arg(hooks_configuration);
}

fn refused_git_command(
    git: &Path,
    environment: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    let mut command = Command::new(git);
    configure_git_command_environment(&mut command, environment);
    command.arg(GIT_HOOKS_ISOLATION_FAILURE_OPTION);
    command
}

pub(crate) fn remove_process_injection_environment(command: &mut Command) {
    remove_matching_environment(
        command,
        env::vars_os(),
        is_process_injection_environment_key,
    );
}

pub(crate) fn is_process_injection_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("LD_")
        || key.starts_with("DYLD_")
        || matches!(key.as_str(), "LIBPATH" | "SHLIB_PATH" | "BASH_ENV" | "ENV")
}

pub(crate) fn repository_lock_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let common_dir = repository_common_dir(root)?;
    lock_path_in_common_dir(&common_dir, relative)
}

fn repository_common_dir(root: &Path) -> Result<PathBuf, String> {
    let mut command = qualified_git_command()?;
    common_dir_from_command(root, &mut command)
}

fn common_dir_from_command(root: &Path, command: &mut Command) -> Result<PathBuf, String> {
    let output = command
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|error| {
            format!(
                "failed to query the Git common directory for {}: {error}",
                root.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "failed to query the Git common directory for {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let raw = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git common directory is not UTF-8: {error}"))?;
    let path = PathBuf::from(raw.trim());
    if !path.is_absolute() {
        return Err(format!(
            "Git common directory must be an absolute path: {}",
            path.display()
        ));
    }
    canonical_real_directory(&path, "Git common directory")
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

fn configure_git_command_environment(
    command: &mut Command,
    environment: impl IntoIterator<Item = (OsString, OsString)>,
) {
    let environment = environment.into_iter().collect::<Vec<_>>();
    remove_matching_environment(command, environment.clone(), is_git_environment_key);
    remove_matching_environment(command, environment, is_process_injection_environment_key);
    isolate_git_configuration(command);
}

pub(crate) fn isolate_git_configuration(command: &mut Command) {
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_OPTIONAL_LOCKS", "0");
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

fn remove_matching_environment(
    command: &mut Command,
    environment: impl IntoIterator<Item = (OsString, OsString)>,
    predicate: fn(&OsStr) -> bool,
) {
    for (key, _) in environment {
        if predicate(&key) {
            command.env_remove(key);
        }
    }
}

pub(crate) fn is_git_environment_key(key: &OsStr) -> bool {
    key.to_string_lossy()
        .to_ascii_uppercase()
        .starts_with("GIT_")
}

fn canonical_regular_file(path: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
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

fn system_git_candidates() -> Result<&'static [&'static str], String> {
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        Ok(&["/usr/bin/git"])
    } else if cfg!(windows) {
        Ok(&[
            r"C:\Program Files\Git\cmd\git.exe",
            r"C:\Program Files\Git\bin\git.exe",
            r"C:\Program Files (x86)\Git\cmd\git.exe",
        ])
    } else {
        Err(format!(
            "Git qualification is not supported on build host {}",
            env::consts::OS
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialized_repository() -> tempfile::TempDir {
        let temporary = tempfile::tempdir().unwrap();
        let mut command = qualified_git_command().unwrap();
        let output = command.arg("init").arg(temporary.path()).output().unwrap();
        assert!(output.status.success());
        temporary
    }

    fn command_with_environment(git: &Path, environment: Vec<(OsString, OsString)>) -> Command {
        let hooks = qualified_git_hooks_directory().unwrap();
        let mut command = Command::new(git);
        for (key, value) in &environment {
            command.env(key, value);
        }
        configure_git_command_environment(&mut command, environment);
        configure_git_command_arguments(&mut command, hooks);
        command
    }

    #[test]
    fn qualified_git_commands_remove_git_and_process_injection_environment() {
        let git = qualified_git_executable().unwrap();
        let environment = [
            (OsString::from("GIT_DIR"), OsString::from("/tmp/alternate")),
            (
                OsString::from("GIT_WORK_TREE"),
                OsString::from("/tmp/worktree"),
            ),
            (
                OsString::from("GIT_OBJECT_DIRECTORY"),
                OsString::from("/tmp/objects"),
            ),
            (OsString::from("LD_PRELOAD"), OsString::from("/tmp/inject")),
        ]
        .into_iter()
        .collect();
        let command = command_with_environment(&git, environment);
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_OBJECT_DIRECTORY",
            "LD_PRELOAD",
        ] {
            assert!(
                command
                    .get_envs()
                    .any(|(name, value)| { name == OsStr::new(key) && value.is_none() })
            );
        }
        assert!(command.get_envs().any(|(name, value)| {
            name == OsStr::new("GIT_OPTIONAL_LOCKS") && value == Some(OsStr::new("0"))
        }));
        assert!(command.get_envs().any(|(name, value)| {
            name == OsStr::new("GIT_CONFIG_NOSYSTEM") && value == Some(OsStr::new("1"))
        }));
        assert!(command.get_envs().any(|(name, value)| {
            name == OsStr::new("GIT_CONFIG_GLOBAL") && value == Some(OsStr::new(null_device()))
        }));
    }

    #[test]
    fn qualified_git_commands_override_repository_hook_configuration() {
        let repository = initialized_repository();
        let hostile_hooks = tempfile::tempdir().unwrap();
        let git = qualified_git_executable().unwrap();

        let output = configured_git_command(&git)
            .arg("-C")
            .arg(repository.path())
            .args(["config", "--local", "core.hooksPath"])
            .arg(hostile_hooks.path())
            .output()
            .unwrap();
        assert!(output.status.success());

        let output = qualified_git_command()
            .unwrap()
            .arg("-C")
            .arg(repository.path())
            .args(["config", "--get", "core.hooksPath"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let hooks = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
        assert!(hooks.is_absolute());
        assert_ne!(hooks, hostile_hooks.path());
        assert_eq!(hooks, qualified_git_hooks_directory().unwrap());
        let metadata = fs::symlink_metadata(&hooks).unwrap();
        assert!(metadata.file_type().is_dir());
        assert!(!metadata.file_type().is_symlink());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
        }
        assert!(fs::read_dir(hooks).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn qualified_git_commits_do_not_execute_repository_or_configured_hooks() {
        use std::os::unix::fs::PermissionsExt;

        let repository = initialized_repository();
        let hostile_hooks = tempfile::tempdir().unwrap();
        let sentinel = repository.path().join("hook-executed");
        let hook = "#!/bin/sh\nprintf executed > \"$BOXDD_HOOK_SENTINEL\"\nexit 1\n";
        let repository_hook = repository.path().join(".git/hooks/pre-commit");
        let configured_hook = hostile_hooks.path().join("pre-commit");
        fs::write(&repository_hook, hook).unwrap();
        fs::set_permissions(&repository_hook, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(repository.path().join("tracked"), "tracked\n").unwrap();
        let output = qualified_git_command()
            .unwrap()
            .arg("-C")
            .arg(repository.path())
            .args(["add", "tracked"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = qualified_git_command()
            .unwrap()
            .env("BOXDD_HOOK_SENTINEL", &sentinel)
            .arg("-C")
            .arg(repository.path())
            .args(["-c", "user.name=boxdd verification"])
            .args(["-c", "user.email=verification@invalid"])
            .args(["commit", "--quiet", "-m", "verify hooks isolation"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!sentinel.exists());

        fs::write(&configured_hook, hook).unwrap();
        fs::set_permissions(&configured_hook, fs::Permissions::from_mode(0o700)).unwrap();
        let output = qualified_git_command()
            .unwrap()
            .arg("-C")
            .arg(repository.path())
            .args(["config", "--local", "core.hooksPath"])
            .arg(hostile_hooks.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        fs::write(repository.path().join("tracked"), "updated\n").unwrap();
        let output = qualified_git_command()
            .unwrap()
            .arg("-C")
            .arg(repository.path())
            .args(["add", "tracked"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = qualified_git_command()
            .unwrap()
            .env("BOXDD_HOOK_SENTINEL", &sentinel)
            .arg("-C")
            .arg(repository.path())
            .args(["-c", "user.name=boxdd verification"])
            .args(["-c", "user.email=verification@invalid"])
            .args([
                "commit",
                "--quiet",
                "-m",
                "verify configured hook isolation",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!sentinel.exists());
    }

    #[test]
    fn failed_hooks_isolation_refuses_to_run_git() {
        let git = qualified_git_executable().unwrap();
        let mut command = refused_git_command(&git, Vec::new());
        assert!(
            command
                .get_args()
                .any(|argument| argument == OsStr::new(GIT_HOOKS_ISOLATION_FAILURE_OPTION))
        );
        let output = command.arg("--version").output().unwrap();
        assert!(!output.status.success());
    }

    #[test]
    fn hooks_isolation_rejects_a_populated_directory() {
        let hooks = ProcessGitHooksDirectory::create().unwrap();
        fs::write(hooks.canonical_path.join("pre-commit"), "hostile\n").unwrap();
        assert!(hooks.revalidate().is_err());
    }

    #[test]
    fn global_excludes_cannot_hide_untracked_repository_inputs() {
        let repository = initialized_repository();
        let git = qualified_git_executable().unwrap();
        let global_config = repository.path().join("hostile-global-config");
        let excludes = repository.path().join("hostile-excludes");
        fs::write(&excludes, "hidden-input\n").unwrap();
        fs::write(repository.path().join("hidden-input"), "input\n").unwrap();

        let output = configured_git_command(&git)
            .args(["config", "--file"])
            .arg(&global_config)
            .arg("core.excludesFile")
            .arg(&excludes)
            .output()
            .unwrap();
        assert!(output.status.success());

        let output = command_with_environment(
            &git,
            vec![(
                OsString::from("GIT_CONFIG_GLOBAL"),
                global_config.into_os_string(),
            )],
        )
        .arg("-C")
        .arg(repository.path())
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .unwrap();
        assert!(output.status.success());
        let status = String::from_utf8(output.stdout).unwrap();
        assert!(status.lines().any(|line| line == "?? hidden-input"));
    }

    #[test]
    fn hostile_git_dir_cannot_redirect_the_common_directory_lock() {
        let repository = initialized_repository();
        let alternate = initialized_repository();
        let expected_common_dir = repository_common_dir(repository.path()).unwrap();
        let git = qualified_git_executable().unwrap();
        let mut command = command_with_environment(
            &git,
            vec![(
                OsString::from("GIT_DIR"),
                alternate.path().join(".git").into_os_string(),
            )],
        );
        let actual_common_dir = common_dir_from_command(repository.path(), &mut command).unwrap();
        assert_eq!(actual_common_dir, expected_common_dir);
        assert_eq!(
            lock_path_in_common_dir(&actual_common_dir, Path::new("boxdd-upstream-sync.lock"))
                .unwrap(),
            repository_lock_path(repository.path(), Path::new("boxdd-upstream-sync.lock")).unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn lock_paths_reject_symlinked_parents_and_escapes() {
        use std::os::unix::fs::symlink;

        let common = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), common.path().join("nested")).unwrap();
        symlink(outside.path().join("lock"), common.path().join("lock")).unwrap();

        assert!(lock_path_in_common_dir(common.path(), Path::new("nested/lock")).is_err());
        assert!(lock_path_in_common_dir(common.path(), Path::new("../lock")).is_err());
        assert!(lock_path_in_common_dir(common.path(), Path::new("lock")).is_err());
    }
}
