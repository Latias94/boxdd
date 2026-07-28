//! Build a deterministic, locally consumable Box2D prebuilt archive.
//!
//! This helper never downloads, extracts, or discovers a provider by package name.  It packages
//! the exact vendored build output selected by an explicit `BOXDD_SYS_PACKAGE_OUT_DIR`, or by a
//! unique build identity written by `boxdd-sys/build.rs`.

#[allow(dead_code)]
#[path = "../../src/provider_manifest.rs"]
mod provider_manifest;

#[allow(dead_code)]
#[path = "../../src/provider_archive.rs"]
mod provider_archive;

#[allow(dead_code)]
#[path = "../../src/source_overlay.rs"]
mod source_overlay;

use flate2::{Compression, write::GzEncoder};
use provider_archive::{ArchiveExpectation, verify_provider_archive};
use provider_manifest::{
    ADAPTER_ABI_VERSION, ArtifactIdentityExpectation, ArtifactManifest, RECORDING_CONTRACT_BLAKE3,
    REQUIRED_ADAPTER_SYMBOLS, required_adapter_symbols_sha256, sha256_bytes, sha256_file,
};
use source_overlay::{adapter_source_sha256, effective_source_identity};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const BUILD_IDENTITY_FILE: &str = "boxdd-build-identity.toml";
const ADAPTER_IDENTITY_FILE: &str = "adapter_identity.rs";
type PackageFiles = BTreeMap<String, Vec<u8>>;
type HeaderFiles = Vec<(String, Vec<u8>)>;

struct PackageRequest<'a> {
    manifest_dir: &'a Path,
    sys_out: &'a Path,
    target: &'a str,
    target_env: &'a str,
    crt: &'a str,
    precision: &'a str,
    crate_version: &'a str,
    source_commit: &'a str,
    release_tag: &'a str,
    effective_source_sha256: &'a str,
    adapter_source_sha256: &'a str,
    private_abi_hash: &'a str,
    snapshot_layout_hash: u32,
}

struct BuildOutputExpectation<'a> {
    crate_version: &'a str,
    target: &'a str,
    precision: &'a str,
    crt: &'a str,
    simd: &'a str,
    validate: bool,
    provider: &'a str,
    upstream_sha: &'a str,
    effective_source_sha256: &'a str,
    adapter_source_sha256: &'a str,
    private_abi_hash: &'a [u8; 32],
    snapshot_layout_hash: u32,
}

fn expected_lib_name(target_env: &str) -> &'static str {
    if target_env == "msvc" {
        "box2d.lib"
    } else {
        "libbox2d.a"
    }
}

fn default_target_triple() -> String {
    if let Ok(target) = env::var("TARGET") {
        return target;
    }
    if let Ok(target) = env::var("CARGO_CFG_TARGET_TRIPLE") {
        return target;
    }
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    match os {
        "windows" => format!("{arch}-pc-windows-msvc"),
        "macos" => format!("{arch}-apple-darwin"),
        "linux" => format!("{arch}-unknown-linux-gnu"),
        _ => format!("{arch}-unknown-{os}"),
    }
}

fn precision() -> &'static str {
    if cfg!(feature = "double-precision") {
        "double"
    } else {
        "single"
    }
}

fn active_target_env() -> String {
    env::var("CARGO_CFG_TARGET_ENV").unwrap_or_else(|_| {
        if cfg!(target_env = "msvc") {
            "msvc".to_owned()
        } else if cfg!(target_env = "gnu") {
            "gnu".to_owned()
        } else {
            String::new()
        }
    })
}

fn active_target_os() -> String {
    env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| std::env::consts::OS.into())
}

fn active_simd_identity() -> &'static str {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH")
        .ok()
        .or_else(|| {
            env::var("TARGET")
                .ok()
                .and_then(|target| target.split('-').next().map(ToOwned::to_owned))
        })
        .unwrap_or_else(|| env::consts::ARCH.to_owned());
    package_simd_identity(
        &target_arch,
        cfg!(feature = "disable-simd"),
        cfg!(feature = "simd-avx2"),
    )
}

fn validation_enabled() -> bool {
    cfg!(feature = "validate")
}

fn package_simd_identity(
    target_arch: &str,
    disable_simd: bool,
    avx2_feature: bool,
) -> &'static str {
    if target_arch == "wasm32" || disable_simd {
        "disabled"
    } else if avx2_feature && target_arch == "x86_64" {
        "avx2"
    } else {
        "default"
    }
}

fn private_abi_hash_hex() -> String {
    boxdd_sys::PRIVATE_ABI_HASH
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn binding_path(manifest_dir: &Path, precision: &str) -> PathBuf {
    manifest_dir.join("src").join(if precision == "double" {
        "bindings_double.rs"
    } else {
        "bindings_pregenerated.rs"
    })
}

fn compose_archive_name(
    crate_short: &str,
    version: &str,
    target: &str,
    precision: &str,
    link_type: &str,
    crt: &str,
) -> String {
    if crt == "none" {
        format!("{crate_short}-prebuilt-{version}-{target}-{precision}-{link_type}.tar.gz")
    } else {
        format!("{crate_short}-prebuilt-{version}-{target}-{precision}-{link_type}-{crt}.tar.gz")
    }
}

fn normalize_crt(value: &str) -> Result<&'static str, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "none" => Ok("none"),
        "md" => Ok("md"),
        "mt" => Ok("mt"),
        _ => Err(format!(
            "BOXDD_SYS_PACKAGE_CRT must be empty, `none`, `md`, or `mt`; got `{value}`"
        )),
    }
}

fn detect_crt(
    target_os: &str,
    target_env: &str,
    target_features: &str,
    explicit_crt: &str,
) -> Result<&'static str, String> {
    let explicit = normalize_crt(explicit_crt)?;
    if explicit != "none" || !explicit_crt.trim().is_empty() {
        return Ok(explicit);
    }
    if target_os == "windows" && target_env == "msvc" {
        if target_features
            .split(',')
            .any(|feature| feature == "crt-static")
        {
            Ok("mt")
        } else {
            Ok("md")
        }
    } else {
        Ok("none")
    }
}

fn parse_upstream_sha(upstream: &[u8]) -> Result<String, String> {
    let value: toml::Value = toml::from_str(
        std::str::from_utf8(upstream)
            .map_err(|error| format!("upstream.toml is not UTF-8: {error}"))?,
    )
    .map_err(|error| format!("upstream.toml is not valid TOML: {error}"))?;
    let sha = value
        .get("active_revision")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "upstream.toml has no active_revision".to_owned())?;
    if sha.len() != 40
        || !sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("upstream.toml active_revision must be a lowercase Git SHA".to_owned());
    }
    Ok(sha.to_owned())
}

fn package_source_commit() -> Result<String, String> {
    let commit = env::var("BOXDD_SYS_PACKAGE_SOURCE_COMMIT")
        .or_else(|_| env::var("GITHUB_SHA"))
        .map_err(|_| {
            "prebuilt packaging requires BOXDD_SYS_PACKAGE_SOURCE_COMMIT or GITHUB_SHA".to_owned()
        })?;
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("prebuilt package source commit must be a lowercase Git SHA".to_owned());
    }
    Ok(commit)
}

fn package_release_tag(crate_version: &str) -> Result<String, String> {
    let tag = env::var("BOXDD_SYS_PACKAGE_RELEASE_TAG")
        .or_else(|_| env::var("GITHUB_REF_NAME"))
        .map_err(|_| {
            "prebuilt packaging requires BOXDD_SYS_PACKAGE_RELEASE_TAG or GITHUB_REF_NAME"
                .to_owned()
        })?;
    let short_tag = format!("v{crate_version}");
    let crate_tag = format!("boxdd-sys-v{crate_version}");
    if tag != short_tag && tag != crate_tag {
        return Err(format!(
            "prebuilt package release tag `{tag}` does not match crate version {crate_version}"
        ));
    }
    Ok(tag)
}

fn identity_matches(path: &Path, expected: &BuildOutputExpectation<'_>) -> bool {
    let Ok(bytes) = fs::read(path.join(BUILD_IDENTITY_FILE)) else {
        return false;
    };
    let marker_matches = bytes == expected_build_identity_marker(expected).as_bytes();
    let expected_adapter_identity =
        adapter_identity_source(expected.private_abi_hash, expected.snapshot_layout_hash);
    marker_matches
        && fs::read_to_string(path.join(ADAPTER_IDENTITY_FILE))
            .ok()
            .as_deref()
            == Some(expected_adapter_identity.as_str())
}

fn expected_build_identity_marker(expected: &BuildOutputExpectation<'_>) -> String {
    format!(
        "schema_version = 2\nprovider = {:?}\ncrate_version = {:?}\nupstream_sha = {:?}\neffective_source_sha256 = {:?}\nprecision = {:?}\ntarget = {:?}\ncrt = {:?}\nsimd = {:?}\nvalidate = {}\nadapter_source_sha256 = {:?}\nrecording_contract_blake3 = {:?}\nmanifest_sha256 = \"\"\narchive_sha256 = \"\"\nprovenance_sha256 = \"\"\ntrusted_root_sha256 = \"\"\n",
        expected.provider,
        expected.crate_version,
        expected.upstream_sha,
        expected.effective_source_sha256,
        expected.precision,
        expected.target,
        expected.crt,
        expected.simd,
        expected.validate,
        expected.adapter_source_sha256,
        RECORDING_CONTRACT_BLAKE3,
    )
}

fn adapter_identity_source(private_abi_hash: &[u8; 32], snapshot_layout_hash: u32) -> String {
    let hash = private_abi_hash
        .iter()
        .map(|byte| format!("0x{byte:02X}, "))
        .collect::<String>();
    format!(
        "pub const PRIVATE_ABI_HASH: [u8; 32] = [{hash}];\n\
         pub const SNAPSHOT_LAYOUT_HASH: u32 = 0x{snapshot_layout_hash:08X};\n"
    )
}

fn locate_sys_out_dir(
    workspace_root: &Path,
    target_env: &str,
    expected: &BuildOutputExpectation<'_>,
) -> Result<PathBuf, String> {
    if let Some(explicit) = env::var_os("BOXDD_SYS_PACKAGE_OUT_DIR") {
        let path = PathBuf::from(explicit);
        if !path.is_dir() {
            return Err(format!(
                "BOXDD_SYS_PACKAGE_OUT_DIR is not a directory: {}",
                path.display()
            ));
        }
        if !identity_matches(&path, expected) || !path.join(expected_lib_name(target_env)).is_file()
        {
            return Err(format!(
                "BOXDD_SYS_PACKAGE_OUT_DIR does not contain the requested vendored build identity: {}",
                path.display()
            ));
        }
        return Ok(path);
    }

    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_owned());
    let roots = [
        target_dir
            .join(expected.target)
            .join(&profile)
            .join("build"),
        target_dir.join(&profile).join("build"),
    ];
    let mut candidates = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let out = path.join("out");
            if name.starts_with("boxdd-sys-")
                && out.is_dir()
                && identity_matches(&out, expected)
                && out.join(expected_lib_name(target_env)).is_file()
            {
                candidates.push(out);
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    if candidates.is_empty() {
        return Err(
            "no unique vendored boxdd-sys build output found; build the crate first or set BOXDD_SYS_PACKAGE_OUT_DIR"
                .to_owned(),
        );
    }
    let mut by_digest: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for candidate in candidates {
        let digest = sha256_file(&candidate.join(expected_lib_name(target_env)))?;
        by_digest.entry(digest).or_default().push(candidate);
    }
    if by_digest.len() == 1 {
        return Ok(by_digest
            .into_values()
            .next()
            .and_then(|mut paths| paths.pop())
            .expect("one digest must contain one build output"));
    }
    Err(format!(
        "matching boxdd-sys build outputs contain different static archives ({}); set BOXDD_SYS_PACKAGE_OUT_DIR explicitly",
        by_digest
            .iter()
            .map(|(digest, paths)| format!(
                "{}: {}",
                &digest[..16],
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn collect_headers(src_root: &Path) -> Result<HeaderFiles, Box<dyn std::error::Error>> {
    let mut stack = vec![src_root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let path = entry.path();
            if file_type.is_symlink() {
                return Err(format!("header tree contains a symlink: {}", path.display()).into());
            }
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if file_type.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("h"))
            {
                let relative = path
                    .strip_prefix(src_root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((format!("include/box2d/{relative}"), fs::read(path)?));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty() {
        return Err(format!("no public headers found under {}", src_root.display()).into());
    }
    Ok(files)
}

fn read_required(path: &Path, label: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if !path.is_file() {
        return Err(format!("required {label} is missing: {}", path.display()).into());
    }
    Ok(fs::read(path)?)
}

fn append_bytes<W: Write>(
    tar: &mut tar::Builder<W>,
    path: &str,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes)?;
    Ok(())
}

fn write_caller_trusted_system_manifest(
    prebuilt_manifest: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_root = prebuilt_manifest
        .parent()
        .ok_or("prebuilt manifest has no parent directory")?;
    let output_root = output
        .parent()
        .ok_or("system manifest output has no parent directory")?;
    if fs::canonicalize(input_root)? != fs::canonicalize(output_root)? {
        return Err(
            "caller-trusted system manifest must remain beside the prebuilt manifest so relative artifact paths stay bound"
                .into(),
        );
    }
    if prebuilt_manifest == output {
        return Err("caller-trusted conversion refuses to overwrite the prebuilt manifest".into());
    }
    let mut manifest = ArtifactManifest::parse(&fs::read(prebuilt_manifest)?)?;
    if manifest.provider != "prebuilt"
        || manifest.source_commit.is_none()
        || manifest.release_tag.is_none()
    {
        return Err("input is not a release-qualified prebuilt provider manifest".into());
    }
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let effective_source = effective_source_identity(&manifest_dir)?;
    if manifest.upstream_sha != effective_source.upstream_sha {
        return Err(
            "input manifest upstream SHA does not match the repository effective source".into(),
        );
    }
    manifest.validate_identity(&ArtifactIdentityExpectation {
        provider: "prebuilt",
        crate_version: &manifest.crate_version,
        upstream_sha: &manifest.upstream_sha,
        effective_source_sha256: &effective_source.effective_source_sha256,
        precision: &manifest.precision,
        target: &manifest.target,
        crt: &manifest.crt,
        simd: &manifest.simd,
        validate: manifest.validate,
        adapter_source_sha256: &manifest.adapter_source_sha256,
        private_abi_hash: &manifest.private_abi_hash,
        snapshot_layout_hash: u32::try_from(manifest.snapshot_layout_hash)?,
    })?;
    let archive =
        provider_manifest::resolve_relative_file(input_root, &manifest.archive, "archive")?;
    let verified_archive = verify_provider_archive(
        &archive,
        &ArchiveExpectation {
            target: &manifest.target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &manifest.private_abi_hash,
            snapshot_layout_hash: u32::try_from(manifest.snapshot_layout_hash)?,
        },
    )?;
    if verified_archive.archive_sha256 != manifest.archive_sha256 {
        return Err("input manifest archive digest does not match its bound file".into());
    }
    manifest.provider = "system".to_owned();
    manifest.source_commit = None;
    manifest.release_tag = None;
    write_new_manifest(output, &manifest.render())?;
    Ok(())
}

fn write_new_manifest(output: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn artifact_relative_path(
    root: &Path,
    path: &Path,
    label: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical = fs::canonicalize(path)?;
    if !canonical.is_file() || !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "caller-trusted {label} must be a regular file below {}: {}",
            canonical_root.display(),
            canonical.display()
        )
        .into());
    }
    let relative = canonical.strip_prefix(&canonical_root)?;
    if relative.as_os_str().is_empty() {
        return Err(format!("caller-trusted {label} path is empty").into());
    }
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn attest_local_system(
    archive: &Path,
    header: &Path,
    bindings: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_root = output
        .parent()
        .ok_or("system manifest output has no parent directory")?;
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let upstream = read_required(&manifest_dir.join("upstream.toml"), "upstream manifest")?;
    let upstream_sha = parse_upstream_sha(&upstream)?;
    let effective_source = effective_source_identity(&manifest_dir)?;
    if effective_source.upstream_sha != upstream_sha {
        return Err("upstream.toml does not match the repository effective-source identity".into());
    }
    let target = default_target_triple();
    let target_env = active_target_env();
    let target_os = active_target_os();
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let crt = detect_crt(
        &target_os,
        &target_env,
        &target_features,
        &env::var("BOXDD_SYS_PACKAGE_CRT").unwrap_or_default(),
    )?;
    let adapter_source_sha256 = adapter_source_sha256(&manifest_dir)?;
    let private_abi_hash = private_abi_hash_hex();
    let verified_archive = verify_provider_archive(
        archive,
        &ArchiveExpectation {
            target: &target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &private_abi_hash,
            snapshot_layout_hash: boxdd_sys::SNAPSHOT_LAYOUT_HASH,
        },
    )?;
    let archive_path = artifact_relative_path(output_root, archive, "archive")?;
    let header_path = artifact_relative_path(output_root, header, "header")?;
    let bindings_path = artifact_relative_path(output_root, bindings, "bindings")?;
    let manifest = ArtifactManifest {
        schema_version: provider_manifest::SCHEMA_VERSION,
        schema: provider_manifest::SCHEMA_NAME.to_owned(),
        provider: "system".to_owned(),
        crate_version: env::var("CARGO_PKG_VERSION")?,
        source_commit: None,
        release_tag: None,
        upstream_sha: upstream_sha.clone(),
        effective_source_sha256: effective_source.effective_source_sha256.clone(),
        precision: precision().to_owned(),
        target,
        link: "static".to_owned(),
        crt: crt.to_owned(),
        simd: active_simd_identity().to_owned(),
        validate: validation_enabled(),
        adapter_abi_version: ADAPTER_ABI_VERSION,
        adapter_source_sha256,
        private_abi_hash: private_abi_hash.clone(),
        snapshot_layout_hash: u64::from(boxdd_sys::SNAPSHOT_LAYOUT_HASH),
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
        required_adapter_symbols_sha256: required_adapter_symbols_sha256(),
        required_adapter_symbols: REQUIRED_ADAPTER_SYMBOLS
            .iter()
            .map(|symbol| (*symbol).to_owned())
            .collect(),
        archive: archive_path,
        archive_sha256: verified_archive.archive_sha256,
        header: header_path,
        header_sha256: sha256_file(header)?,
        bindings: bindings_path,
        bindings_sha256: sha256_file(bindings)?,
    };
    manifest.validate_identity(&ArtifactIdentityExpectation {
        provider: "system",
        crate_version: &manifest.crate_version,
        upstream_sha: &manifest.upstream_sha,
        effective_source_sha256: &effective_source.effective_source_sha256,
        precision: &manifest.precision,
        target: &manifest.target,
        crt: &manifest.crt,
        simd: &manifest.simd,
        validate: manifest.validate,
        adapter_source_sha256: &manifest.adapter_source_sha256,
        private_abi_hash: &private_abi_hash,
        snapshot_layout_hash: boxdd_sys::SNAPSHOT_LAYOUT_HASH,
    })?;
    write_new_manifest(output, &manifest.render())?;
    Ok(())
}

fn build_package_files(
    request: PackageRequest<'_>,
) -> Result<PackageFiles, Box<dyn std::error::Error>> {
    let PackageRequest {
        manifest_dir,
        sys_out,
        target,
        target_env,
        crt,
        precision,
        crate_version,
        source_commit,
        release_tag,
        effective_source_sha256,
        adapter_source_sha256,
        private_abi_hash,
        snapshot_layout_hash,
    } = request;
    let upstream = read_required(&manifest_dir.join("upstream.toml"), "upstream manifest")?;
    let upstream_sha = parse_upstream_sha(&upstream)?;
    let effective_source = effective_source_identity(manifest_dir)?;
    if effective_source.upstream_sha != upstream_sha
        || effective_source.effective_source_sha256 != effective_source_sha256
    {
        return Err(
            "package request does not match the repository effective-source identity".into(),
        );
    }
    let effective_source_manifest = read_required(
        &manifest_dir.join("effective-source.toml"),
        "effective source manifest",
    )?;
    let lib_name = expected_lib_name(target_env);
    let library_path = sys_out.join(lib_name);
    let verified_archive = verify_provider_archive(
        &library_path,
        &ArchiveExpectation {
            target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256,
            private_abi_hash,
            snapshot_layout_hash,
        },
    )?;
    let lib = verified_archive.archive_bytes;
    let include_root = manifest_dir.join("third-party/box2d/include/box2d");
    let headers = collect_headers(&include_root)?;
    let binding_source = binding_path(manifest_dir, precision);
    let binding = read_required(&binding_source, "pregenerated bindings")?;
    let binding_name = binding_source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid binding file name")?;
    let (header_path, header_bytes) = headers
        .iter()
        .find(|(path, _)| path == "include/box2d/box2d.h")
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .ok_or("box2d.h is missing from the public header set")?;

    let simd = active_simd_identity();
    let validate = validation_enabled();
    let mut files = BTreeMap::new();
    files.insert(format!("lib/{lib_name}"), lib.clone());
    for (path, bytes) in headers {
        files.insert(path, bytes);
    }
    files.insert(format!("bindings/{binding_name}"), binding.clone());
    files.insert("metadata/upstream.toml".to_owned(), upstream.clone());
    files.insert(
        "metadata/effective-source.toml".to_owned(),
        effective_source_manifest,
    );
    files.insert(
        "licenses/PROJECT-LICENSE-MIT".to_owned(),
        read_required(&manifest_dir.join("../LICENSE-MIT"), "MIT license")?,
    );
    files.insert(
        "licenses/PROJECT-LICENSE-APACHE".to_owned(),
        read_required(&manifest_dir.join("../LICENSE-APACHE"), "Apache license")?,
    );
    files.insert(
        "licenses/BOX2D-LICENSE".to_owned(),
        read_required(
            &manifest_dir.join("third-party/box2d/LICENSE"),
            "upstream Box2D license",
        )?,
    );

    let manifest = ArtifactManifest {
        schema_version: provider_manifest::SCHEMA_VERSION,
        schema: provider_manifest::SCHEMA_NAME.to_owned(),
        provider: "prebuilt".to_owned(),
        crate_version: crate_version.to_owned(),
        source_commit: Some(source_commit.to_owned()),
        release_tag: Some(release_tag.to_owned()),
        upstream_sha: upstream_sha.clone(),
        effective_source_sha256: effective_source_sha256.to_owned(),
        precision: precision.to_owned(),
        target: target.to_owned(),
        link: "static".to_owned(),
        crt: crt.to_owned(),
        simd: simd.to_owned(),
        validate,
        adapter_abi_version: ADAPTER_ABI_VERSION,
        adapter_source_sha256: adapter_source_sha256.to_owned(),
        private_abi_hash: private_abi_hash.to_owned(),
        snapshot_layout_hash: u64::from(snapshot_layout_hash),
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
        required_adapter_symbols_sha256: required_adapter_symbols_sha256(),
        required_adapter_symbols: REQUIRED_ADAPTER_SYMBOLS
            .iter()
            .map(|symbol| (*symbol).to_owned())
            .collect(),
        archive: format!("lib/{lib_name}"),
        archive_sha256: sha256_bytes(&lib),
        header: header_path,
        header_sha256: sha256_bytes(&header_bytes),
        bindings: format!("bindings/{binding_name}"),
        bindings_sha256: sha256_bytes(&binding),
    };
    let manifest_bytes = manifest.render();
    // Re-parse and validate before packaging so malformed or self-inconsistent generated output
    // cannot enter the archive.
    let generated = ArtifactManifest::parse(&manifest_bytes)
        .map_err(|error| format!("generated provider manifest is invalid: {error}"))?;
    generated
        .validate_identity(&ArtifactIdentityExpectation {
            provider: "prebuilt",
            crate_version,
            upstream_sha: &upstream_sha,
            effective_source_sha256,
            precision,
            target,
            crt,
            simd,
            validate,
            adapter_source_sha256,
            private_abi_hash,
            snapshot_layout_hash,
        })
        .map_err(|error| format!("generated provider manifest is inconsistent: {error}"))?;
    files.insert("manifest.toml".to_owned(), manifest_bytes);

    let checksums = files
        .iter()
        .map(|(path, bytes)| format!("{}  {path}\n", sha256_bytes(bytes)))
        .collect::<String>();
    files.insert("checksums.sha256".to_owned(), checksums.into_bytes());
    Ok(files)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    if let Some(command) = args.next() {
        match command.to_str() {
            Some("trust-local-system") => {
                let input = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("trust-local-system requires an input manifest path")?;
                let output = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("trust-local-system requires an output manifest path")?;
                if args.next().is_some() {
                    return Err("trust-local-system accepts exactly two paths".into());
                }
                write_caller_trusted_system_manifest(&input, &output)?;
                println!(
                    "Caller-trusted system manifest created: {}",
                    output.display()
                );
            }
            Some("attest-local-system") => {
                let archive = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("attest-local-system requires an archive path")?;
                let header = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("attest-local-system requires a header path")?;
                let bindings = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("attest-local-system requires a bindings path")?;
                let output = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("attest-local-system requires an output manifest path")?;
                if args.next().is_some() {
                    return Err("attest-local-system accepts exactly four paths".into());
                }
                attest_local_system(&archive, &header, &bindings, &output)?;
                println!(
                    "Caller-trusted system manifest created: {}",
                    output.display()
                );
            }
            _ => {
                return Err(format!(
                    "unknown package command {:?}; expected `trust-local-system` or `attest-local-system`",
                    command
                )
                .into());
            }
        }
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir.parent().ok_or("workspace root is missing")?;
    let target = default_target_triple();
    let crate_version = env::var("CARGO_PKG_VERSION")?;
    let source_commit = package_source_commit()?;
    let release_tag = package_release_tag(&crate_version)?;
    let target_env = active_target_env();
    let target_os = active_target_os();
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let crt = detect_crt(
        &target_os,
        &target_env,
        &target_features,
        &env::var("BOXDD_SYS_PACKAGE_CRT").unwrap_or_default(),
    )?;
    let precision = precision();
    let simd = active_simd_identity();
    let validate = validation_enabled();
    let adapter_source_sha256 = adapter_source_sha256(&manifest_dir)?;
    let upstream = read_required(&manifest_dir.join("upstream.toml"), "upstream manifest")?;
    let upstream_sha = parse_upstream_sha(&upstream)?;
    let effective_source = effective_source_identity(&manifest_dir)?;
    let private_abi_hash = private_abi_hash_hex();
    let sys_out = locate_sys_out_dir(
        workspace_root,
        &target_env,
        &BuildOutputExpectation {
            crate_version: &crate_version,
            target: &target,
            precision,
            crt,
            simd,
            validate,
            provider: "vendored",
            upstream_sha: &upstream_sha,
            effective_source_sha256: &effective_source.effective_source_sha256,
            adapter_source_sha256: &adapter_source_sha256,
            private_abi_hash: &boxdd_sys::PRIVATE_ABI_HASH,
            snapshot_layout_hash: boxdd_sys::SNAPSHOT_LAYOUT_HASH,
        },
    )?;
    let files = build_package_files(PackageRequest {
        manifest_dir: &manifest_dir,
        sys_out: &sys_out,
        target: &target,
        target_env: &target_env,
        crt,
        precision,
        crate_version: &crate_version,
        source_commit: &source_commit,
        release_tag: &release_tag,
        effective_source_sha256: &effective_source.effective_source_sha256,
        adapter_source_sha256: &adapter_source_sha256,
        private_abi_hash: &private_abi_hash,
        snapshot_layout_hash: boxdd_sys::SNAPSHOT_LAYOUT_HASH,
    })?;

    let package_dir = env::var_os("BOXDD_SYS_PACKAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("packages"));
    fs::create_dir_all(&package_dir)?;
    let archive_name =
        compose_archive_name("boxdd", &crate_version, &target, precision, "static", crt);
    let output = package_dir.join(archive_name);
    let file = fs::File::create(&output)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for (path, bytes) in &files {
        append_bytes(&mut tar, path, bytes)?;
    }
    tar.finish()?;
    println!(
        "Package created: {} ({} files)",
        output.display(),
        files.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn current_built_archive() -> PathBuf {
        PathBuf::from(env!("OUT_DIR")).join(expected_lib_name(&active_target_env()))
    }

    #[test]
    fn archive_name_includes_windows_crt_suffix() {
        assert_eq!(
            compose_archive_name(
                "boxdd",
                "0.6.0",
                "x86_64-pc-windows-msvc",
                "single",
                "static",
                "mt"
            ),
            "boxdd-prebuilt-0.6.0-x86_64-pc-windows-msvc-single-static-mt.tar.gz"
        );
    }

    #[test]
    fn cargo_cfg_detects_windows_crt_and_native_none() {
        assert_eq!(
            detect_crt("windows", "msvc", "crt-static,sse2", "").unwrap(),
            "mt"
        );
        assert_eq!(detect_crt("windows", "msvc", "sse2", "").unwrap(), "md");
        assert_eq!(detect_crt("linux", "gnu", "", "").unwrap(), "none");
    }

    #[test]
    fn package_simd_identity_matches_target_compiler_defines() {
        assert_eq!(package_simd_identity("wasm32", false, true), "disabled");
        assert_eq!(package_simd_identity("x86_64", false, true), "avx2");
        assert_eq!(package_simd_identity("aarch64", false, true), "default");
        assert_eq!(package_simd_identity("x86_64", true, true), "disabled");
    }

    #[test]
    fn package_output_identity_includes_crt_simd_and_validation() {
        let directory = tempdir().unwrap();
        let adapter_source_sha256 = "a".repeat(64);
        let effective_source_sha256 = "9".repeat(64);
        let upstream_sha = "56edae79f2949d86142b03450d5d60f63bcf5a6f";
        let private_abi_hash = [0xAB; 32];
        let snapshot_layout_hash = 0x1234_5678;
        let expected = BuildOutputExpectation {
            crate_version: "0.6.0",
            target: "x86_64-pc-windows-msvc",
            precision: "single",
            crt: "mt",
            simd: "avx2",
            validate: true,
            provider: "vendored",
            upstream_sha,
            effective_source_sha256: &effective_source_sha256,
            adapter_source_sha256: &adapter_source_sha256,
            private_abi_hash: &private_abi_hash,
            snapshot_layout_hash,
        };
        let identity = expected_build_identity_marker(&expected);
        let schema_less = identity.replacen("schema_version = 2\n", "", 1);
        fs::write(directory.path().join(BUILD_IDENTITY_FILE), schema_less).unwrap();
        fs::write(
            directory.path().join(ADAPTER_IDENTITY_FILE),
            adapter_identity_source(&private_abi_hash, snapshot_layout_hash),
        )
        .unwrap();
        assert!(
            !identity_matches(directory.path(), &expected),
            "a schema-less build marker must fail closed"
        );

        for malformed in [
            identity.replacen(
                &format!("effective_source_sha256 = {effective_source_sha256:?}\n"),
                "",
                1,
            ),
            format!("{identity}unreviewed = true\n"),
        ] {
            fs::write(directory.path().join(BUILD_IDENTITY_FILE), malformed).unwrap();
            assert!(
                !identity_matches(directory.path(), &expected),
                "a non-canonical build marker must fail closed"
            );
        }

        fs::write(directory.path().join(BUILD_IDENTITY_FILE), &identity).unwrap();
        assert!(identity_matches(directory.path(), &expected));

        for drifted in [
            identity.replace("crt = \"mt\"", "crt = \"md\""),
            identity.replace("simd = \"avx2\"", "simd = \"default\""),
            identity.replace("validate = true", "validate = false"),
            identity.replace(upstream_sha, "0000000000000000000000000000000000000000"),
            identity.replace(&effective_source_sha256, &"0".repeat(64)),
        ] {
            fs::write(directory.path().join(BUILD_IDENTITY_FILE), drifted).unwrap();
            assert!(!identity_matches(directory.path(), &expected));
        }

        fs::write(directory.path().join(BUILD_IDENTITY_FILE), &identity).unwrap();
        fs::write(
            directory.path().join(ADAPTER_IDENTITY_FILE),
            adapter_identity_source(&[0xCD; 32], snapshot_layout_hash),
        )
        .unwrap();
        assert!(!identity_matches(directory.path(), &expected));
    }

    #[test]
    fn header_layout_is_flat_under_public_include_root() {
        let directory = tempdir().unwrap();
        let root = directory.path().join("box2d");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("box2d.h"), b"root").unwrap();
        fs::write(root.join("nested/types.h"), b"nested").unwrap();
        let headers = collect_headers(&root).unwrap();
        assert_eq!(headers[0].0, "include/box2d/box2d.h");
        assert_eq!(headers[1].0, "include/box2d/nested/types.h");
        assert!(
            headers
                .iter()
                .all(|(path, _)| !path.contains("box2d/box2d/"))
        );
    }

    #[test]
    fn generated_manifest_contains_exact_file_digests() {
        let directory = tempdir().unwrap();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let out = PathBuf::from(env!("OUT_DIR"));
        let target = default_target_triple();
        let target_env = active_target_env();
        let private_abi_hash = private_abi_hash_hex();
        let adapter_source_sha256 = adapter_source_sha256(&manifest_dir).unwrap();
        let effective_source = effective_source_identity(&manifest_dir).unwrap();
        let files = build_package_files(PackageRequest {
            manifest_dir: &manifest_dir,
            sys_out: &out,
            target: &target,
            target_env: &target_env,
            crt: "none",
            precision: "single",
            crate_version: "0.6.0",
            source_commit: "1234567890abcdef1234567890abcdef12345678",
            release_tag: "v0.6.0",
            effective_source_sha256: &effective_source.effective_source_sha256,
            adapter_source_sha256: &adapter_source_sha256,
            private_abi_hash: &private_abi_hash,
            snapshot_layout_hash: boxdd_sys::SNAPSHOT_LAYOUT_HASH,
        })
        .unwrap();
        let manifest = ArtifactManifest::parse(files.get("manifest.toml").unwrap()).unwrap();
        assert_eq!(
            manifest.archive_sha256,
            sha256_file(&current_built_archive()).unwrap()
        );
        assert_eq!(
            manifest.source_commit.as_deref(),
            Some("1234567890abcdef1234567890abcdef12345678")
        );
        assert_eq!(manifest.release_tag.as_deref(), Some("v0.6.0"));
        assert_eq!(manifest.adapter_abi_version, ADAPTER_ABI_VERSION);
        assert_eq!(manifest.adapter_source_sha256, adapter_source_sha256);
        assert_eq!(
            manifest.effective_source_sha256,
            effective_source.effective_source_sha256
        );
        assert_eq!(
            manifest.recording_contract_blake3,
            RECORDING_CONTRACT_BLAKE3
        );
        assert_eq!(
            manifest
                .required_adapter_symbols
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            REQUIRED_ADAPTER_SYMBOLS
        );
        assert_eq!(
            manifest.header_sha256,
            sha256_file(&manifest_dir.join("third-party/box2d/include/box2d/box2d.h")).unwrap()
        );
        assert!(files.contains_key("metadata/upstream.toml"));
        assert_eq!(
            files.get("metadata/effective-source.toml").unwrap(),
            &fs::read(manifest_dir.join("effective-source.toml")).unwrap()
        );
        assert!(files.contains_key("licenses/BOX2D-LICENSE"));
        assert!(files.contains_key("checksums.sha256"));
        assert_eq!(
            sha256_file(&out.join("libbox2d.a")).unwrap(),
            manifest.archive_sha256
        );

        let release_manifest = directory.path().join("manifest.toml");
        let system_manifest = directory.path().join("system-manifest.toml");
        fs::write(&release_manifest, files.get("manifest.toml").unwrap()).unwrap();
        let packaged_archive = directory.path().join(&manifest.archive);
        fs::create_dir_all(packaged_archive.parent().unwrap()).unwrap();
        fs::write(
            &packaged_archive,
            files
                .get(&manifest.archive)
                .expect("package contains archive"),
        )
        .unwrap();
        write_caller_trusted_system_manifest(&release_manifest, &system_manifest).unwrap();
        let system = ArtifactManifest::parse(&fs::read(system_manifest).unwrap()).unwrap();
        assert_eq!(system.provider, "system");
        assert_eq!(system.source_commit, None);
        assert_eq!(system.release_tag, None);
    }

    #[test]
    fn package_refuses_missing_or_tampered_effective_source_metadata() {
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let effective_source = effective_source_identity(&repository).unwrap();
        let directory = tempdir().unwrap();
        let manifest_dir = directory.path().join("boxdd-sys");
        let out = directory.path().join("out");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(&out).unwrap();
        fs::copy(
            repository.join("upstream.toml"),
            manifest_dir.join("upstream.toml"),
        )
        .unwrap();

        let package = || {
            build_package_files(PackageRequest {
                manifest_dir: &manifest_dir,
                sys_out: &out,
                target: "x86_64-unknown-linux-gnu",
                target_env: "gnu",
                crt: "none",
                precision: "single",
                crate_version: "0.6.0",
                source_commit: "1234567890abcdef1234567890abcdef12345678",
                release_tag: "v0.6.0",
                effective_source_sha256: &effective_source.effective_source_sha256,
                adapter_source_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                private_abi_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                snapshot_layout_hash: 0,
            })
        };

        assert!(
            package().is_err(),
            "missing effective-source.toml was accepted"
        );

        let tampered = fs::read_to_string(repository.join("effective-source.toml"))
            .unwrap()
            .replace(
                &effective_source.upstream_sha,
                "0000000000000000000000000000000000000000",
            );
        fs::write(manifest_dir.join("effective-source.toml"), tampered).unwrap();
        assert!(
            package().is_err(),
            "tampered effective-source.toml was accepted"
        );
    }

    #[test]
    fn local_system_attestation_proves_the_archive_and_refuses_overwrite() {
        let directory = tempdir().unwrap();
        let archive = directory
            .path()
            .join(expected_lib_name(if cfg!(target_env = "msvc") {
                "msvc"
            } else {
                ""
            }));
        let header = directory.path().join("box2d.h");
        let bindings = directory.path().join("bindings.rs");
        let manifest_path = directory.path().join("manifest.toml");
        fs::copy(current_built_archive(), &archive).unwrap();
        fs::write(&header, b"header").unwrap();
        fs::write(&bindings, b"bindings").unwrap();

        attest_local_system(&archive, &header, &bindings, &manifest_path).unwrap();
        let manifest = ArtifactManifest::parse(&fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.provider, "system");
        assert_eq!(
            manifest.precision,
            if cfg!(feature = "double-precision") {
                "double"
            } else {
                "single"
            }
        );
        assert_eq!(
            manifest.simd,
            if cfg!(target_arch = "wasm32") || cfg!(feature = "disable-simd") {
                "disabled"
            } else if cfg!(target_arch = "x86_64") && cfg!(feature = "simd-avx2") {
                "avx2"
            } else {
                "default"
            }
        );
        assert_eq!(manifest.validate, cfg!(feature = "validate"));
        assert_eq!(manifest.adapter_abi_version, ADAPTER_ABI_VERSION);
        assert_eq!(
            manifest.recording_contract_blake3,
            RECORDING_CONTRACT_BLAKE3
        );
        assert!(attest_local_system(&archive, &header, &bindings, &manifest_path).is_err());
    }

    #[test]
    fn local_system_attestation_rejects_non_archive_bytes() {
        let directory = tempdir().unwrap();
        let archive = directory
            .path()
            .join(expected_lib_name(&active_target_env()));
        let header = directory.path().join("box2d.h");
        let bindings = directory.path().join("bindings.rs");
        let manifest_path = directory.path().join("manifest.toml");
        fs::write(&archive, b"archive").unwrap();
        fs::write(&header, b"header").unwrap();
        fs::write(&bindings, b"bindings").unwrap();

        let error = attest_local_system(&archive, &header, &bindings, &manifest_path)
            .expect_err("non-archive input must fail closed");
        assert!(error.to_string().contains("not a supported static archive"));
        assert!(!manifest_path.exists());
    }
}
