use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::Command,
};

use flate2::read::GzDecoder;

use crate::{Error, Result, provider_catalog::ProviderCapability};

use super::support::{
    BoundedReader, CARGO_SUBPROCESS_JOBS, CargoEnvironment, WASM_TARGET, run_command,
};

const PACKAGES: &[&str] = &["boxdd-sys", "boxdd", "bevy_boxdd"];
const MAX_PACKAGE_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PACKAGE_ENTRIES: usize = 4_096;
const MAX_PACKAGE_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PACKAGE_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const TAR_BLOCK_BYTES: u64 = 512;
const MAX_PACKAGE_ARCHIVE_STREAM_BYTES: u64 = MAX_PACKAGE_TOTAL_BYTES
    + (MAX_PACKAGE_ENTRIES as u64 * TAR_BLOCK_BYTES * 3)
    + (TAR_BLOCK_BYTES * 2);

const CONSUMERS: &[ConsumerSpec] = &[
    ConsumerSpec {
        name: "sys",
        fixture: "sys",
        internal_packages: &["boxdd-sys"],
        mode: ConsumerMode::Native { double: false },
    },
    ConsumerSpec {
        name: "core",
        fixture: "core",
        internal_packages: &["boxdd", "boxdd-sys"],
        mode: ConsumerMode::Native { double: false },
    },
    ConsumerSpec {
        name: "core-double",
        fixture: "core",
        internal_packages: &["boxdd", "boxdd-sys"],
        mode: ConsumerMode::Native { double: true },
    },
    ConsumerSpec {
        name: "bevy",
        fixture: "bevy",
        internal_packages: &["bevy_boxdd", "boxdd", "boxdd-sys"],
        mode: ConsumerMode::Native { double: false },
    },
    ConsumerSpec {
        name: "bevy-double",
        fixture: "bevy",
        internal_packages: &["bevy_boxdd", "boxdd", "boxdd-sys"],
        mode: ConsumerMode::Native { double: true },
    },
    ConsumerSpec {
        name: "wasm-compile-only-single",
        fixture: "wasm-compile-only",
        internal_packages: &["boxdd-sys"],
        mode: ConsumerMode::WasmCompileOnly { double: false },
    },
    ConsumerSpec {
        name: "wasm-compile-only-double",
        fixture: "wasm-compile-only",
        internal_packages: &["boxdd-sys"],
        mode: ConsumerMode::WasmCompileOnly { double: true },
    },
];

const SYS_REQUIRED_ASSETS: &[&str] = &[
    "effective-source.toml",
    "upstream.toml",
    "abi/wasm32-unknown-unknown-single.toml",
    "abi/wasm32-unknown-unknown-double.toml",
    "security/sigstore/trusted_root.json",
    "native/boxdd_adapter.h",
    "native/boxdd_adapter.c",
    "native/boxdd_wasm_runtime.js",
    "src/bindings_pregenerated.rs",
    "src/bindings_double.rs",
    "src/bindings_wasm32_unknown_unknown.rs",
    "src/bindings_wasm32_unknown_unknown_double.rs",
    "src/bindings_wasm32_wasip1.rs",
    "src/bindings_wasm32_wasip1_double.rs",
    "src/bindgen_headers/wasm32_unknown_unknown/math.h",
    "third-party/box2d/LICENSE",
    "third-party/box2d/include/box2d/box2d.h",
];

#[derive(Clone, Copy, Debug)]
struct ConsumerSpec {
    name: &'static str,
    fixture: &'static str,
    internal_packages: &'static [&'static str],
    mode: ConsumerMode,
}

#[derive(Clone, Copy, Debug)]
enum ConsumerMode {
    Native { double: bool },
    WasmCompileOnly { double: bool },
}

struct ExtractedPackage {
    root: PathBuf,
    files: BTreeSet<PathBuf>,
}

#[derive(Default)]
struct PackageArchiveBudget {
    entries: usize,
    file_bytes: u64,
}

impl PackageArchiveBudget {
    fn observe(&mut self, file_size: Option<u64>) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or_else(|| Error::message("package archive entry count overflow"))?;
        if self.entries > MAX_PACKAGE_ENTRIES {
            return Err(Error::message(format!(
                "package archive contains more than {MAX_PACKAGE_ENTRIES} entries"
            )));
        }
        let Some(file_size) = file_size else {
            return Ok(());
        };
        if file_size > MAX_PACKAGE_FILE_BYTES {
            return Err(Error::message(format!(
                "package archive member exceeds the {MAX_PACKAGE_FILE_BYTES}-byte limit"
            )));
        }
        self.file_bytes = self
            .file_bytes
            .checked_add(file_size)
            .ok_or_else(|| Error::message("package archive expanded byte count overflow"))?;
        if self.file_bytes > MAX_PACKAGE_TOTAL_BYTES {
            return Err(Error::message(format!(
                "package archive expands beyond the {MAX_PACKAGE_TOTAL_BYTES}-byte limit"
            )));
        }
        Ok(())
    }
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
        .prefix("boxdd-package-verification-")
        .tempdir()
        .map_err(|source| Error::io("create package verification directory", source))?;
    let package_target = temporary.path().join("package-target");
    let cargo_home = temporary.path().join("cargo-home");
    fs::create_dir(&cargo_home).map_err(|source| Error::io(&cargo_home, source))?;
    reject_external_cargo_configuration(root)?;
    let extraction_root = temporary.path().join("packages");
    let mut package_roots = BTreeMap::new();
    let archives = package_crates(root, &package_target, &cargo_home, &version)?;

    for &package in PACKAGES {
        let archive = archives
            .get(package)
            .ok_or_else(|| Error::message(format!("packaged archive is missing for {package}")))?;
        let extracted = extract_package_archive(archive, &extraction_root, package, &version)?;
        validate_package_contents(root, package, &version, &extracted)?;
        package_roots.insert(package.to_owned(), extracted.root);
    }

    let consumer_target = temporary.path().join("consumer-target");
    for consumer in CONSUMERS {
        verify_consumer(
            root,
            temporary.path(),
            &consumer_target,
            &cargo_home,
            &package_roots,
            &version,
            *consumer,
        )?;
    }

    println!(
        "verified {} package archives with {} fixed native/WASM consumers",
        PACKAGES.len(),
        CONSUMERS.len()
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
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::message("workspace.package.version is missing"))
}

fn package_crates(
    root: &Path,
    target: &Path,
    cargo_home: &Path,
    version: &str,
) -> Result<BTreeMap<String, PathBuf>> {
    for package in PACKAGES {
        let manifest = root.join(package).join("Cargo.toml");
        if !manifest.is_file() {
            return Err(Error::message(format!(
                "publishable package manifest is missing: {}",
                manifest.display()
            )));
        }
    }

    let mut command = cargo_command(root, target, cargo_home);
    command
        .arg("package")
        .arg("--allow-dirty")
        .arg("--no-verify")
        .arg("--locked");
    for package in PACKAGES {
        command.arg("--package").arg(package);
    }
    run_command(&mut command, "package the publishable workspace batch")?;

    PACKAGES
        .iter()
        .map(|package| {
            let archive = target
                .join("package")
                .join(format!("{package}-{version}.crate"));
            if !archive.is_file() {
                return Err(Error::message(format!(
                    "cargo package did not produce {}",
                    archive.display()
                )));
            }
            Ok(((*package).to_owned(), archive))
        })
        .collect()
}

fn cargo_command(working_dir: &Path, target: &Path, cargo_home: &Path) -> Command {
    let mut command = Command::new("cargo");
    CargoEnvironment::fail_closed(cargo_home).apply(&mut command);
    command
        .current_dir(working_dir)
        .env("CARGO_BUILD_JOBS", CARGO_SUBPROCESS_JOBS)
        .env("CARGO_TARGET_DIR", target);
    command
}

fn reject_external_cargo_configuration(workspace_root: &Path) -> Result<()> {
    for ancestor in workspace_root.ancestors().skip(1) {
        for relative in [".cargo/config.toml", ".cargo/config"] {
            let candidate = ancestor.join(relative);
            match fs::symlink_metadata(&candidate) {
                Ok(_) => {
                    return Err(Error::message(format!(
                        "verify-packages refuses external Cargo configuration: {}",
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

fn extract_package_archive(
    archive: &Path,
    extraction_root: &Path,
    package: &str,
    version: &str,
) -> Result<ExtractedPackage> {
    let prefix = format!("{package}-{version}");
    let destination = extraction_root.join(&prefix);
    fs::create_dir_all(&destination).map_err(|error| Error::io(&destination, error))?;

    // Validate the opened object rather than racing path metadata. A symlink to a
    // regular file is therefore allowed, while archive link entries remain forbidden.
    let file = fs::File::open(archive).map_err(|error| Error::io(archive, error))?;
    let metadata = file.metadata().map_err(|error| Error::io(archive, error))?;
    if !metadata.is_file() {
        return Err(Error::message(format!(
            "opened package archive must be a regular file: {}",
            archive.display()
        )));
    }
    if metadata.len() > MAX_PACKAGE_ARCHIVE_BYTES {
        return Err(Error::message(format!(
            "package archive exceeds the {MAX_PACKAGE_ARCHIVE_BYTES}-byte compressed limit: {}",
            archive.display()
        )));
    }

    let mut tar = tar::Archive::new(BoundedReader::new(
        GzDecoder::new(BoundedReader::new(
            file,
            MAX_PACKAGE_ARCHIVE_BYTES,
            "compressed package archive exceeds the input limit",
        )),
        MAX_PACKAGE_ARCHIVE_STREAM_BYTES,
        "decompressed package archive exceeds the stream limit",
    ));
    let entries = tar
        .entries()
        .map_err(|error| Error::message(format!("read {}: {error}", archive.display())))?;
    let mut seen = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut budget = PackageArchiveBudget::default();

    for entry in entries {
        let mut entry = entry.map_err(|error| {
            Error::message(format!("read {} entry: {error}", archive.display()))
        })?;
        let archive_path = entry
            .path()
            .map_err(|error| Error::message(format!("read package entry path: {error}")))?;
        let relative = package_relative_path(&archive_path, OsStr::new(&prefix))?;
        let entry_type = entry.header().entry_type();
        budget.observe(entry_type.is_file().then_some(entry.size()))?;

        let Some(relative) = relative else {
            if entry_type.is_dir() {
                continue;
            }
            return Err(Error::message(format!(
                "{} contains a non-directory entry at its package root",
                archive.display()
            )));
        };
        if !seen.insert(relative.clone()) {
            return Err(Error::message(format!(
                "{} contains duplicate entry {}",
                archive.display(),
                relative.display()
            )));
        }

        let output = destination.join(&relative);
        if entry_type.is_dir() {
            fs::create_dir_all(&output).map_err(|error| Error::io(&output, error))?;
            continue;
        }
        if !entry_type.is_file() {
            return Err(Error::message(format!(
                "{} contains unsupported non-file entry {}",
                archive.display(),
                relative.display()
            )));
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }
        let mut output_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&output)
            .map_err(|error| Error::io(&output, error))?;
        let expected_size = entry.size();
        let copied =
            io::copy(&mut entry, &mut output_file).map_err(|error| Error::io(&output, error))?;
        if copied != expected_size {
            return Err(Error::message(format!(
                "package archive member {} produced {copied} bytes; expected {expected_size}",
                relative.display()
            )));
        }
        output_file
            .flush()
            .map_err(|error| Error::io(&output, error))?;
        files.insert(relative);
    }

    Ok(ExtractedPackage {
        root: destination,
        files,
    })
}

fn package_relative_path(path: &Path, expected_prefix: &OsStr) -> Result<Option<PathBuf>> {
    if path.to_string_lossy().contains('\\') {
        return Err(Error::message(format!(
            "package archive path contains a backslash: {}",
            path.display()
        )));
    }

    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(prefix)) if prefix == expected_prefix => {}
        _ => {
            return Err(Error::message(format!(
                "package archive path is outside {}: {}",
                expected_prefix.to_string_lossy(),
                path.display()
            )));
        }
    }

    let mut relative = PathBuf::new();
    for component in components {
        match component {
            Component::Normal(component) => relative.push(component),
            _ => {
                return Err(Error::message(format!(
                    "package archive path is not normalized: {}",
                    path.display()
                )));
            }
        }
    }
    Ok((!relative.as_os_str().is_empty()).then_some(relative))
}

fn validate_package_contents(
    repository_root: &Path,
    package: &str,
    version: &str,
    extracted: &ExtractedPackage,
) -> Result<()> {
    let manifest_path = extracted.root.join("Cargo.toml");
    let manifest_source =
        fs::read_to_string(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
    let manifest: toml::Value = toml::from_str(&manifest_source).map_err(|error| {
        Error::message(format!(
            "{} is invalid normalized package TOML: {error}",
            manifest_path.display()
        ))
    })?;
    let identity = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::message("packaged Cargo.toml has no package table"))?;
    if identity.get("name").and_then(toml::Value::as_str) != Some(package)
        || identity.get("version").and_then(toml::Value::as_str) != Some(version)
    {
        return Err(Error::message(format!(
            "packaged manifest identity does not match {} {version}",
            package
        )));
    }
    if package == "boxdd-sys" {
        validate_sys_package_surface(&manifest, &extracted.files)?;
    }

    let source_root = repository_root.join(package);
    let mut required = vec!["LICENSE-APACHE", "LICENSE-MIT"];
    if package == "boxdd-sys" {
        required.extend_from_slice(SYS_REQUIRED_ASSETS);
    } else if package == "bevy_boxdd" {
        required.push("MIGRATION.md");
    }
    for relative in required {
        let relative = Path::new(relative);
        if !extracted.files.contains(relative) {
            return Err(Error::message(format!(
                "packaged {} is missing {}",
                package,
                relative.display()
            )));
        }
        let expected_path = source_root.join(relative);
        let actual_path = extracted.root.join(relative);
        let expected =
            fs::read(&expected_path).map_err(|error| Error::io(&expected_path, error))?;
        let actual = fs::read(&actual_path).map_err(|error| Error::io(&actual_path, error))?;
        if actual != expected {
            return Err(Error::message(format!(
                "packaged {} changed required asset {}",
                package,
                relative.display()
            )));
        }
    }

    let actual_licenses = extracted
        .files
        .iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("LICENSE"))
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut expected_licenses = BTreeSet::from([
        PathBuf::from("LICENSE-APACHE"),
        PathBuf::from("LICENSE-MIT"),
    ]);
    if package == "boxdd-sys" {
        expected_licenses.insert(PathBuf::from("third-party/box2d/LICENSE"));
    }
    if actual_licenses != expected_licenses {
        return Err(Error::message(format!(
            "packaged {} has an unexpected license set: {actual_licenses:?}",
            package
        )));
    }
    Ok(())
}

fn validate_sys_package_surface(manifest: &toml::Value, files: &BTreeSet<PathBuf>) -> Result<()> {
    if let Some(path) = files.iter().find(|path| {
        path.starts_with("bin")
            || path.starts_with("src/bin")
            || path.as_path() == Path::new("src/main.rs")
    }) {
        return Err(Error::message(format!(
            "packaged boxdd-sys contains a binary source under {}",
            path.display()
        )));
    }

    let root = manifest
        .as_table()
        .ok_or_else(|| Error::message("packaged Cargo.toml root must be a table"))?;
    if root
        .get("features")
        .and_then(toml::Value::as_table)
        .is_some_and(|features| features.contains_key("package-bin"))
    {
        return Err(Error::message(
            "packaged boxdd-sys must not expose the removed package-bin feature",
        ));
    }
    if root
        .get("bin")
        .and_then(toml::Value::as_array)
        .is_some_and(|bins| !bins.is_empty())
    {
        return Err(Error::message(
            "packaged boxdd-sys must not declare binary targets",
        ));
    }

    const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "dev-dependencies"];
    for table in DEPENDENCY_TABLES {
        reject_packaging_runtime_dependencies(root.get(table), table)?;
    }
    if let Some(targets) = root.get("target").and_then(toml::Value::as_table) {
        for (target, specification) in targets {
            for table in DEPENDENCY_TABLES {
                let dependencies = specification
                    .as_table()
                    .and_then(|specification| specification.get(table));
                reject_packaging_runtime_dependencies(
                    dependencies,
                    &format!("target.{target}.{table}"),
                )?;
            }
        }
    }
    Ok(())
}

fn reject_packaging_runtime_dependencies(
    dependencies: Option<&toml::Value>,
    scope: &str,
) -> Result<()> {
    let Some(dependencies) = dependencies.and_then(toml::Value::as_table) else {
        return Ok(());
    };
    for (name, specification) in dependencies {
        let package = specification
            .as_table()
            .and_then(|specification| specification.get("package"))
            .and_then(toml::Value::as_str)
            .unwrap_or(name);
        if matches!(package, "flate2" | "tar") {
            return Err(Error::message(format!(
                "packaged boxdd-sys {scope} contains removed packaging dependency {package}"
            )));
        }
    }
    Ok(())
}

fn verify_consumer(
    repository_root: &Path,
    temporary_root: &Path,
    target: &Path,
    cargo_home: &Path,
    package_roots: &BTreeMap<String, PathBuf>,
    version: &str,
    consumer: ConsumerSpec,
) -> Result<()> {
    let source = repository_root
        .join("xtask/fixtures/package-consumers")
        .join(consumer.fixture);
    let destination = temporary_root.join("consumers").join(consumer.name);
    copy_tree(&source, &destination)?;

    let manifest_path = destination.join("Cargo.toml");
    let manifest_source =
        fs::read_to_string(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
    let rendered = render_consumer_manifest(&manifest_source, version, package_roots, consumer)?;
    fs::write(&manifest_path, rendered).map_err(|error| Error::io(&manifest_path, error))?;

    let mut command = cargo_command(repository_root, target, cargo_home);
    match consumer.mode {
        ConsumerMode::Native { double } => {
            command
                .arg("run")
                .arg("--manifest-path")
                .arg(&manifest_path);
            if double {
                command.arg("--features").arg("double-precision");
            }
        }
        ConsumerMode::WasmCompileOnly { double } => {
            command
                .arg("check")
                .arg("--manifest-path")
                .arg(&manifest_path)
                .arg("--lib")
                .arg("--target")
                .arg(WASM_TARGET)
                .arg("--no-default-features")
                .env(
                    "BOXDD_SYS_PROVIDER",
                    ProviderCapability::WasmCompileOnly.as_str(),
                );
            if double {
                command.arg("--features").arg("double-precision");
            }
        }
    }
    run_command(
        &mut command,
        &format!("verify packaged {} consumer", consumer.name),
    )?;
    validate_consumer_lock(
        &destination.join("Cargo.lock"),
        version,
        consumer.internal_packages,
        consumer.name,
    )
}

fn validate_consumer_lock(
    lock_path: &Path,
    version: &str,
    internal_packages: &[&str],
    consumer_name: &str,
) -> Result<()> {
    let source = fs::read_to_string(lock_path).map_err(|error| Error::io(lock_path, error))?;
    let lock: toml::Value = toml::from_str(&source).map_err(|error| {
        Error::message(format!(
            "packaged {consumer_name} consumer generated an invalid Cargo.lock: {error}"
        ))
    })?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            Error::message(format!(
                "packaged {consumer_name} consumer Cargo.lock has no package entries"
            ))
        })?;

    for expected in internal_packages {
        let matches = packages
            .iter()
            .filter_map(toml::Value::as_table)
            .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some(expected))
            .collect::<Vec<_>>();
        let resolved_locally = matches.len() == 1
            && matches[0].get("version").and_then(toml::Value::as_str) == Some(version)
            && !matches[0].contains_key("source");
        if !resolved_locally {
            return Err(Error::message(format!(
                "packaged {consumer_name} consumer must resolve exactly one local {expected} {version} package; found {} matching lock entries",
                matches.len()
            )));
        }
    }
    Ok(())
}

fn render_consumer_manifest(
    source: &str,
    version: &str,
    package_roots: &BTreeMap<String, PathBuf>,
    consumer: ConsumerSpec,
) -> Result<String> {
    let mut manifest: toml::Value = toml::from_str(source)
        .map_err(|error| Error::message(format!("consumer fixture is invalid TOML: {error}")))?;
    let root = manifest
        .as_table_mut()
        .ok_or_else(|| Error::message("consumer fixture root must be a TOML table"))?;
    let package = root
        .get_mut("package")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| Error::message("consumer fixture has no package table"))?;
    package.insert(
        "name".to_owned(),
        toml::Value::String(format!("boxdd-package-consumer-{}", consumer.name)),
    );

    let known_packages = PACKAGES.iter().copied().collect::<BTreeSet<_>>();
    let dependencies = root
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| Error::message("consumer fixture has no dependencies table"))?;
    let mut rewritten_dependencies = 0;
    for (name, dependency) in dependencies {
        if !known_packages.contains(name.as_str()) {
            continue;
        }
        let dependency = dependency.as_table_mut().ok_or_else(|| {
            Error::message(format!(
                "consumer dependency {name} must use structured TOML"
            ))
        })?;
        dependency.insert(
            "version".to_owned(),
            toml::Value::String(format!("={version}")),
        );
        dependency.remove("registry");
        dependency.remove("path");
        dependency.remove("git");
        rewritten_dependencies += 1;
    }
    if rewritten_dependencies == 0 {
        return Err(Error::message(
            "consumer fixture has no publishable package dependency",
        ));
    }

    let mut crates_io = toml::Table::new();
    for package in consumer.internal_packages {
        let package_root = package_roots.get(*package).ok_or_else(|| {
            Error::message(format!("extracted package root is missing for {package}"))
        })?;
        let package_root = package_root.to_str().ok_or_else(|| {
            Error::message(format!(
                "extracted package path is not UTF-8: {}",
                package_root.display()
            ))
        })?;
        crates_io.insert(
            (*package).to_owned(),
            toml::Value::Table(toml::Table::from_iter([(
                "path".to_owned(),
                toml::Value::String(package_root.to_owned()),
            )])),
        );
    }
    root.insert(
        "patch".to_owned(),
        toml::Value::Table(toml::Table::from_iter([(
            "crates-io".to_owned(),
            toml::Value::Table(crates_io),
        )])),
    );

    toml::to_string_pretty(&manifest)
        .map_err(|error| Error::message(format!("render consumer Cargo.toml: {error}")))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(|error| Error::io(source, error))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "consumer fixture must be a real directory: {}",
            source.display()
        )));
    }
    fs::create_dir_all(destination).map_err(|error| Error::io(destination, error))?;

    let mut entries = fs::read_dir(source)
        .map_err(|error| Error::io(source, error))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| Error::io(source, error))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| Error::io(&from, error))?;
        if file_type.is_symlink() {
            return Err(Error::message(format!(
                "consumer fixtures cannot contain symlinks: {}",
                from.display()
            )));
        }
        if file_type.is_dir() {
            copy_tree(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(|error| Error::io(&to, error))?;
        } else {
            return Err(Error::message(format!(
                "consumer fixture contains unsupported entry: {}",
                from.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use flate2::{Compression, write::GzEncoder};

    use super::*;

    #[test]
    fn package_cargo_command_uses_a_private_home_and_removes_injection() {
        let sandbox = tempfile::tempdir().unwrap();
        let workspace = sandbox.path().join("workspace");
        let target = sandbox.path().join("target");
        let cargo_home = sandbox.path().join("cargo-home");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&cargo_home).unwrap();

        let command = cargo_command(&workspace, &target, &cargo_home);
        assert_eq!(command.get_current_dir(), Some(workspace.as_path()));
        assert!(command.get_envs().any(|(key, value)| {
            key == OsStr::new("CARGO_HOME") && value == Some(cargo_home.as_os_str())
        }));
        for key in [
            "RUSTC_WRAPPER",
            "RUSTUP_HOME",
            "RUSTUP_TOOLCHAIN",
            "CARGO_BUILD_RUNNER",
            "CC",
            "BASH_ENV",
        ] {
            assert!(
                command
                    .get_envs()
                    .any(|(actual, value)| actual == OsStr::new(key) && value.is_none()),
                "{key} was not removed"
            );
        }
    }

    #[test]
    fn package_gate_rejects_cargo_configuration_above_the_workspace() {
        let sandbox = tempfile::tempdir().unwrap();
        let external_config = sandbox.path().join(".cargo/config.toml");
        fs::create_dir(sandbox.path().join(".cargo")).unwrap();
        fs::write(&external_config, "[build]\nrustc-wrapper = 'payload'\n").unwrap();
        let workspace = sandbox.path().join("workspace");
        fs::create_dir(&workspace).unwrap();

        let error = reject_external_cargo_configuration(&workspace)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&external_config.display().to_string()));
    }

    #[test]
    fn package_paths_are_confined_to_the_exact_archive_prefix() {
        let prefix = OsStr::new("boxdd-0.6.0");
        assert_eq!(
            package_relative_path(Path::new("boxdd-0.6.0/src/lib.rs"), prefix).unwrap(),
            Some(PathBuf::from("src/lib.rs"))
        );
        assert!(package_relative_path(Path::new("other-0.6.0/src/lib.rs"), prefix).is_err());
        assert!(package_relative_path(Path::new("/boxdd-0.6.0/src/lib.rs"), prefix).is_err());
        assert!(package_relative_path(Path::new("boxdd-0.6.0\\src\\lib.rs"), prefix).is_err());
    }

    #[test]
    fn consumer_manifest_uses_exact_versions_and_local_crate_patches() {
        let source = r#"
[package]
name = "fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
boxdd = { version = "0", registry = "legacy" }

[workspace]
"#;
        let roots = BTreeMap::from([
            ("boxdd".to_owned(), PathBuf::from("/packages/boxdd-0.6.0")),
            (
                "boxdd-sys".to_owned(),
                PathBuf::from("/packages/boxdd-sys-0.6.0"),
            ),
        ]);
        let consumer = CONSUMERS
            .iter()
            .find(|consumer| consumer.name == "core")
            .copied()
            .unwrap();
        let rendered = render_consumer_manifest(source, "0.6.0", &roots, consumer).unwrap();
        let manifest: toml::Value = toml::from_str(&rendered).unwrap();

        assert_eq!(
            manifest["package"]["name"].as_str(),
            Some("boxdd-package-consumer-core")
        );
        assert_eq!(
            manifest["dependencies"]["boxdd"]["version"].as_str(),
            Some("=0.6.0")
        );
        assert!(manifest["dependencies"]["boxdd"].get("registry").is_none());
        assert_eq!(
            manifest["patch"]["crates-io"]["boxdd"]["path"].as_str(),
            Some("/packages/boxdd-0.6.0")
        );
        assert_eq!(
            manifest["patch"]["crates-io"]["boxdd-sys"]["path"].as_str(),
            Some("/packages/boxdd-sys-0.6.0")
        );
        assert!(manifest["patch"]["crates-io"].get("bevy_boxdd").is_none());
    }

    #[test]
    fn consumer_lock_requires_every_internal_package_to_resolve_locally() {
        let lock = r#"
version = 4

[[package]]
name = "boxdd"
version = "0.6.0"

[[package]]
name = "boxdd-sys"
version = "0.6.0"
"#;
        let directory = tempfile::tempdir().unwrap();
        let lock_path = directory.path().join("Cargo.lock");
        fs::write(&lock_path, lock).unwrap();

        validate_consumer_lock(&lock_path, "0.6.0", &["boxdd", "boxdd-sys"], "core").unwrap();
        assert!(
            validate_consumer_lock(
                &lock_path,
                "0.6.0",
                &["boxdd", "boxdd-sys", "bevy_boxdd"],
                "core",
            )
            .unwrap_err()
            .to_string()
            .contains("bevy_boxdd")
        );

        fs::write(
            &lock_path,
            lock.replace(
                "name = \"boxdd-sys\"\nversion = \"0.6.0\"",
                "name = \"boxdd-sys\"\nversion = \"0.6.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"",
            ),
        )
        .unwrap();
        assert!(validate_consumer_lock(&lock_path, "0.6.0", &["boxdd-sys"], "sys").is_err());

        fs::write(
            &lock_path,
            format!(
                "{lock}\n[[package]]\nname = \"boxdd-sys\"\nversion = \"0.6.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
            ),
        )
        .unwrap();
        let error = validate_consumer_lock(&lock_path, "0.6.0", &["boxdd-sys"], "sys")
            .unwrap_err()
            .to_string();
        assert!(error.contains("exactly one local boxdd-sys"), "{error}");
    }

    #[test]
    fn archive_extraction_accepts_regular_files_and_rejects_link_entries() {
        let repository = tempfile::tempdir().unwrap();
        let archive = repository.path().join("boxdd-0.6.0.crate");
        write_archive(&archive, &[("boxdd-0.6.0/Cargo.toml", b"manifest")], None);
        let extracted = extract_package_archive(
            &archive,
            &repository.path().join("extracted"),
            "boxdd",
            "0.6.0",
        )
        .unwrap();
        assert_eq!(
            fs::read(extracted.root.join("Cargo.toml")).unwrap(),
            b"manifest"
        );

        let linked_archive = repository.path().join("boxdd-linked.crate");
        write_archive(
            &linked_archive,
            &[],
            Some(("boxdd-0.6.0/LICENSE-MIT", "target")),
        );
        assert!(
            extract_package_archive(
                &linked_archive,
                &repository.path().join("linked"),
                "boxdd",
                "0.6.0"
            )
            .is_err()
        );
    }

    #[test]
    fn package_archive_budget_rejects_entry_and_expansion_limits() {
        let mut entries = PackageArchiveBudget {
            entries: MAX_PACKAGE_ENTRIES,
            file_bytes: 0,
        };
        assert!(entries.observe(None).is_err());

        let mut member = PackageArchiveBudget::default();
        assert!(member.observe(Some(MAX_PACKAGE_FILE_BYTES + 1)).is_err());

        let mut total = PackageArchiveBudget {
            entries: 1,
            file_bytes: MAX_PACKAGE_TOTAL_BYTES,
        };
        assert!(total.observe(Some(1)).is_err());
    }

    #[test]
    fn package_archive_rejects_an_oversized_compressed_input_before_decoding() {
        let repository = tempfile::tempdir().unwrap();
        let archive = repository.path().join("boxdd-0.6.0.crate");
        let file = fs::File::create(&archive).unwrap();
        file.set_len(MAX_PACKAGE_ARCHIVE_BYTES + 1).unwrap();

        let error = extract_package_archive(
            &archive,
            &repository.path().join("extracted"),
            "boxdd",
            "0.6.0",
        )
        .err()
        .expect("oversized compressed package archive must be rejected")
        .to_string();
        assert!(error.contains("compressed limit"), "{error}");
    }

    #[test]
    fn sys_package_surface_rejects_repository_packaging_tools() {
        let empty_files = BTreeSet::new();
        let cases = [
            (
                "package-bin feature",
                "[features]\npackage-bin = []\n",
                empty_files.clone(),
            ),
            (
                "binary target",
                "[[bin]]\nname = \"diagnostic\"\npath = \"bin/diagnostic.rs\"\n",
                empty_files.clone(),
            ),
            (
                "renamed flate2 dependency",
                "[dependencies]\narchive = { package = \"flate2\", version = \"1\" }\n",
                empty_files.clone(),
            ),
            (
                "target tar dependency",
                "[target.'cfg(unix)'.dependencies]\ntar = \"0.4\"\n",
                empty_files.clone(),
            ),
            (
                "build flate2 dependency",
                "[build-dependencies]\nflate2 = \"1\"\n",
                empty_files.clone(),
            ),
            (
                "dev tar dependency",
                "[dev-dependencies]\ntar = \"0.4\"\n",
                empty_files.clone(),
            ),
            (
                "target build tar dependency",
                "[target.'cfg(unix)'.build-dependencies]\ntar = \"0.4\"\n",
                empty_files.clone(),
            ),
            (
                "target dev flate2 dependency",
                "[target.'cfg(unix)'.dev-dependencies]\nflate2 = \"1\"\n",
                empty_files.clone(),
            ),
            (
                "packaging source",
                "",
                BTreeSet::from([PathBuf::from("bin/package/main.rs")]),
            ),
            (
                "auto-discovered binary",
                "",
                BTreeSet::from([PathBuf::from("src/bin/package.rs")]),
            ),
            (
                "auto-discovered main",
                "",
                BTreeSet::from([PathBuf::from("src/main.rs")]),
            ),
        ];

        for (label, source, files) in cases {
            let manifest = toml::from_str(source).unwrap();
            let error = validate_sys_package_surface(&manifest, &files)
                .expect_err("repository packaging surface must stay out of boxdd-sys");
            assert!(!error.to_string().is_empty(), "missing error for {label}");
        }

        let manifest =
            toml::from_str("[features]\ndefault = []\n[dependencies]\nsha2 = \"0.10\"\n").unwrap();
        validate_sys_package_surface(&manifest, &empty_files).unwrap();
    }

    fn write_archive(path: &Path, files: &[(&str, &[u8])], link: Option<(&str, &str)>) {
        let file = fs::File::create(path).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (path, bytes) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, path, Cursor::new(*bytes))
                .unwrap();
        }
        if let Some((path, target)) = link {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mode(0o777);
            header.set_link_name(target).unwrap();
            header.set_cksum();
            archive.append_data(&mut header, path, io::empty()).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }
}
