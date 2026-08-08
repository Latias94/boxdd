//! Canonical, signed provenance for one official prebuilt provider package.
//!
//! This module is shared by the release validator, the fresh-consumer qualifier,
//! and `boxdd-sys`'s build script. It performs no network access, extraction, or
//! process execution.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

use crate::build_support::VerifiedFileSnapshot;
use crate::{
    provenance_policy::release_tag_matches_version,
    provider_catalog::ProviderCapability,
    provider_manifest::{ArtifactManifest, sha256_bytes},
};

pub const SCHEMA_VERSION: u64 = 1;
pub const SCHEMA_NAME: &str = "boxdd-sys-prebuilt-provenance-v1";
pub const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_MEMBER_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_TOTAL_MEMBER_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_MEMBERS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberDigest {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrebuiltProvenanceStatement {
    pub schema_version: u64,
    pub schema: String,
    pub repository: String,
    pub workflow: String,
    pub workflow_ref: String,
    pub source_commit: String,
    pub release_tag: String,
    pub run_id: String,
    pub run_attempt: String,
    pub crate_version: String,
    pub package_name: String,
    pub package_size: u64,
    pub package_sha256: String,
    pub provider_manifest_sha256: String,
    pub inner_checksums_sha256: String,
    pub provider: String,
    pub target: String,
    pub precision: String,
    pub link: String,
    pub crt: String,
    pub upstream_sha: String,
    pub effective_source_sha256: String,
    pub simd: String,
    pub validate: bool,
    pub adapter_abi_version: u64,
    pub adapter_source_sha256: String,
    pub private_abi_hash: String,
    pub snapshot_layout_hash: u64,
    pub recording_contract_blake3: String,
    pub member_count: u64,
    pub members: Vec<MemberDigest>,
}

impl PrebuiltProvenanceStatement {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let source = std::str::from_utf8(bytes)
            .map_err(|error| format!("prebuilt provenance statement is not UTF-8: {error}"))?;
        let value: toml::Value = toml::from_str(source)
            .map_err(|error| format!("prebuilt provenance statement is not valid TOML: {error}"))?;
        let table = value
            .as_table()
            .ok_or_else(|| "prebuilt provenance statement root must be a TOML table".to_owned())?;
        reject_unknown_statement_fields(table)?;
        let members = required_members(table)?;
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
            package_name: required_string(table, "package_name")?,
            package_size: required_integer(table, "package_size")?,
            package_sha256: required_string(table, "package_sha256")?,
            provider_manifest_sha256: required_string(table, "provider_manifest_sha256")?,
            inner_checksums_sha256: required_string(table, "inner_checksums_sha256")?,
            provider: required_string(table, "provider")?,
            target: required_string(table, "target")?,
            precision: required_string(table, "precision")?,
            link: required_string(table, "link")?,
            crt: required_string(table, "crt")?,
            upstream_sha: required_string(table, "upstream_sha")?,
            effective_source_sha256: required_string(table, "effective_source_sha256")?,
            simd: required_string(table, "simd")?,
            validate: required_bool(table, "validate")?,
            adapter_abi_version: required_integer(table, "adapter_abi_version")?,
            adapter_source_sha256: required_string(table, "adapter_source_sha256")?,
            private_abi_hash: required_string(table, "private_abi_hash")?,
            snapshot_layout_hash: required_integer(table, "snapshot_layout_hash")?,
            recording_contract_blake3: required_string(table, "recording_contract_blake3")?,
            member_count: required_integer(table, "member_count")?,
            members,
        };
        statement.validate_intrinsic()?;
        Ok(statement)
    }

    pub fn parse_canonical(bytes: &[u8]) -> Result<Self, String> {
        let statement = Self::parse(bytes)?;
        if statement.render().as_slice() != bytes {
            return Err("prebuilt provenance statement is not in canonical byte form".to_owned());
        }
        Ok(statement)
    }

    pub fn render(&self) -> Vec<u8> {
        let mut rendered = format!(
            concat!(
                "schema_version = {}\n",
                "schema = {:?}\n",
                "repository = {:?}\n",
                "workflow = {:?}\n",
                "workflow_ref = {:?}\n",
                "source_commit = {:?}\n",
                "release_tag = {:?}\n",
                "run_id = {:?}\n",
                "run_attempt = {:?}\n",
                "crate_version = {:?}\n",
                "package_name = {:?}\n",
                "package_size = {}\n",
                "package_sha256 = {:?}\n",
                "provider_manifest_sha256 = {:?}\n",
                "inner_checksums_sha256 = {:?}\n",
                "provider = {:?}\n",
                "target = {:?}\n",
                "precision = {:?}\n",
                "link = {:?}\n",
                "crt = {:?}\n",
                "upstream_sha = {:?}\n",
                "effective_source_sha256 = {:?}\n",
                "simd = {:?}\n",
                "validate = {}\n",
                "adapter_abi_version = {}\n",
                "adapter_source_sha256 = {:?}\n",
                "private_abi_hash = {:?}\n",
                "snapshot_layout_hash = {}\n",
                "recording_contract_blake3 = {:?}\n",
                "member_count = {}\n",
            ),
            self.schema_version,
            self.schema,
            self.repository,
            self.workflow,
            self.workflow_ref,
            self.source_commit,
            self.release_tag,
            self.run_id,
            self.run_attempt,
            self.crate_version,
            self.package_name,
            self.package_size,
            self.package_sha256,
            self.provider_manifest_sha256,
            self.inner_checksums_sha256,
            self.provider,
            self.target,
            self.precision,
            self.link,
            self.crt,
            self.upstream_sha,
            self.effective_source_sha256,
            self.simd,
            self.validate,
            self.adapter_abi_version,
            self.adapter_source_sha256,
            self.private_abi_hash,
            self.snapshot_layout_hash,
            self.recording_contract_blake3,
            self.member_count,
        );
        for member in &self.members {
            rendered.push_str(&format!(
                "\n[[members]]\npath = {:?}\nsize = {}\nsha256 = {:?}\n",
                member.path, member.size, member.sha256,
            ));
        }
        rendered.into_bytes()
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate_intrinsic()?;
        Ok(self.render())
    }

    pub fn validate_intrinsic(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION || self.schema != SCHEMA_NAME {
            return Err(format!(
                "unsupported prebuilt provenance schema: version={} name={:?}",
                self.schema_version, self.schema
            ));
        }
        validate_repository(&self.repository)?;
        validate_relative_path(&self.workflow)?;
        validate_git_sha("source_commit", &self.source_commit)?;
        validate_git_sha("upstream_sha", &self.upstream_sha)?;
        validate_sha256("package_sha256", &self.package_sha256)?;
        validate_sha256("provider_manifest_sha256", &self.provider_manifest_sha256)?;
        validate_sha256("inner_checksums_sha256", &self.inner_checksums_sha256)?;
        validate_sha256("effective_source_sha256", &self.effective_source_sha256)?;
        validate_sha256("adapter_source_sha256", &self.adapter_source_sha256)?;
        validate_sha256("private_abi_hash", &self.private_abi_hash)?;
        validate_blake3("recording_contract_blake3", &self.recording_contract_blake3)?;
        validate_positive_decimal("run_id", &self.run_id)?;
        validate_positive_decimal("run_attempt", &self.run_attempt)?;
        validate_release_tag(&self.crate_version, &self.release_tag)?;
        let expected_workflow_ref = format!(
            "{}/{}@refs/tags/{}",
            self.repository, self.workflow, self.release_tag
        );
        if self.workflow_ref != expected_workflow_ref {
            return Err(format!(
                "prebuilt provenance workflow_ref {:?} does not match {:?}",
                self.workflow_ref, expected_workflow_ref
            ));
        }
        if self.provider != ProviderCapability::Prebuilt.as_str()
            || self.link != "static"
            || self.simd != "default"
            || self.validate
        {
            return Err(
                "prebuilt provenance provider/link/SIMD/validation coordinates are unsupported"
                    .to_owned(),
            );
        }
        if !matches!(self.precision.as_str(), "single" | "double") {
            return Err(format!(
                "unsupported prebuilt provenance precision {:?}",
                self.precision
            ));
        }
        validate_target_crt(&self.target, &self.crt)?;
        let crt_suffix = if self.crt == "none" {
            String::new()
        } else {
            format!("-{}", self.crt)
        };
        let expected_package_name = format!(
            "boxdd-prebuilt-{}-{}-{}-static{crt_suffix}.tar.gz",
            self.crate_version, self.target, self.precision
        );
        if self.package_name != expected_package_name {
            return Err(format!(
                "prebuilt provenance package name {:?} does not match {:?}",
                self.package_name, expected_package_name
            ));
        }
        if self.package_size == 0 || self.package_size > MAX_PACKAGE_BYTES {
            return Err(format!(
                "prebuilt provenance package_size must be in 1..={MAX_PACKAGE_BYTES}"
            ));
        }
        if self.member_count != self.members.len() as u64
            || self.members.is_empty()
            || self.members.len() > MAX_MEMBERS
        {
            return Err("prebuilt provenance member_count does not match its inventory".to_owned());
        }

        let mut previous = None::<&str>;
        let mut total_size = 0_u64;
        for member in &self.members {
            validate_relative_path(&member.path)?;
            validate_sha256("members.sha256", &member.sha256)?;
            if member.size > MAX_MEMBER_BYTES {
                return Err(format!(
                    "prebuilt provenance member {:?} exceeds the {MAX_MEMBER_BYTES} byte limit",
                    member.path
                ));
            }
            total_size = total_size
                .checked_add(member.size)
                .filter(|total| *total <= MAX_TOTAL_MEMBER_BYTES)
                .ok_or_else(|| {
                    format!(
                        "prebuilt provenance inventory exceeds the {MAX_TOTAL_MEMBER_BYTES} byte limit"
                    )
                })?;
            if previous.is_some_and(|path| path >= member.path.as_str()) {
                return Err(format!(
                    "prebuilt provenance member inventory is duplicated or unsorted at {:?}",
                    member.path
                ));
            }
            previous = Some(&member.path);
        }
        let manifest_member = self.member("manifest.toml")?;
        if manifest_member.sha256 != self.provider_manifest_sha256 {
            return Err(
                "provider_manifest_sha256 does not match the manifest inventory member".to_owned(),
            );
        }
        let checksums_member = self.member("checksums.sha256")?;
        if checksums_member.sha256 != self.inner_checksums_sha256 {
            return Err(
                "inner_checksums_sha256 does not match the checksums inventory member".to_owned(),
            );
        }
        let canonical_checksums = canonical_inner_checksums_bytes(&self.members)?;
        if sha256_bytes(&canonical_checksums) != self.inner_checksums_sha256 {
            return Err(
                "inner checksums member is not bound to the canonical inventory".to_owned(),
            );
        }
        Ok(())
    }

    pub fn validate_publisher(
        &self,
        expected_repository: &str,
        expected_workflow: &str,
    ) -> Result<(), String> {
        self.validate_intrinsic()?;
        if self.repository == expected_repository && self.workflow == expected_workflow {
            Ok(())
        } else {
            Err(format!(
                "prebuilt provenance publisher {}/{} does not match trusted publisher {expected_repository}/{expected_workflow}",
                self.repository, self.workflow
            ))
        }
    }

    pub fn verify_package_bytes(&self, bytes: &[u8]) -> Result<(), String> {
        self.validate_intrinsic()?;
        if bytes.len() as u64 != self.package_size {
            return Err(format!(
                "prebuilt package size mismatch: statement={} actual={}",
                self.package_size,
                bytes.len()
            ));
        }
        let actual = sha256_bytes(bytes);
        if actual != self.package_sha256 {
            return Err(format!(
                "prebuilt package digest mismatch: statement={} actual={actual}",
                self.package_sha256
            ));
        }
        Ok(())
    }

    pub fn verify_outer_package(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.validate_intrinsic()?;
        if path.file_name().and_then(|name| name.to_str()) != Some(self.package_name.as_str()) {
            return Err(format!(
                "prebuilt package filename does not match signed statement: {}",
                path.display()
            ));
        }
        let snapshot = VerifiedFileSnapshot::read(path, self.package_size, "prebuilt package")?;
        self.verify_package_bytes(snapshot.bytes())?;
        Ok(snapshot.into_bytes())
    }

    pub fn verify_extracted_root(&self, root: &Path) -> Result<(), String> {
        self.validate_intrinsic()?;
        let metadata = fs::symlink_metadata(root).map_err(|error| {
            format!(
                "failed to inspect extracted provider root {}: {error}",
                root.display()
            )
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "extracted provider root must be a real directory: {}",
                root.display()
            ));
        }

        let expected_files = self
            .members
            .iter()
            .map(|member| member.path.as_str())
            .collect::<BTreeSet<_>>();
        let expected_directories = expected_parent_directories(&self.members);
        let mut actual_files = BTreeSet::new();
        let mut stack = vec![(root.to_path_buf(), String::new())];
        while let Some((directory, prefix)) = stack.pop() {
            let entries = fs::read_dir(&directory).map_err(|error| {
                format!(
                    "failed to read extracted provider directory {}: {error}",
                    directory.display()
                )
            })?;
            for entry in entries {
                let entry = entry.map_err(|error| {
                    format!(
                        "failed to read extracted provider entry in {}: {error}",
                        directory.display()
                    )
                })?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "extracted provider path is not UTF-8".to_owned())?;
                let relative = if prefix.is_empty() {
                    name
                } else {
                    format!("{prefix}/{name}")
                };
                validate_relative_path(&relative)?;
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path).map_err(|error| {
                    format!(
                        "failed to inspect extracted provider path {}: {error}",
                        path.display()
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(format!(
                        "extracted provider tree contains a symlink: {}",
                        path.display()
                    ));
                }
                if metadata.is_dir() {
                    if !expected_directories.contains(relative.as_str()) {
                        return Err(format!(
                            "extracted provider tree contains unexpected directory {relative:?}"
                        ));
                    }
                    stack.push((path, relative));
                } else if metadata.is_file() {
                    if !expected_files.contains(relative.as_str()) {
                        return Err(format!(
                            "extracted provider tree contains unexpected file {relative:?}"
                        ));
                    }
                    let expected = self.member(&relative)?;
                    let snapshot = VerifiedFileSnapshot::read(
                        &path,
                        expected.size,
                        "extracted provider member",
                    )?;
                    if snapshot.len() as u64 != expected.size
                        || snapshot.sha256() != expected.sha256
                    {
                        return Err(format!(
                            "extracted provider member {relative:?} does not match signed provenance"
                        ));
                    }
                    actual_files.insert(relative);
                } else {
                    return Err(format!(
                        "extracted provider tree contains a special file: {}",
                        path.display()
                    ));
                }
            }
        }
        if actual_files.len() != expected_files.len()
            || actual_files
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != expected_files
        {
            return Err("extracted provider tree is missing signed inventory members".to_owned());
        }
        Ok(())
    }

    pub fn validate_provider_manifest(&self, bytes: &[u8]) -> Result<ArtifactManifest, String> {
        self.validate_intrinsic()?;
        if sha256_bytes(bytes) != self.provider_manifest_sha256 {
            return Err("provider manifest digest does not match signed provenance".to_owned());
        }
        let manifest = ArtifactManifest::parse(bytes)?;
        if manifest.render().as_slice() != bytes {
            return Err("provider manifest is not in canonical byte form".to_owned());
        }
        let source_commit = manifest
            .source_commit
            .as_deref()
            .ok_or_else(|| "prebuilt provider manifest is missing source_commit".to_owned())?;
        let release_tag = manifest
            .release_tag
            .as_deref()
            .ok_or_else(|| "prebuilt provider manifest is missing release_tag".to_owned())?;
        let coordinates_match = manifest.provider == self.provider
            && manifest.crate_version == self.crate_version
            && source_commit == self.source_commit
            && release_tag == self.release_tag
            && manifest.upstream_sha == self.upstream_sha
            && manifest.effective_source_sha256 == self.effective_source_sha256
            && manifest.precision == self.precision
            && manifest.target == self.target
            && manifest.link == self.link
            && manifest.crt == self.crt
            && manifest.simd == self.simd
            && manifest.validate == self.validate
            && manifest.adapter_abi_version == self.adapter_abi_version
            && manifest.adapter_source_sha256 == self.adapter_source_sha256
            && manifest.private_abi_hash == self.private_abi_hash
            && manifest.snapshot_layout_hash == self.snapshot_layout_hash
            && manifest.recording_contract_blake3 == self.recording_contract_blake3;
        if !coordinates_match {
            return Err("provider manifest identity does not match signed provenance".to_owned());
        }
        for (label, path, digest) in [
            (
                "archive",
                manifest.archive.as_str(),
                manifest.archive_sha256.as_str(),
            ),
            (
                "header",
                manifest.header.as_str(),
                manifest.header_sha256.as_str(),
            ),
            (
                "bindings",
                manifest.bindings.as_str(),
                manifest.bindings_sha256.as_str(),
            ),
        ] {
            let member = self.member(path).map_err(|_| {
                format!("provider manifest {label} path {path:?} is absent from signed inventory")
            })?;
            if member.sha256 != digest {
                return Err(format!(
                    "provider manifest {label} digest does not match signed inventory"
                ));
            }
        }
        Ok(manifest)
    }

    pub fn member(&self, path: &str) -> Result<&MemberDigest, String> {
        self.members
            .binary_search_by(|member| member.path.as_str().cmp(path))
            .map(|index| &self.members[index])
            .map_err(|_| format!("signed provenance inventory is missing {path:?}"))
    }
}

pub fn members_from_files(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<MemberDigest>, String> {
    files
        .iter()
        .map(|(path, bytes)| {
            validate_relative_path(path)?;
            Ok(MemberDigest {
                path: path.clone(),
                size: bytes.len() as u64,
                sha256: sha256_bytes(bytes),
            })
        })
        .collect()
}

pub fn canonical_inner_checksums_bytes(members: &[MemberDigest]) -> Result<Vec<u8>, String> {
    let mut rendered = String::new();
    let mut previous = None::<&str>;
    for member in members {
        validate_relative_path(&member.path)?;
        validate_sha256("members.sha256", &member.sha256)?;
        if previous.is_some_and(|path| path >= member.path.as_str()) {
            return Err("cannot render checksums from duplicated or unsorted members".to_owned());
        }
        previous = Some(&member.path);
        if member.path != "checksums.sha256" {
            rendered.push_str(&format!("{}  {}\n", member.sha256, member.path));
        }
    }
    Ok(rendered.into_bytes())
}

fn required_members(
    table: &toml::map::Map<String, toml::Value>,
) -> Result<Vec<MemberDigest>, String> {
    table
        .get("members")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "prebuilt provenance field `members` must be an array of tables".to_owned())?
        .iter()
        .map(|value| {
            let member = value.as_table().ok_or_else(|| {
                "prebuilt provenance field `members` must contain only tables".to_owned()
            })?;
            const FIELDS: &[&str] = &["path", "size", "sha256"];
            if let Some(field) = member
                .keys()
                .find(|field| !FIELDS.contains(&field.as_str()))
            {
                return Err(format!(
                    "prebuilt provenance member contains unsupported field `{field}`"
                ));
            }
            Ok(MemberDigest {
                path: required_string(member, "path")?,
                size: required_integer(member, "size")?,
                sha256: required_string(member, "sha256")?,
            })
        })
        .collect()
}

fn reject_unknown_statement_fields(
    table: &toml::map::Map<String, toml::Value>,
) -> Result<(), String> {
    const FIELDS: &[&str] = &[
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
        "package_name",
        "package_size",
        "package_sha256",
        "provider_manifest_sha256",
        "inner_checksums_sha256",
        "provider",
        "target",
        "precision",
        "link",
        "crt",
        "upstream_sha",
        "effective_source_sha256",
        "simd",
        "validate",
        "adapter_abi_version",
        "adapter_source_sha256",
        "private_abi_hash",
        "snapshot_layout_hash",
        "recording_contract_blake3",
        "member_count",
        "members",
    ];
    if let Some(field) = table.keys().find(|field| !FIELDS.contains(&field.as_str())) {
        return Err(format!(
            "prebuilt provenance statement contains unsupported field `{field}`"
        ));
    }
    Ok(())
}

fn required_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("prebuilt provenance field `{key}` must be a string"))
}

fn required_integer(table: &toml::map::Map<String, toml::Value>, key: &str) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| format!("prebuilt provenance field `{key}` must be a non-negative integer"))
}

fn required_bool(table: &toml::map::Map<String, toml::Value>, key: &str) -> Result<bool, String> {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| format!("prebuilt provenance field `{key}` must be a boolean"))
}

fn validate_release_tag(crate_version: &str, release_tag: &str) -> Result<(), String> {
    if crate_version.is_empty()
        || !crate_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err("prebuilt provenance crate_version is not canonical".to_owned());
    }
    if release_tag_matches_version(crate_version, release_tag) {
        Ok(())
    } else {
        Err(format!(
            "prebuilt provenance release tag {release_tag:?} does not match crate version {crate_version}"
        ))
    }
}

fn validate_repository(repository: &str) -> Result<(), String> {
    let mut components = repository.split('/');
    let owner = components.next().unwrap_or_default();
    let name = components.next().unwrap_or_default();
    let valid_component = |value: &str| {
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    if valid_component(owner) && valid_component(name) && components.next().is_none() {
        Ok(())
    } else {
        Err(format!(
            "prebuilt provenance repository {repository:?} is not canonical"
        ))
    }
}

fn validate_target_crt(target: &str, crt: &str) -> Result<(), String> {
    let valid = match target {
        "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" | "aarch64-apple-darwin" => {
            crt == "none"
        }
        "x86_64-pc-windows-msvc" => matches!(crt, "md" | "mt"),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "unsupported prebuilt provenance target/CRT coordinate {target:?}/{crt:?}"
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
            "prebuilt provenance field `{label}` must be a positive canonical decimal"
        ))
    }
}

fn validate_git_sha(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 40 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "prebuilt provenance field `{label}` is not a lowercase Git SHA"
        ))
    }
}

fn validate_sha256(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "prebuilt provenance field `{label}` is not a SHA-256 digest"
        ))
    }
}

fn validate_blake3(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!(
            "prebuilt provenance field `{label}` is not a BLAKE3 digest"
        ))
    }
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.contains('\\')
        || path.contains("//")
        || path.starts_with("./")
        || path.ends_with('/')
        || Path::new(path).is_absolute()
        || !Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        Err(format!(
            "prebuilt provenance member path {path:?} is not a normalized relative path"
        ))
    } else {
        Ok(())
    }
}

fn expected_parent_directories(members: &[MemberDigest]) -> BTreeSet<&str> {
    let mut directories = BTreeSet::new();
    for member in members {
        for (index, byte) in member.path.bytes().enumerate() {
            if byte == b'/' {
                directories.insert(&member.path[..index]);
            }
        }
    }
    directories
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_manifest::{
        self, ADAPTER_ABI_VERSION, RECORDING_CONTRACT_BLAKE3, SCHEMA_NAME as MANIFEST_SCHEMA_NAME,
        SCHEMA_VERSION as MANIFEST_SCHEMA_VERSION,
    };

    fn fixture() -> (PrebuiltProvenanceStatement, BTreeMap<String, Vec<u8>>) {
        let archive = b"archive".to_vec();
        let header = b"header".to_vec();
        let bindings = b"bindings".to_vec();
        let source_commit = "a".repeat(40);
        let upstream_sha = "b".repeat(40);
        let effective_source_sha256 = "c".repeat(64);
        let adapter_source_sha256 = "d".repeat(64);
        let private_abi_hash = "e".repeat(64);
        let manifest = ArtifactManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            schema: MANIFEST_SCHEMA_NAME.to_owned(),
            provider: "prebuilt".to_owned(),
            crate_version: "0.6.0".to_owned(),
            source_commit: Some(source_commit.clone()),
            release_tag: Some("v0.6.0".to_owned()),
            upstream_sha: upstream_sha.clone(),
            effective_source_sha256: effective_source_sha256.clone(),
            precision: "single".to_owned(),
            target: "x86_64-unknown-linux-gnu".to_owned(),
            link: "static".to_owned(),
            crt: "none".to_owned(),
            simd: "default".to_owned(),
            validate: false,
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: adapter_source_sha256.clone(),
            private_abi_hash: private_abi_hash.clone(),
            snapshot_layout_hash: 42,
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            required_adapter_symbols_sha256: provider_manifest::required_adapter_symbols_sha256(),
            required_adapter_symbols: provider_manifest::REQUIRED_ADAPTER_SYMBOLS
                .iter()
                .map(|symbol| (*symbol).to_owned())
                .collect(),
            archive: "lib/libbox2d.a".to_owned(),
            archive_sha256: sha256_bytes(&archive),
            header: "include/box2d/box2d.h".to_owned(),
            header_sha256: sha256_bytes(&header),
            bindings: "bindings/bindings_pregenerated.rs".to_owned(),
            bindings_sha256: sha256_bytes(&bindings),
        };
        let manifest = manifest.render();
        let mut files = BTreeMap::from([
            ("bindings/bindings_pregenerated.rs".to_owned(), bindings),
            ("include/box2d/box2d.h".to_owned(), header),
            ("lib/libbox2d.a".to_owned(), archive),
            ("manifest.toml".to_owned(), manifest.clone()),
        ]);
        let mut checksums = String::new();
        for (path, bytes) in &files {
            checksums.push_str(&format!("{}  {path}\n", sha256_bytes(bytes)));
        }
        files.insert("checksums.sha256".to_owned(), checksums.into_bytes());
        let members = members_from_files(&files).unwrap();
        let package = b"canonical-package";
        let statement = PrebuiltProvenanceStatement {
            schema_version: SCHEMA_VERSION,
            schema: SCHEMA_NAME.to_owned(),
            repository: "Latias94/boxdd".to_owned(),
            workflow: ".github/workflows/prebuilt-binaries.yml".to_owned(),
            workflow_ref: "Latias94/boxdd/.github/workflows/prebuilt-binaries.yml@refs/tags/v0.6.0"
                .to_owned(),
            source_commit,
            release_tag: "v0.6.0".to_owned(),
            run_id: "123456".to_owned(),
            run_attempt: "1".to_owned(),
            crate_version: "0.6.0".to_owned(),
            package_name: "boxdd-prebuilt-0.6.0-x86_64-unknown-linux-gnu-single-static.tar.gz"
                .to_owned(),
            package_size: package.len() as u64,
            package_sha256: sha256_bytes(package),
            provider_manifest_sha256: sha256_bytes(&manifest),
            inner_checksums_sha256: sha256_bytes(&files["checksums.sha256"]),
            provider: "prebuilt".to_owned(),
            target: "x86_64-unknown-linux-gnu".to_owned(),
            precision: "single".to_owned(),
            link: "static".to_owned(),
            crt: "none".to_owned(),
            upstream_sha,
            effective_source_sha256,
            simd: "default".to_owned(),
            validate: false,
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256,
            private_abi_hash,
            snapshot_layout_hash: 42,
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            member_count: members.len() as u64,
            members,
        };
        (statement, files)
    }

    #[test]
    fn canonical_statement_round_trips_and_binds_manifest() {
        let (statement, files) = fixture();
        let bytes = statement.canonical_bytes().unwrap();
        assert_eq!(
            PrebuiltProvenanceStatement::parse_canonical(&bytes).unwrap(),
            statement
        );
        statement
            .validate_provider_manifest(&files["manifest.toml"])
            .unwrap();
        statement
            .validate_publisher("Latias94/boxdd", ".github/workflows/prebuilt-binaries.yml")
            .unwrap();
    }

    #[test]
    fn package_and_inventory_tampering_fail_closed() {
        let (statement, _) = fixture();
        assert!(statement.verify_package_bytes(b"canonical-package").is_ok());
        assert!(statement.verify_package_bytes(b"changed-package").is_err());

        let mut unsorted = statement.clone();
        unsorted.members.swap(0, 1);
        assert!(unsorted.validate_intrinsic().is_err());

        let mut wrong_run = statement.clone();
        wrong_run.run_attempt = "0".to_owned();
        assert!(wrong_run.validate_intrinsic().is_err());
    }

    #[test]
    fn outer_package_snapshot_is_bounded_by_the_signed_exact_size() {
        let (statement, _) = fixture();
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join(&statement.package_name);
        fs::write(&package, b"canonical-package").unwrap();
        let verified = statement.verify_outer_package(&package).unwrap();
        assert_eq!(verified.as_slice(), b"canonical-package");

        fs::write(&package, b"canonical-package with trailing bytes").unwrap();
        let error = statement
            .verify_outer_package(&package)
            .expect_err("bytes beyond the signed size must fail before acceptance");
        assert!(error.contains("byte limit"), "{error}");
    }

    #[test]
    fn coordinate_and_manifest_tampering_fail_closed() {
        let (statement, files) = fixture();

        let mut target = statement.clone();
        target.target = "aarch64-apple-darwin".to_owned();
        target.package_name =
            "boxdd-prebuilt-0.6.0-aarch64-apple-darwin-single-static.tar.gz".to_owned();
        target.validate_intrinsic().unwrap();
        assert!(
            target
                .validate_provider_manifest(&files["manifest.toml"])
                .is_err()
        );

        let mut precision = statement.clone();
        precision.precision = "double".to_owned();
        precision.package_name =
            "boxdd-prebuilt-0.6.0-x86_64-unknown-linux-gnu-double-static.tar.gz".to_owned();
        precision.validate_intrinsic().unwrap();
        assert!(
            precision
                .validate_provider_manifest(&files["manifest.toml"])
                .is_err()
        );

        let mut crt = statement.clone();
        crt.target = "x86_64-pc-windows-msvc".to_owned();
        crt.crt = "md".to_owned();
        crt.package_name =
            "boxdd-prebuilt-0.6.0-x86_64-pc-windows-msvc-single-static-md.tar.gz".to_owned();
        crt.validate_intrinsic().unwrap();
        assert!(
            crt.validate_provider_manifest(&files["manifest.toml"])
                .is_err()
        );

        let mut simd = statement.clone();
        simd.simd = "disabled".to_owned();
        assert!(simd.validate_intrinsic().is_err());
        let mut validation = statement.clone();
        validation.validate = true;
        assert!(validation.validate_intrinsic().is_err());

        let mut manifest = ArtifactManifest::parse(&files["manifest.toml"]).unwrap();
        manifest.snapshot_layout_hash += 1;
        assert!(
            statement
                .validate_provider_manifest(&manifest.render())
                .is_err()
        );
    }

    #[test]
    fn canonical_parser_rejects_unknown_or_reformatted_input() {
        let (statement, _) = fixture();
        let bytes = statement.render();
        let mut unknown = bytes.clone();
        unknown.extend_from_slice(b"unknown = true\n");
        assert!(PrebuiltProvenanceStatement::parse(&unknown).is_err());

        let mut reformatted = b"\n".to_vec();
        reformatted.extend_from_slice(&bytes);
        assert!(PrebuiltProvenanceStatement::parse_canonical(&reformatted).is_err());
    }

    #[test]
    fn extracted_root_requires_the_exact_signed_file_set() {
        let (statement, files) = fixture();
        let directory = tempfile::tempdir().unwrap();
        for (relative, bytes) in &files {
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        statement.verify_extracted_root(directory.path()).unwrap();

        fs::write(directory.path().join("unexpected"), b"extra").unwrap();
        assert!(statement.verify_extracted_root(directory.path()).is_err());
        fs::remove_file(directory.path().join("unexpected")).unwrap();

        fs::write(directory.path().join("manifest.toml"), b"changed").unwrap();
        assert!(statement.verify_extracted_root(directory.path()).is_err());
    }
}
