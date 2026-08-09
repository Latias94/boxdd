use std::{
    collections::BTreeSet,
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
    process::Command,
};

use crate::{
    Error, Result, isolated_git::remove_process_injection_environment,
    subprocess_policy::run_status,
};

const PAGES_WASM_PROFILE_ENV: &str = "BOXDD_PAGES_WASM_PROFILE";
const WASM_RUST_TOOLCHAIN: &str = "1.97.1";
pub(super) const WASM_TARGET: &str = crate::wasm_provider_contract::CONSUMER_TARGET;
pub(super) const CARGO_SUBPROCESS_JOBS: &str = "1";

#[derive(Default)]
pub(super) struct CargoEnvironment {
    pub(super) remove: BTreeSet<OsString>,
    pub(super) values: Vec<(OsString, OsString)>,
}

impl CargoEnvironment {
    pub(super) fn fail_closed(cargo_home: &Path) -> Self {
        let mut environment = Self::default();
        for (key, _) in env::vars_os() {
            if is_cargo_injection_environment_key(&key) {
                environment.remove.insert(key);
            }
        }
        for key in [
            "BOXDD_SYS_PROVIDER",
            "BOX2D_LIB_DIR",
            "BOXDD_SYS_SYSTEM_MANIFEST",
            "BOXDD_SYS_PREBUILT_MANIFEST",
            "BOXDD_SYS_PREBUILT_PROVENANCE",
            "BOXDD_SYS_PREBUILT_BUNDLE",
            "BOXDD_SYS_PREBUILT_TRUSTED_ROOT",
            "BOXDD_SYS_COSIGN",
            "BOXDD_SYS_LINK_KIND",
            "BOXDD_SYS_SKIP_CC",
            "BOXDD_SYS_FORCE_BINDGEN",
            "BOXDD_SYS_BINDGEN_TARGET",
            "BOXDD_SYS_PACKAGE_CRT",
            "BOXDD_SYS_PACKAGE_DIR",
            "BOXDD_SYS_PACKAGE_OUT_DIR",
            "BOXDD_SYS_PACKAGE_RELEASE_TAG",
            "BOXDD_SYS_PACKAGE_SOURCE_COMMIT",
            "BOXDD_NATIVE_QUALIFICATION_PROVIDER",
            "BOXDD_NATIVE_QUALIFICATION_MANIFEST_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_ARCHIVE_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_PROVENANCE_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_TRUSTED_ROOT_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_NONCE",
            "BOXDD_NATIVE_QUALIFICATION_RECEIPT",
            "RUSTFLAGS",
            "RUSTC",
            "RUSTC_BOOTSTRAP",
            "RUSTDOC",
            "RUSTDOCFLAGS",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTUP_HOME",
            "RUSTUP_TOOLCHAIN",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_INCREMENTAL",
            "CARGO_BUILD_RUSTFLAGS",
            "CARGO_BUILD_TARGET",
            "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
            "CARGO_BUILD_RUNNER",
            "CARGO_HOME",
            "CARGO_TARGET_DIR",
            "CFLAGS",
            "CPPFLAGS",
            "CC",
            "CXX",
            "AR",
            "LD",
            "CL",
            "RANLIB",
            "BINDGEN_EXTRA_CLANG_ARGS",
            "DOCS_RS",
            "CARGO_CFG_DOCSRS",
            "EMSDK",
            "BASH_ENV",
            "ENV",
            "LIBPATH",
            "SHLIB_PATH",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
        ] {
            environment.remove.insert(OsString::from(key));
        }
        environment.set("CARGO_HOME", cargo_home.as_os_str());
        environment
    }

    pub(super) fn set(&mut self, key: impl Into<OsString>, value: impl Into<OsString>) {
        self.values.push((key.into(), value.into()));
    }

    pub(super) fn apply(&self, command: &mut Command) {
        remove_process_injection_environment(command);
        for key in &self.remove {
            command.env_remove(key);
        }
        for (key, value) in &self.values {
            command.env(key, value);
        }
    }
}

pub(super) fn is_cargo_injection_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("BOXDD_SYS_")
        || key.starts_with("BOX2D_")
        || key.starts_with("BOXDD_NATIVE_QUALIFICATION_")
        || key.starts_with("CARGO_UNSTABLE_")
        || key.starts_with("CARGO_BUILD_")
        || key.starts_with("CFLAGS_")
        || key.starts_with("CPPFLAGS_")
        || key.starts_with("CC_")
        || key.starts_with("CXX_")
        || key.starts_with("AR_")
        || key.starts_with("LD_")
        || key.starts_with("RANLIB_")
        || key.starts_with("BINDGEN_EXTRA_CLANG_ARGS_")
        || key.starts_with("CARGO_TARGET_")
        || key.ends_with("_CFLAGS")
        || key.ends_with("_CPPFLAGS")
        || key.ends_with("_CC")
        || key.ends_with("_CXX")
        || key.ends_with("_AR")
        || key.ends_with("_LD")
        || key.ends_with("_RANLIB")
}

pub(super) struct BoundedReader<R> {
    inner: R,
    remaining: u64,
    limit_error: &'static str,
}

impl<R> BoundedReader<R> {
    pub(super) const fn new(inner: R, limit: u64, limit_error: &'static str) -> Self {
        Self {
            inner,
            remaining: limit,
            limit_error,
        }
    }

    pub(super) fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut extra = [0_u8; 1];
            return match self.inner.read(&mut extra)? {
                0 => Ok(0),
                _ => Err(io::Error::other(self.limit_error)),
            };
        }
        let maximum = usize::try_from(self.remaining.min(buffer.len() as u64))
            .expect("bounded read length always fits usize");
        let read = self.inner.read(&mut buffer[..maximum])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

pub(super) fn cosign_command(cosign: &Path) -> Command {
    let mut command = Command::new(cosign);
    remove_process_injection_environment(&mut command);
    command
}

pub(super) fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

pub(super) fn normalize_crlf(mut bytes: Vec<u8>) -> Vec<u8> {
    let mut read = 0;
    let mut write = 0;
    while read < bytes.len() {
        if bytes[read] == b'\r' && bytes.get(read + 1) == Some(&b'\n') {
            read += 1;
        }
        bytes[write] = bytes[read];
        write += 1;
        read += 1;
    }
    bytes.truncate(write);
    bytes
}

#[derive(Clone, Debug)]
pub(super) struct QualifiedCargo {
    workspace_root: PathBuf,
    target_dir: PathBuf,
}

impl QualifiedCargo {
    pub(super) fn qualify(root: &Path) -> Result<Self> {
        let workspace_root = canonical_directory(root, "Cargo workspace root")?;
        Ok(Self {
            target_dir: workspace_root.join("target"),
            workspace_root,
        })
    }

    pub(super) fn command(&self, root: &Path) -> Result<Command> {
        if canonical_directory(root, "Cargo workspace root")? != self.workspace_root {
            return Err(Error::message("Cargo workspace root changed"));
        }
        Ok(self.command_at(&self.workspace_root, &self.target_dir))
    }

    pub(super) fn wasm_command(&self, root: &Path) -> Result<Command> {
        if canonical_directory(root, "Cargo workspace root")? != self.workspace_root {
            return Err(Error::message("Cargo workspace root changed"));
        }
        let mut command = self.command_at(&self.workspace_root, &self.target_dir);
        command.arg(format!("+{WASM_RUST_TOOLCHAIN}"));
        remove_process_injection_environment(&mut command);
        remove_wasm_cargo_injection_environment(&mut command);
        command
            .env("CARGO_BUILD_JOBS", CARGO_SUBPROCESS_JOBS)
            .env("CARGO_TARGET_DIR", &self.target_dir);
        Ok(command)
    }

    pub(super) fn target_dir(&self) -> &Path {
        &self.target_dir
    }

    fn command_at(&self, working_dir: &Path, target_dir: &Path) -> Command {
        let mut command = Command::new("cargo");
        command
            .current_dir(working_dir)
            .env("CARGO_BUILD_JOBS", CARGO_SUBPROCESS_JOBS)
            .env("CARGO_TARGET_DIR", target_dir);
        command
    }
}

fn remove_wasm_cargo_injection_environment(command: &mut Command) {
    for (key, _) in env::vars_os() {
        if is_wasm_cargo_injection_variable(&key) {
            command.env_remove(key);
        }
    }
    for key in [
        "BOXDD_SYS_PROVIDER",
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
        "RUSTC",
        "RUSTC_BOOTSTRAP",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTUP_TOOLCHAIN",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_INCREMENTAL",
        "CARGO_TARGET_DIR",
    ] {
        command.env_remove(key);
    }
}

fn is_wasm_cargo_injection_variable(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("BOXDD_SYS_")
        || key.starts_with("BOX2D_")
        || key.starts_with("CARGO_BUILD_")
        || key.starts_with("CARGO_TARGET_")
        || key.starts_with("CARGO_PROFILE_")
        || key.starts_with("CARGO_UNSTABLE_")
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|source| Error::io(path, source))?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(Error::message(format!(
            "{label} is not a directory: {}",
            path.display()
        )))
    }
}

fn require_normal_child(path: &Path, root: &Path, label: &str) -> Result<()> {
    let relative = path.strip_prefix(root).map_err(|_| {
        Error::message(format!(
            "{label} must remain below {}: {}",
            root.display(),
            path.display()
        ))
    })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::message(format!(
            "{label} must be a direct normalized descendant of {}: {}",
            root.display(),
            path.display()
        )));
    }
    Ok(())
}

#[derive(Copy, Clone)]
pub(super) enum BuildProfile {
    Debug,
    Release,
    WasmRelease,
}

impl BuildProfile {
    pub(super) const fn for_provider_smoke() -> Self {
        Self::Debug
    }

    pub(super) fn for_pages() -> Result<Self> {
        Self::from_env_or(Self::WasmRelease)
    }

    fn from_env_or(default: Self) -> Result<Self> {
        match env::var(PAGES_WASM_PROFILE_ENV) {
            Ok(value) => Self::parse(&value).ok_or_else(|| {
                Error::message(format!(
                    "invalid {PAGES_WASM_PROFILE_ENV} value `{value}`; expected debug, release, or wasm-release"
                ))
            }),
            Err(_) => Ok(default),
        }
    }

    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().replace('_', "-").as_str() {
            "debug" => Some(Self::Debug),
            "release" => Some(Self::Release),
            "wasm-release" => Some(Self::WasmRelease),
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
    command
        .env(
            crate::wasm_provider_memory::FINAL_LINK_OPT_IN_ENV,
            crate::wasm_provider_memory::FINAL_LINK_OPT_IN_VALUE,
        )
        .arg("--")
        .arg("-C")
        .arg("link-arg=--import-memory")
        .arg("-C")
        .arg(format!(
            "link-arg=--initial-memory={}",
            crate::wasm_provider_memory::INITIAL_MEMORY_BYTES
        ))
        .arg("-C")
        .arg(format!(
            "link-arg=--max-memory={}",
            crate::wasm_provider_memory::MAXIMUM_MEMORY_BYTES
        ))
        .arg("-C")
        .arg("link-arg=--no-stack-first")
        .arg("-C")
        .arg("link-arg=--export=__data_end")
        .arg("-C")
        .arg("link-arg=--export=__stack_low")
        .arg("-C")
        .arg("link-arg=--export=__stack_high")
        .arg("-C")
        .arg("link-arg=--export=__heap_base")
        .arg("-C")
        .arg("link-arg=--export=__heap_end");
    command.arg("-C").arg(format!(
        "link-arg=--global-base={}",
        crate::wasm_provider_memory::CONSUMER_GLOBAL_BASE_BYTES
    ));
    for export in export_groups.iter().flat_map(|exports| exports.iter()) {
        command.arg("-C").arg(format!("link-arg=--export={export}"));
    }
}

pub(super) fn run_command(command: &mut Command, label: &str) -> Result<()> {
    let status = run_status(command, label).map_err(Error::message)?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with status {status}"
        )))
    }
}

pub(super) fn replace_dir_under(dir: &Path, allowed_root: &Path) -> Result<()> {
    fs::create_dir_all(allowed_root).map_err(|source| Error::io(allowed_root, source))?;
    require_real_directory(allowed_root, "allowed replacement root")?;
    let canonical_root = allowed_root
        .canonicalize()
        .map_err(|source| Error::io(allowed_root, source))?;
    require_normal_child(dir, allowed_root, "replacement directory")?;
    let parent = dir.parent().ok_or_else(|| {
        Error::message(format!(
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
                return Err(Error::message(format!(
                    "refusing to remove directory outside {}: {}",
                    canonical_root.display(),
                    canonical_dir.display()
                )));
            }
            fs::remove_dir_all(dir).map_err(|source| Error::io(dir, source))?;
        }
        Ok(_) => {
            return Err(Error::message(format!(
                "replacement target is not a real directory: {}",
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
        Error::message(format!(
            "directory must remain under {}: {}",
            root.display(),
            dir.display()
        ))
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(Error::message(format!(
                "directory path is not canonical: {}",
                dir.display()
            )));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            }
            Ok(_) => {
                return Err(Error::message(format!(
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
            return Err(Error::message(format!(
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
        Err(Error::message(format!(
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
        Err(Error::message(format!(
            "{label} not found: {}",
            path.display()
        )))
    }
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
            BuildProfile::parse("WASM_RELEASE"),
            Some(BuildProfile::WasmRelease)
        ));
        assert!(BuildProfile::parse("fast").is_none());
    }

    #[test]
    fn wasm_cargo_command_pins_toolchain_and_removes_compile_injection() {
        let root = tempfile::tempdir().unwrap();
        let cargo = QualifiedCargo::qualify(root.path()).unwrap();
        let command = cargo.wasm_command(root.path()).unwrap();
        assert_eq!(command.get_args().next(), Some(OsStr::new("+1.97.1")));
        for key in [
            "RUSTFLAGS",
            "RUSTC_WRAPPER",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_INCREMENTAL",
            "RUSTUP_TOOLCHAIN",
            "BOXDD_SYS_PROVIDER",
        ] {
            assert!(
                command
                    .get_envs()
                    .any(|(actual, value)| { actual == OsStr::new(key) && value.is_none() })
            );
        }
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("CARGO_BUILD_JOBS")
                && value == Some(OsStr::new(CARGO_SUBPROCESS_JOBS))
        }));
    }

    #[test]
    fn replacement_never_accepts_the_allowed_root_itself() {
        let root = tempfile::tempdir().unwrap();
        assert!(replace_dir_under(root.path(), root.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn replacement_rejects_symlinked_roots_and_parents_without_touching_targets() {
        use std::os::unix::fs::symlink;

        let sandbox = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("keep.txt");
        fs::write(&outside_file, "keep").unwrap();

        let linked_root = sandbox.path().join("linked-root");
        symlink(outside.path(), &linked_root).unwrap();
        assert!(replace_dir_under(&linked_root.join("generated"), &linked_root).is_err());
        assert_eq!(fs::read_to_string(&outside_file).unwrap(), "keep");

        let allowed = sandbox.path().join("allowed");
        fs::create_dir(&allowed).unwrap();
        let linked_parent = allowed.join("linked-parent");
        symlink(outside.path(), &linked_parent).unwrap();
        assert!(replace_dir_under(&linked_parent.join("generated"), &allowed).is_err());
        assert_eq!(fs::read_to_string(&outside_file).unwrap(), "keep");
    }

    #[test]
    fn bounded_reader_reports_the_callers_limit_error_after_the_exact_limit() {
        let mut bounded = BoundedReader::new(
            std::io::Cursor::new([1_u8, 2, 3]),
            2,
            "domain-specific stream limit",
        );
        let mut bytes = Vec::new();
        let error = bounded.read_to_end(&mut bytes).unwrap_err();
        assert_eq!(bytes, [1, 2]);
        assert_eq!(error.to_string(), "domain-specific stream limit");
    }

    #[test]
    fn crlf_normalization_preserves_lf_and_lone_carriage_returns() {
        assert_eq!(
            normalize_crlf(b"first\r\nsecond\rthird\nfourth\r\n".to_vec()),
            b"first\nsecond\rthird\nfourth\n"
        );
    }
}
