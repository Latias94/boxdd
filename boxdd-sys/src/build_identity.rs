//! Canonical build identity exchanged between `boxdd-sys/build.rs` and repository tooling.
//!
//! The build script is the producer and `xtask native-package` is the consumer. Keeping the
//! closed schema, validation, and deterministic rendering here prevents the two sides from
//! silently drifting.

use crate::provider_catalog::ProviderCapability;
use crate::provider_manifest::{BUILD_IDENTITY_SCHEMA_VERSION, RECORDING_CONTRACT_BLAKE3};
use std::collections::BTreeSet;

pub const BUILD_IDENTITY_FILE: &str = "boxdd-build-identity.toml";

const FIELDS: [&str; 19] = [
    "schema_version",
    "provider",
    "crate_version",
    "upstream_sha",
    "effective_source_sha256",
    "precision",
    "target",
    "crt",
    "simd",
    "validate",
    "adapter_source_sha256",
    "recording_contract_blake3",
    "private_abi_hash",
    "snapshot_layout_hash",
    "bindings_sha256",
    "manifest_sha256",
    "archive_sha256",
    "provenance_sha256",
    "trusted_root_sha256",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildIdentity {
    pub provider: ProviderCapability,
    pub crate_version: String,
    pub upstream_sha: String,
    pub effective_source_sha256: String,
    pub precision: String,
    pub target: String,
    pub crt: String,
    pub simd: String,
    pub validate: bool,
    pub adapter_source_sha256: String,
    pub private_abi_hash: String,
    pub snapshot_layout_hash: u32,
    pub bindings_sha256: String,
    pub manifest_sha256: String,
    pub archive_sha256: String,
    pub provenance_sha256: String,
    pub trusted_root_sha256: String,
}

impl BuildIdentity {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(
            std::str::from_utf8(bytes)
                .map_err(|error| format!("build identity is not UTF-8: {error}"))?,
        )
        .map_err(|error| format!("build identity is not valid TOML: {error}"))?;
        let table = value
            .as_table()
            .ok_or_else(|| "build identity root must be a TOML table".to_owned())?;
        let expected = FIELDS.into_iter().collect::<BTreeSet<_>>();
        let actual = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(format!(
                "build identity field set mismatch: expected {expected:?}, found {actual:?}"
            ));
        }

        let schema_version = required_u64(table, "schema_version")?;
        if schema_version != BUILD_IDENTITY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported build identity schema {schema_version}; expected {BUILD_IDENTITY_SCHEMA_VERSION}"
            ));
        }
        let recording_contract_blake3 = required_string(table, "recording_contract_blake3")?;
        if recording_contract_blake3 != RECORDING_CONTRACT_BLAKE3 {
            return Err("build identity recording contract does not match boxdd".to_owned());
        }

        let identity = Self {
            provider: ProviderCapability::parse_build_name(&required_string(table, "provider")?)?,
            crate_version: required_string(table, "crate_version")?,
            upstream_sha: required_string(table, "upstream_sha")?,
            effective_source_sha256: required_string(table, "effective_source_sha256")?,
            precision: required_string(table, "precision")?,
            target: required_string(table, "target")?,
            crt: required_string(table, "crt")?,
            simd: required_string(table, "simd")?,
            validate: required_bool(table, "validate")?,
            adapter_source_sha256: required_string(table, "adapter_source_sha256")?,
            private_abi_hash: required_string(table, "private_abi_hash")?,
            snapshot_layout_hash: u32::try_from(required_u64(table, "snapshot_layout_hash")?)
                .map_err(|_| "build identity snapshot_layout_hash exceeds u32".to_owned())?,
            bindings_sha256: required_string(table, "bindings_sha256")?,
            manifest_sha256: required_string(table, "manifest_sha256")?,
            archive_sha256: required_string(table, "archive_sha256")?,
            provenance_sha256: required_string(table, "provenance_sha256")?,
            trusted_root_sha256: required_string(table, "trusted_root_sha256")?,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn render(&self) -> Result<String, String> {
        self.validate()?;
        Ok(format!(
            "schema_version = {}\nprovider = {:?}\ncrate_version = {:?}\nupstream_sha = {:?}\neffective_source_sha256 = {:?}\nprecision = {:?}\ntarget = {:?}\ncrt = {:?}\nsimd = {:?}\nvalidate = {}\nadapter_source_sha256 = {:?}\nrecording_contract_blake3 = {:?}\nprivate_abi_hash = {:?}\nsnapshot_layout_hash = {}\nbindings_sha256 = {:?}\nmanifest_sha256 = {:?}\narchive_sha256 = {:?}\nprovenance_sha256 = {:?}\ntrusted_root_sha256 = {:?}\n",
            BUILD_IDENTITY_SCHEMA_VERSION,
            self.provider.as_str(),
            self.crate_version,
            self.upstream_sha,
            self.effective_source_sha256,
            self.precision,
            self.target,
            self.crt,
            self.simd,
            self.validate,
            self.adapter_source_sha256,
            RECORDING_CONTRACT_BLAKE3,
            self.private_abi_hash,
            self.snapshot_layout_hash,
            self.bindings_sha256,
            self.manifest_sha256,
            self.archive_sha256,
            self.provenance_sha256,
            self.trusted_root_sha256,
        ))
    }

    pub fn require_native(&self) -> Result<(), String> {
        if self.provider.is_wasm()
            || self.private_abi_hash == "0".repeat(64)
            || self.snapshot_layout_hash == 0
        {
            return Err("build identity does not describe an available native ABI".to_owned());
        }
        let vendored_artifact_is_bound = self.manifest_sha256.is_empty()
            && !self.archive_sha256.is_empty()
            && self.provenance_sha256.is_empty()
            && self.trusted_root_sha256.is_empty();
        let external_artifact_is_bound = !self.manifest_sha256.is_empty()
            && !self.archive_sha256.is_empty()
            && self.provenance_sha256.is_empty()
            && self.trusted_root_sha256.is_empty();
        let authenticated_prebuilt_is_bound = !self.manifest_sha256.is_empty()
            && !self.archive_sha256.is_empty()
            && !self.provenance_sha256.is_empty()
            && !self.trusted_root_sha256.is_empty();
        let consistent = match self.provider {
            ProviderCapability::Vendored => vendored_artifact_is_bound,
            ProviderCapability::System => external_artifact_is_bound,
            ProviderCapability::Prebuilt => authenticated_prebuilt_is_bound,
            ProviderCapability::WasmCompileOnly | ProviderCapability::WasmProvider => false,
        };
        if consistent {
            Ok(())
        } else {
            Err(format!(
                "build identity provider digests are inconsistent for {}",
                self.provider.as_str()
            ))
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.crate_version.is_empty()
            || self.target.is_empty()
            || !matches!(self.precision.as_str(), "single" | "double")
            || !matches!(self.crt.as_str(), "none" | "md" | "mt")
            || !matches!(self.simd.as_str(), "default" | "disabled" | "avx2")
        {
            return Err("build identity contains an unsupported target coordinate".to_owned());
        }
        validate_source_commit(&self.upstream_sha)?;
        for (label, digest) in [
            (
                "effective_source_sha256",
                self.effective_source_sha256.as_str(),
            ),
            ("adapter_source_sha256", self.adapter_source_sha256.as_str()),
            ("private_abi_hash", self.private_abi_hash.as_str()),
            ("bindings_sha256", self.bindings_sha256.as_str()),
        ] {
            validate_sha256(label, digest, false)?;
        }
        for (label, digest) in [
            ("manifest_sha256", self.manifest_sha256.as_str()),
            ("archive_sha256", self.archive_sha256.as_str()),
            ("provenance_sha256", self.provenance_sha256.as_str()),
            ("trusted_root_sha256", self.trusted_root_sha256.as_str()),
        ] {
            validate_sha256(label, digest, true)?;
        }
        Ok(())
    }
}

pub fn validate_source_commit(commit: &str) -> Result<(), String> {
    if !is_lower_hex(commit, 40) {
        return Err("prebuilt package source commit must be a lowercase Git SHA".to_owned());
    }
    Ok(())
}

pub fn validate_sha256(label: &str, value: &str, allow_empty: bool) -> Result<(), String> {
    if allow_empty && value.is_empty() {
        return Ok(());
    }
    if !is_lower_hex(value, 64) {
        return Err(format!(
            "build identity {label} must be a lowercase SHA-256"
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

fn required_string(table: &toml::value::Table, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("build identity {key} must be a string"))
}

fn required_u64(table: &toml::value::Table, key: &str) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| format!("build identity {key} must be a non-negative integer"))
}

fn required_bool(table: &toml::value::Table, key: &str) -> Result<bool, String> {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| format!("build identity {key} must be a boolean"))
}
