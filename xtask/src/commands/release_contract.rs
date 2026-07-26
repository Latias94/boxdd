use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsStr,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use yaml_serde::{Mapping as YamlMapping, Value as YamlValue};

use crate::emscripten_sdk::{SDK_CONTRACT_RELATIVE_PATH, SdkContract};
use crate::provenance_policy::{
    self, COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW, SIGSTORE_TRUSTED_ROOT_SHA256,
};
use crate::provider_archive::{ArchiveExpectation, verify_provider_archive};
use crate::provider_manifest::{
    self, ArtifactExpectation, ArtifactIdentityExpectation, ArtifactManifest,
};
use crate::source_overlay::effective_source_identity;
use crate::{Error, Result};

use super::support::run_command;

const UPSTREAM_SHA: &str = "56edae79f2949d86142b03450d5d60f63bcf5a6f";
const CHECKSUMS_FILE: &str = "SHA256SUMS";
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4_096;
const TAR_BLOCK_BYTES: u64 = 512;
const MAX_ARCHIVE_STREAM_BYTES: u64 = MAX_ARCHIVE_TOTAL_BYTES
    + (MAX_ARCHIVE_ENTRIES as u64 * TAR_BLOCK_BYTES * 2)
    + (TAR_BLOCK_BYTES * 2);
const QUALIFICATION_TOOLCHAINS: &[&str] = &["1.95.0", "1.97.1"];
const QUALIFICATION_PRECISIONS: &[&str] = &["single", "double"];
const SYSTEM_QUALIFICATION_COMMAND: &str = "cargo +${{ matrix.toolchain }} run --locked -p xtask -- qualify-native-provider --provider system --toolchain ${{ matrix.toolchain }} --precision ${{ matrix.precision }} --target x86_64-unknown-linux-gnu --crt none --artifacts \"${{ runner.temp }}/boxdd-system-artifact\"";
const PREBUILT_QUALIFICATION_COMMAND: &str = "cargo +${{ matrix.toolchain }} run --locked -p xtask -- qualify-native-provider --provider prebuilt --toolchain ${{ matrix.toolchain }} --precision ${{ matrix.precision }} --target ${{ matrix.platform.target }} --crt ${{ matrix.platform.crt }} --artifacts \"${{ runner.temp }}/release-inputs\" --cosign cosign";
const RUST_TOOLCHAIN_ACTION: &str =
    "dtolnay/rust-toolchain@2c7215f132e9ebf062739d9130488b56d53c060c";
const CHECKOUT_ACTION: &str = "actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0";
const RUST_CACHE_ACTION: &str = "Swatinem/rust-cache@c19371144df3bb44fab255c43d04cbc2ab54d1c4";
const SETUP_NODE_ACTION: &str = "actions/setup-node@395ad3262231945c25e8478fd5baf05154b1d79f";
const CONFIGURE_PAGES_ACTION: &str =
    "actions/configure-pages@45bfe0192ca1faeb007ade9deae92b16b8254a0d";
const UPLOAD_PAGES_ACTION: &str =
    "actions/upload-pages-artifact@fc324d3547104276b827a68afc52ff2a11cc49c9";
const DEPLOY_PAGES_ACTION: &str = "actions/deploy-pages@cd2ce8fcbc39b97be8ca5fce6e763baed58fa128";
const UPLOAD_ARTIFACT_ACTION: &str =
    "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";
const DOWNLOAD_ARTIFACT_ACTION: &str =
    "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c";
const COSIGN_INSTALLER_ACTION: &str =
    "sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6";
#[cfg(any(target_os = "linux", target_os = "macos"))]
const GITHUB_SYSTEM_GIT: &str = "/usr/bin/git";
#[cfg(target_os = "windows")]
const GITHUB_SYSTEM_GIT: &str = r"C:\Program Files\Git\cmd\git.exe";
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const GITHUB_SYSTEM_GIT: &str = "";

struct BoundedReader<R> {
    inner: R,
    remaining: u64,
}

impl<R> BoundedReader<R> {
    const fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}

impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut extra = [0_u8; 1];
            return match self.inner.read(&mut extra)? {
                0 => Ok(0),
                _ => Err(io::Error::other(
                    "decompressed release archive exceeds the stream limit",
                )),
            };
        }
        let maximum = usize::try_from(self.remaining.min(buffer.len() as u64))
            .expect("bounded read length always fits usize");
        let read = self.inner.read(&mut buffer[..maximum])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

#[derive(Clone, Copy)]
enum WorkflowStepKind {
    Action(&'static str),
    Run,
}

#[derive(Clone, Copy)]
struct WorkflowStepPolicy {
    name: &'static str,
    kind: WorkflowStepKind,
    keys: &'static [&'static str],
    digest: &'static str,
}

const BUILD_PREBUILT_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Checkout immutable tag commit",
        kind: WorkflowStepKind::Action(CHECKOUT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "27bb57383d9e5b84526b6d4b4598fbdc81903fba955372ca3763918a28ccbb37",
    },
    WorkflowStepPolicy {
        name: "Install Rust toolchain",
        kind: WorkflowStepKind::Action(RUST_TOOLCHAIN_ACTION),
        keys: &["name", "uses", "with"],
        digest: "b8e18f7ff94e2538e2a44d8397592a9887ff335f14453cca62378b753aa6efea",
    },
    WorkflowStepPolicy {
        name: "Cache Rust dependencies",
        kind: WorkflowStepKind::Action(RUST_CACHE_ACTION),
        keys: &["name", "uses", "with"],
        digest: "04585d669708eab2f1ab3d207112d84147dd5bb39f9a139bb5c990b23915ce0e",
    },
    WorkflowStepPolicy {
        name: "Install build tools (Linux)",
        kind: WorkflowStepKind::Run,
        keys: &["name", "if", "run"],
        digest: "02c45e37796c3e35ade9ee5b1acae49e61e734569539056cc382a1790b133afc",
    },
    WorkflowStepPolicy {
        name: "Select static CRT (Windows MT)",
        kind: WorkflowStepKind::Run,
        keys: &["name", "if", "shell", "run"],
        digest: "19f1701716f219cd714c61bae4da6aefbf77c2f8f5b90835bdca6156bcf2111f",
    },
    WorkflowStepPolicy {
        name: "Build and package (single precision)",
        kind: WorkflowStepKind::Run,
        keys: &["name", "if", "run"],
        digest: "69aaf7064607d6d62c6892020bc58d966af44107bdcc0a1edd3c42ea8b079d0c",
    },
    WorkflowStepPolicy {
        name: "Build and package (double precision)",
        kind: WorkflowStepKind::Run,
        keys: &["name", "if", "run"],
        digest: "2d3dd2f10cf36ae60800bfb9883e31e9021bc6641c2af31dfb7e12658be68ef4",
    },
    WorkflowStepPolicy {
        name: "Stage one immutable release input",
        kind: WorkflowStepKind::Run,
        keys: &["name", "shell", "run"],
        digest: "4377c03d79f9c49f46ff30b1ff0ecdabb68c8022243feb55eccdb659830bc94e",
    },
    WorkflowStepPolicy {
        name: "Upload unprivileged release input",
        kind: WorkflowStepKind::Action(UPLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "263389a75d385e8ca3814a42815b05b956b086a00c9b750530329442a2ad4a42",
    },
];

const AGGREGATE_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Checkout immutable tag commit",
        kind: WorkflowStepKind::Action(CHECKOUT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "27bb57383d9e5b84526b6d4b4598fbdc81903fba955372ca3763918a28ccbb37",
    },
    WorkflowStepPolicy {
        name: "Install Rust toolchain",
        kind: WorkflowStepKind::Action(RUST_TOOLCHAIN_ACTION),
        keys: &["name", "uses", "with"],
        digest: "b1973be64e6f706ab0d2058552c5734e809dbf04c3040971d66222a314747587",
    },
    WorkflowStepPolicy {
        name: "Download exact run inputs",
        kind: WorkflowStepKind::Action(DOWNLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "9c8040bd529b22ca288a1a27d213544e1ae97175392d0f722eddb52ca2d06d64",
    },
    WorkflowStepPolicy {
        name: "Validate exact contents and export canonical provenance manifests",
        kind: WorkflowStepKind::Run,
        keys: &["name", "run"],
        digest: "fd1e90eccc426dbc674ef44b4bfc858f8b76ac88c694e1085abc3db450a2d2f5",
    },
    WorkflowStepPolicy {
        name: "Upload validated attestation input",
        kind: WorkflowStepKind::Action(UPLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "a5105393d7466266455c4c08aaffb0ff6d4c04a0e27a5f7d9b66e9460f090aec",
    },
];

const ATTEST_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Download validated attestation input",
        kind: WorkflowStepKind::Action(DOWNLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "f45b37c56ac03d2265eddc23446655aa0e97a64cf98538da8cbac88bff632b64",
    },
    WorkflowStepPolicy {
        name: "Install exact Cosign",
        kind: WorkflowStepKind::Action(COSIGN_INSTALLER_ACTION),
        keys: &["name", "uses", "with"],
        digest: "2dce8d19eaac67464cdfe885beaf6845847e7080b2c2b2c0efd29ee63ceb7126",
    },
    WorkflowStepPolicy {
        name: "Sign and immediately verify every validated payload",
        kind: WorkflowStepKind::Run,
        keys: &["name", "shell", "run"],
        digest: "4f178886b05d790199301919819bed76c611275a943ddf9c5c56ae46c2cf50bf",
    },
    WorkflowStepPolicy {
        name: "Upload signed release inputs",
        kind: WorkflowStepKind::Action(UPLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "241f5a26edb9bb583ec40477994cb29a77a590c4709841b185ad40943b9bed21",
    },
];

const VERIFY_SIGNED_RELEASE_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Checkout immutable tag commit",
        kind: WorkflowStepKind::Action(CHECKOUT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "27bb57383d9e5b84526b6d4b4598fbdc81903fba955372ca3763918a28ccbb37",
    },
    WorkflowStepPolicy {
        name: "Install Rust toolchain",
        kind: WorkflowStepKind::Action(RUST_TOOLCHAIN_ACTION),
        keys: &["name", "uses", "with"],
        digest: "b1973be64e6f706ab0d2058552c5734e809dbf04c3040971d66222a314747587",
    },
    WorkflowStepPolicy {
        name: "Install exact Cosign",
        kind: WorkflowStepKind::Action(COSIGN_INSTALLER_ACTION),
        keys: &["name", "uses", "with"],
        digest: "2dce8d19eaac67464cdfe885beaf6845847e7080b2c2b2c0efd29ee63ceb7126",
    },
    WorkflowStepPolicy {
        name: "Download signed aggregate",
        kind: WorkflowStepKind::Action(DOWNLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "f3161116c83afd15c3c55bcc0d58afedf81b64effc0f8d2dd7c3d5e3bf5e982d",
    },
    WorkflowStepPolicy {
        name: "Reverify exact signed release contract",
        kind: WorkflowStepKind::Run,
        keys: &["name", "run"],
        digest: "745195d6d42de984c1de94269e7d5675411f46d1b9b3a0ec50283d13e9323569",
    },
];

const QUALIFY_PREBUILT_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Checkout immutable tag commit",
        kind: WorkflowStepKind::Action(CHECKOUT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "27bb57383d9e5b84526b6d4b4598fbdc81903fba955372ca3763918a28ccbb37",
    },
    WorkflowStepPolicy {
        name: "Install Rust toolchain",
        kind: WorkflowStepKind::Action(RUST_TOOLCHAIN_ACTION),
        keys: &["name", "uses", "with"],
        digest: "babd2b6f21c08dad41aae3cff45dd468ccfc4e5e2114bdfea0f0fffeff4e182b",
    },
    WorkflowStepPolicy {
        name: "Install exact Cosign",
        kind: WorkflowStepKind::Action(COSIGN_INSTALLER_ACTION),
        keys: &["name", "uses", "with"],
        digest: "2dce8d19eaac67464cdfe885beaf6845847e7080b2c2b2c0efd29ee63ceb7126",
    },
    WorkflowStepPolicy {
        name: "Download signed aggregate",
        kind: WorkflowStepKind::Action(DOWNLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "f3161116c83afd15c3c55bcc0d58afedf81b64effc0f8d2dd7c3d5e3bf5e982d",
    },
    WorkflowStepPolicy {
        name: "Consume through the authenticated prebuilt provider",
        kind: WorkflowStepKind::Run,
        keys: &["name", "run"],
        digest: "7f2f0bf67701c90b38487629d7356700646c6e739fbfb8f77c3a319b22425963",
    },
];

const PUBLISH_DRAFT_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Download qualified signed aggregate",
        kind: WorkflowStepKind::Action(DOWNLOAD_ARTIFACT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "63f6d81dd8464728322231b9b38708c74af83b1b0633664141d9835ececce719",
    },
    WorkflowStepPolicy {
        name: "Create protected draft release",
        kind: WorkflowStepKind::Run,
        keys: &["name", "shell", "env", "run"],
        digest: "b0020ed4b6626bda28b85bc34642d844aeba85b037312cf1db4098403c397aa0",
    },
];

const SYSTEM_PROVIDER_STEPS: &[WorkflowStepPolicy] = &[
    WorkflowStepPolicy {
        name: "Checkout",
        kind: WorkflowStepKind::Action(CHECKOUT_ACTION),
        keys: &["name", "uses", "with"],
        digest: "b8aae87cc187dff7701257b398903e9dfa31e43c5b81fbbf8f7a312850125c84",
    },
    WorkflowStepPolicy {
        name: "Install Rust",
        kind: WorkflowStepKind::Action(RUST_TOOLCHAIN_ACTION),
        keys: &["name", "uses", "with"],
        digest: "def9eb57440baef8fb508240a99d49601b27acdfb0bac7d7ebb3cf94ae94bdbd",
    },
    WorkflowStepPolicy {
        name: "Prepare caller-attested system artifact",
        kind: WorkflowStepKind::Run,
        keys: &["name", "shell", "run"],
        digest: "5f696fbb23f3e3b61f17031195584cb613654ff3032936dcfd8e1b990d1e4f9b",
    },
    WorkflowStepPolicy {
        name: "Qualify the freshly packaged crate against the system artifact",
        kind: WorkflowStepKind::Run,
        keys: &["name", "run"],
        digest: "293c3e13d912d05ebb1601b25ee13ff77311f4c1229a66a4b2171b61ffa1a6b5",
    },
];

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

#[derive(Clone, Debug, Eq, PartialEq)]
enum Mode {
    Check,
    CheckContent,
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

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    let identity = validate_repository_identity(root, &options)?;
    validate_release_workflow(root, &identity.commit)?;
    validate_ci_workflow(root)?;
    validate_pages_workflow(root)?;
    validate_audit_policy(root)?;
    validate_semver_intent(&identity.version)?;

    match (&options.mode, &options.artifacts) {
        (Mode::WriteChecksums, Some(artifacts)) => {
            write_aggregate_checksums(artifacts, &identity.version)
        }
        (Mode::WriteChecksums, None) => Err(Error::message(
            "release-contract --write-checksums requires --artifacts",
        )),
        (Mode::CheckContent, None) => Err(Error::message(
            "release-contract --check-content requires --artifacts",
        )),
        (Mode::CheckContent, Some(artifacts)) => {
            validate_release_context(&options, &identity)?;
            validate_artifacts(root, artifacts, &options, &identity, false)
        }
        (Mode::Check, Some(artifacts)) => {
            validate_release_context(&options, &identity)?;
            validate_artifacts(root, artifacts, &options, &identity, true)
        }
        (Mode::Check, None) => {
            println!(
                "release contract prepared for {} at {} (Box2D {})",
                identity.tag, identity.commit, identity.upstream_sha
            );
            Ok(())
        }
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
                "release-contract requires --check, --check-content, or --write-checksums",
            ));
        }
        if options.payloads.is_some() && options.mode != Mode::CheckContent {
            return Err(Error::message(
                "release-contract --payloads is valid only with --check-content",
            ));
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

fn validate_repository_identity(root: &Path, options: &Options) -> Result<ReleaseIdentity> {
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
        let source = fs::read_to_string(root.join(manifest))
            .map_err(|error| Error::io(root.join(manifest), error))?;
        if !source.contains("version.workspace = true") {
            return Err(Error::message(format!(
                "{manifest} must inherit the release version from the workspace"
            )));
        }
    }
    if version != "0.6.0" {
        return Err(Error::message(format!(
            "release contract is pinned to version 0.6.0; workspace reports {version}"
        )));
    }

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
        .unwrap_or_else(|| format!("v{version}"));
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
    if upstream_sha != UPSTREAM_SHA {
        return Err(Error::message(format!(
            "release upstream SHA {upstream_sha} does not match pinned {UPSTREAM_SHA}"
        )));
    }
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

    validate_changelog(root, &version, options.artifacts.is_some())?;
    Ok(ReleaseIdentity {
        version,
        tag,
        commit,
        upstream_sha,
    })
}

fn validate_tag(tag: &str, version: &str) -> Result<()> {
    if tag == format!("v{version}") || tag == format!("boxdd-sys-v{version}") {
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

fn validate_changelog(root: &Path, version: &str, release_artifacts_present: bool) -> Result<()> {
    let path = root.join("CHANGELOG.md");
    let changelog = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let release_heading = format!("## [{version}]");
    let heading = if changelog.contains(&release_heading) {
        release_heading
    } else if !release_artifacts_present && changelog.contains("## [Unreleased]") {
        "## [Unreleased]".to_owned()
    } else {
        return Err(Error::message(format!(
            "CHANGELOG.md is stale: protected release requires heading {release_heading}"
        )));
    };
    let start = changelog
        .find(&heading)
        .expect("selected changelog heading must exist");
    let section = &changelog[start..];
    let end = section[heading.len()..]
        .find("\n## [")
        .map(|offset| heading.len() + offset)
        .unwrap_or(section.len());
    let section = &section[..end];
    if !section.contains("### Breaking")
        || !section.contains("### Added")
        || !section.contains("### Migration")
    {
        return Err(Error::message(
            "0.6.0 changelog must contain Breaking, Added, and Migration sections",
        ));
    }
    Ok(())
}

fn validate_semver_intent(version: &str) -> Result<()> {
    let previous = Version::parse("0.5.0")?;
    let current = Version::parse(version)?;
    if current.allows_breaking_from(previous) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "intentional public break requires a minor or major version bump; 0.5.0 -> {version} is insufficient"
        )))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    fn parse(value: &str) -> Result<Self> {
        let numbers = value
            .split('.')
            .map(|part| part.parse::<u64>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| Error::message(format!("invalid release version {value:?}")))?;
        if numbers.len() != 3 {
            return Err(Error::message(format!("invalid release version {value:?}")));
        }
        Ok(Self {
            major: numbers[0],
            minor: numbers[1],
            patch: numbers[2],
        })
    }

    const fn allows_breaking_from(self, previous: Self) -> bool {
        self.major > previous.major
            || (self.major == 0 && previous.major == 0 && self.minor > previous.minor)
    }
}

fn validate_release_context(options: &Options, identity: &ReleaseIdentity) -> Result<()> {
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
    Ok(())
}

fn expected_artifacts(version: &str) -> Vec<ArtifactSpec> {
    let mut artifacts = Vec::new();
    for platform in PLATFORMS {
        for precision in ["single", "double"] {
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

fn write_aggregate_checksums(root: &Path, version: &str) -> Result<()> {
    let files = collect_files(root)?;
    let expected = expected_artifacts(version);
    let archives = map_expected_archives(&files, &expected)?;
    let mut rendered = String::new();
    for spec in expected {
        let path = &archives[&spec.archive];
        let digest = sha256_file(path)?;
        let sidecar = path.with_file_name(format!("{}.sha256", spec.archive));
        let sidecar_source = format!("{digest}  {}\n", spec.archive);
        fs::write(&sidecar, sidecar_source).map_err(|error| Error::io(&sidecar, error))?;
        rendered.push_str(&format!("{digest}  {}\n", spec.archive));
    }
    let destination = root.join(CHECKSUMS_FILE);
    fs::write(&destination, rendered).map_err(|error| Error::io(destination, error))
}

struct ValidatedArchive {
    _directory: TempDir,
    provenance_payload: PathBuf,
}

fn validate_artifacts(
    repository_root: &Path,
    artifact_root: &Path,
    options: &Options,
    identity: &ReleaseIdentity,
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
    let trusted_root = options
        .trusted_root
        .clone()
        .unwrap_or_else(|| PathBuf::from("boxdd-sys/security/sigstore/trusted_root.json"));
    let trusted_root = if trusted_root.is_absolute() {
        trusted_root
    } else {
        env::current_dir()
            .map_err(|error| Error::io("read current directory", error))?
            .join(trusted_root)
    };
    require_trusted_root(&trusted_root)?;
    if require_signatures {
        verify_cosign_version(&options.cosign)?;
    }
    let payload_root = options
        .payloads
        .as_deref()
        .map(prepare_empty_payload_directory)
        .transpose()?;

    let files = collect_files(&canonical_root)?;
    let expected = expected_artifacts(&identity.version);
    let archives = map_expected_archives(&files, &expected)?;
    let run_id = option_or_env(&options.run_id, "GITHUB_RUN_ID")?
        .ok_or_else(|| Error::message("release run ID is missing"))?;
    let run_attempt = option_or_env(&options.run_attempt, "GITHUB_RUN_ATTEMPT")?
        .ok_or_else(|| Error::message("release run attempt is missing"))?;
    let mut allowed = BTreeSet::new();
    let mut aggregate = String::new();

    for spec in &expected {
        let archive = &archives[&spec.archive];
        let expected_parent = format!(
            "prebuilt-input-{}-{}-{}-{run_id}-{run_attempt}-{}",
            spec.target, spec.precision, spec.crt, identity.commit
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
        let digest = sha256_file(archive)?;
        aggregate.push_str(&format!("{digest}  {}\n", spec.archive));
        let checksum = archive.with_file_name(format!("{}.sha256", spec.archive));
        let bundle = archive.with_file_name(format!("{}.sigstore.json", spec.archive));
        let expected_checksum = format!("{digest}  {}\n", spec.archive);
        let actual_checksum =
            fs::read_to_string(&checksum).map_err(|error| Error::io(&checksum, error))?;
        if actual_checksum != expected_checksum {
            return Err(Error::message(format!(
                "non-canonical or incorrect checksum sidecar {}",
                checksum.display()
            )));
        }
        let validated = validate_archive_manifest(repository_root, archive, spec, identity)?;
        if let Some(payload_root) = &payload_root {
            let destination = payload_root.join(format!("{}.manifest", spec.archive));
            fs::copy(&validated.provenance_payload, &destination)
                .map_err(|error| Error::io(&destination, error))?;
        }
        allowed.extend([archive.clone(), checksum]);
        if require_signatures {
            if !bundle.is_file() {
                return Err(Error::message(format!(
                    "artifact {} is missing Sigstore bundle {}",
                    spec.archive,
                    bundle.display()
                )));
            }
            verify_sigstore(
                &options.cosign,
                &validated.provenance_payload,
                &bundle,
                &trusted_root,
                identity,
            )?;
            allowed.insert(bundle);
        }
    }

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

fn map_expected_archives(
    files: &[PathBuf],
    expected: &[ArtifactSpec],
) -> Result<BTreeMap<String, PathBuf>> {
    let expected_names = expected
        .iter()
        .map(|spec| spec.archive.as_str())
        .collect::<BTreeSet<_>>();
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

fn validate_archive_manifest(
    repository_root: &Path,
    archive_path: &Path,
    spec: &ArtifactSpec,
    identity: &ReleaseIdentity,
) -> Result<ValidatedArchive> {
    let expected_paths = expected_archive_paths(repository_root, spec)?;
    let directory = tempfile::Builder::new()
        .prefix("boxdd-release-archive-")
        .tempdir()
        .map_err(|error| Error::io("create release archive inspection directory", error))?;
    let files = read_release_archive(archive_path, &expected_paths, directory.path())?;

    verify_inner_checksums(&files, archive_path)?;
    verify_repository_owned_files(repository_root, &files, spec)?;
    let effective_source =
        effective_source_identity(&repository_root.join("boxdd-sys")).map_err(|error| {
            Error::message(format!(
                "cannot recompute the repository effective-source identity: {error}"
            ))
        })?;
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
    let native_identity = verify_provider_archive(
        &library,
        &ArchiveExpectation {
            target: spec.target,
            required_symbols: provider_manifest::REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &manifest.private_abi_hash,
            snapshot_layout_hash,
        },
    )
    .map_err(|error| Error::message(format!("provider archive proof failed: {error}")))?;
    let adapter_source_sha256 =
        provider_manifest::adapter_source_sha256(&repository_root.join("boxdd-sys"))
            .map_err(Error::message)?;
    let header = repository_root.join("boxdd-sys/third-party/box2d/include/box2d/box2d.h");
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
                private_abi_hash: &native_identity.private_abi_hash,
                snapshot_layout_hash: native_identity.snapshot_layout_hash,
            },
            header_path: &header,
            bindings_path: &bindings,
        },
    )
    .map_err(|error| Error::message(format!("provider artifact validation failed: {error}")))?;
    if native_identity.archive_sha256 != verified.archive_sha256 {
        return Err(Error::message(
            "packaged provider archive changed between structural and manifest verification",
        ));
    }
    if verified.archive_path
        != fs::canonicalize(&library).map_err(|error| Error::io(&library, error))?
    {
        return Err(Error::message(
            "provider manifest did not resolve to the canonical packaged library",
        ));
    }

    let provenance_payload = directory.path().join("manifest.toml");
    Ok(ValidatedArchive {
        _directory: directory,
        provenance_payload,
    })
}

fn read_release_archive(
    archive_path: &Path,
    expected_paths: &BTreeSet<String>,
    destination_root: &Path,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let file = fs::File::open(archive_path).map_err(|error| Error::io(archive_path, error))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(BoundedReader::new(decoder, MAX_ARCHIVE_STREAM_BYTES));
    let entries = archive
        .entries()
        .map_err(|error| Error::message(format!("read {}: {error}", archive_path.display())))?;
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
    Ok(files)
}

fn expected_archive_paths(repository_root: &Path, spec: &ArtifactSpec) -> Result<BTreeSet<String>> {
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
    collect_expected_headers(
        &repository_root.join("boxdd-sys/third-party/box2d/include/box2d"),
        &repository_root.join("boxdd-sys/third-party/box2d/include/box2d"),
        &mut paths,
    )?;
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

fn collect_expected_headers(
    root: &Path,
    directory: &Path,
    paths: &mut BTreeSet<String>,
) -> Result<()> {
    let entries = fs::read_dir(directory).map_err(|error| Error::io(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(directory, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| Error::io(entry.path(), error))?;
        if file_type.is_symlink() {
            return Err(Error::message(format!(
                "public header source cannot be a symlink: {}",
                entry.path().display()
            )));
        }
        if file_type.is_dir() {
            collect_expected_headers(root, &entry.path(), paths)?;
        } else if file_type.is_file()
            && entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("h"))
        {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| Error::message("public header escaped its source root"))?
                .to_str()
                .ok_or_else(|| Error::message("public header path is not UTF-8"))?
                .replace('\\', "/");
            paths.insert(format!("include/box2d/{relative}"));
        }
    }
    Ok(())
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
    if !entry.header().entry_type().is_file() {
        return Err(Error::message(format!(
            "{} contains a non-regular tar entry",
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
) -> Result<()> {
    let fixed = [
        (
            "metadata/effective-source.toml",
            "boxdd-sys/effective-source.toml",
        ),
        ("metadata/upstream.toml", "boxdd-sys/upstream.toml"),
        ("licenses/PROJECT-LICENSE-MIT", "LICENSE-MIT"),
        ("licenses/PROJECT-LICENSE-APACHE", "LICENSE-APACHE"),
        (
            "licenses/BOX2D-LICENSE",
            "boxdd-sys/third-party/box2d/LICENSE",
        ),
    ];
    for (packaged, source) in fixed {
        require_packaged_bytes(files, packaged, &repository_root.join(source))?;
    }
    let header_prefix = "include/box2d/";
    for packaged in files.keys().filter(|path| path.starts_with(header_prefix)) {
        require_packaged_bytes(
            files,
            packaged,
            &repository_root
                .join("boxdd-sys/third-party/box2d/include/box2d")
                .join(&packaged[header_prefix.len()..]),
        )?;
    }
    let bindings = expected_bindings_path(spec);
    require_packaged_bytes(
        files,
        &bindings,
        &repository_root
            .join("boxdd-sys/src")
            .join(Path::new(&bindings).file_name().expect("binding file name")),
    )
}

fn require_packaged_bytes(
    files: &BTreeMap<String, Vec<u8>>,
    packaged: &str,
    source: &Path,
) -> Result<()> {
    let expected = fs::read(source).map_err(|error| Error::io(source, error))?;
    if files.get(packaged).map(Vec::as_slice) != Some(expected.as_slice()) {
        return Err(Error::message(format!(
            "packaged {packaged} does not exactly match {}",
            source.display()
        )));
    }
    Ok(())
}

fn require_trusted_root(path: &Path) -> Result<()> {
    let digest = sha256_file(path)?;
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
    let output = Command::new(cosign)
        .arg("version")
        .output()
        .map_err(|error| Error::io(cosign, error))?;
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
    let mut command = Command::new(cosign);
    command.args(args);
    run_command(
        &mut command,
        &format!("verify Sigstore identity for {}", payload.display()),
    )
}

fn parse_workflow_yaml(source: &str, name: &str) -> Result<YamlValue> {
    yaml_serde::from_str(source)
        .map_err(|error| Error::message(format!("{name} is invalid YAML: {error}")))
}

fn yaml_mapping<'a>(value: &'a YamlValue, context: &str) -> Result<&'a YamlMapping> {
    value
        .as_mapping()
        .ok_or_else(|| Error::message(format!("{context} must be a YAML mapping")))
}

fn yaml_field<'a>(mapping: &'a YamlMapping, key: &str) -> Option<&'a YamlValue> {
    mapping.get(YamlValue::String(key.to_owned()))
}

fn required_yaml_field<'a>(
    mapping: &'a YamlMapping,
    key: &str,
    context: &str,
) -> Result<&'a YamlValue> {
    yaml_field(mapping, key)
        .ok_or_else(|| Error::message(format!("{context} is missing top-level {key:?}")))
}

fn workflow_jobs<'a>(workflow: &'a YamlMapping, name: &str) -> Result<&'a YamlMapping> {
    yaml_mapping(
        required_yaml_field(workflow, "jobs", name)?,
        &format!("{name} jobs"),
    )
}

fn workflow_job_mapping<'a>(
    jobs: &'a YamlMapping,
    job_name: &str,
    workflow_name: &str,
) -> Result<&'a YamlMapping> {
    yaml_mapping(
        required_yaml_field(jobs, job_name, &format!("{workflow_name} jobs"))?,
        &format!("{workflow_name} job {job_name:?}"),
    )
}

fn require_exact_string_field(
    mapping: &YamlMapping,
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    let actual = required_yaml_field(mapping, key, context)?
        .as_str()
        .ok_or_else(|| Error::message(format!("{context} {key:?} must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{context} {key:?} must be exactly {expected:?}; found {actual:?}"
        )))
    }
}

fn require_exact_bool_field(
    mapping: &YamlMapping,
    key: &str,
    expected: bool,
    context: &str,
) -> Result<()> {
    let actual = required_yaml_field(mapping, key, context)?
        .as_bool()
        .ok_or_else(|| Error::message(format!("{context} {key:?} must be a boolean")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{context} {key:?} must be exactly {expected}; found {actual}"
        )))
    }
}

fn require_exact_trimmed_string_field(
    mapping: &YamlMapping,
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    let actual = required_yaml_field(mapping, key, context)?
        .as_str()
        .ok_or_else(|| Error::message(format!("{context} {key:?} must be a string")))?;
    if actual.trim() == expected.trim() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{context} {key:?} does not match the reviewed command contract"
        )))
    }
}

fn require_absent_field(mapping: &YamlMapping, key: &str, context: &str) -> Result<()> {
    if yaml_field(mapping, key).is_none() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{context} must not declare top-level {key:?}"
        )))
    }
}

fn require_exact_permissions(
    mapping: &YamlMapping,
    context: &str,
    expected: &[(&str, &str)],
) -> Result<()> {
    let permissions = yaml_mapping(
        required_yaml_field(mapping, "permissions", context)?,
        &format!("{context} permissions"),
    )?;
    let actual = permissions
        .iter()
        .map(|(key, value)| {
            let key = key.as_str().ok_or_else(|| {
                Error::message(format!("{context} permission name must be a string"))
            })?;
            let value = value.as_str().ok_or_else(|| {
                Error::message(format!("{context} permission {key:?} must be a string"))
            })?;
            Ok((key, value))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected && permissions.len() == expected.len() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{context} permissions must be exactly {expected:?}; found {actual:?}"
        )))
    }
}

fn validate_release_security_job(
    job: &YamlMapping,
    name: &str,
    expected_needs: Option<&str>,
    expected_permissions: &[(&str, &str)],
    expected_environment: Option<&str>,
) -> Result<()> {
    let context = format!("release job {name:?}");
    require_exact_string_field(job, "if", "${{ github.ref_protected == true }}", &context)?;
    match expected_needs {
        Some(needs) => require_exact_string_field(job, "needs", needs, &context)?,
        None => require_absent_field(job, "needs", &context)?,
    }
    require_exact_permissions(job, &context, expected_permissions)?;
    match expected_environment {
        Some(environment) => require_exact_string_field(job, "environment", environment, &context)?,
        None => require_absent_field(job, "environment", &context)?,
    }
    require_absent_field(job, "continue-on-error", &context)
}

fn validate_release_trigger(workflow: &YamlMapping) -> Result<()> {
    let trigger = yaml_mapping(
        required_yaml_field(workflow, "on", "release workflow")?,
        "release workflow trigger",
    )?;
    require_exact_mapping_keys(trigger, &["push"], "release workflow trigger")?;
    let push = yaml_mapping(
        required_yaml_field(trigger, "push", "release workflow trigger")?,
        "release workflow push trigger",
    )?;
    require_exact_mapping_keys(push, &["tags"], "release workflow push trigger")?;
    let tags = required_yaml_field(push, "tags", "release workflow push trigger")?
        .as_sequence()
        .ok_or_else(|| Error::message("release workflow tag filters must be a sequence"))?;
    let actual = tags
        .iter()
        .map(|tag| {
            tag.as_str()
                .ok_or_else(|| Error::message("release workflow tag filters must be strings"))
        })
        .collect::<Result<Vec<_>>>()?;
    let expected = ["v*", "boxdd-sys-v*"];
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "release workflow tag filters must be exactly {expected:?}; found {actual:?}"
        )))
    }
}

fn job_executable_lines(job: &YamlMapping, name: &str) -> Result<Vec<String>> {
    let mut commands = Vec::new();
    for (index, step) in workflow_steps(job, name)?.iter().enumerate() {
        let context = format!("{name} step {index}");
        let step = yaml_mapping(step, &context)?;
        let Some(run) = yaml_field(step, "run") else {
            continue;
        };
        let run = run
            .as_str()
            .ok_or_else(|| Error::message(format!("{context} run command must be a string")))?;
        for line in run.lines() {
            let command = line
                .trim()
                .split_once(" #")
                .map_or_else(|| line.trim(), |(command, _)| command.trim());
            if !command.is_empty()
                && !command.starts_with('#')
                && !command.starts_with("echo ")
                && !command.starts_with("printf ")
            {
                commands.push(command.to_owned());
            }
        }
    }
    Ok(commands)
}

fn job_action_references<'a>(job: &'a YamlMapping, name: &str) -> Result<Vec<&'a str>> {
    workflow_steps(job, name)?
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            let context = format!("{name} step {index}");
            let step = match yaml_mapping(step, &context) {
                Ok(step) => step,
                Err(error) => return Some(Err(error)),
            };
            yaml_field(step, "uses").map(|uses| {
                uses.as_str().ok_or_else(|| {
                    Error::message(format!("{context} action reference must be a string"))
                })
            })
        })
        .collect()
}

fn require_job_command_fragment(commands: &[String], fragment: &str, message: &str) -> Result<()> {
    if commands.iter().any(|command| command.contains(fragment)) {
        Ok(())
    } else {
        Err(Error::message(message.to_owned()))
    }
}

fn require_action_input_fragment(
    job: &YamlMapping,
    name: &str,
    input: &str,
    fragment: &str,
    message: &str,
) -> Result<()> {
    for (index, step) in workflow_steps(job, name)?.iter().enumerate() {
        let context = format!("{name} step {index}");
        let step = yaml_mapping(step, &context)?;
        let Some(inputs) = yaml_field(step, "with") else {
            continue;
        };
        let inputs = yaml_mapping(inputs, &format!("{context} inputs"))?;
        if yaml_field(inputs, input)
            .and_then(YamlValue::as_str)
            .is_some_and(|value| value.contains(fragment))
        {
            return Ok(());
        }
    }
    Err(Error::message(message.to_owned()))
}

fn forbid_job_fragments(
    job: &YamlMapping,
    name: &str,
    forbidden: &[&str],
    message: &str,
) -> Result<()> {
    let commands = job_executable_lines(job, name)?;
    let actions = job_action_references(job, name)?;
    for fragment in forbidden {
        if commands.iter().any(|command| command.contains(fragment))
            || actions.iter().any(|action| action.contains(fragment))
        {
            return Err(Error::message(format!("{message} {fragment:?}")));
        }
    }
    Ok(())
}

fn validate_release_workflow(root: &Path, expected_commit: &str) -> Result<()> {
    let github_sha = match env::var("GITHUB_SHA") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            return Err(Error::message("GITHUB_SHA must be valid Unicode"));
        }
    };
    if let Some(github_sha) = &github_sha {
        require_matching_identity("workflow GITHUB_SHA", github_sha, expected_commit)?;
    }
    let source = read_release_workflow_source(root, github_sha.as_deref())?;
    validate_release_workflow_source(&source)
}

fn read_release_workflow_source(root: &Path, commit: Option<&str>) -> Result<String> {
    let path = root.join(PUBLISHER_WORKFLOW);
    let Some(commit) = commit else {
        return fs::read_to_string(&path).map_err(|error| Error::io(&path, error));
    };
    validate_git_sha("release workflow commit", commit)?;
    require_unflagged_workflow_index_entry(root)?;
    read_immutable_git_blob(root, commit, PUBLISHER_WORKFLOW)
}

fn require_unflagged_workflow_index_entry(root: &Path) -> Result<()> {
    let output = isolated_git_output(
        root,
        &["ls-files", "-v", "--full-name", "--", PUBLISHER_WORKFLOW],
        "inspect release workflow index flags",
    )?;
    let actual = String::from_utf8(output.stdout)
        .map_err(|_| Error::message("release workflow index entry must be valid UTF-8"))?;
    let expected = format!("H {PUBLISHER_WORKFLOW}\n");
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "release workflow index entry must be an ordinary tracked file without assume-unchanged or skip-worktree flags; found {:?}",
            actual.trim_end()
        )))
    }
}

fn read_immutable_git_blob(root: &Path, commit: &str, path: &str) -> Result<String> {
    let object = format!("{commit}:{path}");
    let output = isolated_git_output(
        root,
        &["--no-replace-objects", "cat-file", "blob", &object],
        "read immutable release workflow blob",
    )?;
    String::from_utf8(output.stdout)
        .map_err(|_| Error::message("release workflow Git blob must be valid UTF-8"))
}

fn isolated_git_output(root: &Path, args: &[&str], label: &str) -> Result<Output> {
    let mut command = release_git_command(true)?;
    remove_git_environment(&mut command);
    let output = command
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| Error::io(label, error))?;
    require_success(&output, label)?;
    Ok(output)
}

fn release_git_command(require_system_git: bool) -> Result<Command> {
    if !require_system_git {
        return Ok(Command::new("git"));
    }
    if GITHUB_SYSTEM_GIT.is_empty() {
        return Err(Error::message(
            "immutable release verification has no reviewed system Git path for this platform",
        ));
    }
    let path = Path::new(GITHUB_SYSTEM_GIT);
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "reviewed system Git must be a regular non-symlink file: {}",
            path.display()
        )));
    }
    Ok(Command::new(path))
}

fn github_execution_context() -> bool {
    env::var_os("GITHUB_SHA").is_some()
        || env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true")
}

fn remove_git_environment(command: &mut Command) {
    for (key, _) in env::vars_os() {
        if is_git_environment_key(&key) {
            command.env_remove(key);
        }
    }
}

fn is_git_environment_key(key: &OsStr) -> bool {
    key.to_string_lossy()
        .to_ascii_uppercase()
        .starts_with("GIT_")
}

fn validate_release_workflow_source(source: &str) -> Result<()> {
    let workflow = parse_workflow_yaml(source, "release workflow")?;
    let workflow_mapping = yaml_mapping(&workflow, "release workflow")?;
    require_exact_mapping_keys(
        workflow_mapping,
        &["name", "on", "permissions", "env", "jobs"],
        "release workflow",
    )?;
    require_exact_string_field(
        workflow_mapping,
        "name",
        "Build Prebuilt Binaries (boxdd-sys)",
        "release workflow",
    )?;
    require_exact_string_mapping_field(
        workflow_mapping,
        "env",
        &[("CARGO_TERM_COLOR", "always")],
        "release workflow",
    )?;
    let structured_jobs = workflow_jobs(workflow_mapping, "release workflow")?;
    require_exact_permissions(workflow_mapping, "release workflow", &[])?;
    validate_release_trigger(workflow_mapping)?;
    require_exact_mapping_keys(
        structured_jobs,
        &[
            "aggregate",
            "attest",
            "build-prebuilt",
            "publish-draft",
            "qualify-prebuilt",
            "verify-signed-release",
        ],
        "release workflow jobs",
    )?;
    let build_yaml = workflow_job_mapping(structured_jobs, "build-prebuilt", "release workflow")?;
    let aggregate_yaml = workflow_job_mapping(structured_jobs, "aggregate", "release workflow")?;
    let attest_yaml = workflow_job_mapping(structured_jobs, "attest", "release workflow")?;
    let signed_yaml =
        workflow_job_mapping(structured_jobs, "verify-signed-release", "release workflow")?;
    let qualify_yaml =
        workflow_job_mapping(structured_jobs, "qualify-prebuilt", "release workflow")?;
    let publish_yaml = workflow_job_mapping(structured_jobs, "publish-draft", "release workflow")?;

    validate_exact_workflow_job(
        build_yaml,
        "build-prebuilt",
        &[
            "name",
            "if",
            "runs-on",
            "permissions",
            "env",
            "strategy",
            "steps",
        ],
        "Build ${{ matrix.platform.target }} ${{ matrix.precision }} ${{ matrix.platform.crt }}",
        "${{ matrix.platform.os }}",
        BUILD_PREBUILT_STEPS,
    )?;
    require_exact_string_mapping_field(
        build_yaml,
        "env",
        &[
            ("BOXDD_SYS_PACKAGE_DIR", "${{ github.workspace }}/packages"),
            ("BOXDD_SYS_PACKAGE_CRT", "${{ matrix.platform.crt }}"),
            ("BOXDD_SYS_PACKAGE_SOURCE_COMMIT", "${{ github.sha }}"),
            ("BOXDD_SYS_PACKAGE_RELEASE_TAG", "${{ github.ref_name }}"),
            ("TARGET", "${{ matrix.platform.target }}"),
        ],
        "release build-prebuilt job",
    )?;
    validate_exact_workflow_job(
        aggregate_yaml,
        "aggregate",
        &["name", "if", "needs", "runs-on", "permissions", "steps"],
        "Validate release contents before signing",
        "ubuntu-latest",
        AGGREGATE_STEPS,
    )?;
    validate_exact_workflow_job(
        attest_yaml,
        "attest",
        &[
            "name",
            "if",
            "needs",
            "runs-on",
            "permissions",
            "environment",
            "steps",
        ],
        "Sign validated provenance manifests",
        "ubuntu-latest",
        ATTEST_STEPS,
    )?;
    validate_exact_workflow_job(
        signed_yaml,
        "verify-signed-release",
        &["name", "if", "needs", "runs-on", "permissions", "steps"],
        "Reverify signed aggregate",
        "ubuntu-latest",
        VERIFY_SIGNED_RELEASE_STEPS,
    )?;
    validate_exact_workflow_job(
        qualify_yaml,
        "qualify-prebuilt",
        &[
            "name",
            "if",
            "needs",
            "runs-on",
            "permissions",
            "strategy",
            "steps",
        ],
        "Qualify ${{ matrix.platform.target }} ${{ matrix.precision }} ${{ matrix.platform.crt }} Rust ${{ matrix.toolchain }}",
        "${{ matrix.platform.os }}",
        QUALIFY_PREBUILT_STEPS,
    )?;
    validate_exact_workflow_job(
        publish_yaml,
        "publish-draft",
        &["name", "if", "needs", "runs-on", "permissions", "steps"],
        "Publish protected draft release",
        "ubuntu-latest",
        PUBLISH_DRAFT_STEPS,
    )?;

    validate_release_security_job(
        build_yaml,
        "build-prebuilt",
        None,
        &[("contents", "read")],
        None,
    )?;
    validate_release_security_job(
        aggregate_yaml,
        "aggregate",
        Some("build-prebuilt"),
        &[("contents", "read")],
        None,
    )?;
    validate_release_security_job(
        attest_yaml,
        "attest",
        Some("aggregate"),
        &[("contents", "read"), ("id-token", "write")],
        Some("release"),
    )?;
    validate_release_security_job(
        signed_yaml,
        "verify-signed-release",
        Some("attest"),
        &[("contents", "read")],
        None,
    )?;
    validate_release_security_job(
        qualify_yaml,
        "qualify-prebuilt",
        Some("verify-signed-release"),
        &[("contents", "read")],
        None,
    )?;
    validate_release_security_job(
        publish_yaml,
        "publish-draft",
        Some("qualify-prebuilt"),
        &[("contents", "write")],
        None,
    )?;
    validate_release_matrix(build_yaml, "build-prebuilt", false)?;
    validate_release_matrix(qualify_yaml, "qualify-prebuilt", true)?;
    let aggregate_commands = job_executable_lines(aggregate_yaml, "aggregate")?;
    require_job_command_fragment(
        &aggregate_commands,
        "release-contract --check-content",
        "aggregate must validate archive contents before signing",
    )?;
    require_job_command_fragment(
        &aggregate_commands,
        "--payloads \"$payloads\"",
        "aggregate must export canonical provenance manifests",
    )?;
    require_action_input_fragment(
        aggregate_yaml,
        "aggregate",
        "name",
        "github.run_attempt",
        "aggregate inputs must be isolated per workflow attempt",
    )?;

    forbid_job_fragments(
        attest_yaml,
        "attest",
        &["actions/checkout", "rust-toolchain", "cargo ", "gh release"],
        "OIDC-only attest job must not contain",
    )?;
    let attest_commands = job_executable_lines(attest_yaml, "attest")?;
    for required in [
        "cosign sign-blob",
        "cosign verify-blob",
        ".manifest",
        "--certificate-github-workflow-trigger push",
        "--certificate-github-workflow-name \"Build Prebuilt Binaries (boxdd-sys)\"",
    ] {
        require_job_command_fragment(
            &attest_commands,
            required,
            "attest job is missing strict Cosign identity binding",
        )?;
    }
    if attest_commands
        .iter()
        .any(|command| command.contains(".link"))
    {
        return Err(Error::message(
            "attest must sign canonical provenance manifests, not bare link payloads",
        ));
    }

    let signed_commands = job_executable_lines(signed_yaml, "verify-signed-release")?;
    require_job_command_fragment(
        &signed_commands,
        "release-contract --check --artifacts",
        "signed aggregate must be revalidated",
    )?;
    validate_matrix_toolchain_install(
        qualify_yaml,
        "prebuilt qualification",
        Some("${{ matrix.platform.target }}"),
    )?;
    validate_exact_qualification_job(
        qualify_yaml,
        "prebuilt qualification",
        PREBUILT_QUALIFICATION_COMMAND,
        Some("${{ github.ref_protected == true }}"),
        true,
    )?;
    let publish_commands = job_executable_lines(publish_yaml, "publish-draft")?;
    require_job_command_fragment(
        &publish_commands,
        "gh release create",
        "publishing must create the draft release",
    )?;
    require_job_command_fragment(
        &publish_commands,
        "--draft",
        "publishing must remain a draft",
    )?;
    for required in [
        "test \"${GITHUB_REF_TYPE}\" = \"tag\"",
        "test \"${GITHUB_REF}\" = \"refs/tags/${GITHUB_REF_NAME}\"",
        "git/ref/tags/${tag_name_uri}",
        "git/tags/${object_sha}",
        "test \"${object_type}\" = \"commit\"",
        "test \"${object_sha}\" = \"${GITHUB_SHA}\"",
        "--verify-tag",
    ] {
        require_job_command_fragment(
            &publish_commands,
            required,
            "publishing must bind the protected tag to the immutable workflow commit",
        )?;
    }
    let tag_identity_position = publish_commands
        .iter()
        .position(|command| command.contains("test \"${object_sha}\" = \"${GITHUB_SHA}\""))
        .ok_or_else(|| Error::message("publish tag identity check is missing"))?;
    let release_position = publish_commands
        .iter()
        .position(|command| command.contains("gh release create"))
        .ok_or_else(|| Error::message("publish release command is missing"))?;
    if tag_identity_position > release_position {
        return Err(Error::message(
            "publish must verify the protected tag before creating the release",
        ));
    }
    forbid_job_fragments(
        publish_yaml,
        "publish-draft",
        &[
            "actions/checkout",
            "rust-toolchain",
            "cargo ",
            "cosign sign-blob",
        ],
        "publish job must not build or sign through",
    )
}

fn validate_release_matrix(job: &YamlMapping, name: &str, require_toolchains: bool) -> Result<()> {
    require_exact_string_field(
        job,
        "runs-on",
        "${{ matrix.platform.os }}",
        &format!("{name} job"),
    )?;
    validate_exact_strategy(job, name)?;
    let matrix = job_matrix(job, name)?;
    let expected_axes = if require_toolchains {
        &["platform", "precision", "toolchain"][..]
    } else {
        &["platform", "precision"][..]
    };
    require_exact_mapping_keys(matrix, expected_axes, &format!("{name} matrix"))?;
    validate_matrix_axis(matrix, name, "precision", QUALIFICATION_PRECISIONS)?;
    if require_toolchains {
        validate_matrix_axis(matrix, name, "toolchain", QUALIFICATION_TOOLCHAINS)?;
    }
    validate_platform_matrix(matrix, name)
}

fn validate_system_provider_matrix(job: &YamlMapping) -> Result<()> {
    let name = "system provider";
    require_exact_string_field(job, "runs-on", "ubuntu-latest", "CI system provider job")?;
    validate_exact_strategy(job, name)?;
    let matrix = job_matrix(job, name)?;
    require_exact_mapping_keys(
        matrix,
        &["precision", "toolchain"],
        "CI system provider matrix",
    )?;
    validate_matrix_axis(matrix, name, "toolchain", QUALIFICATION_TOOLCHAINS)?;
    validate_matrix_axis(matrix, name, "precision", QUALIFICATION_PRECISIONS)
}

fn job_matrix<'a>(job: &'a YamlMapping, name: &str) -> Result<&'a YamlMapping> {
    let strategy = yaml_mapping(
        required_yaml_field(job, "strategy", name)?,
        &format!("{name} strategy"),
    )?;
    yaml_mapping(
        required_yaml_field(strategy, "matrix", &format!("{name} strategy"))?,
        &format!("{name} matrix"),
    )
}

fn require_exact_mapping_keys(
    mapping: &YamlMapping,
    expected: &[&str],
    context: &str,
) -> Result<()> {
    let actual = mapping
        .keys()
        .map(|key| {
            key.as_str()
                .ok_or_else(|| Error::message(format!("{context} key must be a string")))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected && mapping.len() == expected.len() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{context} keys must be exactly {expected:?}; found {actual:?}"
        )))
    }
}

fn require_exact_string_mapping_field(
    parent: &YamlMapping,
    key: &str,
    expected: &[(&str, &str)],
    context: &str,
) -> Result<()> {
    let mapping = yaml_mapping(
        required_yaml_field(parent, key, context)?,
        &format!("{context} {key}"),
    )?;
    let expected_keys = expected.iter().map(|(key, _)| *key).collect::<Vec<_>>();
    require_exact_mapping_keys(mapping, &expected_keys, &format!("{context} {key}"))?;
    for (field, value) in expected {
        require_exact_string_field(mapping, field, value, &format!("{context} {key}"))?;
    }
    Ok(())
}

fn validate_exact_strategy(job: &YamlMapping, name: &str) -> Result<()> {
    let strategy = yaml_mapping(
        required_yaml_field(job, "strategy", name)?,
        &format!("{name} strategy"),
    )?;
    require_exact_mapping_keys(
        strategy,
        &["fail-fast", "matrix"],
        &format!("{name} strategy"),
    )?;
    require_exact_bool_field(strategy, "fail-fast", false, &format!("{name} strategy"))
}

fn validate_exact_workflow_job(
    job: &YamlMapping,
    job_name: &str,
    expected_keys: &[&str],
    expected_display_name: &str,
    expected_runner: &str,
    expected_steps: &[WorkflowStepPolicy],
) -> Result<()> {
    let context = format!("workflow job {job_name:?}");
    require_exact_mapping_keys(job, expected_keys, &context)?;
    require_exact_string_field(job, "name", expected_display_name, &context)?;
    require_exact_string_field(job, "runs-on", expected_runner, &context)?;
    validate_exact_workflow_steps(job, job_name, expected_steps)
}

fn validate_exact_workflow_steps(
    job: &YamlMapping,
    job_name: &str,
    expected: &[WorkflowStepPolicy],
) -> Result<()> {
    let steps = workflow_steps(job, job_name)?;
    if steps.len() != expected.len() {
        return Err(Error::message(format!(
            "workflow job {job_name:?} must contain exactly {} reviewed steps; found {}",
            expected.len(),
            steps.len()
        )));
    }

    for (index, (step, policy)) in steps.iter().zip(expected).enumerate() {
        let context = format!("workflow job {job_name:?} step {index}");
        let step = yaml_mapping(step, &context)?;
        require_exact_mapping_keys(step, policy.keys, &context)?;
        require_exact_string_field(step, "name", policy.name, &context)?;
        match policy.kind {
            WorkflowStepKind::Action(action) => {
                require_exact_string_field(step, "uses", action, &context)?;
            }
            WorkflowStepKind::Run => {
                required_yaml_field(step, "run", &context)?
                    .as_str()
                    .ok_or_else(|| Error::message(format!("{context} run must be a string")))?;
            }
        }
        let actual_digest = yaml_value_digest(&YamlValue::Mapping(step.clone()), &context)?;
        if actual_digest != policy.digest {
            return Err(Error::message(format!(
                "{context} does not match the reviewed step contract: expected digest {}, found {actual_digest}",
                policy.digest
            )));
        }
    }
    Ok(())
}

fn yaml_value_digest(value: &YamlValue, context: &str) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        Error::message(format!(
            "failed to serialize {context} for exact policy comparison: {error}"
        ))
    })?;
    Ok(hex_digest(Sha256::digest(bytes)))
}

fn validate_matrix_axis(
    matrix: &YamlMapping,
    name: &str,
    axis: &str,
    expected: &[&str],
) -> Result<()> {
    let values = required_yaml_field(matrix, axis, &format!("{name} matrix"))?
        .as_sequence()
        .ok_or_else(|| Error::message(format!("{name} {axis} matrix axis must be a sequence")))?;
    let actual_values = values
        .iter()
        .map(|value| {
            value.as_str().ok_or_else(|| {
                Error::message(format!("{name} {axis} matrix coordinates must be strings"))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let actual = actual_values.iter().copied().collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected && values.len() == expected.len() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{name} {axis} matrix must be exactly {expected:?}; found {actual:?}"
        )))
    }
}

fn validate_platform_matrix(matrix: &YamlMapping, name: &str) -> Result<()> {
    let platforms = required_yaml_field(matrix, "platform", &format!("{name} matrix"))?
        .as_sequence()
        .ok_or_else(|| Error::message(format!("{name} platform matrix must be a sequence")))?;
    let mut actual = BTreeMap::new();
    for (index, platform) in platforms.iter().enumerate() {
        let context = format!("{name} platform matrix entry {index}");
        let platform = yaml_mapping(platform, &context)?;
        require_exact_mapping_keys(platform, &["crt", "os", "target"], &context)?;
        let target = required_yaml_field(platform, "target", &context)?
            .as_str()
            .ok_or_else(|| Error::message(format!("{context} target must be a string")))?;
        let os = required_yaml_field(platform, "os", &context)?
            .as_str()
            .ok_or_else(|| Error::message(format!("{context} os must be a string")))?;
        let crt = required_yaml_field(platform, "crt", &context)?
            .as_str()
            .ok_or_else(|| Error::message(format!("{context} CRT must be a string")))?;
        if actual
            .insert((target.to_owned(), crt.to_owned()), os.to_owned())
            .is_some()
        {
            return Err(Error::message(format!(
                "{name} repeats platform/CRT coordinate {target}/{crt}"
            )));
        }
    }
    let expected = BTreeMap::from([
        (
            ("aarch64-apple-darwin".to_owned(), "none".to_owned()),
            "macos-latest".to_owned(),
        ),
        (
            ("x86_64-apple-darwin".to_owned(), "none".to_owned()),
            "macos-15-intel".to_owned(),
        ),
        (
            ("x86_64-pc-windows-msvc".to_owned(), "md".to_owned()),
            "windows-latest".to_owned(),
        ),
        (
            ("x86_64-pc-windows-msvc".to_owned(), "mt".to_owned()),
            "windows-latest".to_owned(),
        ),
        (
            ("x86_64-unknown-linux-gnu".to_owned(), "none".to_owned()),
            "ubuntu-latest".to_owned(),
        ),
    ]);
    if actual != expected {
        return Err(Error::message(format!(
            "{name} platform/CRT matrix must be exactly {expected:?}; found {actual:?}"
        )));
    }
    Ok(())
}

fn require_enabled_fail_closed_job(section: &str, job: &YamlMapping, name: &str) -> Result<()> {
    let context = format!("CI {name} job");
    require_absent_field(job, "continue-on-error", &context)?;
    require_absent_field(job, "if", &context)?;
    for (index, step) in workflow_steps(job, name)?.iter().enumerate() {
        let context = format!("CI {name} step {index}");
        let step = yaml_mapping(step, &context)?;
        require_absent_field(step, "continue-on-error", &context)?;
        if let Some(condition) = yaml_field(step, "if") {
            let condition = condition
                .as_str()
                .ok_or_else(|| Error::message(format!("{context} condition must be a string")))?;
            if condition != "runner.os == 'Linux'" {
                return Err(Error::message(format!(
                    "{name} uses an unreviewed condition {condition:?}"
                )));
            }
        }
    }
    for command in ci_run_commands(section) {
        if command.line == "exit 0"
            || command.line.starts_with("exit 0 ")
            || command.line == "set +e"
            || command.line.contains("|| true")
            || command.line.contains("|| :")
            || command.line.contains("|| exit 0")
            || command.line.starts_with("if ")
            || command.line.contains(" then")
            || command.line == "fi"
            || command.line.starts_with("function ")
            || command.line.contains("() {")
            || command.line.contains("<<")
        {
            return Err(Error::message(format!(
                "{name} contains a command that can hide gate failures: {:?}",
                command.line
            )));
        }
    }
    Ok(())
}

fn validate_exact_qualification_job(
    job: &YamlMapping,
    name: &str,
    exact_command: &str,
    allowed_job_condition: Option<&str>,
    require_only_command: bool,
) -> Result<()> {
    let context = format!("{name} job");
    match allowed_job_condition {
        Some(condition) => require_exact_string_field(job, "if", condition, &context)?,
        None => require_absent_field(job, "if", &context)?,
    }
    require_absent_field(job, "continue-on-error", &context)?;
    for field in ["defaults", "env", "container", "services"] {
        require_absent_field(job, field, &context)?;
    }

    let steps = workflow_steps(job, name)?;
    let mut run_steps = 0_usize;
    let mut matching_steps = 0_usize;
    for (index, step) in steps.iter().enumerate() {
        let context = format!("{name} step {index}");
        let step = yaml_mapping(step, &context)?;
        require_absent_field(step, "continue-on-error", &context)?;
        require_absent_field(step, "if", &context)?;
        if require_only_command {
            require_absent_field(step, "shell", &context)?;
        }
        let Some(run) = yaml_field(step, "run") else {
            continue;
        };
        run_steps += 1;
        let run = run
            .as_str()
            .ok_or_else(|| Error::message(format!("{context} run command must be a string")))?;
        if run.contains("--allow-dirty") {
            return Err(Error::message(format!(
                "{name} must package a clean checkout without --allow-dirty"
            )));
        }
        if run == exact_command && !run.contains('\n') {
            require_exact_mapping_keys(step, &["name", "run"], &context)?;
            require_absent_field(step, "shell", &context)?;
            matching_steps += 1;
        }
    }
    if matching_steps != 1 || require_only_command && run_steps != 1 {
        return Err(Error::message(format!(
            "{name} must execute exactly one unconditional inline helper command {exact_command:?}"
        )));
    }
    Ok(())
}

fn validate_matrix_toolchain_install(
    job: &YamlMapping,
    name: &str,
    expected_target: Option<&str>,
) -> Result<()> {
    let mut installers = Vec::new();
    for (index, step) in workflow_steps(job, name)?.iter().enumerate() {
        let context = format!("{name} step {index}");
        let step = yaml_mapping(step, &context)?;
        if yaml_field(step, "uses").and_then(YamlValue::as_str) == Some(RUST_TOOLCHAIN_ACTION) {
            installers.push((context, step));
        }
    }
    if installers.len() != 1 {
        return Err(Error::message(format!(
            "{name} must install the matrix toolchain through exactly one pinned {RUST_TOOLCHAIN_ACTION} step"
        )));
    }
    let (context, installer) = &installers[0];
    let inputs = yaml_mapping(
        required_yaml_field(installer, "with", context)?,
        &format!("{context} inputs"),
    )?;
    let expected_keys = if expected_target.is_some() {
        &["targets", "toolchain"][..]
    } else {
        &["toolchain"][..]
    };
    require_exact_mapping_keys(inputs, expected_keys, &format!("{context} inputs"))?;
    require_exact_string_field(
        inputs,
        "toolchain",
        "${{ matrix.toolchain }}",
        &format!("{context} inputs"),
    )?;
    if let Some(target) = expected_target {
        require_exact_string_field(inputs, "targets", target, &format!("{context} inputs"))?;
    }
    Ok(())
}

fn workflow_steps<'a>(job: &'a YamlMapping, name: &str) -> Result<&'a Vec<YamlValue>> {
    required_yaml_field(job, "steps", &format!("{name} job"))?
        .as_sequence()
        .ok_or_else(|| Error::message(format!("{name} job steps must be a sequence")))
}

fn is_false_condition(value: &str) -> bool {
    let normalized = value
        .split_once(" #")
        .map_or(value, |(value, _)| value)
        .trim()
        .trim_matches(['"', '\''])
        .replace([' ', '\t'], "");
    normalized == "false" || normalized == "${{false}}"
}

#[derive(Debug, Eq, PartialEq)]
struct CiCommand {
    line: String,
    condition: Option<String>,
    inline: bool,
}

fn ci_run_commands(section: &str) -> Vec<CiCommand> {
    let mut commands = Vec::new();
    let mut run_indent = None;
    let mut step_indent = None;
    let mut disabled_step = false;
    let mut step_condition = None::<String>;

    for raw_line in section.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if trimmed.starts_with("- ") {
            step_indent = Some(indent);
            run_indent = None;
            disabled_step = false;
            step_condition = None;
        } else if let Some(base_indent) = step_indent
            && !trimmed.is_empty()
            && indent <= base_indent
        {
            step_indent = None;
            run_indent = None;
            disabled_step = false;
            step_condition = None;
        }

        if let Some(value) = trimmed.strip_prefix("if:") {
            let value = value
                .split_once(" #")
                .map_or(value, |(value, _)| value)
                .trim();
            step_condition = Some(value.to_owned());
            if is_false_condition(value) {
                disabled_step = true;
                run_indent = None;
            }
        }

        if disabled_step {
            continue;
        }

        if let Some(active_indent) = run_indent {
            if trimmed.is_empty() || indent > active_indent {
                let command = trimmed
                    .split_once(" #")
                    .map_or(trimmed, |(value, _)| value)
                    .trim();
                if !command.is_empty()
                    && !command.starts_with('#')
                    && !command.starts_with("echo ")
                    && !command.starts_with("printf ")
                {
                    commands.push(CiCommand {
                        line: command.to_owned(),
                        condition: step_condition.clone(),
                        inline: false,
                    });
                }
                continue;
            }
            run_indent = None;
        }

        if let Some(value) = trimmed.strip_prefix("run:") {
            let value = value
                .split_once(" #")
                .map_or(value, |(value, _)| value)
                .trim();
            if value == "|" || value == "|-" || value == "|+" || value == ">" || value == ">-" {
                run_indent = Some(indent);
            } else if !value.is_empty()
                && !value.starts_with('#')
                && !value.starts_with("echo ")
                && !value.starts_with("printf ")
            {
                commands.push(CiCommand {
                    line: value.to_owned(),
                    condition: step_condition.clone(),
                    inline: true,
                });
            }
        }
    }
    commands
}

fn is_command_fragment(fragment: &str) -> bool {
    let fragment = fragment.trim_start();
    fragment.starts_with("cargo ")
        || fragment.starts_with("BOXDD_SYS_") && fragment.contains('=')
        || fragment.starts_with("CARGO_TARGET_DIR=")
        || fragment.starts_with("cp ")
        || fragment.starts_with("mapfile ")
        || fragment.starts_with("test ")
        || fragment.starts_with("LIB=")
        || fragment.starts_with("npm ")
        || fragment.starts_with("npx ")
}

fn require_ci_command(
    commands: &[CiCommand],
    name: &str,
    command: &str,
    allowed_condition: Option<&str>,
) -> Result<()> {
    if commands.iter().any(|candidate| {
        candidate.line == command && candidate.condition.as_deref() == allowed_condition
    }) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "CI {name} job is missing required executable command {command:?}"
        )))
    }
}

fn require_ci_metadata_line(section: &str, name: &str, line: &str) -> Result<()> {
    if section.lines().any(|candidate| candidate.trim() == line) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "CI {name} job is missing required metadata line {line:?}"
        )))
    }
}

fn validate_ci_triggers(global: &str) -> Result<()> {
    require_workflow_fragment(
        global,
        "on:\n  push:\n    branches: [ main, master ]\n  pull_request:\n    branches: [ main, master ]",
        "CI must run on pushes and pull requests to main/master without path filtering",
    )?;
    for line in global.lines().map(str::trim) {
        if line.starts_with("paths:")
            || line.starts_with("paths-ignore:")
            || line.starts_with("branches-ignore:")
        {
            return Err(Error::message(format!(
                "CI trigger contains a restrictive filter: {line}"
            )));
        }
    }
    Ok(())
}

fn validate_native_matrix(section: &str) -> Result<()> {
    require_ci_metadata_line(
        section,
        "native matrix",
        "os: [ubuntu-latest, macos-latest, windows-latest]",
    )?;
    for line in section.lines().map(str::trim) {
        if line.starts_with("include:") || line.starts_with("exclude:") {
            return Err(Error::message(
                "CI native matrix must not include or exclude platform overrides",
            ));
        }
    }
    Ok(())
}

fn require_workflow_fragment(source: &str, fragment: &str, message: &str) -> Result<()> {
    if source.contains(fragment) {
        Ok(())
    } else {
        Err(Error::message(message.to_owned()))
    }
}

fn workflow_job<'a>(jobs: &'a str, name: &str) -> Result<&'a str> {
    let marker = format!("  {name}:\n");
    let start = jobs
        .find(&marker)
        .ok_or_else(|| Error::message(format!("release workflow is missing job {name}")))?;
    let body_start = start + marker.len();
    let rest = &jobs[body_start..];
    let mut end = rest.len();
    for (offset, _) in rest.match_indices("\n  ") {
        let line_start = offset + 1;
        let line = &rest[line_start..];
        if !line.starts_with("    ")
            && line
                .split_once('\n')
                .map_or(line, |(line, _)| line)
                .ends_with(':')
        {
            end = offset;
            break;
        }
    }
    Ok(&rest[..end])
}

fn validate_ci_workflow(root: &Path) -> Result<()> {
    let path = root.join(".github/workflows/ci.yml");
    let source = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    validate_ci_workflow_source(&source)
}

fn validate_ci_workflow_source(source: &str) -> Result<()> {
    let workflow = parse_workflow_yaml(source, "CI workflow")?;
    let workflow_mapping = yaml_mapping(&workflow, "CI workflow")?;
    require_exact_mapping_keys(
        workflow_mapping,
        &["name", "on", "permissions", "env", "jobs"],
        "CI workflow",
    )?;
    require_exact_string_field(workflow_mapping, "name", "CI", "CI workflow")?;
    require_exact_string_mapping_field(
        workflow_mapping,
        "env",
        &[("CARGO_TERM_COLOR", "always")],
        "CI workflow",
    )?;
    let structured_jobs = workflow_jobs(workflow_mapping, "CI workflow")?;
    require_exact_mapping_keys(
        structured_jobs,
        &[
            "build",
            "compiler-baseline",
            "features",
            "lint",
            "miri",
            "provider-runtime",
            "sanitizers",
            "security",
            "system-provider",
            "wasm",
        ],
        "CI workflow jobs",
    )?;
    let jobs = source
        .split_once("\njobs:\n")
        .map(|(_, jobs)| jobs)
        .ok_or_else(|| Error::message("CI workflow has no jobs section"))?;
    let global = source
        .split_once("\njobs:\n")
        .map(|(global, _)| global)
        .unwrap_or(source);
    validate_ci_triggers(global)?;
    require_exact_permissions(workflow_mapping, "CI workflow", &[("contents", "read")])?;
    for forbidden in [
        "contents: write",
        "id-token: write",
        "pages: write",
        "serialize",
        "--all-features",
    ] {
        if source.contains(forbidden) {
            return Err(Error::message(format!(
                "CI contains stale or privileged policy fragment {forbidden:?}"
            )));
        }
    }

    let compiler_baseline = workflow_job(jobs, "compiler-baseline")?;
    let lint = workflow_job(jobs, "lint")?;
    let system_provider = workflow_job(jobs, "system-provider")?;
    let native = workflow_job(jobs, "build")?;
    let features = workflow_job(jobs, "features")?;
    let wasm = workflow_job(jobs, "wasm")?;
    let provider_runtime = workflow_job(jobs, "provider-runtime")?;
    let security = workflow_job(jobs, "security")?;
    let miri = workflow_job(jobs, "miri")?;
    let sanitizers = workflow_job(jobs, "sanitizers")?;
    let compiler_baseline_yaml =
        workflow_job_mapping(structured_jobs, "compiler-baseline", "CI workflow")?;
    let lint_yaml = workflow_job_mapping(structured_jobs, "lint", "CI workflow")?;
    let system_provider_yaml =
        workflow_job_mapping(structured_jobs, "system-provider", "CI workflow")?;
    let native_yaml = workflow_job_mapping(structured_jobs, "build", "CI workflow")?;
    let features_yaml = workflow_job_mapping(structured_jobs, "features", "CI workflow")?;
    let wasm_yaml = workflow_job_mapping(structured_jobs, "wasm", "CI workflow")?;
    let provider_runtime_yaml =
        workflow_job_mapping(structured_jobs, "provider-runtime", "CI workflow")?;
    let security_yaml = workflow_job_mapping(structured_jobs, "security", "CI workflow")?;
    let miri_yaml = workflow_job_mapping(structured_jobs, "miri", "CI workflow")?;
    let sanitizers_yaml = workflow_job_mapping(structured_jobs, "sanitizers", "CI workflow")?;
    validate_native_matrix(native)?;
    validate_exact_workflow_job(
        system_provider_yaml,
        "system-provider",
        &["name", "runs-on", "permissions", "strategy", "steps"],
        "System Provider (Rust ${{ matrix.toolchain }}, ${{ matrix.precision }})",
        "ubuntu-latest",
        SYSTEM_PROVIDER_STEPS,
    )?;
    validate_system_provider_matrix(system_provider_yaml)?;
    validate_matrix_toolchain_install(system_provider_yaml, "system provider qualification", None)?;
    validate_exact_qualification_job(
        system_provider_yaml,
        "system provider qualification",
        SYSTEM_QUALIFICATION_COMMAND,
        None,
        false,
    )?;
    for (name, section, job) in [
        (
            "compiler baseline",
            compiler_baseline,
            compiler_baseline_yaml,
        ),
        ("lint and integration", lint, lint_yaml),
        ("system provider", system_provider, system_provider_yaml),
        ("native matrix", native, native_yaml),
        ("feature matrix", features, features_yaml),
        ("WASM compile", wasm, wasm_yaml),
        (
            "WASM provider runtime",
            provider_runtime,
            provider_runtime_yaml,
        ),
        ("supply chain", security, security_yaml),
        ("Miri", miri, miri_yaml),
        ("sanitizers", sanitizers, sanitizers_yaml),
    ] {
        require_exact_permissions(job, &format!("CI {name} job"), &[("contents", "read")])?;
        require_enabled_fail_closed_job(section, job, name)?;
    }

    require_ci_job_fragments(
        compiler_baseline,
        "compiler baseline",
        &[
            "toolchain: 1.95.0",
            "cargo +1.95.0 run --locked -p xtask -- verify-toolchains",
            "cargo +1.95.0 check --locked --workspace --all-targets",
        ],
    )?;
    require_ci_job_fragments(
        lint,
        "lint and integration",
        &[
            "cargo fmt --all -- --check",
            "cargo clippy --locked -p boxdd-sys --all-targets -- -D warnings",
            "cargo clippy --locked -p boxdd-sys --all-targets --features \"double-precision validate disable-simd\" -- -D warnings",
            "cargo clippy --locked -p boxdd-sys --features package-bin --bin package -- -D warnings",
            "cargo clippy --locked -p boxdd-sys --features \"package-bin,double-precision,validate,disable-simd\" --bin package -- -D warnings",
            "cargo clippy --locked -p boxdd-sys --features \"package-bin,simd-avx2\" --bin package -- -D warnings",
            "cargo clippy --locked -p boxdd --all-targets --features \"serde mint nalgebra glam bytemuck unchecked\" -- -D warnings",
            "cargo clippy --locked -p boxdd --all-targets --features \"double-precision serde mint nalgebra glam bytemuck unchecked validate disable-simd\" -- -D warnings",
            "cargo clippy --locked -p bevy_boxdd --all-targets --no-default-features -- -D warnings",
            "cargo clippy --locked -p bevy_boxdd --all-targets --features double-precision -- -D warnings",
            "cargo run --locked -p xtask -- verify-precision-contract",
            "cargo run --locked -p xtask -- upstream-sync --check",
            "cargo run --locked -p xtask -- api-coverage --check",
            "cargo run --locked -p xtask -- sample-parity --check",
            "cargo run --locked -p xtask -- verify-feature-matrix",
            "cargo run --locked -p xtask -- verify-compile-fail",
            "cargo run --locked -p xtask -- validate-pages",
            "cargo check --locked --workspace --all-targets",
            "cargo nextest run --locked --workspace",
            "cargo test --locked -p boxdd-sys --features package-bin --bin package",
            "cargo test --locked --target-dir target/package-helper-double -p boxdd-sys --features \"package-bin,double-precision,validate,disable-simd\" --bin package",
            "cargo test --locked --target-dir target/package-helper-avx2 -p boxdd-sys --features \"package-bin,simd-avx2\" --bin package",
            "cargo nextest run --locked -p boxdd -p boxdd-sys --features boxdd/double-precision",
            "cargo nextest run --locked -p bevy_boxdd --features double-precision",
            "cargo nextest run --locked -p boxdd --test serde_values --features serde",
            "cargo nextest run --locked --target-dir target/serde-double -p boxdd --test serde_values --features \"double-precision serde\"",
            "cargo nextest run --locked --target-dir target/interops-double -p boxdd --test mint_interop --test nalgebra_interop --test glam_interop --test bytemuck_api --features \"double-precision mint nalgebra glam bytemuck\"",
            "cargo check --locked -p boxdd --examples --features \"double-precision mint\"",
            "cargo check --locked -p bevy_boxdd --examples --features double-precision",
            "cargo check --locked -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed",
        ],
    )?;
    require_ci_job_fragments(
        system_provider,
        "system provider",
        &[
            "toolchain: [\"1.95.0\", \"1.97.1\"]",
            "precision: [single, double]",
            "CARGO_TARGET_DIR=\"$source_target\" cargo +${{ matrix.toolchain }} build --locked -p boxdd-sys --features \"${{ matrix.precision == 'double' && 'double-precision' || '' }}\" --quiet",
            "mapfile -t archives < <(find \"$source_target/debug/build\" -path '*/boxdd-sys-*/out/libbox2d.a' -type f -print)",
            "test \"${#archives[@]}\" -eq 1",
            "SYS_DIR=\"${RUNNER_TEMP}/boxdd-system-artifact\"",
            "CARGO_TARGET_DIR=\"$attest_target\" cargo +${{ matrix.toolchain }} run --locked -p boxdd-sys --features \"${{ matrix.precision == 'double' && 'package-bin,double-precision' || 'package-bin' }}\" --bin package -- attest-local-system \"$SYS_DIR/libbox2d.a\" \"$SYS_DIR/box2d.h\" \"$SYS_DIR/bindings.rs\" \"$SYS_DIR/manifest.toml\"",
            SYSTEM_QUALIFICATION_COMMAND,
        ],
    )?;
    require_ci_job_fragments(
        native,
        "native matrix",
        &[
            "runs-on: ${{ matrix.os }}",
            "os: [ubuntu-latest, macos-latest, windows-latest]",
            "tool: nextest",
            "cargo check --locked --workspace --all-targets",
            "cargo nextest run --locked -p boxdd -p boxdd-sys",
            "cargo nextest run --locked --target-dir target/core-double -p boxdd -p boxdd-sys --features boxdd/double-precision",
            "cargo test --locked -p boxdd-abi-probe --test abi --no-default-features",
            "cargo test --locked --target-dir target/abi-probe-double -p boxdd-abi-probe --test abi --no-default-features --features double-precision",
            "cargo doc --locked --no-deps --workspace",
            "cargo doc --locked --no-deps -p boxdd --features double-precision",
            "cargo doc --locked --no-deps -p bevy_boxdd --features double-precision",
        ],
    )?;
    require_ci_job_fragments(
        features,
        "feature matrix",
        &[
            "toolchain: [\"1.95.0\", \"1.97.1\"]",
            "cargo +${{ matrix.toolchain }} run --locked -p xtask -- verify-feature-matrix",
        ],
    )?;
    require_ci_job_fragments(
        wasm,
        "WASM compile",
        &[
            "toolchain: [\"1.95.0\", \"1.97.1\"]",
            "targets: wasm32-unknown-unknown,wasm32-wasip1",
            "cargo +${{ matrix.toolchain }} run --locked -p xtask -- verify-wasm --compile-only",
        ],
    )?;
    require_ci_job_fragments(
        provider_runtime,
        "WASM provider runtime",
        &[
            "EMSDK_VERSION: \"6.0.3\"",
            "EMSDK_REVISION: \"db04e88298d9916fc51fcd3743045ca3eb695127\"",
            "CARGO_TARGET_DIR: target/provider-runtime",
            "targets: wasm32-unknown-unknown",
            "npm ci --ignore-scripts",
            "npx playwright install --with-deps chromium",
            "cargo run --locked -p xtask -- verify-wasm --runtime",
        ],
    )?;
    require_ci_job_fragments(
        security,
        "supply chain",
        &[
            "cargo audit --file Cargo.lock",
            "cargo run --locked -p xtask -- verify-packages",
            "cargo run --locked -p xtask -- verify-semver",
            "cargo run --locked -p xtask -- release-contract --check",
        ],
    )?;
    require_ci_job_fragments(
        miri,
        "Miri",
        &[
            "toolchain: nightly-2026-05-27",
            "components: miri, rust-src",
            "cargo +nightly-2026-05-27 run --locked -p xtask -- verify-miri",
        ],
    )?;
    require_ci_job_fragments(
        sanitizers,
        "sanitizers",
        &[
            "sanitizer: [address, undefined, thread]",
            "toolchain: nightly-2026-05-27",
            "components: rust-src",
            "cargo +nightly-2026-05-27 run --locked -p xtask -- verify-sanitizers --${{ matrix.sanitizer }}",
        ],
    )?;
    Ok(())
}

fn require_ci_job_fragments(section: &str, name: &str, fragments: &[&str]) -> Result<()> {
    let commands = ci_run_commands(section);
    for fragment in fragments {
        if is_command_fragment(fragment) {
            let allowed_condition = if name == "native matrix" && fragment.starts_with("cargo doc ")
            {
                Some("runner.os == 'Linux'")
            } else {
                None
            };
            require_ci_command(&commands, name, fragment, allowed_condition)?;
        } else {
            require_ci_metadata_line(section, name, fragment)?;
        }
    }
    Ok(())
}

fn validate_pages_workflow_source(source: &str, sdk: &SdkContract) -> Result<()> {
    let workflow = parse_workflow_yaml(source, "Pages workflow")?;
    let workflow = yaml_mapping(&workflow, "Pages workflow")?;
    require_exact_mapping_keys(
        workflow,
        &["name", "on", "permissions", "concurrency", "jobs"],
        "Pages workflow",
    )?;
    require_exact_string_field(workflow, "name", "Pages", "Pages workflow")?;
    require_exact_permissions(workflow, "Pages workflow", &[("contents", "read")])?;

    let trigger = yaml_mapping(
        required_yaml_field(workflow, "on", "Pages workflow")?,
        "Pages workflow trigger",
    )?;
    require_exact_mapping_keys(
        trigger,
        &["push", "workflow_dispatch"],
        "Pages workflow trigger",
    )?;
    let push = yaml_mapping(
        required_yaml_field(trigger, "push", "Pages workflow trigger")?,
        "Pages workflow push trigger",
    )?;
    require_exact_mapping_keys(push, &["branches"], "Pages workflow push trigger")?;
    let branches = required_yaml_field(push, "branches", "Pages workflow push trigger")?
        .as_sequence()
        .ok_or_else(|| Error::message("Pages workflow branches must be a sequence"))?;
    if branches.as_slice() != [YamlValue::String("main".to_owned())] {
        return Err(Error::message(
            "Pages workflow push branches must be exactly [main]",
        ));
    }
    match required_yaml_field(trigger, "workflow_dispatch", "Pages workflow trigger")? {
        YamlValue::Null => {}
        YamlValue::Mapping(mapping) if mapping.is_empty() => {}
        _ => {
            return Err(Error::message(
                "Pages workflow_dispatch trigger must not declare inputs or options",
            ));
        }
    }

    let concurrency = yaml_mapping(
        required_yaml_field(workflow, "concurrency", "Pages workflow")?,
        "Pages workflow concurrency",
    )?;
    require_exact_mapping_keys(
        concurrency,
        &["group", "cancel-in-progress"],
        "Pages workflow concurrency",
    )?;
    require_exact_string_field(
        concurrency,
        "group",
        "github-pages",
        "Pages workflow concurrency",
    )?;
    require_exact_bool_field(
        concurrency,
        "cancel-in-progress",
        true,
        "Pages workflow concurrency",
    )?;

    let jobs = workflow_jobs(workflow, "Pages workflow")?;
    require_exact_mapping_keys(jobs, &["build", "deploy"], "Pages workflow jobs")?;
    let build = workflow_job_mapping(jobs, "build", "Pages workflow")?;
    let deploy = workflow_job_mapping(jobs, "deploy", "Pages workflow")?;
    require_exact_mapping_keys(
        build,
        &["name", "runs-on", "env", "steps"],
        "Pages build job",
    )?;
    require_exact_string_field(
        build,
        "name",
        "Build and upload static site",
        "Pages build job",
    )?;
    require_exact_string_field(build, "runs-on", "ubuntu-latest", "Pages build job")?;
    require_absent_field(build, "permissions", "Pages build job")?;
    require_exact_string_mapping_field(
        build,
        "env",
        &[
            ("EMSDK_VERSION", sdk.emscripten_version.as_str()),
            ("EMSDK_REVISION", sdk.emsdk_revision.as_str()),
        ],
        "Pages build job",
    )?;
    validate_pages_build_steps(build, sdk)?;

    require_exact_mapping_keys(
        deploy,
        &[
            "name",
            "if",
            "needs",
            "runs-on",
            "permissions",
            "environment",
            "steps",
        ],
        "Pages deploy job",
    )?;
    require_exact_string_field(deploy, "name", "Deploy", "Pages deploy job")?;
    require_exact_string_field(
        deploy,
        "if",
        "github.ref == 'refs/heads/main' && github.ref_protected == true",
        "Pages deploy job",
    )?;
    require_exact_string_field(deploy, "needs", "build", "Pages deploy job")?;
    require_exact_string_field(deploy, "runs-on", "ubuntu-latest", "Pages deploy job")?;
    require_exact_permissions(
        deploy,
        "Pages deploy job",
        &[
            ("contents", "read"),
            ("id-token", "write"),
            ("pages", "write"),
        ],
    )?;
    let environment = yaml_mapping(
        required_yaml_field(deploy, "environment", "Pages deploy job")?,
        "Pages deploy environment",
    )?;
    require_exact_mapping_keys(environment, &["name", "url"], "Pages deploy environment")?;
    require_exact_string_field(
        environment,
        "name",
        "github-pages",
        "Pages deploy environment",
    )?;
    require_exact_string_field(
        environment,
        "url",
        "${{ steps.deployment.outputs.page_url }}",
        "Pages deploy environment",
    )?;
    validate_pages_deploy_steps(deploy)
}

fn pages_step<'a>(
    steps: &'a [YamlValue],
    index: usize,
    expected_name: &str,
    expected_keys: &[&str],
) -> Result<&'a YamlMapping> {
    let context = format!("Pages workflow step {index}");
    let step = steps
        .get(index)
        .ok_or_else(|| Error::message(format!("Pages workflow is missing step {index}")))?;
    let step = yaml_mapping(step, &context)?;
    require_exact_mapping_keys(step, expected_keys, &context)?;
    require_exact_string_field(step, "name", expected_name, &context)?;
    Ok(step)
}

fn validate_pages_build_steps(build: &YamlMapping, sdk: &SdkContract) -> Result<()> {
    const EMSDK_INSTALL_COMMAND_TEMPLATE: &str = r#"set -euo pipefail
git init "${RUNNER_TEMP}/emsdk"
git -C "${RUNNER_TEMP}/emsdk" remote add origin __EMSDK_REPOSITORY__
git -C "${RUNNER_TEMP}/emsdk" fetch --depth 1 origin "${EMSDK_REVISION}"
git -C "${RUNNER_TEMP}/emsdk" checkout --detach FETCH_HEAD
test "$(git -C "${RUNNER_TEMP}/emsdk" rev-parse HEAD)" = "${EMSDK_REVISION}"
"${RUNNER_TEMP}/emsdk/emsdk" install "${EMSDK_VERSION}"
"${RUNNER_TEMP}/emsdk/emsdk" activate "${EMSDK_VERSION}"
{
  echo "EMSDK=${RUNNER_TEMP}/emsdk"
} >> "$GITHUB_ENV"
{
  echo "${RUNNER_TEMP}/emsdk"
  echo "${RUNNER_TEMP}/emsdk/upstream/emscripten"
  echo "${RUNNER_TEMP}/emsdk/upstream/bin"
} >> "$GITHUB_PATH""#;
    let emsdk_install_command =
        EMSDK_INSTALL_COMMAND_TEMPLATE.replace("__EMSDK_REPOSITORY__", &sdk.emsdk_repository);

    let steps = workflow_steps(build, "Pages build")?;
    if steps.len() != 13 {
        return Err(Error::message(format!(
            "Pages build job must contain exactly 13 reviewed steps; found {}",
            steps.len()
        )));
    }

    let checkout = pages_step(steps, 0, "Checkout", &["name", "uses", "with"])?;
    require_exact_string_field(checkout, "uses", CHECKOUT_ACTION, "Pages checkout step")?;
    let checkout_inputs = yaml_mapping(
        required_yaml_field(checkout, "with", "Pages checkout step")?,
        "Pages checkout inputs",
    )?;
    require_exact_mapping_keys(
        checkout_inputs,
        &["submodules", "persist-credentials"],
        "Pages checkout inputs",
    )?;
    require_exact_string_field(
        checkout_inputs,
        "submodules",
        "recursive",
        "Pages checkout inputs",
    )?;
    require_exact_bool_field(
        checkout_inputs,
        "persist-credentials",
        false,
        "Pages checkout inputs",
    )?;

    let rust = pages_step(steps, 1, "Install Rust", &["name", "uses", "with"])?;
    require_exact_string_field(rust, "uses", RUST_TOOLCHAIN_ACTION, "Pages Rust step")?;
    require_exact_string_mapping_field(
        rust,
        "with",
        &[
            ("toolchain", "1.97.1"),
            ("targets", "wasm32-unknown-unknown"),
        ],
        "Pages Rust step",
    )?;

    let cache = pages_step(steps, 2, "Cache Rust dependencies", &["name", "uses"])?;
    require_exact_string_field(cache, "uses", RUST_CACHE_ACTION, "Pages cache step")?;

    let wasm_bindgen = pages_step(steps, 3, "Install wasm-bindgen CLI", &["name", "run"])?;
    require_exact_string_field(
        wasm_bindgen,
        "run",
        &format!(
            "cargo install wasm-bindgen-cli --version {} --locked",
            sdk.wasm_bindgen_version
        ),
        "Pages wasm-bindgen step",
    )?;

    let emsdk = pages_step(
        steps,
        4,
        "Install Emscripten SDK",
        &["name", "shell", "run"],
    )?;
    require_exact_string_field(emsdk, "shell", "bash", "Pages Emscripten step")?;
    require_exact_trimmed_string_field(
        emsdk,
        "run",
        &emsdk_install_command,
        "Pages Emscripten step",
    )?;

    let build_assets = pages_step(steps, 5, "Build Pages WASM assets", &["name", "run"])?;
    require_exact_string_field(
        build_assets,
        "run",
        "cargo run --locked -p xtask -- build-pages-wasm",
        "Pages build-assets step",
    )?;

    let node = pages_step(steps, 6, "Install Node.js", &["name", "uses", "with"])?;
    require_exact_string_field(node, "uses", SETUP_NODE_ACTION, "Pages Node.js step")?;
    require_exact_string_mapping_field(
        node,
        "with",
        &[
            ("node-version", sdk.node_version.as_str()),
            ("cache", "npm"),
        ],
        "Pages Node.js step",
    )?;

    let npm = pages_step(
        steps,
        7,
        "Install browser test dependencies",
        &["name", "run"],
    )?;
    require_exact_string_field(
        npm,
        "run",
        "npm ci --ignore-scripts",
        "Pages npm dependency step",
    )?;

    let chromium = pages_step(steps, 8, "Install Chromium", &["name", "run"])?;
    require_exact_string_field(
        chromium,
        "run",
        "npx playwright install --with-deps chromium",
        "Pages Chromium step",
    )?;

    let browser = pages_step(
        steps,
        9,
        "Prove published Pages runtime in Chromium",
        &["name", "run"],
    )?;
    require_exact_string_field(
        browser,
        "run",
        "npm run test:pages-browser",
        "Pages browser proof step",
    )?;

    let validate = pages_step(
        steps,
        10,
        "Validate commit-bound Pages content and runtime manifest",
        &["name", "run"],
    )?;
    require_exact_string_field(
        validate,
        "run",
        "cargo run --locked -p xtask -- validate-pages",
        "Pages validation step",
    )?;

    let configure = pages_step(steps, 11, "Configure Pages", &["name", "uses"])?;
    require_exact_string_field(
        configure,
        "uses",
        CONFIGURE_PAGES_ACTION,
        "Pages configuration step",
    )?;

    let upload = pages_step(steps, 12, "Upload artifact", &["name", "uses", "with"])?;
    require_exact_string_field(upload, "uses", UPLOAD_PAGES_ACTION, "Pages upload step")?;
    require_exact_string_mapping_field(
        upload,
        "with",
        &[("path", "docs/pages")],
        "Pages upload step",
    )
}

fn validate_pages_deploy_steps(deploy: &YamlMapping) -> Result<()> {
    let steps = workflow_steps(deploy, "Pages deploy")?;
    if steps.len() != 1 {
        return Err(Error::message(format!(
            "Pages deploy job must contain exactly one reviewed step; found {}",
            steps.len()
        )));
    }
    let deployment = pages_step(steps, 0, "Deploy Pages", &["name", "id", "uses"])?;
    require_exact_string_field(deployment, "id", "deployment", "Pages deployment step")?;
    require_exact_string_field(
        deployment,
        "uses",
        DEPLOY_PAGES_ACTION,
        "Pages deployment step",
    )
}

fn validate_pages_workflow(root: &Path) -> Result<()> {
    let path = root.join(".github/workflows/pages.yml");
    let source = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let sdk_path = root.join("boxdd-sys").join(SDK_CONTRACT_RELATIVE_PATH);
    let sdk_source = fs::read_to_string(&sdk_path).map_err(|error| Error::io(&sdk_path, error))?;
    let sdk = SdkContract::parse(&sdk_source).map_err(Error::message)?;
    validate_pages_workflow_source(&source, &sdk)
}

fn validate_audit_policy(root: &Path) -> Result<()> {
    let path = root.join(".cargo/audit.toml");
    match fs::symlink_metadata(&path) {
        Ok(_) => Err(Error::message(format!(
            "{} must remain absent; cargo audit exceptions are not part of the release contract",
            path.display()
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io(&path, error)),
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| Error::io(&directory, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| Error::io(&directory, error))?;
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

fn sha256_file(path: &Path) -> Result<String> {
    let source = fs::read(path).map_err(|error| Error::io(path, error))?;
    Ok(hex_digest(Sha256::digest(source)))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
    let github = github_execution_context();
    let mut command = release_git_command(github)?;
    if github {
        remove_git_environment(&mut command);
        command.arg("--no-replace-objects");
    }
    let output = command
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| Error::io(label, error))?;
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

    fn pages_sdk_contract() -> SdkContract {
        SdkContract::parse(include_str!("../../../boxdd-sys/emscripten-sdk.toml")).unwrap()
    }

    fn write_tar_fixture(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            archive.append_data(&mut header, *name, *bytes).unwrap();
        }
        archive.finish().unwrap();
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
        assert_eq!(artifacts.len(), 10);
        assert_eq!(
            artifacts
                .iter()
                .map(|artifact| artifact.archive.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            10
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
    }

    #[test]
    fn workflow_matrix_axes_must_be_direct_strategy_matrix_children() {
        let direct = parse_workflow_yaml(
            "strategy:\n  matrix:\n    toolchain: [\"1.95.0\", \"1.97.1\"]\n",
            "fixture",
        )
        .unwrap();
        let direct = yaml_mapping(&direct, "fixture").unwrap();
        let direct = job_matrix(direct, "fixture").unwrap();
        assert!(
            validate_matrix_axis(direct, "fixture", "toolchain", QUALIFICATION_TOOLCHAINS).is_ok()
        );

        let forged = parse_workflow_yaml(
            "strategy:\n  matrix:\n    precision: [single, double]\nenv:\n  toolchain: [\"1.95.0\", \"1.97.1\"]\n",
            "fixture",
        )
        .unwrap();
        let forged = yaml_mapping(&forged, "fixture").unwrap();
        let forged = job_matrix(forged, "fixture").unwrap();
        assert!(
            validate_matrix_axis(forged, "fixture", "toolchain", QUALIFICATION_TOOLCHAINS).is_err(),
            "matrix validation accepted an axis forged outside strategy.matrix"
        );
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
    fn intentional_break_requires_minor_or_major_bump() {
        let previous = Version::parse("0.5.0").unwrap();
        assert!(
            Version::parse("0.6.0")
                .unwrap()
                .allows_breaking_from(previous)
        );
        assert!(
            !Version::parse("0.5.1")
                .unwrap()
                .allows_breaking_from(previous)
        );
        assert!(
            Version::parse("1.0.0")
                .unwrap()
                .allows_breaking_from(previous)
        );
    }

    #[test]
    fn command_parser_rejects_arbitrary_or_duplicate_inputs() {
        let options = Options::parse(&[
            "--check".to_owned(),
            "--tag".to_owned(),
            "v0.6.0".to_owned(),
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
        ])
        .unwrap();
        assert_eq!(attempt.run_attempt.as_deref(), Some("2"));
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

        let mut bounded = BoundedReader::new(std::io::Cursor::new([1_u8, 2, 3]), 2);
        let mut bytes = Vec::new();
        assert!(bounded.read_to_end(&mut bytes).is_err());
        assert_eq!(bytes, [1, 2]);
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
    fn github_release_policy_reads_the_commit_blob_and_rejects_hidden_index_state() {
        let temp = TempDir::new().unwrap();
        let workflow_path = temp.path().join(PUBLISHER_WORKFLOW);
        fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
        let reviewed = include_str!("../../../.github/workflows/prebuilt-binaries.yml");
        fs::write(&workflow_path, reviewed).unwrap();

        for args in [
            &["init", "--initial-branch=main"][..],
            &["config", "user.name", "Boxdd Test"][..],
            &["config", "user.email", "boxdd@example.invalid"][..],
            &["config", "commit.gpgsign", "false"][..],
            &["add", PUBLISHER_WORKFLOW][..],
            &["commit", "-m", "review workflow"][..],
        ] {
            isolated_git_output(temp.path(), args, "prepare release workflow Git fixture").unwrap();
        }
        let commit = isolated_git_output(
            temp.path(),
            &["rev-parse", "HEAD"],
            "read release workflow fixture commit",
        )
        .unwrap();
        let commit = String::from_utf8(commit.stdout).unwrap();
        let commit = commit.trim();

        let injected = reviewed.replacen(
            "    environment: release\n    steps:\n      - name: Download validated attestation input",
            "    environment: release\n    steps:\n      - name: Unreviewed OIDC action\n        uses: example.invalid/action@deadbeef\n\n      - name: Download validated attestation input",
            1,
        );
        assert_ne!(injected, reviewed);
        fs::write(&workflow_path, &injected).unwrap();

        let local_source = read_release_workflow_source(temp.path(), None).unwrap();
        assert!(validate_release_workflow_source(&local_source).is_err());
        let immutable_source = read_release_workflow_source(temp.path(), Some(commit)).unwrap();
        assert_eq!(immutable_source, reviewed);
        assert!(validate_release_workflow_source(&immutable_source).is_ok());

        isolated_git_output(
            temp.path(),
            &["update-index", "--assume-unchanged", PUBLISHER_WORKFLOW],
            "hide release workflow through assume-unchanged",
        )
        .unwrap();
        assert!(read_release_workflow_source(temp.path(), Some(commit)).is_err());

        isolated_git_output(
            temp.path(),
            &["update-index", "--no-assume-unchanged", PUBLISHER_WORKFLOW],
            "clear release workflow assume-unchanged flag",
        )
        .unwrap();
        fs::write(&workflow_path, reviewed).unwrap();
        isolated_git_output(
            temp.path(),
            &["update-index", "--skip-worktree", PUBLISHER_WORKFLOW],
            "hide release workflow through skip-worktree",
        )
        .unwrap();
        fs::write(&workflow_path, injected).unwrap();
        assert!(read_release_workflow_source(temp.path(), Some(commit)).is_err());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn immutable_release_git_ignores_path_substitution() {
        let temp = TempDir::new().unwrap();
        fs::copy("/usr/bin/false", temp.path().join("git")).unwrap();
        let output = release_git_command(true)
            .unwrap()
            .env("PATH", temp.path())
            .arg("--version")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .starts_with("git version ")
        );
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
        let paths = expected_archive_paths(workspace, &spec).unwrap();
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
    fn trusted_root_digest_is_pinned_and_not_caller_selected() {
        assert_eq!(SIGSTORE_TRUSTED_ROOT_SHA256.len(), 64);
        assert!(
            SIGSTORE_TRUSTED_ROOT_SHA256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );
    }

    #[test]
    fn checked_in_workflows_satisfy_least_privilege_policy() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask must live below the workspace");
        let commit = git_output(workspace, &["rev-parse", "HEAD"], "read test commit").unwrap();
        validate_release_workflow(workspace, &commit).unwrap();
        validate_ci_workflow(workspace).unwrap();
        validate_pages_workflow(workspace).unwrap();
        validate_audit_policy(workspace).unwrap();
    }

    #[test]
    fn pages_workflow_rejects_execution_and_provenance_drift() {
        let source = include_str!("../../../.github/workflows/pages.yml");
        let sdk = pages_sdk_contract();
        assert!(validate_pages_workflow_source(source, &sdk).is_ok());

        for (reviewed, drifted) in [
            ("branches: [ main ]", "branches: [ feature ]"),
            ("group: github-pages", "group: unreviewed-pages"),
            ("cancel-in-progress: true", "cancel-in-progress: false"),
            (
                "actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0",
                "actions/checkout@main",
            ),
            (
                "dtolnay/rust-toolchain@2c7215f132e9ebf062739d9130488b56d53c060c",
                "dtolnay/rust-toolchain@master",
            ),
            (
                "Swatinem/rust-cache@c19371144df3bb44fab255c43d04cbc2ab54d1c4",
                "Swatinem/rust-cache@v2",
            ),
            (
                "actions/setup-node@395ad3262231945c25e8478fd5baf05154b1d79f",
                "actions/setup-node@main",
            ),
            (
                "actions/configure-pages@45bfe0192ca1faeb007ade9deae92b16b8254a0d",
                "actions/configure-pages@v6",
            ),
            (
                "actions/upload-pages-artifact@fc324d3547104276b827a68afc52ff2a11cc49c9",
                "actions/upload-pages-artifact@v5",
            ),
            (
                "actions/deploy-pages@cd2ce8fcbc39b97be8ca5fce6e763baed58fa128",
                "actions/deploy-pages@v5",
            ),
            ("toolchain: 1.97.1", "toolchain: stable"),
            ("targets: wasm32-unknown-unknown", "targets: wasm32-wasip1"),
            ("EMSDK_VERSION: \"6.0.3\"", "EMSDK_VERSION: latest"),
            (
                "EMSDK_REVISION: \"db04e88298d9916fc51fcd3743045ca3eb695127\"",
                "EMSDK_REVISION: main",
            ),
            (
                "cargo install wasm-bindgen-cli --version 0.2.126 --locked",
                "cargo install wasm-bindgen-cli --locked",
            ),
            ("node-version: \"22.16.0\"", "node-version: latest"),
            ("npm ci --ignore-scripts", "npm install"),
            (
                "npx playwright install --with-deps chromium",
                "npx playwright install chromium",
            ),
            ("npm run test:pages-browser", "true # browser proof removed"),
            (
                "cargo run --locked -p xtask -- build-pages-wasm",
                "cargo run --locked -p xtask -- validate-pages",
            ),
            (
                "cargo run --locked -p xtask -- validate-pages",
                "true # validation removed",
            ),
            ("path: docs/pages", "path: docs"),
            ("needs: build", "needs: unreviewed-build"),
            (
                "if: github.ref == 'refs/heads/main' && github.ref_protected == true",
                "if: github.ref == 'refs/heads/main'",
            ),
            ("pages: write", "pages: read"),
            ("id-token: write", "id-token: read"),
            ("name: github-pages", "name: unprotected-pages"),
        ] {
            assert!(source.contains(reviewed), "missing fixture {reviewed:?}");
            let mutated = source.replacen(reviewed, drifted, 1);
            assert!(
                validate_pages_workflow_source(&mutated, &sdk).is_err(),
                "Pages workflow policy accepted drift of {reviewed:?}"
            );
        }

        let extra_trigger = source.replacen(
            "  workflow_dispatch:\n",
            "  workflow_dispatch:\n    inputs:\n      unsafe:\n        required: false\n",
            1,
        );
        assert!(
            validate_pages_workflow_source(&extra_trigger, &sdk).is_err(),
            "Pages workflow policy accepted unreviewed dispatch inputs"
        );

        let extra_job = source.replacen(
            "jobs:\n  build:",
            "jobs:\n  unreviewed:\n    runs-on: ubuntu-latest\n    steps: []\n  build:",
            1,
        );
        assert!(
            validate_pages_workflow_source(&extra_job, &sdk).is_err(),
            "Pages workflow policy accepted an unreviewed job"
        );

        let injected_build_command = source.replacen(
            "          set -euo pipefail",
            "          set -euo pipefail\n          true # unreviewed command",
            1,
        );
        assert!(
            validate_pages_workflow_source(&injected_build_command, &sdk).is_err(),
            "Pages workflow policy accepted an unreviewed Emscripten setup command"
        );

        let privileged_build = source.replacen(
            "    env:\n      EMSDK_VERSION:",
            "    permissions:\n      contents: write\n    env:\n      EMSDK_VERSION:",
            1,
        );
        assert!(
            validate_pages_workflow_source(&privileged_build, &sdk).is_err(),
            "Pages workflow policy accepted build-job write permissions"
        );

        let mut drifted_sdk = sdk.clone();
        drifted_sdk.node_version = "22.17.0".to_owned();
        assert!(validate_pages_workflow_source(source, &drifted_sdk).is_err());
    }

    #[test]
    fn ci_workflow_rejects_verification_contract_drift() {
        let source = include_str!("../../../.github/workflows/ci.yml");
        assert!(validate_ci_workflow_source(source).is_ok());

        for required in [
            "cargo run --locked -p xtask -- verify-precision-contract",
            "cargo nextest run --locked --workspace",
            "cargo test --locked -p boxdd-sys --features package-bin --bin package",
            "cargo test --locked --target-dir target/package-helper-double -p boxdd-sys --features \"package-bin,double-precision,validate,disable-simd\" --bin package",
            "os: [ubuntu-latest, macos-latest, windows-latest]",
            "cargo nextest run --locked --target-dir target/core-double -p boxdd -p boxdd-sys --features boxdd/double-precision",
            "cargo test --locked --target-dir target/abi-probe-double -p boxdd-abi-probe --test abi --no-default-features --features double-precision",
            "cargo clippy --locked -p boxdd --all-targets --features \"double-precision serde mint nalgebra glam bytemuck unchecked validate disable-simd\" -- -D warnings",
            "cargo nextest run --locked --target-dir target/serde-double -p boxdd --test serde_values --features \"double-precision serde\"",
            "cargo nextest run --locked --target-dir target/interops-double -p boxdd --test mint_interop --test nalgebra_interop --test glam_interop --test bytemuck_api --features \"double-precision mint nalgebra glam bytemuck\"",
            "cargo check --locked -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed",
            "cargo clippy --locked -p boxdd-sys --features \"package-bin,double-precision,validate,disable-simd\" --bin package -- -D warnings",
            "toolchain: [\"1.95.0\", \"1.97.1\"]",
            "precision: [single, double]",
            "CARGO_TARGET_DIR=\"$source_target\" cargo +${{ matrix.toolchain }} build --locked -p boxdd-sys --features \"${{ matrix.precision == 'double' && 'double-precision' || '' }}\" --quiet",
            "CARGO_TARGET_DIR=\"$attest_target\" cargo +${{ matrix.toolchain }} run --locked -p boxdd-sys --features \"${{ matrix.precision == 'double' && 'package-bin,double-precision' || 'package-bin' }}\" --bin package -- attest-local-system \"$SYS_DIR/libbox2d.a\" \"$SYS_DIR/box2d.h\" \"$SYS_DIR/bindings.rs\" \"$SYS_DIR/manifest.toml\"",
            SYSTEM_QUALIFICATION_COMMAND,
            "cargo doc --locked --no-deps -p bevy_boxdd --features double-precision",
        ] {
            assert!(
                source.contains(required),
                "missing test fixture {required:?}"
            );
            let drifted = source.replacen(required, "true # required CI gate removed", 1);
            assert!(
                validate_ci_workflow_source(&drifted).is_err(),
                "CI policy accepted removal of {required:?}"
            );
        }

        for (double_command, single_features) in [
            (
                "--features \"double-precision serde\"",
                "--features \"serde\"",
            ),
            (
                "--features \"double-precision mint nalgebra glam bytemuck\"",
                "--features \"mint nalgebra glam bytemuck\"",
            ),
        ] {
            let drifted = source.replacen(double_command, single_features, 1);
            assert_ne!(drifted, source, "missing test fixture {double_command:?}");
            assert!(
                validate_ci_workflow_source(&drifted).is_err(),
                "CI policy accepted single-precision replacement for {double_command:?}"
            );
        }

        let abi_double = "cargo test --locked --target-dir target/abi-probe-double -p boxdd-abi-probe --test abi --no-default-features --features double-precision";
        let abi_single = "cargo test --locked --target-dir target/abi-probe-double -p boxdd-abi-probe --test abi --no-default-features";
        let drifted = source.replacen(abi_double, abi_single, 1);
        assert_ne!(drifted, source, "missing test fixture {abi_double:?}");
        assert!(
            validate_ci_workflow_source(&drifted).is_err(),
            "CI policy accepted a single-precision ABI probe"
        );

        let commented = source.replacen(
            "          cargo nextest run --locked --workspace",
            "          # cargo nextest run --locked --workspace",
            1,
        );
        assert!(
            validate_ci_workflow_source(&commented).is_err(),
            "CI policy accepted a required gate hidden in a comment"
        );

        let disabled = source.replacen(
            "      - name: Workspace integration\n        run: |",
            "      - name: Workspace integration\n        if: ${{ false }}\n        run: |",
            1,
        );
        assert!(
            validate_ci_workflow_source(&disabled).is_err(),
            "CI policy accepted a required gate in a disabled step"
        );

        let echoed = source.replacen(
            "          cargo nextest run --locked --workspace",
            "          echo cargo nextest run --locked --workspace",
            1,
        );
        assert!(
            validate_ci_workflow_source(&echoed).is_err(),
            "CI policy accepted a required gate that only gets echoed"
        );

        let privileged = source.replacen(
            "    permissions:\n      contents: read\n    steps:",
            "    permissions:\n      contents: read\n      packages: write\n    steps:",
            1,
        );
        assert!(
            validate_ci_workflow_source(&privileged).is_err(),
            "CI policy accepted an undeclared write permission"
        );

        let swallowed = source.replacen(
            "          cargo nextest run --locked --workspace",
            "          cargo nextest run --locked --workspace || true",
            1,
        );
        assert!(
            validate_ci_workflow_source(&swallowed).is_err(),
            "CI policy accepted a required gate whose failure is swallowed"
        );

        let continue_on_error = source.replacen(
            "      - name: Workspace integration\n        run: |",
            "      - name: Workspace integration\n        continue-on-error: true\n        run: |",
            1,
        );
        assert!(
            validate_ci_workflow_source(&continue_on_error).is_err(),
            "CI policy accepted continue-on-error on a required gate"
        );

        let provider_bypass = source.replacen("--provider system", "--provider vendored", 1);
        assert!(
            validate_ci_workflow_source(&provider_bypass).is_err(),
            "CI policy accepted a double consumer that bypasses the attested system provider"
        );

        let missing_msrv = source.replacen(
            "toolchain: [\"1.95.0\", \"1.97.1\"]",
            "toolchain: [\"1.97.1\"]",
            1,
        );
        assert!(
            validate_ci_workflow_source(&missing_msrv).is_err(),
            "CI policy accepted a system provider matrix without Rust 1.95"
        );

        let missing_double =
            source.replacen("precision: [single, double]", "precision: [single]", 1);
        assert!(
            validate_ci_workflow_source(&missing_double).is_err(),
            "CI policy accepted a single-only system provider matrix"
        );

        let excluded_system_coordinate = source.replacen(
            "        precision: [single, double]\n    steps:",
            "        precision: [single, double]\n        exclude:\n          - toolchain: \"1.95.0\"\n            precision: double\n    steps:",
            1,
        );
        assert!(
            validate_ci_workflow_source(&excluded_system_coordinate).is_err(),
            "CI policy accepted an excluded system provider coordinate"
        );

        let helper_continue_on_error = source.replacen(
            "      - name: Qualify the freshly packaged crate against the system artifact\n        run:",
            "      - name: Qualify the freshly packaged crate against the system artifact\n        continue-on-error: true\n        run:",
            1,
        );
        assert!(
            validate_ci_workflow_source(&helper_continue_on_error).is_err(),
            "CI policy accepted continue-on-error on the native helper"
        );

        let default_outer_cargo = source.replacen(
            SYSTEM_QUALIFICATION_COMMAND,
            SYSTEM_QUALIFICATION_COMMAND
                .replacen("cargo +${{ matrix.toolchain }} run", "cargo run", 1)
                .as_str(),
            1,
        );
        assert!(
            validate_ci_workflow_source(&default_outer_cargo).is_err(),
            "CI policy accepted the native helper through the default Cargo"
        );

        let default_helper_toolchain = source.replacen(
            "--toolchain ${{ matrix.toolchain }}",
            "--toolchain stable",
            1,
        );
        assert!(
            validate_ci_workflow_source(&default_helper_toolchain).is_err(),
            "CI policy accepted a native helper using the default toolchain"
        );

        let dirty_package = source.replacen(
            "--artifacts \"${{ runner.temp }}/boxdd-system-artifact\"",
            "--artifacts \"${{ runner.temp }}/boxdd-system-artifact\" --allow-dirty",
            1,
        );
        assert!(
            validate_ci_workflow_source(&dirty_package).is_err(),
            "CI policy accepted --allow-dirty in native qualification"
        );

        let disabled_helper = source.replacen(
            "      - name: Qualify the freshly packaged crate against the system artifact\n        run:",
            "      - name: Qualify the freshly packaged crate against the system artifact\n        if: ${{ false }}\n        run:",
            1,
        );
        assert!(
            validate_ci_workflow_source(&disabled_helper).is_err(),
            "CI policy accepted a disabled native helper"
        );

        let wrapped_helper = source.replacen(
            SYSTEM_QUALIFICATION_COMMAND,
            format!("bash -c '{SYSTEM_QUALIFICATION_COMMAND}'").as_str(),
            1,
        );
        assert!(
            validate_ci_workflow_source(&wrapped_helper).is_err(),
            "CI policy accepted a shell-wrapped native helper"
        );

        let unreviewed_condition = source.replacen(
            "      - name: Workspace integration\n        run: |",
            "      - name: Workspace integration\n        if: ${{ 1 == 2 }}\n        run: |",
            1,
        );
        assert!(
            validate_ci_workflow_source(&unreviewed_condition).is_err(),
            "CI policy accepted an unreviewed condition on a required gate"
        );

        let early_exit = source.replacen(
            "        run: |\n          cargo check --locked --workspace --all-targets",
            "        run: |\n          exit 0\n          cargo check --locked --workspace --all-targets",
            1,
        );
        assert!(
            validate_ci_workflow_source(&early_exit).is_err(),
            "CI policy accepted an early successful exit before a required gate"
        );

        let excluded_platforms = source.replacen(
            "        os: [ubuntu-latest, macos-latest, windows-latest]",
            "        os: [ubuntu-latest, macos-latest, windows-latest]\n        exclude:\n          - os: macos-latest\n          - os: windows-latest",
            1,
        );
        assert!(
            validate_ci_workflow_source(&excluded_platforms).is_err(),
            "CI policy accepted exclusions from the required native matrix"
        );

        let linux_only_native_tests = source.replacen(
            "      - name: Native source tests (single and double precision)\n        run: |",
            "      - name: Native source tests (single and double precision)\n        if: runner.os == 'Linux'\n        run: |",
            1,
        );
        assert!(
            validate_ci_workflow_source(&linux_only_native_tests).is_err(),
            "CI policy accepted Linux-only native tests as a three-platform gate"
        );

        let quoted_write = source.replacen(
            "    permissions:\n      contents: read\n    steps:",
            "    permissions:\n      contents: read\n      packages: \"write\"\n    steps:",
            1,
        );
        assert!(
            validate_ci_workflow_source(&quoted_write).is_err(),
            "CI policy accepted a quoted write permission"
        );

        let missing_pull_request =
            source.replacen("  pull_request:\n    branches: [ main, master ]\n", "", 1);
        assert!(
            validate_ci_workflow_source(&missing_pull_request).is_err(),
            "CI policy accepted removal of the pull-request trigger"
        );

        let path_filtered = source.replacen(
            "  pull_request:\n    branches: [ main, master ]",
            "  pull_request:\n    branches: [ main, master ]\n    paths: [ docs/** ]",
            1,
        );
        assert!(
            validate_ci_workflow_source(&path_filtered).is_err(),
            "CI policy accepted a path-filtered pull-request trigger"
        );
    }

    #[test]
    fn audit_policy_requires_exception_file_to_remain_absent() {
        let directory = tempfile::tempdir().unwrap();
        assert!(validate_audit_policy(directory.path()).is_ok());

        let cargo = directory.path().join(".cargo");
        fs::create_dir(&cargo).unwrap();
        fs::write(cargo.join("audit.toml"), b"[advisories]\nignore = []\n").unwrap();
        assert!(validate_audit_policy(directory.path()).is_err());
    }

    #[test]
    fn release_workflow_rejects_topology_and_privilege_drift() {
        let source = include_str!("../../../.github/workflows/prebuilt-binaries.yml");
        assert!(validate_release_workflow_source(source).is_ok());

        let wrong_dependency = source.replace("needs: aggregate", "needs: build-prebuilt");
        assert!(validate_release_workflow_source(&wrong_dependency).is_err());

        let privileged_attest = source.replace(
            "contents: read\n      id-token: write",
            "contents: write\n      id-token: write",
        );
        assert!(validate_release_workflow_source(&privileged_attest).is_err());

        let missing_tag_binding = source.replace(
            "test \"${object_sha}\" = \"${GITHUB_SHA}\"",
            "true # tag identity check removed",
        );
        assert!(validate_release_workflow_source(&missing_tag_binding).is_err());

        let missing_ref_binding = source.replace(
            "test \"${GITHUB_REF}\" = \"refs/tags/${GITHUB_REF_NAME}\"",
            "true # workflow ref check removed",
        );
        assert!(validate_release_workflow_source(&missing_ref_binding).is_err());

        let missing_annotation_peel =
            source.replace("git/tags/${object_sha}", "git/commits/${object_sha}");
        assert!(validate_release_workflow_source(&missing_annotation_peel).is_err());

        let wrong_platform_runner = source.replacen("os: macos-latest", "os: ubuntu-latest", 1);
        assert!(
            validate_release_workflow_source(&wrong_platform_runner).is_err(),
            "release policy accepted a target on the wrong runner"
        );

        let missing_msrv = source.replacen(
            "toolchain: [\"1.95.0\", \"1.97.1\"]",
            "toolchain: [\"1.97.1\"]",
            1,
        );
        assert!(
            validate_release_workflow_source(&missing_msrv).is_err(),
            "release policy accepted prebuilt qualification without Rust 1.95"
        );

        let excluded_msrv = source.replacen(
            "        toolchain: [\"1.95.0\", \"1.97.1\"]\n        precision: [single, double]",
            "        toolchain: [\"1.95.0\", \"1.97.1\"]\n        precision: [single, double]\n        exclude:\n          - toolchain: \"1.95.0\"",
            1,
        );
        assert!(
            validate_release_workflow_source(&excluded_msrv).is_err(),
            "release policy accepted an excluded Rust 1.95 qualification coordinate"
        );

        let included_override = source.replacen(
            "        toolchain: [\"1.95.0\", \"1.97.1\"]\n        precision: [single, double]",
            "        toolchain: [\"1.95.0\", \"1.97.1\"]\n        precision: [single, double]\n        include:\n          - toolchain: \"stable\"\n            precision: single",
            1,
        );
        assert!(
            validate_release_workflow_source(&included_override).is_err(),
            "release policy accepted an included qualification override"
        );

        let default_outer_cargo = source.replacen(
            PREBUILT_QUALIFICATION_COMMAND,
            PREBUILT_QUALIFICATION_COMMAND
                .replacen("cargo +${{ matrix.toolchain }} run", "cargo run", 1)
                .as_str(),
            1,
        );
        assert!(
            validate_release_workflow_source(&default_outer_cargo).is_err(),
            "release policy accepted the prebuilt helper through default Cargo"
        );

        let default_helper_toolchain = source.replacen(
            "--toolchain ${{ matrix.toolchain }}",
            "--toolchain stable",
            1,
        );
        assert!(
            validate_release_workflow_source(&default_helper_toolchain).is_err(),
            "release policy accepted a default helper toolchain"
        );

        let missing_helper = source.replacen(
            PREBUILT_QUALIFICATION_COMMAND,
            "true # qualification helper removed",
            1,
        );
        assert!(
            validate_release_workflow_source(&missing_helper).is_err(),
            "release policy accepted a missing qualification helper"
        );

        let step_condition = source.replacen(
            "      - name: Consume through the authenticated prebuilt provider\n        run:",
            "      - name: Consume through the authenticated prebuilt provider\n        if: ${{ false }}\n        run:",
            1,
        );
        assert!(
            validate_release_workflow_source(&step_condition).is_err(),
            "release policy accepted a conditional qualification helper"
        );

        let step_continue = source.replacen(
            "      - name: Consume through the authenticated prebuilt provider\n        run:",
            "      - name: Consume through the authenticated prebuilt provider\n        continue-on-error: true\n        run:",
            1,
        );
        assert!(
            validate_release_workflow_source(&step_continue).is_err(),
            "release policy accepted continue-on-error on the qualification step"
        );

        let job_continue = source.replacen(
            "    if: ${{ github.ref_protected == true }}\n    needs: verify-signed-release",
            "    if: ${{ github.ref_protected == true }}\n    continue-on-error: true\n    needs: verify-signed-release",
            1,
        );
        assert!(
            validate_release_workflow_source(&job_continue).is_err(),
            "release policy accepted continue-on-error on the qualification job"
        );

        let wrapped_helper = source.replacen(
            PREBUILT_QUALIFICATION_COMMAND,
            format!("pwsh -Command '{PREBUILT_QUALIFICATION_COMMAND}'").as_str(),
            1,
        );
        assert!(
            validate_release_workflow_source(&wrapped_helper).is_err(),
            "release policy accepted a shell-wrapped qualification helper"
        );

        let extra_run_step = source.replacen(
            PREBUILT_QUALIFICATION_COMMAND,
            format!(
                "{PREBUILT_QUALIFICATION_COMMAND}\n\n      - name: Hide an override\n        run: echo CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=true"
            )
            .as_str(),
            1,
        );
        assert!(
            validate_release_workflow_source(&extra_run_step).is_err(),
            "release policy accepted an extra run step beside the qualification helper"
        );

        let selected_shell = source.replacen(
            "      - name: Consume through the authenticated prebuilt provider\n        run:",
            "      - name: Consume through the authenticated prebuilt provider\n        shell: pwsh\n        run:",
            1,
        );
        assert!(
            validate_release_workflow_source(&selected_shell).is_err(),
            "release policy accepted a shell selection around the cross-platform helper"
        );

        let dirty_helper = source.replacen("--cosign cosign", "--cosign cosign --allow-dirty", 1);
        assert!(
            validate_release_workflow_source(&dirty_helper).is_err(),
            "release policy accepted --allow-dirty in release qualification"
        );

        let fixed_install =
            source.replacen("toolchain: ${{ matrix.toolchain }}", "toolchain: 1.97.1", 1);
        assert!(
            validate_release_workflow_source(&fixed_install).is_err(),
            "release policy accepted a fixed qualification toolchain installer"
        );
    }

    #[test]
    fn release_security_fields_cannot_be_forged_by_comments_or_wrong_nesting() {
        let source = include_str!("../../../.github/workflows/prebuilt-binaries.yml");
        assert!(validate_release_workflow_source(source).is_ok());

        let commented_publish_condition = source.replacen(
            "  publish-draft:\n    name: Publish protected draft release\n    if: ${{ github.ref_protected == true }}\n    needs: qualify-prebuilt",
            "  publish-draft:\n    name: Publish protected draft release\n    # if: ${{ github.ref_protected == true }}\n    needs: qualify-prebuilt",
            1,
        );
        assert!(
            validate_release_workflow_source(&commented_publish_condition).is_err(),
            "release policy accepted a commented protected-ref condition"
        );

        let step_scoped_publish_needs = source
            .replacen(
                "    needs: qualify-prebuilt\n    runs-on: ubuntu-latest",
                "    runs-on: ubuntu-latest",
                1,
            )
            .replacen(
                "      - name: Download qualified signed aggregate\n        uses:",
                "      - name: Download qualified signed aggregate\n        needs: qualify-prebuilt\n        uses:",
                1,
            );
        assert!(
            validate_release_workflow_source(&step_scoped_publish_needs).is_err(),
            "release policy accepted publish needs nested under a step"
        );

        let step_scoped_publish_permissions = source
            .replacen(
                "    permissions:\n      contents: write\n    steps:\n      - name: Download qualified signed aggregate",
                "    steps:\n      - name: Download qualified signed aggregate\n        permissions:\n          contents: write",
                1,
            );
        assert!(
            validate_release_workflow_source(&step_scoped_publish_permissions).is_err(),
            "release policy accepted publish permissions nested under a step"
        );

        let step_scoped_attest_environment = source
            .replacen("    environment: release\n    steps:", "    steps:", 1)
            .replacen(
                "      - name: Download validated attestation input\n        uses:",
                "      - name: Download validated attestation input\n        environment: release\n        uses:",
                1,
            );
        assert!(
            validate_release_workflow_source(&step_scoped_attest_environment).is_err(),
            "release policy accepted attest environment nested under a step"
        );

        let unexpected_publish_environment = source.replacen(
            "    permissions:\n      contents: write\n    steps:",
            "    permissions:\n      contents: write\n    environment: release\n    steps:",
            1,
        );
        assert!(
            validate_release_workflow_source(&unexpected_publish_environment).is_err(),
            "release policy did not bind publish environment absence"
        );
    }

    #[test]
    fn qualification_helpers_must_be_direct_unconditional_run_steps() {
        let release = include_str!("../../../.github/workflows/prebuilt-binaries.yml");
        let nested_prebuilt_helper = release.replacen(
            format!(
                "      - name: Consume through the authenticated prebuilt provider\n        run: {PREBUILT_QUALIFICATION_COMMAND}"
            )
            .as_str(),
            format!(
                "      - name: Consume through the authenticated prebuilt provider\n        uses: example.invalid/forgery@deadbeef\n        with:\n          run: {PREBUILT_QUALIFICATION_COMMAND}"
            )
            .as_str(),
            1,
        );
        assert!(
            validate_release_workflow_source(&nested_prebuilt_helper).is_err(),
            "release policy accepted a helper string nested under action inputs"
        );

        let commented_prebuilt_helper = release.replacen(
            format!("        run: {PREBUILT_QUALIFICATION_COMMAND}").as_str(),
            format!("        run: \"true\"\n        # run: {PREBUILT_QUALIFICATION_COMMAND}")
                .as_str(),
            1,
        );
        assert!(
            validate_release_workflow_source(&commented_prebuilt_helper).is_err(),
            "release policy accepted a helper command present only in a comment"
        );

        let commented_matrix_toolchain = release.replacen(
            "          toolchain: ${{ matrix.toolchain }}\n          targets: ${{ matrix.platform.target }}",
            "          toolchain: 1.97.1\n          # toolchain: ${{ matrix.toolchain }}\n          targets: ${{ matrix.platform.target }}",
            1,
        );
        assert!(
            validate_release_workflow_source(&commented_matrix_toolchain).is_err(),
            "release policy accepted the matrix toolchain only in a comment"
        );

        let ci = include_str!("../../../.github/workflows/ci.yml");
        let nested_system_helper = ci.replacen(
            format!(
                "      - name: Qualify the freshly packaged crate against the system artifact\n        run: {SYSTEM_QUALIFICATION_COMMAND}"
            )
            .as_str(),
            format!(
                "      - name: Qualify the freshly packaged crate against the system artifact\n        uses: example.invalid/forgery@deadbeef\n        with:\n          run: {SYSTEM_QUALIFICATION_COMMAND}"
            )
            .as_str(),
            1,
        );
        assert!(
            validate_ci_workflow_source(&nested_system_helper).is_err(),
            "CI policy accepted a helper string nested under action inputs"
        );
    }

    #[test]
    fn protected_workflows_reject_execution_context_injection() {
        let release = include_str!("../../../.github/workflows/prebuilt-binaries.yml");
        assert!(validate_release_workflow_source(release).is_ok());

        let workflow_defaults = release.replacen(
            "permissions: {}\n\nenv:",
            "permissions: {}\n\ndefaults:\n  run:\n    shell: bash\n\nenv:",
            1,
        );
        assert!(
            validate_release_workflow_source(&workflow_defaults).is_err(),
            "release policy accepted workflow-level shell defaults"
        );

        let qualification_job_env = release.replacen(
            "    needs: verify-signed-release\n    runs-on:",
            "    needs: verify-signed-release\n    env:\n      PATH: /tmp/fake-cargo\n    runs-on:",
            1,
        );
        assert!(
            validate_release_workflow_source(&qualification_job_env).is_err(),
            "release policy accepted a qualification PATH override"
        );

        let qualification_job_defaults = release.replacen(
            "    needs: verify-signed-release\n    runs-on:",
            "    needs: verify-signed-release\n    defaults:\n      run:\n        shell: bash\n    runs-on:",
            1,
        );
        assert!(
            validate_release_workflow_source(&qualification_job_defaults).is_err(),
            "release policy accepted qualification-job shell defaults"
        );

        let qualification_step_env = release.replacen(
            "      - name: Consume through the authenticated prebuilt provider\n        run:",
            "      - name: Consume through the authenticated prebuilt provider\n        env:\n          PATH: /tmp/fake-cargo\n        run:",
            1,
        );
        assert!(
            validate_release_workflow_source(&qualification_step_env).is_err(),
            "release policy accepted a helper-step PATH override"
        );

        let qualification_working_directory = release.replacen(
            "      - name: Consume through the authenticated prebuilt provider\n        run:",
            "      - name: Consume through the authenticated prebuilt provider\n        working-directory: /tmp/forged-checkout\n        run:",
            1,
        );
        assert!(
            validate_release_workflow_source(&qualification_working_directory).is_err(),
            "release policy accepted a helper working-directory override"
        );

        let extra_attest_action = release.replacen(
            "    environment: release\n    steps:\n      - name: Download validated attestation input",
            "    environment: release\n    steps:\n      - name: Unreviewed action\n        uses: example.invalid/action@deadbeef\n\n      - name: Download validated attestation input",
            1,
        );
        assert!(
            validate_release_workflow_source(&extra_attest_action).is_err(),
            "release policy accepted an unreviewed attest action"
        );

        let extra_publish_action = release.replacen(
            "    permissions:\n      contents: write\n    steps:\n      - name: Download qualified signed aggregate",
            "    permissions:\n      contents: write\n    steps:\n      - name: Unreviewed publish action\n        uses: example.invalid/action@deadbeef\n\n      - name: Download qualified signed aggregate",
            1,
        );
        assert!(
            validate_release_workflow_source(&extra_publish_action).is_err(),
            "release policy accepted an unreviewed publish-path job"
        );

        let changed_attest_run = release.replacen(
            "          root=\"${RUNNER_TEMP}/attestation-input/signing-payloads/trusted_root.json\"",
            "          true # unreviewed command\n          root=\"${RUNNER_TEMP}/attestation-input/signing-payloads/trusted_root.json\"",
            1,
        );
        assert!(
            validate_release_workflow_source(&changed_attest_run).is_err(),
            "release policy accepted an unreviewed attest command"
        );

        let changed_publish_run = release.replacen(
            "          test \"${GITHUB_REF_TYPE}\" = \"tag\"",
            "          true # unreviewed command\n          test \"${GITHUB_REF_TYPE}\" = \"tag\"",
            1,
        );
        assert!(
            validate_release_workflow_source(&changed_publish_run).is_err(),
            "release policy accepted an unreviewed publish command"
        );

        let ci = include_str!("../../../.github/workflows/ci.yml");
        assert!(validate_ci_workflow_source(ci).is_ok());
        let extra_system_action = ci.replacen(
            "        precision: [single, double]\n    steps:\n      - name: Checkout",
            "        precision: [single, double]\n    steps:\n      - name: Unreviewed provider action\n        uses: example.invalid/action@deadbeef\n\n      - name: Checkout",
            1,
        );
        assert!(
            validate_ci_workflow_source(&extra_system_action).is_err(),
            "CI policy accepted an unreviewed provider-path job"
        );
    }
}
