use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(test)]
use std::ffi::OsStr;

use flate2::bufread::GzDecoder;
#[cfg(test)]
use tempfile::TempDir;

use crate::build_support::{VerifiedFileSnapshot, snapshot_file_create_new};
#[cfg(test)]
use crate::isolated_git::is_git_environment_key;
use crate::isolated_git::isolated_git_command;
use crate::prebuilt_provenance::{self, PrebuiltProvenanceStatement};
use crate::provenance_policy::{
    self, COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW, SIGSTORE_TRUSTED_ROOT_SHA256,
};
use crate::provider_archive::{ArchiveExpectation, verify_provider_archive};
use crate::provider_manifest::{
    self, ArtifactExpectation, ArtifactIdentityExpectation, ArtifactManifest,
};
use crate::source_overlay::adapter_source_sha256;
use crate::subprocess_policy::{run_output, run_status};
use crate::wasm_release_provenance::WasmReleaseProvenanceStatement;
use crate::{Error, Result};

use super::{
    effective_source_snapshot::EffectiveHeaderSnapshot,
    support::{BoundedReader, cosign_command, normalize_crlf},
    wasm_release::{self, UnsignedReleaseContext},
};

const CHECKSUMS_FILE: &str = "SHA256SUMS";
const MAX_ARCHIVE_ENTRY_BYTES: u64 = prebuilt_provenance::MAX_MEMBER_BYTES;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = prebuilt_provenance::MAX_TOTAL_MEMBER_BYTES;
const MAX_ARCHIVE_ENTRIES: usize = prebuilt_provenance::MAX_MEMBERS;
const TAR_BLOCK_BYTES: u64 = 512;
const MAX_ARCHIVE_STREAM_BYTES: u64 = MAX_ARCHIVE_TOTAL_BYTES
    + (MAX_ARCHIVE_ENTRIES as u64 * TAR_BLOCK_BYTES * 2)
    + (TAR_BLOCK_BYTES * 2);
const MAX_PROVENANCE_STATEMENT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SIGSTORE_BUNDLE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TRUSTED_ROOT_BYTES: u64 = 4 * 1024 * 1024;
const DEFAULT_TRUSTED_ROOT_RELATIVE_PATH: &str = "boxdd-sys/security/sigstore/trusted_root.json";
const RELEASE_PRECISIONS: &[&str] = &["single", "double"];

const PLATFORMS: &[Platform] = &[
    Platform {
        target: "x86_64-unknown-linux-gnu",
        crt: "none",
    },
    Platform {
        target: "x86_64-apple-darwin",
        crt: "none",
    },
    Platform {
        target: "aarch64-apple-darwin",
        crt: "none",
    },
    Platform {
        target: "x86_64-pc-windows-msvc",
        crt: "md",
    },
    Platform {
        target: "x86_64-pc-windows-msvc",
        crt: "mt",
    },
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Platform {
    target: &'static str,
    crt: &'static str,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ArtifactSpec {
    target: &'static str,
    precision: &'static str,
    crt: &'static str,
    archive: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Check,
    CheckContent,
    PrintPackageVersion,
    WriteChecksums,
}

#[derive(Debug)]
struct Options {
    mode: Mode,
    tag: Option<String>,
    commit: Option<String>,
    artifacts: Option<PathBuf>,
    run_id: Option<String>,
    run_attempt: Option<String>,
    repository: Option<String>,
    workflow_ref: Option<String>,
    trusted_root: Option<PathBuf>,
    payloads: Option<PathBuf>,
    cosign: PathBuf,
}

#[derive(Debug)]
struct ReleaseIdentity {
    version: String,
    tag: String,
    commit: String,
    upstream_sha: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValidatedReleaseContext {
    run_id: String,
    run_attempt: String,
    repository: String,
    workflow_ref: String,
}

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    if options.mode == Mode::PrintPackageVersion {
        println!("{}", workspace_release_version(root)?);
        return Ok(());
    }
    let identity = validate_repository_identity(root, &options)?;
    let artifacts = options
        .artifacts
        .as_deref()
        .expect("all validation modes require --artifacts");

    match options.mode {
        Mode::WriteChecksums => write_aggregate_checksums(artifacts, &identity.version),
        Mode::CheckContent => {
            let context = validate_release_context(&options, &identity)?;
            validate_artifacts(root, artifacts, &options, &identity, &context, false)
        }
        Mode::Check => {
            let context = validate_release_context(&options, &identity)?;
            validate_artifacts(root, artifacts, &options, &identity, &context, true)
        }
        Mode::PrintPackageVersion => unreachable!("version mode returns before validation"),
    }
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self {
            mode: Mode::Check,
            tag: None,
            commit: None,
            artifacts: None,
            run_id: None,
            run_attempt: None,
            repository: None,
            workflow_ref: None,
            trusted_root: None,
            payloads: None,
            cosign: PathBuf::from("cosign"),
        };
        let mut cursor = 0;
        let mut mode_seen = false;
        while cursor < args.len() {
            match args[cursor].as_str() {
                "--check" if !mode_seen => {
                    options.mode = Mode::Check;
                    mode_seen = true;
                    cursor += 1;
                }
                "--check-content" if !mode_seen => {
                    options.mode = Mode::CheckContent;
                    mode_seen = true;
                    cursor += 1;
                }
                "--print-package-version" if !mode_seen => {
                    options.mode = Mode::PrintPackageVersion;
                    mode_seen = true;
                    cursor += 1;
                }
                "--write-checksums" if !mode_seen => {
                    options.mode = Mode::WriteChecksums;
                    mode_seen = true;
                    cursor += 1;
                }
                "--tag" => parse_value(args, &mut cursor, &mut options.tag, "--tag")?,
                "--commit" => parse_value(args, &mut cursor, &mut options.commit, "--commit")?,
                "--artifacts" => {
                    let mut value = None;
                    parse_value(args, &mut cursor, &mut value, "--artifacts")?;
                    options.artifacts = value.map(PathBuf::from);
                }
                "--run-id" => parse_value(args, &mut cursor, &mut options.run_id, "--run-id")?,
                "--run-attempt" => {
                    parse_value(args, &mut cursor, &mut options.run_attempt, "--run-attempt")?
                }
                "--repository" => {
                    parse_value(args, &mut cursor, &mut options.repository, "--repository")?
                }
                "--workflow-ref" => parse_value(
                    args,
                    &mut cursor,
                    &mut options.workflow_ref,
                    "--workflow-ref",
                )?,
                "--trusted-root" => {
                    let mut value = None;
                    parse_value(args, &mut cursor, &mut value, "--trusted-root")?;
                    options.trusted_root = value.map(PathBuf::from);
                }
                "--payloads" => {
                    let mut value = None;
                    parse_value(args, &mut cursor, &mut value, "--payloads")?;
                    options.payloads = value.map(PathBuf::from);
                }
                "--cosign" => {
                    let mut value = None;
                    parse_value(args, &mut cursor, &mut value, "--cosign")?;
                    options.cosign = PathBuf::from(value.expect("parsed option value"));
                }
                value => {
                    return Err(Error::message(format!(
                        "unsupported release-contract argument {value:?}"
                    )));
                }
            }
        }
        if !mode_seen {
            return Err(Error::message(
                "release-contract requires --check, --check-content, --print-package-version, or --write-checksums",
            ));
        }
        if options.payloads.is_some() && options.mode != Mode::CheckContent {
            return Err(Error::message(
                "release-contract --payloads is valid only with --check-content",
            ));
        }
        if options.mode == Mode::PrintPackageVersion
            && (options.tag.is_some()
                || options.commit.is_some()
                || options.artifacts.is_some()
                || options.run_id.is_some()
                || options.run_attempt.is_some()
                || options.repository.is_some()
                || options.workflow_ref.is_some()
                || options.trusted_root.is_some()
                || options.payloads.is_some()
                || options.cosign != Path::new("cosign"))
        {
            return Err(Error::message(
                "release-contract --print-package-version does not accept release context",
            ));
        }
        if options.mode != Mode::PrintPackageVersion && options.artifacts.is_none() {
            return Err(Error::message(format!(
                "release-contract {} requires --artifacts",
                match options.mode {
                    Mode::Check => "--check",
                    Mode::CheckContent => "--check-content",
                    Mode::WriteChecksums => "--write-checksums",
                    Mode::PrintPackageVersion => unreachable!(),
                }
            )));
        }
        Ok(options)
    }
}

fn parse_value(
    args: &[String],
    cursor: &mut usize,
    destination: &mut Option<String>,
    option: &str,
) -> Result<()> {
    if destination.is_some() {
        return Err(Error::message(format!(
            "{option} was provided more than once"
        )));
    }
    let value = args
        .get(*cursor + 1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| Error::message(format!("{option} requires a value")))?;
    *destination = Some(value.clone());
    *cursor += 2;
    Ok(())
}

fn reconcile_context(
    explicit: Option<String>,
    environment: Option<String>,
    key: &str,
) -> Result<Option<String>> {
    match (explicit, environment) {
        (Some(explicit), Some(environment)) if explicit != environment => Err(Error::message(
            format!("explicit release context for {key} does not match immutable GitHub context"),
        )),
        (Some(explicit), _) => Ok(Some(explicit)),
        (_, Some(environment)) => Ok(Some(environment)),
        (None, None) => Ok(None),
    }
}

fn option_or_env(value: &Option<String>, key: &str) -> Result<Option<String>> {
    reconcile_context(value.clone(), env::var(key).ok(), key)
}

fn workspace_release_version(root: &Path) -> Result<String> {
    let root_manifest = read_toml(&root.join("Cargo.toml"))?;
    let version = root_manifest
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::message("workspace.package.version is missing"))?
        .to_owned();
    for manifest in [
        "boxdd-sys/Cargo.toml",
        "boxdd/Cargo.toml",
        "bevy_boxdd/Cargo.toml",
    ] {
        let manifest_value = read_toml(&root.join(manifest))?;
        let inherits_workspace_version = manifest_value
            .get("package")
            .and_then(|package| package.get("version"))
            .and_then(|version| version.get("workspace"))
            .and_then(toml::Value::as_bool)
            == Some(true);
        if !inherits_workspace_version {
            return Err(Error::message(format!(
                "{manifest} must inherit the release version from the workspace"
            )));
        }
    }
    semver::Version::parse(&version)
        .map_err(|error| Error::message(format!("invalid release version {version:?}: {error}")))?;
    Ok(version)
}

fn validate_repository_identity(root: &Path, options: &Options) -> Result<ReleaseIdentity> {
    let version = workspace_release_version(root)?;
    let tag = options
        .tag
        .clone()
        .or_else(|| {
            options
                .artifacts
                .is_some()
                .then(|| env::var("GITHUB_REF_NAME").ok())
                .flatten()
        })
        .unwrap_or_else(|| provenance_policy::workspace_release_tag(&version));
    validate_tag(&tag, &version)?;
    let commit = options
        .commit
        .clone()
        .or_else(|| env::var("GITHUB_SHA").ok())
        .unwrap_or(git_output(
            root,
            &["rev-parse", "HEAD"],
            "read release commit",
        )?);
    validate_git_sha("release commit", &commit)?;
    let checkout_commit = git_output(root, &["rev-parse", "HEAD"], "read checkout commit")?;
    require_matching_identity("checkout HEAD", &checkout_commit, &commit)?;
    if let Ok(github_sha) = env::var("GITHUB_SHA")
        && github_sha != commit
    {
        return Err(Error::message(format!(
            "release commit {commit} does not match immutable GITHUB_SHA {github_sha}"
        )));
    }
    if options.artifacts.is_some() {
        if let Ok(ref_type) = env::var("GITHUB_REF_TYPE")
            && ref_type != "tag"
        {
            return Err(Error::message(format!(
                "release validation only accepts tag events; GITHUB_REF_TYPE={ref_type:?}"
            )));
        }
        if let Ok(github_ref) = env::var("GITHUB_REF") {
            let expected = format!("refs/tags/{tag}");
            if github_ref != expected {
                return Err(Error::message(format!(
                    "release ref {github_ref:?} does not match protected tag {expected:?}"
                )));
            }
        }
        let status = git_output(
            root,
            &["status", "--porcelain=v1", "--untracked-files=all"],
            "inspect release checkout state",
        )?;
        require_clean_status("release checkout", &status)?;
        let tag_revision = format!("refs/tags/{tag}^{{commit}}");
        let tag_commit = git_output(
            root,
            &["rev-parse", tag_revision.as_str()],
            "resolve protected release tag",
        )?;
        require_matching_identity("protected tag commit", &tag_commit, &commit)?;
    }

    let upstream = read_toml(&root.join("boxdd-sys/upstream.toml"))?;
    let upstream_sha = upstream
        .get("active_revision")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::message("upstream.toml active_revision is missing"))?
        .to_owned();
    validate_git_sha("upstream.toml active_revision", &upstream_sha)?;
    let gitlink = git_output(
        root,
        &["ls-files", "-s", "boxdd-sys/third-party/box2d"],
        "read Box2D gitlink",
    )?;
    let fields = gitlink.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 3 || fields[0] != "160000" || fields[1] != upstream_sha {
        return Err(Error::message(format!(
            "Box2D gitlink does not match upstream.toml: {gitlink:?}"
        )));
    }
    let submodule = "boxdd-sys/third-party/box2d";
    let checkout_upstream = git_output(
        root,
        &["-C", submodule, "rev-parse", "HEAD"],
        "read Box2D checkout commit",
    )?;
    require_matching_identity("Box2D checkout HEAD", &checkout_upstream, &upstream_sha)?;
    if options.artifacts.is_some() {
        let status = git_output(
            root,
            &[
                "-C",
                submodule,
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
                "--ignored=matching",
            ],
            "inspect Box2D checkout state",
        )?;
        require_clean_status("Box2D release checkout", &status)?;
    }

    validate_changelog(root, &version)?;
    Ok(ReleaseIdentity {
        version,
        tag,
        commit,
        upstream_sha,
    })
}

fn validate_tag(tag: &str, version: &str) -> Result<()> {
    if provenance_policy::release_tag_matches_version(version, tag) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "release tag {tag:?} does not match workspace version {version}"
        )))
    }
}

fn require_matching_identity(label: &str, actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} {actual:?} does not match release identity {expected:?}"
        )))
    }
}

fn require_clean_status(label: &str, status: &str) -> Result<()> {
    if status.is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} contains disallowed working tree changes: {status:?}"
        )))
    }
}

fn validate_changelog(root: &Path, version: &str) -> Result<()> {
    let path = root.join("CHANGELOG.md");
    let changelog = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let release_heading = format!("## [{version}]");
    if !changelog.contains(&release_heading) {
        return Err(Error::message(format!(
            "CHANGELOG.md is stale: protected release requires heading {release_heading}"
        )));
    }
    let start = changelog
        .find(&release_heading)
        .expect("selected changelog heading must exist");
    let section = &changelog[start..];
    let end = section[release_heading.len()..]
        .find("\n## [")
        .map(|offset| release_heading.len() + offset)
        .unwrap_or(section.len());
    if section[release_heading.len()..end].trim().is_empty() {
        return Err(Error::message(format!(
            "CHANGELOG.md release section {release_heading} is empty"
        )));
    }
    Ok(())
}

fn validate_release_context(
    options: &Options,
    identity: &ReleaseIdentity,
) -> Result<ValidatedReleaseContext> {
    let protected = env::var("GITHUB_REF_PROTECTED")
        .map_err(|_| Error::message("artifact validation requires GITHUB_REF_PROTECTED=true"))?;
    if protected != "true" {
        return Err(Error::message(format!(
            "release provenance requires a protected tag; GITHUB_REF_PROTECTED={protected:?}"
        )));
    }
    let run_id = option_or_env(&options.run_id, "GITHUB_RUN_ID")?
        .ok_or_else(|| Error::message("artifact validation requires --run-id or GITHUB_RUN_ID"))?;
    if run_id.is_empty() || !run_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::message(
            "release run ID must contain only decimal digits",
        ));
    }
    let run_attempt =
        option_or_env(&options.run_attempt, "GITHUB_RUN_ATTEMPT")?.ok_or_else(|| {
            Error::message("artifact validation requires --run-attempt or GITHUB_RUN_ATTEMPT")
        })?;
    if run_attempt.is_empty() || !run_attempt.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::message(
            "release run attempt must contain only decimal digits",
        ));
    }
    let repository = option_or_env(&options.repository, "GITHUB_REPOSITORY")?
        .ok_or_else(|| Error::message("artifact validation requires --repository"))?;
    if repository != PUBLISHER_REPOSITORY {
        return Err(Error::message(format!(
            "untrusted release repository {repository:?}; expected {PUBLISHER_REPOSITORY}"
        )));
    }
    let workflow_ref = option_or_env(&options.workflow_ref, "GITHUB_WORKFLOW_REF")?
        .ok_or_else(|| Error::message("artifact validation requires --workflow-ref"))?;
    let expected = format!(
        "{PUBLISHER_REPOSITORY}/{PUBLISHER_WORKFLOW}@refs/tags/{}",
        identity.tag
    );
    if workflow_ref != expected {
        return Err(Error::message(format!(
            "release workflow ref {workflow_ref:?} is not the protected tag workflow {expected:?}"
        )));
    }
    Ok(ValidatedReleaseContext {
        run_id,
        run_attempt,
        repository,
        workflow_ref,
    })
}

fn expected_artifacts(version: &str) -> Vec<ArtifactSpec> {
    let mut artifacts = Vec::new();
    for platform in PLATFORMS {
        for precision in RELEASE_PRECISIONS.iter().copied() {
            let suffix = if platform.crt == "none" {
                String::new()
            } else {
                format!("-{}", platform.crt)
            };
            artifacts.push(ArtifactSpec {
                target: platform.target,
                precision,
                crt: platform.crt,
                archive: format!(
                    "boxdd-prebuilt-{version}-{}-{precision}-static{suffix}.tar.gz",
                    platform.target
                ),
            });
        }
    }
    artifacts.sort();
    artifacts
}

fn expected_wasm_artifacts(version: &str) -> Result<Vec<(&'static str, String)>> {
    RELEASE_PRECISIONS
        .iter()
        .copied()
        .map(|precision| {
            wasm_release::archive_name(version, precision).map(|archive| (precision, archive))
        })
        .collect()
}

fn expected_release_archive_names(version: &str) -> Result<Vec<String>> {
    let mut names = expected_artifacts(version)
        .into_iter()
        .map(|spec| spec.archive)
        .collect::<Vec<_>>();
    names.extend(
        expected_wasm_artifacts(version)?
            .into_iter()
            .map(|(_, archive)| archive),
    );
    names.sort();
    if names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Error::message(
            "release archive names must be globally unique",
        ));
    }
    Ok(names)
}

fn write_aggregate_checksums(root: &Path, version: &str) -> Result<()> {
    let files = collect_files(root)?;
    let expected = expected_release_archive_names(version)?;
    let archives = map_expected_archive_names(&files, &expected)?;
    let mut rendered = String::new();
    for archive in expected {
        let path = &archives[&archive];
        let digest = VerifiedFileSnapshot::read(
            path,
            prebuilt_provenance::MAX_PACKAGE_BYTES,
            "release archive",
        )
        .map_err(Error::message)?
        .sha256()
        .to_owned();
        let sidecar = path.with_file_name(format!("{archive}.sha256"));
        let sidecar_source = format!("{digest}  {archive}\n");
        fs::write(&sidecar, &sidecar_source).map_err(|error| Error::io(&sidecar, error))?;
        rendered.push_str(&sidecar_source);
    }
    let destination = root.join(CHECKSUMS_FILE);
    fs::write(&destination, rendered).map_err(|error| Error::io(destination, error))
}

struct ValidatedArchive {
    statement: PrebuiltProvenanceStatement,
}

fn validate_artifacts(
    repository_root: &Path,
    artifact_root: &Path,
    options: &Options,
    identity: &ReleaseIdentity,
    context: &ValidatedReleaseContext,
    require_signatures: bool,
) -> Result<()> {
    let canonical_root =
        fs::canonicalize(artifact_root).map_err(|error| Error::io(artifact_root, error))?;
    if !canonical_root.is_dir() {
        return Err(Error::message(format!(
            "artifact root is not a directory: {}",
            canonical_root.display()
        )));
    }
    let trusted_root = resolve_trusted_root(repository_root, options.trusted_root.as_deref());
    let verification_inputs = tempfile::Builder::new()
        .prefix("boxdd-release-verification-")
        .tempdir()
        .map_err(|error| Error::io("create private release verification directory", error))?;
    let trusted_root = snapshot_verification_input(
        &trusted_root,
        verification_inputs.path(),
        "trusted-root.json",
        MAX_TRUSTED_ROOT_BYTES,
        "Sigstore trusted root",
    )?;
    require_trusted_root(trusted_root.path(), trusted_root.bytes())?;
    if require_signatures {
        verify_cosign_version(&options.cosign)?;
    }
    let payload_root = options
        .payloads
        .as_deref()
        .map(prepare_empty_payload_directory)
        .transpose()?;
    if payload_root
        .as_ref()
        .is_some_and(|payload_root| payload_root.starts_with(&canonical_root))
    {
        return Err(Error::message(
            "signing payload output must remain outside the release input tree",
        ));
    }

    let files = collect_files(&canonical_root)?;
    let expected = expected_artifacts(&identity.version);
    let expected_wasm = expected_wasm_artifacts(&identity.version)?;
    let expected_names = expected_release_archive_names(&identity.version)?;
    let archives = map_expected_archive_names(&files, &expected_names)?;
    require_exact_release_file_set(&files, &archives, &canonical_root, require_signatures)?;
    let mut allowed = BTreeSet::new();
    let mut aggregate_entries = BTreeMap::new();

    for spec in &expected {
        let archive = &archives[&spec.archive];
        let expected_parent = format!(
            "prebuilt-input-{}-{}-{}-{run_id}-{run_attempt}-{}",
            spec.target,
            spec.precision,
            spec.crt,
            identity.commit,
            run_id = context.run_id,
            run_attempt = context.run_attempt,
        );
        let parent = archive
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if parent != expected_parent {
            return Err(Error::message(format!(
                "artifact {} came from mutable or mismatched workflow artifact {parent:?}; expected {expected_parent:?}",
                spec.archive
            )));
        }
        let validated =
            validate_archive_manifest(repository_root, archive, spec, identity, context)?;
        let digest = &validated.statement.package_sha256;
        if aggregate_entries
            .insert(spec.archive.clone(), digest.clone())
            .is_some()
        {
            return Err(Error::message(format!(
                "release aggregate contains duplicate archive {}",
                spec.archive
            )));
        }
        let checksum = archive.with_file_name(format!("{}.sha256", spec.archive));
        let statement = archive.with_file_name(format!("{}.provenance.toml", spec.archive));
        let bundle = archive.with_file_name(format!("{}.provenance.sigstore.json", spec.archive));
        let expected_checksum = format!("{digest}  {}\n", spec.archive);
        let actual_checksum =
            fs::read_to_string(&checksum).map_err(|error| Error::io(&checksum, error))?;
        if actual_checksum != expected_checksum {
            return Err(Error::message(format!(
                "non-canonical or incorrect checksum sidecar {}",
                checksum.display()
            )));
        }
        if let Some(payload_root) = &payload_root {
            let destination = payload_root.join(format!("{}.provenance.toml", spec.archive));
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .map_err(|error| Error::io(&destination, error))?;
            output
                .write_all(
                    &validated
                        .statement
                        .canonical_bytes()
                        .map_err(Error::message)?,
                )
                .map_err(|error| Error::io(&destination, error))?;
        }
        allowed.extend([archive.clone(), checksum]);
        if require_signatures {
            let statement_snapshot = snapshot_verification_input(
                &statement,
                verification_inputs.path(),
                &format!("{}.provenance.toml", spec.archive),
                MAX_PROVENANCE_STATEMENT_BYTES,
                "prebuilt provenance statement",
            )?;
            let bundle_snapshot = snapshot_verification_input(
                &bundle,
                verification_inputs.path(),
                &format!("{}.provenance.sigstore.json", spec.archive),
                MAX_SIGSTORE_BUNDLE_BYTES,
                "prebuilt Sigstore bundle",
            )?;
            let supplied = PrebuiltProvenanceStatement::parse_canonical(statement_snapshot.bytes())
                .map_err(Error::message)?;
            if supplied != validated.statement {
                return Err(Error::message(format!(
                    "artifact {} provenance statement does not match its exact package and release context",
                    spec.archive
                )));
            }
            verify_sigstore(
                &options.cosign,
                statement_snapshot.path(),
                bundle_snapshot.path(),
                trusted_root.path(),
                identity,
            )?;
            revalidate_sigstore_inputs(
                &statement_snapshot,
                &bundle_snapshot,
                &trusted_root,
                "prebuilt",
            )?;
            allowed.extend([statement, bundle]);
        }
    }

    for (precision, archive_name) in &expected_wasm {
        let archive = &archives[archive_name];
        let expected_parent = format!(
            "wasm-input-{precision}-{run_id}-{run_attempt}-{}",
            identity.commit,
            run_id = context.run_id,
            run_attempt = context.run_attempt,
        );
        let parent = archive
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if parent != expected_parent {
            return Err(Error::message(format!(
                "artifact {archive_name} came from mutable or mismatched workflow artifact {parent:?}; expected {expected_parent:?}"
            )));
        }
        let validated = wasm_release::validate_unsigned_package(
            repository_root,
            archive,
            precision,
            UnsignedReleaseContext {
                repository: &context.repository,
                workflow_ref: &context.workflow_ref,
                source_commit: &identity.commit,
                release_tag: &identity.tag,
                run_id: &context.run_id,
                run_attempt: &context.run_attempt,
                crate_version: &identity.version,
            },
        )?;
        let digest = &validated.package_sha256;
        if aggregate_entries
            .insert(archive_name.clone(), digest.clone())
            .is_some()
        {
            return Err(Error::message(format!(
                "release aggregate contains duplicate archive {archive_name}"
            )));
        }
        let checksum = archive.with_file_name(format!("{archive_name}.sha256"));
        let statement = archive.with_file_name(format!("{archive_name}.provenance.toml"));
        let bundle = archive.with_file_name(format!("{archive_name}.provenance.sigstore.json"));
        let expected_checksum = format!("{digest}  {archive_name}\n");
        let actual_checksum =
            fs::read_to_string(&checksum).map_err(|error| Error::io(&checksum, error))?;
        if actual_checksum != expected_checksum {
            return Err(Error::message(format!(
                "non-canonical or incorrect checksum sidecar {}",
                checksum.display()
            )));
        }
        if let Some(payload_root) = &payload_root {
            let destination = payload_root.join(format!("{archive_name}.provenance.toml"));
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .map_err(|error| Error::io(&destination, error))?;
            output
                .write_all(&validated.canonical_bytes().map_err(Error::message)?)
                .map_err(|error| Error::io(&destination, error))?;
        }
        allowed.extend([archive.clone(), checksum]);
        if require_signatures {
            let statement_snapshot = snapshot_verification_input(
                &statement,
                verification_inputs.path(),
                &format!("{archive_name}.provenance.toml"),
                MAX_PROVENANCE_STATEMENT_BYTES,
                "WASM provider provenance statement",
            )?;
            let bundle_snapshot = snapshot_verification_input(
                &bundle,
                verification_inputs.path(),
                &format!("{archive_name}.provenance.sigstore.json"),
                MAX_SIGSTORE_BUNDLE_BYTES,
                "WASM provider Sigstore bundle",
            )?;
            let supplied =
                WasmReleaseProvenanceStatement::parse_canonical(statement_snapshot.bytes())
                    .map_err(Error::message)?;
            if supplied != validated {
                return Err(Error::message(format!(
                    "artifact {archive_name} provenance statement does not match its exact package and release context"
                )));
            }
            verify_sigstore(
                &options.cosign,
                statement_snapshot.path(),
                bundle_snapshot.path(),
                trusted_root.path(),
                identity,
            )?;
            revalidate_sigstore_inputs(
                &statement_snapshot,
                &bundle_snapshot,
                &trusted_root,
                "WASM provider",
            )?;
            allowed.extend([statement, bundle]);
        }
    }

    if aggregate_entries.len() != expected_names.len() {
        return Err(Error::message(
            "release aggregate did not validate every expected archive exactly once",
        ));
    }
    let aggregate = aggregate_entries
        .iter()
        .map(|(archive, digest)| format!("{digest}  {archive}\n"))
        .collect::<String>();

    let aggregate_path = canonical_root.join(CHECKSUMS_FILE);
    let aggregate_source =
        fs::read_to_string(&aggregate_path).map_err(|error| Error::io(&aggregate_path, error))?;
    if aggregate_source != aggregate {
        return Err(Error::message(
            "SHA256SUMS is missing, unsorted, non-canonical, or inconsistent with release archives",
        ));
    }
    allowed.insert(aggregate_path);
    let unexpected = files
        .into_iter()
        .filter(|path| !allowed.contains(path))
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(Error::message(format!(
            "release input contains unexpected files: {}",
            unexpected
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(())
}

fn resolve_trusted_root(repository_root: &Path, configured: Option<&Path>) -> PathBuf {
    let trusted_root = configured.unwrap_or_else(|| Path::new(DEFAULT_TRUSTED_ROOT_RELATIVE_PATH));
    if trusted_root.is_absolute() {
        trusted_root.to_path_buf()
    } else {
        repository_root.join(trusted_root)
    }
}

fn prepare_empty_payload_directory(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::message(format!(
                "signing payload output must be a real directory: {}",
                path.display()
            )));
        }
        if fs::read_dir(path)
            .map_err(|error| Error::io(path, error))?
            .next()
            .is_some()
        {
            return Err(Error::message(format!(
                "signing payload output must start empty: {}",
                path.display()
            )));
        }
    } else {
        fs::create_dir_all(path).map_err(|error| Error::io(path, error))?;
    }
    fs::canonicalize(path).map_err(|error| Error::io(path, error))
}

#[cfg(test)]
fn map_expected_archives(
    files: &[PathBuf],
    expected: &[ArtifactSpec],
) -> Result<BTreeMap<String, PathBuf>> {
    let expected_names = expected
        .iter()
        .map(|spec| spec.archive.clone())
        .collect::<Vec<_>>();
    map_expected_archive_names(files, &expected_names)
}

fn map_expected_archive_names(
    files: &[PathBuf],
    expected: &[String],
) -> Result<BTreeMap<String, PathBuf>> {
    let expected_names = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut archives = BTreeMap::new();
    for path in files {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if expected_names.contains(name) && archives.insert(name.to_owned(), path.clone()).is_some()
        {
            return Err(Error::message(format!(
                "release input contains duplicate archive {name}"
            )));
        }
    }
    let missing = expected_names
        .iter()
        .filter(|name| !archives.contains_key(**name))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(Error::message(format!(
            "release input is missing expected archives: {}",
            missing.join(", ")
        )));
    }
    Ok(archives)
}

fn require_exact_release_file_set(
    files: &[PathBuf],
    archives: &BTreeMap<String, PathBuf>,
    root: &Path,
    require_signatures: bool,
) -> Result<()> {
    let mut expected = BTreeSet::from([root.join(CHECKSUMS_FILE)]);
    for (archive_name, archive) in archives {
        expected.insert(archive.clone());
        expected.insert(archive.with_file_name(format!("{archive_name}.sha256")));
        if require_signatures {
            expected.insert(archive.with_file_name(format!("{archive_name}.provenance.toml")));
            expected
                .insert(archive.with_file_name(format!("{archive_name}.provenance.sigstore.json")));
        }
    }
    let actual = files.iter().cloned().collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).collect::<Vec<_>>();
    let unexpected = actual.difference(&expected).collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() {
        return Ok(());
    }
    Err(Error::message(format!(
        "release input file set mismatch; missing=[{}] unexpected=[{}]",
        missing
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        unexpected
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

fn validate_archive_manifest(
    repository_root: &Path,
    archive_path: &Path,
    spec: &ArtifactSpec,
    identity: &ReleaseIdentity,
    context: &ValidatedReleaseContext,
) -> Result<ValidatedArchive> {
    let package_bytes = read_regular_package(archive_path)?;
    let effective_header =
        EffectiveHeaderSnapshot::capture(&repository_root.join("boxdd-sys"), "repository release")?;
    let expected_paths = expected_archive_paths(spec, &effective_header)?;
    let directory = tempfile::Builder::new()
        .prefix("boxdd-release-archive-")
        .tempdir()
        .map_err(|error| Error::io("create release archive inspection directory", error))?;
    let files = read_release_archive_bytes(
        archive_path,
        &package_bytes,
        &expected_paths,
        directory.path(),
    )?;

    verify_inner_checksums(&files, archive_path)?;
    verify_repository_owned_files(repository_root, &files, spec, &effective_header)?;
    let effective_source = effective_header.identity();
    if effective_source.upstream_sha != identity.upstream_sha {
        return Err(Error::message(format!(
            "repository effective-source upstream SHA {} does not match release upstream SHA {}",
            effective_source.upstream_sha, identity.upstream_sha
        )));
    }

    let manifest_bytes = files.get("manifest.toml").ok_or_else(|| {
        Error::message(format!(
            "{} does not contain manifest.toml",
            archive_path.display()
        ))
    })?;
    let manifest = ArtifactManifest::parse(manifest_bytes)
        .map_err(|error| Error::message(format!("invalid provider manifest: {error}")))?;
    if manifest.source_commit.as_deref() != Some(identity.commit.as_str())
        || manifest.release_tag.as_deref() != Some(identity.tag.as_str())
    {
        return Err(Error::message(format!(
            "{} provenance identity does not match release tag {} at {}",
            archive_path.display(),
            identity.tag,
            identity.commit
        )));
    }
    let expected_library = expected_library_path(spec);
    let expected_bindings = expected_bindings_path(spec);
    if manifest.archive != expected_library
        || manifest.header != "include/box2d/box2d.h"
        || manifest.bindings != expected_bindings
    {
        return Err(Error::message(format!(
            "{} manifest paths do not match the canonical package layout",
            archive_path.display()
        )));
    }

    let library = directory.path().join(&expected_library);
    let snapshot_layout_hash = u32::try_from(manifest.snapshot_layout_hash).map_err(|_| {
        Error::message("provider snapshot layout hash does not fit the native u32 contract")
    })?;
    let adapter_source_sha256 =
        adapter_source_sha256(&repository_root.join("boxdd-sys")).map_err(Error::message)?;
    let bindings = repository_root
        .join("boxdd-sys/src")
        .join(if spec.precision == "double" {
            "bindings_double.rs"
        } else {
            "bindings_pregenerated.rs"
        });
    let verified = provider_manifest::verify_artifact(
        &directory.path().join("manifest.toml"),
        &ArtifactExpectation {
            identity: ArtifactIdentityExpectation {
                provider: "prebuilt",
                crate_version: &identity.version,
                upstream_sha: &identity.upstream_sha,
                effective_source_sha256: &effective_source.effective_source_sha256,
                precision: spec.precision,
                target: spec.target,
                crt: spec.crt,
                simd: "default",
                validate: false,
                adapter_source_sha256: &adapter_source_sha256,
                private_abi_hash: &manifest.private_abi_hash,
                snapshot_layout_hash,
            },
            header_path: effective_header.header_path(),
            bindings_path: &bindings,
        },
    )
    .map_err(|error| Error::message(format!("provider artifact validation failed: {error}")))?;
    effective_header.revalidate("repository release")?;
    let native_identity = verify_provider_archive(
        &verified.archive_snapshot,
        &ArchiveExpectation {
            target: spec.target,
            required_symbols: provider_manifest::REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &manifest.private_abi_hash,
            snapshot_layout_hash,
        },
    )
    .map_err(|error| Error::message(format!("provider archive proof failed: {error}")))?;
    if native_identity.archive_sha256 != verified.manifest.archive_sha256 {
        return Err(Error::message(
            "packaged provider archive changed between structural and manifest verification",
        ));
    }
    let canonical_library =
        fs::canonicalize(&library).map_err(|error| Error::io(&library, error))?;
    if verified.archive_snapshot.path() != canonical_library.as_path() {
        return Err(Error::message(
            "provider manifest did not resolve to the canonical packaged library",
        ));
    }

    let members = prebuilt_provenance::members_from_files(&files).map_err(Error::message)?;
    let inner_checksums = files
        .get("checksums.sha256")
        .expect("checksums presence was validated above");
    let statement = PrebuiltProvenanceStatement {
        schema_version: prebuilt_provenance::SCHEMA_VERSION,
        schema: prebuilt_provenance::SCHEMA_NAME.to_owned(),
        repository: context.repository.clone(),
        workflow: PUBLISHER_WORKFLOW.to_owned(),
        workflow_ref: context.workflow_ref.clone(),
        source_commit: identity.commit.clone(),
        release_tag: identity.tag.clone(),
        run_id: context.run_id.clone(),
        run_attempt: context.run_attempt.clone(),
        crate_version: identity.version.clone(),
        package_name: spec.archive.clone(),
        package_size: package_bytes.len() as u64,
        package_sha256: provider_manifest::sha256_bytes(&package_bytes),
        provider_manifest_sha256: provider_manifest::sha256_bytes(manifest_bytes),
        inner_checksums_sha256: provider_manifest::sha256_bytes(inner_checksums),
        provider: manifest.provider.clone(),
        target: manifest.target.clone(),
        precision: manifest.precision.clone(),
        link: manifest.link.clone(),
        crt: manifest.crt.clone(),
        upstream_sha: manifest.upstream_sha.clone(),
        effective_source_sha256: manifest.effective_source_sha256.clone(),
        simd: manifest.simd.clone(),
        validate: manifest.validate,
        adapter_abi_version: manifest.adapter_abi_version,
        adapter_source_sha256: manifest.adapter_source_sha256.clone(),
        private_abi_hash: manifest.private_abi_hash.clone(),
        snapshot_layout_hash: manifest.snapshot_layout_hash,
        recording_contract_blake3: manifest.recording_contract_blake3.clone(),
        member_count: members.len() as u64,
        members,
    };
    statement.validate_intrinsic().map_err(Error::message)?;
    statement
        .validate_publisher(PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW)
        .map_err(Error::message)?;
    statement
        .verify_package_bytes(&package_bytes)
        .map_err(Error::message)?;
    statement
        .validate_provider_manifest(manifest_bytes)
        .map_err(Error::message)?;
    statement
        .verify_extracted_root(directory.path())
        .map_err(Error::message)?;
    Ok(ValidatedArchive { statement })
}

fn read_regular_package(path: &Path) -> Result<Vec<u8>> {
    let snapshot = VerifiedFileSnapshot::read(
        path,
        prebuilt_provenance::MAX_PACKAGE_BYTES,
        "release package",
    )
    .map_err(Error::message)?;
    if snapshot.is_empty() {
        return Err(Error::message(format!(
            "release package {} must not be empty; maximum is {}",
            path.display(),
            prebuilt_provenance::MAX_PACKAGE_BYTES
        )));
    }
    Ok(snapshot.into_bytes())
}

#[cfg(test)]
fn read_release_archive(
    archive_path: &Path,
    expected_paths: &BTreeSet<String>,
    destination_root: &Path,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let bytes = read_regular_package(archive_path)?;
    read_release_archive_bytes(archive_path, &bytes, expected_paths, destination_root)
}

fn read_release_archive_bytes(
    archive_path: &Path,
    archive_bytes: &[u8],
    expected_paths: &BTreeSet<String>,
    destination_root: &Path,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let decoder = GzDecoder::new(std::io::Cursor::new(archive_bytes));
    let mut archive = tar::Archive::new(BoundedReader::new(
        decoder,
        MAX_ARCHIVE_STREAM_BYTES,
        "decompressed release archive exceeds the stream limit",
    ));
    let entries = archive
        .entries()
        .map_err(|error| Error::message(format!("read {}: {error}", archive_path.display())))?
        .raw(true);
    let mut files = BTreeMap::new();
    let mut total_bytes = 0_u64;
    let mut entry_count = 0_usize;
    let mut previous_path = None::<String>;
    for entry in entries {
        entry_count = entry_count
            .checked_add(1)
            .ok_or_else(|| Error::message("release archive entry count overflow"))?;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            return Err(Error::message(format!(
                "{} exceeds the {} entry limit",
                archive_path.display(),
                MAX_ARCHIVE_ENTRIES
            )));
        }
        let entry = entry.map_err(|error| {
            Error::message(format!("read {} entry: {error}", archive_path.display()))
        })?;
        let (path, bytes) = read_canonical_archive_entry(entry, archive_path)?;
        if !expected_paths.contains(&path) {
            return Err(Error::message(format!(
                "{} contains unexpected file {path:?}",
                archive_path.display()
            )));
        }
        if previous_path
            .as_ref()
            .is_some_and(|previous| previous >= &path)
        {
            return Err(Error::message(format!(
                "{} entries are duplicated or not in canonical path order at {path:?}",
                archive_path.display()
            )));
        }
        previous_path = Some(path.clone());
        total_bytes = total_bytes
            .checked_add(bytes.len() as u64)
            .filter(|total| *total <= MAX_ARCHIVE_TOTAL_BYTES)
            .ok_or_else(|| {
                Error::message(format!(
                    "{} exceeds the {} byte uncompressed limit",
                    archive_path.display(),
                    MAX_ARCHIVE_TOTAL_BYTES
                ))
            })?;
        if files.insert(path.clone(), bytes.clone()).is_some() {
            return Err(Error::message(format!(
                "{} contains duplicate file {path:?}",
                archive_path.display()
            )));
        }
        let destination = destination_root.join(&path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| Error::io(&destination, error))?;
        output
            .write_all(&bytes)
            .map_err(|error| Error::io(&destination, error))?;
    }
    let actual_paths = files.keys().cloned().collect::<BTreeSet<_>>();
    require_exact_archive_paths(&actual_paths, expected_paths, archive_path)?;
    let mut bounded = archive.into_inner();
    let mut decompressed_tail = Vec::new();
    bounded
        .by_ref()
        .take(TAR_BLOCK_BYTES + 1)
        .read_to_end(&mut decompressed_tail)
        .map_err(|error| {
            Error::message(format!(
                "read {} canonical tar terminator: {error}",
                archive_path.display()
            ))
        })?;
    if decompressed_tail.len() as u64 != TAR_BLOCK_BYTES
        || decompressed_tail.iter().any(|byte| *byte != 0)
    {
        return Err(Error::message(format!(
            "{} does not contain exactly two canonical tar termination blocks",
            archive_path.display()
        )));
    }
    let decoder = bounded.into_inner();
    if decoder.get_ref().position() != archive_bytes.len() as u64 {
        return Err(Error::message(format!(
            "{} contains a second gzip member or trailing compressed data",
            archive_path.display()
        )));
    }
    Ok(files)
}

fn expected_archive_paths(
    spec: &ArtifactSpec,
    effective_headers: &EffectiveHeaderSnapshot,
) -> Result<BTreeSet<String>> {
    let mut paths = BTreeSet::from([
        "checksums.sha256".to_owned(),
        "licenses/BOX2D-LICENSE".to_owned(),
        "licenses/PROJECT-LICENSE-APACHE".to_owned(),
        "licenses/PROJECT-LICENSE-MIT".to_owned(),
        "manifest.toml".to_owned(),
        "metadata/effective-source.toml".to_owned(),
        "metadata/upstream.toml".to_owned(),
        expected_bindings_path(spec),
        expected_library_path(spec),
    ]);
    paths.extend(effective_headers.public_header_paths()?);
    Ok(paths)
}

fn require_exact_archive_paths(
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
    archive_path: &Path,
) -> Result<()> {
    let missing = expected.difference(actual).cloned().collect::<Vec<_>>();
    let extra = actual.difference(expected).cloned().collect::<Vec<_>>();
    if missing.is_empty() && extra.is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{} package layout mismatch; missing=[{}] extra=[{}]",
            archive_path.display(),
            missing.join(", "),
            extra.join(", ")
        )))
    }
}

fn expected_library_path(spec: &ArtifactSpec) -> String {
    if spec.target.ends_with("-windows-msvc") {
        "lib/box2d.lib".to_owned()
    } else {
        "lib/libbox2d.a".to_owned()
    }
}

fn expected_bindings_path(spec: &ArtifactSpec) -> String {
    if spec.precision == "double" {
        "bindings/bindings_double.rs".to_owned()
    } else {
        "bindings/bindings_pregenerated.rs".to_owned()
    }
}

fn read_canonical_archive_entry<R: Read>(
    entry: tar::Entry<'_, R>,
    archive_path: &Path,
) -> Result<(String, Vec<u8>)> {
    let header = entry.header();
    if !header.entry_type().is_file() {
        return Err(Error::message(format!(
            "{} contains a non-regular tar entry",
            archive_path.display()
        )));
    }
    let mode = header.mode().map_err(|error| {
        Error::message(format!(
            "read {} entry mode: {error}",
            archive_path.display()
        ))
    })?;
    let uid = header.uid().map_err(|error| {
        Error::message(format!(
            "read {} entry uid: {error}",
            archive_path.display()
        ))
    })?;
    let gid = header.gid().map_err(|error| {
        Error::message(format!(
            "read {} entry gid: {error}",
            archive_path.display()
        ))
    })?;
    let mtime = header.mtime().map_err(|error| {
        Error::message(format!(
            "read {} entry mtime: {error}",
            archive_path.display()
        ))
    })?;
    if mode != 0o644 || uid != 0 || gid != 0 || mtime != 0 {
        return Err(Error::message(format!(
            "{} contains non-canonical tar metadata: mode={mode:o} uid={uid} gid={gid} mtime={mtime}",
            archive_path.display()
        )));
    }
    let path = entry.path().map_err(|error| {
        Error::message(format!(
            "read {} entry path: {error}",
            archive_path.display()
        ))
    })?;
    let rendered = canonical_archive_path(&path, archive_path)?;
    let declared_size = entry.header().size().map_err(|error| {
        Error::message(format!(
            "read {} entry size for {rendered:?}: {error}",
            archive_path.display()
        ))
    })?;
    require_archive_entry_size(archive_path, &rendered, declared_size)?;
    let mut bytes = Vec::with_capacity(declared_size as usize);
    entry
        .take(MAX_ARCHIVE_ENTRY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            Error::message(format!(
                "read {} entry {rendered:?}: {error}",
                archive_path.display()
            ))
        })?;
    if bytes.len() as u64 != declared_size {
        return Err(Error::message(format!(
            "{} entry {rendered:?} is truncated or exceeds its declared size",
            archive_path.display()
        )));
    }
    Ok((rendered, bytes))
}

fn require_archive_entry_size(archive_path: &Path, path: &str, size: u64) -> Result<()> {
    if size <= MAX_ARCHIVE_ENTRY_BYTES {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{} entry {path:?} exceeds the {} byte limit",
            archive_path.display(),
            MAX_ARCHIVE_ENTRY_BYTES
        )))
    }
}

fn canonical_archive_path(path: &Path, archive_path: &Path) -> Result<String> {
    let rendered = path
        .to_str()
        .ok_or_else(|| Error::message("release archive paths must be UTF-8"))?
        .to_owned();
    if rendered.contains('\\')
        || rendered.contains("//")
        || rendered.starts_with("./")
        || rendered.ends_with('/')
        || Path::new(&rendered)
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(Error::message(format!(
            "{} contains unsafe or non-canonical path {rendered:?}",
            archive_path.display()
        )));
    }
    Ok(rendered)
}

fn verify_inner_checksums(files: &BTreeMap<String, Vec<u8>>, archive_path: &Path) -> Result<()> {
    let actual = files
        .get("checksums.sha256")
        .ok_or_else(|| Error::message("prebuilt package is missing checksums.sha256"))?;
    let mut expected = String::new();
    for (path, bytes) in files {
        if path != "checksums.sha256" {
            expected.push_str(&format!(
                "{}  {path}\n",
                provider_manifest::sha256_bytes(bytes)
            ));
        }
    }
    if actual.as_slice() != expected.as_bytes() {
        return Err(Error::message(format!(
            "{} contains a stale, non-canonical, or tampered checksums.sha256",
            archive_path.display()
        )));
    }
    Ok(())
}

fn verify_repository_owned_files(
    repository_root: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    spec: &ArtifactSpec,
    effective_headers: &EffectiveHeaderSnapshot,
) -> Result<()> {
    let fixed = [
        (
            "metadata/effective-source.toml",
            "boxdd-sys/effective-source.toml",
        ),
        ("metadata/upstream.toml", "boxdd-sys/upstream.toml"),
    ];
    for (packaged, source) in fixed {
        require_packaged_bytes(files, packaged, &repository_root.join(source))?;
    }
    let licenses = [
        ("licenses/PROJECT-LICENSE-MIT", "LICENSE-MIT"),
        ("licenses/PROJECT-LICENSE-APACHE", "LICENSE-APACHE"),
        (
            "licenses/BOX2D-LICENSE",
            "boxdd-sys/third-party/box2d/LICENSE",
        ),
    ];
    for (packaged, source) in licenses {
        require_packaged_text_bytes(files, packaged, &repository_root.join(source))?;
    }
    verify_packaged_effective_headers(files, effective_headers)?;
    let bindings = expected_bindings_path(spec);
    require_packaged_bytes(
        files,
        &bindings,
        &repository_root
            .join("boxdd-sys/src")
            .join(Path::new(&bindings).file_name().expect("binding file name")),
    )
}

fn verify_packaged_effective_headers(
    files: &BTreeMap<String, Vec<u8>>,
    effective_headers: &EffectiveHeaderSnapshot,
) -> Result<()> {
    for (packaged, bytes) in files
        .iter()
        .filter(|(path, _)| path.starts_with("include/box2d/"))
    {
        effective_headers.verify_packaged_header(packaged, bytes)?;
    }
    Ok(())
}

fn require_packaged_bytes(
    files: &BTreeMap<String, Vec<u8>>,
    packaged: &str,
    source: &Path,
) -> Result<()> {
    let expected = VerifiedFileSnapshot::read(source, MAX_ARCHIVE_ENTRY_BYTES, "package source")
        .map_err(Error::message)?;
    if files.get(packaged).map(Vec::as_slice) != Some(expected.bytes()) {
        return Err(Error::message(format!(
            "packaged {packaged} does not exactly match {}",
            source.display()
        )));
    }
    Ok(())
}

fn require_packaged_text_bytes(
    files: &BTreeMap<String, Vec<u8>>,
    packaged: &str,
    source: &Path,
) -> Result<()> {
    let expected = VerifiedFileSnapshot::read(source, MAX_ARCHIVE_ENTRY_BYTES, "package source")
        .map_err(Error::message)?;
    let expected = normalize_crlf(expected.into_bytes());
    if files.get(packaged).map(Vec::as_slice) != Some(expected.as_slice()) {
        return Err(Error::message(format!(
            "packaged {packaged} does not match canonical LF content from {}",
            source.display()
        )));
    }
    Ok(())
}

fn snapshot_verification_input(
    source: &Path,
    destination_root: &Path,
    destination_name: &str,
    maximum_bytes: u64,
    label: &str,
) -> Result<VerifiedFileSnapshot> {
    let destination = destination_root.join(destination_name);
    snapshot_file_create_new(source, &destination, maximum_bytes, label).map_err(Error::message)
}

fn revalidate_sigstore_inputs(
    statement: &VerifiedFileSnapshot,
    bundle: &VerifiedFileSnapshot,
    trusted_root: &VerifiedFileSnapshot,
    artifact_kind: &str,
) -> Result<()> {
    statement
        .revalidate(&format!(
            "authenticated {artifact_kind} provenance statement"
        ))
        .map_err(Error::message)?;
    bundle
        .revalidate(&format!("authenticated {artifact_kind} Sigstore bundle"))
        .map_err(Error::message)?;
    trusted_root
        .revalidate("authenticated Sigstore trusted root")
        .map_err(Error::message)
}

fn require_trusted_root(path: &Path, bytes: &[u8]) -> Result<()> {
    let digest = provider_manifest::sha256_bytes(bytes);
    if digest == SIGSTORE_TRUSTED_ROOT_SHA256 {
        Ok(())
    } else {
        Err(Error::message(format!(
            "Sigstore trusted root {} has digest {digest}; crate-owned trust anchor requires {SIGSTORE_TRUSTED_ROOT_SHA256}",
            path.display()
        )))
    }
}

fn verify_cosign_version(cosign: &Path) -> Result<()> {
    let mut command = cosign_command(cosign);
    command.arg("version");
    let output =
        run_output(&mut command, "release Cosign version qualification").map_err(Error::message)?;
    require_success(&output, "cosign version")?;
    let source = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if provenance_policy::cosign_version_is_qualified(&source) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "release verification requires exact Cosign {COSIGN_VERSION}; found {}",
            source
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("unknown")
        )))
    }
}

fn verify_sigstore(
    cosign: &Path,
    payload: &Path,
    bundle: &Path,
    trusted_root: &Path,
    identity: &ReleaseIdentity,
) -> Result<()> {
    let args = provenance_policy::cosign_verify_blob_args(provenance_policy::PrebuiltProvenance {
        crate_version: &identity.version,
        source_commit: &identity.commit,
        release_tag: &identity.tag,
        payload,
        bundle,
        trusted_root,
    })
    .map_err(|error| Error::message(format!("invalid Sigstore policy input: {error}")))?;
    let mut command = cosign_command(cosign);
    command.args(args);
    let label = format!("verify Sigstore identity for {}", payload.display());
    let status = run_status(&mut command, &label).map_err(Error::message)?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with status {status}"
        )))
    }
}

fn release_git_command() -> Result<Command> {
    isolated_git_command().map_err(Error::message)
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| Error::io(&directory, error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| Error::io(&directory, error))?;
        if directory != root && entries.is_empty() {
            return Err(Error::message(format!(
                "release input cannot contain empty artifact directories: {}",
                directory.display()
            )));
        }
        for entry in entries {
            let kind = entry
                .file_type()
                .map_err(|error| Error::io(entry.path(), error))?;
            if kind.is_symlink() {
                return Err(Error::message(format!(
                    "release input cannot contain symlinks: {}",
                    entry.path().display()
                )));
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            } else {
                return Err(Error::message(format!(
                    "release input cannot contain special filesystem entries: {}",
                    entry.path().display()
                )));
            }
        }
    }
    files.sort();
    Ok(files)
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let source = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    toml::from_str(&source)
        .map_err(|error| Error::message(format!("{} is invalid TOML: {error}", path.display())))
}

fn validate_git_sha(label: &str, value: &str) -> Result<()> {
    if value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} must be a lowercase 40-character Git SHA"
        )))
    }
}

fn git_output(root: &Path, args: &[&str], label: &str) -> Result<String> {
    let mut command = release_git_command()?;
    command.current_dir(root).args(args);
    let output = run_output(&mut command, label).map_err(Error::message)?;
    require_success(&output, label)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn require_success(output: &Output, label: &str) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tar_fixture(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_uid(0);
            header.set_gid(0);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            archive.append_data(&mut header, *name, *bytes).unwrap();
        }
        archive.finish().unwrap();
    }

    fn read_gzip_payload(path: &Path) -> Vec<u8> {
        let mut decoder = flate2::read::GzDecoder::new(fs::File::open(path).unwrap());
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes).unwrap();
        bytes
    }

    fn write_gzip_payload(path: &Path, bytes: &[u8]) {
        let file = fs::File::create(path).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap();
    }

    fn write_metadata_tar_fixture(path: &Path, entry_type: tar::EntryType) {
        let file = fs::File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let metadata_bytes = if entry_type.is_gnu_longname() || entry_type.is_gnu_longlink() {
            b"a\0".as_slice()
        } else {
            b"10 path=a\n".as_slice()
        };
        let mut metadata = tar::Header::new_gnu();
        metadata.set_entry_type(entry_type);
        metadata.set_mode(0o644);
        metadata.set_mtime(0);
        metadata.set_uid(0);
        metadata.set_gid(0);
        metadata.set_size(metadata_bytes.len() as u64);
        metadata.set_cksum();
        archive
            .append_data(&mut metadata, "metadata", metadata_bytes)
            .unwrap();
        let mut file = tar::Header::new_gnu();
        file.set_mode(0o644);
        file.set_mtime(0);
        file.set_uid(0);
        file.set_gid(0);
        file.set_size(3);
        file.set_cksum();
        archive
            .append_data(&mut file, "a", b"one".as_slice())
            .unwrap();
        archive.finish().unwrap();
    }

    #[test]
    fn repository_text_comparison_accepts_crlf_source_for_canonical_lf_member() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("LICENSE");
        fs::write(&source, b"first\r\nsecond\r\n").unwrap();
        let files = BTreeMap::from([(
            "licenses/PROJECT-LICENSE-MIT".to_owned(),
            b"first\nsecond\n".to_vec(),
        )]);

        require_packaged_text_bytes(&files, "licenses/PROJECT-LICENSE-MIT", &source).unwrap();
        assert!(require_packaged_bytes(&files, "licenses/PROJECT-LICENSE-MIT", &source).is_err());
    }

    fn write_symlink_tar_fixture(path: &Path) {
        let file = fs::File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_mode(0o777);
        header.set_size(0);
        header.set_link_name("outside").unwrap();
        header.set_cksum();
        archive
            .append_data(&mut header, "manifest.toml", std::io::empty())
            .unwrap();
        archive.finish().unwrap();
    }

    #[test]
    fn release_artifact_matrix_is_exact_and_unambiguous() {
        let artifacts = expected_artifacts("0.6.0");
        let native_artifact_count = PLATFORMS.len() * RELEASE_PRECISIONS.len();
        assert_eq!(artifacts.len(), native_artifact_count);
        assert_eq!(
            artifacts
                .iter()
                .map(|artifact| artifact.archive.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            native_artifact_count
        );
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.archive.ends_with("-md.tar.gz"))
        );
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.archive.ends_with("-mt.tar.gz"))
        );
        assert!(artifacts.iter().all(|artifact| {
            artifact.archive.contains(artifact.target)
                && artifact.archive.contains(artifact.precision)
        }));
        let all = expected_release_archive_names("0.6.0").unwrap();
        let release_artifact_count = native_artifact_count + RELEASE_PRECISIONS.len();
        assert_eq!(all.len(), release_artifact_count);
        assert_eq!(
            all.iter().collect::<BTreeSet<_>>().len(),
            release_artifact_count
        );
        assert!(all.iter().any(|archive| {
            archive == "boxdd-wasm-provider-0.6.0-wasm32-unknown-unknown-single.tar.gz"
        }));
        assert!(all.iter().any(|archive| {
            archive == "boxdd-wasm-provider-0.6.0-wasm32-unknown-unknown-double.tar.gz"
        }));
    }

    #[test]
    fn tag_version_and_commit_validation_fail_closed() {
        assert!(validate_tag("v0.6.0", "0.6.0").is_ok());
        assert!(validate_tag("boxdd-sys-v0.6.0", "0.6.0").is_ok());
        assert!(validate_tag("v0.6.1", "0.6.0").is_err());
        assert!(validate_tag("main", "0.6.0").is_err());
        assert!(validate_git_sha("commit", "1234567890abcdef1234567890abcdef12345678").is_ok());
        assert!(validate_git_sha("commit", "1234").is_err());
    }

    #[test]
    fn command_parser_rejects_arbitrary_or_duplicate_inputs() {
        let version = Options::parse(&["--print-package-version".to_owned()]).unwrap();
        assert_eq!(version.mode, Mode::PrintPackageVersion);
        assert!(
            Options::parse(&[
                "--print-package-version".to_owned(),
                "--tag".to_owned(),
                "v0.6.0".to_owned(),
            ])
            .is_err()
        );
        let options = Options::parse(&[
            "--check".to_owned(),
            "--tag".to_owned(),
            "v0.6.0".to_owned(),
            "--artifacts".to_owned(),
            "inputs".to_owned(),
        ])
        .unwrap();
        assert_eq!(options.mode, Mode::Check);
        assert_eq!(options.tag.as_deref(), Some("v0.6.0"));
        let attempt = Options::parse(&[
            "--check".to_owned(),
            "--run-id".to_owned(),
            "42".to_owned(),
            "--run-attempt".to_owned(),
            "2".to_owned(),
            "--artifacts".to_owned(),
            "inputs".to_owned(),
        ])
        .unwrap();
        assert_eq!(attempt.run_attempt.as_deref(), Some("2"));
        assert!(Options::parse(&["--check".to_owned()]).is_err());
        assert!(Options::parse(&["--check".to_owned(), "--branch".to_owned()]).is_err());
        assert!(
            Options::parse(&[
                "--check".to_owned(),
                "--tag".to_owned(),
                "v0.6.0".to_owned(),
                "--tag".to_owned(),
                "v0.6.1".to_owned(),
            ])
            .is_err()
        );
        let content = Options::parse(&[
            "--check-content".to_owned(),
            "--artifacts".to_owned(),
            "inputs".to_owned(),
            "--payloads".to_owned(),
            "payloads".to_owned(),
        ])
        .unwrap();
        assert_eq!(content.mode, Mode::CheckContent);
        assert!(
            Options::parse(&[
                "--check".to_owned(),
                "--payloads".to_owned(),
                "payloads".to_owned(),
            ])
            .is_err()
        );
    }

    #[test]
    fn archive_mapping_rejects_missing_and_duplicate_inputs() {
        let expected = expected_artifacts("0.6.0");
        let files = expected
            .iter()
            .map(|artifact| PathBuf::from(&artifact.archive))
            .collect::<Vec<_>>();
        assert_eq!(map_expected_archives(&files, &expected).unwrap().len(), 10);
        assert!(map_expected_archives(&files[..9], &expected).is_err());
        let mut duplicate = files;
        duplicate.push(duplicate[0].clone());
        assert!(map_expected_archives(&duplicate, &expected).is_err());
    }

    #[test]
    fn release_file_set_requires_exact_unsigned_and_signed_assets() {
        let root = PathBuf::from("release-inputs");
        let archives = expected_release_archive_names("0.6.0")
            .unwrap()
            .into_iter()
            .map(|name| {
                let path = root.join(format!("artifact-{name}")).join(&name);
                (name, path)
            })
            .collect::<BTreeMap<_, _>>();
        let mut unsigned = vec![root.join(CHECKSUMS_FILE)];
        for (name, archive) in &archives {
            unsigned.push(archive.clone());
            unsigned.push(archive.with_file_name(format!("{name}.sha256")));
        }
        assert_eq!(unsigned.len(), 25);
        assert!(require_exact_release_file_set(&unsigned, &archives, &root, false).is_ok());

        let mut signed = unsigned.clone();
        for (name, archive) in &archives {
            signed.push(archive.with_file_name(format!("{name}.provenance.toml")));
            signed.push(archive.with_file_name(format!("{name}.provenance.sigstore.json")));
        }
        assert_eq!(signed.len(), 49);
        assert!(require_exact_release_file_set(&signed, &archives, &root, true).is_ok());

        let mut missing_statement = signed.clone();
        missing_statement.pop();
        assert!(
            require_exact_release_file_set(&missing_statement, &archives, &root, true).is_err()
        );
        let mut legacy = signed;
        let (name, archive) = archives.iter().next().unwrap();
        legacy.push(archive.with_file_name(format!("{name}.manifest")));
        assert!(require_exact_release_file_set(&legacy, &archives, &root, true).is_err());
    }

    #[test]
    fn verification_inputs_are_bounded_private_create_new_snapshots() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let snapshots = temp.path().join("snapshots");
        fs::create_dir(&snapshots).unwrap();
        fs::write(&source, b"trusted bytes").unwrap();
        let snapshot =
            snapshot_verification_input(&source, &snapshots, "statement.toml", 64, "fixture")
                .unwrap();
        assert_eq!(snapshot.bytes(), b"trusted bytes");
        assert_eq!(fs::read(snapshot.path()).unwrap(), b"trusted bytes");
        fs::write(&source, b"changed bytes").unwrap();
        assert_eq!(snapshot.bytes(), b"trusted bytes");
        snapshot.revalidate("fixture").unwrap();
        fs::write(snapshot.path(), b"tampered bytes").unwrap();
        assert!(snapshot.revalidate("fixture").is_err());
        assert!(
            snapshot_verification_input(&source, &snapshots, "statement.toml", 64, "fixture",)
                .is_err(),
            "snapshot helper overwrote an existing private input"
        );
        assert!(
            snapshot_verification_input(&source, &snapshots, "oversized", 2, "fixture").is_err()
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = temp.path().join("source-link");
            symlink(&source, &link).unwrap();
            assert!(snapshot_verification_input(&link, &snapshots, "link", 64, "fixture").is_err());
        }
    }

    #[test]
    fn sigstore_postflight_revalidates_statement_bundle_and_trusted_root() {
        let temp = TempDir::new().unwrap();
        let sources = temp.path().join("sources");
        let snapshots = temp.path().join("snapshots");
        fs::create_dir(&sources).unwrap();
        fs::create_dir(&snapshots).unwrap();
        let statement_source = sources.join("statement");
        let bundle_source = sources.join("bundle");
        let trusted_root_source = sources.join("trusted-root");
        fs::write(&statement_source, b"statement").unwrap();
        fs::write(&bundle_source, b"bundle").unwrap();
        fs::write(&trusted_root_source, b"trusted-root").unwrap();
        let statement = snapshot_verification_input(
            &statement_source,
            &snapshots,
            "statement",
            64,
            "statement",
        )
        .unwrap();
        let bundle =
            snapshot_verification_input(&bundle_source, &snapshots, "bundle", 64, "bundle")
                .unwrap();
        let trusted_root = snapshot_verification_input(
            &trusted_root_source,
            &snapshots,
            "trusted-root",
            64,
            "trusted root",
        )
        .unwrap();
        revalidate_sigstore_inputs(&statement, &bundle, &trusted_root, "fixture").unwrap();

        for snapshot in [&statement, &bundle, &trusted_root] {
            let original = snapshot.bytes().to_vec();
            fs::write(snapshot.path(), vec![b'!'; original.len()]).unwrap();
            assert!(
                revalidate_sigstore_inputs(&statement, &bundle, &trusted_root, "fixture").is_err()
            );
            fs::write(snapshot.path(), original).unwrap();
        }
        revalidate_sigstore_inputs(&statement, &bundle, &trusted_root, "fixture").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn release_file_collection_rejects_special_entries() {
        use std::os::unix::net::UnixListener;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("regular"), b"bytes").unwrap();
        let socket = temp.path().join("socket");
        let _listener = UnixListener::bind(&socket).unwrap();
        assert!(
            collect_files(temp.path()).is_err(),
            "release input traversal silently ignored a Unix socket"
        );
    }

    #[test]
    fn release_file_collection_rejects_empty_artifact_directories() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("regular"), b"bytes").unwrap();
        fs::create_dir(temp.path().join("unexpected-empty-artifact")).unwrap();
        assert!(
            collect_files(temp.path()).is_err(),
            "release input traversal silently ignored an empty artifact directory"
        );
    }

    #[test]
    fn archive_layout_rejects_missing_and_extra_files() {
        let expected = BTreeSet::from(["manifest.toml".to_owned(), "lib/libbox2d.a".to_owned()]);
        assert!(require_exact_archive_paths(&expected, &expected, Path::new("ok.tar.gz")).is_ok());

        let missing = BTreeSet::from(["manifest.toml".to_owned()]);
        assert!(
            require_exact_archive_paths(&missing, &expected, Path::new("missing.tar.gz")).is_err()
        );

        let mut extra = expected.clone();
        extra.insert("unexpected.txt".to_owned());
        assert!(require_exact_archive_paths(&extra, &expected, Path::new("extra.tar.gz")).is_err());
    }

    #[test]
    fn archive_paths_reject_traversal_and_noncanonical_spellings() {
        let archive = Path::new("fixture.tar.gz");
        assert_eq!(
            canonical_archive_path(Path::new("lib/libbox2d.a"), archive).unwrap(),
            "lib/libbox2d.a"
        );
        for path in [
            "../libbox2d.a",
            "/libbox2d.a",
            "./libbox2d.a",
            "lib//libbox2d.a",
            "lib\\libbox2d.a",
        ] {
            assert!(
                canonical_archive_path(Path::new(path), archive).is_err(),
                "unexpectedly accepted {path:?}"
            );
        }
    }

    #[test]
    fn archive_reader_rejects_nonregular_unsorted_duplicate_and_extra_entries() {
        let temp = tempfile::tempdir().unwrap();
        let expected = BTreeSet::from(["a".to_owned(), "b".to_owned()]);

        let valid = temp.path().join("valid.tar.gz");
        write_tar_fixture(&valid, &[("a", b"one"), ("b", b"two")]);
        let output = temp.path().join("valid-output");
        fs::create_dir(&output).unwrap();
        let files = read_release_archive(&valid, &expected, &output).unwrap();
        assert_eq!(files["a"], b"one");

        let canonical_tar = read_gzip_payload(&valid);
        assert!(canonical_tar.ends_with(&[0; 1024]));
        let missing_terminator = temp.path().join("missing-terminator.tar.gz");
        write_gzip_payload(
            &missing_terminator,
            &canonical_tar[..canonical_tar.len() - TAR_BLOCK_BYTES as usize],
        );
        let output = temp.path().join("missing-terminator-output");
        fs::create_dir(&output).unwrap();
        assert!(read_release_archive(&missing_terminator, &expected, &output).is_err());

        let extra_terminator = temp.path().join("extra-terminator.tar.gz");
        let mut extra_tar = canonical_tar;
        extra_tar.extend_from_slice(&[0; TAR_BLOCK_BYTES as usize]);
        write_gzip_payload(&extra_terminator, &extra_tar);
        let output = temp.path().join("extra-terminator-output");
        fs::create_dir(&output).unwrap();
        assert!(read_release_archive(&extra_terminator, &expected, &output).is_err());

        let trailing = temp.path().join("trailing.tar.gz");
        fs::copy(&valid, &trailing).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(&trailing)
            .unwrap()
            .write_all(b"trailing")
            .unwrap();
        let output = temp.path().join("trailing-output");
        fs::create_dir(&output).unwrap();
        assert!(read_release_archive(&trailing, &expected, &output).is_err());

        let second_member = temp.path().join("second-member.tar.gz");
        let mut first = fs::read(&valid).unwrap();
        first.extend_from_slice(&fs::read(&valid).unwrap());
        fs::write(&second_member, first).unwrap();
        let output = temp.path().join("second-member-output");
        fs::create_dir(&output).unwrap();
        assert!(read_release_archive(&second_member, &expected, &output).is_err());

        for (name, entries) in [
            (
                "unsorted",
                vec![("b", b"two".as_slice()), ("a", b"one".as_slice())],
            ),
            (
                "duplicate",
                vec![("a", b"one".as_slice()), ("a", b"two".as_slice())],
            ),
            (
                "extra",
                vec![
                    ("a", b"one".as_slice()),
                    ("b", b"two".as_slice()),
                    ("c", b"extra".as_slice()),
                ],
            ),
        ] {
            let archive = temp.path().join(format!("{name}.tar.gz"));
            write_tar_fixture(&archive, &entries);
            let output = temp.path().join(format!("{name}-output"));
            fs::create_dir(&output).unwrap();
            assert!(read_release_archive(&archive, &expected, &output).is_err());
        }

        let symlink = temp.path().join("symlink.tar.gz");
        write_symlink_tar_fixture(&symlink);
        let output = temp.path().join("symlink-output");
        fs::create_dir(&output).unwrap();
        assert!(
            read_release_archive(
                &symlink,
                &BTreeSet::from(["manifest.toml".to_owned()]),
                &output,
            )
            .is_err()
        );

        let metadata = temp.path().join("metadata.tar.gz");
        let file = fs::File::create(&metadata).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        header.set_size(3);
        header.set_cksum();
        archive
            .append_data(&mut header, "a", b"one".as_slice())
            .unwrap();
        archive.finish().unwrap();
        let output = temp.path().join("metadata-output");
        fs::create_dir(&output).unwrap();
        assert!(
            read_release_archive(&metadata, &BTreeSet::from(["a".to_owned()]), &output,).is_err()
        );

        for (name, entry_type) in [
            ("pax", tar::EntryType::XHeader),
            ("global-pax", tar::EntryType::XGlobalHeader),
            ("gnu-long-name", tar::EntryType::GNULongName),
            ("gnu-long-link", tar::EntryType::GNULongLink),
        ] {
            let metadata = temp.path().join(format!("{name}.tar.gz"));
            write_metadata_tar_fixture(&metadata, entry_type);
            let output = temp.path().join(format!("{name}-output"));
            fs::create_dir(&output).unwrap();
            assert!(
                read_release_archive(&metadata, &BTreeSet::from(["a".to_owned()]), &output)
                    .is_err(),
                "raw archive reader accepted hidden metadata entry {name}"
            );
        }
    }

    #[test]
    fn archive_reader_enforces_entry_and_total_size_bounds() {
        assert!(require_archive_entry_size(Path::new("ok"), "entry", 0).is_ok());
        assert!(
            require_archive_entry_size(
                Path::new("too-large"),
                "entry",
                MAX_ARCHIVE_ENTRY_BYTES + 1,
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_inner_checksums_detect_tampering() {
        let archive = Path::new("fixture.tar.gz");
        let mut files = BTreeMap::from([("manifest.toml".to_owned(), b"identity".to_vec())]);
        let checksums = format!(
            "{}  manifest.toml\n",
            provider_manifest::sha256_bytes(b"identity")
        );
        files.insert("checksums.sha256".to_owned(), checksums.into_bytes());
        assert!(verify_inner_checksums(&files, archive).is_ok());
        files.insert("manifest.toml".to_owned(), b"tampered".to_vec());
        assert!(verify_inner_checksums(&files, archive).is_err());
    }

    #[test]
    fn context_identity_rejects_cli_override_and_accepts_equal_values() {
        assert_eq!(
            reconcile_context(
                Some("run-1".to_owned()),
                Some("run-1".to_owned()),
                "GITHUB_RUN_ID",
            )
            .unwrap()
            .as_deref(),
            Some("run-1")
        );
        assert!(
            reconcile_context(
                Some("run-1".to_owned()),
                Some("run-2".to_owned()),
                "GITHUB_RUN_ID",
            )
            .is_err()
        );
    }

    #[test]
    fn identity_matching_rejects_mismatched_checkout_or_tag() {
        assert!(require_matching_identity("HEAD", "abc", "abc").is_ok());
        assert!(require_matching_identity("HEAD", "abc", "def").is_err());
        assert!(require_clean_status("checkout", "").is_ok());
        assert!(require_clean_status("checkout", "?? injected.c").is_err());
    }

    #[test]
    fn github_git_environment_filter_is_case_insensitive_and_exact() {
        assert!(is_git_environment_key(OsStr::new("GIT_DIR")));
        assert!(is_git_environment_key(OsStr::new("git_object_directory")));
        assert!(!is_git_environment_key(OsStr::new("GITHUB_SHA")));
        assert!(!is_git_environment_key(OsStr::new("LEGIT_SETTING")));
    }

    #[test]
    fn expected_package_layout_includes_owned_headers_licenses_and_precision_binding() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask must live below the workspace");
        let spec = ArtifactSpec {
            target: "x86_64-unknown-linux-gnu",
            precision: "double",
            crt: "none",
            archive: "fixture.tar.gz".to_owned(),
        };
        let effective_headers =
            EffectiveHeaderSnapshot::capture(&workspace.join("boxdd-sys"), "test fixture").unwrap();
        let paths = expected_archive_paths(&spec, &effective_headers).unwrap();
        assert!(paths.contains("bindings/bindings_double.rs"));
        assert!(paths.contains("include/box2d/box2d.h"));
        assert!(paths.contains("licenses/PROJECT-LICENSE-MIT"));
        assert!(paths.contains("licenses/PROJECT-LICENSE-APACHE"));
        assert!(paths.contains("licenses/BOX2D-LICENSE"));
        assert!(paths.contains("checksums.sha256"));
        assert!(paths.contains("metadata/effective-source.toml"));
    }

    #[test]
    fn packaged_effective_source_metadata_must_match_repository_bytes() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask must live below the workspace");
        let source = workspace.join("boxdd-sys/effective-source.toml");
        let canonical = fs::read(&source).unwrap();
        let packaged = "metadata/effective-source.toml";
        let mut files = BTreeMap::from([(packaged.to_owned(), canonical.clone())]);
        assert!(require_packaged_bytes(&files, packaged, &source).is_ok());

        files.get_mut(packaged).unwrap().push(b'\n');
        assert!(require_packaged_bytes(&files, packaged, &source).is_err());
    }

    #[test]
    fn packaged_headers_are_verified_against_materialized_effective_source() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask must live below the workspace");
        let manifest_dir = workspace.join("boxdd-sys");
        let effective_headers =
            EffectiveHeaderSnapshot::capture(&manifest_dir, "test fixture").unwrap();
        let packaged = "include/box2d/box2d.h";
        let effective = fs::read(effective_headers.header_path()).unwrap();
        let vendored =
            fs::read(manifest_dir.join("third-party/box2d/include/box2d/box2d.h")).unwrap();
        let mut files = BTreeMap::from([(packaged.to_owned(), effective)]);

        verify_packaged_effective_headers(&files, &effective_headers).unwrap();
        files.insert(packaged.to_owned(), vendored);
        assert!(verify_packaged_effective_headers(&files, &effective_headers).is_err());
    }

    #[test]
    fn trusted_root_paths_are_resolved_from_the_repository_root() {
        let repository = TempDir::new().unwrap();
        let alternate_cwd = TempDir::new().unwrap();
        let default = repository.path().join(DEFAULT_TRUSTED_ROOT_RELATIVE_PATH);
        let relative = Path::new("fixtures/trusted-root.json");
        let absolute = alternate_cwd.path().join("trusted-root.json");

        assert_eq!(resolve_trusted_root(repository.path(), None), default);
        assert_ne!(
            resolve_trusted_root(repository.path(), None),
            alternate_cwd
                .path()
                .join(DEFAULT_TRUSTED_ROOT_RELATIVE_PATH)
        );
        assert_eq!(
            resolve_trusted_root(repository.path(), Some(relative)),
            repository.path().join(relative)
        );
        assert_eq!(
            resolve_trusted_root(repository.path(), Some(&absolute)),
            absolute
        );
    }

    #[test]
    fn trusted_root_digest_is_pinned_and_not_caller_selected() {
        assert_eq!(SIGSTORE_TRUSTED_ROOT_SHA256.len(), 64);
        assert!(
            SIGSTORE_TRUSTED_ROOT_SHA256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );
    }
}
