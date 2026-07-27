use std::{
    collections::BTreeSet,
    env,
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;

use crate::{
    Error, Result,
    prebuilt_provenance::{MAX_PACKAGE_BYTES, PrebuiltProvenanceStatement},
    provenance_policy::{
        self, COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW,
        SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH, SIGSTORE_TRUSTED_ROOT_SHA256,
    },
    provider_archive::{ArchiveExpectation, verify_provider_archive},
    provider_manifest,
    qualified_git::qualified_git_command,
    source_overlay::effective_source_identity,
};

const QUALIFIED_TOOLCHAINS: &[&str] = &["1.95.0", "1.97.1"];
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4_096;
const TAR_BLOCK_BYTES: u64 = 512;
const MAX_ARCHIVE_STREAM_BYTES: u64 = MAX_ARCHIVE_TOTAL_BYTES
    + (MAX_ARCHIVE_ENTRIES as u64 * TAR_BLOCK_BYTES * 2)
    + (TAR_BLOCK_BYTES * 2);
const MAX_PROVENANCE_STATEMENT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SIGSTORE_BUNDLE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TRUSTED_ROOT_BYTES: u64 = 4 * 1024 * 1024;
const CONSUMER_NAME: &str = "boxdd-native-provider-consumer";

struct BoundedReader<R> {
    inner: R,
    remaining: u64,
}

impl<R> BoundedReader<R> {
    const fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
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
                _ => Err(io::Error::other(
                    "decompressed archive exceeds the qualification stream limit",
                )),
            };
        }
        let maximum = usize::try_from(self.remaining.min(buffer.len() as u64))
            .expect("bounded read length always fits usize");
        let read = self.inner.read(&mut buffer[..maximum])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provider {
    System,
    Prebuilt,
}

impl Provider {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "system" => Ok(Self::System),
            "prebuilt" => Ok(Self::Prebuilt),
            _ => Err(Error::message(format!(
                "unsupported native provider {value:?}; expected system or prebuilt"
            ))),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Prebuilt => "prebuilt",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Precision {
    Single,
    Double,
}

impl Precision {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "single" => Ok(Self::Single),
            "double" => Ok(Self::Double),
            _ => Err(Error::message(format!(
                "unsupported native precision {value:?}; expected single or double"
            ))),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Options {
    provider: Provider,
    toolchain: String,
    precision: Precision,
    target: String,
    crt: String,
    artifacts: PathBuf,
    cosign: Option<PathBuf>,
    allow_dirty: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut provider = None;
        let mut toolchain = None;
        let mut precision = None;
        let mut target = None;
        let mut crt = None;
        let mut artifacts = None;
        let mut cosign = None;
        let mut allow_dirty = false;
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            if flag == "--allow-dirty" {
                if allow_dirty {
                    return Err(Error::message("--allow-dirty may only be supplied once"));
                }
                allow_dirty = true;
                index += 1;
                continue;
            }
            let value = args.get(index + 1).ok_or_else(|| {
                Error::message(format!(
                    "qualify-native-provider requires a value for {flag}"
                ))
            })?;
            match flag {
                "--provider" => set_once(&mut provider, Provider::parse(value)?, flag)?,
                "--toolchain" => set_once(&mut toolchain, value.clone(), flag)?,
                "--precision" => set_once(&mut precision, Precision::parse(value)?, flag)?,
                "--target" => set_once(&mut target, value.clone(), flag)?,
                "--crt" => set_once(&mut crt, value.clone(), flag)?,
                "--artifacts" => {
                    set_once(&mut artifacts, PathBuf::from(value), flag)?;
                }
                "--cosign" => set_once(&mut cosign, PathBuf::from(value), flag)?,
                _ => {
                    return Err(Error::message(format!(
                        "unknown qualify-native-provider argument {flag:?}"
                    )));
                }
            }
            index += 2;
        }

        let options = Self {
            provider: required(provider, "--provider")?,
            toolchain: required(toolchain, "--toolchain")?,
            precision: required(precision, "--precision")?,
            target: required(target, "--target")?,
            crt: required(crt, "--crt")?,
            artifacts: required(artifacts, "--artifacts")?,
            cosign,
            allow_dirty,
        };
        options.validate()?;
        Ok(options)
    }

    fn validate(&self) -> Result<()> {
        if !QUALIFIED_TOOLCHAINS.contains(&self.toolchain.as_str()) {
            return Err(Error::message(format!(
                "toolchain {:?} is not in the native qualification allowlist {QUALIFIED_TOOLCHAINS:?}",
                self.toolchain
            )));
        }
        let valid_coordinate = matches!(
            (self.target.as_str(), self.crt.as_str()),
            ("x86_64-unknown-linux-gnu", "none")
                | ("x86_64-apple-darwin", "none")
                | ("aarch64-apple-darwin", "none")
                | ("x86_64-pc-windows-msvc", "md" | "mt")
        );
        if !valid_coordinate {
            return Err(Error::message(format!(
                "unsupported native target/CRT coordinate {}/{}",
                self.target, self.crt
            )));
        }
        match (self.provider, self.cosign.is_some()) {
            (Provider::System, false) | (Provider::Prebuilt, true) => Ok(()),
            (Provider::System, true) => Err(Error::message(
                "system qualification must not accept a Cosign executable",
            )),
            (Provider::Prebuilt, false) => {
                Err(Error::message("prebuilt qualification requires --cosign"))
            }
        }
    }
}

fn required<T>(value: Option<T>, flag: &str) -> Result<T> {
    value.ok_or_else(|| Error::message(format!("qualify-native-provider requires {flag}")))
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        Err(Error::message(format!("{flag} may only be supplied once")))
    } else {
        Ok(())
    }
}

fn cargo_arguments<S: AsRef<OsStr>>(toolchain: &str, arguments: &[S]) -> Result<Vec<OsString>> {
    if !QUALIFIED_TOOLCHAINS.contains(&toolchain) {
        return Err(Error::message(format!(
            "toolchain {toolchain:?} is not qualified for native provider consumption"
        )));
    }
    let mut result = Vec::with_capacity(arguments.len() + 1);
    result.push(OsString::from(format!("+{toolchain}")));
    result.extend(
        arguments
            .iter()
            .map(|argument| argument.as_ref().to_os_string()),
    );
    Ok(result)
}

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    qualify(root, &options)
}

#[derive(Default)]
struct CommandEnvironment {
    remove: BTreeSet<OsString>,
    values: Vec<(OsString, OsString)>,
}

impl CommandEnvironment {
    fn fail_closed(cargo_home: &Path) -> Self {
        let mut environment = Self::default();
        for (key, _) in env::vars_os() {
            if is_qualification_sensitive_env(&key) {
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
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "CARGO_ENCODED_RUSTFLAGS",
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
        ] {
            environment.remove.insert(OsString::from(key));
        }
        environment.set("CARGO_HOME", cargo_home.as_os_str());
        environment
    }

    fn set(&mut self, key: impl Into<OsString>, value: impl Into<OsString>) {
        self.values.push((key.into(), value.into()));
    }

    fn apply(&self, command: &mut Command) {
        for key in &self.remove {
            command.env_remove(key);
        }
        for (key, value) in &self.values {
            command.env(key, value);
        }
    }
}

fn is_qualification_sensitive_env(key: &OsStr) -> bool {
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

fn qualify(root: &Path, options: &Options) -> Result<()> {
    let checkout = canonicalize(root, "workspace root")?;
    let scratch = tempfile::Builder::new()
        .prefix("boxdd-native-provider-")
        .tempdir()
        .map_err(|source| Error::io("native provider qualification tempdir", source))?;
    let scratch_root = canonicalize(scratch.path(), "qualification tempdir")?;
    if scratch_root.starts_with(&checkout) {
        return Err(Error::message(format!(
            "qualification tempdir must be outside the checkout: {}",
            scratch_root.display()
        )));
    }
    let cargo_home = scratch_root.join("cargo-home");
    fs::create_dir(&cargo_home).map_err(|source| Error::io(&cargo_home, source))?;
    let cargo_home = canonicalize(&cargo_home, "isolated Cargo home")?;
    validate_cargo_configuration_isolation(&scratch_root, &cargo_home)?;

    let version = workspace_version(&checkout)?;
    let checkout_commit = if options.provider == Provider::Prebuilt {
        Some(qualified_checkout_commit(&checkout)?)
    } else {
        None
    };
    let prepared_prebuilt = if options.provider == Provider::Prebuilt {
        Some(prepare_prebuilt_provider(
            options,
            &version,
            checkout_commit
                .as_deref()
                .expect("prebuilt qualification must resolve one checkout commit"),
            &checkout.join("boxdd-sys"),
            &scratch_root,
            &checkout,
        )?)
    } else {
        None
    };
    let crate_archive =
        package_boxdd_sys(&checkout, &scratch_root, &cargo_home, options, &version)?;
    let crate_extract = scratch_root.join("crate-extract");
    fs::create_dir(&crate_extract).map_err(|source| Error::io(&crate_extract, source))?;
    let expected_root = format!("boxdd-sys-{version}");
    extract_archive(&crate_archive, &crate_extract, Some(&expected_root))?;
    let crate_root = validate_extracted_crate_root(
        &crate_extract,
        &crate_extract.join(&expected_root),
        &scratch_root,
        &checkout,
    )?;

    let fixture = checkout.join("boxdd-sys/tests/fixtures/native-provider-consumer");
    let consumer_root = scratch_root.join("consumer");
    copy_fixture(&fixture, &consumer_root, &checkout)?;
    let consumer_manifest = consumer_root.join("Cargo.toml");
    rewrite_consumer_dependency(
        &consumer_manifest,
        &crate_root,
        &crate_extract,
        &scratch_root,
        &checkout,
    )?;

    let prepared = match prepared_prebuilt {
        Some(prepared) => {
            revalidate_prebuilt_provider(
                &prepared,
                options,
                &version,
                checkout_commit
                    .as_deref()
                    .expect("prebuilt qualification must retain its checkout commit"),
                &crate_root,
                &scratch_root,
                &checkout,
            )?;
            prepared
        }
        None => prepare_system_provider(options, &version, &crate_root, &checkout)?,
    };
    let target_dir = scratch_root.join(format!(
        "target-{}-{}-{}-{}",
        options.provider.as_str(),
        options.toolchain,
        options.precision.as_str(),
        options.crt
    ));
    let environment = prepared.command_environment(options, &target_dir, &cargo_home);

    let lock_args = vec![
        OsString::from("generate-lockfile"),
        OsString::from("--manifest-path"),
        consumer_manifest.as_os_str().to_os_string(),
    ];
    run_cargo(
        &scratch_root,
        &options.toolchain,
        &lock_args,
        &environment,
        "generate fresh native consumer lockfile",
    )?;

    let metadata_args = vec![
        OsString::from("metadata"),
        OsString::from("--locked"),
        OsString::from("--format-version"),
        OsString::from("1"),
        OsString::from("--manifest-path"),
        consumer_manifest.as_os_str().to_os_string(),
    ];
    let metadata = output_cargo(
        &scratch_root,
        &options.toolchain,
        &metadata_args,
        &environment,
        "resolve fresh native consumer metadata",
    )?;
    let consumer_package_id = validate_metadata_source(
        &metadata.stdout,
        &consumer_manifest,
        &crate_root.join("Cargo.toml"),
        &crate_extract,
        &scratch_root,
        &checkout,
    )?;

    let build_args = vec![
        OsString::from("build"),
        OsString::from("--locked"),
        OsString::from("--manifest-path"),
        consumer_manifest.as_os_str().to_os_string(),
        OsString::from("--features"),
        OsString::from(options.precision.as_str()),
        OsString::from("--target"),
        OsString::from(&options.target),
        OsString::from("--message-format"),
        OsString::from("json-render-diagnostics"),
    ];
    let build = output_cargo(
        &scratch_root,
        &options.toolchain,
        &build_args,
        &environment,
        "build fresh native provider consumer",
    )?;
    let executable = consumer_executable_from_cargo_messages(
        &build.stdout,
        &consumer_package_id,
        &target_dir,
        &scratch_root,
        &checkout,
    )?;
    let receipt = scratch_root.join("consumer-executed.receipt");
    let nonce = qualification_nonce(&scratch_root)?;
    let runtime_environment = prepared.runtime_environment(&cargo_home, &receipt, &nonce);
    run_consumer_executable(&executable, &consumer_root, &runtime_environment)?;
    validate_execution_receipt(&receipt, &nonce, &scratch_root)?;

    println!(
        "qualified packaged boxdd-sys {version} with {} / {} / {} / {} on Rust {}",
        options.provider.as_str(),
        options.precision.as_str(),
        options.target,
        options.crt,
        options.toolchain
    );
    Ok(())
}

fn workspace_version(root: &Path) -> Result<String> {
    let path = root.join("Cargo.toml");
    let source = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let manifest: toml::Value = toml::from_str(&source)
        .map_err(|error| Error::message(format!("{} is invalid TOML: {error}", path.display())))?;
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .filter(|version| !version.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::message("workspace.package.version is required"))
}

fn qualified_checkout_commit(root: &Path) -> Result<String> {
    let mut command = qualified_git_command().map_err(Error::message)?;
    let output = command
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "HEAD^{commit}"])
        .output()
        .map_err(|error| Error::io("resolve native qualification checkout commit", error))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "failed to resolve native qualification checkout commit: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let commit = String::from_utf8(output.stdout)
        .map_err(|error| Error::message(format!("Git returned a non-UTF-8 commit: {error}")))?
        .trim()
        .to_owned();
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::message(format!(
            "native qualification checkout commit must be a lowercase 40-character Git SHA; found {commit:?}"
        )));
    }
    Ok(commit)
}

fn package_boxdd_sys(
    checkout: &Path,
    scratch: &Path,
    cargo_home: &Path,
    options: &Options,
    version: &str,
) -> Result<PathBuf> {
    let target = scratch.join("package-target");
    let mut args = vec![
        OsString::from("package"),
        OsString::from("--locked"),
        OsString::from("--manifest-path"),
        checkout
            .join("boxdd-sys/Cargo.toml")
            .as_os_str()
            .to_os_string(),
        OsString::from("--no-verify"),
        OsString::from("--target-dir"),
        target.as_os_str().to_os_string(),
    ];
    if options.allow_dirty {
        args.push(OsString::from("--allow-dirty"));
    }
    run_cargo(
        scratch,
        &options.toolchain,
        &args,
        &CommandEnvironment::fail_closed(cargo_home),
        "package boxdd-sys for fresh consumption",
    )?;

    let package_dir = target.join("package");
    let expected_name = format!("boxdd-sys-{version}.crate");
    let mut archives = regular_files_with_extension(&package_dir, "crate")?;
    if archives.len() != 1
        || archives[0].file_name().and_then(OsStr::to_str) != Some(expected_name.as_str())
    {
        return Err(Error::message(format!(
            "cargo package must produce exactly {expected_name}; found {archives:?}"
        )));
    }
    let archive = canonicalize(&archives.remove(0), "packaged crate archive")?;
    require_contained(&archive, scratch, "packaged crate archive")?;
    Ok(archive)
}

fn regular_files_with_extension(root: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(root).map_err(|error| Error::io(root, error))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(root, error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| Error::io(&path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::message(format!(
                "package output must not contain symlinks: {}",
                path.display()
            )));
        }
        if metadata.is_file() && path.extension() == Some(OsStr::new(extension)) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn extract_archive(archive_path: &Path, output: &Path, required_root: Option<&str>) -> Result<()> {
    let metadata =
        fs::symlink_metadata(archive_path).map_err(|error| Error::io(archive_path, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "archive must be a regular non-symlink file: {}",
            archive_path.display()
        )));
    }
    let file = File::open(archive_path).map_err(|error| Error::io(archive_path, error))?;
    extract_archive_reader(file, archive_path, output, required_root)
}

fn extract_archive_reader<R: Read>(
    compressed: R,
    archive_path: &Path,
    output: &Path,
    required_root: Option<&str>,
) -> Result<()> {
    let decoder = GzDecoder::new(compressed);
    let mut archive = tar::Archive::new(BoundedReader::new(decoder, MAX_ARCHIVE_STREAM_BYTES));
    let entries = archive.entries().map_err(|error| {
        Error::message(format!(
            "failed to read archive {}: {error}",
            archive_path.display()
        ))
    })?;
    let mut seen = BTreeSet::new();
    let mut total = 0_u64;
    let mut entry_count = 0_usize;
    for entry in entries {
        entry_count = entry_count.checked_add(1).ok_or_else(|| {
            Error::message(format!(
                "archive {} entry count overflow",
                archive_path.display()
            ))
        })?;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            return Err(Error::message(format!(
                "archive {} exceeds the {} entry limit",
                archive_path.display(),
                MAX_ARCHIVE_ENTRIES
            )));
        }
        let mut entry = entry.map_err(|error| {
            Error::message(format!(
                "failed to read an entry from {}: {error}",
                archive_path.display()
            ))
        })?;
        let relative = entry.path().map_err(|error| {
            Error::message(format!(
                "archive {} contains an invalid path: {error}",
                archive_path.display()
            ))
        })?;
        let relative = normalized_archive_path(&relative, archive_path)?;
        if let Some(required_root) = required_root {
            let first = relative.components().next();
            if first != Some(Component::Normal(OsStr::new(required_root))) {
                return Err(Error::message(format!(
                    "archive {} contains an entry outside its single {required_root}/ root: {}",
                    archive_path.display(),
                    relative.display()
                )));
            }
        }
        if !seen.insert(relative.clone()) {
            return Err(Error::message(format!(
                "archive {} repeats entry {}",
                archive_path.display(),
                relative.display()
            )));
        }
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(Error::message(format!(
                "archive {} contains a non-regular entry {} ({entry_type:?})",
                archive_path.display(),
                relative.display()
            )));
        }
        let size = entry.header().size().map_err(|error| {
            Error::message(format!(
                "archive {} has an invalid size for {}: {error}",
                archive_path.display(),
                relative.display()
            ))
        })?;
        if size > MAX_ARCHIVE_ENTRY_BYTES {
            return Err(Error::message(format!(
                "archive entry {} exceeds the {} byte limit",
                relative.display(),
                MAX_ARCHIVE_ENTRY_BYTES
            )));
        }
        total = total.checked_add(size).ok_or_else(|| {
            Error::message(format!("archive {} size overflow", archive_path.display()))
        })?;
        if total > MAX_ARCHIVE_TOTAL_BYTES {
            return Err(Error::message(format!(
                "archive {} exceeds the {} byte total limit",
                archive_path.display(),
                MAX_ARCHIVE_TOTAL_BYTES
            )));
        }
        let destination = output.join(&relative);
        if entry_type.is_dir() {
            fs::create_dir_all(&destination).map_err(|error| Error::io(&destination, error))?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }
        let mut destination_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| Error::io(&destination, error))?;
        let copied = std::io::copy(
            &mut entry.by_ref().take(MAX_ARCHIVE_ENTRY_BYTES + 1),
            &mut destination_file,
        )
        .map_err(|error| Error::io(&destination, error))?;
        if copied != size {
            return Err(Error::message(format!(
                "archive entry {} declared {size} bytes but yielded {copied}",
                relative.display()
            )));
        }
        destination_file
            .flush()
            .map_err(|error| Error::io(&destination, error))?;
    }
    if seen.is_empty() {
        return Err(Error::message(format!(
            "archive {} is empty",
            archive_path.display()
        )));
    }
    Ok(())
}

fn normalized_archive_path(path: &Path, archive: &Path) -> Result<PathBuf> {
    let rendered = path.to_str().ok_or_else(|| {
        Error::message(format!(
            "archive {} contains a non-UTF-8 path",
            archive.display()
        ))
    })?;
    if rendered.is_empty() || rendered.contains('\\') || rendered.contains("//") {
        return Err(Error::message(format!(
            "archive {} contains a non-canonical path {rendered:?}",
            archive.display()
        )));
    }
    let components = path.components().collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::message(format!(
            "archive {} contains an unsafe path {rendered:?}",
            archive.display()
        )));
    }
    Ok(components.iter().collect())
}

fn validate_extracted_crate_root(
    extraction_root: &Path,
    crate_root: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<PathBuf> {
    let extraction_root = canonicalize(extraction_root, "crate extraction root")?;
    let crate_root = canonicalize(crate_root, "extracted crate root")?;
    require_contained(&crate_root, &extraction_root, "extracted crate root")?;
    require_contained(&crate_root, scratch, "extracted crate root")?;
    require_outside(&crate_root, checkout, "extracted crate root")?;
    let manifest = crate_root.join("Cargo.toml");
    let manifest = canonicalize(&manifest, "extracted crate manifest")?;
    if manifest.parent() != Some(crate_root.as_path()) || !manifest.is_file() {
        return Err(Error::message(format!(
            "extracted Cargo.toml must be an immediate regular child of {}",
            crate_root.display()
        )));
    }
    Ok(crate_root)
}

fn copy_fixture(source: &Path, destination: &Path, checkout: &Path) -> Result<()> {
    let source = canonicalize(source, "native provider consumer fixture")?;
    require_contained(&source, checkout, "native provider consumer fixture")?;
    copy_tree(&source, destination)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(|error| Error::io(source, error))?;
    if metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "consumer fixture must not contain symlinks: {}",
            source.display()
        )));
    }
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }
        fs::copy(source, destination).map_err(|error| Error::io(destination, error))?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(Error::message(format!(
            "consumer fixture contains a non-file entry: {}",
            source.display()
        )));
    }
    fs::create_dir(destination).map_err(|error| Error::io(destination, error))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| Error::io(source, error))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| Error::io(source, error))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn rewrite_consumer_dependency(
    manifest_path: &Path,
    crate_root: &Path,
    extraction_root: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<()> {
    let source =
        fs::read_to_string(manifest_path).map_err(|error| Error::io(manifest_path, error))?;
    let mut manifest: toml::Value = toml::from_str(&source).map_err(|error| {
        Error::message(format!(
            "consumer fixture {} is invalid TOML: {error}",
            manifest_path.display()
        ))
    })?;
    let dependency = manifest
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
        .and_then(|dependencies| dependencies.get_mut("boxdd-sys"))
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| {
            Error::message("consumer fixture must declare a structured boxdd-sys dependency")
        })?;
    let crate_root_text = crate_root.to_str().ok_or_else(|| {
        Error::message(format!(
            "extracted crate path is not UTF-8: {}",
            crate_root.display()
        ))
    })?;
    dependency.insert(
        "path".to_owned(),
        toml::Value::String(crate_root_text.to_owned()),
    );
    let rendered = toml::to_string_pretty(&manifest).map_err(|error| {
        Error::message(format!("failed to render consumer Cargo.toml: {error}"))
    })?;
    fs::write(manifest_path, rendered.as_bytes())
        .map_err(|error| Error::io(manifest_path, error))?;
    validate_consumer_dependency(
        rendered.as_bytes(),
        manifest_path,
        crate_root,
        extraction_root,
        scratch,
        checkout,
    )
}

fn validate_consumer_dependency(
    source: &[u8],
    consumer_manifest: &Path,
    expected_crate_root: &Path,
    extraction_root: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<()> {
    let source = std::str::from_utf8(source)
        .map_err(|error| Error::message(format!("consumer Cargo.toml is not UTF-8: {error}")))?;
    let manifest: toml::Value = toml::from_str(source)
        .map_err(|error| Error::message(format!("consumer Cargo.toml is invalid: {error}")))?;
    let path = manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("boxdd-sys"))
        .and_then(|dependency| dependency.get("path"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::message("consumer boxdd-sys dependency path is missing"))?;
    let candidate = Path::new(path);
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        consumer_manifest
            .parent()
            .ok_or_else(|| Error::message("consumer manifest has no parent"))?
            .join(candidate)
    };
    let candidate = canonicalize(&candidate, "consumer boxdd-sys dependency")?;
    let expected = canonicalize(expected_crate_root, "expected extracted crate root")?;
    if candidate != expected {
        return Err(Error::message(format!(
            "consumer boxdd-sys path resolved to {}, expected exact extracted root {}",
            candidate.display(),
            expected.display()
        )));
    }
    require_contained(&candidate, extraction_root, "consumer boxdd-sys dependency")?;
    require_contained(&candidate, scratch, "consumer boxdd-sys dependency")?;
    require_outside(&candidate, checkout, "consumer boxdd-sys dependency")
}

fn validate_metadata_source(
    source: &[u8],
    consumer_manifest: &Path,
    expected_crate_manifest: &Path,
    extraction_root: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<String> {
    let metadata: serde_json::Value = serde_json::from_slice(source).map_err(|error| {
        Error::message(format!("cargo metadata returned invalid JSON: {error}"))
    })?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| Error::message("cargo metadata omitted packages"))?;
    let canonical_consumer = canonicalize(consumer_manifest, "consumer manifest")?;
    let consumers = packages
        .iter()
        .filter(|package| {
            package.get("name").and_then(serde_json::Value::as_str) == Some(CONSUMER_NAME)
        })
        .collect::<Vec<_>>();
    if consumers.len() != 1 {
        return Err(Error::message(format!(
            "cargo metadata must contain exactly one {CONSUMER_NAME} package"
        )));
    }
    let consumer = consumers[0];
    let consumer_package_id = consumer
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Error::message("consumer metadata omitted its package id"))?
        .to_owned();
    let actual_consumer = metadata_manifest_path(consumer, "consumer")?;
    if actual_consumer != canonical_consumer {
        return Err(Error::message(format!(
            "cargo metadata consumer manifest {} does not match {}",
            actual_consumer.display(),
            canonical_consumer.display()
        )));
    }
    let dependencies = consumer
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| Error::message("consumer metadata omitted dependencies"))?;
    let direct = dependencies
        .iter()
        .filter(|dependency| {
            dependency.get("name").and_then(serde_json::Value::as_str) == Some("boxdd-sys")
        })
        .collect::<Vec<_>>();
    if direct.len() != 1
        || direct[0]
            .get("source")
            .is_some_and(|source| !source.is_null())
    {
        return Err(Error::message(
            "consumer metadata must contain one direct local boxdd-sys dependency",
        ));
    }
    let direct_path = direct[0]
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::message("direct boxdd-sys metadata path is missing"))?;
    let direct_path = canonicalize(Path::new(direct_path), "direct boxdd-sys metadata path")?;

    let sys_packages = packages
        .iter()
        .filter(|package| {
            package.get("name").and_then(serde_json::Value::as_str) == Some("boxdd-sys")
        })
        .collect::<Vec<_>>();
    if sys_packages.len() != 1
        || sys_packages[0]
            .get("source")
            .is_some_and(|source| !source.is_null())
    {
        return Err(Error::message(
            "cargo metadata must resolve exactly one local boxdd-sys package",
        ));
    }
    let actual_manifest = metadata_manifest_path(sys_packages[0], "boxdd-sys")?;
    let expected_manifest = canonicalize(expected_crate_manifest, "extracted crate manifest")?;
    if actual_manifest != expected_manifest
        || direct_path != expected_manifest.parent().unwrap_or(Path::new(""))
    {
        return Err(Error::message(format!(
            "cargo metadata resolved boxdd-sys to {} (direct path {}), expected exact packaged manifest {}",
            actual_manifest.display(),
            direct_path.display(),
            expected_manifest.display()
        )));
    }
    require_contained(
        &actual_manifest,
        extraction_root,
        "metadata boxdd-sys manifest",
    )?;
    require_contained(&actual_manifest, scratch, "metadata boxdd-sys manifest")?;
    require_outside(&actual_manifest, checkout, "metadata boxdd-sys manifest")?;
    Ok(consumer_package_id)
}

struct PreparedProvider {
    provider: Provider,
    root: PathBuf,
    manifest: PathBuf,
    archive: Option<PathBuf>,
    provenance: Option<PathBuf>,
    bundle: Option<PathBuf>,
    cosign: Option<PathBuf>,
    manifest_sha256: String,
    archive_sha256: String,
    provenance_sha256: String,
    trusted_root_sha256: String,
}

impl PreparedProvider {
    fn command_environment(
        &self,
        options: &Options,
        target_dir: &Path,
        cargo_home: &Path,
    ) -> CommandEnvironment {
        let mut environment = CommandEnvironment::fail_closed(cargo_home);
        environment.set("BOXDD_SYS_PROVIDER", self.provider.as_str());
        environment.set("BOXDD_SYS_LINK_KIND", "static");
        environment.set("CARGO_TARGET_DIR", target_dir.as_os_str());
        match self.provider {
            Provider::System => {
                environment.set("BOX2D_LIB_DIR", self.root.as_os_str());
                environment.set("BOXDD_SYS_SYSTEM_MANIFEST", self.manifest.as_os_str());
            }
            Provider::Prebuilt => {
                environment.set("BOXDD_SYS_PREBUILT_MANIFEST", self.manifest.as_os_str());
                environment.set(
                    "BOXDD_SYS_PREBUILT_PROVENANCE",
                    self.provenance
                        .as_deref()
                        .expect("prebuilt preparation must bind one provenance statement")
                        .as_os_str(),
                );
                environment.set(
                    "BOXDD_SYS_PREBUILT_BUNDLE",
                    self.bundle
                        .as_deref()
                        .expect("prebuilt preparation must bind one bundle")
                        .as_os_str(),
                );
                environment.set(
                    "BOXDD_SYS_COSIGN",
                    self.cosign
                        .as_deref()
                        .expect("prebuilt preparation must bind one Cosign executable")
                        .as_os_str(),
                );
            }
        }
        if options.crt == "mt" {
            environment.set("RUSTFLAGS", "-C target-feature=+crt-static");
        }
        environment
    }

    fn runtime_environment(
        &self,
        cargo_home: &Path,
        receipt: &Path,
        nonce: &str,
    ) -> CommandEnvironment {
        let mut environment = CommandEnvironment::fail_closed(cargo_home);
        environment.set(
            "BOXDD_NATIVE_QUALIFICATION_PROVIDER",
            self.provider.as_str(),
        );
        environment.set(
            "BOXDD_NATIVE_QUALIFICATION_MANIFEST_SHA256",
            self.manifest_sha256.as_str(),
        );
        environment.set(
            "BOXDD_NATIVE_QUALIFICATION_ARCHIVE_SHA256",
            self.archive_sha256.as_str(),
        );
        environment.set(
            "BOXDD_NATIVE_QUALIFICATION_PROVENANCE_SHA256",
            self.provenance_sha256.as_str(),
        );
        environment.set(
            "BOXDD_NATIVE_QUALIFICATION_TRUSTED_ROOT_SHA256",
            self.trusted_root_sha256.as_str(),
        );
        environment.set("BOXDD_NATIVE_QUALIFICATION_NONCE", nonce);
        environment.set("BOXDD_NATIVE_QUALIFICATION_RECEIPT", receipt.as_os_str());
        environment
    }
}

fn qualification_nonce(scratch: &Path) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::message(format!("system clock is before Unix epoch: {error}")))?;
    let material = format!(
        "boxdd.native-provider.receipt.v1\0{}\0{}\0{}",
        scratch.display(),
        std::process::id(),
        now.as_nanos()
    );
    Ok(provider_manifest::sha256_bytes(material.as_bytes()))
}

fn validate_execution_receipt(receipt: &Path, nonce: &str, scratch: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(receipt).map_err(|error| {
        Error::message(format!(
            "native consumer did not write its execution receipt {}: {error}",
            receipt.display()
        ))
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "native consumer execution receipt must be a regular non-symlink file: {}",
            receipt.display()
        )));
    }
    let receipt = canonicalize(receipt, "native consumer execution receipt")?;
    require_contained(&receipt, scratch, "native consumer execution receipt")?;
    let actual = fs::read_to_string(&receipt).map_err(|error| Error::io(&receipt, error))?;
    if actual == nonce {
        Ok(())
    } else {
        Err(Error::message(
            "native consumer execution receipt did not match this qualification run",
        ))
    }
}

fn prepare_system_provider(
    options: &Options,
    version: &str,
    crate_root: &Path,
    checkout: &Path,
) -> Result<PreparedProvider> {
    if options.provider != Provider::System {
        return Err(Error::message(
            "system provider preparation received a non-system provider",
        ));
    }
    let artifacts = canonicalize(&options.artifacts, "native provider artifact directory")?;
    if !artifacts.is_dir() {
        return Err(Error::message(format!(
            "native provider artifacts must be a directory: {}",
            artifacts.display()
        )));
    }
    require_outside(&artifacts, checkout, "native provider artifact directory")?;
    let manifest = artifacts.join("manifest.toml");
    let identity = validate_provider_manifest(
        &manifest, &artifacts, options, version, crate_root, checkout,
    )?;
    Ok(PreparedProvider {
        provider: Provider::System,
        root: artifacts,
        manifest,
        archive: None,
        provenance: None,
        bundle: None,
        cosign: None,
        manifest_sha256: identity.manifest_sha256,
        archive_sha256: identity.archive_sha256,
        provenance_sha256: String::new(),
        trusted_root_sha256: String::new(),
    })
}

fn prepare_prebuilt_provider(
    options: &Options,
    version: &str,
    checkout_commit: &str,
    checkout_crate_root: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<PreparedProvider> {
    if options.provider != Provider::Prebuilt {
        return Err(Error::message(
            "prebuilt provider preparation received a non-prebuilt provider",
        ));
    }
    let artifacts = canonicalize(&options.artifacts, "native provider artifact directory")?;
    if !artifacts.is_dir() {
        return Err(Error::message(format!(
            "native provider artifacts must be a directory: {}",
            artifacts.display()
        )));
    }
    require_outside(&artifacts, checkout, "native provider artifact directory")?;

    let archive_name = prebuilt_archive_name(version, options);
    let archive_source = find_exact_regular_file(&artifacts, &archive_name)?;
    let provenance_source =
        adjacent_prebuilt_input(&archive_source, ".provenance.toml", "provenance statement")?;
    let bundle_source = adjacent_prebuilt_input(
        &archive_source,
        ".provenance.sigstore.json",
        "Sigstore bundle",
    )?;
    require_contained(
        &provenance_source,
        &artifacts,
        "prebuilt provenance statement",
    )?;
    require_contained(&bundle_source, &artifacts, "prebuilt Sigstore bundle")?;

    let private_inputs = scratch.join("prebuilt-inputs");
    fs::create_dir(&private_inputs).map_err(|error| Error::io(&private_inputs, error))?;
    let private_inputs = canonicalize(&private_inputs, "private prebuilt input directory")?;
    require_contained(&private_inputs, scratch, "private prebuilt input directory")?;
    require_outside(
        &private_inputs,
        checkout,
        "private prebuilt input directory",
    )?;
    let archive = snapshot_bounded_regular_file(
        &archive_source,
        &private_inputs.join(&archive_name),
        MAX_PACKAGE_BYTES,
        "prebuilt package",
    )?;
    let provenance_name = format!("{archive_name}.provenance.toml");
    let provenance = snapshot_bounded_regular_file(
        &provenance_source,
        &private_inputs.join(provenance_name),
        MAX_PROVENANCE_STATEMENT_BYTES,
        "prebuilt provenance statement",
    )?;
    let bundle_name = format!("{archive_name}.provenance.sigstore.json");
    let bundle = snapshot_bounded_regular_file(
        &bundle_source,
        &private_inputs.join(bundle_name),
        MAX_SIGSTORE_BUNDLE_BYTES,
        "prebuilt Sigstore bundle",
    )?;

    let statement =
        read_and_validate_prebuilt_statement(&provenance, options, version, checkout_commit)?;
    let cosign = resolve_executable(
        options
            .cosign
            .as_deref()
            .expect("validated prebuilt options must include Cosign"),
    )?;
    verify_cosign_version(&cosign)?;
    let (checkout_trusted_root, trusted_root_sha256) = snapshot_crate_trusted_root(
        checkout_crate_root,
        &private_inputs.join("trusted-root.checkout.json"),
        "checkout crate-owned Sigstore trusted root",
    )?;
    verify_prebuilt_signature(
        &cosign,
        &provenance,
        &bundle,
        &checkout_trusted_root,
        &statement,
    )?;

    let extraction = scratch.join("prebuilt-artifact");
    extract_authenticated_prebuilt(&statement, &archive, &extraction)?;
    let extraction = canonicalize(&extraction, "prebuilt artifact extraction root")?;
    require_contained(&extraction, scratch, "prebuilt artifact extraction root")?;
    require_outside(&extraction, checkout, "prebuilt artifact extraction root")?;
    validate_signed_provider_tree(&statement, &extraction)?;
    let manifest = extraction.join("manifest.toml");
    let identity = validate_provider_manifest(
        &manifest,
        &extraction,
        options,
        version,
        checkout_crate_root,
        checkout,
    )?;
    let provenance_sha256 = provider_manifest::sha256_file(&provenance).map_err(Error::message)?;

    Ok(PreparedProvider {
        provider: Provider::Prebuilt,
        root: extraction,
        manifest,
        archive: Some(archive),
        provenance: Some(provenance),
        bundle: Some(bundle),
        cosign: Some(cosign),
        manifest_sha256: identity.manifest_sha256,
        archive_sha256: identity.archive_sha256,
        provenance_sha256,
        trusted_root_sha256,
    })
}

fn revalidate_prebuilt_provider(
    prepared: &PreparedProvider,
    options: &Options,
    version: &str,
    checkout_commit: &str,
    packaged_crate_root: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<()> {
    if prepared.provider != Provider::Prebuilt || options.provider != Provider::Prebuilt {
        return Err(Error::message(
            "packaged prebuilt revalidation requires a prebuilt provider",
        ));
    }
    validate_packaged_prebuilt_policy(packaged_crate_root, checkout)?;
    let provenance = prepared
        .provenance
        .as_deref()
        .ok_or_else(|| Error::message("prebuilt preparation omitted its provenance statement"))?;
    let current_checkout_commit = qualified_checkout_commit(checkout)?;
    if current_checkout_commit != checkout_commit {
        return Err(Error::message(format!(
            "checkout HEAD changed during prebuilt qualification: expected {checkout_commit}, found {current_checkout_commit}"
        )));
    }
    let statement =
        read_and_validate_prebuilt_statement(provenance, options, version, checkout_commit)?;
    let archive = prepared
        .archive
        .as_deref()
        .ok_or_else(|| Error::message("prebuilt preparation omitted its package snapshot"))?;
    let bundle = prepared
        .bundle
        .as_deref()
        .ok_or_else(|| Error::message("prebuilt preparation omitted its Sigstore bundle"))?;
    let cosign = prepared
        .cosign
        .as_deref()
        .ok_or_else(|| Error::message("prebuilt preparation omitted its Cosign executable"))?;
    verify_cosign_version(cosign)?;
    let packaged_trusted_root_destination = scratch
        .join("prebuilt-inputs")
        .join("trusted-root.packaged.json");
    let (packaged_trusted_root, trusted_root_sha256) = snapshot_crate_trusted_root(
        packaged_crate_root,
        &packaged_trusted_root_destination,
        "packaged crate-owned Sigstore trusted root",
    )?;
    if trusted_root_sha256 != prepared.trusted_root_sha256 {
        return Err(Error::message(
            "packaged crate-owned Sigstore trusted root changed after checkout qualification",
        ));
    }
    verify_prebuilt_signature(
        cosign,
        provenance,
        bundle,
        &packaged_trusted_root,
        &statement,
    )?;
    statement.verify_outer_package(archive).map_err(|error| {
        Error::message(format!(
            "packaged qualification prebuilt package mismatch: {error}"
        ))
    })?;
    validate_signed_provider_tree(&statement, &prepared.root)?;
    let identity = validate_provider_manifest(
        &prepared.manifest,
        &prepared.root,
        options,
        version,
        packaged_crate_root,
        checkout,
    )?;
    if identity.manifest_sha256 != prepared.manifest_sha256
        || identity.archive_sha256 != prepared.archive_sha256
    {
        return Err(Error::message(
            "provider identity changed between checkout and packaged-crate qualification",
        ));
    }
    Ok(())
}

fn snapshot_bounded_regular_file(
    source: &Path,
    destination: &Path,
    maximum_bytes: u64,
    label: &str,
) -> Result<PathBuf> {
    let source_metadata = fs::symlink_metadata(source).map_err(|error| Error::io(source, error))?;
    if !source_metadata.file_type().is_file() || source_metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be a regular non-symlink file: {}",
            source.display()
        )));
    }
    if source_metadata.len() == 0 || source_metadata.len() > maximum_bytes {
        return Err(Error::message(format!(
            "{label} size {} is outside the accepted 1..={maximum_bytes} byte range",
            source_metadata.len()
        )));
    }
    let mut input = File::open(source).map_err(|error| Error::io(source, error))?;
    let opened_metadata = input.metadata().map_err(|error| Error::io(source, error))?;
    if !opened_metadata.is_file() || opened_metadata.len() != source_metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while it was being opened for snapshotting: {}",
            source.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if opened_metadata.dev() != source_metadata.dev()
            || opened_metadata.ino() != source_metadata.ino()
        {
            return Err(Error::message(format!(
                "{label} changed while it was being opened for snapshotting: {}",
                source.display()
            )));
        }
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| Error::io(destination, error))?;
    let copied = io::copy(
        &mut Read::by_ref(&mut input).take(maximum_bytes + 1),
        &mut output,
    )
    .map_err(|error| Error::io(destination, error))?;
    if copied != source_metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while its exact bytes were being snapshotted: {}",
            source.display()
        )));
    }
    output
        .flush()
        .map_err(|error| Error::io(destination, error))?;
    let destination = canonicalize(destination, &format!("private {label} snapshot"))?;
    let destination_metadata =
        fs::symlink_metadata(&destination).map_err(|error| Error::io(&destination, error))?;
    if !destination_metadata.file_type().is_file()
        || destination_metadata.file_type().is_symlink()
        || destination_metadata.len() != copied
    {
        return Err(Error::message(format!(
            "private {label} snapshot is not the exact regular file written: {}",
            destination.display()
        )));
    }
    Ok(destination)
}

fn read_and_validate_prebuilt_statement(
    path: &Path,
    options: &Options,
    version: &str,
    checkout_commit: &str,
) -> Result<PrebuiltProvenanceStatement> {
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_PROVENANCE_STATEMENT_BYTES
    {
        return Err(Error::message(format!(
            "prebuilt provenance statement must be a bounded regular non-symlink file: {}",
            path.display()
        )));
    }
    let bytes = fs::read(path).map_err(|error| Error::io(path, error))?;
    if bytes.len() as u64 != metadata.len() {
        return Err(Error::message(
            "prebuilt provenance statement changed while it was being read",
        ));
    }
    let statement = PrebuiltProvenanceStatement::parse_canonical(&bytes).map_err(|error| {
        Error::message(format!("invalid prebuilt provenance statement: {error}"))
    })?;
    statement
        .validate_publisher(PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW)
        .map_err(|error| {
            Error::message(format!("untrusted prebuilt provenance statement: {error}"))
        })?;
    if statement.source_commit != checkout_commit {
        return Err(Error::message(format!(
            "prebuilt provenance source commit {} does not match qualified checkout HEAD {checkout_commit}",
            statement.source_commit
        )));
    }
    if statement.crate_version != version
        || statement.target != options.target
        || statement.precision != options.precision.as_str()
        || statement.crt != options.crt
    {
        return Err(Error::message(format!(
            "prebuilt provenance coordinates {}/{}/{}/{} do not match requested {version}/{}/{}/{}",
            statement.crate_version,
            statement.target,
            statement.precision,
            statement.crt,
            options.target,
            options.precision.as_str(),
            options.crt,
        )));
    }
    Ok(statement)
}

fn snapshot_crate_trusted_root(
    crate_root: &Path,
    destination: &Path,
    label: &str,
) -> Result<(PathBuf, String)> {
    let crate_root = canonicalize(crate_root, "crate root for Sigstore policy")?;
    let source = crate_root.join(SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH);
    let metadata = fs::symlink_metadata(&source).map_err(|error| Error::io(&source, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be a regular non-symlink file: {}",
            source.display()
        )));
    }
    let source = canonicalize(&source, label)?;
    require_contained(&source, &crate_root, label)?;
    let snapshot =
        snapshot_bounded_regular_file(&source, destination, MAX_TRUSTED_ROOT_BYTES, label)?;
    let digest = provider_manifest::sha256_file(&snapshot).map_err(Error::message)?;
    if digest != SIGSTORE_TRUSTED_ROOT_SHA256 {
        return Err(Error::message(format!(
            "{label} digest {digest} does not match the qualified crate-owned trust anchor {SIGSTORE_TRUSTED_ROOT_SHA256}"
        )));
    }
    Ok((snapshot, digest))
}

fn verify_cosign_version(cosign: &Path) -> Result<()> {
    let output = Command::new(cosign)
        .arg("version")
        .output()
        .map_err(|error| Error::io(cosign, error))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "prebuilt qualification requires Cosign {COSIGN_VERSION}; version command failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let version_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if provenance_policy::cosign_version_is_qualified(&version_text) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "prebuilt qualification requires exact Cosign {COSIGN_VERSION}; found {}",
            version_text
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("unknown version")
        )))
    }
}

fn verify_prebuilt_signature(
    cosign: &Path,
    provenance: &Path,
    bundle: &Path,
    trusted_root: &Path,
    statement: &PrebuiltProvenanceStatement,
) -> Result<()> {
    let args = provenance_policy::cosign_verify_blob_args(provenance_policy::PrebuiltProvenance {
        crate_version: &statement.crate_version,
        source_commit: &statement.source_commit,
        release_tag: &statement.release_tag,
        payload: provenance,
        bundle,
        trusted_root,
    })
    .map_err(|error| Error::message(format!("invalid prebuilt Sigstore policy input: {error}")))?;
    let output = Command::new(cosign)
        .args(args)
        .output()
        .map_err(|error| Error::io("verify prebuilt provenance signature", error))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "prebuilt provenance signature verification failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn extract_authenticated_prebuilt(
    statement: &PrebuiltProvenanceStatement,
    archive: &Path,
    extraction: &Path,
) -> Result<()> {
    let package_bytes = statement.verify_outer_package(archive).map_err(|error| {
        Error::message(format!(
            "prebuilt package does not match signed provenance: {error}"
        ))
    })?;
    fs::create_dir(extraction).map_err(|error| Error::io(extraction, error))?;
    extract_archive_reader(io::Cursor::new(package_bytes), archive, extraction, None)
}

fn validate_signed_provider_tree(
    statement: &PrebuiltProvenanceStatement,
    root: &Path,
) -> Result<()> {
    statement.verify_extracted_root(root).map_err(|error| {
        Error::message(format!(
            "prebuilt extracted tree does not match signed provenance: {error}"
        ))
    })?;
    let manifest_path = root.join("manifest.toml");
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
    statement
        .validate_provider_manifest(&manifest_bytes)
        .map_err(|error| {
            Error::message(format!(
                "prebuilt provider manifest does not match signed provenance: {error}"
            ))
        })?;
    Ok(())
}

fn validate_packaged_prebuilt_policy(packaged_crate_root: &Path, checkout: &Path) -> Result<()> {
    let checkout_crate_root = canonicalize(&checkout.join("boxdd-sys"), "checkout boxdd-sys")?;
    let packaged_crate_root = canonicalize(packaged_crate_root, "packaged boxdd-sys")?;
    for relative in [
        "build.rs",
        "src/build_support.rs",
        "src/prebuilt_provenance.rs",
        "src/provenance_policy.rs",
        "src/provider_archive.rs",
        "src/provider_manifest.rs",
        "src/source_overlay.rs",
    ] {
        let checkout_path = checkout_crate_root.join(relative);
        let packaged_path = packaged_crate_root.join(relative);
        let checkout_bytes = read_regular_file(&checkout_path, "checkout prebuilt policy input")?;
        let packaged_bytes = read_regular_file(&packaged_path, "packaged prebuilt policy input")?;
        if packaged_bytes != checkout_bytes {
            return Err(Error::message(format!(
                "packaged prebuilt policy {relative} does not match the qualified checkout"
            )));
        }
    }
    Ok(())
}

fn read_regular_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        )));
    }
    let bytes = fs::read(path).map_err(|error| Error::io(path, error))?;
    if bytes.len() as u64 != metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while it was being read: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

struct ProviderIdentity {
    manifest_sha256: String,
    archive_sha256: String,
}

fn validate_provider_manifest(
    manifest_path: &Path,
    root: &Path,
    options: &Options,
    version: &str,
    crate_root: &Path,
    checkout: &Path,
) -> Result<ProviderIdentity> {
    let metadata =
        fs::symlink_metadata(manifest_path).map_err(|error| Error::io(manifest_path, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "provider manifest must be a regular non-symlink file: {}",
            manifest_path.display()
        )));
    }
    let root = canonicalize(root, "provider artifact root")?;
    let manifest_path = canonicalize(manifest_path, "provider manifest")?;
    if manifest_path != root.join("manifest.toml") {
        return Err(Error::message(format!(
            "provider manifest must be the exact manifest.toml at {}",
            root.display()
        )));
    }
    let bytes = fs::read(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
    let manifest = provider_manifest::ArtifactManifest::parse(&bytes).map_err(Error::message)?;
    if manifest.render() != bytes {
        return Err(Error::message(
            "provider manifest must use its canonical byte representation",
        ));
    }
    let repository_sys = checkout.join("boxdd-sys");
    let effective_source = effective_source_identity(&repository_sys).map_err(|error| {
        Error::message(format!(
            "cannot recompute the qualification effective-source identity: {error}"
        ))
    })?;
    let repository_effective_manifest = fs::read(repository_sys.join("effective-source.toml"))
        .map_err(|error| Error::io(repository_sys.join("effective-source.toml"), error))?;
    let packaged_effective_manifest = fs::read(crate_root.join("effective-source.toml"))
        .map_err(|error| Error::io(crate_root.join("effective-source.toml"), error))?;
    if packaged_effective_manifest != repository_effective_manifest {
        return Err(Error::message(
            "packaged boxdd-sys effective-source.toml does not match the checkout",
        ));
    }
    if options.provider == Provider::Prebuilt {
        let artifact_effective_manifest = fs::read(root.join("metadata/effective-source.toml"))
            .map_err(|error| Error::io(root.join("metadata/effective-source.toml"), error))?;
        if artifact_effective_manifest != repository_effective_manifest {
            return Err(Error::message(
                "prebuilt metadata/effective-source.toml does not match the checkout",
            ));
        }
    }
    let adapter_source_sha256 =
        provider_manifest::adapter_source_sha256(crate_root).map_err(Error::message)?;
    let snapshot_layout_hash = u32::try_from(manifest.snapshot_layout_hash).map_err(|_| {
        Error::message("provider snapshot layout hash does not fit the native u32 contract")
    })?;
    let header = crate_root.join("third-party/box2d/include/box2d/box2d.h");
    let bindings = crate_root
        .join("src")
        .join(if options.precision == Precision::Double {
            "bindings_double.rs"
        } else {
            "bindings_pregenerated.rs"
        });
    let verified = provider_manifest::verify_artifact(
        &manifest_path,
        &provider_manifest::ArtifactExpectation {
            identity: provider_manifest::ArtifactIdentityExpectation {
                provider: options.provider.as_str(),
                crate_version: version,
                upstream_sha: &effective_source.upstream_sha,
                effective_source_sha256: &effective_source.effective_source_sha256,
                precision: options.precision.as_str(),
                target: &options.target,
                crt: &options.crt,
                simd: "default",
                validate: false,
                adapter_source_sha256: &adapter_source_sha256,
                private_abi_hash: &manifest.private_abi_hash,
                snapshot_layout_hash,
            },
            header_path: &header,
            bindings_path: &bindings,
        },
    )
    .map_err(|error| Error::message(format!("provider artifact validation failed: {error}")))?;
    let verified_archive = verify_provider_archive(
        &verified.archive_path,
        &ArchiveExpectation {
            target: &options.target,
            required_symbols: provider_manifest::REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &manifest.private_abi_hash,
            snapshot_layout_hash,
        },
    )
    .map_err(|error| Error::message(format!("provider archive proof failed: {error}")))?;
    if verified_archive.archive_sha256 != verified.archive_sha256 {
        return Err(Error::message(
            "provider archive changed between manifest and structural verification",
        ));
    }
    if options.provider == Provider::System
        && verified.archive_path.parent() != Some(root.as_path())
    {
        return Err(Error::message(
            "system provider archive must be an immediate child of BOX2D_LIB_DIR",
        ));
    }
    Ok(ProviderIdentity {
        manifest_sha256: verified.manifest_sha256,
        archive_sha256: verified.archive_sha256,
    })
}

fn prebuilt_archive_name(version: &str, options: &Options) -> String {
    let crt = if options.crt == "none" {
        String::new()
    } else {
        format!("-{}", options.crt)
    };
    format!(
        "boxdd-prebuilt-{version}-{}-{}-static{crt}.tar.gz",
        options.target,
        options.precision.as_str()
    )
}

fn find_exact_regular_file(root: &Path, name: &str) -> Result<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(directory) = stack.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| Error::io(&directory, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| Error::io(&directory, error))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| Error::io(&path, error))?;
            if metadata.file_type().is_symlink() {
                return Err(Error::message(format!(
                    "prebuilt input tree must not contain symlinks: {}",
                    path.display()
                )));
            }
            if metadata.is_dir() {
                stack.push(path);
            } else if metadata.is_file() && path.file_name() == Some(OsStr::new(name)) {
                found.push(canonicalize(&path, "prebuilt archive")?);
            }
        }
    }
    found.sort();
    if found.len() == 1 {
        Ok(found.remove(0))
    } else {
        Err(Error::message(format!(
            "expected exactly one prebuilt archive named {name:?}; found {found:?}"
        )))
    }
}

fn adjacent_prebuilt_input(archive: &Path, suffix: &str, label: &str) -> Result<PathBuf> {
    let mut name = archive.as_os_str().to_os_string();
    name.push(suffix);
    let path = PathBuf::from(name);
    let metadata = fs::symlink_metadata(&path).map_err(|error| Error::io(&path, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "prebuilt {label} must be a regular adjacent non-symlink file: {}",
            path.display()
        )));
    }
    canonicalize(&path, &format!("prebuilt {label}"))
}

fn resolve_executable(requested: &Path) -> Result<PathBuf> {
    let candidates = if requested.is_absolute() || requested.components().count() > 1 {
        vec![requested.to_path_buf()]
    } else {
        let path = env::var_os("PATH")
            .ok_or_else(|| Error::message("PATH is required to resolve the Cosign executable"))?;
        env::split_paths(&path)
            .flat_map(|directory| executable_candidates(&directory, requested))
            .collect::<Vec<_>>()
    };
    for candidate in candidates {
        let Ok(canonical) = fs::canonicalize(&candidate) else {
            continue;
        };
        if !canonical.is_file() || !is_executable(&canonical)? {
            continue;
        }
        return Ok(canonical);
    }
    Err(Error::message(format!(
        "failed to resolve executable Cosign path {:?}",
        requested
    )))
}

fn executable_candidates(directory: &Path, requested: &Path) -> Vec<PathBuf> {
    let direct = directory.join(requested);
    #[cfg(windows)]
    {
        let mut candidates = vec![direct.clone()];
        if requested.extension().is_none() {
            let extensions = env::var_os("PATHEXT").unwrap_or_else(|| ".EXE;.COM".into());
            candidates.extend(
                extensions
                    .to_string_lossy()
                    .split(';')
                    .filter(|extension| !extension.is_empty())
                    .map(|extension| {
                        let mut name = requested.as_os_str().to_os_string();
                        name.push(extension);
                        directory.join(name)
                    }),
            );
        }
        candidates
    }
    #[cfg(not(windows))]
    {
        vec![direct]
    }
}

fn is_executable(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).map_err(|error| Error::io(path, error))?;
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(path.is_file())
    }
}

fn metadata_manifest_path(package: &serde_json::Value, label: &str) -> Result<PathBuf> {
    let path = package
        .get("manifest_path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::message(format!("{label} metadata omitted manifest_path")))?;
    canonicalize(Path::new(path), &format!("{label} metadata manifest"))
}

fn consumer_executable_from_cargo_messages(
    source: &[u8],
    expected_package_id: &str,
    target_dir: &Path,
    scratch: &Path,
    checkout: &Path,
) -> Result<PathBuf> {
    let mut executables = BTreeSet::new();
    for (index, line) in source.split(|byte| *byte == b'\n').enumerate() {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let message: serde_json::Value = serde_json::from_slice(line).map_err(|error| {
            Error::message(format!(
                "cargo build message {} is not valid JSON: {error}",
                index + 1
            ))
        })?;
        if message.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-artifact")
            || message
                .get("package_id")
                .and_then(serde_json::Value::as_str)
                != Some(expected_package_id)
        {
            continue;
        }
        let target = message
            .get("target")
            .ok_or_else(|| Error::message("consumer compiler artifact omitted target"))?;
        let is_consumer_binary = target.get("name").and_then(serde_json::Value::as_str)
            == Some(CONSUMER_NAME)
            && target
                .get("kind")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")));
        if !is_consumer_binary {
            continue;
        }
        let executable = message
            .get("executable")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| Error::message("consumer compiler artifact omitted executable"))?;
        executables.insert(PathBuf::from(executable));
    }
    if executables.len() != 1 {
        return Err(Error::message(format!(
            "cargo build must report exactly one {CONSUMER_NAME} executable; found {executables:?}"
        )));
    }
    let executable = executables
        .pop_first()
        .expect("one executable was required above");
    let metadata =
        fs::symlink_metadata(&executable).map_err(|error| Error::io(&executable, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "consumer executable must be a regular non-symlink file: {}",
            executable.display()
        )));
    }
    let executable = canonicalize(&executable, "fresh consumer executable")?;
    require_contained(&executable, target_dir, "fresh consumer executable")?;
    require_contained(&executable, scratch, "fresh consumer executable")?;
    require_outside(&executable, checkout, "fresh consumer executable")?;
    Ok(executable)
}

fn run_consumer_executable(
    executable: &Path,
    current_dir: &Path,
    environment: &CommandEnvironment,
) -> Result<()> {
    let mut command = consumer_command(executable, current_dir, environment);
    let status = command
        .status()
        .map_err(|error| Error::io("run fresh native provider consumer directly", error))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "fresh native provider consumer failed with status {status}"
        )))
    }
}

fn consumer_command(
    executable: &Path,
    current_dir: &Path,
    environment: &CommandEnvironment,
) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(current_dir);
    environment.apply(&mut command);
    command
}

fn run_cargo(
    current_dir: &Path,
    toolchain: &str,
    args: &[OsString],
    environment: &CommandEnvironment,
    label: &str,
) -> Result<()> {
    let mut command = cargo_command(current_dir, toolchain, args, environment)?;
    let status = command.status().map_err(|error| Error::io(label, error))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with status {status}"
        )))
    }
}

fn output_cargo(
    current_dir: &Path,
    toolchain: &str,
    args: &[OsString],
    environment: &CommandEnvironment,
    label: &str,
) -> Result<Output> {
    let output = cargo_command(current_dir, toolchain, args, environment)?
        .output()
        .map_err(|error| Error::io(label, error))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(Error::message(format!(
            "{label} failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn cargo_command(
    current_dir: &Path,
    toolchain: &str,
    args: &[OsString],
    environment: &CommandEnvironment,
) -> Result<Command> {
    let args = cargo_arguments(toolchain, args)?;
    let mut command = Command::new("cargo");
    command.args(args).current_dir(current_dir);
    environment.apply(&mut command);
    Ok(command)
}

fn validate_cargo_configuration_isolation(
    working_directory: &Path,
    cargo_home: &Path,
) -> Result<()> {
    let working_directory = canonicalize(working_directory, "Cargo working directory")?;
    let cargo_home = canonicalize(cargo_home, "isolated Cargo home")?;
    if cargo_home.parent() != Some(working_directory.as_path()) {
        return Err(Error::message(format!(
            "isolated Cargo home {} must be an immediate child of qualification root {}",
            cargo_home.display(),
            working_directory.display()
        )));
    }
    let mut cargo_home_entries =
        fs::read_dir(&cargo_home).map_err(|error| Error::io(&cargo_home, error))?;
    if cargo_home_entries
        .next()
        .transpose()
        .map_err(|error| Error::io(&cargo_home, error))?
        .is_some()
    {
        return Err(Error::message(format!(
            "isolated Cargo home must start empty: {}",
            cargo_home.display()
        )));
    }

    for directory in working_directory.ancestors() {
        for relative in [".cargo/config.toml", ".cargo/config"] {
            let candidate = directory.join(relative);
            match fs::symlink_metadata(&candidate) {
                Ok(_) => {
                    return Err(Error::message(format!(
                        "Cargo configuration search is not isolated; found {}",
                        candidate.display()
                    )));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::io(&candidate, error)),
            }
        }
    }
    Ok(())
}

fn canonicalize(path: &Path, label: &str) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|error| {
        Error::message(format!(
            "failed to resolve {label} {}: {error}",
            path.display()
        ))
    })
}

fn require_contained(path: &Path, root: &Path, label: &str) -> Result<()> {
    let root = canonicalize(root, "containment root")?;
    if path.starts_with(&root) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} {} escapes {}",
            path.display(),
            root.display()
        )))
    }
}

fn require_outside(path: &Path, forbidden: &Path, label: &str) -> Result<()> {
    let forbidden = canonicalize(forbidden, "forbidden checkout root")?;
    if path.starts_with(&forbidden) {
        Err(Error::message(format!(
            "{label} resolved back into the checkout: {}",
            path.display()
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_archive(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            archive.append_data(&mut header, *name, *bytes).unwrap();
        }
        archive.finish().unwrap();
    }

    fn write_link_archive(path: &Path, entry_type: tar::EntryType) {
        let file = File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(entry_type);
        header.set_mode(0o777);
        header.set_size(0);
        header.set_link_name("outside").unwrap();
        header.set_cksum();
        archive
            .append_data(&mut header, "boxdd-sys-0.6.0/Cargo.toml", std::io::empty())
            .unwrap();
        archive.finish().unwrap();
    }

    fn metadata_json(consumer_manifest: &Path, direct_path: &Path, sys_manifest: &Path) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "packages": [
                {
                    "id": "path+file:///qualified/consumer#boxdd-native-provider-consumer@0.0.0",
                    "name": CONSUMER_NAME,
                    "manifest_path": consumer_manifest,
                    "dependencies": [{
                        "name": "boxdd-sys",
                        "path": direct_path,
                        "source": null
                    }]
                },
                {
                    "id": "path+file:///qualified/crate#boxdd-sys@0.6.0",
                    "name": "boxdd-sys",
                    "manifest_path": sys_manifest,
                    "source": null,
                    "dependencies": []
                }
            ]
        }))
        .unwrap()
    }

    fn arguments(provider: &str, precision: &str, target: &str, crt: &str) -> Vec<String> {
        let mut args = vec![
            "--provider",
            provider,
            "--toolchain",
            "1.95.0",
            "--precision",
            precision,
            "--target",
            target,
            "--crt",
            crt,
            "--artifacts",
            "artifacts",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        if provider == "prebuilt" {
            args.extend(["--cosign".to_owned(), "cosign".to_owned()]);
        }
        args
    }

    fn prebuilt_statement(package_name: &str, package_bytes: &[u8]) -> PrebuiltProvenanceStatement {
        use crate::prebuilt_provenance::{
            MemberDigest, SCHEMA_NAME, SCHEMA_VERSION, canonical_inner_checksums_bytes,
            sha256_bytes,
        };

        let manifest = b"canonical provider manifest";
        let mut members = vec![
            MemberDigest {
                path: "checksums.sha256".to_owned(),
                size: 0,
                sha256: "0".repeat(64),
            },
            MemberDigest {
                path: "manifest.toml".to_owned(),
                size: manifest.len() as u64,
                sha256: sha256_bytes(manifest),
            },
        ];
        let checksums = canonical_inner_checksums_bytes(&members).unwrap();
        members[0].size = checksums.len() as u64;
        members[0].sha256 = sha256_bytes(&checksums);
        PrebuiltProvenanceStatement {
            schema_version: SCHEMA_VERSION,
            schema: SCHEMA_NAME.to_owned(),
            repository: PUBLISHER_REPOSITORY.to_owned(),
            workflow: PUBLISHER_WORKFLOW.to_owned(),
            workflow_ref: format!("{PUBLISHER_REPOSITORY}/{PUBLISHER_WORKFLOW}@refs/tags/v0.6.0"),
            source_commit: "a".repeat(40),
            release_tag: "v0.6.0".to_owned(),
            run_id: "1".to_owned(),
            run_attempt: "1".to_owned(),
            crate_version: "0.6.0".to_owned(),
            package_name: package_name.to_owned(),
            package_size: package_bytes.len() as u64,
            package_sha256: sha256_bytes(package_bytes),
            provider_manifest_sha256: sha256_bytes(manifest),
            inner_checksums_sha256: sha256_bytes(&checksums),
            provider: "prebuilt".to_owned(),
            target: "x86_64-unknown-linux-gnu".to_owned(),
            precision: "single".to_owned(),
            link: "static".to_owned(),
            crt: "none".to_owned(),
            upstream_sha: "b".repeat(40),
            effective_source_sha256: "c".repeat(64),
            simd: "default".to_owned(),
            validate: false,
            adapter_abi_version: provider_manifest::ADAPTER_ABI_VERSION,
            adapter_source_sha256: "d".repeat(64),
            private_abi_hash: "e".repeat(64),
            snapshot_layout_hash: 1,
            recording_contract_blake3: provider_manifest::RECORDING_CONTRACT_BLAKE3.to_owned(),
            member_count: members.len() as u64,
            members,
        }
    }

    #[test]
    fn parser_accepts_only_qualified_provider_coordinates() {
        assert!(
            Options::parse(&arguments(
                "system",
                "single",
                "x86_64-unknown-linux-gnu",
                "none"
            ))
            .is_ok()
        );
        assert!(
            Options::parse(&arguments(
                "prebuilt",
                "double",
                "x86_64-pc-windows-msvc",
                "mt"
            ))
            .is_ok()
        );
        assert!(
            Options::parse(&arguments(
                "vendored",
                "single",
                "x86_64-unknown-linux-gnu",
                "none"
            ))
            .is_err()
        );
        assert!(
            Options::parse(&arguments(
                "system",
                "quad",
                "x86_64-unknown-linux-gnu",
                "none"
            ))
            .is_err()
        );
        assert!(
            Options::parse(&arguments(
                "system",
                "single",
                "x86_64-pc-windows-msvc",
                "none"
            ))
            .is_err()
        );
    }

    #[test]
    fn parser_rejects_default_or_unqualified_toolchains() {
        let mut args = arguments("system", "single", "x86_64-unknown-linux-gnu", "none");
        let value = args
            .iter_mut()
            .find(|value| value.as_str() == "1.95.0")
            .unwrap();
        *value = "stable".to_owned();
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn every_child_cargo_plan_starts_with_the_exact_toolchain() {
        let args = cargo_arguments("1.97.1", &["metadata", "--locked"]).unwrap();
        assert_eq!(args[0], OsString::from("+1.97.1"));
        assert_eq!(args[1], OsString::from("metadata"));
        assert!(cargo_arguments("stable", &["metadata"]).is_err());
    }

    #[test]
    fn fail_closed_environment_removes_runners_and_compiler_replacements() {
        assert!(is_qualification_sensitive_env(OsStr::new(
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER"
        )));
        assert!(is_qualification_sensitive_env(OsStr::new(
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS"
        )));
        let temp = tempfile::tempdir().unwrap();
        let environment = CommandEnvironment::fail_closed(temp.path());
        for key in [
            "RUSTC",
            "RUSTC_BOOTSTRAP",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUNNER",
            "CARGO_TARGET_DIR",
            "BOXDD_SYS_PREBUILT_PROVENANCE",
        ] {
            assert!(environment.remove.contains(OsStr::new(key)), "{key}");
        }
        assert!(is_qualification_sensitive_env(OsStr::new(
            "CARGO_UNSTABLE_BYPASS_PRELUDE"
        )));
        assert!(is_qualification_sensitive_env(OsStr::new(
            "CARGO_BUILD_RUSTFLAGS"
        )));
        assert_eq!(
            environment
                .values
                .iter()
                .find(|(key, _)| key == OsStr::new("CARGO_HOME"))
                .map(|(_, value)| value.as_os_str()),
            Some(temp.path().as_os_str())
        );
    }

    #[test]
    fn prebuilt_sidecars_require_canonical_provenance_names() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp
            .path()
            .join("boxdd-prebuilt-0.6.0-x86_64-unknown-linux-gnu-single-static.tar.gz");
        fs::write(&archive, b"archive").unwrap();
        let mut legacy_name = archive.as_os_str().to_os_string();
        legacy_name.push(".sigstore.json");
        fs::write(PathBuf::from(legacy_name), b"legacy").unwrap();
        assert!(
            adjacent_prebuilt_input(&archive, ".provenance.toml", "provenance statement").is_err()
        );
        assert!(
            adjacent_prebuilt_input(&archive, ".provenance.sigstore.json", "Sigstore bundle")
                .is_err()
        );

        let mut provenance_name = archive.as_os_str().to_os_string();
        provenance_name.push(".provenance.toml");
        let provenance = PathBuf::from(provenance_name);
        fs::write(&provenance, b"statement").unwrap();
        let mut bundle_name = archive.as_os_str().to_os_string();
        bundle_name.push(".provenance.sigstore.json");
        let bundle = PathBuf::from(bundle_name);
        fs::write(&bundle, b"bundle").unwrap();
        assert_eq!(
            adjacent_prebuilt_input(&archive, ".provenance.toml", "provenance statement").unwrap(),
            fs::canonicalize(provenance).unwrap()
        );
        assert_eq!(
            adjacent_prebuilt_input(&archive, ".provenance.sigstore.json", "Sigstore bundle")
                .unwrap(),
            fs::canonicalize(bundle).unwrap()
        );
    }

    #[test]
    fn prebuilt_input_snapshot_is_bounded_non_symlink_and_create_new() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("snapshot");
        fs::write(&source, b"exact bytes").unwrap();
        let snapshot =
            snapshot_bounded_regular_file(&source, &destination, 32, "test input").unwrap();
        assert_eq!(fs::read(snapshot).unwrap(), b"exact bytes");
        assert!(snapshot_bounded_regular_file(&source, &destination, 32, "test input").is_err());
        assert!(
            snapshot_bounded_regular_file(&source, &temp.path().join("too-small"), 4, "test input")
                .is_err()
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&source, temp.path().join("source-link")).unwrap();
            assert!(
                snapshot_bounded_regular_file(
                    &temp.path().join("source-link"),
                    &temp.path().join("link-snapshot"),
                    32,
                    "test input"
                )
                .is_err()
            );
        }
    }

    #[test]
    fn provenance_statement_rejects_replayed_commit_and_coordinate() {
        let temp = tempfile::tempdir().unwrap();
        let package_name = "boxdd-prebuilt-0.6.0-x86_64-unknown-linux-gnu-single-static.tar.gz";
        let statement = prebuilt_statement(package_name, b"package");
        let path = temp.path().join(format!("{package_name}.provenance.toml"));
        fs::write(&path, statement.canonical_bytes().unwrap()).unwrap();
        let options = Options::parse(&arguments(
            "prebuilt",
            "single",
            "x86_64-unknown-linux-gnu",
            "none",
        ))
        .unwrap();
        assert!(
            read_and_validate_prebuilt_statement(&path, &options, "0.6.0", &"a".repeat(40)).is_ok()
        );
        assert!(
            read_and_validate_prebuilt_statement(&path, &options, "0.6.0", &"f".repeat(40))
                .is_err()
        );
        let other_target = Options::parse(&arguments(
            "prebuilt",
            "single",
            "x86_64-apple-darwin",
            "none",
        ))
        .unwrap();
        assert!(
            read_and_validate_prebuilt_statement(&path, &other_target, "0.6.0", &"a".repeat(40))
                .is_err()
        );
    }

    #[test]
    fn outer_package_mismatch_is_rejected_before_extraction() {
        let temp = tempfile::tempdir().unwrap();
        let package_name = "boxdd-prebuilt-0.6.0-x86_64-unknown-linux-gnu-single-static.tar.gz";
        let archive = temp.path().join(package_name);
        write_archive(&archive, &[("manifest.toml", b"manifest")]);
        let original = fs::read(&archive).unwrap();
        let statement = prebuilt_statement(package_name, &original);
        OpenOptions::new()
            .append(true)
            .open(&archive)
            .unwrap()
            .write_all(b"tampered")
            .unwrap();
        let extraction = temp.path().join("must-not-exist");
        assert!(extract_authenticated_prebuilt(&statement, &archive, &extraction).is_err());
        assert!(!extraction.exists());
    }

    #[test]
    fn prebuilt_build_environment_uses_private_statement_and_bundle_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo-home");
        fs::create_dir(&cargo_home).unwrap();
        let provenance = temp.path().join("private.provenance.toml");
        let bundle = temp.path().join("private.provenance.sigstore.json");
        let prepared = PreparedProvider {
            provider: Provider::Prebuilt,
            root: temp.path().join("provider"),
            manifest: temp.path().join("provider/manifest.toml"),
            archive: Some(temp.path().join("private.tar.gz")),
            provenance: Some(provenance.clone()),
            bundle: Some(bundle.clone()),
            cosign: Some(temp.path().join("cosign")),
            manifest_sha256: "1".repeat(64),
            archive_sha256: "2".repeat(64),
            provenance_sha256: "3".repeat(64),
            trusted_root_sha256: "4".repeat(64),
        };
        let options = Options::parse(&arguments(
            "prebuilt",
            "single",
            "x86_64-unknown-linux-gnu",
            "none",
        ))
        .unwrap();
        let environment =
            prepared.command_environment(&options, &temp.path().join("target"), &cargo_home);
        assert_eq!(
            environment
                .values
                .iter()
                .find(|(key, _)| key == OsStr::new("BOXDD_SYS_PREBUILT_PROVENANCE"))
                .map(|(_, value)| value.as_os_str()),
            Some(provenance.as_os_str())
        );
        assert_eq!(
            environment
                .values
                .iter()
                .find(|(key, _)| key == OsStr::new("BOXDD_SYS_PREBUILT_BUNDLE"))
                .map(|(_, value)| value.as_os_str()),
            Some(bundle.as_os_str())
        );
        assert!(
            environment
                .values
                .iter()
                .all(|(key, _)| { key != OsStr::new("BOXDD_SYS_PREBUILT_TRUSTED_ROOT") })
        );
    }

    #[test]
    fn archive_extraction_requires_one_normal_root_and_regular_entries() {
        let temp = tempfile::tempdir().unwrap();
        let valid = temp.path().join("valid.crate");
        write_archive(&valid, &[("boxdd-sys-0.6.0/Cargo.toml", b"[package]\n")]);
        let valid_output = temp.path().join("valid-output");
        fs::create_dir(&valid_output).unwrap();
        extract_archive(&valid, &valid_output, Some("boxdd-sys-0.6.0")).unwrap();

        let wrong_root = temp.path().join("wrong-root.crate");
        write_archive(&wrong_root, &[("checkout/Cargo.toml", b"[package]\n")]);
        let wrong_output = temp.path().join("wrong-output");
        fs::create_dir(&wrong_output).unwrap();
        assert!(extract_archive(&wrong_root, &wrong_output, Some("boxdd-sys-0.6.0")).is_err());

        for (name, entry_type) in [
            ("symlink", tar::EntryType::Symlink),
            ("hardlink", tar::EntryType::Link),
        ] {
            let archive = temp.path().join(format!("{name}.crate"));
            write_link_archive(&archive, entry_type);
            let output = temp.path().join(format!("{name}-output"));
            fs::create_dir(&output).unwrap();
            assert!(
                extract_archive(&archive, &output, Some("boxdd-sys-0.6.0")).is_err(),
                "accepted {name}"
            );
        }
        assert!(normalized_archive_path(Path::new("../checkout"), &valid).is_err());
        assert!(normalized_archive_path(Path::new("/checkout"), &valid).is_err());
        assert!(normalized_archive_path(Path::new("boxdd\\Cargo.toml"), &valid).is_err());
    }

    #[test]
    fn archive_extraction_bounds_zero_length_entries_and_decompressed_streams() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("too-many.tar.gz");
        let file = File::create(&archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for index in 0..=MAX_ARCHIVE_ENTRIES {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(0);
            header.set_cksum();
            archive
                .append_data(
                    &mut header,
                    format!("boxdd-sys-0.6.0/{index:04}.empty"),
                    std::io::empty(),
                )
                .unwrap();
        }
        archive.finish().unwrap();
        drop(archive);
        let output = temp.path().join("output");
        fs::create_dir(&output).unwrap();
        let error = extract_archive(&archive_path, &output, Some("boxdd-sys-0.6.0"))
            .expect_err("entry-count bomb must be rejected");
        assert!(error.to_string().contains("entry limit"), "{error}");

        let mut bounded = BoundedReader::new(std::io::Cursor::new([1_u8, 2, 3]), 2);
        let mut bytes = Vec::new();
        assert!(bounded.read_to_end(&mut bytes).is_err());
        assert_eq!(bytes, [1, 2]);
    }

    #[test]
    fn cargo_configuration_search_isolated_from_home_and_ancestors() {
        let root = tempfile::tempdir().unwrap();
        let working = root.path().join("qualification");
        let cargo_home = working.join("cargo-home");
        fs::create_dir_all(&cargo_home).unwrap();
        assert!(validate_cargo_configuration_isolation(&working, &cargo_home).is_ok());

        let cargo_config = root.path().join(".cargo/config.toml");
        fs::create_dir_all(cargo_config.parent().unwrap()).unwrap();
        fs::write(&cargo_config, "[target.fake]\nrunner = 'true'\n").unwrap();
        assert!(validate_cargo_configuration_isolation(&working, &cargo_home).is_err());
    }

    #[test]
    fn build_environment_excludes_runtime_attestation_and_runtime_executes_exact_binary() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo-home");
        fs::create_dir(&cargo_home).unwrap();
        let prepared = PreparedProvider {
            provider: Provider::System,
            root: temp.path().join("provider"),
            manifest: temp.path().join("provider/manifest.toml"),
            archive: None,
            provenance: None,
            bundle: None,
            cosign: None,
            manifest_sha256: "1".repeat(64),
            archive_sha256: "2".repeat(64),
            provenance_sha256: String::new(),
            trusted_root_sha256: String::new(),
        };
        let options = Options::parse(&arguments(
            "system",
            "single",
            "x86_64-unknown-linux-gnu",
            "none",
        ))
        .unwrap();
        let build =
            prepared.command_environment(&options, &temp.path().join("target"), &cargo_home);
        assert!(build.values.iter().all(|(key, _)| {
            !key.to_string_lossy()
                .starts_with("BOXDD_NATIVE_QUALIFICATION_")
        }));

        let receipt = temp.path().join("receipt");
        let runtime = prepared.runtime_environment(&cargo_home, &receipt, "nonce");
        for key in [
            "BOXDD_NATIVE_QUALIFICATION_PROVIDER",
            "BOXDD_NATIVE_QUALIFICATION_MANIFEST_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_ARCHIVE_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_PROVENANCE_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_TRUSTED_ROOT_SHA256",
            "BOXDD_NATIVE_QUALIFICATION_NONCE",
            "BOXDD_NATIVE_QUALIFICATION_RECEIPT",
        ] {
            assert!(
                runtime
                    .values
                    .iter()
                    .any(|(actual, _)| actual == OsStr::new(key))
            );
        }
        assert!(
            runtime
                .values
                .iter()
                .all(|(key, _)| !key.to_string_lossy().starts_with("BOXDD_SYS_"))
        );
        let executable = temp.path().join("qualified-consumer");
        let command = consumer_command(&executable, temp.path(), &runtime);
        assert_eq!(command.get_program(), executable.as_os_str());
    }

    #[test]
    fn cargo_messages_bind_the_exact_consumer_executable_inside_the_scratch_target() {
        let checkout = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let target = scratch.path().join("target");
        fs::create_dir(&target).unwrap();
        let executable = target.join("boxdd-native-provider-consumer");
        fs::write(&executable, b"binary").unwrap();
        let package_id = "path+file:///qualified/consumer#boxdd-native-provider-consumer@0.0.0";
        let source = serde_json::to_vec(&serde_json::json!({
            "reason": "compiler-artifact",
            "package_id": package_id,
            "target": { "name": CONSUMER_NAME, "kind": ["bin"] },
            "executable": executable,
        }))
        .unwrap();
        let resolved = consumer_executable_from_cargo_messages(
            &source,
            package_id,
            &target,
            scratch.path(),
            checkout.path(),
        )
        .unwrap();
        assert_eq!(resolved, fs::canonicalize(&executable).unwrap());

        let checkout_executable = checkout.path().join("boxdd-native-provider-consumer");
        fs::write(&checkout_executable, b"bypass").unwrap();
        let bypass = serde_json::to_vec(&serde_json::json!({
            "reason": "compiler-artifact",
            "package_id": package_id,
            "target": { "name": CONSUMER_NAME, "kind": ["bin"] },
            "executable": checkout_executable,
        }))
        .unwrap();
        assert!(
            consumer_executable_from_cargo_messages(
                &bypass,
                package_id,
                &target,
                scratch.path(),
                checkout.path(),
            )
            .is_err()
        );
    }

    #[test]
    fn dependency_path_must_resolve_to_the_extracted_crate() {
        let checkout = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let extraction = scratch.path().join("crate-extract");
        let crate_root = extraction.join("boxdd-sys-0.6.0");
        let consumer = scratch.path().join("consumer");
        fs::create_dir_all(&crate_root).unwrap();
        fs::create_dir_all(&consumer).unwrap();
        fs::write(crate_root.join("Cargo.toml"), "[package]\n").unwrap();
        let consumer_manifest = consumer.join("Cargo.toml");
        fs::write(&consumer_manifest, "[workspace]\n").unwrap();

        let valid = format!(
            "[dependencies]\nboxdd-sys = {{ path = {:?} }}\n",
            crate_root.to_str().unwrap()
        );
        assert!(
            validate_consumer_dependency(
                valid.as_bytes(),
                &consumer_manifest,
                &crate_root,
                &extraction,
                scratch.path(),
                checkout.path(),
            )
            .is_ok()
        );

        let checkout_crate = checkout.path().join("boxdd-sys");
        fs::create_dir(&checkout_crate).unwrap();
        fs::write(checkout_crate.join("Cargo.toml"), "[package]\n").unwrap();
        let escaped = format!(
            "[dependencies]\nboxdd-sys = {{ path = {:?} }}\n",
            checkout_crate.to_str().unwrap()
        );
        assert!(
            validate_consumer_dependency(
                escaped.as_bytes(),
                &consumer_manifest,
                &crate_root,
                &extraction,
                scratch.path(),
                checkout.path(),
            )
            .is_err()
        );
    }

    #[test]
    fn metadata_must_resolve_the_exact_packaged_manifest() {
        let checkout = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let extraction = scratch.path().join("crate-extract");
        let crate_root = extraction.join("boxdd-sys-0.6.0");
        let consumer_root = scratch.path().join("consumer");
        fs::create_dir_all(&crate_root).unwrap();
        fs::create_dir_all(&consumer_root).unwrap();
        let crate_manifest = crate_root.join("Cargo.toml");
        let consumer_manifest = consumer_root.join("Cargo.toml");
        fs::write(&crate_manifest, "[package]\n").unwrap();
        fs::write(&consumer_manifest, "[package]\n").unwrap();
        let valid = metadata_json(&consumer_manifest, &crate_root, &crate_manifest);
        assert!(
            validate_metadata_source(
                &valid,
                &consumer_manifest,
                &crate_manifest,
                &extraction,
                scratch.path(),
                checkout.path(),
            )
            .is_ok()
        );

        let checkout_crate = checkout.path().join("boxdd-sys");
        fs::create_dir(&checkout_crate).unwrap();
        let checkout_manifest = checkout_crate.join("Cargo.toml");
        fs::write(&checkout_manifest, "[package]\n").unwrap();
        let bypass = metadata_json(&consumer_manifest, &checkout_crate, &checkout_manifest);
        assert!(
            validate_metadata_source(
                &bypass,
                &consumer_manifest,
                &crate_manifest,
                &extraction,
                scratch.path(),
                checkout.path(),
            )
            .is_err()
        );
    }

    #[test]
    fn execution_receipt_must_be_exact_and_inside_scratch() {
        let scratch = tempfile::tempdir().unwrap();
        let receipt = scratch.path().join("receipt");
        fs::write(&receipt, "nonce").unwrap();
        assert!(validate_execution_receipt(&receipt, "nonce", scratch.path()).is_ok());
        assert!(validate_execution_receipt(&receipt, "different", scratch.path()).is_err());
        assert!(
            validate_execution_receipt(&scratch.path().join("missing"), "nonce", scratch.path())
                .is_err()
        );
    }
}
