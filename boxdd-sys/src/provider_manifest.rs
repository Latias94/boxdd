//! Canonical provider identity and artifact manifest handling.
//!
//! This module is shared by the build script and the package helper.  It deliberately
//! has no network, archive extraction, or tool invocation responsibilities: callers hand
//! it a local directory and it verifies the exact files that will be consumed.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: u64 = 3;
pub const SCHEMA_NAME: &str = "boxdd-sys-provider-v3";
pub const ADAPTER_ABI_VERSION: u64 = 2;
pub const RECORDING_CONTRACT_BLAKE3: &str =
    "26e9ed79e7e4d7ac00d927be5e9c184f2058c585c7369c589ced11da14ddefe2";
pub const VENDORED_SOURCE_IDENTITY_SHA256: &str =
    "0fac2ddee3fb443075db2a9adb8abef375fc0a27f050af93d7bf1771ec4b8de7";
pub const ADAPTER_SOURCE_PATHS: &[&str] = &[
    "effective-source.toml",
    "native/boxdd_adapter.h",
    "native/boxdd_adapter.c",
    "native/boxdd_identity_values.c",
    "native/boxdd_private_abi.inl",
    "native/boxdd_recording_adapter.c",
    "native/boxdd_snapshot_layout.inl",
    "native/boxdd_snapshot_validate.c",
    "src/source_overlay.rs",
];
pub const REQUIRED_ADAPTER_SYMBOLS: &[&str] = &[
    "boxddAdapter_AbiVersion",
    "boxddAdapter_GetIdentity",
    "boxddAdapter_GetSnapshotLayoutHash",
    "boxddEffectiveSourceSha256",
    "boxddRecPlayer_IsHealthy",
    "boxddSnapshot_Validate",
];

/// Callable adapter imports that every WASM app must retain before it can use Box2D.
///
/// `boxddEffectiveSourceSha256` remains part of [`REQUIRED_ADAPTER_SYMBOLS`] because native
/// archive inspection reads its immutable data bytes. WASM imports are functions, however; the
/// runtime receives and verifies that same digest through `boxddAdapter_GetIdentity`.
#[allow(dead_code)] // Shared with xtask; boxdd-sys/build.rs only audits native archive symbols.
pub const REQUIRED_RUNTIME_IDENTITY_IMPORTS: &[&str] = &[
    "boxddAdapter_AbiVersion",
    "boxddAdapter_GetIdentity",
    "boxddAdapter_GetSnapshotLayoutHash",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactManifest {
    pub schema_version: u64,
    pub schema: String,
    pub provider: String,
    pub crate_version: String,
    pub source_commit: Option<String>,
    pub release_tag: Option<String>,
    pub upstream_sha: String,
    pub effective_source_sha256: String,
    pub precision: String,
    pub target: String,
    pub link: String,
    pub crt: String,
    pub simd: String,
    pub validate: bool,
    pub adapter_abi_version: u64,
    pub adapter_source_sha256: String,
    pub private_abi_hash: String,
    pub snapshot_layout_hash: u64,
    pub recording_contract_blake3: String,
    pub required_adapter_symbols_sha256: String,
    pub required_adapter_symbols: Vec<String>,
    pub archive: String,
    pub archive_sha256: String,
    pub header: String,
    pub header_sha256: String,
    pub bindings: String,
    pub bindings_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArtifact {
    pub manifest: ArtifactManifest,
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
    pub archive_path: PathBuf,
    pub header_path: PathBuf,
    pub bindings_path: PathBuf,
    pub archive_sha256: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ArtifactIdentityExpectation<'a> {
    pub provider: &'a str,
    pub crate_version: &'a str,
    pub upstream_sha: &'a str,
    pub effective_source_sha256: &'a str,
    pub precision: &'a str,
    pub target: &'a str,
    pub crt: &'a str,
    pub simd: &'a str,
    pub validate: bool,
    pub adapter_source_sha256: &'a str,
    pub private_abi_hash: &'a str,
    pub snapshot_layout_hash: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ArtifactExpectation<'a> {
    pub identity: ArtifactIdentityExpectation<'a>,
    pub header_path: &'a Path,
    pub bindings_path: &'a Path,
}

impl ArtifactManifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(
            std::str::from_utf8(bytes)
                .map_err(|error| format!("provider manifest is not UTF-8: {error}"))?,
        )
        .map_err(|error| format!("provider manifest is not valid TOML: {error}"))?;
        let table = value
            .as_table()
            .ok_or_else(|| "provider manifest root must be a TOML table".to_owned())?;
        reject_unknown_fields(table)?;

        Ok(Self {
            schema_version: required_integer(table, "schema_version")?,
            schema: required_string(table, "schema")?,
            provider: required_string(table, "provider")?,
            crate_version: required_string(table, "crate_version")?,
            source_commit: optional_string(table, "source_commit")?,
            release_tag: optional_string(table, "release_tag")?,
            upstream_sha: required_string(table, "upstream_sha")?,
            effective_source_sha256: required_string(table, "effective_source_sha256")?,
            precision: required_string(table, "precision")?,
            target: required_string(table, "target")?,
            link: required_string(table, "link")?,
            crt: required_string(table, "crt")?,
            simd: required_string(table, "simd")?,
            validate: required_bool(table, "validate")?,
            adapter_abi_version: required_integer(table, "adapter_abi_version")?,
            adapter_source_sha256: required_string(table, "adapter_source_sha256")?,
            private_abi_hash: required_string(table, "private_abi_hash")?,
            snapshot_layout_hash: required_integer(table, "snapshot_layout_hash")?,
            recording_contract_blake3: required_string(table, "recording_contract_blake3")?,
            required_adapter_symbols_sha256: required_string(
                table,
                "required_adapter_symbols_sha256",
            )?,
            required_adapter_symbols: required_string_array(table, "required_adapter_symbols")?,
            archive: required_string(table, "archive")?,
            archive_sha256: required_string(table, "archive_sha256")?,
            header: required_string(table, "header")?,
            header_sha256: required_string(table, "header_sha256")?,
            bindings: required_string(table, "bindings")?,
            bindings_sha256: required_string(table, "bindings_sha256")?,
        })
    }

    pub fn validate_identity(
        &self,
        expected: &ArtifactIdentityExpectation<'_>,
    ) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION || self.schema != SCHEMA_NAME {
            return Err(format!(
                "unsupported provider manifest schema: version={} name={:?}",
                self.schema_version, self.schema
            ));
        }
        if self.provider != expected.provider {
            return Err(format!(
                "provider manifest selects `{}`, expected `{}`",
                self.provider, expected.provider
            ));
        }
        if self.crate_version != expected.crate_version {
            return Err(format!(
                "provider crate version `{}` does not match `{}`",
                self.crate_version, expected.crate_version
            ));
        }
        match expected.provider {
            "prebuilt" => self.validate_prebuilt_release_identity(expected.crate_version)?,
            "system" if self.source_commit.is_some() || self.release_tag.is_some() => {
                return Err(
                    "caller-trusted system manifests must not claim authenticated release provenance"
                        .to_owned(),
                );
            }
            _ => {}
        }
        if self.upstream_sha != expected.upstream_sha {
            return Err(format!(
                "provider upstream SHA {} does not match {}",
                self.upstream_sha, expected.upstream_sha
            ));
        }
        if self.upstream_sha.len() != 40
            || !self
                .upstream_sha
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("provider upstream_sha is not a lowercase Git SHA".to_owned());
        }
        validate_sha256("effective_source_sha256", &self.effective_source_sha256)?;
        if self.effective_source_sha256 != expected.effective_source_sha256 {
            return Err(format!(
                "provider effective source SHA-256 {} does not match {}",
                self.effective_source_sha256, expected.effective_source_sha256
            ));
        }
        if self.precision != expected.precision {
            return Err(format!(
                "provider precision `{}` does not match `{}`",
                self.precision, expected.precision
            ));
        }
        if self.target != expected.target {
            return Err(format!(
                "provider target `{}` does not match `{}`",
                self.target, expected.target
            ));
        }
        if self.target.is_empty()
            || self.precision.is_empty()
            || self.provider.is_empty()
            || self.crate_version.is_empty()
        {
            return Err("provider manifest identity fields must not be empty".to_owned());
        }
        if self.link != "static" {
            return Err(format!(
                "provider link kind `{}` is not allowed; only static archives are accepted",
                self.link
            ));
        }
        if self.crt != expected.crt {
            return Err(format!(
                "provider CRT identity `{}` does not match `{}`",
                self.crt, expected.crt
            ));
        }
        if self.simd != expected.simd {
            return Err(format!(
                "provider SIMD identity `{}` does not match `{}`",
                self.simd, expected.simd
            ));
        }
        if self.validate != expected.validate {
            return Err(format!(
                "provider validation identity {} does not match {}",
                self.validate, expected.validate
            ));
        }
        if self.adapter_abi_version != ADAPTER_ABI_VERSION {
            return Err(format!(
                "provider adapter ABI version {} does not match {}",
                self.adapter_abi_version, ADAPTER_ABI_VERSION
            ));
        }
        validate_sha256("adapter_source_sha256", &self.adapter_source_sha256)?;
        if self.adapter_source_sha256 != expected.adapter_source_sha256 {
            return Err(format!(
                "provider adapter source SHA-256 {} does not match {}",
                self.adapter_source_sha256, expected.adapter_source_sha256
            ));
        }
        validate_hex_digest("private_abi_hash", &self.private_abi_hash)?;
        if self.private_abi_hash != expected.private_abi_hash {
            return Err(format!(
                "provider private ABI hash {} does not match {}",
                self.private_abi_hash, expected.private_abi_hash
            ));
        }
        if self.snapshot_layout_hash != u64::from(expected.snapshot_layout_hash) {
            return Err(format!(
                "provider snapshot layout hash {} does not match {}",
                self.snapshot_layout_hash, expected.snapshot_layout_hash
            ));
        }
        validate_blake3("recording_contract_blake3", &self.recording_contract_blake3)?;
        if self.recording_contract_blake3 != RECORDING_CONTRACT_BLAKE3 {
            return Err(format!(
                "provider recording contract BLAKE3 {} does not match {}",
                self.recording_contract_blake3, RECORDING_CONTRACT_BLAKE3
            ));
        }
        validate_required_adapter_symbols(&self.required_adapter_symbols)?;
        validate_sha256(
            "required_adapter_symbols_sha256",
            &self.required_adapter_symbols_sha256,
        )?;
        let expected_symbols_digest = required_adapter_symbols_sha256();
        if self.required_adapter_symbols_sha256 != expected_symbols_digest {
            return Err(format!(
                "provider required-symbol digest {} does not match {}",
                self.required_adapter_symbols_sha256, expected_symbols_digest
            ));
        }
        validate_sha256("archive_sha256", &self.archive_sha256)?;
        validate_sha256("header_sha256", &self.header_sha256)?;
        validate_sha256("bindings_sha256", &self.bindings_sha256)?;
        Ok(())
    }

    fn validate_prebuilt_release_identity(
        &self,
        expected_crate_version: &str,
    ) -> Result<(), String> {
        let source_commit = self.source_commit.as_deref().ok_or_else(|| {
            "prebuilt provider manifest is missing the release source_commit".to_owned()
        })?;
        validate_git_sha("source_commit", source_commit)?;
        let release_tag = self
            .release_tag
            .as_deref()
            .ok_or_else(|| "prebuilt provider manifest is missing the release_tag".to_owned())?;
        let short_tag = format!("v{expected_crate_version}");
        let crate_tag = format!("boxdd-sys-v{expected_crate_version}");
        if release_tag != short_tag && release_tag != crate_tag {
            return Err(format!(
                "prebuilt release tag `{release_tag}` does not match crate version {expected_crate_version}"
            ));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn render(&self) -> Vec<u8> {
        let mut rendered = format!(
            "schema_version = {}\nschema = {:?}\nprovider = {:?}\ncrate_version = {:?}\n",
            self.schema_version, self.schema, self.provider, self.crate_version,
        );
        if let Some(source_commit) = &self.source_commit {
            rendered.push_str(&format!("source_commit = {source_commit:?}\n"));
        }
        if let Some(release_tag) = &self.release_tag {
            rendered.push_str(&format!("release_tag = {release_tag:?}\n"));
        }
        rendered.push_str(&format!(
            "upstream_sha = {:?}\neffective_source_sha256 = {:?}\nprecision = {:?}\ntarget = {:?}\nlink = {:?}\ncrt = {:?}\nsimd = {:?}\nvalidate = {}\nadapter_abi_version = {}\nadapter_source_sha256 = {:?}\nprivate_abi_hash = {:?}\nsnapshot_layout_hash = {}\nrecording_contract_blake3 = {:?}\nrequired_adapter_symbols_sha256 = {:?}\nrequired_adapter_symbols = {:?}\narchive = {:?}\narchive_sha256 = {:?}\nheader = {:?}\nheader_sha256 = {:?}\nbindings = {:?}\nbindings_sha256 = {:?}\n",
            self.upstream_sha,
            self.effective_source_sha256,
            self.precision,
            self.target,
            self.link,
            self.crt,
            self.simd,
            self.validate,
            self.adapter_abi_version,
            self.adapter_source_sha256,
            self.private_abi_hash,
            self.snapshot_layout_hash,
            self.recording_contract_blake3,
            self.required_adapter_symbols_sha256,
            self.required_adapter_symbols,
            self.archive,
            self.archive_sha256,
            self.header,
            self.header_sha256,
            self.bindings,
            self.bindings_sha256,
        ));
        rendered.into_bytes()
    }
}

pub fn verify_artifact(
    manifest_path: &Path,
    expected: &ArtifactExpectation<'_>,
) -> Result<VerifiedArtifact, String> {
    let manifest_metadata = fs::symlink_metadata(manifest_path).map_err(|error| {
        format!(
            "failed to inspect provider manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    if !manifest_metadata.file_type().is_file() || manifest_metadata.file_type().is_symlink() {
        return Err(format!(
            "provider manifest must be a regular non-symlink file: {}",
            manifest_path.display()
        ));
    }
    let manifest_bytes = fs::read(manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest = ArtifactManifest::parse(&manifest_bytes)?;
    if manifest.render() != manifest_bytes {
        return Err(format!(
            "provider manifest {} is not in canonical byte form",
            manifest_path.display()
        ));
    }
    manifest.validate_identity(&expected.identity)?;

    let root = manifest_path.parent().ok_or_else(|| {
        format!(
            "manifest has no parent directory: {}",
            manifest_path.display()
        )
    })?;
    let archive_path = resolve_relative_file(root, &manifest.archive, "archive")?;
    let header_path = resolve_relative_file(root, &manifest.header, "header")?;
    let bindings_path = resolve_relative_file(root, &manifest.bindings, "bindings")?;

    let archive_sha256 = sha256_file(&archive_path)?;
    if archive_sha256 != manifest.archive_sha256 {
        return Err(format!(
            "archive digest mismatch for {}: manifest={} actual={archive_sha256}",
            archive_path.display(),
            manifest.archive_sha256
        ));
    }
    let header_sha256 = sha256_file(&header_path)?;
    if header_sha256 != manifest.header_sha256 {
        return Err(format!(
            "header digest mismatch for {}: manifest={} actual={header_sha256}",
            header_path.display(),
            manifest.header_sha256
        ));
    }
    let expected_header_sha256 = sha256_file(expected.header_path)?;
    if expected_header_sha256 != header_sha256 {
        return Err(format!(
            "provider header {} does not match the crate header {}",
            header_path.display(),
            expected.header_path.display()
        ));
    }
    let bindings_sha256 = sha256_file(&bindings_path)?;
    if bindings_sha256 != manifest.bindings_sha256 {
        return Err(format!(
            "bindings digest mismatch for {}: manifest={} actual={bindings_sha256}",
            bindings_path.display(),
            manifest.bindings_sha256
        ));
    }
    let expected_bindings_sha256 = sha256_file(expected.bindings_path)?;
    if expected_bindings_sha256 != bindings_sha256 {
        return Err(format!(
            "provider bindings {} do not match the crate bindings {}",
            bindings_path.display(),
            expected.bindings_path.display()
        ));
    }

    Ok(VerifiedArtifact {
        manifest,
        manifest_path: fs::canonicalize(manifest_path).map_err(|error| {
            format!(
                "failed to resolve provider manifest {}: {error}",
                manifest_path.display()
            )
        })?,
        manifest_sha256: sha256_bytes(&manifest_bytes),
        archive_path,
        header_path,
        bindings_path,
        archive_sha256,
    })
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read {} for SHA-256: {error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

pub fn adapter_source_sha256(manifest_dir: &Path) -> Result<String, String> {
    let mut digest = Sha256::new();
    digest.update(b"boxdd.adapter.sources.v1\0");
    for relative_path in ADAPTER_SOURCE_PATHS {
        let contents = fs::read(manifest_dir.join(relative_path))
            .map_err(|error| format!("failed to read adapter source {relative_path}: {error}"))?;
        digest.update((relative_path.len() as u64).to_le_bytes());
        digest.update(relative_path.as_bytes());
        digest.update((contents.len() as u64).to_le_bytes());
        digest.update(contents);
    }
    Ok(hex_digest(digest.finalize()))
}

pub fn vendored_source_identity_sha256(
    upstream_sha: &str,
    source_tree: &str,
    source_root: &Path,
    relative_paths: &[PathBuf],
) -> Result<String, String> {
    validate_git_sha("upstream_sha", upstream_sha)?;
    validate_git_sha("source_inventory.tree", source_tree)?;

    let canonical_root = fs::canonicalize(source_root).map_err(|error| {
        format!(
            "failed to resolve vendored source root {}: {error}",
            source_root.display()
        )
    })?;
    let mut paths = relative_paths
        .iter()
        .map(|path| {
            let rendered = path
                .to_str()
                .ok_or_else(|| format!("vendored source path is not UTF-8: {}", path.display()))?;
            if rendered.is_empty()
                || rendered.contains('\\')
                || path.is_absolute()
                || !path
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
            {
                return Err(format!(
                    "vendored source path {rendered:?} is not a normalized relative path"
                ));
            }
            Ok((rendered.to_owned(), path.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    if paths
        .windows(2)
        .any(|pair| pair[0].0.as_str() == pair[1].0.as_str())
    {
        return Err("vendored source inventory contains duplicate paths".to_owned());
    }

    let mut digest = Sha256::new();
    digest.update(b"boxdd.vendored-source-identity.v1\0");
    digest.update((upstream_sha.len() as u64).to_le_bytes());
    digest.update(upstream_sha.as_bytes());
    digest.update((source_tree.len() as u64).to_le_bytes());
    digest.update(source_tree.as_bytes());
    digest.update((paths.len() as u64).to_le_bytes());
    for (rendered, relative_path) in paths {
        let candidate = source_root.join(&relative_path);
        let metadata = fs::symlink_metadata(&candidate).map_err(|error| {
            format!(
                "failed to inspect vendored source {rendered:?} at {}: {error}",
                candidate.display()
            )
        })?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(format!(
                "vendored source {rendered:?} must be a regular non-symlink file"
            ));
        }
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            format!(
                "failed to resolve vendored source {rendered:?} at {}: {error}",
                candidate.display()
            )
        })?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "vendored source {rendered:?} escapes {}",
                source_root.display()
            ));
        }
        let contents = fs::read(&canonical).map_err(|error| {
            format!(
                "failed to read vendored source {rendered:?} at {}: {error}",
                canonical.display()
            )
        })?;
        digest.update((rendered.len() as u64).to_le_bytes());
        digest.update(rendered.as_bytes());
        digest.update((contents.len() as u64).to_le_bytes());
        digest.update(contents);
    }
    Ok(hex_digest(digest.finalize()))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn resolve_relative_file(root: &Path, relative: &str, label: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "{label} path {relative:?} must be a non-empty relative path without `..`"
        ));
    }
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        format!(
            "failed to resolve provider manifest root {}: {error}",
            root.display()
        )
    })?;
    let resolved = root.join(path);
    let canonical = fs::canonicalize(&resolved).map_err(|error| {
        format!(
            "provider {label} is missing or cannot be resolved: {}: {error}",
            resolved.display()
        )
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "provider {label} path escapes manifest root through a symlink: {}",
            resolved.display()
        ));
    }
    if !canonical.is_file() {
        return Err(format!(
            "provider {label} is missing or not a file: {}",
            resolved.display()
        ));
    }
    Ok(canonical)
}

fn required_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("provider manifest field `{key}` must be a string"))
}

fn optional_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<Option<String>, String> {
    match table.get(key) {
        Some(value) => value
            .as_str()
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| format!("provider manifest field `{key}` must be a string")),
        None => Ok(None),
    }
}

fn required_string_array(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("provider manifest field `{key}` must be an array of strings"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("provider manifest field `{key}` must contain only strings"))
        })
        .collect()
}

fn reject_unknown_fields(table: &toml::map::Map<String, toml::Value>) -> Result<(), String> {
    const FIELDS: &[&str] = &[
        "schema_version",
        "schema",
        "provider",
        "crate_version",
        "source_commit",
        "release_tag",
        "upstream_sha",
        "effective_source_sha256",
        "precision",
        "target",
        "link",
        "crt",
        "simd",
        "validate",
        "adapter_abi_version",
        "adapter_source_sha256",
        "private_abi_hash",
        "snapshot_layout_hash",
        "recording_contract_blake3",
        "required_adapter_symbols_sha256",
        "required_adapter_symbols",
        "archive",
        "archive_sha256",
        "header",
        "header_sha256",
        "bindings",
        "bindings_sha256",
    ];
    if let Some(field) = table.keys().find(|field| !FIELDS.contains(&field.as_str())) {
        return Err(format!(
            "provider manifest contains unsupported field `{field}`"
        ));
    }
    Ok(())
}

fn required_integer(table: &toml::map::Map<String, toml::Value>, key: &str) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| format!("provider manifest field `{key}` must be a non-negative integer"))
}

fn required_bool(table: &toml::map::Map<String, toml::Value>, key: &str) -> Result<bool, String> {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| format!("provider manifest field `{key}` must be a boolean"))
}

fn validate_sha256(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a SHA-256 digest"
        ))
    }
}

fn validate_blake3(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a BLAKE3 digest"
        ))
    }
}

fn validate_hex_digest(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a canonical 32-byte hexadecimal identity"
        ))
    }
}

pub fn required_adapter_symbols_sha256() -> String {
    let mut digest = Sha256::new();
    digest.update(b"boxdd.adapter.required-symbols.v1\0");
    digest.update((REQUIRED_ADAPTER_SYMBOLS.len() as u64).to_le_bytes());
    for symbol in REQUIRED_ADAPTER_SYMBOLS {
        digest.update((symbol.len() as u64).to_le_bytes());
        digest.update(symbol.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn validate_required_adapter_symbols(symbols: &[String]) -> Result<(), String> {
    if symbols
        .windows(2)
        .any(|pair| pair[0].as_str() >= pair[1].as_str())
    {
        return Err(
            "provider required_adapter_symbols must be strictly sorted without duplicates"
                .to_owned(),
        );
    }
    if symbols.len() != REQUIRED_ADAPTER_SYMBOLS.len()
        || symbols
            .iter()
            .map(String::as_str)
            .ne(REQUIRED_ADAPTER_SYMBOLS.iter().copied())
    {
        return Err(format!(
            "provider required_adapter_symbols do not match the crate adapter contract: expected {REQUIRED_ADAPTER_SYMBOLS:?}, found {symbols:?}"
        ));
    }
    Ok(())
}

fn validate_git_sha(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a lowercase Git SHA"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const TEST_EFFECTIVE_SOURCE_SHA256: &str =
        "9999999999999999999999999999999999999999999999999999999999999999";

    fn valid_manifest(adapter_source_sha256: &str) -> ArtifactManifest {
        ArtifactManifest {
            schema_version: SCHEMA_VERSION,
            schema: SCHEMA_NAME.to_owned(),
            provider: "system".to_owned(),
            crate_version: "0.6.0".to_owned(),
            source_commit: None,
            release_tag: None,
            upstream_sha: "56edae79f2949d86142b03450d5d60f63bcf5a6f".to_owned(),
            effective_source_sha256: TEST_EFFECTIVE_SOURCE_SHA256.to_owned(),
            precision: "single".to_owned(),
            target: "x86_64-unknown-linux-gnu".to_owned(),
            link: "static".to_owned(),
            crt: "none".to_owned(),
            simd: "default".to_owned(),
            validate: false,
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: adapter_source_sha256.to_owned(),
            private_abi_hash: "e".repeat(64),
            snapshot_layout_hash: 0x1234_5678,
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            required_adapter_symbols_sha256: required_adapter_symbols_sha256(),
            required_adapter_symbols: REQUIRED_ADAPTER_SYMBOLS
                .iter()
                .map(|symbol| (*symbol).to_owned())
                .collect(),
            archive: "lib/libbox2d.a".to_owned(),
            archive_sha256: "b".repeat(64),
            header: "include/box2d/box2d.h".to_owned(),
            header_sha256: "c".repeat(64),
            bindings: "bindings/bindings.rs".to_owned(),
            bindings_sha256: "d".repeat(64),
        }
    }

    fn expectation<'a>(adapter_source_sha256: &'a str) -> ArtifactIdentityExpectation<'a> {
        ArtifactIdentityExpectation {
            provider: "system",
            crate_version: "0.6.0",
            upstream_sha: "56edae79f2949d86142b03450d5d60f63bcf5a6f",
            effective_source_sha256: TEST_EFFECTIVE_SOURCE_SHA256,
            precision: "single",
            target: "x86_64-unknown-linux-gnu",
            crt: "none",
            simd: "default",
            validate: false,
            adapter_source_sha256,
            private_abi_hash: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            snapshot_layout_hash: 0x1234_5678,
        }
    }

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn relative_paths_reject_escape_and_missing_files() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("ok.a"), b"ok").unwrap();
        assert!(resolve_relative_file(directory.path(), "ok.a", "archive").is_ok());
        assert!(resolve_relative_file(directory.path(), "../ok.a", "archive").is_err());
        assert!(resolve_relative_file(directory.path(), "/tmp/ok.a", "archive").is_err());
        assert!(resolve_relative_file(directory.path(), "missing.a", "archive").is_err());
    }

    #[test]
    fn digest_validation_requires_canonical_lowercase_hex() {
        assert!(validate_sha256("digest", &"a".repeat(64)).is_ok());
        assert!(validate_sha256("digest", &"A".repeat(64)).is_err());
        assert!(validate_sha256("digest", &"0".repeat(63)).is_err());
    }

    #[test]
    fn adapter_contract_roundtrips_and_matches_crate_owned_expectations() {
        assert_eq!(SCHEMA_VERSION, 3);
        assert_eq!(SCHEMA_NAME, "boxdd-sys-provider-v3");
        assert_eq!(ADAPTER_ABI_VERSION, 2);
        assert!(ADAPTER_SOURCE_PATHS.contains(&"effective-source.toml"));
        assert!(REQUIRED_ADAPTER_SYMBOLS.contains(&"boxddEffectiveSourceSha256"));
        assert_eq!(
            REQUIRED_RUNTIME_IDENTITY_IMPORTS,
            [
                "boxddAdapter_AbiVersion",
                "boxddAdapter_GetIdentity",
                "boxddAdapter_GetSnapshotLayoutHash",
            ]
        );
        let adapter_source_sha256 = "a".repeat(64);
        let manifest = valid_manifest(&adapter_source_sha256);
        let parsed = ArtifactManifest::parse(&manifest.render()).unwrap();
        assert_eq!(parsed, manifest);
        parsed
            .validate_identity(&expectation(&adapter_source_sha256))
            .unwrap();
    }

    #[test]
    fn manifest_parser_rejects_a_missing_effective_source_identity() {
        let rendered = String::from_utf8(valid_manifest(&"a".repeat(64)).render()).unwrap();
        let without_effective_source = rendered
            .lines()
            .filter(|line| !line.starts_with("effective_source_sha256 = "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";

        assert!(ArtifactManifest::parse(without_effective_source.as_bytes()).is_err());
    }

    #[test]
    fn provider_identity_rejects_the_previous_schema_and_effective_source_drift() {
        let adapter_source_sha256 = "a".repeat(64);
        let expected = expectation(&adapter_source_sha256);
        let mut manifest = valid_manifest(&adapter_source_sha256);

        manifest.schema_version = 2;
        manifest.schema = "boxdd-sys-provider-v2".to_owned();
        assert!(manifest.validate_identity(&expected).is_err());

        manifest.schema_version = SCHEMA_VERSION;
        manifest.schema = SCHEMA_NAME.to_owned();
        manifest.effective_source_sha256 = "8".repeat(64);
        assert!(manifest.validate_identity(&expected).is_err());
    }

    #[test]
    fn adapter_contract_rejects_source_recording_and_symbol_drift() {
        let adapter_source_sha256 = "a".repeat(64);
        let expected = expectation(&adapter_source_sha256);

        let mut wrong_source = valid_manifest(&"e".repeat(64));
        assert!(wrong_source.validate_identity(&expected).is_err());
        wrong_source.adapter_source_sha256 = adapter_source_sha256.clone();
        wrong_source.effective_source_sha256 = "f".repeat(64);
        assert!(wrong_source.validate_identity(&expected).is_err());
        wrong_source.effective_source_sha256 = TEST_EFFECTIVE_SOURCE_SHA256.to_owned();
        wrong_source.recording_contract_blake3 = "f".repeat(64);
        assert!(wrong_source.validate_identity(&expected).is_err());

        let mut missing_symbol = valid_manifest(&adapter_source_sha256);
        missing_symbol.required_adapter_symbols.pop();
        assert!(missing_symbol.validate_identity(&expected).is_err());

        let mut duplicate_symbol = valid_manifest(&adapter_source_sha256);
        duplicate_symbol.required_adapter_symbols[1] =
            duplicate_symbol.required_adapter_symbols[0].clone();
        assert!(duplicate_symbol.validate_identity(&expected).is_err());
    }

    #[test]
    fn adapter_source_digest_is_ordered_and_content_sensitive() {
        const EFFECTIVE_SOURCE_MANIFEST: &str = "effective-source.toml";
        const OVERLAY_SOURCE: &str = "src/source_overlay.rs";
        assert!(ADAPTER_SOURCE_PATHS.contains(&EFFECTIVE_SOURCE_MANIFEST));
        assert!(ADAPTER_SOURCE_PATHS.contains(&OVERLAY_SOURCE));
        let directory = tempdir().unwrap();
        for (index, relative) in ADAPTER_SOURCE_PATHS.iter().enumerate() {
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("source {index}")).unwrap();
        }
        let initial = adapter_source_sha256(directory.path()).unwrap();
        fs::write(directory.path().join(OVERLAY_SOURCE), "changed overlay").unwrap();
        let changed = adapter_source_sha256(directory.path()).unwrap();
        assert_ne!(initial, changed);
        fs::write(
            directory.path().join(EFFECTIVE_SOURCE_MANIFEST),
            "changed effective-source policy",
        )
        .unwrap();
        let changed_policy = adapter_source_sha256(directory.path()).unwrap();
        assert_ne!(changed, changed_policy);
    }

    #[test]
    fn vendored_source_identity_binds_revision_tree_inventory_and_bytes() {
        let directory = tempdir().unwrap();
        let paths = vec![PathBuf::from("src/a.c"), PathBuf::from("include/box2d/a.h")];
        for path in &paths {
            let full = directory.path().join(path);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, path.to_string_lossy().as_bytes()).unwrap();
        }
        let revision = "56edae79f2949d86142b03450d5d60f63bcf5a6f";
        let tree = "63a1ab02e3d2bf7c4d86b257b78976842b8c5ddb";
        let initial =
            vendored_source_identity_sha256(revision, tree, directory.path(), &paths).unwrap();
        assert_ne!(
            initial,
            vendored_source_identity_sha256(
                "0000000000000000000000000000000000000000",
                tree,
                directory.path(),
                &paths,
            )
            .unwrap()
        );
        fs::write(directory.path().join(&paths[0]), b"changed").unwrap();
        assert_ne!(
            initial,
            vendored_source_identity_sha256(revision, tree, directory.path(), &paths).unwrap()
        );
        assert!(
            vendored_source_identity_sha256(
                revision,
                tree,
                directory.path(),
                &[paths[0].clone(), paths[0].clone()],
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_manifest_payload_changes_for_coordinate_or_archive_digest_drift() {
        let mut manifest = valid_manifest(&"a".repeat(64));
        let baseline = manifest.render();
        manifest.effective_source_sha256 = "8".repeat(64);
        assert_ne!(baseline, manifest.render());
        manifest.effective_source_sha256 = TEST_EFFECTIVE_SOURCE_SHA256.to_owned();
        manifest.crt = "md".to_owned();
        assert_ne!(baseline, manifest.render());
        manifest.crt = "none".to_owned();
        manifest.archive_sha256 = "f".repeat(64);
        assert_ne!(baseline, manifest.render());
    }

    #[test]
    fn manifest_parser_rejects_unknown_fields() {
        let source = b"schema_version = 1\nschema = \"boxdd-sys-provider-v1\"\nprovider = \"system\"\ncrate_version = \"0.6.0\"\nupstream_sha = \"56edae79f2949d86142b03450d5d60f63bcf5a6f\"\nprecision = \"single\"\ntarget = \"x86_64-unknown-linux-gnu\"\nlink = \"static\"\ncrt = \"none\"\nsimd = \"default\"\nvalidate = false\narchive = \"lib/libbox2d.a\"\narchive_sha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\nheader = \"include/box2d/box2d.h\"\nheader_sha256 = \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"\nbindings = \"bindings/bindings.rs\"\nbindings_sha256 = \"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\"\nunreviewed = true\n";
        assert!(ArtifactManifest::parse(source).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn relative_paths_reject_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("archive.a"), b"outside").unwrap();
        symlink(
            outside.path().join("archive.a"),
            root.path().join("archive.a"),
        )
        .unwrap();
        assert!(resolve_relative_file(root.path(), "archive.a", "archive").is_err());
    }
}
