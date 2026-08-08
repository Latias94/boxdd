//! Canonical provider identity and artifact manifest handling.
//!
//! This module is shared by the build script and repository packaging tooling. It deliberately
//! has no network, archive extraction, or tool invocation responsibilities: callers hand
//! it a local directory and it verifies the exact files that will be consumed.

use crate::{
    build_support::VerifiedFileSnapshot, provenance_policy::release_tag_matches_version,
    provider_catalog::ProviderCapability,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: u64 = 3;
pub const BUILD_IDENTITY_SCHEMA_VERSION: u64 = 3;
pub const SCHEMA_NAME: &str = "boxdd-sys-provider-v3";
pub const ADAPTER_ABI_VERSION: u64 = crate::adapter_contract::ADAPTER_ABI_VERSION as u64;
pub const MAX_PROVIDER_MANIFEST_BYTES: u64 = 1024 * 1024;
pub const MAX_PROVIDER_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_PROVIDER_HEADER_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_PROVIDER_BINDINGS_BYTES: u64 = 64 * 1024 * 1024;
pub const RECORDING_CONTRACT_BLAKE3: &str =
    "0b17edca6df8d03b6bfa4d4ebf3cb6a30ccf547b9ea1d389b8dd468e1a921a24";
pub const VENDORED_SOURCE_IDENTITY_SHA256: &str =
    "0fac2ddee3fb443075db2a9adb8abef375fc0a27f050af93d7bf1771ec4b8de7";
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

/// Non-Box2D function exports that form part of the WASM provider ABI.
///
/// This list covers the adapter functions that Rust consumers may import from the provider module.
#[allow(dead_code)] // Shared with repository tooling; native sys builds do not consume this list.
pub const REQUIRED_WASM_PROVIDER_ADAPTER_EXPORTS: &[&str] = &[
    "boxddAdapter_AbiVersion",
    "boxddAdapter_GetIdentity",
    "boxddAdapter_GetSnapshotLayoutHash",
    "boxddRecPlayer_IsHealthy",
    "boxddSnapshot_Validate",
];

pub fn validate_recording_contract_blake3(actual: &str) -> Result<(), String> {
    validate_blake3("recording_contract_blake3", actual)?;
    if actual == RECORDING_CONTRACT_BLAKE3 {
        Ok(())
    } else {
        Err(format!(
            "recording contract BLAKE3 {actual} does not match {RECORDING_CONTRACT_BLAKE3}"
        ))
    }
}

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

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedArtifact {
    pub manifest: ArtifactManifest,
    pub manifest_snapshot: VerifiedFileSnapshot,
    pub archive_snapshot: VerifiedFileSnapshot,
    pub header_snapshot: VerifiedFileSnapshot,
    pub bindings_snapshot: VerifiedFileSnapshot,
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
        let provider = ProviderCapability::parse_build_name(&self.provider)?;
        let expected_provider = ProviderCapability::parse_build_name(expected.provider)?;
        if provider != expected_provider {
            return Err(format!(
                "provider manifest selects `{}`, expected `{}`",
                self.provider, expected.provider
            ));
        }
        if !provider.supports_native_qualification() {
            return Err(format!(
                "provider manifest selects unsupported external provider `{}`",
                self.provider
            ));
        }
        if self.crate_version != expected.crate_version {
            return Err(format!(
                "provider crate version `{}` does not match `{}`",
                self.crate_version, expected.crate_version
            ));
        }
        match provider {
            ProviderCapability::Prebuilt => {
                self.validate_prebuilt_release_identity(expected.crate_version)?
            }
            ProviderCapability::System
                if self.source_commit.is_some() || self.release_tag.is_some() =>
            {
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
        if self.target.is_empty() || self.provider.is_empty() || self.crate_version.is_empty() {
            return Err("provider manifest identity fields must not be empty".to_owned());
        }
        if !matches!(self.precision.as_str(), "single" | "double") {
            return Err(format!(
                "provider precision `{}` is unsupported; expected `single` or `double`",
                self.precision
            ));
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
        if !matches!(self.crt.as_str(), "none" | "md" | "mt") {
            return Err(format!(
                "provider CRT identity `{}` is unsupported",
                self.crt
            ));
        }
        if self.simd != expected.simd {
            return Err(format!(
                "provider SIMD identity `{}` does not match `{}`",
                self.simd, expected.simd
            ));
        }
        if !matches!(self.simd.as_str(), "default" | "disabled" | "avx2") {
            return Err(format!(
                "provider SIMD identity `{}` is unsupported",
                self.simd
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
        validate_recording_contract_blake3(&self.recording_contract_blake3)?;
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
        if !release_tag_matches_version(expected_crate_version, release_tag) {
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
    let manifest_snapshot = VerifiedFileSnapshot::read(
        manifest_path,
        MAX_PROVIDER_MANIFEST_BYTES,
        "provider manifest",
    )?;
    let manifest = ArtifactManifest::parse(manifest_snapshot.bytes())?;
    if manifest.render().as_slice() != manifest_snapshot.bytes() {
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

    let archive_snapshot = VerifiedFileSnapshot::read(
        &archive_path,
        MAX_PROVIDER_ARCHIVE_BYTES,
        "provider archive",
    )?;
    archive_snapshot.verify_sha256(&manifest.archive_sha256, "provider archive")?;

    let header_snapshot =
        VerifiedFileSnapshot::read(&header_path, MAX_PROVIDER_HEADER_BYTES, "provider header")?;
    let expected_header = VerifiedFileSnapshot::read(
        expected.header_path,
        MAX_PROVIDER_HEADER_BYTES,
        "crate header",
    )?;
    header_snapshot.verify_exact(
        expected_header.bytes(),
        &manifest.header_sha256,
        "provider header",
    )?;

    let bindings_snapshot = VerifiedFileSnapshot::read(
        &bindings_path,
        MAX_PROVIDER_BINDINGS_BYTES,
        "provider bindings",
    )?;
    let expected_bindings = VerifiedFileSnapshot::read(
        expected.bindings_path,
        MAX_PROVIDER_BINDINGS_BYTES,
        "crate bindings",
    )?;
    bindings_snapshot.verify_exact(
        expected_bindings.bytes(),
        &manifest.bindings_sha256,
        "provider bindings",
    )?;

    Ok(VerifiedArtifact {
        manifest,
        manifest_snapshot,
        archive_snapshot,
        header_snapshot,
        bindings_snapshot,
    })
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
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{label} path {relative:?} must be a non-empty normalized relative path"
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
    if is_canonical_32_byte_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a SHA-256 digest"
        ))
    }
}

fn validate_blake3(label: &str, value: &str) -> Result<(), String> {
    if is_canonical_32_byte_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a BLAKE3 digest"
        ))
    }
}

fn validate_hex_digest(label: &str, value: &str) -> Result<(), String> {
    if is_canonical_32_byte_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "provider manifest field `{label}` is not a canonical 32-byte hexadecimal identity"
        ))
    }
}

fn is_canonical_32_byte_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    use crate::source_overlay::{ADAPTER_SOURCE_PATHS, adapter_source_sha256};
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
        assert!(resolve_relative_file(directory.path(), "./ok.a", "archive").is_err());
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
    fn recording_contract_digest_requires_the_canonical_fixture_identity() {
        assert!(validate_recording_contract_blake3(RECORDING_CONTRACT_BLAKE3).is_ok());
        assert!(validate_recording_contract_blake3(&"0".repeat(64)).is_err());
        assert!(validate_recording_contract_blake3("not-a-digest").is_err());
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
    fn manifest_identity_rejects_self_consistent_unknown_coordinates() {
        let adapter_source_sha256 = "a".repeat(64);

        let mut precision_manifest = valid_manifest(&adapter_source_sha256);
        precision_manifest.precision = "quad".to_owned();
        let mut precision_expectation = expectation(&adapter_source_sha256);
        precision_expectation.precision = "quad";
        assert!(
            precision_manifest
                .validate_identity(&precision_expectation)
                .unwrap_err()
                .contains("unsupported")
        );

        let mut crt_manifest = valid_manifest(&adapter_source_sha256);
        crt_manifest.crt = "dynamic".to_owned();
        let mut crt_expectation = expectation(&adapter_source_sha256);
        crt_expectation.crt = "dynamic";
        assert!(
            crt_manifest
                .validate_identity(&crt_expectation)
                .unwrap_err()
                .contains("unsupported")
        );

        let mut simd_manifest = valid_manifest(&adapter_source_sha256);
        simd_manifest.simd = "host".to_owned();
        let mut simd_expectation = expectation(&adapter_source_sha256);
        simd_expectation.simd = "host";
        assert!(
            simd_manifest
                .validate_identity(&simd_expectation)
                .unwrap_err()
                .contains("unsupported")
        );

        let mut provider_manifest = valid_manifest(&adapter_source_sha256);
        provider_manifest.provider = "mirror".to_owned();
        let mut provider_expectation = expectation(&adapter_source_sha256);
        provider_expectation.provider = "mirror";
        assert!(
            provider_manifest
                .validate_identity(&provider_expectation)
                .unwrap_err()
                .contains("unsupported")
        );
    }

    #[test]
    fn verified_artifact_retains_the_exact_archive_header_and_bindings_snapshots() {
        let provider_root = tempdir().unwrap();
        let crate_root = tempdir().unwrap();
        let adapter_source_sha256 = "a".repeat(64);
        let archive_bytes = b"provider archive bytes";
        let header_bytes = b"provider header bytes";
        let bindings_bytes = b"provider bindings bytes";

        let archive_path = provider_root.path().join("lib/libbox2d.a");
        let header_path = provider_root.path().join("include/box2d/box2d.h");
        let bindings_path = provider_root.path().join("bindings/bindings.rs");
        for path in [&archive_path, &header_path, &bindings_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&archive_path, archive_bytes).unwrap();
        fs::write(&header_path, header_bytes).unwrap();
        fs::write(&bindings_path, bindings_bytes).unwrap();

        let expected_header = crate_root.path().join("box2d.h");
        let expected_bindings = crate_root.path().join("bindings.rs");
        fs::write(&expected_header, header_bytes).unwrap();
        fs::write(&expected_bindings, bindings_bytes).unwrap();

        let mut manifest = valid_manifest(&adapter_source_sha256);
        manifest.archive_sha256 = sha256_bytes(archive_bytes);
        manifest.header_sha256 = sha256_bytes(header_bytes);
        manifest.bindings_sha256 = sha256_bytes(bindings_bytes);
        let manifest_path = provider_root.path().join("manifest.toml");
        fs::write(&manifest_path, manifest.render()).unwrap();

        let verified = verify_artifact(
            &manifest_path,
            &ArtifactExpectation {
                identity: expectation(&adapter_source_sha256),
                header_path: &expected_header,
                bindings_path: &expected_bindings,
            },
        )
        .unwrap();
        assert_eq!(verified.archive_snapshot.bytes(), archive_bytes);
        assert_eq!(verified.header_snapshot.bytes(), header_bytes);
        assert_eq!(verified.bindings_snapshot.bytes(), bindings_bytes);

        fs::write(&archive_path, b"replacement archive bytes").unwrap();
        assert_eq!(verified.archive_snapshot.bytes(), archive_bytes);
        assert_eq!(
            verified.archive_snapshot.sha256(),
            manifest.archive_sha256.as_str()
        );
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
        const ADAPTER_SOURCE: &str = "native/boxdd_adapter.c";
        assert!(ADAPTER_SOURCE_PATHS.contains(&EFFECTIVE_SOURCE_MANIFEST));
        assert!(ADAPTER_SOURCE_PATHS.contains(&ADAPTER_SOURCE));
        let directory = tempdir().unwrap();
        for (index, relative) in ADAPTER_SOURCE_PATHS.iter().enumerate() {
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("source {index}")).unwrap();
        }
        let initial = adapter_source_sha256(directory.path()).unwrap();
        fs::write(directory.path().join(ADAPTER_SOURCE), "changed adapter").unwrap();
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
