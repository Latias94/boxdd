use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result, qualified_git::qualified_git_command};

use super::{
    provider::ProviderPrecision,
    support::{QualifiedCargo, WASM_TARGET, run_command},
};

const REGISTRY_NAME: &str = "boxdd-local";
const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";
const VERSION_PLACEHOLDER: &str = "0.0.0-BOXDD-VERSION";
const WASM_PROVIDER_PRECISION_PLACEHOLDER: &str = "BOXDD-WASM-PRECISION";
const PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        name: "boxdd-sys",
        manifest: "boxdd-sys/Cargo.toml",
    },
    PackageSpec {
        name: "boxdd",
        manifest: "boxdd/Cargo.toml",
    },
    PackageSpec {
        name: "bevy_boxdd",
        manifest: "bevy_boxdd/Cargo.toml",
    },
];

const CONSUMERS: &[ConsumerSpec] = &[
    ConsumerSpec {
        name: "sys",
        fixture: "sys",
        internal_packages: &["boxdd-sys"],
        mode: ConsumerMode::RunNative,
    },
    ConsumerSpec {
        name: "core",
        fixture: "core",
        internal_packages: &["boxdd", "boxdd-sys"],
        mode: ConsumerMode::RunNative,
    },
    ConsumerSpec {
        name: "bevy",
        fixture: "bevy",
        internal_packages: &["bevy_boxdd", "boxdd", "boxdd-sys"],
        mode: ConsumerMode::RunNative,
    },
    ConsumerSpec {
        name: "wasm-provider-single",
        fixture: "wasm-provider",
        internal_packages: &["boxdd-sys"],
        mode: ConsumerMode::CheckWasmProvider(ProviderPrecision::Single),
    },
    ConsumerSpec {
        name: "wasm-provider-double",
        fixture: "wasm-provider",
        internal_packages: &["boxdd-sys"],
        mode: ConsumerMode::CheckWasmProvider(ProviderPrecision::Double),
    },
];

#[derive(Clone, Copy)]
struct PackageSpec {
    name: &'static str,
    manifest: &'static str,
}

#[derive(Clone, Copy)]
struct ConsumerSpec {
    name: &'static str,
    fixture: &'static str,
    internal_packages: &'static [&'static str],
    mode: ConsumerMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConsumerMode {
    RunNative,
    CheckWasmProvider(ProviderPrecision),
}

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    match args {
        [] => {}
        [arg] if arg == "--check" => {}
        _ => {
            return Err(Error::message(
                "verify-packages accepts only optional --check",
            ));
        }
    }

    let version = workspace_version(root)?;
    let temporary = tempfile::Builder::new()
        .prefix("boxdd-isolated-registry-")
        .tempdir()
        .map_err(|source| Error::io("create isolated registry", source))?;
    let cargo_home = create_isolated_cargo_home(temporary.path())?;
    let temporary_root = temporary
        .path()
        .canonicalize()
        .map_err(|source| Error::io(temporary.path(), source))?;
    let cargo = QualifiedCargo::qualify_isolated(root, &cargo_home, &temporary_root)?;
    let registry_root = temporary_root.join("registry");
    let index_root = registry_root.join("index");
    let crate_root = registry_root.join("crates");
    fs::create_dir_all(&index_root).map_err(|source| Error::io(&index_root, source))?;
    fs::create_dir_all(&crate_root).map_err(|source| Error::io(&crate_root, source))?;

    let server = CrateServer::start(crate_root.clone())?;
    let index_url = file_url(&index_root)?;
    initialize_index(&index_root, &server.download_url())?;

    let package_target = temporary_root.join("package-target");
    for package in PACKAGES {
        let archive = package_crate(
            root,
            &temporary_root,
            &package_target,
            package,
            &version,
            &index_url,
            &cargo,
        )?;
        index_package(
            &index_root,
            &crate_root,
            &index_url,
            package,
            &version,
            &archive,
        )?;
    }

    for consumer in CONSUMERS {
        consume_fixture(
            root,
            &temporary_root,
            &index_url,
            &version,
            &cargo,
            *consumer,
        )?;
    }

    println!(
        "isolated registry verified {} packages in publish order and {} fixed consumers",
        PACKAGES.len(),
        CONSUMERS.len()
    );
    Ok(())
}

fn create_isolated_cargo_home(temporary: &Path) -> Result<PathBuf> {
    let root = fs::canonicalize(temporary).map_err(|source| Error::io(temporary, source))?;
    let cargo_home = temporary.join("cargo-home");
    fs::create_dir(&cargo_home).map_err(|source| Error::io(&cargo_home, source))?;
    let metadata =
        fs::symlink_metadata(&cargo_home).map_err(|source| Error::io(&cargo_home, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "isolated Cargo home must be a real directory: {}",
            cargo_home.display()
        )));
    }
    let canonical =
        fs::canonicalize(&cargo_home).map_err(|source| Error::io(&cargo_home, source))?;
    if canonical.parent() != Some(root.as_path()) {
        return Err(Error::message(format!(
            "isolated Cargo home escaped its temporary root: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn workspace_version(root: &Path) -> Result<String> {
    let path = root.join("Cargo.toml");
    let source = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let manifest: toml::Value = toml::from_str(&source)
        .map_err(|error| Error::message(format!("{} is invalid TOML: {error}", path.display())))?;
    manifest
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::message("workspace.package.version is missing"))
}

fn package_crate(
    root: &Path,
    temporary: &Path,
    target: &Path,
    package: &PackageSpec,
    version: &str,
    index_url: &str,
    cargo: &QualifiedCargo,
) -> Result<PathBuf> {
    let manifest = root.join(package.manifest);
    if !manifest.is_file() {
        return Err(Error::message(format!(
            "publishable package manifest is missing: {}",
            manifest.display()
        )));
    }
    let staging = stage_package_workspace(root, temporary, package, version)?;
    let mut command = package_cargo_command(&staging, target, package.name, index_url, cargo)?;
    run_command(
        &mut command,
        &format!("package {} from repository source", package.name),
    )?;

    let archive = target
        .join("package")
        .join(format!("{}-{version}.crate", package.name));
    if archive.is_file() {
        validate_package_archive(root, &archive, package.name, version)?;
        Ok(archive)
    } else {
        Err(Error::message(format!(
            "cargo package did not produce {}",
            archive.display()
        )))
    }
}

fn package_cargo_command(
    staging: &Path,
    target: &Path,
    package: &str,
    index_url: &str,
    cargo: &QualifiedCargo,
) -> Result<Command> {
    let mut command = cargo.command_in(staging, target)?;
    add_loopback_proxy_bypass(&mut command);
    command.args([
        "package",
        "--allow-dirty",
        "--no-verify",
        "--registry",
        REGISTRY_NAME,
    ]);
    command
        .args(["-p", package])
        .env("CARGO_REGISTRIES_BOXDD_LOCAL_INDEX", index_url);
    Ok(command)
}

fn validate_package_archive(
    repository_root: &Path,
    archive: &Path,
    package: &str,
    version: &str,
) -> Result<()> {
    let prefix = format!("{package}-{version}/");
    let file = fs::File::open(archive).map_err(|error| Error::io(archive, error))?;
    let mut tar = tar::Archive::new(GzDecoder::new(file));
    let entries = tar
        .entries()
        .map_err(|error| Error::message(format!("read {}: {error}", archive.display())))?;
    let mut paths = BTreeMap::new();
    for entry in entries {
        let mut entry = entry.map_err(|error| {
            Error::message(format!("read {} entry: {error}", archive.display()))
        })?;
        if !entry.header().entry_type().is_file() {
            return Err(Error::message(format!(
                "{} contains a non-regular package entry",
                archive.display()
            )));
        }
        let path = entry
            .path()
            .map_err(|error| Error::message(format!("read package entry path: {error}")))?
            .to_string_lossy()
            .into_owned();
        if !path.starts_with(&prefix) || path.contains("../") || path.contains('\\') {
            return Err(Error::message(format!(
                "{} contains an unsafe package path {path:?}",
                archive.display()
            )));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| Error::message(format!("read package entry {path}: {error}")))?;
        if paths.insert(path.clone(), bytes).is_some() {
            return Err(Error::message(format!(
                "{} contains duplicate package entry {path:?}",
                archive.display()
            )));
        }
    }
    let mut required = BTreeMap::from([
        (
            format!("{prefix}LICENSE-MIT"),
            repository_root.join("LICENSE-MIT"),
        ),
        (
            format!("{prefix}LICENSE-APACHE"),
            repository_root.join("LICENSE-APACHE"),
        ),
    ]);
    if package == "boxdd-sys" {
        required.extend([
            (
                format!("{prefix}third-party/box2d/LICENSE"),
                repository_root.join("boxdd-sys/third-party/box2d/LICENSE"),
            ),
            (
                format!("{prefix}security/sigstore/trusted_root.json"),
                repository_root.join("boxdd-sys/security/sigstore/trusted_root.json"),
            ),
            (
                format!("{prefix}abi/wasm32-unknown-unknown-single.toml"),
                repository_root.join("boxdd-sys/abi/wasm32-unknown-unknown-single.toml"),
            ),
            (
                format!("{prefix}abi/wasm32-unknown-unknown-double.toml"),
                repository_root.join("boxdd-sys/abi/wasm32-unknown-unknown-double.toml"),
            ),
        ]);
    }
    for (path, source) in &required {
        let expected = fs::read(source).map_err(|error| Error::io(source, error))?;
        if paths.get(path).map(Vec::as_slice) != Some(expected.as_slice()) {
            return Err(Error::message(format!(
                "{} is missing or changed required package asset {path}",
                archive.display(),
            )));
        }
    }
    let actual_licenses = paths
        .keys()
        .filter(|path| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("LICENSE"))
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_licenses = required
        .keys()
        .filter(|path| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("LICENSE"))
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_licenses != expected_licenses {
        return Err(Error::message(format!(
            "{} has an unexpected license set: {actual_licenses:?}",
            archive.display()
        )));
    }
    Ok(())
}

fn stage_package_workspace(
    root: &Path,
    temporary: &Path,
    package: &PackageSpec,
    version: &str,
) -> Result<PathBuf> {
    let staging = temporary.join("package-sources").join(package.name);
    fs::create_dir_all(&staging).map_err(|error| Error::io(&staging, error))?;
    let source_dir = root.join(
        Path::new(package.manifest)
            .parent()
            .ok_or_else(|| Error::message("package manifest has no parent"))?,
    );
    let package_dir = staging.join(
        source_dir
            .file_name()
            .ok_or_else(|| Error::message("package directory has no name"))?,
    );
    copy_tree(&source_dir, &package_dir)?;

    let readme = root.join("README.md");
    if readme.is_file() {
        let destination = staging.join("README.md");
        fs::copy(&readme, &destination).map_err(|error| Error::io(&destination, error))?;
    }

    let root_manifest = root.join("Cargo.toml");
    let source =
        fs::read_to_string(&root_manifest).map_err(|error| Error::io(&root_manifest, error))?;
    let mut manifest: toml::Value = toml::from_str(&source).map_err(|error| {
        Error::message(format!(
            "{} is invalid TOML: {error}",
            root_manifest.display()
        ))
    })?;
    let workspace = manifest
        .get_mut("workspace")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| Error::message("root manifest has no workspace table"))?;
    workspace.insert(
        "members".to_owned(),
        toml::Value::Array(vec![toml::Value::String(
            package_dir
                .file_name()
                .expect("staged package directory has a name")
                .to_string_lossy()
                .into_owned(),
        )]),
    );
    let dependencies = workspace
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| Error::message("root manifest has no workspace.dependencies table"))?;
    for internal in PACKAGES {
        dependencies.insert(
            internal.name.to_owned(),
            toml::Value::Table(toml::Table::from_iter([
                (
                    "version".to_owned(),
                    toml::Value::String(version.to_owned()),
                ),
                (
                    "registry".to_owned(),
                    toml::Value::String(REGISTRY_NAME.to_owned()),
                ),
            ])),
        );
    }
    let rendered = toml::to_string(&manifest)
        .map_err(|error| Error::message(format!("render staged workspace: {error}")))?;
    write_file(&staging.join("Cargo.toml"), rendered.as_bytes())?;
    Ok(staging)
}

fn initialize_index(index_root: &Path, download_url: &str) -> Result<()> {
    write_file(
        &index_root.join("config.json"),
        format!("{{\"dl\":{download_url:?},\"auth-required\":false}}\n").as_bytes(),
    )?;
    run_git(
        index_root,
        &["init", "--quiet"],
        "initialize local registry index",
    )?;
    run_git(index_root, &["add", "config.json"], "stage registry config")?;
    commit_index(index_root, "initialize isolated registry")
}

fn index_package(
    index_root: &Path,
    crate_root: &Path,
    index_url: &str,
    package: &PackageSpec,
    version: &str,
    archive: &Path,
) -> Result<()> {
    let archive_bytes = fs::read(archive).map_err(|source| Error::io(archive, source))?;
    let checksum = hex_digest(Sha256::digest(&archive_bytes));
    let destination = crate_root.join(package.name).join(version).join("download");
    write_file(&destination, &archive_bytes)?;

    let normalized = normalized_manifest(&archive_bytes, package.name, version)?;
    if normalized.package.name != package.name || normalized.package.version != version {
        return Err(Error::message(format!(
            "packaged manifest identity mismatch: expected {} {version}, found {} {}",
            package.name, normalized.package.name, normalized.package.version
        )));
    }
    let record = IndexRecord::from_manifest(normalized, checksum, index_url)?;
    let entry = index_root.join(index_relative_path(package.name));
    let rendered = serde_json::to_string(&record)
        .map_err(|error| Error::message(format!("serialize registry index entry: {error}")))?;
    write_file(&entry, format!("{rendered}\n").as_bytes())?;

    let relative = entry
        .strip_prefix(index_root)
        .map_err(|_| Error::message("registry index entry escaped index root"))?;
    run_git(
        index_root,
        &["add", relative.to_string_lossy().as_ref()],
        "stage package index entry",
    )?;
    commit_index(index_root, &format!("index {} {version}", package.name))
}

fn consume_fixture(
    root: &Path,
    temporary: &Path,
    index_url: &str,
    version: &str,
    cargo: &QualifiedCargo,
    consumer: ConsumerSpec,
) -> Result<()> {
    let source = root
        .join("xtask/fixtures/package-consumers")
        .join(consumer.fixture);
    let destination = temporary.join("consumers").join(consumer.name);
    copy_tree(&source, &destination)?;
    let manifest = destination.join("Cargo.toml");
    let manifest_source =
        fs::read_to_string(&manifest).map_err(|error| Error::io(&manifest, error))?;
    let rendered = render_consumer_manifest(&manifest_source, version, consumer)?;
    write_file(&manifest, rendered.as_bytes())?;

    let target = temporary.join("consumer-targets").join(consumer.name);
    let mut generate = consumer_lockfile_command(&destination, &target, index_url, cargo)?;
    run_command(
        &mut generate,
        &format!(
            "resolve {} consumer through isolated registry",
            consumer.name
        ),
    )?;
    assert_internal_sources(
        &destination.join("Cargo.lock"),
        index_url,
        version,
        consumer.internal_packages,
    )?;

    let mut verify =
        consumer_verify_command(&destination, &target, index_url, cargo, consumer.mode)?;
    run_command(
        &mut verify,
        &format!("verify fixed {} package consumer", consumer.name),
    )
}

fn consumer_lockfile_command(
    destination: &Path,
    target: &Path,
    index_url: &str,
    cargo: &QualifiedCargo,
) -> Result<Command> {
    let mut command = cargo.command_in(destination, target)?;
    add_loopback_proxy_bypass(&mut command);
    command
        .env("CARGO_REGISTRIES_BOXDD_LOCAL_INDEX", index_url)
        .args(["generate-lockfile"]);
    Ok(command)
}

fn render_consumer_manifest(source: &str, version: &str, consumer: ConsumerSpec) -> Result<String> {
    if !source.contains(VERSION_PLACEHOLDER) {
        return Err(Error::message(format!(
            "package consumer {} has no version placeholder",
            consumer.name
        )));
    }
    let mut rendered = source.replace(VERSION_PLACEHOLDER, version);
    match consumer.mode {
        ConsumerMode::RunNative => {
            if rendered.contains(WASM_PROVIDER_PRECISION_PLACEHOLDER) {
                return Err(Error::message(format!(
                    "native package consumer {} contains a WASM precision placeholder",
                    consumer.name
                )));
            }
        }
        ConsumerMode::CheckWasmProvider(precision) => {
            if !rendered.contains(WASM_PROVIDER_PRECISION_PLACEHOLDER) {
                return Err(Error::message(format!(
                    "WASM provider consumer {} has no precision placeholder",
                    consumer.name
                )));
            }
            rendered = rendered.replace(WASM_PROVIDER_PRECISION_PLACEHOLDER, precision.as_str());
        }
    }
    Ok(rendered)
}

fn consumer_verify_command(
    destination: &Path,
    target: &Path,
    index_url: &str,
    cargo: &QualifiedCargo,
    mode: ConsumerMode,
) -> Result<Command> {
    let mut command = cargo.command_in(destination, target)?;
    add_loopback_proxy_bypass(&mut command);
    command.env("CARGO_REGISTRIES_BOXDD_LOCAL_INDEX", index_url);
    match mode {
        ConsumerMode::RunNative => {
            command.args(["run", "--locked"]);
        }
        ConsumerMode::CheckWasmProvider(precision) => {
            command
                .args([
                    "check",
                    "--locked",
                    "--target",
                    WASM_TARGET,
                    "--no-default-features",
                ])
                .env("BOXDD_SYS_PROVIDER", "wasm-provider");
            if let Some(feature) = precision.cargo_feature() {
                command.args(["--features", feature]);
            }
        }
    }
    Ok(command)
}

fn add_loopback_proxy_bypass(command: &mut Command) {
    let keys: &[&str] = if cfg!(windows) {
        &["NO_PROXY"]
    } else {
        &["NO_PROXY", "no_proxy"]
    };
    for key in keys {
        let explicit = command.get_envs().find(|(name, _)| {
            #[cfg(windows)]
            {
                name.to_string_lossy().eq_ignore_ascii_case(key)
            }
            #[cfg(not(windows))]
            {
                *name == std::ffi::OsStr::new(key)
            }
        });
        let mut value = match explicit {
            Some((_, Some(value))) => value.to_owned(),
            Some((_, None)) => Default::default(),
            None => std::env::var_os(key).unwrap_or_default(),
        };
        if !value.is_empty() {
            value.push(",");
        }
        value.push("127.0.0.1,localhost,::1");
        command.env(key, value);
    }
}

fn assert_internal_sources(
    lockfile: &Path,
    index_url: &str,
    version: &str,
    packages: &[&str],
) -> Result<()> {
    let source = fs::read_to_string(lockfile).map_err(|error| Error::io(lockfile, error))?;
    let lock: toml::Value = toml::from_str(&source)
        .map_err(|error| Error::message(format!("{} is invalid: {error}", lockfile.display())))?;
    let entries = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| Error::message("consumer Cargo.lock has no package entries"))?;
    for package in packages {
        let entry = entries.iter().find(|entry| {
            entry.get("name").and_then(toml::Value::as_str) == Some(*package)
                && entry.get("version").and_then(toml::Value::as_str) == Some(version)
        });
        let entry = entry.ok_or_else(|| {
            Error::message(format!(
                "consumer did not resolve required local package {package} {version}"
            ))
        })?;
        let source = entry
            .get("source")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        if source != format!("registry+{index_url}") {
            return Err(Error::message(format!(
                "consumer resolved {package} from {source:?}, expected isolated registry {index_url}"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct NormalizedManifest {
    package: NormalizedPackage,
    #[serde(default)]
    features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    dependencies: BTreeMap<String, DependencyValue>,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, DependencyValue>,
    #[serde(default, rename = "build-dependencies")]
    build_dependencies: BTreeMap<String, DependencyValue>,
    #[serde(default)]
    target: BTreeMap<String, TargetDependencies>,
}

#[derive(Debug, Deserialize)]
struct NormalizedPackage {
    name: String,
    version: String,
    links: Option<String>,
    #[serde(rename = "rust-version")]
    rust_version: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct TargetDependencies {
    #[serde(default)]
    dependencies: BTreeMap<String, DependencyValue>,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, DependencyValue>,
    #[serde(default, rename = "build-dependencies")]
    build_dependencies: BTreeMap<String, DependencyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependencyValue {
    Version(String),
    Detailed(DependencyDetail),
}

#[derive(Debug, Deserialize, Default)]
struct DependencyDetail {
    version: Option<String>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    optional: bool,
    #[serde(default = "default_true", rename = "default-features")]
    default_features: bool,
    package: Option<String>,
    registry: Option<String>,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct IndexRecord {
    name: String,
    vers: String,
    deps: Vec<IndexDependency>,
    cksum: String,
    features: BTreeMap<String, Vec<String>>,
    features2: BTreeMap<String, Vec<String>>,
    yanked: bool,
    links: Option<String>,
    rust_version: Option<String>,
    v: u8,
}

#[derive(Debug, Serialize)]
struct IndexDependency {
    name: String,
    req: String,
    features: Vec<String>,
    optional: bool,
    default_features: bool,
    target: Option<String>,
    kind: String,
    registry: Option<String>,
    package: Option<String>,
}

impl IndexRecord {
    fn from_manifest(
        manifest: NormalizedManifest,
        checksum: String,
        index_url: &str,
    ) -> Result<Self> {
        let mut dependencies = Vec::new();
        extend_dependencies(
            &mut dependencies,
            manifest.dependencies,
            "normal",
            None,
            index_url,
        )?;
        extend_dependencies(
            &mut dependencies,
            manifest.dev_dependencies,
            "dev",
            None,
            index_url,
        )?;
        extend_dependencies(
            &mut dependencies,
            manifest.build_dependencies,
            "build",
            None,
            index_url,
        )?;
        for (target, table) in manifest.target {
            extend_dependencies(
                &mut dependencies,
                table.dependencies,
                "normal",
                Some(&target),
                index_url,
            )?;
            extend_dependencies(
                &mut dependencies,
                table.dev_dependencies,
                "dev",
                Some(&target),
                index_url,
            )?;
            extend_dependencies(
                &mut dependencies,
                table.build_dependencies,
                "build",
                Some(&target),
                index_url,
            )?;
        }
        dependencies.sort_by(|left, right| {
            (&left.name, &left.kind, &left.target).cmp(&(&right.name, &right.kind, &right.target))
        });

        let mut features = BTreeMap::new();
        let mut features2 = BTreeMap::new();
        for (name, values) in manifest.features {
            if values.iter().any(|value| {
                value.starts_with("dep:") || value.contains("?/") || value.ends_with('?')
            }) {
                features2.insert(name, values);
            } else {
                features.insert(name, values);
            }
        }

        Ok(Self {
            name: manifest.package.name,
            vers: manifest.package.version,
            deps: dependencies,
            cksum: checksum,
            features,
            features2,
            yanked: false,
            links: manifest.package.links,
            rust_version: manifest.package.rust_version,
            v: 2,
        })
    }
}

fn extend_dependencies(
    output: &mut Vec<IndexDependency>,
    dependencies: BTreeMap<String, DependencyValue>,
    kind: &str,
    target: Option<&str>,
    index_url: &str,
) -> Result<()> {
    for (name, dependency) in dependencies {
        let (request, features, optional, default_features, package, explicit_registry) =
            match dependency {
                DependencyValue::Version(version) => (version, Vec::new(), false, true, None, None),
                DependencyValue::Detailed(detail) => (
                    detail.version.ok_or_else(|| {
                        Error::message(format!(
                            "normalized dependency {name} has no version requirement"
                        ))
                    })?,
                    detail.features,
                    detail.optional,
                    detail.default_features,
                    detail.package,
                    detail.registry,
                ),
            };
        let actual_name = package.as_deref().unwrap_or(&name);
        let registry = if is_internal_package(actual_name) {
            Some(index_url.to_owned())
        } else {
            explicit_registry.or_else(|| Some(CRATES_IO_INDEX.to_owned()))
        };
        output.push(IndexDependency {
            name,
            req: request,
            features,
            optional,
            default_features,
            target: target.map(ToOwned::to_owned),
            kind: kind.to_owned(),
            registry,
            package,
        });
    }
    Ok(())
}

fn is_internal_package(name: &str) -> bool {
    PACKAGES.iter().any(|package| package.name == name)
}

fn normalized_manifest(bytes: &[u8], name: &str, version: &str) -> Result<NormalizedManifest> {
    let decoder = GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    let expected = format!("{name}-{version}/Cargo.toml");
    let entries = archive
        .entries()
        .map_err(|error| Error::message(format!("read .crate archive: {error}")))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| Error::message(format!("read .crate archive entry: {error}")))?;
        let path = entry
            .path()
            .map_err(|error| Error::message(format!("read .crate entry path: {error}")))?;
        if path == Path::new(&expected) {
            let mut source = String::new();
            entry
                .read_to_string(&mut source)
                .map_err(|error| Error::message(format!("read normalized Cargo.toml: {error}")))?;
            return toml::from_str(&source).map_err(|error| {
                Error::message(format!(
                    "packaged normalized Cargo.toml is invalid: {error}"
                ))
            });
        }
    }
    Err(Error::message(format!(
        ".crate archive is missing normalized manifest {expected}"
    )))
}

fn index_relative_path(name: &str) -> PathBuf {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => PathBuf::from("1").join(name),
        2 => PathBuf::from("2").join(name),
        3 => PathBuf::from("3").join(&name[..1]).join(name),
        _ => PathBuf::from(&name[..2]).join(&name[2..4]).join(name),
    }
}

fn file_url(path: &Path) -> Result<String> {
    let canonical = fs::canonicalize(path).map_err(|error| Error::io(path, error))?;
    let rendered = canonical.to_string_lossy();
    if rendered.contains(['\n', '\r', '#', '?']) {
        return Err(Error::message(format!(
            "isolated registry path cannot be represented as a Cargo file URL: {}",
            canonical.display()
        )));
    }
    #[cfg(windows)]
    return Ok(format!("file:///{}", rendered.replace('\\', "/")));
    #[cfg(not(windows))]
    Ok(format!("file://{rendered}"))
}

fn run_git(root: &Path, args: &[&str], label: &str) -> Result<()> {
    let mut command = qualified_git_command().map_err(Error::Message)?;
    command.current_dir(root).args(args);
    run_command(&mut command, label)
}

fn commit_index(root: &Path, message: &str) -> Result<()> {
    let mut command = qualified_git_command().map_err(Error::Message)?;
    command
        .current_dir(root)
        .args(["-c", "user.name=boxdd verification"])
        .args(["-c", "user.email=verification@invalid"])
        .args(["commit", "--quiet", "-m", message]);
    run_command(&mut command, "commit isolated registry index transaction")
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
    }
    fs::write(path, bytes).map_err(|error| Error::io(path, error))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_dir() {
        return Err(Error::message(format!(
            "fixed package consumer fixture is missing: {}",
            source.display()
        )));
    }
    fs::create_dir_all(destination).map_err(|error| Error::io(destination, error))?;
    let entries = fs::read_dir(source).map_err(|error| Error::io(source, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(source, error))?;
        let kind = entry
            .file_type()
            .map_err(|error| Error::io(entry.path(), error))?;
        if kind.is_symlink() {
            return Err(Error::message(format!(
                "package consumer fixtures cannot contain symlinks: {}",
                entry.path().display()
            )));
        }
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| Error::io(&target, error))?;
        }
    }
    Ok(())
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct CrateServer {
    address: std::net::SocketAddr,
    stopping: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl CrateServer {
    fn start(root: PathBuf) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| Error::io("bind isolated registry HTTP server", error))?;
        let address = listener
            .local_addr()
            .map_err(|error| Error::io("inspect isolated registry HTTP server", error))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| Error::io("configure isolated registry HTTP server", error))?;
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let thread = thread::Builder::new()
            .name("boxdd-local-registry".to_owned())
            .spawn(move || {
                while !worker_stopping.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => serve_crate(stream, &root),
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| Error::io("spawn isolated registry HTTP server", error))?;
        Ok(Self {
            address,
            stopping,
            thread: Some(thread),
        })
    }

    fn download_url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for CrateServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve_crate(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Some(request) = read_http_request(&mut stream) else {
        let _ = write_response(&mut stream, "400 Bad Request", b"");
        return;
    };
    let request = String::from_utf8_lossy(&request);
    let Some(line) = request.lines().next() else {
        return;
    };
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        let _ = write_response(&mut stream, "405 Method Not Allowed", b"");
        return;
    }
    let Some(path) = parts.next() else {
        let _ = write_response(&mut stream, "400 Bad Request", b"");
        return;
    };
    let segments = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() != 3
        || segments[2] != "download"
        || segments[..2].iter().any(|segment| {
            segment.is_empty()
                || matches!(*segment, "." | "..")
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
    {
        let _ = write_response(&mut stream, "404 Not Found", b"");
        return;
    }
    let Ok(canonical_root) = fs::canonicalize(root) else {
        let _ = write_response(&mut stream, "404 Not Found", b"");
        return;
    };
    let candidate = root.join(segments[0]).join(segments[1]).join("download");
    let file = fs::canonicalize(&candidate).ok().filter(|file| {
        file.starts_with(&canonical_root)
            && fs::symlink_metadata(file)
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
    });
    match file.and_then(|file| fs::read(file).ok()) {
        Some(bytes) => {
            let _ = write_response(&mut stream, "200 OK", &bytes);
        }
        None => {
            let _ = write_response(&mut stream, "404 Not Found", b"");
        }
    }
}

fn read_http_request(stream: &mut TcpStream) -> Option<Vec<u8>> {
    const MAX_REQUEST_BYTES: usize = 8192;
    let mut request = Vec::with_capacity(512);
    let mut chunk = [0_u8; 512];
    while request.len() < MAX_REQUEST_BYTES {
        let remaining = MAX_REQUEST_BYTES - request.len();
        let read_len = remaining.min(chunk.len());
        let length = stream.read(&mut chunk[..read_len]).ok()?;
        if length == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..length]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Some(request);
        }
    }
    None
}

fn write_response(stream: &mut TcpStream, status: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, ffi::OsStr};
    use syn::{
        Expr, Item, Lit, Token,
        parse::Parser,
        punctuated::Punctuated,
        visit::{self, Visit},
    };

    fn isolated_test_cargo() -> (tempfile::TempDir, PathBuf, PathBuf, QualifiedCargo) {
        let temporary = tempfile::tempdir().unwrap();
        let cargo_home = create_isolated_cargo_home(temporary.path()).unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let cargo = QualifiedCargo::qualify_isolated(repository, &cargo_home, &root).unwrap();
        (temporary, root, cargo_home, cargo)
    }

    fn declared_adapter_abi_version(source: &str) -> u32 {
        let syntax = syn::parse_file(source).expect("adapter source must parse");
        syntax
            .items
            .into_iter()
            .find_map(|item| {
                let Item::Const(item) = item else {
                    return None;
                };
                if item.ident != "ADAPTER_ABI_VERSION" {
                    return None;
                }
                let Expr::Lit(expression) = *item.expr else {
                    panic!("adapter ABI version must be an integer literal");
                };
                let Lit::Int(value) = expression.lit else {
                    panic!("adapter ABI version must be an integer literal");
                };
                Some(
                    value
                        .base10_parse()
                        .expect("adapter ABI version must fit in u32"),
                )
            })
            .expect("adapter source must declare ADAPTER_ABI_VERSION")
    }

    #[derive(Default)]
    struct AdapterAbiAssertion {
        version: Option<u32>,
    }

    impl<'ast> Visit<'ast> for AdapterAbiAssertion {
        fn visit_macro(&mut self, macro_call: &'ast syn::Macro) {
            if macro_call.path.is_ident("assert_eq") {
                let arguments = Punctuated::<Expr, Token![,]>::parse_terminated
                    .parse2(macro_call.tokens.clone())
                    .expect("package consumer assert_eq! arguments must parse");
                let mut arguments = arguments.iter();
                if let (Some(Expr::Path(actual)), Some(Expr::Lit(expected))) =
                    (arguments.next(), arguments.next())
                {
                    let expected_path = ["boxdd_sys", "adapter", "ADAPTER_ABI_VERSION"];
                    let is_adapter_abi = actual.path.segments.len() == expected_path.len()
                        && actual
                            .path
                            .segments
                            .iter()
                            .zip(expected_path)
                            .all(|(segment, expected)| segment.ident == expected);
                    if is_adapter_abi {
                        let Lit::Int(value) = &expected.lit else {
                            panic!("package consumer adapter ABI assertion must use an integer");
                        };
                        self.version = Some(
                            value
                                .base10_parse()
                                .expect("package consumer adapter ABI must fit in u32"),
                        );
                    }
                }
            }
            visit::visit_macro(self, macro_call);
        }
    }

    fn asserted_adapter_abi_version(source: &str) -> u32 {
        let syntax = syn::parse_file(source).expect("sys package consumer source must parse");
        let mut assertion = AdapterAbiAssertion::default();
        assertion.visit_file(&syntax);
        assertion
            .version
            .expect("sys package consumer must assert the adapter ABI version")
    }

    #[test]
    fn registry_paths_follow_cargo_index_layout() {
        assert_eq!(index_relative_path("a"), PathBuf::from("1/a"));
        assert_eq!(index_relative_path("ab"), PathBuf::from("2/ab"));
        assert_eq!(index_relative_path("abc"), PathBuf::from("3/a/abc"));
        assert_eq!(
            index_relative_path("boxdd-sys"),
            PathBuf::from("bo/xd/boxdd-sys")
        );
    }

    #[test]
    fn publish_order_places_every_internal_dependency_before_its_consumer() {
        let positions = PACKAGES
            .iter()
            .enumerate()
            .map(|(index, package)| (package.name, index))
            .collect::<BTreeMap<_, _>>();
        assert!(positions["boxdd-sys"] < positions["boxdd"]);
        assert!(positions["boxdd"] < positions["bevy_boxdd"]);
        assert_eq!(positions.len(), 3);
    }

    #[test]
    fn local_registry_server_rejects_traversal_and_serves_exact_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("crates");
        write_file(&root.join("boxdd/0.6.0/download"), b"crate-bytes").unwrap();
        write_file(&temp.path().join("Cargo.toml/download"), b"outside-secret").unwrap();
        let server = CrateServer::start(root).unwrap();

        let mut stream = TcpStream::connect(server.address).unwrap();
        stream.write_all(b"GET ").unwrap();
        thread::sleep(Duration::from_millis(20));
        stream
            .write_all(b"/boxdd/0.6.0/download HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        assert!(response.ends_with(b"crate-bytes"));

        let mut traversal = TcpStream::connect(server.address).unwrap();
        traversal
            .write_all(b"GET /../Cargo.toml/download HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        traversal.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 404"));
        assert!(!response.contains("outside-secret"));
    }

    #[test]
    fn internal_package_set_is_exact() {
        let names = PACKAGES
            .iter()
            .map(|package| package.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names, BTreeSet::from(["bevy_boxdd", "boxdd", "boxdd-sys"]));
    }

    #[test]
    fn isolated_cargo_home_starts_empty_under_the_temporary_root() {
        let temporary = tempfile::tempdir().unwrap();
        let cargo_home = create_isolated_cargo_home(temporary.path()).unwrap();
        let root = temporary.path().canonicalize().unwrap();

        assert_eq!(cargo_home.parent(), Some(root.as_path()));
        assert_eq!(fs::read_dir(&cargo_home).unwrap().count(), 0);
        assert!(create_isolated_cargo_home(temporary.path()).is_err());
    }

    #[test]
    fn package_and_consumer_commands_share_isolated_cargo_home() {
        let (_temporary, root, cargo_home, cargo) = isolated_test_cargo();
        let staging = root.join("staging");
        let consumer_root = root.join("consumer");
        fs::create_dir_all(&staging).unwrap();
        fs::create_dir_all(&consumer_root).unwrap();
        let package_target = root.join("targets/package");
        let consumer_target = root.join("targets/consumer");
        let index_url = "file:///isolated-index";
        let package =
            package_cargo_command(&staging, &package_target, "boxdd-sys", index_url, &cargo)
                .unwrap();
        let lockfile =
            consumer_lockfile_command(&consumer_root, &consumer_target, index_url, &cargo).unwrap();
        let consumer = consumer_verify_command(
            &consumer_root,
            &consumer_target,
            index_url,
            &cargo,
            ConsumerMode::RunNative,
        )
        .unwrap();

        for command in [&package, &lockfile, &consumer] {
            assert!(Path::new(command.get_program()).is_absolute());
            let environment = command
                .get_envs()
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(
                environment.get(OsStr::new("CARGO_HOME")).copied().flatten(),
                Some(cargo_home.as_os_str()),
            );
            assert_eq!(environment.get(OsStr::new("DOCS_RS")).copied(), Some(None));
            assert_eq!(
                environment.get(OsStr::new("RUSTFLAGS")).copied(),
                Some(None)
            );
            assert_eq!(
                environment
                    .get(OsStr::new("CARGO_NET_GIT_FETCH_WITH_CLI"))
                    .copied(),
                Some(None)
            );
            assert_eq!(
                environment
                    .get(OsStr::new("CARGO_REGISTRIES_BOXDD_LOCAL_INDEX"))
                    .copied()
                    .flatten(),
                Some(OsStr::new(index_url))
            );
            let rustc = environment
                .get(OsStr::new("RUSTC"))
                .copied()
                .flatten()
                .expect("qualified RUSTC");
            assert!(Path::new(rustc).is_absolute());
            let target = Path::new(
                environment
                    .get(OsStr::new("CARGO_TARGET_DIR"))
                    .copied()
                    .flatten()
                    .expect("controlled Cargo target"),
            );
            assert!(target.is_absolute());
            assert!(target.starts_with(&root));
            assert_ne!(target, root.as_path());
        }
    }

    #[test]
    fn package_commands_preserve_proxies_and_bypass_the_loopback_registry() {
        let (_temporary, root, _cargo_home, cargo) = isolated_test_cargo();
        let staging = root.join("staging");
        fs::create_dir(&staging).unwrap();
        let mut command = cargo.command_in(&staging, &root.join("target")).unwrap();
        command
            .env("CARGO_HTTP_PROXY", "http://127.0.0.1:10809")
            .env("NO_PROXY", "example.invalid");
        #[cfg(not(windows))]
        command.env("no_proxy", "internal.invalid");

        add_loopback_proxy_bypass(&mut command);

        let environment = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            environment
                .get(OsStr::new("CARGO_HTTP_PROXY"))
                .copied()
                .flatten(),
            Some(OsStr::new("http://127.0.0.1:10809"))
        );
        assert_eq!(
            environment.get(OsStr::new("NO_PROXY")).copied().flatten(),
            Some(OsStr::new("example.invalid,127.0.0.1,localhost,::1"))
        );
        #[cfg(not(windows))]
        assert_eq!(
            environment.get(OsStr::new("no_proxy")).copied().flatten(),
            Some(OsStr::new("internal.invalid,127.0.0.1,localhost,::1"))
        );
    }

    #[test]
    fn wasm_provider_consumers_route_distinct_single_and_double_coordinates() {
        let single = *CONSUMERS
            .iter()
            .find(|consumer| {
                consumer.mode == ConsumerMode::CheckWasmProvider(ProviderPrecision::Single)
            })
            .expect("single-precision WASM provider consumer");
        let double = *CONSUMERS
            .iter()
            .find(|consumer| {
                consumer.mode == ConsumerMode::CheckWasmProvider(ProviderPrecision::Double)
            })
            .expect("double-precision WASM provider consumer");
        assert_ne!(single.name, double.name);
        assert_eq!(single.fixture, "wasm-provider");
        assert_eq!(double.fixture, "wasm-provider");

        let manifest_source =
            include_str!("../../fixtures/package-consumers/wasm-provider/Cargo.toml");
        let single_manifest = render_consumer_manifest(manifest_source, "0.6.0", single).unwrap();
        let double_manifest = render_consumer_manifest(manifest_source, "0.6.0", double).unwrap();
        assert!(single_manifest.contains("name = \"boxdd-package-consumer-wasm-provider-single\""));
        assert!(double_manifest.contains("name = \"boxdd-package-consumer-wasm-provider-double\""));
        assert!(single_manifest.contains("double-precision = [\"boxdd-sys/double-precision\"]"));
        assert!(double_manifest.contains("double-precision = [\"boxdd-sys/double-precision\"]"));
        assert!(!single_manifest.contains(WASM_PROVIDER_PRECISION_PLACEHOLDER));
        assert!(!double_manifest.contains(WASM_PROVIDER_PRECISION_PLACEHOLDER));

        let (_temporary, root, cargo_home, cargo) = isolated_test_cargo();
        let destination = root.join("consumer");
        fs::create_dir(&destination).unwrap();
        let single_target = root.join("targets/wasm-provider-single");
        let double_target = root.join("targets/wasm-provider-double");
        let index_url = "file:///isolated-index";
        let single_command =
            consumer_verify_command(&destination, &single_target, index_url, &cargo, single.mode)
                .unwrap();
        let double_command =
            consumer_verify_command(&destination, &double_target, index_url, &cargo, double.mode)
                .unwrap();
        let single_args = single_command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let double_args = double_command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            single_args,
            [
                "check",
                "--locked",
                "--target",
                WASM_TARGET,
                "--no-default-features"
            ]
        );
        assert_eq!(
            double_args,
            [
                "check",
                "--locked",
                "--target",
                WASM_TARGET,
                "--no-default-features",
                "--features",
                "double-precision"
            ]
        );

        for (command, target) in [
            (&single_command, &single_target),
            (&double_command, &double_target),
        ] {
            let environment = command
                .get_envs()
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(
                environment
                    .get(OsStr::new("BOXDD_SYS_PROVIDER"))
                    .copied()
                    .flatten(),
                Some(OsStr::new("wasm-provider"))
            );
            assert_eq!(
                environment
                    .get(OsStr::new("CARGO_TARGET_DIR"))
                    .copied()
                    .flatten(),
                Some(target.as_os_str())
            );
            assert_eq!(
                environment.get(OsStr::new("CARGO_HOME")).copied().flatten(),
                Some(cargo_home.as_os_str()),
            );
            assert_eq!(
                environment
                    .get(OsStr::new("CARGO_BUILD_JOBS"))
                    .copied()
                    .flatten(),
                Some(OsStr::new("2"))
            );
            assert_eq!(
                environment
                    .get(OsStr::new("CARGO_INCREMENTAL"))
                    .copied()
                    .flatten(),
                Some(OsStr::new("0"))
            );
            for key in [
                "DOCS_RS",
                "CARGO_CFG_DOCSRS",
                "RUSTFLAGS",
                "CARGO_ENCODED_RUSTFLAGS",
            ] {
                assert_eq!(
                    environment.get(OsStr::new(key)).copied(),
                    Some(None),
                    "{key} must be removed from {target:?}"
                );
            }
        }
    }

    #[test]
    fn sys_consumer_asserts_current_adapter_abi_version() {
        let declared =
            declared_adapter_abi_version(include_str!("../../../boxdd-sys/src/adapter.rs"));
        let asserted = asserted_adapter_abi_version(include_str!(
            "../../fixtures/package-consumers/sys/src/main.rs"
        ));

        assert_eq!(
            asserted, declared,
            "the isolated-registry sys consumer must track the published adapter ABI"
        );
    }
}
