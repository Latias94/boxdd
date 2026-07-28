//! Canonical provenance statement for one official WASM provider package.
//!
//! This module is intentionally limited to deterministic parsing, rendering, and
//! byte-level verification. SDK provisioning, package extraction, signing, and
//! network access remain responsibilities of release orchestration.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

pub(crate) use crate::provider_manifest::sha256_bytes;

pub(crate) const SCHEMA_VERSION: u64 = 1;
pub(crate) const SCHEMA_NAME: &str = "boxdd-wasm-release-provenance-v1";
pub(crate) const PUBLISHER_WORKFLOW: &str = ".github/workflows/prebuilt-binaries.yml";
pub(crate) const PACKAGE_TYPE: &str = "wasm-provider";
pub(crate) const PROVIDER_ABI: &str = "box2d-sys-v1";
pub(crate) const TARGET: &str = "wasm32-unknown-unknown";
pub(crate) const COMPILER_TARGET: &str = "wasm32-unknown-emscripten";
pub(crate) const ADAPTER_ABI_VERSION: u64 = 2;
pub(crate) const SIMD_MODE: &str = "disabled";
pub(crate) const POINTER_WIDTH: u64 = 32;
pub(crate) const ENDIANNESS: &str = "little";
pub(crate) const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_MEMBER_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_TOTAL_MEMBER_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const PACKAGE_MEMBER_COUNT: usize = 7;

const STATEMENT_FIELDS: &[&str] = &[
    "schema_version",
    "schema",
    "repository",
    "workflow",
    "workflow_ref",
    "source_commit",
    "release_tag",
    "run_id",
    "run_attempt",
    "crate_version",
    "package_type",
    "package_name",
    "package_size",
    "package_sha256",
    "provider_abi",
    "target",
    "compiler_target",
    "precision",
    "upstream_sha",
    "source_tree",
    "effective_source_sha256",
    "adapter_abi_version",
    "adapter_source_sha256",
    "recording_contract_blake3",
    "validation_enabled",
    "simd",
    "pointer_width",
    "endianness",
    "emscripten_sdk_contract_sha256",
    "wasm_provider_contract_sha256",
    "bindings_sha256",
    "private_abi_hash",
    "snapshot_layout_hash",
    "member_count",
    "members",
];

const MEMBER_FIELDS: &[&str] = &["path", "size", "sha256"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WasmReleaseMember {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WasmReleaseContext<'a> {
    pub(crate) repository: &'a str,
    pub(crate) workflow: &'a str,
    pub(crate) workflow_ref: &'a str,
    pub(crate) source_commit: &'a str,
    pub(crate) release_tag: &'a str,
    pub(crate) run_id: &'a str,
    pub(crate) run_attempt: &'a str,
    pub(crate) crate_version: &'a str,
    pub(crate) precision: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WasmReleaseProvenanceStatement {
    pub(crate) schema_version: u64,
    pub(crate) schema: String,
    pub(crate) repository: String,
    pub(crate) workflow: String,
    pub(crate) workflow_ref: String,
    pub(crate) source_commit: String,
    pub(crate) release_tag: String,
    pub(crate) run_id: String,
    pub(crate) run_attempt: String,
    pub(crate) crate_version: String,
    pub(crate) package_type: String,
    pub(crate) package_name: String,
    pub(crate) package_size: u64,
    pub(crate) package_sha256: String,
    pub(crate) provider_abi: String,
    pub(crate) target: String,
    pub(crate) compiler_target: String,
    pub(crate) precision: String,
    pub(crate) upstream_sha: String,
    pub(crate) source_tree: String,
    pub(crate) effective_source_sha256: String,
    pub(crate) adapter_abi_version: u64,
    pub(crate) adapter_source_sha256: String,
    pub(crate) recording_contract_blake3: String,
    pub(crate) validation_enabled: bool,
    pub(crate) simd: String,
    pub(crate) pointer_width: u64,
    pub(crate) endianness: String,
    pub(crate) emscripten_sdk_contract_sha256: String,
    pub(crate) wasm_provider_contract_sha256: String,
    pub(crate) bindings_sha256: String,
    pub(crate) private_abi_hash: String,
    pub(crate) snapshot_layout_hash: u64,
    pub(crate) member_count: u64,
    pub(crate) members: Vec<WasmReleaseMember>,
}

impl WasmReleaseProvenanceStatement {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        let source = std::str::from_utf8(bytes)
            .map_err(|error| format!("WASM release provenance is not UTF-8: {error}"))?;
        let value: toml::Value = toml::from_str(source)
            .map_err(|error| format!("WASM release provenance is not valid TOML: {error}"))?;
        let table = value
            .as_table()
            .ok_or_else(|| "WASM release provenance root must be a TOML table".to_owned())?;
        require_exact_fields(table, STATEMENT_FIELDS, "WASM release provenance statement")?;
        let statement = Self {
            schema_version: required_integer(table, "schema_version")?,
            schema: required_string(table, "schema")?,
            repository: required_string(table, "repository")?,
            workflow: required_string(table, "workflow")?,
            workflow_ref: required_string(table, "workflow_ref")?,
            source_commit: required_string(table, "source_commit")?,
            release_tag: required_string(table, "release_tag")?,
            run_id: required_string(table, "run_id")?,
            run_attempt: required_string(table, "run_attempt")?,
            crate_version: required_string(table, "crate_version")?,
            package_type: required_string(table, "package_type")?,
            package_name: required_string(table, "package_name")?,
            package_size: required_integer(table, "package_size")?,
            package_sha256: required_string(table, "package_sha256")?,
            provider_abi: required_string(table, "provider_abi")?,
            target: required_string(table, "target")?,
            compiler_target: required_string(table, "compiler_target")?,
            precision: required_string(table, "precision")?,
            upstream_sha: required_string(table, "upstream_sha")?,
            source_tree: required_string(table, "source_tree")?,
            effective_source_sha256: required_string(table, "effective_source_sha256")?,
            adapter_abi_version: required_integer(table, "adapter_abi_version")?,
            adapter_source_sha256: required_string(table, "adapter_source_sha256")?,
            recording_contract_blake3: required_string(table, "recording_contract_blake3")?,
            validation_enabled: required_bool(table, "validation_enabled")?,
            simd: required_string(table, "simd")?,
            pointer_width: required_integer(table, "pointer_width")?,
            endianness: required_string(table, "endianness")?,
            emscripten_sdk_contract_sha256: required_string(
                table,
                "emscripten_sdk_contract_sha256",
            )?,
            wasm_provider_contract_sha256: required_string(table, "wasm_provider_contract_sha256")?,
            bindings_sha256: required_string(table, "bindings_sha256")?,
            private_abi_hash: required_string(table, "private_abi_hash")?,
            snapshot_layout_hash: required_integer(table, "snapshot_layout_hash")?,
            member_count: required_integer(table, "member_count")?,
            members: required_members(table)?,
        };
        statement.validate_intrinsic()?;
        Ok(statement)
    }

    pub(crate) fn parse_canonical(bytes: &[u8]) -> Result<Self, String> {
        let statement = Self::parse(bytes)?;
        if statement.render().as_slice() != bytes {
            return Err(
                "WASM release provenance is not in its canonical byte representation".to_owned(),
            );
        }
        Ok(statement)
    }

    pub(crate) fn parse_canonical_for_package(
        bytes: &[u8],
        expected: WasmReleaseContext<'_>,
        package: &[u8],
    ) -> Result<Self, String> {
        let statement = Self::parse_canonical(bytes)?;
        statement.validate_release_context(expected)?;
        statement.verify_package_bytes(package)?;
        Ok(statement)
    }

    fn render(&self) -> Vec<u8> {
        let mut rendered = format!(
            concat!(
                "schema_version = {}\n",
                "schema = {}\n",
                "repository = {}\n",
                "workflow = {}\n",
                "workflow_ref = {}\n",
                "source_commit = {}\n",
                "release_tag = {}\n",
                "run_id = {}\n",
                "run_attempt = {}\n",
                "crate_version = {}\n",
                "package_type = {}\n",
                "package_name = {}\n",
                "package_size = {}\n",
                "package_sha256 = {}\n",
                "provider_abi = {}\n",
                "target = {}\n",
                "compiler_target = {}\n",
                "precision = {}\n",
                "upstream_sha = {}\n",
                "source_tree = {}\n",
                "effective_source_sha256 = {}\n",
                "adapter_abi_version = {}\n",
                "adapter_source_sha256 = {}\n",
                "recording_contract_blake3 = {}\n",
                "validation_enabled = {}\n",
                "simd = {}\n",
                "pointer_width = {}\n",
                "endianness = {}\n",
                "emscripten_sdk_contract_sha256 = {}\n",
                "wasm_provider_contract_sha256 = {}\n",
                "bindings_sha256 = {}\n",
                "private_abi_hash = {}\n",
                "snapshot_layout_hash = {}\n",
                "member_count = {}\n",
            ),
            self.schema_version,
            toml_string(&self.schema),
            toml_string(&self.repository),
            toml_string(&self.workflow),
            toml_string(&self.workflow_ref),
            toml_string(&self.source_commit),
            toml_string(&self.release_tag),
            toml_string(&self.run_id),
            toml_string(&self.run_attempt),
            toml_string(&self.crate_version),
            toml_string(&self.package_type),
            toml_string(&self.package_name),
            self.package_size,
            toml_string(&self.package_sha256),
            toml_string(&self.provider_abi),
            toml_string(&self.target),
            toml_string(&self.compiler_target),
            toml_string(&self.precision),
            toml_string(&self.upstream_sha),
            toml_string(&self.source_tree),
            toml_string(&self.effective_source_sha256),
            self.adapter_abi_version,
            toml_string(&self.adapter_source_sha256),
            toml_string(&self.recording_contract_blake3),
            self.validation_enabled,
            toml_string(&self.simd),
            self.pointer_width,
            toml_string(&self.endianness),
            toml_string(&self.emscripten_sdk_contract_sha256),
            toml_string(&self.wasm_provider_contract_sha256),
            toml_string(&self.bindings_sha256),
            toml_string(&self.private_abi_hash),
            self.snapshot_layout_hash,
            self.member_count,
        );
        for member in &self.members {
            rendered.push_str(&format!(
                "\n[[members]]\npath = {}\nsize = {}\nsha256 = {}\n",
                toml_string(&member.path),
                member.size,
                toml_string(&member.sha256),
            ));
        }
        rendered.into_bytes()
    }

    pub(crate) fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate_intrinsic()?;
        Ok(self.render())
    }

    pub(crate) fn validate_intrinsic(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION || self.schema != SCHEMA_NAME {
            return Err(format!(
                "unsupported WASM release provenance schema: version={} name={:?}",
                self.schema_version, self.schema
            ));
        }
        validate_repository(&self.repository)?;
        if self.workflow != PUBLISHER_WORKFLOW {
            return Err(format!(
                "unsupported WASM release publisher workflow {:?}",
                self.workflow
            ));
        }
        validate_git_sha("source_commit", &self.source_commit)?;
        validate_git_sha("upstream_sha", &self.upstream_sha)?;
        validate_git_sha("source_tree", &self.source_tree)?;
        validate_positive_decimal("run_id", &self.run_id)?;
        validate_positive_decimal("run_attempt", &self.run_attempt)?;
        validate_release_tag(&self.crate_version, &self.release_tag)?;
        let expected_workflow_ref = format!(
            "{}/{}@refs/tags/{}",
            self.repository, self.workflow, self.release_tag
        );
        if self.workflow_ref != expected_workflow_ref {
            return Err(format!(
                "WASM release workflow_ref {:?} does not match {:?}",
                self.workflow_ref, expected_workflow_ref
            ));
        }

        if self.package_type != PACKAGE_TYPE
            || self.provider_abi != PROVIDER_ABI
            || self.target != TARGET
            || self.compiler_target != COMPILER_TARGET
        {
            return Err(
                "WASM release package/provider/target coordinates are unsupported".to_owned(),
            );
        }
        if !matches!(self.precision.as_str(), "single" | "double") {
            return Err(format!(
                "unsupported WASM release precision {:?}",
                self.precision
            ));
        }
        if self.adapter_abi_version != ADAPTER_ABI_VERSION
            || self.validation_enabled
            || self.simd != SIMD_MODE
            || self.pointer_width != POINTER_WIDTH
            || self.endianness != ENDIANNESS
        {
            return Err("WASM release runtime ABI coordinates are unsupported".to_owned());
        }
        if self.snapshot_layout_hash == 0 || u32::try_from(self.snapshot_layout_hash).is_err() {
            return Err(
                "WASM release snapshot_layout_hash must be a non-zero u32 value".to_owned(),
            );
        }

        let expected_package_name = format!(
            "boxdd-wasm-provider-{}-{TARGET}-{}.tar.gz",
            self.crate_version, self.precision
        );
        if self.package_name != expected_package_name {
            return Err(format!(
                "WASM release package name {:?} does not match {:?}",
                self.package_name, expected_package_name
            ));
        }
        if self.package_size == 0 || self.package_size > MAX_PACKAGE_BYTES {
            return Err(format!(
                "WASM release package_size must be in 1..={MAX_PACKAGE_BYTES}"
            ));
        }

        validate_sha256("package_sha256", &self.package_sha256)?;
        validate_sha256("effective_source_sha256", &self.effective_source_sha256)?;
        validate_sha256("adapter_source_sha256", &self.adapter_source_sha256)?;
        validate_blake3("recording_contract_blake3", &self.recording_contract_blake3)?;
        validate_sha256(
            "emscripten_sdk_contract_sha256",
            &self.emscripten_sdk_contract_sha256,
        )?;
        validate_sha256(
            "wasm_provider_contract_sha256",
            &self.wasm_provider_contract_sha256,
        )?;
        validate_sha256("bindings_sha256", &self.bindings_sha256)?;
        validate_sha256("private_abi_hash", &self.private_abi_hash)?;
        if self.private_abi_hash.bytes().all(|byte| byte == b'0') {
            return Err("WASM release private_abi_hash must be non-zero".to_owned());
        }

        if self.member_count != self.members.len() as u64
            || self.members.len() != PACKAGE_MEMBER_COUNT
        {
            return Err(
                "WASM release member_count does not match the fixed package inventory".to_owned(),
            );
        }
        let expected_paths = expected_member_paths(&self.precision);
        if !self
            .members
            .iter()
            .map(|member| member.path.as_str())
            .eq(expected_paths.iter().map(String::as_str))
        {
            return Err(format!(
                "WASM release package members do not match the fixed inventory for {:?}",
                self.precision
            ));
        }

        let mut previous = None::<&str>;
        let mut total_size = 0_u64;
        for member in &self.members {
            validate_relative_path(&member.path)?;
            validate_sha256("members.sha256", &member.sha256)?;
            if member.size == 0 || member.size > MAX_MEMBER_BYTES {
                return Err(format!(
                    "WASM release member {:?} must be in 1..={MAX_MEMBER_BYTES} bytes",
                    member.path
                ));
            }
            total_size = total_size
                .checked_add(member.size)
                .filter(|total| *total <= MAX_TOTAL_MEMBER_BYTES)
                .ok_or_else(|| {
                    format!(
                        "WASM release inventory exceeds the {MAX_TOTAL_MEMBER_BYTES} byte limit"
                    )
                })?;
            if previous.is_some_and(|path| path >= member.path.as_str()) {
                return Err(format!(
                    "WASM release member inventory is duplicated or unsorted at {:?}",
                    member.path
                ));
            }
            previous = Some(&member.path);
        }

        let checksums = canonical_inner_checksums_bytes(&self.members)?;
        let checksums_member = self.member("checksums.sha256")?;
        if checksums_member.size != checksums.len() as u64
            || checksums_member.sha256 != sha256_bytes(&checksums)
        {
            return Err(
                "WASM release checksums member is not bound to the canonical inventory".to_owned(),
            );
        }
        Ok(())
    }

    pub(crate) fn validate_publisher(
        &self,
        expected_repository: &str,
        expected_workflow: &str,
    ) -> Result<(), String> {
        self.validate_intrinsic()?;
        if self.repository == expected_repository && self.workflow == expected_workflow {
            Ok(())
        } else {
            Err(format!(
                "WASM release publisher {}/{} does not match trusted publisher {expected_repository}/{expected_workflow}",
                self.repository, self.workflow
            ))
        }
    }

    pub(crate) fn validate_release_context(
        &self,
        expected: WasmReleaseContext<'_>,
    ) -> Result<(), String> {
        self.validate_intrinsic()?;
        if self.repository == expected.repository
            && self.workflow == expected.workflow
            && self.workflow_ref == expected.workflow_ref
            && self.source_commit == expected.source_commit
            && self.release_tag == expected.release_tag
            && self.run_id == expected.run_id
            && self.run_attempt == expected.run_attempt
            && self.crate_version == expected.crate_version
            && self.precision == expected.precision
        {
            Ok(())
        } else {
            Err(format!(
                "WASM release context does not match repository/workflow/commit/tag/run/version/precision {}/{}/{}/{}/{}/{}/{}/{}",
                expected.repository,
                expected.workflow_ref,
                expected.source_commit,
                expected.release_tag,
                expected.run_id,
                expected.run_attempt,
                expected.crate_version,
                expected.precision,
            ))
        }
    }

    pub(crate) fn verify_package_bytes(&self, bytes: &[u8]) -> Result<(), String> {
        self.validate_intrinsic()?;
        if bytes.len() as u64 != self.package_size {
            return Err(format!(
                "WASM release package size mismatch: statement={} actual={}",
                self.package_size,
                bytes.len()
            ));
        }
        let actual = sha256_bytes(bytes);
        if actual != self.package_sha256 {
            return Err(format!(
                "WASM release package digest mismatch: statement={} actual={actual}",
                self.package_sha256
            ));
        }
        Ok(())
    }

    pub(crate) fn verify_members(&self, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
        self.validate_intrinsic()?;
        let actual = members_from_files(files)?;
        if actual == self.members {
            Ok(())
        } else {
            Err("WASM release package contents do not match the signed inventory".to_owned())
        }
    }

    pub(crate) fn member(&self, path: &str) -> Result<&WasmReleaseMember, String> {
        self.members
            .binary_search_by(|member| member.path.as_str().cmp(path))
            .map(|index| &self.members[index])
            .map_err(|_| format!("signed WASM release inventory is missing {path:?}"))
    }
}

pub(crate) fn members_from_files(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<WasmReleaseMember>, String> {
    files
        .iter()
        .map(|(path, bytes)| {
            validate_relative_path(path)?;
            Ok(WasmReleaseMember {
                path: path.clone(),
                size: bytes.len() as u64,
                sha256: sha256_bytes(bytes),
            })
        })
        .collect()
}

pub(crate) fn canonical_inner_checksums_bytes(
    members: &[WasmReleaseMember],
) -> Result<Vec<u8>, String> {
    let mut rendered = String::new();
    let mut previous = None::<&str>;
    for member in members {
        validate_relative_path(&member.path)?;
        validate_sha256("members.sha256", &member.sha256)?;
        if previous.is_some_and(|path| path >= member.path.as_str()) {
            return Err(
                "cannot render WASM checksums from duplicated or unsorted members".to_owned(),
            );
        }
        previous = Some(&member.path);
        if member.path != "checksums.sha256" {
            rendered.push_str(&format!("{}  {}\n", member.sha256, member.path));
        }
    }
    Ok(rendered.into_bytes())
}

fn required_members(table: &toml::Table) -> Result<Vec<WasmReleaseMember>, String> {
    table
        .get("members")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            "WASM release provenance field `members` must be an array of tables".to_owned()
        })?
        .iter()
        .map(|value| {
            let member = value.as_table().ok_or_else(|| {
                "WASM release provenance field `members` must contain only tables".to_owned()
            })?;
            require_exact_fields(member, MEMBER_FIELDS, "WASM release provenance member")?;
            Ok(WasmReleaseMember {
                path: required_string(member, "path")?,
                size: required_integer(member, "size")?,
                sha256: required_string(member, "sha256")?,
            })
        })
        .collect()
}

fn require_exact_fields(table: &toml::Table, expected: &[&str], label: &str) -> Result<(), String> {
    let actual = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} fields do not match the closed schema: expected {expected:?}, found {actual:?}"
        ))
    }
}

fn required_string(table: &toml::Table, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("WASM release provenance field `{key}` must be a non-empty string"))
}

fn required_integer(table: &toml::Table, key: &str) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            format!("WASM release provenance field `{key}` must be a non-negative integer")
        })
}

fn required_bool(table: &toml::Table, key: &str) -> Result<bool, String> {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| format!("WASM release provenance field `{key}` must be a boolean"))
}

fn validate_release_tag(crate_version: &str, release_tag: &str) -> Result<(), String> {
    if !is_canonical_semver(crate_version) {
        return Err("WASM release crate_version is not canonical".to_owned());
    }
    let workspace_tag = format!("v{crate_version}");
    let sys_tag = format!("boxdd-sys-v{crate_version}");
    if release_tag == workspace_tag || release_tag == sys_tag {
        Ok(())
    } else {
        Err(format!(
            "WASM release tag {release_tag:?} does not match crate version {crate_version}"
        ))
    }
}

pub(crate) fn is_canonical_semver(version: &str) -> bool {
    let (without_build, build) = match version.split_once('+') {
        Some((left, right)) if !right.contains('+') => (left, Some(right)),
        Some(_) => return false,
        None => (version, None),
    };
    let (core, prerelease) = match without_build.split_once('-') {
        Some((left, right)) => (left, Some(right)),
        None => (without_build, None),
    };
    let core = core.split('.').collect::<Vec<_>>();
    if core.len() != 3
        || !core.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (*part == "0" || !part.starts_with('0'))
        })
    {
        return false;
    }
    let identifiers_are_canonical = |value: &str, reject_numeric_leading_zero: bool| {
        !value.is_empty()
            && value.split('.').all(|identifier| {
                !identifier.is_empty()
                    && identifier
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    && (!reject_numeric_leading_zero
                        || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                        || identifier == "0"
                        || !identifier.starts_with('0'))
            })
    };
    prerelease.is_none_or(|value| identifiers_are_canonical(value, true))
        && build.is_none_or(|value| identifiers_are_canonical(value, false))
}

fn validate_repository(repository: &str) -> Result<(), String> {
    let mut components = repository.split('/');
    let owner = components.next().unwrap_or_default();
    let name = components.next().unwrap_or_default();
    let valid_component = |value: &str| {
        !value.is_empty()
            && !matches!(value, "." | "..")
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    if valid_component(owner) && valid_component(name) && components.next().is_none() {
        Ok(())
    } else {
        Err(format!(
            "WASM release repository {repository:?} is not canonical"
        ))
    }
}

fn validate_positive_decimal(label: &str, value: &str) -> Result<(), String> {
    if !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && !value.starts_with('0')
    {
        Ok(())
    } else {
        Err(format!(
            "WASM release field `{label}` must be a positive canonical decimal"
        ))
    }
}

fn validate_git_sha(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 40 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "WASM release field `{label}` is not a lowercase Git SHA"
        ))
    }
}

fn validate_sha256(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "WASM release field `{label}` is not a SHA-256 digest"
        ))
    }
}

fn validate_blake3(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "WASM release field `{label}` is not a BLAKE3 digest"
        ))
    }
}

pub(crate) fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if !is_portable_normalized_relative_path(path) {
        Err(format!(
            "WASM release member path {path:?} is not a portable normalized relative path"
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn is_portable_normalized_relative_path(path: &str) -> bool {
    let portable_ascii = path
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'));
    !path.is_empty()
        && portable_ascii
        && !path.contains("//")
        && !path.starts_with("./")
        && !path.ends_with('/')
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && !path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
}

fn expected_member_paths(precision: &str) -> [String; PACKAGE_MEMBER_COUNT] {
    [
        "checksums.sha256".to_owned(),
        "licenses/BOX2D-LICENSE".to_owned(),
        "licenses/PROJECT-LICENSE-APACHE".to_owned(),
        "licenses/PROJECT-LICENSE-MIT".to_owned(),
        "manifest.toml".to_owned(),
        format!("provider/{PROVIDER_ABI}-{precision}.js"),
        format!("provider/{PROVIDER_ABI}-{precision}.wasm"),
    ]
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_context(precision: &str) -> WasmReleaseContext<'_> {
        WasmReleaseContext {
            repository: "Latias94/boxdd",
            workflow: PUBLISHER_WORKFLOW,
            workflow_ref: "Latias94/boxdd/.github/workflows/prebuilt-binaries.yml@refs/tags/v0.6.0",
            source_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            release_tag: "v0.6.0",
            run_id: "123456",
            run_attempt: "1",
            crate_version: "0.6.0",
            precision,
        }
    }

    fn fixture(precision: &str) -> (WasmReleaseProvenanceStatement, BTreeMap<String, Vec<u8>>) {
        let mut files = BTreeMap::from([
            (
                "licenses/BOX2D-LICENSE".to_owned(),
                b"Box2D license\n".to_vec(),
            ),
            (
                "licenses/PROJECT-LICENSE-APACHE".to_owned(),
                b"Apache license\n".to_vec(),
            ),
            (
                "licenses/PROJECT-LICENSE-MIT".to_owned(),
                b"MIT license\n".to_vec(),
            ),
            ("manifest.toml".to_owned(), b"manifest\n".to_vec()),
            (
                format!("provider/{PROVIDER_ABI}-{precision}.js"),
                b"provider JavaScript\n".to_vec(),
            ),
            (
                format!("provider/{PROVIDER_ABI}-{precision}.wasm"),
                b"provider WebAssembly\n".to_vec(),
            ),
        ]);
        let checksums = canonical_inner_checksums_bytes(&members_from_files(&files).unwrap())
            .expect("canonical checksums");
        files.insert("checksums.sha256".to_owned(), checksums);
        let members = members_from_files(&files).unwrap();
        let package = b"canonical WASM provider package";
        let statement = WasmReleaseProvenanceStatement {
            schema_version: SCHEMA_VERSION,
            schema: SCHEMA_NAME.to_owned(),
            repository: "Latias94/boxdd".to_owned(),
            workflow: PUBLISHER_WORKFLOW.to_owned(),
            workflow_ref: format!("Latias94/boxdd/{PUBLISHER_WORKFLOW}@refs/tags/v0.6.0"),
            source_commit: "a".repeat(40),
            release_tag: "v0.6.0".to_owned(),
            run_id: "123456".to_owned(),
            run_attempt: "1".to_owned(),
            crate_version: "0.6.0".to_owned(),
            package_type: PACKAGE_TYPE.to_owned(),
            package_name: format!("boxdd-wasm-provider-0.6.0-{TARGET}-{precision}.tar.gz"),
            package_size: package.len() as u64,
            package_sha256: sha256_bytes(package),
            provider_abi: PROVIDER_ABI.to_owned(),
            target: TARGET.to_owned(),
            compiler_target: COMPILER_TARGET.to_owned(),
            precision: precision.to_owned(),
            upstream_sha: "b".repeat(40),
            source_tree: "c".repeat(40),
            effective_source_sha256: "d".repeat(64),
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: "e".repeat(64),
            recording_contract_blake3: "f".repeat(64),
            validation_enabled: false,
            simd: SIMD_MODE.to_owned(),
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS.to_owned(),
            emscripten_sdk_contract_sha256: "1".repeat(64),
            wasm_provider_contract_sha256: "2".repeat(64),
            bindings_sha256: "3".repeat(64),
            private_abi_hash: "4".repeat(64),
            snapshot_layout_hash: 0x1234_5678,
            member_count: members.len() as u64,
            members,
        };
        (statement, files)
    }

    #[test]
    fn canonical_statement_round_trips_for_both_precisions() {
        for precision in ["single", "double"] {
            let (statement, files) = fixture(precision);
            let bytes = statement.canonical_bytes().unwrap();
            assert_eq!(
                WasmReleaseProvenanceStatement::parse_canonical(&bytes).unwrap(),
                statement
            );
            assert_eq!(
                WasmReleaseProvenanceStatement::parse_canonical_for_package(
                    &bytes,
                    expected_context(precision),
                    b"canonical WASM provider package",
                )
                .unwrap(),
                statement
            );
            assert!(
                WasmReleaseProvenanceStatement::parse_canonical_for_package(
                    &bytes,
                    WasmReleaseContext {
                        run_attempt: "2",
                        ..expected_context(precision)
                    },
                    b"canonical WASM provider package",
                )
                .is_err()
            );
            assert!(
                WasmReleaseProvenanceStatement::parse_canonical_for_package(
                    &bytes,
                    expected_context(precision),
                    b"different package",
                )
                .is_err()
            );
            statement
                .validate_publisher("Latias94/boxdd", PUBLISHER_WORKFLOW)
                .unwrap();
            statement
                .validate_release_context(expected_context(precision))
                .unwrap();
            statement.verify_members(&files).unwrap();
            assert!(statement.member("manifest.toml").is_ok());
        }
    }

    #[test]
    fn parser_rejects_unknown_duplicate_missing_and_noncanonical_fields() {
        let (statement, _) = fixture("single");
        let source = String::from_utf8(statement.render()).unwrap();
        let unknown = source.replacen("\n[[members]]", "\nunknown = true\n\n[[members]]", 1);
        assert!(WasmReleaseProvenanceStatement::parse(unknown.as_bytes()).is_err());

        let duplicate = source.replacen(
            "schema_version = 1\n",
            "schema_version = 1\nschema_version = 1\n",
            1,
        );
        assert!(WasmReleaseProvenanceStatement::parse(duplicate.as_bytes()).is_err());

        let duplicate_member = source.replacen(
            "path = \"checksums.sha256\"\n",
            "path = \"checksums.sha256\"\npath = \"checksums.sha256\"\n",
            1,
        );
        assert!(WasmReleaseProvenanceStatement::parse(duplicate_member.as_bytes()).is_err());

        let missing = source.replacen("package_type = \"wasm-provider\"\n", "", 1);
        assert!(WasmReleaseProvenanceStatement::parse(missing.as_bytes()).is_err());

        let unknown_member = source.replacen(
            "path = \"checksums.sha256\"\n",
            "path = \"checksums.sha256\"\nrole = \"checksums\"\n",
            1,
        );
        assert!(WasmReleaseProvenanceStatement::parse(unknown_member.as_bytes()).is_err());

        let reformatted = format!("\n{source}");
        assert!(WasmReleaseProvenanceStatement::parse(reformatted.as_bytes()).is_ok());
        assert!(WasmReleaseProvenanceStatement::parse_canonical(reformatted.as_bytes()).is_err());
    }

    #[test]
    fn publisher_and_release_context_are_fail_closed() {
        let (statement, _) = fixture("single");
        assert!(
            statement
                .validate_publisher("other/boxdd", PUBLISHER_WORKFLOW)
                .is_err()
        );
        assert!(
            statement
                .validate_release_context(WasmReleaseContext {
                    source_commit: "9999999999999999999999999999999999999999",
                    ..expected_context("single")
                })
                .is_err()
        );
        assert!(
            statement
                .validate_release_context(expected_context("double"))
                .is_err()
        );
        assert!(
            statement
                .validate_release_context(WasmReleaseContext {
                    run_id: "654321",
                    ..expected_context("single")
                })
                .is_err()
        );
        assert!(
            statement
                .validate_release_context(WasmReleaseContext {
                    run_attempt: "2",
                    ..expected_context("single")
                })
                .is_err()
        );

        let mut branch_replay = statement.clone();
        branch_replay.workflow_ref =
            "Latias94/boxdd/.github/workflows/prebuilt-binaries.yml@refs/heads/main".to_owned();
        assert!(branch_replay.validate_intrinsic().is_err());

        let mut old_commit = statement;
        old_commit.source_commit = "9".repeat(40);
        assert!(
            old_commit
                .validate_release_context(expected_context("single"))
                .is_err()
        );
    }

    #[test]
    fn release_coordinates_require_semver_and_normalized_repository_slug() {
        let (statement, _) = fixture("single");
        for version in [".", "0.6", "00.6.0", "0.6.0-01", "0.6.0+"] {
            let mut invalid = statement.clone();
            invalid.crate_version = version.to_owned();
            invalid.release_tag = format!("v{version}");
            invalid.package_name = format!("boxdd-wasm-provider-{version}-{TARGET}-single.tar.gz");
            invalid.workflow_ref = format!(
                "Latias94/boxdd/{PUBLISHER_WORKFLOW}@refs/tags/{}",
                invalid.release_tag
            );
            assert!(
                invalid.validate_intrinsic().is_err(),
                "accepted {version:?}"
            );
        }
        for repository in ["../boxdd", "Latias94/..", "./boxdd", "Latias94/."] {
            let mut invalid = statement.clone();
            invalid.repository = repository.to_owned();
            invalid.workflow_ref = format!(
                "{repository}/{PUBLISHER_WORKFLOW}@refs/tags/{}",
                invalid.release_tag
            );
            assert!(
                invalid.validate_intrinsic().is_err(),
                "accepted {repository:?}"
            );
        }
    }

    #[test]
    fn package_and_runtime_coordinates_are_closed() {
        let (statement, _) = fixture("single");
        let mut cases = Vec::new();

        let mut package_type = statement.clone();
        package_type.package_type = "pages-runtime".to_owned();
        cases.push(package_type);

        let mut provider_abi = statement.clone();
        provider_abi.provider_abi = "box2d-sys-v2".to_owned();
        cases.push(provider_abi);

        let mut target = statement.clone();
        target.target = COMPILER_TARGET.to_owned();
        cases.push(target);

        let mut compiler_target = statement.clone();
        compiler_target.compiler_target = TARGET.to_owned();
        cases.push(compiler_target);

        let mut adapter_abi = statement.clone();
        adapter_abi.adapter_abi_version += 1;
        cases.push(adapter_abi);

        let mut validation = statement.clone();
        validation.validation_enabled = true;
        cases.push(validation);

        let mut simd = statement.clone();
        simd.simd = "default".to_owned();
        cases.push(simd);

        let mut pointer_width = statement.clone();
        pointer_width.pointer_width = 64;
        cases.push(pointer_width);

        let mut endianness = statement.clone();
        endianness.endianness = "big".to_owned();
        cases.push(endianness);

        let mut layout = statement.clone();
        layout.snapshot_layout_hash = 0;
        cases.push(layout);

        let mut private_abi = statement;
        private_abi.private_abi_hash = "0".repeat(64);
        cases.push(private_abi);

        for changed in cases {
            assert!(changed.validate_intrinsic().is_err());
        }
    }

    #[test]
    fn package_name_and_precision_must_move_together() {
        let (statement, _) = fixture("single");
        let mut precision_only = statement.clone();
        precision_only.precision = "double".to_owned();
        assert!(precision_only.validate_intrinsic().is_err());

        let (double, _) = fixture("double");
        assert!(double.validate_intrinsic().is_ok());

        let mut unsupported = statement;
        unsupported.precision = "extended".to_owned();
        unsupported.package_name = format!("boxdd-wasm-provider-0.6.0-{TARGET}-extended.tar.gz");
        assert!(unsupported.validate_intrinsic().is_err());
    }

    #[test]
    fn package_and_exact_member_tampering_fail_closed() {
        let (statement, files) = fixture("single");
        assert!(
            statement
                .verify_package_bytes(b"canonical WASM provider package")
                .is_ok()
        );
        assert!(statement.verify_package_bytes(b"changed package").is_err());

        let mut changed = files.clone();
        changed.insert("manifest.toml".to_owned(), b"changed\n".to_vec());
        assert!(statement.verify_members(&changed).is_err());

        let mut extra = files.clone();
        extra.insert("provider/unexpected.wasm".to_owned(), b"extra".to_vec());
        assert!(statement.verify_members(&extra).is_err());

        let mut missing = files;
        missing.remove("licenses/PROJECT-LICENSE-MIT");
        assert!(statement.verify_members(&missing).is_err());
    }

    #[test]
    fn inventory_order_topology_and_checksums_are_intrinsic() {
        let (statement, _) = fixture("single");

        let mut unsorted = statement.clone();
        unsorted.members.swap(0, 1);
        assert!(unsorted.validate_intrinsic().is_err());

        let mut duplicate = statement.clone();
        duplicate.members[1] = duplicate.members[0].clone();
        assert!(duplicate.validate_intrinsic().is_err());

        let mut wrong_path = statement.clone();
        wrong_path.members[4].path = "./manifest.toml".to_owned();
        assert!(wrong_path.validate_intrinsic().is_err());

        let mut checksum_digest = statement.clone();
        checksum_digest.members[0].sha256 = "9".repeat(64);
        assert!(checksum_digest.validate_intrinsic().is_err());

        let mut checksum_size = statement;
        checksum_size.members[0].size += 1;
        assert!(checksum_size.validate_intrinsic().is_err());
    }
}
