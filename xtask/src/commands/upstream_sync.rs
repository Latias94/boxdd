use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    io::Write as _,
    path::{Component, Path, PathBuf},
    process::{Command, Output},
};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::{
    Error, Result, bindgen_contract,
    commands::{
        UpdateMode, parse_update_mode,
        support::{QualifiedCargo, WASM_TARGET},
    },
    config::{read_toml, render_toml, write_atomic},
    isolated_git::{isolated_git_command, repository_lock_path},
    paths::WorkspacePaths,
    provider_catalog::ProviderCapability,
    provider_manifest::{
        ADAPTER_ABI_VERSION, RECORDING_CONTRACT_BLAKE3, sha256_bytes,
        validate_recording_contract_blake3,
    },
    recording_ops,
    recording_wire::REVIEWED_RECORDING_INPUT_PATHS,
    source_overlay::{
        EffectiveSourceIdentity, UPSTREAM_MANIFEST_SCHEMA, UPSTREAM_REPOSITORY,
        adapter_source_sha256, effective_source_file_sha256s, effective_source_identity,
    },
    subprocess_policy::run_output,
    wasm_provider_contract::{
        COMPILER_TARGET, ENDIANNESS, POINTER_WIDTH, PROVIDER_ABI, SIMD_MODE,
        WasmProviderExpectation, WasmProviderIdentity, contract_relative_path,
    },
};

const BOX2D_GITLINK: &str = "boxdd-sys/third-party/box2d";
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInventory {
    pub tree: String,
    pub c_sources: Vec<String>,
    pub private_headers: Vec<String>,
    pub inline_files: Vec<String>,
    pub public_headers: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    Bindings,
    RecordingWire,
    ProviderIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactProducer {
    Bindgen,
    RecordingCodegen,
    ProviderAttestation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Precision {
    Single,
    Double,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactTarget {
    Universal,
    Wasm32UnknownUnknown,
    Wasm32Wasip1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactProvider {
    Universal,
    WasmRuntime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
enum RustTarget {
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    X86_64UnknownLinuxGnu,
    #[serde(rename = "wasm32-unknown-unknown")]
    Wasm32UnknownUnknown,
    #[serde(rename = "wasm32-wasip1")]
    Wasm32Wasip1,
}

impl ArtifactProducer {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bindgen => "bindgen",
            Self::RecordingCodegen => "recording-codegen",
            Self::ProviderAttestation => "provider-attestation",
        }
    }
}

impl Precision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }
}

impl ArtifactTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::Wasm32UnknownUnknown => "wasm32-unknown-unknown",
            Self::Wasm32Wasip1 => "wasm32-wasip1",
        }
    }
}

impl ArtifactProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::WasmRuntime => "wasm-runtime",
        }
    }
}

impl RustTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
            Self::Wasm32UnknownUnknown => "wasm32-unknown-unknown",
            Self::Wasm32Wasip1 => "wasm32-wasip1",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedArtifact {
    pub name: String,
    pub kind: ArtifactKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<Precision>,
    pub target: ArtifactTarget,
    pub provider: ArtifactProvider,
    pub producer: ArtifactProducer,
    pub content_blake3: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingInputIdentity {
    pub path: String,
    pub git_blob: String,
    pub blake3: String,
    pub effective_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamManifest {
    pub schema_version: u32,
    pub repository: String,
    pub active_revision: String,
    pub recording_revision: String,
    pub recording_inputs: Vec<RecordingInputIdentity>,
    pub artifacts: Vec<GeneratedArtifact>,
    pub source_inventory: SourceInventory,
}

impl UpstreamManifest {
    pub fn load(paths: &WorkspacePaths) -> Result<Self> {
        let path = paths.upstream_manifest();
        let manifest: Self = read_toml(&path)?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }

    pub fn artifact(&self, kind: ArtifactKind) -> Result<&GeneratedArtifact> {
        let mut matching = self
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == kind);
        let artifact = matching
            .next()
            .ok_or_else(|| Error::message(format!("upstream manifest has no {kind:?} artifact")))?;
        if matching.next().is_some() {
            return Err(Error::message(format!(
                "upstream manifest has multiple {kind:?} artifacts; select one by its coordinates"
            )));
        }
        Ok(artifact)
    }

    pub fn artifact_path(&self, root: &Path, kind: ArtifactKind) -> Result<PathBuf> {
        Ok(root.join(&self.artifact(kind)?.path))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamSnapshot {
    pub active_revision: String,
    pub gitlink_revision: String,
    pub worktree_revision: String,
}

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    match parse_update_mode("upstream-sync", args)? {
        UpdateMode::Check => {
            let manifest = UpstreamManifest::load(paths)?;
            validate_repository(paths, &manifest)?;
            println!(
                "upstream manifest, Box2D submodule, recording inputs, and {} artifacts agree at {}",
                manifest.artifacts.len(),
                manifest.active_revision
            );
        }
        UpdateMode::Write => refresh_current_checkout(paths)?,
    }
    Ok(())
}

pub fn checked_snapshot(paths: &WorkspacePaths) -> Result<UpstreamSnapshot> {
    let manifest = UpstreamManifest::load(paths)?;
    require_provider_identity_topology(&manifest)?;
    validate_repository(paths, &manifest)
}

pub fn validate_repository(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<UpstreamSnapshot> {
    validate_manifest(manifest)?;
    require_provider_identity_topology(manifest)?;
    let snapshot = validate_repository_checkout(paths, manifest)?;
    validate_recording_input_identities(paths, manifest)?;
    validate_recording_operations(paths, manifest)?;
    validate_artifact_identities(paths, manifest)?;
    Ok(snapshot)
}

pub(crate) fn require_provider_identity_topology(manifest: &UpstreamManifest) -> Result<()> {
    let identities = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
        .collect::<Vec<_>>();
    let expected = [
        ("wasm-provider-identity-single", Precision::Single),
        ("wasm-provider-identity-double", Precision::Double),
    ];
    if identities.len() != expected.len()
        || expected.iter().any(|(name, precision)| {
            !identities.iter().any(|artifact| {
                artifact.name == *name
                    && artifact.precision == Some(*precision)
                    && artifact.target == ArtifactTarget::Wasm32UnknownUnknown
                    && artifact.provider == ArtifactProvider::WasmRuntime
                    && artifact.producer == ArtifactProducer::ProviderAttestation
            })
        })
    {
        return Err(Error::message(
            "upstream manifest must contain the canonical single and double WASM provider identities",
        ));
    }
    Ok(())
}

fn refresh_current_checkout(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let mut manifest = UpstreamManifest::load(paths)?;
    validate_checkout_revisions(paths, &manifest)?;

    manifest.source_inventory = source_inventory(&paths.box2d(), &manifest.active_revision)?;
    let effective_recording_inputs = effective_recording_input_sha256s(paths)?;
    manifest.recording_inputs = recording_input_identities(
        &paths.box2d(),
        &manifest.recording_revision,
        &effective_recording_inputs,
    )?;

    // The build script reads the reviewed source inventory. Publishing this small intermediate
    // state is intentional: Git keeps it visible and recoverable if a later tool fails.
    write_atomic(&paths.upstream_manifest(), &render_toml(&manifest)?)?;

    let generated_bindings = generate_bindings(paths, &manifest)?;
    for (path, content) in generated_bindings {
        write_atomic(&path, &content)?;
    }

    if provider_identities_need_refresh(paths.root(), &manifest)? {
        let sdk = super::provider::qualified_provider_sdk().map_err(|error| {
            Error::message(format!(
                "WASM provider identities are stale and must be regenerated with the pinned Emscripten SDK: {error}"
            ))
        })?;
        let target = paths.root().join("target/upstream-sync-provider");
        super::provider::refresh_wasm_provider_contracts_unlocked(paths.root(), &target, &sdk)?;
    }

    for artifact in &mut manifest.artifacts {
        artifact.content_blake3 = file_blake3(&paths.root().join(&artifact.path))?;
    }
    write_atomic(&paths.upstream_manifest(), &render_toml(&manifest)?)?;
    validate_repository(paths, &manifest)?;
    println!(
        "refreshed bindings, provider identities when required, and upstream manifest at {}",
        manifest.active_revision
    );
    Ok(())
}

fn validate_manifest(manifest: &UpstreamManifest) -> Result<()> {
    let mut errors = Vec::new();
    if manifest.schema_version != UPSTREAM_MANIFEST_SCHEMA {
        errors.push(format!(
            "schema {} is unsupported; expected {UPSTREAM_MANIFEST_SCHEMA}",
            manifest.schema_version
        ));
    }
    if manifest.repository != UPSTREAM_REPOSITORY {
        errors.push(format!(
            "repository must be the official Box2D repository `{UPSTREAM_REPOSITORY}`"
        ));
    }
    for (label, revision) in [
        ("active_revision", manifest.active_revision.as_str()),
        ("recording_revision", manifest.recording_revision.as_str()),
    ] {
        if !is_hex(revision, 40) {
            errors.push(format!("{label} must be a lowercase 40-character Git SHA"));
        }
    }
    validate_sorted_inventory(&manifest.source_inventory, &mut errors);
    validate_recording_input_topology(&manifest.recording_inputs, &mut errors);
    validate_artifact_topology(&manifest.artifacts, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "invalid upstream manifest:\n- {}",
            errors.join("\n- ")
        )))
    }
}

fn validate_sorted_inventory(inventory: &SourceInventory, errors: &mut Vec<String>) {
    if !is_hex(&inventory.tree, 40) {
        errors.push("source_inventory.tree must be a lowercase 40-character Git SHA".to_owned());
    }
    for (label, paths) in [
        ("c_sources", &inventory.c_sources),
        ("private_headers", &inventory.private_headers),
        ("inline_files", &inventory.inline_files),
        ("public_headers", &inventory.public_headers),
    ] {
        if !is_strictly_sorted(paths) {
            errors.push(format!(
                "source_inventory.{label} must be sorted and unique"
            ));
        }
        for path in paths {
            if validate_relative_path(path).is_err() {
                errors.push(format!(
                    "source_inventory.{label} contains invalid path `{path}`"
                ));
            }
        }
    }
}

fn validate_recording_input_topology(inputs: &[RecordingInputIdentity], errors: &mut Vec<String>) {
    let actual = inputs
        .iter()
        .map(|input| input.path.as_str())
        .collect::<Vec<_>>();
    if actual != REVIEWED_RECORDING_INPUT_PATHS {
        errors.push(format!(
            "recording_inputs must be the exact reviewed source list {REVIEWED_RECORDING_INPUT_PATHS:?}"
        ));
    }
    for input in inputs {
        if !is_hex(&input.git_blob, 40) {
            errors.push(format!(
                "recording input `{}` has an invalid Git blob",
                input.path
            ));
        }
        if !is_hex(&input.blake3, 64) {
            errors.push(format!(
                "recording input `{}` has an invalid BLAKE3",
                input.path
            ));
        }
        if !is_hex(&input.effective_sha256, 64) {
            errors.push(format!(
                "recording input `{}` has an invalid effective SHA-256",
                input.path
            ));
        }
    }
}

#[derive(Clone, Copy)]
struct ArtifactSpec {
    name: &'static str,
    kind: ArtifactKind,
    path: &'static str,
    precision: Option<Precision>,
    target: ArtifactTarget,
    provider: ArtifactProvider,
    producer: ArtifactProducer,
}

const ARTIFACT_SPECS: &[ArtifactSpec] = &[
    ArtifactSpec {
        name: "bindings-single",
        kind: ArtifactKind::Bindings,
        path: "boxdd-sys/src/bindings_pregenerated.rs",
        precision: Some(Precision::Single),
        target: ArtifactTarget::Universal,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::Bindgen,
    },
    ArtifactSpec {
        name: "bindings-wasm32-unknown-unknown-single",
        kind: ArtifactKind::Bindings,
        path: "boxdd-sys/src/bindings_wasm32_unknown_unknown.rs",
        precision: Some(Precision::Single),
        target: ArtifactTarget::Wasm32UnknownUnknown,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::Bindgen,
    },
    ArtifactSpec {
        name: "bindings-wasm32-wasip1-single",
        kind: ArtifactKind::Bindings,
        path: "boxdd-sys/src/bindings_wasm32_wasip1.rs",
        precision: Some(Precision::Single),
        target: ArtifactTarget::Wasm32Wasip1,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::Bindgen,
    },
    ArtifactSpec {
        name: "bindings-double",
        kind: ArtifactKind::Bindings,
        path: "boxdd-sys/src/bindings_double.rs",
        precision: Some(Precision::Double),
        target: ArtifactTarget::Universal,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::Bindgen,
    },
    ArtifactSpec {
        name: "bindings-wasm32-unknown-unknown-double",
        kind: ArtifactKind::Bindings,
        path: "boxdd-sys/src/bindings_wasm32_unknown_unknown_double.rs",
        precision: Some(Precision::Double),
        target: ArtifactTarget::Wasm32UnknownUnknown,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::Bindgen,
    },
    ArtifactSpec {
        name: "bindings-wasm32-wasip1-double",
        kind: ArtifactKind::Bindings,
        path: "boxdd-sys/src/bindings_wasm32_wasip1_double.rs",
        precision: Some(Precision::Double),
        target: ArtifactTarget::Wasm32Wasip1,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::Bindgen,
    },
    ArtifactSpec {
        name: "recording-wire",
        kind: ArtifactKind::RecordingWire,
        path: "xtask/tests/fixtures/recording_wire_contract.toml",
        precision: None,
        target: ArtifactTarget::Universal,
        provider: ArtifactProvider::Universal,
        producer: ArtifactProducer::RecordingCodegen,
    },
    ArtifactSpec {
        name: "wasm-provider-identity-single",
        kind: ArtifactKind::ProviderIdentity,
        path: "boxdd-sys/abi/wasm32-unknown-unknown-single.toml",
        precision: Some(Precision::Single),
        target: ArtifactTarget::Wasm32UnknownUnknown,
        provider: ArtifactProvider::WasmRuntime,
        producer: ArtifactProducer::ProviderAttestation,
    },
    ArtifactSpec {
        name: "wasm-provider-identity-double",
        kind: ArtifactKind::ProviderIdentity,
        path: "boxdd-sys/abi/wasm32-unknown-unknown-double.toml",
        precision: Some(Precision::Double),
        target: ArtifactTarget::Wasm32UnknownUnknown,
        provider: ArtifactProvider::WasmRuntime,
        producer: ArtifactProducer::ProviderAttestation,
    },
];

fn validate_artifact_topology(artifacts: &[GeneratedArtifact], errors: &mut Vec<String>) {
    let names = artifacts
        .iter()
        .map(|artifact| artifact.name.as_str())
        .collect::<BTreeSet<_>>();
    let paths = artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    if names.len() != artifacts.len() {
        errors.push("artifact names must be unique".to_owned());
    }
    if paths.len() != artifacts.len() {
        errors.push("artifact paths must be unique".to_owned());
    }
    if artifacts.len() != ARTIFACT_SPECS.len() {
        errors.push(format!(
            "artifact topology must contain exactly {} entries, observed {}",
            ARTIFACT_SPECS.len(),
            artifacts.len()
        ));
    }
    for spec in ARTIFACT_SPECS {
        match artifacts.iter().find(|artifact| artifact.name == spec.name) {
            None => errors.push(format!("missing canonical artifact `{}`", spec.name)),
            Some(artifact)
                if artifact.kind != spec.kind
                    || artifact.path != spec.path
                    || artifact.precision != spec.precision
                    || artifact.target != spec.target
                    || artifact.provider != spec.provider
                    || artifact.producer != spec.producer =>
            {
                errors.push(format!(
                    "artifact `{}` has non-canonical coordinates",
                    spec.name
                ));
            }
            Some(_) => {}
        }
    }
    for artifact in artifacts {
        if validate_relative_path(&artifact.path).is_err() {
            errors.push(format!(
                "artifact `{}` has invalid path `{}`",
                artifact.name, artifact.path
            ));
        }
        if !is_hex(&artifact.content_blake3, 64) {
            errors.push(format!(
                "artifact `{}` has an invalid BLAKE3",
                artifact.name
            ));
        }
    }
}

fn validate_repository_checkout(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<UpstreamSnapshot> {
    let (gitlink_revision, worktree_revision) = validate_checkout_revisions(paths, manifest)?;
    let actual_inventory = source_inventory(&paths.box2d(), &manifest.active_revision)?;
    if actual_inventory != manifest.source_inventory {
        return Err(Error::message(format!(
            "Box2D source inventory differs from boxdd-sys/upstream.toml at {}",
            manifest.active_revision
        )));
    }
    Ok(UpstreamSnapshot {
        active_revision: manifest.active_revision.clone(),
        gitlink_revision,
        worktree_revision,
    })
}

fn validate_checkout_revisions(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<(String, String)> {
    let submodule = paths.box2d();
    if !submodule.exists() {
        return Err(Error::message(format!(
            "Box2D submodule is not initialized at {}",
            submodule.display()
        )));
    }
    let dirty = git_output(
        &submodule,
        ["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    if !dirty.trim().is_empty() {
        return Err(Error::message(format!(
            "Box2D submodule is dirty:\n{dirty}"
        )));
    }
    let worktree_revision = git_output(&submodule, ["rev-parse", "HEAD"])?
        .trim()
        .to_owned();
    let gitlink_revision = indexed_gitlink(paths.root())?;
    if worktree_revision != manifest.active_revision {
        return Err(Error::message(format!(
            "submodule checkout {worktree_revision} does not match active revision {}",
            manifest.active_revision
        )));
    }
    if gitlink_revision != manifest.active_revision {
        return Err(Error::message(format!(
            "gitlink {gitlink_revision} does not match active revision {}",
            manifest.active_revision
        )));
    }
    ensure_commit_object(&submodule, &manifest.active_revision)?;
    ensure_commit_object(&submodule, &manifest.recording_revision)?;
    Ok((gitlink_revision, worktree_revision))
}

fn validate_artifact_identities(paths: &WorkspacePaths, manifest: &UpstreamManifest) -> Result<()> {
    let provider_inputs = ProviderIdentityInputs::capture(paths.root(), manifest)?;
    let mut provider_identities = Vec::new();
    for artifact in &manifest.artifacts {
        let path = paths.root().join(&artifact.path);
        ensure_regular_file(&path, &format!("artifact `{}`", artifact.name))?;
        validate_file_blake3(&path, &artifact.content_blake3, &artifact.name)?;
        match artifact.kind {
            ArtifactKind::Bindings => validate_binding_identity(&path, artifact, manifest)?,
            ArtifactKind::ProviderIdentity => {
                provider_identities.push(validate_provider_identity(
                    paths.root(),
                    artifact,
                    &provider_inputs,
                )?);
            }
            ArtifactKind::RecordingWire => {
                validate_recording_contract_blake3(&artifact.content_blake3).map_err(|error| {
                    Error::message(format!(
                        "artifact `{}` is not the canonical recording contract: {error}",
                        artifact.name
                    ))
                })?;
                validate_revision_identity(&path, &manifest.recording_revision, &artifact.name)?;
            }
        }
    }
    validate_provider_compiled_identity_cohort(&provider_identities)
}

struct ProviderIdentityInputs {
    effective_source: EffectiveSourceIdentity,
    adapter_source_sha256: String,
    single_bindings_sha256: String,
    double_bindings_sha256: String,
}

impl ProviderIdentityInputs {
    fn capture(root: &Path, manifest: &UpstreamManifest) -> Result<Self> {
        let crate_root = root.join("boxdd-sys");
        Ok(Self {
            effective_source: effective_source_identity(&crate_root).map_err(Error::message)?,
            adapter_source_sha256: adapter_source_sha256(&crate_root).map_err(Error::message)?,
            single_bindings_sha256: wasm_bindings_sha256(root, manifest, Precision::Single)?,
            double_bindings_sha256: wasm_bindings_sha256(root, manifest, Precision::Double)?,
        })
    }

    fn expectation(&self, precision: Precision) -> WasmProviderExpectation<'_> {
        WasmProviderExpectation {
            provider_abi: PROVIDER_ABI,
            target: WASM_TARGET,
            compiler_target: COMPILER_TARGET,
            precision: precision.as_str(),
            upstream_sha: &self.effective_source.upstream_sha,
            source_tree: &self.effective_source.source_tree,
            effective_source_sha256: &self.effective_source.effective_source_sha256,
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: &self.adapter_source_sha256,
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3,
            validation_enabled: false,
            simd: SIMD_MODE,
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS,
            bindings_sha256: match precision {
                Precision::Single => &self.single_bindings_sha256,
                Precision::Double => &self.double_bindings_sha256,
            },
        }
    }
}

fn wasm_bindings_sha256(
    root: &Path,
    manifest: &UpstreamManifest,
    precision: Precision,
) -> Result<String> {
    let mut artifacts = manifest.artifacts.iter().filter(|artifact| {
        artifact.kind == ArtifactKind::Bindings
            && artifact.precision == Some(precision)
            && artifact.target == ArtifactTarget::Wasm32UnknownUnknown
            && artifact.provider == ArtifactProvider::Universal
    });
    let artifact = artifacts.next().ok_or_else(|| {
        Error::message(format!(
            "upstream manifest has no {} wasm32-unknown-unknown bindings artifact",
            precision.as_str()
        ))
    })?;
    if artifacts.next().is_some() {
        return Err(Error::message(format!(
            "upstream manifest has multiple {} wasm32-unknown-unknown bindings artifacts",
            precision.as_str()
        )));
    }
    let path = root.join(&artifact.path);
    ensure_regular_file(&path, &format!("artifact `{}`", artifact.name))?;
    let bytes = fs::read(&path).map_err(|source| Error::io(&path, source))?;
    Ok(sha256_bytes(&bytes))
}

fn validate_provider_identity(
    root: &Path,
    artifact: &GeneratedArtifact,
    inputs: &ProviderIdentityInputs,
) -> Result<WasmProviderIdentity> {
    let precision = artifact.precision.ok_or_else(|| {
        Error::message(format!(
            "provider identity artifact `{}` has no precision",
            artifact.name
        ))
    })?;
    let relative = contract_relative_path(precision.as_str()).map_err(Error::message)?;
    let expected_path = Path::new("boxdd-sys").join(relative);
    if Path::new(&artifact.path) != expected_path {
        return Err(Error::message(format!(
            "provider identity artifact `{}` must use {}",
            artifact.name,
            expected_path.display()
        )));
    }
    WasmProviderIdentity::load(
        &root.join("boxdd-sys"),
        Path::new(relative),
        &inputs.expectation(precision),
    )
    .map_err(|error| {
        Error::message(format!(
            "provider identity artifact `{}` does not match current repository inputs: {error}",
            artifact.name
        ))
    })
}

fn validate_provider_compiled_identity_cohort(identities: &[WasmProviderIdentity]) -> Result<()> {
    let [single, double] = identities else {
        return Err(Error::message(format!(
            "expected two typed WASM provider identities, observed {}",
            identities.len()
        )));
    };
    if single.definition_cookie == double.definition_cookie {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM provider definition cookies disagree across precisions: observed {} and {}",
            single.definition_cookie, double.definition_cookie
        )))
    }
}

fn validate_binding_identity(
    path: &Path,
    artifact: &GeneratedArtifact,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let source = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    let expected = binding_provenance(artifact, &manifest.active_revision)?;
    if source.starts_with(&expected) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "bindings artifact `{}` is missing exact manifest provenance for {}",
            artifact.name, manifest.active_revision
        )))
    }
}

fn binding_provenance(artifact: &GeneratedArtifact, revision: &str) -> Result<String> {
    let precision = artifact.precision.ok_or_else(|| {
        Error::message(format!(
            "bindings artifact `{}` has no precision",
            artifact.name
        ))
    })?;
    let rust_target = binding_generation_target(artifact)?;
    let (wasi_version, wasi_headers, freestanding_headers) = match rust_target {
        RustTarget::X86_64UnknownLinuxGnu => ("none", "none", "none"),
        RustTarget::Wasm32UnknownUnknown => (
            "none",
            "none",
            bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256,
        ),
        RustTarget::Wasm32Wasip1 => (
            bindgen_contract::WASI_LIBC_VERSION,
            bindgen_contract::WASI_LIBC_HEADERS_SHA256,
            "none",
        ),
    };
    Ok(format!(
        "// AUTOGENERATED: pregenerated bindings for docs.rs/offline builds\n\
// boxdd-upstream-revision: {revision}\n\
// boxdd-artifact-name: {}\n\
// boxdd-artifact-precision: {}\n\
// boxdd-artifact-target: {}\n\
// boxdd-artifact-provider: {}\n\
// boxdd-artifact-producer: {}\n\
// boxdd-artifact-rust-target: {}\n\
// boxdd-wasi-libc-version: {wasi_version}\n\
// boxdd-wasi-headers-sha256: {wasi_headers}\n\
// boxdd-freestanding-math-header-sha256: {freestanding_headers}\n\
// Authority: boxdd-sys/upstream.toml\n\
// Refresh with: cargo run -p xtask -- upstream-sync --write\n",
        artifact.name,
        precision.as_str(),
        artifact.target.as_str(),
        artifact.provider.as_str(),
        artifact.producer.as_str(),
        rust_target.as_str(),
    ))
}

#[derive(Deserialize)]
struct RevisionIdentity {
    upstream_sha: String,
}

fn validate_revision_identity(path: &Path, expected: &str, label: &str) -> Result<()> {
    let identity: RevisionIdentity = read_toml(path)?;
    if identity.upstream_sha == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "artifact `{label}` revision {} does not match {expected}",
            identity.upstream_sha
        )))
    }
}

pub(crate) fn validate_recording_input_identities(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let effective = effective_recording_input_sha256s(paths)?;
    let reviewed =
        recording_input_identities(&paths.box2d(), &manifest.recording_revision, &effective)?;
    require_recording_inputs_match(
        &manifest.recording_inputs,
        &reviewed,
        &format!(
            "reviewed recording inputs differ from Box2D {}",
            manifest.recording_revision
        ),
    )?;
    if manifest.active_revision != manifest.recording_revision {
        let active =
            recording_input_identities(&paths.box2d(), &manifest.active_revision, &effective)?;
        require_recording_inputs_match(
            &manifest.recording_inputs,
            &active,
            &format!(
                "active Box2D {} changes reviewed recording inputs pinned at {}",
                manifest.active_revision, manifest.recording_revision
            ),
        )?;
    }
    Ok(())
}

fn require_recording_inputs_match(
    expected: &[RecordingInputIdentity],
    actual: &[RecordingInputIdentity],
    message: &str,
) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(message))
    }
}

fn effective_recording_input_sha256s(paths: &WorkspacePaths) -> Result<BTreeMap<String, String>> {
    effective_source_file_sha256s(
        &paths.root().join("boxdd-sys"),
        REVIEWED_RECORDING_INPUT_PATHS,
    )
    .map_err(|error| Error::message(format!("validate effective recording sources: {error}")))
}

fn recording_input_identities(
    repository: &Path,
    revision: &str,
    effective_sha256s: &BTreeMap<String, String>,
) -> Result<Vec<RecordingInputIdentity>> {
    REVIEWED_RECORDING_INPUT_PATHS
        .iter()
        .map(|path| {
            let bytes = git_blob_bytes(repository, revision, path)?;
            Ok(RecordingInputIdentity {
                path: (*path).to_owned(),
                git_blob: git_output(repository, ["rev-parse", &format!("{revision}:{path}")])?
                    .trim()
                    .to_owned(),
                blake3: blake3::hash(&bytes).to_hex().to_string(),
                effective_sha256: effective_sha256s.get(*path).cloned().ok_or_else(|| {
                    Error::message(format!(
                        "effective recording source identity is missing `{path}`"
                    ))
                })?,
            })
        })
        .collect()
}

fn validate_recording_operations(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let bytes = git_blob_bytes(
        &paths.box2d(),
        &manifest.recording_revision,
        recording_ops::RECORDING_OPS_PATH,
    )?;
    let source = String::from_utf8(bytes).map_err(|error| {
        Error::message(format!(
            "{}:{} is not UTF-8: {error}",
            manifest.recording_revision,
            recording_ops::RECORDING_OPS_PATH,
        ))
    })?;
    recording_ops::parse(&source).map(|_| ())
}

fn source_inventory(repository: &Path, revision: &str) -> Result<SourceInventory> {
    let tree = git_output(repository, ["rev-parse", &format!("{revision}^{{tree}}")])?
        .trim()
        .to_owned();
    let files = git_output(
        repository,
        [
            "ls-tree",
            "-r",
            "--name-only",
            revision,
            "--",
            "src",
            "include/box2d",
        ],
    )?;
    let mut inventory = SourceInventory {
        tree,
        c_sources: Vec::new(),
        private_headers: Vec::new(),
        inline_files: Vec::new(),
        public_headers: Vec::new(),
    };
    for path in files.lines() {
        let candidate = Path::new(path);
        let extension = candidate.extension().and_then(OsStr::to_str);
        if candidate.starts_with("src") && candidate != Path::new("src") {
            match extension {
                Some("c") => inventory.c_sources.push(path.to_owned()),
                Some("h") => inventory.private_headers.push(path.to_owned()),
                Some("inl") => inventory.inline_files.push(path.to_owned()),
                _ => {}
            }
        } else if candidate.starts_with("include/box2d")
            && candidate != Path::new("include/box2d")
            && extension == Some("h")
        {
            inventory.public_headers.push(path.to_owned());
        }
    }
    Ok(inventory)
}

fn generate_bindings(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<Vec<(PathBuf, String)>> {
    let cargo = QualifiedCargo::qualify(paths.root())?;
    fs::create_dir_all(cargo.target_dir())
        .map_err(|source| Error::io(cargo.target_dir(), source))?;
    let temporary = TempDir::new_in(cargo.target_dir())
        .map_err(|source| Error::io(cargo.target_dir(), source))?;
    let wasi_sysroot = bindgen_contract::resolve_wasi_sysroot(
        RustTarget::Wasm32Wasip1.as_str(),
        true,
        std::env::var_os("BOXDD_SYS_WASI_SYSROOT")
            .map(PathBuf::from)
            .as_deref(),
    )
    .map_err(Error::message)?
    .ok_or_else(|| Error::message("wasm32-wasip1 binding generation requires a WASI sysroot"))?;
    let freestanding_headers = bindgen_contract::resolve_unknown_unknown_headers(
        &paths.root().join("boxdd-sys"),
        RustTarget::Wasm32UnknownUnknown.as_str(),
        true,
    )
    .map_err(Error::message)?
    .ok_or_else(|| {
        Error::message("wasm32-unknown-unknown binding generation requires freestanding headers")
    })?;
    let mut generated = Vec::new();
    for artifact in manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
    {
        let precision = artifact.precision.ok_or_else(|| {
            Error::message(format!(
                "bindings artifact `{}` has no precision",
                artifact.name
            ))
        })?;
        let rust_target = binding_generation_target(artifact)?;
        let features = match precision {
            Precision::Single => "bindgen",
            Precision::Double => "bindgen,double-precision",
        };
        let target_dir = temporary.path().join(&artifact.name);
        let mut command = cargo.command(paths.root())?;
        command
            .args([
                "build",
                "--locked",
                "--target",
                rust_target.as_str(),
                "-p",
                "boxdd-sys",
                "--features",
                features,
            ])
            .env("CARGO_TARGET_DIR", &target_dir)
            .env("BOXDD_SYS_SKIP_CC", "1")
            .env("BOXDD_SYS_FORCE_BINDGEN", "1")
            .env("BOXDD_SYS_BINDGEN_TARGET", rust_target.as_str())
            .env_remove("BOXDD_SYS_WASI_SYSROOT")
            .env(
                "BOXDD_SYS_PROVIDER",
                match rust_target {
                    RustTarget::X86_64UnknownLinuxGnu => ProviderCapability::Vendored.as_str(),
                    RustTarget::Wasm32UnknownUnknown | RustTarget::Wasm32Wasip1 => {
                        ProviderCapability::WasmCompileOnly.as_str()
                    }
                },
            );
        if rust_target == RustTarget::Wasm32Wasip1 {
            command.env("BOXDD_SYS_WASI_SYSROOT", &wasi_sysroot.canonical_path);
        }
        command_success(&mut command, &format!("generate {}", artifact.name))?;
        let content =
            load_generated_binding(&target_dir.join(rust_target.as_str()), &artifact.name)?;
        let provenance = binding_provenance(artifact, &manifest.active_revision)?;
        let expected_header_identity = match rust_target {
            RustTarget::Wasm32Wasip1 => wasi_sysroot.identity_sha256(),
            RustTarget::Wasm32UnknownUnknown => freestanding_headers.identity_sha256(),
            RustTarget::X86_64UnknownLinuxGnu => "none",
        };
        let pinned_header_identity = match rust_target {
            RustTarget::Wasm32Wasip1 => bindgen_contract::WASI_LIBC_HEADERS_SHA256,
            RustTarget::Wasm32UnknownUnknown => {
                bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256
            }
            RustTarget::X86_64UnknownLinuxGnu => "none",
        };
        if expected_header_identity != pinned_header_identity {
            return Err(Error::message(format!(
                "{} header identity changed: expected {pinned_header_identity}, observed {expected_header_identity}",
                rust_target.as_str()
            )));
        }
        generated.push((
            paths.root().join(&artifact.path),
            format!("{provenance}\n{content}"),
        ));
    }
    Ok(generated)
}

fn binding_generation_target(artifact: &GeneratedArtifact) -> Result<RustTarget> {
    match artifact.target {
        ArtifactTarget::Universal => Ok(RustTarget::X86_64UnknownLinuxGnu),
        ArtifactTarget::Wasm32UnknownUnknown => Ok(RustTarget::Wasm32UnknownUnknown),
        ArtifactTarget::Wasm32Wasip1 => Ok(RustTarget::Wasm32Wasip1),
    }
}

fn collect_generated_bindings(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(|source| Error::io(directory, source))? {
        let entry = entry.map_err(|source| Error::io(directory, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| Error::io(&path, source))?;
        if file_type.is_dir() {
            collect_generated_bindings(&path, output)?;
        } else if file_type.is_file()
            && path
                .file_name()
                .is_some_and(is_content_addressed_bindings_name)
        {
            output.push(path);
        }
    }
    Ok(())
}

fn load_generated_binding(directory: &Path, artifact_name: &str) -> Result<String> {
    let mut candidates = Vec::new();
    collect_generated_bindings(directory, &mut candidates)?;
    if candidates.len() != 1 {
        return Err(Error::message(format!(
            "{artifact_name} generation produced {} bindings files; expected exactly one",
            candidates.len()
        )));
    }
    fs::read_to_string(&candidates[0]).map_err(|source| Error::io(&candidates[0], source))
}

fn is_content_addressed_bindings_name(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(digest) = name
        .strip_prefix("boxdd-bindings-")
        .and_then(|name| name.strip_suffix(".rs"))
    else {
        return false;
    };
    is_hex(digest, 64)
}

fn provider_identities_need_refresh(root: &Path, manifest: &UpstreamManifest) -> Result<bool> {
    let inputs = ProviderIdentityInputs::capture(root, manifest)?;
    let mut identities = Vec::new();
    for artifact in manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
    {
        let path = root.join(&artifact.path);
        if ensure_regular_file(&path, &format!("artifact `{}`", artifact.name)).is_err()
            || validate_file_blake3(&path, &artifact.content_blake3, &artifact.name).is_err()
        {
            return Ok(true);
        }
        let Ok(identity) = validate_provider_identity(root, artifact, &inputs) else {
            return Ok(true);
        };
        identities.push(identity);
    }
    Ok(validate_provider_compiled_identity_cohort(&identities).is_err())
}

fn indexed_gitlink(root: &Path) -> Result<String> {
    let output = git_output(root, ["ls-files", "--stage", "--", BOX2D_GITLINK])?;
    let lines = output.lines().collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err(Error::message(format!(
            "{BOX2D_GITLINK} must have exactly one stage-0 index entry"
        )));
    }
    let (metadata, indexed_path) = lines[0]
        .split_once('\t')
        .ok_or_else(|| Error::message("invalid Box2D gitlink index entry"))?;
    let fields = metadata.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3
        || fields[0] != "160000"
        || !is_hex(fields[1], 40)
        || fields[2] != "0"
        || indexed_path != BOX2D_GITLINK
    {
        return Err(Error::message(format!(
            "{BOX2D_GITLINK} is not a canonical stage-0 Git submodule entry"
        )));
    }
    Ok(fields[1].to_owned())
}

fn ensure_commit_object(repository: &Path, revision: &str) -> Result<()> {
    let object = format!("{revision}^{{commit}}");
    let mut command = git_command()?;
    command
        .current_dir(repository)
        .args(["cat-file", "-e", &object]);
    let output = run_output(&mut command, "verify Box2D commit object").map_err(Error::message)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("git cat-file", &output))
    }
}

fn git_blob_bytes(repository: &Path, revision: &str, path: &str) -> Result<Vec<u8>> {
    let object = format!("{revision}:{path}");
    let mut command = git_command()?;
    command
        .current_dir(repository)
        .args(["cat-file", "blob", &object]);
    let output = run_output(&mut command, "read reviewed Box2D blob").map_err(Error::message)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(command_error("git cat-file blob", &output))
    }
}

fn git_output<I, S>(repository: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = git_command()?;
    command.current_dir(repository).args(args);
    let output = run_output(&mut command, "run Git").map_err(Error::message)?;
    if !output.status.success() {
        return Err(command_error("git", &output));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| Error::message(format!("git emitted non-UTF-8 output: {error}")))
}

fn git_command() -> Result<Command> {
    isolated_git_command().map_err(Error::message)
}

fn command_success(command: &mut Command, label: &str) -> Result<()> {
    let output = run_output(command, label).map_err(Error::message)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error(label, &output))
    }
}

fn command_error(label: &str, output: &Output) -> Error {
    Error::message(format!(
        "{label} failed with {}:\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn ensure_regular_file(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if metadata.file_type().is_file() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        )))
    }
}

fn file_blake3(path: &Path) -> Result<String> {
    let content = fs::read(path).map_err(|source| Error::io(path, source))?;
    Ok(blake3::hash(&content).to_hex().to_string())
}

fn validate_file_blake3(path: &Path, expected: &str, label: &str) -> Result<()> {
    let actual = file_blake3(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "artifact `{label}` digest drifted: expected {expected}, observed {actual}"
        )))
    }
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn validate_relative_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    let lexical_path = path.to_str().unwrap_or_default();
    if lexical_path.is_empty()
        || lexical_path.contains('\\')
        || lexical_path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(Error::message(
            "path must be a non-empty normalized relative path",
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct UpdateLock {
    file: Option<fs::File>,
}

impl UpdateLock {
    pub(crate) fn acquire(root: &Path) -> Result<Self> {
        let path = repository_lock_path(root, Path::new("boxdd-upstream-sync.lock"))
            .map_err(Error::message)?;
        let mut file = open_repository_lock(&path).map_err(|source| Error::io(&path, source))?;
        file.try_lock().map_err(|source| {
            Error::message(format!(
                "could not acquire repository update lock {}: {source}",
                path.display()
            ))
        })?;
        file.set_len(0).map_err(|source| Error::io(&path, source))?;
        writeln!(file, "pid={}", std::process::id()).map_err(|source| Error::io(&path, source))?;
        Ok(Self { file: Some(file) })
    }
}

fn open_repository_lock(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.custom_flags(libc::O_NOFOLLOW);
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }

    let file = options.open(path)?;
    let opened = file.metadata()?;
    if !opened.file_type().is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "repository update lock is not a regular file",
        ));
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        if opened.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "repository update lock is a reparse point",
            ));
        }
    }

    let linked = fs::symlink_metadata(path)?;
    if !linked.file_type().is_file() || linked.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "repository update lock is not a regular non-symlink file",
        ));
    }
    Ok(file)
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn repository_lock_open_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let victim = directory.path().join("victim");
        let lock = directory.path().join("lock");
        fs::write(&victim, "preserve").unwrap();
        symlink(&victim, &lock).unwrap();

        assert!(open_repository_lock(&lock).is_err());
        assert_eq!(fs::read_to_string(&victim).unwrap(), "preserve");
    }

    fn test_manifest(revision: &str) -> UpstreamManifest {
        UpstreamManifest {
            schema_version: UPSTREAM_MANIFEST_SCHEMA,
            repository: UPSTREAM_REPOSITORY.to_owned(),
            active_revision: revision.to_owned(),
            recording_revision: revision.to_owned(),
            recording_inputs: Vec::new(),
            artifacts: Vec::new(),
            source_inventory: SourceInventory {
                tree: String::new(),
                c_sources: Vec::new(),
                private_headers: Vec::new(),
                inline_files: Vec::new(),
                public_headers: Vec::new(),
            },
        }
    }

    fn commit_file(repository: &Path, name: &str, content: &str, message: &str) -> String {
        fs::write(repository.join(name), content).unwrap();
        git_output(repository, ["add", "--", name]).unwrap();
        git_output(
            repository,
            [
                "-c",
                "user.name=Boxdd Test",
                "-c",
                "user.email=boxdd@example.invalid",
                "commit",
                "--quiet",
                "-m",
                message,
            ],
        )
        .unwrap();
        git_output(repository, ["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_owned()
    }

    fn provider_identity(precision: Precision, definition_cookie: i32) -> WasmProviderIdentity {
        let upstream_sha = "a".repeat(40);
        let source_tree = "b".repeat(40);
        let effective_source_sha256 = "c".repeat(64);
        let adapter_source_sha256 = "d".repeat(64);
        let bindings_sha256 = "e".repeat(64);
        WasmProviderIdentity::from_compiled(
            &WasmProviderExpectation {
                provider_abi: PROVIDER_ABI,
                target: WASM_TARGET,
                compiler_target: COMPILER_TARGET,
                precision: precision.as_str(),
                upstream_sha: &upstream_sha,
                source_tree: &source_tree,
                effective_source_sha256: &effective_source_sha256,
                adapter_abi_version: ADAPTER_ABI_VERSION,
                adapter_source_sha256: &adapter_source_sha256,
                recording_contract_blake3: RECORDING_CONTRACT_BLAKE3,
                validation_enabled: false,
                simd: SIMD_MODE,
                pointer_width: POINTER_WIDTH,
                endianness: ENDIANNESS,
                bindings_sha256: &bindings_sha256,
            },
            [1; 32],
            1,
            definition_cookie,
        )
        .expect("valid provider identity")
    }

    #[test]
    fn artifact_topology_is_small_and_explicit() {
        assert_eq!(ARTIFACT_SPECS.len(), 9);
        assert_eq!(
            ARTIFACT_SPECS
                .iter()
                .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
                .count(),
            6
        );
    }

    #[test]
    fn normalized_paths_reject_escape_components() {
        assert!(validate_relative_path("boxdd-sys/src/bindings.rs").is_ok());
        assert!(validate_relative_path("../bindings.rs").is_err());
        assert!(validate_relative_path("boxdd-sys/./bindings.rs").is_err());
        assert!(validate_relative_path("").is_err());
    }

    #[test]
    fn checkout_and_gitlink_revision_drift_fail_independently() {
        let root = TempDir::new().unwrap();
        let paths = WorkspacePaths::new(root.path());
        let submodule = paths.box2d();
        fs::create_dir_all(&submodule).unwrap();
        git_output(&submodule, ["init", "--quiet"]).unwrap();
        let first_revision = commit_file(&submodule, "source.c", "first\n", "first");

        git_output(root.path(), ["init", "--quiet"]).unwrap();
        git_output(root.path(), ["add", "--", BOX2D_GITLINK]).unwrap();

        let mut manifest = test_manifest(&first_revision);
        assert_eq!(
            validate_checkout_revisions(&paths, &manifest).unwrap(),
            (first_revision.clone(), first_revision.clone())
        );

        let second_revision = commit_file(&submodule, "source.c", "second\n", "second");
        let error = validate_checkout_revisions(&paths, &manifest)
            .expect_err("checkout drift must fail before gitlink validation");
        assert!(error.to_string().contains("submodule checkout"));

        manifest.active_revision.clone_from(&second_revision);
        manifest.recording_revision = second_revision;
        let error = validate_checkout_revisions(&paths, &manifest)
            .expect_err("stale gitlink must fail after checkout validation");
        assert!(error.to_string().contains("gitlink"));
    }

    #[test]
    fn generated_binding_selection_is_scoped_and_requires_one_candidate() {
        let root = TempDir::new().unwrap();
        let exact_target = root.path().join("current/target");
        let stale_target = root.path().join("stale/target");
        fs::create_dir_all(&exact_target).unwrap();
        fs::create_dir_all(&stale_target).unwrap();
        let first_name = format!("boxdd-bindings-{}.rs", "a".repeat(64));
        let second_name = format!("boxdd-bindings-{}.rs", "b".repeat(64));
        fs::write(exact_target.join(&first_name), "current").unwrap();
        fs::write(stale_target.join(&second_name), "stale").unwrap();
        fs::write(
            exact_target.join("boxdd-bindings-not-a-digest.rs"),
            "invalid",
        )
        .unwrap();

        assert_eq!(
            load_generated_binding(&exact_target, "fixture").unwrap(),
            "current"
        );

        fs::write(exact_target.join(&second_name), "ambiguous").unwrap();
        let error = load_generated_binding(&exact_target, "fixture")
            .expect_err("multiple current candidates must fail closed");
        assert!(error.to_string().contains("produced 2 bindings files"));
    }

    #[test]
    fn provider_identity_cohort_requires_one_definition_cookie() {
        let single = provider_identity(Precision::Single, 7);
        let double = provider_identity(Precision::Double, 7);
        assert!(validate_provider_compiled_identity_cohort(&[single, double]).is_ok());

        let single = provider_identity(Precision::Single, 7);
        let double = provider_identity(Precision::Double, 8);
        let error = validate_provider_compiled_identity_cohort(&[single, double])
            .expect_err("split definition cookie must fail");
        assert!(error.to_string().contains("definition cookies disagree"));
    }

    #[test]
    fn recording_identity_comparison_binds_raw_and_effective_sources() {
        let expected = vec![RecordingInputIdentity {
            path: "src/recording.c".to_owned(),
            git_blob: "a".repeat(40),
            blake3: "b".repeat(64),
            effective_sha256: "c".repeat(64),
        }];
        assert!(require_recording_inputs_match(&expected, &expected, "drift").is_ok());

        for mutation in [
            RecordingInputIdentity {
                git_blob: "d".repeat(40),
                ..expected[0].clone()
            },
            RecordingInputIdentity {
                blake3: "d".repeat(64),
                ..expected[0].clone()
            },
            RecordingInputIdentity {
                effective_sha256: "d".repeat(64),
                ..expected[0].clone()
            },
        ] {
            let error = require_recording_inputs_match(&expected, &[mutation], "drift")
                .expect_err("every recording source identity mutation must fail");
            assert_eq!(error.to_string(), "drift");
        }
    }
}
