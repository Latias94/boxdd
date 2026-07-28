use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use crate::{
    Error, Result,
    provider_manifest::sha256_file,
    qualified_git::{
        is_git_environment_key, is_process_injection_environment_key, isolate_git_configuration,
        remove_process_injection_environment,
    },
    toolchains::DEVELOPMENT,
};

const PAGES_WASM_PROFILE_ENV: &str = "BOXDD_PAGES_WASM_PROFILE";
pub(super) const WASM_TARGET: &str = crate::wasm_provider_contract::CONSUMER_TARGET;
const CARGO_SUBPROCESS_JOBS: &str = "2";
const CARGO_BUILD_ENVIRONMENT_KEYS: &[&str] = &[
    "AR",
    "BINDGEN_EXTRA_CLANG_ARGS",
    "CARGO_CFG_DOCSRS",
    "CARGO_ENCODED_RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_HOME",
    "CARGO_INCREMENTAL",
    "CARGO_NET_GIT_FETCH_WITH_CLI",
    "CC",
    "CFLAGS",
    "CL",
    "CPATH",
    "CPPFLAGS",
    "CPLUS_INCLUDE_PATH",
    "C_INCLUDE_PATH",
    "CXX",
    "DOCS_RS",
    "DYLD_INSERT_LIBRARIES",
    "EMSDK",
    "EMSDK_NODE",
    "EMSDK_PYTHON",
    "EM_CONFIG",
    "LD",
    "LDFLAGS",
    "LD_PRELOAD",
    "LIBRARY_PATH",
    "MAKEFLAGS",
    "NUM_JOBS",
    "OBJC_INCLUDE_PATH",
    "RANLIB",
    "RUSTC",
    "RUSTC_BOOTSTRAP",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTC_WRAPPER",
    "RUSTDOCFLAGS",
    "RUSTFLAGS",
    "RUST_TARGET_PATH",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "SDKROOT",
    "BOXDD_EMSDK_REVISION",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct QualifiedProgram {
    invocation_path: PathBuf,
    resolved_path: PathBuf,
    sha256: String,
}

impl QualifiedProgram {
    fn qualify(invocation_path: PathBuf, label: &str) -> Result<Self> {
        if !invocation_path.is_absolute() {
            return Err(Error::Message(format!(
                "qualified {label} path must be absolute: {}",
                invocation_path.display()
            )));
        }
        let resolved_path = fs::canonicalize(&invocation_path)
            .map_err(|source| Error::io(&invocation_path, source))?;
        let metadata = fs::symlink_metadata(&resolved_path)
            .map_err(|source| Error::io(&resolved_path, source))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(Error::Message(format!(
                "qualified {label} must resolve to a regular file: {}",
                resolved_path.display()
            )));
        }
        let sha256 = sha256_file(&resolved_path).map_err(Error::Message)?;
        Ok(Self {
            invocation_path,
            resolved_path,
            sha256,
        })
    }

    fn revalidate(&self, label: &str) -> Result<()> {
        let current = fs::canonicalize(&self.invocation_path)
            .map_err(|source| Error::io(&self.invocation_path, source))?;
        if current != self.resolved_path {
            return Err(Error::Message(format!(
                "qualified {label} path changed after qualification: expected {}, found {}",
                self.resolved_path.display(),
                current.display()
            )));
        }
        let digest = sha256_file(&current).map_err(Error::Message)?;
        if digest != self.sha256 {
            return Err(Error::Message(format!(
                "qualified {label} changed after qualification: {}",
                current.display()
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct QualifiedCargo {
    workspace_root: PathBuf,
    target_dir: PathBuf,
    cargo_home: PathBuf,
    isolated_scope: Option<IsolatedCargoScope>,
    cargo: QualifiedProgram,
    rustc: QualifiedProgram,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IsolatedCargoScope {
    working_root: PathBuf,
    output_root: PathBuf,
}

impl QualifiedCargo {
    pub(super) fn qualify(root: &Path) -> Result<Self> {
        require_real_directory(root, "Cargo workspace root")?;
        let workspace_root = root
            .canonicalize()
            .map_err(|source| Error::io(root, source))?;
        let cargo_home = cargo_home()?;
        validate_cargo_configuration_isolation(&workspace_root, Some(&cargo_home))?;
        Self::qualify_with(
            workspace_root.clone(),
            workspace_root.join("target"),
            cargo_home,
            None,
        )
    }

    pub(super) fn qualify_isolated(
        root: &Path,
        cargo_home: &Path,
        external_root: &Path,
    ) -> Result<Self> {
        Self::qualify_isolated_scoped(root, external_root, cargo_home, external_root)
    }

    pub(super) fn qualify_isolated_scoped(
        root: &Path,
        working_root: &Path,
        cargo_home: &Path,
        output_root: &Path,
    ) -> Result<Self> {
        require_real_directory(root, "Cargo workspace root")?;
        let workspace_root = root
            .canonicalize()
            .map_err(|source| Error::io(root, source))?;
        let working_root = canonical_real_directory(working_root, "isolated Cargo working root")?;
        let output_root = canonical_real_directory(output_root, "isolated Cargo output root")?;
        let cargo_home = canonical_real_directory(cargo_home, "isolated Cargo home")?;
        if cargo_home == output_root || !cargo_home.starts_with(&output_root) {
            return Err(Error::Message(format!(
                "isolated Cargo home must remain below {}: {}",
                output_root.display(),
                cargo_home.display()
            )));
        }
        validate_cargo_configuration_isolation(&working_root, Some(&cargo_home))?;
        Self::qualify_with(
            workspace_root,
            output_root.clone(),
            cargo_home,
            Some(IsolatedCargoScope {
                working_root,
                output_root,
            }),
        )
    }

    fn qualify_with(
        workspace_root: PathBuf,
        target_dir: PathBuf,
        cargo_home: PathBuf,
        isolated_scope: Option<IsolatedCargoScope>,
    ) -> Result<Self> {
        let cargo = QualifiedProgram::qualify(
            PathBuf::from(env!("BOXDD_XTASK_CARGO")),
            "compile-time anchored cargo",
        )?;
        let rustc = QualifiedProgram::qualify(
            PathBuf::from(env!("BOXDD_XTASK_RUSTC")),
            "compile-time anchored rustc",
        )?;
        let qualified = Self {
            target_dir,
            workspace_root,
            cargo_home,
            isolated_scope,
            cargo,
            rustc,
        };
        qualified.revalidate()?;
        qualified.verify_version(&qualified.cargo, "cargo")?;
        qualified.verify_version(&qualified.rustc, "rustc")?;
        Ok(qualified)
    }

    pub(super) fn command(&self, root: &Path) -> Result<Command> {
        if self.isolated_scope.is_some() {
            return Err(Error::Message(
                "isolated Cargo qualification requires command_in".to_owned(),
            ));
        }
        self.revalidate_root(root)?;
        self.revalidate()?;
        let mut command = Command::new(&self.cargo.invocation_path);
        configure_cargo_build_environment(&mut command, self);
        command.current_dir(&self.workspace_root);
        Ok(command)
    }

    pub(super) fn command_in(&self, working_dir: &Path, target_dir: &Path) -> Result<Command> {
        let scope = self.isolated_scope.as_ref().ok_or_else(|| {
            Error::Message("workspace Cargo qualification cannot leave its root".to_owned())
        })?;
        self.revalidate()?;
        let working_dir = canonical_directory_at_or_within(
            working_dir,
            &scope.working_root,
            "isolated Cargo working directory",
        )?;
        validate_cargo_configuration_isolation(&working_dir, Some(&self.cargo_home))?;
        let target_dir = validate_external_target_dir(target_dir, &scope.output_root)?;
        let mut command = Command::new(&self.cargo.invocation_path);
        configure_cargo_subprocess_environment(&mut command, &self.cargo_home);
        command
            .current_dir(working_dir)
            .env("CARGO_TARGET_DIR", target_dir)
            .env("RUSTC", &self.rustc.invocation_path);
        Ok(command)
    }

    pub(super) fn command_at_working_root(&self, target_dir: &Path) -> Result<Command> {
        let working_root = self
            .isolated_scope
            .as_ref()
            .ok_or_else(|| {
                Error::Message("workspace Cargo qualification requires command".to_owned())
            })?
            .working_root
            .clone();
        self.command_in(&working_root, target_dir)
    }

    pub(super) fn target_dir(&self) -> &Path {
        &self.target_dir
    }

    fn revalidate_root(&self, root: &Path) -> Result<()> {
        require_real_directory(root, "Cargo workspace root")?;
        let current = root
            .canonicalize()
            .map_err(|source| Error::io(root, source))?;
        if current == self.workspace_root {
            Ok(())
        } else {
            Err(Error::Message(format!(
                "Cargo workspace changed after qualification: expected {}, found {}",
                self.workspace_root.display(),
                current.display()
            )))
        }
    }

    fn revalidate(&self) -> Result<()> {
        self.revalidate_root(&self.workspace_root)?;
        self.cargo.revalidate("cargo")?;
        self.rustc.revalidate("rustc")?;
        let cargo_home = canonical_real_directory(&self.cargo_home, "Cargo home")?;
        if cargo_home != self.cargo_home {
            return Err(Error::Message(format!(
                "Cargo home changed after qualification: expected {}, found {}",
                self.cargo_home.display(),
                cargo_home.display()
            )));
        }
        match &self.isolated_scope {
            Some(scope) => {
                let working_root =
                    canonical_real_directory(&scope.working_root, "isolated Cargo working root")?;
                if working_root != scope.working_root {
                    return Err(Error::Message(format!(
                        "isolated Cargo working root changed after qualification: expected {}, found {}",
                        scope.working_root.display(),
                        working_root.display()
                    )));
                }
                let output_root =
                    canonical_real_directory(&scope.output_root, "isolated Cargo output root")?;
                if output_root != scope.output_root {
                    return Err(Error::Message(format!(
                        "isolated Cargo output root changed after qualification: expected {}, found {}",
                        scope.output_root.display(),
                        output_root.display()
                    )));
                }
                if self.cargo_home == scope.output_root
                    || !self.cargo_home.starts_with(&scope.output_root)
                {
                    return Err(Error::Message(format!(
                        "isolated Cargo home escaped {}: {}",
                        scope.output_root.display(),
                        self.cargo_home.display()
                    )));
                }
                validate_cargo_configuration_isolation(&scope.working_root, Some(&self.cargo_home))
            }
            None => {
                validate_cargo_configuration_isolation(
                    &self.workspace_root,
                    Some(&self.cargo_home),
                )?;
                validate_target_dir(&self.target_dir, &self.workspace_root)
            }
        }
    }

    fn verify_version(&self, program: &QualifiedProgram, name: &str) -> Result<()> {
        program.revalidate(name)?;
        let mut command = Command::new(&program.invocation_path);
        configure_cargo_subprocess_environment(&mut command, &self.cargo_home);
        command
            .current_dir(
                self.isolated_scope
                    .as_ref()
                    .map(|scope| scope.working_root.as_path())
                    .unwrap_or(&self.workspace_root),
            )
            .env("RUSTUP_TOOLCHAIN", DEVELOPMENT)
            .arg("--version");
        let output = command
            .output()
            .map_err(|source| Error::io(format!("qualified {name} --version"), source))?;
        if !output.status.success() {
            return Err(Error::Message(format!(
                "qualified {name} --version failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let version = String::from_utf8(output.stdout).map_err(|error| {
            Error::Message(format!("qualified {name} printed non-UTF-8: {error}"))
        })?;
        let expected_prefix = format!("{name} {DEVELOPMENT} (");
        if version.lines().count() != 1 || !version.trim().starts_with(&expected_prefix) {
            return Err(Error::Message(format!(
                "provider tooling requires {name} {DEVELOPMENT}; found {}",
                version.lines().next().unwrap_or("unknown version")
            )));
        }
        Ok(())
    }
}

fn configure_cargo_build_environment(command: &mut Command, cargo: &QualifiedCargo) {
    configure_cargo_subprocess_environment(command, &cargo.cargo_home);
    command
        .env("CARGO_TARGET_DIR", &cargo.target_dir)
        .env("RUSTC", &cargo.rustc.invocation_path);
}

pub(super) fn configure_cargo_subprocess_environment(command: &mut Command, cargo_home: &Path) {
    let keys = env::vars_os()
        .map(|(key, _)| key)
        .chain(command.get_envs().map(|(key, _)| key.to_os_string()))
        .collect::<Vec<_>>();
    remove_process_injection_environment(command);
    configure_cargo_subprocess_environment_for_keys(command, cargo_home, keys);
}

fn configure_cargo_subprocess_environment_for_keys<I, K>(
    command: &mut Command,
    cargo_home: &Path,
    keys: I,
) where
    I: IntoIterator<Item = K>,
    K: AsRef<OsStr>,
{
    for key in CARGO_BUILD_ENVIRONMENT_KEYS {
        command.env_remove(key);
    }
    for key in keys {
        if is_cargo_build_environment_key(key.as_ref()) {
            command.env_remove(key.as_ref());
        }
    }
    command
        .env("CARGO_BUILD_JOBS", CARGO_SUBPROCESS_JOBS)
        .env("CARGO_HOME", cargo_home)
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTUP_TOOLCHAIN", DEVELOPMENT)
        .env("RUSTC_WRAPPER", "")
        .env("RUSTC_WORKSPACE_WRAPPER", "");
    isolate_git_configuration(command);
}

fn is_cargo_build_environment_key(key: &OsStr) -> bool {
    let name = key.to_string_lossy().to_ascii_uppercase();
    is_git_environment_key(key)
        || is_process_injection_environment_key(key)
        || name == "BOX2D_LIB_DIR"
        || name.starts_with("BOXDD_SYS_")
        || CARGO_BUILD_ENVIRONMENT_KEYS.contains(&name.as_str())
        || name.starts_with("CARGO_BUILD_")
        || name.starts_with("CARGO_REGISTRIES_")
        || name.starts_with("CARGO_REGISTRY_")
        || name.starts_with("CARGO_SOURCE_")
        || name.starts_with("CARGO_TARGET_")
        || name.starts_with("CARGO_PROFILE_")
        || name.starts_with("CFLAGS_")
        || name.ends_with("_CFLAGS")
        || name.starts_with("CPPFLAGS_")
        || name.ends_with("_CPPFLAGS")
        || name.starts_with("LDFLAGS_")
        || name.ends_with("_LDFLAGS")
        || name.starts_with("CC_")
        || name.ends_with("_CC")
        || name.starts_with("CXX_")
        || name.ends_with("_CXX")
        || name.starts_with("AR_")
        || name.ends_with("_AR")
        || name.starts_with("LD_")
        || name.ends_with("_LD")
        || name.starts_with("RANLIB_")
        || name.ends_with("_RANLIB")
        || name.starts_with("BINDGEN_EXTRA_CLANG_ARGS_")
}

fn cargo_home() -> Result<PathBuf> {
    let requested = env::var_os("CARGO_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
                .map(|home| home.join(".cargo"))
        })
        .ok_or_else(|| Error::Message("cannot resolve Cargo home".to_owned()))?;
    if !requested.is_absolute() {
        return Err(Error::Message(format!(
            "Cargo home must be absolute: {}",
            requested.display()
        )));
    }
    require_real_directory(&requested, "Cargo home")?;
    requested
        .canonicalize()
        .map_err(|source| Error::io(&requested, source))
}

fn validate_cargo_configuration_isolation(
    workspace: &Path,
    cargo_home: Option<&Path>,
) -> Result<()> {
    for ancestor in workspace.ancestors() {
        for relative in [".cargo/config.toml", ".cargo/config"] {
            reject_cargo_config(&ancestor.join(relative), "workspace ancestor")?;
        }
    }
    if let Some(cargo_home) = cargo_home {
        for name in ["config.toml", "config"] {
            reject_cargo_config(&cargo_home.join(name), "Cargo home")?;
        }
    }
    Ok(())
}

fn reject_cargo_config(path: &Path, location: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io(path, source)),
        Ok(_) => Err(Error::Message(format!(
            "{location} Cargo config would alter the qualified provider build: {}",
            path.display()
        ))),
    }
}

fn validate_target_dir(target_dir: &Path, workspace_root: &Path) -> Result<()> {
    match fs::symlink_metadata(target_dir) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io(target_dir, source)),
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            let canonical = target_dir
                .canonicalize()
                .map_err(|source| Error::io(target_dir, source))?;
            if canonical == workspace_root.join("target") {
                Ok(())
            } else {
                Err(Error::Message(format!(
                    "qualified Cargo target directory escaped the workspace: {}",
                    canonical.display()
                )))
            }
        }
        Ok(_) => Err(Error::Message(format!(
            "qualified Cargo target directory must be a real directory: {}",
            target_dir.display()
        ))),
    }
}

fn canonical_real_directory(path: &Path, label: &str) -> Result<PathBuf> {
    require_real_directory(path, label)?;
    path.canonicalize()
        .map_err(|source| Error::io(path, source))
}

fn canonical_directory_within(path: &Path, root: &Path, label: &str) -> Result<PathBuf> {
    validate_controlled_child_path(path, root, label)?;
    require_real_directory(path, label)?;
    ensure_real_directory_tree(root, path, root)?;
    let canonical = path
        .canonicalize()
        .map_err(|source| Error::io(path, source))?;
    if canonical.starts_with(root) && canonical != root {
        Ok(canonical)
    } else {
        Err(Error::Message(format!(
            "{label} escaped {}: {}",
            root.display(),
            canonical.display()
        )))
    }
}

fn canonical_directory_at_or_within(path: &Path, root: &Path, label: &str) -> Result<PathBuf> {
    if path == root {
        return canonical_real_directory(path, label);
    }
    canonical_directory_within(path, root, label)
}

pub(super) fn controlled_child_directory(
    root: &Path,
    relative: &Path,
    label: &str,
) -> Result<PathBuf> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Message(format!(
            "{label} must be a canonical relative path: {}",
            relative.display()
        )));
    }
    let root = canonical_real_directory(root, "controlled directory root")?;
    let path = root.join(relative);
    ensure_real_directory_tree(&root, &path, &root)?;
    canonical_directory_within(&path, &root, label)
}

fn validate_external_target_dir(target_dir: &Path, root: &Path) -> Result<PathBuf> {
    validate_controlled_child_path(target_dir, root, "external Cargo target directory")?;
    ensure_real_directory_tree(root, target_dir, root)?;
    canonical_directory_within(target_dir, root, "external Cargo target directory")
}

fn validate_controlled_child_path(path: &Path, root: &Path, label: &str) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::Message(format!(
            "{label} must be absolute: {}",
            path.display()
        )));
    }
    let relative = path.strip_prefix(root).map_err(|_| {
        Error::Message(format!(
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
        return Err(Error::Message(format!(
            "{label} must be a canonical child of {}: {}",
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
    fn cargo_build_environment_classifies_compiler_and_target_injection() {
        for name in [
            "BOXDD_SYS_PROVIDER",
            "BOXDD_SYS_FORCE_BINDGEN",
            "DOCS_RS",
            "CARGO_CFG_DOCSRS",
            "CARGO_HOME",
            "RUSTFLAGS",
            "RUSTDOCFLAGS",
            "RUSTC",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTC_BOOTSTRAP",
            "RUST_TARGET_PATH",
            "RUSTUP_HOME",
            "RUSTUP_TOOLCHAIN",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_RUSTFLAGS",
            "CARGO_NET_GIT_FETCH_WITH_CLI",
            "CARGO_PROFILE_RELEASE_LTO",
            "CARGO_REGISTRIES_CRATES_IO_INDEX",
            "CARGO_REGISTRIES_BOXDD_LOCAL_INDEX",
            "CARGO_REGISTRY_DEFAULT",
            "CARGO_SOURCE_CRATES_IO_REPLACE_WITH",
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER",
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_COUNT",
            "GIT_DIR",
            "GIT_SSH_COMMAND",
            "CC_wasm32_unknown_unknown",
            "wasm32_unknown_unknown_CFLAGS",
            "BINDGEN_EXTRA_CLANG_ARGS_wasm32_unknown_unknown",
            "LD_PRELOAD",
        ] {
            assert!(is_cargo_build_environment_key(OsStr::new(name)), "{name}");
        }
        for name in [
            "HOME",
            "PATH",
            "BOXDD_WASM_PRECISION",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
            "CARGO_HTTP_PROXY",
        ] {
            assert!(!is_cargo_build_environment_key(OsStr::new(name)), "{name}");
        }
    }

    #[test]
    fn cargo_subprocess_environment_removes_command_local_injection() {
        let removed_keys = [
            "DOCS_RS",
            "CARGO_CFG_DOCSRS",
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_NET_GIT_FETCH_WITH_CLI",
            "CARGO_REGISTRIES_CRATES_IO_INDEX",
            "CARGO_REGISTRIES_BOXDD_LOCAL_INDEX",
            "CARGO_REGISTRY_DEFAULT",
            "CARGO_SOURCE_CRATES_IO_REPLACE_WITH",
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
            "GIT_CONFIG_COUNT",
            "GIT_DIR",
            "GIT_SSH_COMMAND",
            "RUST_TARGET_PATH",
        ];
        let mut command = Command::new("fixture-cargo");
        for key in removed_keys {
            command.env(key, "injected");
        }
        command.env("GIT_CONFIG_GLOBAL", "injected");
        command.env("CARGO_HOME", "injected");
        command.env("CARGO_HTTP_PROXY", "http://127.0.0.1:10809");

        let cargo_home = Path::new("isolated-cargo-home");
        configure_cargo_subprocess_environment_for_keys(
            &mut command,
            cargo_home,
            removed_keys.into_iter().chain(["CARGO_HTTP_PROXY"]),
        );

        let environment = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        for key in removed_keys {
            assert_eq!(
                environment.get(OsStr::new(key)).copied(),
                Some(None),
                "{key} must be explicitly removed from Cargo build subprocesses"
            );
        }
        assert_eq!(
            environment
                .get(OsStr::new("GIT_CONFIG_GLOBAL"))
                .copied()
                .flatten(),
            Some(OsStr::new(if cfg!(windows) { "NUL" } else { "/dev/null" }))
        );
        assert_eq!(
            environment
                .get(OsStr::new("GIT_CONFIG_NOSYSTEM"))
                .copied()
                .flatten(),
            Some(OsStr::new("1"))
        );
        for key in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            assert_eq!(
                environment.get(OsStr::new(key)).copied().flatten(),
                Some(OsStr::new("")),
                "{key} must be explicitly disabled"
            );
        }
        assert_eq!(
            environment.get(OsStr::new("CARGO_HOME")).copied().flatten(),
            Some(cargo_home.as_os_str())
        );
        assert_eq!(
            environment
                .get(OsStr::new("CARGO_HTTP_PROXY"))
                .copied()
                .flatten(),
            Some(OsStr::new("http://127.0.0.1:10809"))
        );
        for (key, value) in [
            ("CARGO_BUILD_JOBS", CARGO_SUBPROCESS_JOBS),
            ("CARGO_INCREMENTAL", "0"),
        ] {
            assert_eq!(
                environment.get(OsStr::new(key)).copied().flatten(),
                Some(OsStr::new(value)),
                "{key} must use the isolated Cargo policy"
            );
        }
    }

    #[test]
    fn cargo_subprocess_environment_removes_command_local_git_injection() {
        let mut command = Command::new("fixture-cargo");
        for key in [
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_0",
            "GIT_CONFIG_VALUE_0",
            "GIT_DIR",
            "GIT_SSH_COMMAND",
            "RUST_TARGET_PATH",
        ] {
            command.env(key, "injected");
        }
        command.env("GIT_CONFIG_GLOBAL", "injected");
        command.env("HTTPS_PROXY", "http://127.0.0.1:10809");

        configure_cargo_subprocess_environment(&mut command, Path::new("isolated-cargo-home"));

        let environment = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        for key in [
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_0",
            "GIT_CONFIG_VALUE_0",
            "GIT_DIR",
            "GIT_SSH_COMMAND",
            "RUST_TARGET_PATH",
        ] {
            assert_eq!(
                environment.get(OsStr::new(key)).copied(),
                Some(None),
                "{key} must be explicitly removed from Cargo subprocesses"
            );
        }
        assert_eq!(
            environment
                .get(OsStr::new("GIT_CONFIG_GLOBAL"))
                .copied()
                .flatten(),
            Some(OsStr::new(if cfg!(windows) { "NUL" } else { "/dev/null" }))
        );
        assert_eq!(
            environment
                .get(OsStr::new("GIT_CONFIG_NOSYSTEM"))
                .copied()
                .flatten(),
            Some(OsStr::new("1"))
        );
        assert_eq!(
            environment
                .get(OsStr::new("HTTPS_PROXY"))
                .copied()
                .flatten(),
            Some(OsStr::new("http://127.0.0.1:10809"))
        );
    }

    #[test]
    fn cargo_configuration_rejects_workspace_ancestor_and_cargo_home_files() {
        let fixture = tempfile::tempdir().unwrap();
        let workspace = fixture.path().join("parent/workspace");
        let cargo_home = fixture.path().join("cargo-home");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&cargo_home).unwrap();

        assert!(validate_cargo_configuration_isolation(&workspace, Some(&cargo_home)).is_ok());

        let ancestor_config = fixture.path().join("parent/.cargo");
        fs::create_dir_all(&ancestor_config).unwrap();
        fs::write(
            ancestor_config.join("config.toml"),
            "[build]\nrustflags = []\n",
        )
        .unwrap();
        assert!(validate_cargo_configuration_isolation(&workspace, Some(&cargo_home)).is_err());
        fs::remove_file(ancestor_config.join("config.toml")).unwrap();

        fs::write(cargo_home.join("config"), "[build]\nrustflags = []\n").unwrap();
        assert!(validate_cargo_configuration_isolation(&workspace, Some(&cargo_home)).is_err());
    }

    #[test]
    fn qualified_cargo_uses_absolute_pinned_tools_and_revalidates_them() {
        let cargo =
            QualifiedCargo::qualify(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
                .expect("workspace Cargo toolchain must qualify");
        let command = cargo
            .command(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
            .expect("qualified Cargo command");
        assert!(Path::new(command.get_program()).is_absolute());
        assert_eq!(
            command
                .get_envs()
                .find(|(name, _)| *name == OsStr::new("RUSTUP_TOOLCHAIN"))
                .and_then(|(_, value)| value),
            Some(OsStr::new("1.97.1"))
        );
        assert!(
            command
                .get_envs()
                .any(|(name, value)| name == OsStr::new("RUSTC") && value.is_some())
        );
    }

    #[test]
    fn qualified_isolated_cargo_controls_external_paths_and_rejects_late_config() {
        let fixture = tempfile::tempdir().unwrap();
        let repository = fixture.path().join("repository");
        let external = fixture.path().join("external");
        let cargo_home = external.join("cargo-home");
        let working_parent = external.join("work");
        let working = working_parent.join("consumer");
        fs::create_dir_all(repository.join(".cargo")).unwrap();
        fs::write(
            repository.join(".cargo/config.toml"),
            "[build]\nrustflags = ['workspace-only']\n",
        )
        .unwrap();
        fs::create_dir_all(&cargo_home).unwrap();
        fs::create_dir_all(&working).unwrap();
        let external = external.canonicalize().unwrap();
        let cargo_home = cargo_home.canonicalize().unwrap();
        let working = working.canonicalize().unwrap();
        let target = external.join("targets/consumer");

        let cargo = QualifiedCargo::qualify_isolated(&repository, &cargo_home, &external)
            .expect("isolated Cargo must not inspect repository Cargo configuration");
        let command = cargo
            .command_in(&working, &target)
            .expect("controlled external Cargo command");
        assert!(Path::new(command.get_program()).is_absolute());
        assert_eq!(command.get_current_dir(), Some(working.as_path()));
        let environment = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            environment.get(OsStr::new("CARGO_HOME")).copied().flatten(),
            Some(cargo_home.as_os_str())
        );
        assert_eq!(
            environment
                .get(OsStr::new("CARGO_TARGET_DIR"))
                .copied()
                .flatten(),
            Some(target.as_os_str())
        );
        let rustc = environment
            .get(OsStr::new("RUSTC"))
            .copied()
            .flatten()
            .expect("qualified RUSTC");
        assert!(Path::new(rustc).is_absolute());
        assert!(
            cargo
                .command_in(&working, &external.join("../outside-target"))
                .is_err()
        );

        fs::create_dir_all(working_parent.join(".cargo")).unwrap();
        fs::write(
            working_parent.join(".cargo/config"),
            "[build]\nrustflags = ['late-injection']\n",
        )
        .unwrap();
        assert!(cargo.command_in(&working, &target).is_err());
    }

    #[test]
    fn qualified_program_rejects_post_qualification_replacement() {
        let fixture = tempfile::tempdir().unwrap();
        let program = fixture.path().join("tool");
        fs::write(&program, b"first").unwrap();
        let qualified = QualifiedProgram::qualify(program.clone(), "fixture").unwrap();
        fs::write(&program, b"second").unwrap();
        assert!(qualified.revalidate("fixture").is_err());
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
