//! Build or attest deterministic native Box2D provider artifacts.
//!
//! The command consumes the exact build identity emitted by `boxdd-sys/build.rs`. It never scans
//! Cargo fingerprints or infers target ABI state from the host `xtask` compilation.

use crate::build_identity::{
    BUILD_IDENTITY_FILE, BuildIdentity, validate_sha256, validate_source_commit,
};
use crate::build_support::{
    VerifiedFileSnapshot, generate_file_create_new, snapshot_file_create_new,
};
use crate::prebuilt_provenance;
use crate::provenance_policy::release_tag_matches_version;
use crate::provider_archive::{ArchiveExpectation, verify_provider_archive};
use crate::provider_catalog::ProviderCapability;
use crate::provider_manifest::{
    self, ADAPTER_ABI_VERSION, ArtifactExpectation, ArtifactIdentityExpectation, ArtifactManifest,
    MAX_PROVIDER_ARCHIVE_BYTES, MAX_PROVIDER_BINDINGS_BYTES, MAX_PROVIDER_HEADER_BYTES,
    MAX_PROVIDER_MANIFEST_BYTES, RECORDING_CONTRACT_BLAKE3, REQUIRED_ADAPTER_SYMBOLS,
    required_adapter_symbols_sha256, sha256_bytes,
};
use crate::source_overlay::{
    adapter_source_sha256, effective_source_identity, materialize_effective_box2d_sources,
};
use crate::{Error, Result as XtaskResult};
use flate2::{Compression, write::GzEncoder};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const ADAPTER_IDENTITY_FILE: &str = "adapter_identity.rs";
const MAX_PACKAGE_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACKAGE_PUBLIC_HEADERS: usize = 64;
const MAX_PACKAGE_PUBLIC_HEADER_BYTES: u64 = 64 * 1024 * 1024;
type PackageFiles = BTreeMap<String, Vec<u8>>;
type HeaderFiles = Vec<(String, Vec<u8>)>;

struct PackageRequest<'a> {
    manifest_dir: &'a Path,
    sys_out: &'a Path,
    identity: &'a BuildIdentity,
    source_commit: &'a str,
    release_tag: &'a str,
}

struct ExplicitBuildIdentity {
    identity: BuildIdentity,
    marker: VerifiedFileSnapshot,
    adapter: VerifiedFileSnapshot,
}

fn expected_lib_name(target_env: &str) -> &'static str {
    if target_env == "msvc" {
        "box2d.lib"
    } else {
        "libbox2d.a"
    }
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
    if !is_lower_hex(sha, 40) {
        return Err("upstream.toml active_revision must be a lowercase Git SHA".to_owned());
    }
    Ok(sha.to_owned())
}

fn parse_public_header_inventory(upstream: &[u8]) -> Result<Vec<String>, String> {
    let value: toml::Value = toml::from_str(
        std::str::from_utf8(upstream)
            .map_err(|error| format!("upstream.toml is not UTF-8: {error}"))?,
    )
    .map_err(|error| format!("upstream.toml is not valid TOML: {error}"))?;
    value
        .get("source_inventory")
        .and_then(|value| value.get("public_headers"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "upstream.toml has no source_inventory.public_headers array".to_owned())?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                "upstream.toml source_inventory.public_headers must contain only strings".to_owned()
            })
        })
        .collect()
}

fn validate_release_tag(crate_version: &str, tag: &str) -> Result<(), String> {
    if !release_tag_matches_version(crate_version, tag) {
        return Err(format!(
            "prebuilt package release tag `{tag}` does not match crate version {crate_version}"
        ));
    }
    Ok(())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

fn decode_sha256(value: &str) -> Result<[u8; 32], String> {
    validate_sha256("private_abi_hash", value, false)?;
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("invalid private ABI hash: {error}"))?;
    }
    Ok(bytes)
}

impl ExplicitBuildIdentity {
    fn load(path: &Path) -> Result<Self, String> {
        if path.file_name().and_then(|name| name.to_str()) != Some(BUILD_IDENTITY_FILE) {
            return Err(format!(
                "explicit build identity must be named {BUILD_IDENTITY_FILE}: {}",
                path.display()
            ));
        }
        let marker = VerifiedFileSnapshot::read(
            path,
            MAX_PACKAGE_METADATA_BYTES,
            "explicit build identity marker",
        )?;
        let identity = BuildIdentity::parse(marker.bytes())?;
        identity.require_native()?;
        let out_dir = path
            .parent()
            .ok_or_else(|| "build identity path has no parent directory".to_owned())?;
        let adapter = VerifiedFileSnapshot::read(
            &out_dir.join(ADAPTER_IDENTITY_FILE),
            MAX_PACKAGE_METADATA_BYTES,
            "adapter identity marker",
        )?;
        let private_abi_hash = decode_sha256(&identity.private_abi_hash)?;
        let expected = adapter_identity_source(&private_abi_hash, identity.snapshot_layout_hash);
        if adapter.bytes() != expected.as_bytes() {
            return Err(
                "adapter identity marker does not match the explicit build identity".to_owned(),
            );
        }
        Ok(Self {
            identity,
            marker,
            adapter,
        })
    }

    fn require_out_dir(&self, sys_out: &Path) -> Result<(), String> {
        let expected = fs::canonicalize(sys_out).map_err(|error| {
            format!(
                "failed to canonicalize sys output {}: {error}",
                sys_out.display()
            )
        })?;
        let actual = fs::canonicalize(
            self.marker
                .path()
                .parent()
                .ok_or_else(|| "build identity path has no parent directory".to_owned())?,
        )
        .map_err(|error| format!("failed to canonicalize build identity parent: {error}"))?;
        if actual != expected {
            return Err(format!(
                "build identity belongs to {}, not explicit sys output {}",
                actual.display(),
                expected.display()
            ));
        }
        Ok(())
    }

    fn revalidate(&self) -> Result<(), String> {
        self.marker
            .revalidate("native package build identity cohort")?;
        self.adapter
            .revalidate("native package adapter identity cohort")
    }
}

fn collect_headers(
    source_root: &Path,
    inventory: &[String],
) -> Result<HeaderFiles, Box<dyn std::error::Error>> {
    if inventory.is_empty() || inventory.len() > MAX_PACKAGE_PUBLIC_HEADERS {
        return Err(format!(
            "public header inventory must contain 1..={MAX_PACKAGE_PUBLIC_HEADERS} entries"
        )
        .into());
    }
    let mut files = Vec::with_capacity(inventory.len());
    let mut total_bytes = 0_u64;
    let mut previous = None::<&str>;
    for relative in inventory {
        validate_public_header_path(relative)?;
        if previous.is_some_and(|previous| previous >= relative.as_str()) {
            return Err(
                "public header inventory must be strictly sorted without duplicates".into(),
            );
        }
        previous = Some(relative.as_str());
        let path = source_root.join(relative);
        let snapshot = VerifiedFileSnapshot::read(
            &path,
            MAX_PROVIDER_HEADER_BYTES,
            "reviewed public Box2D header",
        )?;
        let header_bytes = u64::try_from(snapshot.len())
            .map_err(|_| "public header length does not fit in u64")?;
        total_bytes = total_bytes
            .checked_add(header_bytes)
            .ok_or("public header aggregate byte length overflow")?;
        if total_bytes > MAX_PACKAGE_PUBLIC_HEADER_BYTES {
            return Err(format!(
                "public headers exceed the {MAX_PACKAGE_PUBLIC_HEADER_BYTES} byte aggregate limit"
            )
            .into());
        }
        files.push((relative.clone(), snapshot.into_bytes()));
    }
    Ok(files)
}

fn validate_public_header_path(relative: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(relative);
    if relative.contains('\\')
        || !relative.starts_with("include/box2d/")
        || path.extension().and_then(|extension| extension.to_str()) != Some("h")
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("invalid public header inventory path {relative:?}").into());
    }
    Ok(())
}

fn read_required(
    path: &Path,
    label: &str,
    maximum_bytes: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(VerifiedFileSnapshot::read(path, maximum_bytes, &format!("required {label}"))?.into_bytes())
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

fn validate_prebuilt_member_limits<'a>(
    members: impl IntoIterator<Item = (&'a str, u64)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut member_count = 0_usize;
    let mut total_bytes = 0_u64;
    for (path, size) in members {
        member_count = member_count
            .checked_add(1)
            .ok_or("prebuilt package member count overflowed usize")?;
        if member_count > prebuilt_provenance::MAX_MEMBERS {
            return Err(format!(
                "prebuilt package exceeds the {} member protocol limit",
                prebuilt_provenance::MAX_MEMBERS
            )
            .into());
        }
        if size > prebuilt_provenance::MAX_MEMBER_BYTES {
            return Err(format!(
                "prebuilt package member {path:?} exceeds the {} byte protocol limit",
                prebuilt_provenance::MAX_MEMBER_BYTES
            )
            .into());
        }
        total_bytes = total_bytes
            .checked_add(size)
            .filter(|total| *total <= prebuilt_provenance::MAX_TOTAL_MEMBER_BYTES)
            .ok_or_else(|| {
                format!(
                    "prebuilt package members exceed the {} byte aggregate protocol limit",
                    prebuilt_provenance::MAX_TOTAL_MEMBER_BYTES
                )
            })?;
    }
    if member_count == 0 {
        return Err("prebuilt package must contain at least one member".into());
    }
    Ok(())
}

fn write_caller_trusted_system_manifest(
    manifest_dir: &Path,
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
    let manifest_snapshot = VerifiedFileSnapshot::read(
        prebuilt_manifest,
        MAX_PROVIDER_MANIFEST_BYTES,
        "prebuilt provider manifest",
    )?;
    let manifest = ArtifactManifest::parse(manifest_snapshot.bytes())?;
    if manifest.provider != ProviderCapability::Prebuilt.as_str()
        || manifest.source_commit.is_none()
        || manifest.release_tag.is_none()
    {
        return Err("input is not a release-qualified prebuilt provider manifest".into());
    }
    let workspace_root = manifest_dir
        .parent()
        .ok_or("boxdd-sys manifest directory has no workspace parent")?;
    let repository_version = workspace_version(workspace_root)?;
    if manifest.crate_version != repository_version {
        return Err(format!(
            "input manifest crate version {} does not match workspace version {repository_version}",
            manifest.crate_version
        )
        .into());
    }
    let effective_source = effective_source_identity(manifest_dir)?;
    let expected_adapter_source_sha256 = adapter_source_sha256(manifest_dir)?;
    if manifest.upstream_sha != effective_source.upstream_sha {
        return Err(
            "input manifest upstream SHA does not match the repository effective source".into(),
        );
    }
    if manifest.adapter_source_sha256 != expected_adapter_source_sha256 {
        return Err("input manifest adapter source does not match the repository contract".into());
    }
    let materialization = tempfile::tempdir()?;
    let effective_sources =
        materialize_effective_box2d_sources(manifest_dir, materialization.path())?;
    let header_path = effective_sources.public_include.join("box2d/box2d.h");
    let bindings_path = binding_path(manifest_dir, &manifest.precision);
    let verified = provider_manifest::verify_artifact(
        prebuilt_manifest,
        &ArtifactExpectation {
            identity: ArtifactIdentityExpectation {
                provider: ProviderCapability::Prebuilt.as_str(),
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
            },
            header_path: &header_path,
            bindings_path: &bindings_path,
        },
    )?;
    let verified_archive = verify_provider_archive(
        &verified.archive_snapshot,
        &ArchiveExpectation {
            target: &verified.manifest.target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &verified.manifest.private_abi_hash,
            snapshot_layout_hash: u32::try_from(verified.manifest.snapshot_layout_hash)?,
        },
    )?;
    if verified_archive.archive_sha256 != verified.manifest.archive_sha256 {
        return Err("input manifest archive digest does not match its bound file".into());
    }
    let provider_manifest::VerifiedArtifact {
        mut manifest,
        manifest_snapshot,
        archive_snapshot,
        header_snapshot,
        bindings_snapshot,
    } = verified;
    manifest.provider = ProviderCapability::System.as_str().to_owned();
    manifest.source_commit = None;
    manifest.release_tag = None;
    manifest_snapshot.revalidate("caller-trusted manifest conversion cohort")?;
    archive_snapshot.revalidate("caller-trusted archive conversion cohort")?;
    header_snapshot.revalidate("caller-trusted header conversion cohort")?;
    bindings_snapshot.revalidate("caller-trusted bindings conversion cohort")?;
    write_new_manifest(output, &manifest.render())?;
    Ok(())
}

fn workspace_version(workspace_root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest = read_required(
        &workspace_root.join("Cargo.toml"),
        "workspace manifest",
        MAX_PACKAGE_METADATA_BYTES,
    )?;
    let value: toml::Value = toml::from_str(std::str::from_utf8(&manifest)?)?;
    value
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "workspace.package.version is missing".into())
}

fn validate_repository_identity(
    workspace_root: &Path,
    identity: &BuildIdentity,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = workspace_root.join("boxdd-sys");
    let upstream = read_required(
        &manifest_dir.join("upstream.toml"),
        "upstream manifest",
        MAX_PACKAGE_METADATA_BYTES,
    )?;
    let upstream_sha = parse_upstream_sha(&upstream)?;
    let effective_source = effective_source_identity(&manifest_dir)?;
    let adapter_sha256 = adapter_source_sha256(&manifest_dir)?;
    let version = workspace_version(workspace_root)?;
    if identity.upstream_sha != upstream_sha
        || identity.upstream_sha != effective_source.upstream_sha
        || identity.effective_source_sha256 != effective_source.effective_source_sha256
        || identity.adapter_source_sha256 != adapter_sha256
        || identity.crate_version != version
    {
        return Err(
            "explicit build identity does not match the current repository source contract".into(),
        );
    }
    Ok(())
}

fn write_new_manifest(output: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    generate_file_create_new(
        output,
        MAX_PROVIDER_MANIFEST_BYTES,
        "caller-trusted system manifest",
        |file| {
            file.write_all(bytes)
                .map_err(|error| format!("failed to write caller-trusted system manifest: {error}"))
        },
    )?;
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

fn validate_system_attestation_archive(
    identity: &BuildIdentity,
    archive: &VerifiedFileSnapshot,
) -> Result<(), String> {
    if !matches!(
        identity.provider,
        ProviderCapability::Vendored | ProviderCapability::System
    ) {
        return Err(format!(
            "local system attestation requires a vendored or system build identity, found {}",
            identity.provider.as_str()
        ));
    }
    if archive.sha256() != identity.archive_sha256 {
        return Err(
            "caller-trusted archive does not match the exact explicit build identity".to_owned(),
        );
    }
    Ok(())
}

fn attest_local_system(
    workspace_root: &Path,
    build_identity: &ExplicitBuildIdentity,
    archive: &Path,
    header_output: &Path,
    bindings: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity = &build_identity.identity;
    validate_repository_identity(workspace_root, identity)?;
    let output_root = output
        .parent()
        .ok_or("system manifest output has no parent directory")?;
    let archive_snapshot = VerifiedFileSnapshot::read(
        archive,
        MAX_PROVIDER_ARCHIVE_BYTES,
        "caller-trusted provider archive",
    )?;
    validate_system_attestation_archive(identity, &archive_snapshot)?;
    let verified_archive = verify_provider_archive(
        &archive_snapshot,
        &ArchiveExpectation {
            target: &identity.target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &identity.effective_source_sha256,
            private_abi_hash: &identity.private_abi_hash,
            snapshot_layout_hash: identity.snapshot_layout_hash,
        },
    )?;
    let archive_path = artifact_relative_path(output_root, archive, "archive")?;
    let bindings_path = artifact_relative_path(output_root, bindings, "bindings")?;
    let bindings_snapshot = VerifiedFileSnapshot::read(
        bindings,
        MAX_PROVIDER_BINDINGS_BYTES,
        "caller-trusted provider bindings",
    )?;
    if bindings_snapshot.sha256() != identity.bindings_sha256 {
        return Err("caller-trusted bindings do not match the explicit build identity".into());
    }
    archive_snapshot.revalidate("caller-trusted archive attestation cohort")?;
    bindings_snapshot.revalidate("caller-trusted bindings attestation cohort")?;
    build_identity.revalidate()?;

    let materialization = tempfile::tempdir()?;
    let effective_sources = materialize_effective_box2d_sources(
        &workspace_root.join("boxdd-sys"),
        materialization.path(),
    )?;
    let header_snapshot = snapshot_file_create_new(
        &effective_sources.public_include.join("box2d/box2d.h"),
        header_output,
        MAX_PROVIDER_HEADER_BYTES,
        "effective public Box2D header",
    )?;

    let publication = (|| -> Result<(), Box<dyn std::error::Error>> {
        let header_path = artifact_relative_path(output_root, header_output, "header")?;
        let manifest = ArtifactManifest {
            schema_version: provider_manifest::SCHEMA_VERSION,
            schema: provider_manifest::SCHEMA_NAME.to_owned(),
            provider: ProviderCapability::System.as_str().to_owned(),
            crate_version: identity.crate_version.clone(),
            source_commit: None,
            release_tag: None,
            upstream_sha: identity.upstream_sha.clone(),
            effective_source_sha256: identity.effective_source_sha256.clone(),
            precision: identity.precision.clone(),
            target: identity.target.clone(),
            link: "static".to_owned(),
            crt: identity.crt.clone(),
            simd: identity.simd.clone(),
            validate: identity.validate,
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: identity.adapter_source_sha256.clone(),
            private_abi_hash: identity.private_abi_hash.clone(),
            snapshot_layout_hash: u64::from(identity.snapshot_layout_hash),
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            required_adapter_symbols_sha256: required_adapter_symbols_sha256(),
            required_adapter_symbols: REQUIRED_ADAPTER_SYMBOLS
                .iter()
                .map(|symbol| (*symbol).to_owned())
                .collect(),
            archive: archive_path,
            archive_sha256: verified_archive.archive_sha256,
            header: header_path,
            header_sha256: header_snapshot.sha256().to_owned(),
            bindings: bindings_path,
            bindings_sha256: bindings_snapshot.sha256().to_owned(),
        };
        manifest.validate_identity(&ArtifactIdentityExpectation {
            provider: ProviderCapability::System.as_str(),
            crate_version: &manifest.crate_version,
            upstream_sha: &manifest.upstream_sha,
            effective_source_sha256: &identity.effective_source_sha256,
            precision: &manifest.precision,
            target: &manifest.target,
            crt: &manifest.crt,
            simd: &manifest.simd,
            validate: manifest.validate,
            adapter_source_sha256: &manifest.adapter_source_sha256,
            private_abi_hash: &identity.private_abi_hash,
            snapshot_layout_hash: identity.snapshot_layout_hash,
        })?;
        archive_snapshot.revalidate("caller-trusted archive attestation cohort")?;
        header_snapshot.revalidate("effective header attestation cohort")?;
        bindings_snapshot.revalidate("caller-trusted bindings attestation cohort")?;
        build_identity.revalidate()?;
        write_new_manifest(output, &manifest.render())?;
        Ok(())
    })();

    if let Err(error) = publication {
        return match fs::remove_file(header_output) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; failed to remove generated header {}: {cleanup_error}",
                header_output.display()
            )
            .into()),
        };
    }
    Ok(())
}

fn build_package_files(
    request: PackageRequest<'_>,
) -> Result<PackageFiles, Box<dyn std::error::Error>> {
    let PackageRequest {
        manifest_dir,
        sys_out,
        identity,
        source_commit,
        release_tag,
    } = request;
    let upstream_path = manifest_dir.join("upstream.toml");
    let upstream_snapshot = VerifiedFileSnapshot::read(
        &upstream_path,
        MAX_PACKAGE_METADATA_BYTES,
        "required upstream manifest",
    )?;
    let upstream_sha = parse_upstream_sha(upstream_snapshot.bytes())?;
    let public_header_inventory = parse_public_header_inventory(upstream_snapshot.bytes())?;
    let effective_source_manifest_path = manifest_dir.join("effective-source.toml");
    let effective_source_manifest_snapshot = VerifiedFileSnapshot::read(
        &effective_source_manifest_path,
        MAX_PACKAGE_METADATA_BYTES,
        "required effective source manifest",
    )?;
    let source_materialization = tempfile::tempdir()?;
    let effective_sources =
        materialize_effective_box2d_sources(manifest_dir, source_materialization.path())?;
    if effective_sources.identity.upstream_sha != upstream_sha
        || effective_sources.identity.effective_source_sha256 != identity.effective_source_sha256
    {
        return Err("package request does not match the captured effective-source identity".into());
    }
    let target_env = if identity.target.ends_with("-msvc") {
        "msvc"
    } else {
        ""
    };
    let lib_name = expected_lib_name(target_env);
    let library_path = sys_out.join(lib_name);
    let archive_snapshot = VerifiedFileSnapshot::read(
        &library_path,
        prebuilt_provenance::MAX_MEMBER_BYTES,
        "built provider archive",
    )?;
    if archive_snapshot.sha256() != identity.archive_sha256 {
        return Err("built provider archive does not match the explicit build identity".into());
    }
    verify_provider_archive(
        &archive_snapshot,
        &ArchiveExpectation {
            target: &identity.target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &identity.effective_source_sha256,
            private_abi_hash: &identity.private_abi_hash,
            snapshot_layout_hash: identity.snapshot_layout_hash,
        },
    )?;
    let archive_sha256 = archive_snapshot.sha256().to_owned();
    archive_snapshot.revalidate("built provider archive package cohort")?;
    let lib = archive_snapshot.into_bytes();
    let headers = collect_headers(&effective_sources.root, &public_header_inventory)?;
    let binding_source = binding_path(manifest_dir, &identity.precision);
    let binding_snapshot = VerifiedFileSnapshot::read(
        &binding_source,
        MAX_PROVIDER_BINDINGS_BYTES,
        "required pregenerated bindings",
    )?;
    let bindings_sha256 = binding_snapshot.sha256().to_owned();
    if bindings_sha256 != identity.bindings_sha256 {
        return Err("checked bindings do not match the explicit build identity".into());
    }
    let binding_name = binding_source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid binding file name")?;
    let (header_path, header_sha256) = headers
        .iter()
        .find(|(path, _)| path == "include/box2d/box2d.h")
        .map(|(path, bytes)| (path.clone(), sha256_bytes(bytes)))
        .ok_or("box2d.h is missing from the public header set")?;

    upstream_snapshot.revalidate("upstream manifest package cohort")?;
    effective_source_manifest_snapshot.revalidate("effective source manifest package cohort")?;
    binding_snapshot.revalidate("pregenerated bindings package cohort")?;
    let upstream = upstream_snapshot.into_bytes();
    let effective_source_manifest = effective_source_manifest_snapshot.into_bytes();
    let binding = binding_snapshot.into_bytes();

    let mut files = BTreeMap::new();
    files.insert(format!("lib/{lib_name}"), lib);
    for (path, bytes) in headers {
        files.insert(path, bytes);
    }
    files.insert(format!("bindings/{binding_name}"), binding);
    files.insert("metadata/upstream.toml".to_owned(), upstream);
    files.insert(
        "metadata/effective-source.toml".to_owned(),
        effective_source_manifest,
    );
    files.insert(
        "licenses/PROJECT-LICENSE-MIT".to_owned(),
        read_required(
            &manifest_dir.join("../LICENSE-MIT"),
            "MIT license",
            MAX_PACKAGE_METADATA_BYTES,
        )?,
    );
    files.insert(
        "licenses/PROJECT-LICENSE-APACHE".to_owned(),
        read_required(
            &manifest_dir.join("../LICENSE-APACHE"),
            "Apache license",
            MAX_PACKAGE_METADATA_BYTES,
        )?,
    );
    files.insert(
        "licenses/BOX2D-LICENSE".to_owned(),
        read_required(
            &manifest_dir.join("third-party/box2d/LICENSE"),
            "upstream Box2D license",
            MAX_PACKAGE_METADATA_BYTES,
        )?,
    );

    let manifest = ArtifactManifest {
        schema_version: provider_manifest::SCHEMA_VERSION,
        schema: provider_manifest::SCHEMA_NAME.to_owned(),
        provider: ProviderCapability::Prebuilt.as_str().to_owned(),
        crate_version: identity.crate_version.clone(),
        source_commit: Some(source_commit.to_owned()),
        release_tag: Some(release_tag.to_owned()),
        upstream_sha: upstream_sha.clone(),
        effective_source_sha256: identity.effective_source_sha256.clone(),
        precision: identity.precision.clone(),
        target: identity.target.clone(),
        link: "static".to_owned(),
        crt: identity.crt.clone(),
        simd: identity.simd.clone(),
        validate: identity.validate,
        adapter_abi_version: ADAPTER_ABI_VERSION,
        adapter_source_sha256: identity.adapter_source_sha256.clone(),
        private_abi_hash: identity.private_abi_hash.clone(),
        snapshot_layout_hash: u64::from(identity.snapshot_layout_hash),
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
        required_adapter_symbols_sha256: required_adapter_symbols_sha256(),
        required_adapter_symbols: REQUIRED_ADAPTER_SYMBOLS
            .iter()
            .map(|symbol| (*symbol).to_owned())
            .collect(),
        archive: format!("lib/{lib_name}"),
        archive_sha256,
        header: header_path,
        header_sha256,
        bindings: format!("bindings/{binding_name}"),
        bindings_sha256,
    };
    let manifest_bytes = manifest.render();
    // Re-parse and validate before packaging so malformed or self-inconsistent generated output
    // cannot enter the archive.
    let generated = ArtifactManifest::parse(&manifest_bytes)
        .map_err(|error| format!("generated provider manifest is invalid: {error}"))?;
    generated
        .validate_identity(&ArtifactIdentityExpectation {
            provider: ProviderCapability::Prebuilt.as_str(),
            crate_version: &identity.crate_version,
            upstream_sha: &upstream_sha,
            effective_source_sha256: &identity.effective_source_sha256,
            precision: &identity.precision,
            target: &identity.target,
            crt: &identity.crt,
            simd: &identity.simd,
            validate: identity.validate,
            adapter_source_sha256: &identity.adapter_source_sha256,
            private_abi_hash: &identity.private_abi_hash,
            snapshot_layout_hash: identity.snapshot_layout_hash,
        })
        .map_err(|error| format!("generated provider manifest is inconsistent: {error}"))?;
    files.insert("manifest.toml".to_owned(), manifest_bytes);

    let checksums = files
        .iter()
        .map(|(path, bytes)| format!("{}  {path}\n", sha256_bytes(bytes)))
        .collect::<String>();
    files.insert("checksums.sha256".to_owned(), checksums.into_bytes());
    validate_prebuilt_member_limits(
        files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.len() as u64)),
    )?;
    Ok(files)
}

#[derive(Debug, Eq, PartialEq)]
struct BuildCommand {
    sys_out: PathBuf,
    build_identity: PathBuf,
    output: PathBuf,
    source_commit: String,
    release_tag: String,
}

fn set_argument<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!(
            "native-package build flag {flag} may only be supplied once"
        ))
    } else {
        Ok(())
    }
}

fn parse_build_command(args: &[String]) -> Result<BuildCommand, String> {
    let mut sys_out = None;
    let mut build_identity = None;
    let mut output = None;
    let mut source_commit = None;
    let mut release_tag = None;
    let mut pairs = args.chunks_exact(2);
    for pair in &mut pairs {
        match pair[0].as_str() {
            "--sys-out" => set_argument(&mut sys_out, PathBuf::from(&pair[1]), "--sys-out")?,
            "--build-identity" => set_argument(
                &mut build_identity,
                PathBuf::from(&pair[1]),
                "--build-identity",
            )?,
            "--output" => set_argument(&mut output, PathBuf::from(&pair[1]), "--output")?,
            "--source-commit" => {
                set_argument(&mut source_commit, pair[1].clone(), "--source-commit")?
            }
            "--release-tag" => set_argument(&mut release_tag, pair[1].clone(), "--release-tag")?,
            flag => return Err(format!("unknown native-package build flag {flag:?}")),
        }
    }
    if !pairs.remainder().is_empty() {
        return Err("native-package build flags require values".to_owned());
    }
    Ok(BuildCommand {
        sys_out: sys_out.ok_or("native-package build requires --sys-out")?,
        build_identity: build_identity.ok_or("native-package build requires --build-identity")?,
        output: output.ok_or("native-package build requires --output")?,
        source_commit: source_commit.ok_or("native-package build requires --source-commit")?,
        release_tag: release_tag.ok_or("native-package build requires --release-tag")?,
    })
}

fn build_native_package(
    workspace_root: &Path,
    command: &BuildCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let build_identity = ExplicitBuildIdentity::load(&command.build_identity)?;
    build_identity.require_out_dir(&command.sys_out)?;
    let identity = &build_identity.identity;
    if identity.provider != ProviderCapability::Vendored {
        return Err(format!(
            "official native packages require a vendored build identity, found {}",
            identity.provider.as_str()
        )
        .into());
    }
    validate_repository_identity(workspace_root, identity)?;
    validate_source_commit(&command.source_commit)?;
    validate_release_tag(&identity.crate_version, &command.release_tag)?;
    let manifest_dir = workspace_root.join("boxdd-sys");
    let files = build_package_files(PackageRequest {
        manifest_dir: &manifest_dir,
        sys_out: &command.sys_out,
        identity,
        source_commit: &command.source_commit,
        release_tag: &command.release_tag,
    })?;

    fs::create_dir_all(&command.output)?;
    let archive_name = compose_archive_name(
        "boxdd",
        &identity.crate_version,
        &identity.target,
        &identity.precision,
        "static",
        &identity.crt,
    );
    let output = command.output.join(archive_name);
    let file_count = files.len();
    build_identity.revalidate()?;
    let archive_snapshot = generate_file_create_new(
        &output,
        prebuilt_provenance::MAX_PACKAGE_BYTES,
        "generated prebuilt package",
        move |file| {
            let encoder = GzEncoder::new(file, Compression::default());
            let mut tar = tar::Builder::new(encoder);
            for (path, bytes) in &files {
                append_bytes(&mut tar, path, bytes).map_err(|error| error.to_string())?;
            }
            let encoder = tar.into_inner().map_err(|error| {
                format!("failed to finish generated prebuilt tar stream: {error}")
            })?;
            encoder.finish().map_err(|error| {
                format!("failed to finish generated prebuilt gzip stream: {error}")
            })?;
            Ok(())
        },
    )?;
    println!(
        "Package created: {} ({} files)",
        archive_snapshot.path().display(),
        file_count
    );
    Ok(())
}

fn run_command(workspace_root: &Path, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = workspace_root.join("boxdd-sys");
    match args {
        [command, rest @ ..] if command == "build" => {
            let command = parse_build_command(rest)?;
            build_native_package(workspace_root, &command)
        }
        [command, input, output] if command == "trust-local-system" => {
            write_caller_trusted_system_manifest(
                &manifest_dir,
                Path::new(input),
                Path::new(output),
            )?;
            println!("Caller-trusted system manifest created: {output}");
            Ok(())
        }
        [command, identity, archive, header_output, bindings, output]
            if command == "attest-local-system" =>
        {
            let identity = ExplicitBuildIdentity::load(Path::new(identity))?;
            attest_local_system(
                workspace_root,
                &identity,
                Path::new(archive),
                Path::new(header_output),
                Path::new(bindings),
                Path::new(output),
            )?;
            println!("Caller-trusted system manifest created: {output}");
            Ok(())
        }
        _ => Err(
            "native-package expects `build --sys-out <dir> --build-identity <file> --output <dir> --source-commit <sha> --release-tag <tag>`, `attest-local-system <build-identity> <archive> <header-output> <bindings> <output>`, or `trust-local-system <input> <output>`"
                .into(),
        ),
    }
}

pub fn run(workspace_root: &Path, args: &[String]) -> XtaskResult<()> {
    run_command(workspace_root, args).map_err(|error| Error::message(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn build_identity_source() -> String {
        BuildIdentity {
            provider: ProviderCapability::Vendored,
            crate_version: "0.6.0".to_owned(),
            upstream_sha: "56edae79f2949d86142b03450d5d60f63bcf5a6f".to_owned(),
            effective_source_sha256: "9".repeat(64),
            precision: "single".to_owned(),
            target: "x86_64-unknown-linux-gnu".to_owned(),
            crt: "none".to_owned(),
            simd: "default".to_owned(),
            validate: false,
            adapter_source_sha256: "a".repeat(64),
            private_abi_hash: "b".repeat(64),
            snapshot_layout_hash: 0x1234_5678,
            bindings_sha256: "c".repeat(64),
            manifest_sha256: String::new(),
            archive_sha256: "d".repeat(64),
            provenance_sha256: String::new(),
            trusted_root_sha256: String::new(),
        }
        .render()
        .unwrap()
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
    fn build_identity_requires_the_exact_schema_field_set() {
        let source = build_identity_source();
        let mut identity = BuildIdentity::parse(source.as_bytes()).unwrap();
        identity.require_native().unwrap();
        identity.archive_sha256.clear();
        assert!(identity.require_native().is_err());

        let lines = source.lines().collect::<Vec<_>>();
        for deleted in 0..lines.len() {
            let mutated = lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| (index != deleted).then_some(*line))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                BuildIdentity::parse(mutated.as_bytes()).is_err(),
                "build identity accepted deletion of field {}",
                lines[deleted]
            );
        }

        let extended = format!("{source}unreviewed = true\n");
        assert!(BuildIdentity::parse(extended.as_bytes()).is_err());
    }

    #[test]
    fn explicit_identity_binds_the_adjacent_adapter_marker() {
        let directory = tempdir().unwrap();
        let marker = directory.path().join(BUILD_IDENTITY_FILE);
        fs::write(&marker, build_identity_source()).unwrap();
        fs::write(
            directory.path().join(ADAPTER_IDENTITY_FILE),
            adapter_identity_source(&[0xbb; 32], 0x1234_5678),
        )
        .unwrap();

        let explicit = ExplicitBuildIdentity::load(&marker).unwrap();
        explicit.require_out_dir(directory.path()).unwrap();

        fs::write(
            directory.path().join(ADAPTER_IDENTITY_FILE),
            adapter_identity_source(&[0xcc; 32], 0x1234_5678),
        )
        .unwrap();
        assert!(ExplicitBuildIdentity::load(&marker).is_err());
    }

    #[test]
    fn local_system_attestation_binds_provider_and_exact_archive_bytes() {
        let directory = tempdir().unwrap();
        let archive_path = directory.path().join("libbox2d.a");
        fs::write(&archive_path, b"reviewed archive bytes").unwrap();
        let archive = VerifiedFileSnapshot::read(
            &archive_path,
            MAX_PROVIDER_ARCHIVE_BYTES,
            "attestation archive fixture",
        )
        .unwrap();
        let mut identity = BuildIdentity::parse(build_identity_source().as_bytes()).unwrap();

        identity.archive_sha256 = archive.sha256().to_owned();
        validate_system_attestation_archive(&identity, &archive).unwrap();

        identity.provider = ProviderCapability::System;
        validate_system_attestation_archive(&identity, &archive).unwrap();

        identity.provider = ProviderCapability::Prebuilt;
        assert!(validate_system_attestation_archive(&identity, &archive).is_err());

        identity.provider = ProviderCapability::Vendored;
        identity.archive_sha256 = "0".repeat(64);
        assert!(validate_system_attestation_archive(&identity, &archive).is_err());
    }

    #[test]
    fn native_build_command_requires_every_explicit_input() {
        let parsed = parse_build_command(
            &[
                "--sys-out",
                "target/sys-out",
                "--build-identity",
                "target/sys-out/boxdd-build-identity.toml",
                "--output",
                "packages",
                "--source-commit",
                "1234567890abcdef1234567890abcdef12345678",
                "--release-tag",
                "v0.6.0",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(parsed.sys_out, PathBuf::from("target/sys-out"));
        assert_eq!(parsed.output, PathBuf::from("packages"));

        assert!(
            parse_build_command(
                &[
                    "--sys-out",
                    "target/sys-out",
                    "--build-identity",
                    "target/sys-out/boxdd-build-identity.toml",
                ]
                .map(str::to_owned),
            )
            .is_err()
        );
        assert!(
            parse_build_command(
                &[
                    "--sys-out",
                    "one",
                    "--sys-out",
                    "two",
                    "--build-identity",
                    "identity",
                    "--output",
                    "packages",
                    "--source-commit",
                    "1234567890abcdef1234567890abcdef12345678",
                    "--release-tag",
                    "v0.6.0",
                ]
                .map(str::to_owned),
            )
            .is_err()
        );
    }

    #[test]
    fn public_header_packaging_uses_only_the_reviewed_closed_inventory() {
        let directory = tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("include/box2d/nested")).unwrap();
        fs::write(root.join("include/box2d/box2d.h"), b"root").unwrap();
        fs::write(root.join("include/box2d/nested/types.h"), b"nested").unwrap();
        fs::write(root.join("include/box2d/unreviewed.h"), b"extra").unwrap();
        let inventory = vec![
            "include/box2d/box2d.h".to_owned(),
            "include/box2d/nested/types.h".to_owned(),
        ];
        let headers = collect_headers(root, &inventory).unwrap();
        assert_eq!(headers.len(), inventory.len());
        assert!(
            headers
                .iter()
                .all(|(path, _)| path != "include/box2d/unreviewed.h")
        );

        let oversized_inventory = (0..=MAX_PACKAGE_PUBLIC_HEADERS)
            .map(|index| format!("include/box2d/header-{index:03}.h"))
            .collect::<Vec<_>>();
        assert!(collect_headers(root, &oversized_inventory).is_err());
    }

    #[test]
    fn package_headers_are_the_materialized_effective_source_contract() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask manifest has a workspace parent");
        let manifest_dir = workspace_root.join("boxdd-sys");
        let upstream = fs::read(manifest_dir.join("upstream.toml")).unwrap();
        let inventory = parse_public_header_inventory(&upstream).unwrap();
        let materialization = tempdir().unwrap();
        let effective =
            materialize_effective_box2d_sources(&manifest_dir, materialization.path()).unwrap();
        let headers = collect_headers(&effective.root, &inventory).unwrap();
        let packaged_box2d = headers
            .iter()
            .find(|(path, _)| path == "include/box2d/box2d.h")
            .map(|(_, bytes)| bytes.as_slice())
            .expect("reviewed inventory contains box2d.h");
        let effective_box2d = fs::read(effective.public_include.join("box2d/box2d.h")).unwrap();
        let unpatched_box2d =
            fs::read(manifest_dir.join("third-party/box2d/include/box2d/box2d.h")).unwrap();

        assert_eq!(packaged_box2d, effective_box2d);
        assert_ne!(
            packaged_box2d, unpatched_box2d,
            "the reviewed public-header transformations must not be replaced by vendored bytes"
        );
    }

    #[test]
    fn package_member_limits_match_the_signed_provenance_protocol() {
        validate_prebuilt_member_limits([
            ("archive", prebuilt_provenance::MAX_MEMBER_BYTES),
            (
                "metadata",
                prebuilt_provenance::MAX_TOTAL_MEMBER_BYTES - prebuilt_provenance::MAX_MEMBER_BYTES,
            ),
        ])
        .unwrap();
        assert!(
            validate_prebuilt_member_limits([(
                "oversized",
                prebuilt_provenance::MAX_MEMBER_BYTES + 1,
            )])
            .is_err()
        );
        assert!(
            validate_prebuilt_member_limits(
                (0..=prebuilt_provenance::MAX_MEMBERS).map(|_| ("member", 0))
            )
            .is_err()
        );
        assert!(validate_prebuilt_member_limits(std::iter::empty::<(&str, u64)>()).is_err());
    }
}
