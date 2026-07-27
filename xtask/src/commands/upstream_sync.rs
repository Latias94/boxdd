use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write as _,
    path::{Component, Path, PathBuf},
    process::{Command, Output},
};

#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, TempDir};

#[path = "../../../boxdd-sys/src/bindgen_contract.rs"]
mod bindgen_contract;

use crate::{
    Error, Result,
    abi_probe::{AbiProbePrecision, GeneratedAbiProbe, generate_workspace_probe},
    commands::{
        UpdateMode, parse_update_mode,
        support::{QualifiedCargo, controlled_child_directory},
    },
    config::{
        UPSTREAM_MANIFEST_SCHEMA, ensure_no_pending_atomic_batches_for_workspace, read_toml,
        recover_atomic_batches, render_toml, write_atomic, write_atomic_bytes,
        write_new_bytes_noclobber,
    },
    paths::WorkspacePaths,
    qualified_git::{qualified_git_command, repository_lock_path},
    recording_ops,
};

const BOX2D_GITLINK: &str = "boxdd-sys/third-party/box2d";
const ISOLATED_GENERATION_DIRECTORY_PREFIX: &str = "boxdd-upstream-sync-";
const ISOLATED_GENERATION_MARKER: &str = ".boxdd-isolated-generation.toml";
const ISOLATED_GENERATION_MARKER_SCHEMA: u32 = 1;
const UNINITIALIZED_BLAKE3: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const ABI_PROBE_METADATA_SCHEMA: u32 = 1;
const GENERATOR_INPUT_PATHS: &[&str] = &[
    ".cargo",
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    "boxdd/Cargo.toml",
    "boxdd/src",
    "boxdd/tests",
    "boxdd-sys/Cargo.toml",
    "boxdd-sys/build.rs",
    "boxdd-sys/native",
    "boxdd-sys/src",
    "tools/abi-probe",
    "xtask/build.rs",
    "xtask/Cargo.toml",
    "xtask/src",
    "xtask/tests",
    "xtask/toolchains",
];

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
    ApiContract,
    RecordingWire,
    ApiCoverageReport,
    AbiMetadata,
    ProviderIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactProducer {
    Bindgen,
    Reviewed,
    ApiCoverage,
    AbiProbe,
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
    Native,
    Wasm32UnknownUnknown,
    Wasm32Wasip1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactProvider {
    Universal,
    Source,
    SystemStatic,
    PrebuiltStatic,
    WasmRuntime,
    WasmCompileOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RustTarget {
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
            Self::Reviewed => "reviewed",
            Self::ApiCoverage => "api-coverage",
            Self::AbiProbe => "abi-probe",
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
            Self::Native => "native",
            Self::Wasm32UnknownUnknown => "wasm32-unknown-unknown",
            Self::Wasm32Wasip1 => "wasm32-wasip1",
        }
    }
}

impl ArtifactProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::Source => "source",
            Self::SystemStatic => "system-static",
            Self::PrebuiltStatic => "prebuilt-static",
            Self::WasmRuntime => "wasm-runtime",
            Self::WasmCompileOnly => "wasm-compile-only",
        }
    }
}

impl RustTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
            Self::Wasm32UnknownUnknown => "wasm32-unknown-unknown",
            Self::Wasm32Wasip1 => "wasm32-wasip1",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RouteBindingSpec {
    name: &'static str,
    path: &'static str,
    precision: Precision,
    target: ArtifactTarget,
    rust_target: RustTarget,
}

const ROUTE_BINDING_SPECS: [RouteBindingSpec; 6] = [
    RouteBindingSpec {
        name: "bindings-single",
        path: "boxdd-sys/src/bindings_pregenerated.rs",
        precision: Precision::Single,
        target: ArtifactTarget::Universal,
        rust_target: RustTarget::X86_64UnknownLinuxGnu,
    },
    RouteBindingSpec {
        name: "bindings-wasm32-unknown-unknown-single",
        path: "boxdd-sys/src/bindings_wasm32_unknown_unknown.rs",
        precision: Precision::Single,
        target: ArtifactTarget::Wasm32UnknownUnknown,
        rust_target: RustTarget::Wasm32UnknownUnknown,
    },
    RouteBindingSpec {
        name: "bindings-wasm32-wasip1-single",
        path: "boxdd-sys/src/bindings_wasm32_wasip1.rs",
        precision: Precision::Single,
        target: ArtifactTarget::Wasm32Wasip1,
        rust_target: RustTarget::Wasm32Wasip1,
    },
    RouteBindingSpec {
        name: "bindings-double",
        path: "boxdd-sys/src/bindings_double.rs",
        precision: Precision::Double,
        target: ArtifactTarget::Universal,
        rust_target: RustTarget::X86_64UnknownLinuxGnu,
    },
    RouteBindingSpec {
        name: "bindings-wasm32-unknown-unknown-double",
        path: "boxdd-sys/src/bindings_wasm32_unknown_unknown_double.rs",
        precision: Precision::Double,
        target: ArtifactTarget::Wasm32UnknownUnknown,
        rust_target: RustTarget::Wasm32UnknownUnknown,
    },
    RouteBindingSpec {
        name: "bindings-wasm32-wasip1-double",
        path: "boxdd-sys/src/bindings_wasm32_wasip1_double.rs",
        precision: Precision::Double,
        target: ArtifactTarget::Wasm32Wasip1,
        rust_target: RustTarget::Wasm32Wasip1,
    },
];

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_blake3: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AbiProbeMetadata {
    schema_version: u32,
    upstream_sha: String,
    repository: String,
    source_tree: String,
    source_inventory_blake3: String,
    source_provenance: String,
    provider_source_sha: String,
    precision: Precision,
    target: ArtifactTarget,
    provider: ArtifactProvider,
    producer: ArtifactProducer,
    bindings_generation_target: RustTarget,
    binding_artifact: String,
    binding_blake3: String,
    probe_content_blake3: String,
    c_probe_blake3: String,
    mixed_precision_c_probe_blake3: String,
    rust_cases_blake3: String,
    structure_count: usize,
    field_count: usize,
    layout_case_count: usize,
    symbol_count: usize,
    callback_count: usize,
    callable_callback_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingRoute {
    pub mode: Precision,
    pub provider: ArtifactProvider,
    pub artifact: String,
    pub rust_target: RustTarget,
    pub rust_features: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingInputIdentity {
    pub path: String,
    pub git_blob: String,
    pub blake3: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamManifest {
    pub schema_version: u32,
    pub repository: String,
    pub active_revision: String,
    pub next_revision: Option<String>,
    pub recording_revision: String,
    pub artifact_digests_initialized: bool,
    pub binding_routes: Vec<BindingRoute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_binding_routes: Vec<BindingRoute>,
    pub recording_inputs: Vec<RecordingInputIdentity>,
    pub artifacts: Vec<GeneratedArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_artifacts: Vec<GeneratedArtifact>,
    pub source_inventory: SourceInventory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_inventory: Option<SourceInventory>,
}

impl UpstreamManifest {
    pub fn load(paths: &WorkspacePaths) -> Result<Self> {
        let path = paths.upstream_manifest();
        let source = fs::read(&path).map_err(|error| Error::io(&path, error))?;
        Self::from_bytes(paths, &source)
    }

    fn from_bytes(paths: &WorkspacePaths, source: &[u8]) -> Result<Self> {
        let path = paths.upstream_manifest();
        let source = std::str::from_utf8(source)
            .map_err(|error| Error::message(format!("{} is not UTF-8: {error}", path.display())))?;
        let manifest: Self = toml::from_str(source).map_err(|error| {
            Error::message(format!("{}: invalid TOML: {error}", path.display()))
        })?;
        validate_manifest(&manifest)?;
        validate_binding_route_feature_catalog(paths, &manifest.binding_routes)?;
        validate_binding_route_feature_catalog(paths, &manifest.next_binding_routes)?;
        Ok(manifest)
    }

    pub fn artifact(&self, kind: ArtifactKind) -> Result<&GeneratedArtifact> {
        if matches!(
            kind,
            ArtifactKind::Bindings | ArtifactKind::AbiMetadata | ArtifactKind::ProviderIdentity
        ) {
            return Err(Error::message(
                "precision-specific artifacts must be selected by precision, target, and provider",
            ));
        }
        let mut matches = self
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == kind);
        let artifact = matches
            .next()
            .ok_or_else(|| Error::message(format!("upstream manifest has no {kind:?} artifact")))?;
        if matches.next().is_some() {
            return Err(Error::message(format!(
                "upstream manifest has multiple {kind:?} artifacts"
            )));
        }
        Ok(artifact)
    }

    pub fn artifact_path(&self, root: &Path, kind: ArtifactKind) -> Result<PathBuf> {
        Ok(root.join(&self.artifact(kind)?.path))
    }

    pub fn binding_artifact(
        &self,
        precision: Precision,
        target: ArtifactTarget,
        provider: ArtifactProvider,
    ) -> Result<&GeneratedArtifact> {
        let mut matches = self.artifacts.iter().filter(|artifact| {
            artifact.kind == ArtifactKind::Bindings
                && artifact.precision == Some(precision)
                && artifact.target == target
                && artifact.provider == provider
        });
        let artifact = matches.next().ok_or_else(|| {
            Error::message(format!(
                "upstream manifest has no bindings artifact for {}/{}/{}",
                precision.as_str(),
                target.as_str(),
                provider.as_str()
            ))
        })?;
        if matches.next().is_some() {
            return Err(Error::message(format!(
                "upstream manifest has duplicate bindings artifacts for {}/{}/{}",
                precision.as_str(),
                target.as_str(),
                provider.as_str()
            )));
        }
        Ok(artifact)
    }

    pub fn binding_path(
        &self,
        root: &Path,
        precision: Precision,
        target: ArtifactTarget,
        provider: ArtifactProvider,
    ) -> Result<PathBuf> {
        Ok(root.join(&self.binding_artifact(precision, target, provider)?.path))
    }

    pub fn binding_route_artifact(&self, route: &BindingRoute) -> Result<&GeneratedArtifact> {
        let artifact = self
            .artifacts
            .iter()
            .find(|artifact| artifact.name == route.artifact)
            .ok_or_else(|| {
                Error::message(format!(
                    "binding route {}/{} references missing artifact `{}`",
                    route.mode.as_str(),
                    route.provider.as_str(),
                    route.artifact
                ))
            })?;
        if artifact.kind != ArtifactKind::Bindings {
            return Err(Error::message(format!(
                "binding route {}/{} references non-bindings artifact `{}`",
                route.mode.as_str(),
                route.provider.as_str(),
                route.artifact
            )));
        }
        Ok(artifact)
    }

    pub fn recording_source_git_blobs(&self) -> BTreeMap<String, String> {
        self.recording_inputs
            .iter()
            .map(|input| (input.path.clone(), input.git_blob.clone()))
            .collect()
    }

    fn reviewed_artifacts(&self) -> impl Iterator<Item = &GeneratedArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.producer == ArtifactProducer::Reviewed)
    }

    fn binding_artifacts(&self) -> impl Iterator<Item = &GeneratedArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
    }

    fn abi_metadata_artifacts(&self) -> impl Iterator<Item = &GeneratedArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::AbiMetadata)
    }

    fn promoted_for_generation(&self, target: &str) -> Result<Self> {
        let mut promoted = self.clone();
        promoted.active_revision = target.to_owned();
        promoted.next_revision = None;
        promoted.recording_revision = target.to_owned();
        if !promoted.next_binding_routes.is_empty() {
            promoted.binding_routes = std::mem::take(&mut promoted.next_binding_routes);
        }
        promoted.artifacts.append(&mut promoted.next_artifacts);
        if let Some(next_inventory) = promoted.next_inventory.take() {
            promoted.source_inventory = next_inventory;
        }
        for artifact in &mut promoted.artifacts {
            artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
            artifact.candidate_path = None;
            artifact.candidate_blake3 = None;
        }
        promoted.artifact_digests_initialized = false;
        validate_manifest(&promoted)?;
        Ok(promoted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamSnapshot {
    pub active_revision: String,
    pub next_revision: Option<String>,
    pub gitlink_revision: String,
    pub worktree_revision: String,
}

fn load_manifest_snapshot(
    paths: &WorkspacePaths,
    require_clean_generator_inputs: bool,
) -> Result<(UpstreamManifest, ManagedSnapshot)> {
    let loaded_generation = GenerationBaseline::capture_with_policy(paths.root(), false)?;
    let manifest_path = paths.upstream_manifest();
    let manifest_content =
        fs::read(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    let manifest = UpstreamManifest::from_bytes(paths, &manifest_content)?;
    let require_clean_generator_inputs =
        require_clean_generator_inputs || manifest.next_revision.is_some();
    let baseline = if require_clean_generator_inputs {
        ManagedSnapshot::capture(paths, &manifest)?
    } else {
        ManagedSnapshot::capture_observed(paths, &manifest)?
    };
    baseline.verify_loaded_state(paths, &manifest_content, &loaded_generation)?;
    baseline.verify_all(paths)?;
    Ok((manifest, baseline))
}

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    if matches!(args, [argument] if argument == "--refresh-routes") {
        return refresh_routes(paths);
    }
    if matches!(args, [argument] if argument == "--prepare-next") {
        return prepare_next_candidate(paths);
    }
    if matches!(args, [argument] if argument == "--check-next") {
        return check_next_candidate(paths);
    }
    let mode = parse_update_mode("upstream-sync", args)?;
    match mode {
        UpdateMode::Check => {
            let _lock = UpdateLock::acquire(paths.root())?;
            let (manifest, baseline) = load_manifest_snapshot(paths, false)?;
            require_provider_identity_topology(&manifest)?;
            let snapshot = validate_repository(paths, &manifest, false)?;
            super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
            super::api_coverage::check(paths)?;
            if manifest.next_revision.is_some() {
                let (summary, digest) =
                    validate_registered_next_candidate(paths, &manifest, &baseline)?;
                print_next_candidate_summary(&summary, &digest);
            }
            baseline.verify_all(paths)?;
            println!(
                "upstream sync ok: active {}, next {}",
                snapshot.active_revision,
                snapshot.next_revision.as_deref().unwrap_or("<none>")
            );
            Ok(())
        }
        UpdateMode::Write => apply_update(paths),
    }
}

fn refresh_routes(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let inputs = capture_route_refresh_inputs(paths.root())?;
    let repository_revision = inputs.repository_revision.clone();
    let manifest_path = paths.upstream_manifest();
    let manifest_content =
        fs::read(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    let original = UpstreamManifest::from_bytes(paths, &manifest_content)?;
    require_provider_identity_topology(&original)?;
    validate_repository_core(paths, &original)?;
    validate_recording_input_identities(paths, &original)?;
    validate_recording_operations(paths, &original)?;
    let target = canonical_route_refresh_manifest(paths, &original)?;
    let baseline =
        RouteRefreshBaseline::capture(paths, inputs, &manifest_content, &original, &target)?;

    let generation =
        IsolatedGeneration::create_at(paths, &repository_revision, &original.active_revision)?;
    let staged_result = (|| {
        baseline.inputs.overlay(&generation.worktree)?;
        generation.prepare_route_refresh(&original, &target)
    })();
    let cleanup_result = generation.finish();
    let staged = match (staged_result, cleanup_result) {
        (Ok(staged), Ok(())) => staged,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(cleanup)) => return Err(cleanup),
        (Err(error), Err(cleanup)) => {
            return Err(Error::message(format!(
                "route refresh staging failed: {error}\nisolated worktree cleanup also failed: {cleanup}"
            )));
        }
    };

    baseline.verify_before_install(paths)?;
    install_route_refresh(paths, &original, &staged, &baseline, None, || {
        validate_repository(paths, &staged.manifest, false)?;
        super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
        super::api_coverage::check(paths)
    })?;
    println!(
        "refreshed {} binding routes and {} generated artifacts at Box2D {}",
        staged.manifest.binding_routes.len(),
        staged.manifest.artifacts.len(),
        staged.manifest.active_revision
    );
    Ok(())
}

fn capture_route_refresh_inputs(root: &Path) -> Result<GeneratorInputSnapshot> {
    let clean_baseline = GenerationBaseline::capture(root)?;
    let inputs = GeneratorInputSnapshot::capture(root)?;
    clean_baseline.verify(root)?;
    Ok(inputs)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NextCandidateSummary {
    upstream_sha: String,
    function_count: usize,
    safe_count: usize,
    raw_count: usize,
    omitted_count: usize,
    deferred_count: usize,
    abi_struct_count: usize,
    abi_field_count: usize,
    abi_callback_count: usize,
    routes: Vec<String>,
}

#[derive(Debug)]
struct NextCandidateRegistration<'a> {
    target: &'a str,
    path: &'a str,
    digest: &'a str,
}

fn check_next_candidate(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let (manifest, baseline) = load_manifest_snapshot(paths, true)?;
    require_provider_identity_topology(&manifest)?;
    validate_repository(paths, &manifest, true)?;
    super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
    super::api_coverage::check(paths)?;
    let (summary, digest) = validate_registered_next_candidate(paths, &manifest, &baseline)?;
    print_next_candidate_summary(&summary, &digest);
    Ok(())
}

fn validate_registered_next_candidate(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
    baseline: &ManagedSnapshot,
) -> Result<(NextCandidateSummary, String)> {
    let registration = next_candidate_registration(manifest)?;
    let candidate_path = paths.root().join(registration.path);
    let registered =
        fs::read(&candidate_path).map_err(|source| Error::io(&candidate_path, source))?;
    let generation = IsolatedGeneration::create_at(
        paths,
        &baseline.generation.repository_revision,
        registration.target,
    )?;
    let rendered_result = generation.render_next_candidate(manifest, registration.target);
    let cleanup_result = generation.finish();
    let rendered = match (rendered_result, cleanup_result) {
        (Ok(rendered), Ok(())) => rendered,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(cleanup)) => return Err(cleanup),
        (Err(error), Err(cleanup)) => {
            return Err(Error::message(format!(
                "target API candidate verification failed: {error}\nisolated worktree cleanup also failed: {cleanup}"
            )));
        }
    };

    baseline.verify_all(paths)?;
    let summary = validate_next_candidate_bytes(manifest, &registered, &rendered)?;
    Ok((summary, registration.digest.to_owned()))
}

fn print_next_candidate_summary(summary: &NextCandidateSummary, digest: &str) {
    println!(
        "next API candidate ok: revision {}; functions {} (safe {}, raw {}, omitted {}, deferred {}); ABI {} structs/{} fields/{} callbacks; routes {}; blake3 {}",
        summary.upstream_sha,
        summary.function_count,
        summary.safe_count,
        summary.raw_count,
        summary.omitted_count,
        summary.deferred_count,
        summary.abi_struct_count,
        summary.abi_field_count,
        summary.abi_callback_count,
        summary.routes.join(", "),
        digest,
    );
}

fn next_candidate_registration(
    manifest: &UpstreamManifest,
) -> Result<NextCandidateRegistration<'_>> {
    let target = manifest
        .next_revision
        .as_deref()
        .ok_or_else(|| Error::message("upstream manifest has no next_revision to check"))?;
    let artifact = manifest.artifact(ArtifactKind::ApiContract)?;
    let path = artifact.candidate_path.as_deref().ok_or_else(|| {
        Error::message(format!(
            "reviewed artifact `{}` has no target candidate_path to check",
            artifact.name
        ))
    })?;
    let digest = artifact.candidate_blake3.as_deref().ok_or_else(|| {
        Error::message(format!(
            "reviewed artifact `{}` has no target candidate_blake3 to check",
            artifact.name
        ))
    })?;
    Ok(NextCandidateRegistration {
        target,
        path,
        digest,
    })
}

fn validate_next_candidate_bytes(
    manifest: &UpstreamManifest,
    registered: &[u8],
    rendered: &[u8],
) -> Result<NextCandidateSummary> {
    let registration = next_candidate_registration(manifest)?;
    let registered_digest = blake3::hash(registered).to_hex().to_string();
    if registered_digest != registration.digest {
        return Err(Error::message(format!(
            "registered next API candidate digest drifted: expected {}, observed {registered_digest}",
            registration.digest
        )));
    }

    let promoted = manifest.promoted_for_generation(registration.target)?;
    let registered_summary = summarize_next_candidate(&promoted, registered, "registered")?;
    let rendered_summary = summarize_next_candidate(&promoted, rendered, "rendered")?;
    if registered != rendered {
        let rendered_digest = blake3::hash(rendered).to_hex().to_string();
        return Err(Error::message(format!(
            "registered next API candidate is stale: registered blake3 {registered_digest}, freshly rendered blake3 {rendered_digest}; run `cargo run -p xtask -- upstream-sync --prepare-next`, review, and commit the replacement"
        )));
    }
    if registered_summary != rendered_summary {
        return Err(Error::message(
            "registered and freshly rendered next API candidate summaries differ despite identical bytes",
        ));
    }
    Ok(rendered_summary)
}

fn validate_promotion_candidate(
    root: &Path,
    manifest: &UpstreamManifest,
    rendered: &[u8],
) -> Result<NextCandidateSummary> {
    let registration = next_candidate_registration(manifest)?;
    let registered_path = root.join(registration.path);
    let registered =
        fs::read(&registered_path).map_err(|source| Error::io(&registered_path, source))?;
    validate_next_candidate_bytes(manifest, &registered, rendered)
}

fn summarize_next_candidate(
    promoted: &UpstreamManifest,
    candidate: &[u8],
    label: &str,
) -> Result<NextCandidateSummary> {
    use super::api_coverage::{ApiContract, Classification};

    let source = std::str::from_utf8(candidate).map_err(|error| {
        Error::message(format!("{label} next API candidate is not UTF-8: {error}"))
    })?;
    let contract: ApiContract = toml::from_str(source).map_err(|error| {
        Error::message(format!(
            "could not parse {label} next API candidate as the reviewed contract: {error}"
        ))
    })?;
    if contract.upstream_sha != promoted.active_revision {
        return Err(Error::message(format!(
            "{label} next API candidate revision {} does not match promoted revision {}",
            contract.upstream_sha, promoted.active_revision
        )));
    }

    let route_coordinates = promoted
        .binding_routes
        .iter()
        .map(|route| {
            (
                route.mode.as_str().to_owned(),
                route.provider.as_str().to_owned(),
            )
        })
        .collect::<BTreeSet<_>>();
    let modes = route_coordinates
        .iter()
        .map(|(mode, _)| mode.clone())
        .collect::<BTreeSet<_>>();
    for function in &contract.functions {
        let function_modes = function.modes.iter().cloned().collect::<BTreeSet<_>>();
        let function_providers = function.providers.iter().cloned().collect::<BTreeSet<_>>();
        let function_coordinates = function_modes
            .iter()
            .flat_map(|mode| {
                function_providers
                    .iter()
                    .map(move |provider| (mode.clone(), provider.clone()))
            })
            .collect::<BTreeSet<_>>();
        if function_coordinates != route_coordinates {
            return Err(Error::message(format!(
                "{label} next API candidate function `{}` covers routes {:?}, expected {:?}",
                function.logical_name, function_coordinates, route_coordinates
            )));
        }
        let link_modes = function
            .link_symbols
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        if link_modes != modes {
            return Err(Error::message(format!(
                "{label} next API candidate function `{}` declares link modes {:?}, expected {:?}",
                function.logical_name, link_modes, modes
            )));
        }
    }
    for policy in &contract.abi.policies {
        validate_candidate_mode_provider_matrix(
            label,
            &format!("ABI policy `{}`", policy.id),
            &policy.modes,
            &policy.providers,
            &route_coordinates,
        )?;
    }
    for structure in &contract.abi.structs {
        validate_candidate_mappings(
            label,
            &format!("ABI struct `{}`", structure.name),
            structure
                .raw_mappings
                .iter()
                .map(|mapping| (&mapping.mode, &mapping.provider)),
            &route_coordinates,
        )?;
        for field in &structure.fields {
            validate_candidate_mappings(
                label,
                &format!("ABI field `{}::{}`", structure.name, field.name),
                field
                    .raw_mappings
                    .iter()
                    .map(|mapping| (&mapping.mode, &mapping.provider)),
                &route_coordinates,
            )?;
        }
    }
    for callback in &contract.abi.callbacks {
        validate_candidate_mappings(
            label,
            &format!("ABI callback `{}`", callback.name),
            callback
                .raw_mappings
                .iter()
                .map(|mapping| (&mapping.mode, &mapping.provider)),
            &route_coordinates,
        )?;
    }

    let mut safe_count = 0;
    let mut raw_count = 0;
    let mut omitted_count = 0;
    let mut deferred_count = 0;
    for function in &contract.functions {
        match function.classification {
            Classification::Safe => safe_count += 1,
            Classification::Raw => raw_count += 1,
            Classification::Omitted => omitted_count += 1,
            Classification::Deferred => deferred_count += 1,
        }
    }
    Ok(NextCandidateSummary {
        upstream_sha: contract.upstream_sha,
        function_count: contract.functions.len(),
        safe_count,
        raw_count,
        omitted_count,
        deferred_count,
        abi_struct_count: contract.abi.structs.len(),
        abi_field_count: contract
            .abi
            .structs
            .iter()
            .map(|structure| structure.fields.len())
            .sum(),
        abi_callback_count: contract.abi.callbacks.len(),
        routes: route_coordinates
            .into_iter()
            .map(|(mode, provider)| format!("{mode}/{provider}"))
            .collect(),
    })
}

fn validate_candidate_mode_provider_matrix(
    label: &str,
    subject: &str,
    modes: &[String],
    providers: &[String],
    expected: &BTreeSet<(String, String)>,
) -> Result<()> {
    let observed = modes
        .iter()
        .flat_map(|mode| {
            providers
                .iter()
                .map(move |provider| (mode.clone(), provider.clone()))
        })
        .collect::<BTreeSet<_>>();
    if observed == *expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} next API candidate {subject} covers routes {observed:?}, expected {expected:?}"
        )))
    }
}

fn validate_candidate_mappings<'a>(
    label: &str,
    subject: &str,
    mappings: impl Iterator<Item = (&'a String, &'a String)>,
    expected: &BTreeSet<(String, String)>,
) -> Result<()> {
    let observed = mappings
        .map(|(mode, provider)| (mode.clone(), provider.clone()))
        .collect::<BTreeSet<_>>();
    if observed == *expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} next API candidate {subject} maps routes {observed:?}, expected {expected:?}"
        )))
    }
}

fn prepare_next_candidate(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let (manifest, baseline) = load_manifest_snapshot(paths, true)?;
    let manifest_baseline = baseline.manifest_content(paths)?.to_vec();
    require_provider_identity_topology(&manifest)?;
    validate_repository(paths, &manifest, true)?;
    super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
    super::api_coverage::check(paths)?;
    let target = manifest
        .next_revision
        .as_deref()
        .ok_or_else(|| Error::message("upstream manifest has no next_revision to prepare"))?;
    let generation =
        IsolatedGeneration::create_at(paths, &baseline.generation.repository_revision, target)?;
    let candidate_result = generation.render_next_candidate(&manifest, target);
    let cleanup_result = generation.finish();
    let candidate = match (candidate_result, cleanup_result) {
        (Ok(candidate), Ok(())) => candidate,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(cleanup)) => return Err(cleanup),
        (Err(error), Err(cleanup)) => {
            return Err(Error::message(format!(
                "target API candidate generation failed: {error}\nisolated worktree cleanup also failed: {cleanup}"
            )));
        }
    };
    baseline.verify_all(paths)?;
    let candidate_path = reviewed_candidate_path(manifest.artifact(ArtifactKind::ApiContract)?)?;
    let writes = [ManagedArtifactWrite::reviewed_candidate(
        "api-contract",
        &candidate_path,
        candidate,
    )];
    install_managed_artifact_writes_locked(paths, &writes, Some(&manifest_baseline), || {
        let installed_manifest = UpstreamManifest::load(paths)?;
        validate_repository(paths, &installed_manifest, false)?;
        super::api_coverage::check(paths)
    })?;
    println!("prepared reviewed API contract candidate for Box2D {target} at {candidate_path}");
    Ok(())
}

fn reviewed_candidate_path(artifact: &GeneratedArtifact) -> Result<String> {
    let active = Path::new(&artifact.path);
    let stem = active
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            Error::message(format!(
                "reviewed artifact `{}` has no UTF-8 file stem",
                artifact.name
            ))
        })?;
    let file_name = match active.extension().and_then(|value| value.to_str()) {
        Some(extension) => format!("{stem}.next.{extension}"),
        None => format!("{stem}.next"),
    };
    let candidate = active
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(file_name);
    canonical_manifest_path(&candidate)
        .ok_or_else(|| Error::message("reviewed candidate path is not canonical UTF-8"))
}

pub fn checked_snapshot(paths: &WorkspacePaths) -> Result<UpstreamSnapshot> {
    let manifest = UpstreamManifest::load(paths)?;
    require_provider_identity_topology(&manifest)?;
    let snapshot = validate_repository(paths, &manifest, false)?;
    super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
    Ok(snapshot)
}

pub fn validate_repository(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
    reject_managed_changes: bool,
) -> Result<UpstreamSnapshot> {
    if !manifest.artifact_digests_initialized {
        return Err(Error::message(
            "artifact digests are not initialized; run `cargo run -p xtask -- api-coverage --refresh-abi` to bootstrap them",
        ));
    }
    let snapshot = validate_repository_core(paths, manifest)?;
    validate_artifact_identities(paths, manifest)?;
    validate_candidate_identities(paths, manifest)?;
    validate_recording_input_identities(paths, manifest)?;
    validate_recording_operations(paths, manifest)?;
    validate_abi_probe_artifacts(paths, manifest)?;
    if reject_managed_changes {
        reject_managed_changes_if_present(paths, manifest)?;
    }
    Ok(snapshot)
}

fn validate_repository_core(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<UpstreamSnapshot> {
    validate_manifest(manifest)?;
    let submodule = paths.box2d();
    if !submodule.join(".git").exists() && !submodule.join("HEAD").exists() {
        return Err(Error::message(format!(
            "Box2D submodule is not initialized at {}",
            submodule.display()
        )));
    }
    let dirty = git_output(&submodule, ["status", "--porcelain=v1"])?;
    if !dirty.trim().is_empty() {
        return Err(Error::message(format!(
            "Box2D submodule is dirty; upstream-sync refuses to continue:\n{dirty}"
        )));
    }
    let worktree_revision = git_output(&submodule, ["rev-parse", "HEAD"])?;
    let worktree_revision = worktree_revision.trim().to_owned();
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
    for revision in [
        Some(manifest.active_revision.as_str()),
        manifest.next_revision.as_deref(),
        Some(manifest.recording_revision.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        ensure_commit_object(&submodule, revision)?;
    }
    let observed_inventory = source_inventory(&submodule, &manifest.active_revision)?;
    validate_exact_inventory(&manifest.source_inventory, &observed_inventory)?;
    if let (Some(next_revision), Some(next_inventory)) =
        (&manifest.next_revision, &manifest.next_inventory)
    {
        let observed_next = source_inventory(&submodule, next_revision)?;
        validate_exact_inventory(next_inventory, &observed_next)?;
    }
    Ok(UpstreamSnapshot {
        active_revision: manifest.active_revision.clone(),
        next_revision: manifest.next_revision.clone(),
        gitlink_revision,
        worktree_revision,
    })
}

fn reject_managed_changes_if_present(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let dirty = managed_status(paths.root(), manifest)?;
    if dirty.trim().is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "upstream-managed files are dirty; commit intentional artifact changes before syncing:\n{dirty}"
        )))
    }
}

fn validate_manifest(manifest: &UpstreamManifest) -> Result<()> {
    let mut errors = Vec::new();
    if manifest.schema_version != UPSTREAM_MANIFEST_SCHEMA {
        errors.push(format!(
            "upstream manifest schema {} does not match supported schema {UPSTREAM_MANIFEST_SCHEMA}",
            manifest.schema_version
        ));
    }
    if manifest.repository != "https://github.com/erincatto/box2d.git" {
        errors.push("upstream repository must be the official Box2D repository".to_owned());
    }
    if !is_full_sha(&manifest.active_revision) {
        errors.push("active_revision must be a lowercase 40-character Git SHA".to_owned());
    }
    if !is_full_sha(&manifest.recording_revision) {
        errors.push("recording_revision must be a lowercase 40-character Git SHA".to_owned());
    }
    if let Some(next) = &manifest.next_revision {
        if !is_full_sha(next) {
            errors.push("next_revision must be a lowercase 40-character Git SHA".to_owned());
        }
        if next == &manifest.active_revision {
            errors.push("next_revision must differ from active_revision".to_owned());
        }
    }
    if manifest.recording_revision != manifest.active_revision
        && manifest.next_revision.as_deref() != Some(&manifest.recording_revision)
    {
        errors.push("recording_revision must equal active_revision or next_revision".to_owned());
    }
    validate_artifacts(&manifest.artifacts, &mut errors);
    let zero_digests = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.content_blake3 == UNINITIALIZED_BLAKE3)
        .count();
    match (manifest.artifact_digests_initialized, zero_digests) {
        (false, count) if count != manifest.artifacts.len() => errors.push(
            "an uninitialized artifact digest manifest must contain only zero content_blake3 values"
                .to_owned(),
        ),
        (true, count) if count != 0 => errors.push(
            "an initialized artifact digest manifest cannot contain zero content_blake3 values"
                .to_owned(),
        ),
        (false, _) | (true, _) => {}
    }
    validate_binding_routes(&manifest.binding_routes, &manifest.artifacts, &mut errors);
    validate_abi_metadata_topology(
        &manifest.binding_routes,
        &manifest.artifacts,
        false,
        &mut errors,
    );
    validate_next_binding_topology(manifest, &mut errors);
    validate_recording_input_shape(&manifest.recording_inputs, &mut errors);
    validate_inventory_shape(&manifest.source_inventory, &mut errors);
    match (&manifest.next_revision, &manifest.next_inventory) {
        (Some(_), Some(inventory)) => validate_inventory_shape(inventory, &mut errors),
        (Some(_), None) => errors.push("next_revision requires next_inventory".to_owned()),
        (None, Some(_)) => errors.push("next_inventory requires next_revision".to_owned()),
        (None, None) => {}
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn validate_next_binding_topology(manifest: &UpstreamManifest, errors: &mut Vec<String>) {
    if manifest.next_binding_routes.is_empty() && manifest.next_artifacts.is_empty() {
        return;
    }
    if manifest.next_revision.is_none() {
        errors.push("next binding topology requires next_revision".to_owned());
    }
    if manifest.next_binding_routes.is_empty() {
        errors.push("next binding topology has no binding routes".to_owned());
        return;
    }
    for artifact in &manifest.next_artifacts {
        if !matches!(
            (artifact.kind, artifact.producer),
            (ArtifactKind::Bindings, ArtifactProducer::Bindgen)
                | (ArtifactKind::AbiMetadata, ArtifactProducer::AbiProbe)
        ) {
            errors.push(format!(
                "next artifact `{}` must be produced by its deterministic bindings or ABI probe generator",
                artifact.name
            ));
        }
        if artifact.content_blake3 != UNINITIALIZED_BLAKE3 {
            errors.push(format!(
                "next artifact `{}` must keep an uninitialized digest until target generation",
                artifact.name
            ));
        }
        if artifact.candidate_path.is_some() || artifact.candidate_blake3.is_some() {
            errors.push(format!(
                "next artifact `{}` cannot declare a reviewed candidate",
                artifact.name
            ));
        }
    }
    let mut projected_artifacts = manifest.artifacts.clone();
    projected_artifacts.extend(manifest.next_artifacts.iter().cloned());
    validate_artifacts(&projected_artifacts, errors);
    validate_binding_routes(&manifest.next_binding_routes, &projected_artifacts, errors);
    validate_abi_metadata_topology(
        &manifest.next_binding_routes,
        &projected_artifacts,
        projected_artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::AbiMetadata),
        errors,
    );
}

fn validate_abi_metadata_topology(
    routes: &[BindingRoute],
    artifacts: &[GeneratedArtifact],
    required: bool,
    errors: &mut Vec<String>,
) {
    let metadata = artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::AbiMetadata)
        .collect::<Vec<_>>();
    if metadata.is_empty() && !required {
        return;
    }

    let expected = routes
        .iter()
        .filter(|route| route.provider == ArtifactProvider::Source)
        .map(|route| (route.mode, route.provider))
        .collect::<BTreeSet<_>>();
    let observed = metadata
        .iter()
        .filter_map(|artifact| {
            artifact
                .precision
                .map(|precision| (precision, artifact.provider))
        })
        .collect::<BTreeSet<_>>();
    if observed != expected {
        errors.push(format!(
            "source ABI metadata coordinates {observed:?} do not match source binding routes {expected:?}"
        ));
    }

    for artifact in metadata {
        if artifact.target != ArtifactTarget::Native {
            errors.push(format!(
                "ABI metadata artifact `{}` must target native C ABI qualification",
                artifact.name
            ));
        }
        if artifact.provider != ArtifactProvider::Source {
            errors.push(format!(
                "ABI metadata artifact `{}` must identify the vendored source provider; other providers require their own attestation contract",
                artifact.name
            ));
        }
        let Some(precision) = artifact.precision else {
            continue;
        };
        if !routes
            .iter()
            .any(|route| route.mode == precision && route.provider == artifact.provider)
        {
            errors.push(format!(
                "ABI metadata artifact `{}` has no matching {}/{} binding route",
                artifact.name,
                precision.as_str(),
                artifact.provider.as_str()
            ));
        }
    }
}

fn validate_binding_routes(
    routes: &[BindingRoute],
    artifacts: &[GeneratedArtifact],
    errors: &mut Vec<String>,
) {
    if routes.is_empty() {
        errors.push("upstream manifest has no binding routes".to_owned());
        return;
    }
    if !routes.windows(2).all(|pair| pair[0] < pair[1]) {
        errors.push("binding routes must be sorted and unique".to_owned());
    }
    let mut coordinates = BTreeSet::new();
    let mut routed_artifacts = BTreeSet::new();
    for route in routes {
        let coordinate = (route.mode, route.provider);
        if !coordinates.insert(coordinate) {
            errors.push(format!(
                "duplicate binding route for {}/{}",
                route.mode.as_str(),
                route.provider.as_str()
            ));
        }
        if route.provider == ArtifactProvider::Universal {
            errors.push(format!(
                "binding route {}/{} must name a concrete provider",
                route.mode.as_str(),
                route.provider.as_str()
            ));
        }
        if !route.rust_features.windows(2).all(|pair| pair[0] < pair[1]) {
            errors.push(format!(
                "binding route {}/{} rust_features must be sorted and unique",
                route.mode.as_str(),
                route.provider.as_str()
            ));
        }
        let enables_double_precision = route
            .rust_features
            .iter()
            .any(|feature| feature == "double-precision");
        match route.mode {
            Precision::Single if enables_double_precision => errors.push(format!(
                "single-precision binding route {}/{} must not enable `double-precision`",
                route.mode.as_str(),
                route.provider.as_str()
            )),
            Precision::Double if !enables_double_precision => errors.push(format!(
                "double-precision binding route {}/{} must enable `double-precision`",
                route.mode.as_str(),
                route.provider.as_str()
            )),
            Precision::Single | Precision::Double => {}
        }
        routed_artifacts.insert(route.artifact.as_str());
        let Some(artifact) = artifacts
            .iter()
            .find(|artifact| artifact.name == route.artifact)
        else {
            errors.push(format!(
                "binding route {}/{} references missing artifact `{}`",
                route.mode.as_str(),
                route.provider.as_str(),
                route.artifact
            ));
            continue;
        };
        if artifact.kind != ArtifactKind::Bindings {
            errors.push(format!(
                "binding route {}/{} references non-bindings artifact `{}`",
                route.mode.as_str(),
                route.provider.as_str(),
                route.artifact
            ));
        }
        if artifact.precision != Some(route.mode) {
            errors.push(format!(
                "binding route {}/{} references `{}` with precision {:?}",
                route.mode.as_str(),
                route.provider.as_str(),
                route.artifact,
                artifact.precision
            ));
        }
        let provider_compatible = match route.provider {
            ArtifactProvider::Source
            | ArtifactProvider::SystemStatic
            | ArtifactProvider::PrebuiltStatic => matches!(
                artifact.provider,
                ArtifactProvider::Universal
                    | ArtifactProvider::Source
                    | ArtifactProvider::SystemStatic
                    | ArtifactProvider::PrebuiltStatic
            ),
            ArtifactProvider::WasmRuntime | ArtifactProvider::WasmCompileOnly => matches!(
                artifact.provider,
                ArtifactProvider::Universal
                    | ArtifactProvider::WasmRuntime
                    | ArtifactProvider::WasmCompileOnly
            ),
            ArtifactProvider::Universal => false,
        };
        if !provider_compatible {
            errors.push(format!(
                "binding route {}/{} references `{}` for provider {}",
                route.mode.as_str(),
                route.provider.as_str(),
                route.artifact,
                artifact.provider.as_str()
            ));
        }
        let target_compatible = match (route.provider, route.rust_target) {
            (
                ArtifactProvider::Source
                | ArtifactProvider::SystemStatic
                | ArtifactProvider::PrebuiltStatic,
                RustTarget::X86_64UnknownLinuxGnu,
            ) => matches!(
                artifact.target,
                ArtifactTarget::Universal | ArtifactTarget::Native
            ),
            (
                ArtifactProvider::WasmRuntime | ArtifactProvider::WasmCompileOnly,
                RustTarget::Wasm32UnknownUnknown,
            ) => artifact.target == ArtifactTarget::Wasm32UnknownUnknown,
            (
                ArtifactProvider::WasmRuntime | ArtifactProvider::WasmCompileOnly,
                RustTarget::Wasm32Wasip1,
            ) => artifact.target == ArtifactTarget::Wasm32Wasip1,
            _ => false,
        };
        if !target_compatible {
            errors.push(format!(
                "binding route {}/{} has incompatible target coordinate: Rust {}, artifact `{}` {}",
                route.mode.as_str(),
                route.provider.as_str(),
                route.rust_target.as_str(),
                route.artifact,
                artifact.target.as_str()
            ));
        }
    }
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
    {
        if !routed_artifacts.contains(artifact.name.as_str()) {
            errors.push(format!(
                "bindings artifact `{}` is not used by an executable route",
                artifact.name
            ));
        }
    }
}

pub(crate) fn canonical_route_binding_artifacts() -> Vec<GeneratedArtifact> {
    ROUTE_BINDING_SPECS
        .iter()
        .map(|spec| GeneratedArtifact {
            name: spec.name.to_owned(),
            kind: ArtifactKind::Bindings,
            path: spec.path.to_owned(),
            precision: Some(spec.precision),
            target: spec.target,
            provider: ArtifactProvider::Universal,
            producer: ArtifactProducer::Bindgen,
            content_blake3: UNINITIALIZED_BLAKE3.to_owned(),
            candidate_path: None,
            candidate_blake3: None,
        })
        .collect()
}

pub(crate) fn canonical_binding_routes() -> Vec<BindingRoute> {
    let mut routes = Vec::with_capacity(10);
    for precision in [Precision::Single, Precision::Double] {
        let native = match precision {
            Precision::Single => "bindings-single",
            Precision::Double => "bindings-double",
        };
        let unknown = match precision {
            Precision::Single => "bindings-wasm32-unknown-unknown-single",
            Precision::Double => "bindings-wasm32-unknown-unknown-double",
        };
        let wasip1 = match precision {
            Precision::Single => "bindings-wasm32-wasip1-single",
            Precision::Double => "bindings-wasm32-wasip1-double",
        };
        let rust_features = match precision {
            Precision::Single => Vec::new(),
            Precision::Double => vec!["double-precision".to_owned()],
        };
        for (provider, artifact, rust_target) in [
            (
                ArtifactProvider::Source,
                native,
                RustTarget::X86_64UnknownLinuxGnu,
            ),
            (
                ArtifactProvider::SystemStatic,
                native,
                RustTarget::X86_64UnknownLinuxGnu,
            ),
            (
                ArtifactProvider::PrebuiltStatic,
                native,
                RustTarget::X86_64UnknownLinuxGnu,
            ),
            (
                ArtifactProvider::WasmRuntime,
                unknown,
                RustTarget::Wasm32UnknownUnknown,
            ),
            (
                ArtifactProvider::WasmCompileOnly,
                wasip1,
                RustTarget::Wasm32Wasip1,
            ),
        ] {
            routes.push(BindingRoute {
                mode: precision,
                provider,
                artifact: artifact.to_owned(),
                rust_target,
                rust_features: rust_features.clone(),
            });
        }
    }
    routes
}

fn validate_route_refresh_topology(manifest: &UpstreamManifest) -> Result<()> {
    let expected_routes = canonical_binding_routes();
    if manifest.binding_routes != expected_routes {
        return Err(Error::message(format!(
            "route refresh requires the exact canonical 10-route matrix; observed {:?}, expected {:?}",
            manifest.binding_routes, expected_routes
        )));
    }

    let expected = canonical_route_binding_artifacts()
        .into_iter()
        .map(|artifact| {
            (
                artifact.name,
                artifact.path,
                artifact.precision,
                artifact.target,
                artifact.provider,
                artifact.producer,
            )
        })
        .collect::<BTreeSet<_>>();
    let observed = manifest
        .binding_artifacts()
        .map(|artifact| {
            (
                artifact.name.clone(),
                artifact.path.clone(),
                artifact.precision,
                artifact.target,
                artifact.provider,
                artifact.producer,
            )
        })
        .collect::<BTreeSet<_>>();
    if observed != expected {
        return Err(Error::message(format!(
            "route refresh bindings topology is incomplete or incorrect; observed {observed:?}, expected {expected:?}"
        )));
    }
    Ok(())
}

fn canonical_route_refresh_manifest(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<UpstreamManifest> {
    if manifest.next_revision.is_some()
        || !manifest.next_binding_routes.is_empty()
        || !manifest.next_artifacts.is_empty()
        || manifest.next_inventory.is_some()
    {
        return Err(Error::message(
            "route refresh cannot run while an upstream revision transition is registered",
        ));
    }
    if manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.candidate_path.is_some() || artifact.candidate_blake3.is_some())
    {
        return Err(Error::message(
            "route refresh cannot run while reviewed artifact candidates are registered",
        ));
    }
    if manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.path == "boxdd-sys/upstream.toml")
    {
        return Err(Error::message(
            "route refresh artifacts cannot alias the upstream manifest",
        ));
    }
    if manifest.recording_revision != manifest.active_revision {
        return Err(Error::message(
            "route refresh requires recording_revision to equal active_revision",
        ));
    }

    let mut target = manifest.clone();
    target
        .artifacts
        .retain(|artifact| artifact.kind != ArtifactKind::Bindings);
    let mut bindings = canonical_route_binding_artifacts();
    bindings.append(&mut target.artifacts);
    target.artifacts = bindings;
    target.binding_routes = canonical_binding_routes();
    for artifact in &mut target.artifacts {
        artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
    }
    target.artifact_digests_initialized = false;

    validate_manifest(&target)?;
    validate_binding_route_feature_catalog(paths, &target.binding_routes)?;
    validate_route_refresh_topology(&target)?;
    Ok(target)
}

fn validate_binding_route_feature_catalog(
    paths: &WorkspacePaths,
    routes: &[BindingRoute],
) -> Result<()> {
    let mut errors = Vec::new();
    for route in routes {
        if let Err(error) = expanded_binding_route_features(paths, &route.rust_features) {
            errors.push(format!(
                "binding route {}/{} has invalid Rust features: {error}",
                route.mode.as_str(),
                route.provider.as_str()
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

pub(crate) fn expanded_binding_route_features(
    paths: &WorkspacePaths,
    requested: &[String],
) -> Result<BTreeSet<String>> {
    let cargo_manifest_path = paths.root().join("boxdd/Cargo.toml");
    let cargo_manifest: toml::Value = read_toml(&cargo_manifest_path)?;
    let features = cargo_manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            Error::message(format!(
                "{} has no [features] table",
                cargo_manifest_path.display()
            ))
        })?;
    let mut expanded = BTreeSet::new();
    let mut pending = requested.to_vec();
    while let Some(feature) = pending.pop() {
        let definition = features
            .get(&feature)
            .ok_or_else(|| Error::message(format!("unknown boxdd Rust feature `{feature}`")))?;
        if !expanded.insert(feature.clone()) {
            continue;
        }
        let edges = definition.as_array().ok_or_else(|| {
            Error::message(format!("boxdd Rust feature `{feature}` must be an array"))
        })?;
        for edge in edges {
            let edge = edge.as_str().ok_or_else(|| {
                Error::message(format!(
                    "boxdd Rust feature `{feature}` contains a non-string dependency"
                ))
            })?;
            if edge.starts_with("dep:") || edge.contains('/') {
                continue;
            }
            if features.contains_key(edge) {
                pending.push(edge.to_owned());
            }
        }
    }
    Ok(expanded)
}

fn validate_recording_input_shape(inputs: &[RecordingInputIdentity], errors: &mut Vec<String>) {
    let actual = inputs
        .iter()
        .map(|input| input.path.as_str())
        .collect::<Vec<_>>();
    if actual != crate::recording_wire::REVIEWED_RECORDING_INPUT_PATHS {
        errors.push(format!(
            "recording inputs must be the exact sorted reviewed set {:?}, observed {actual:?}",
            crate::recording_wire::REVIEWED_RECORDING_INPUT_PATHS
        ));
    }
    for input in inputs {
        if !is_canonical_manifest_path(&input.path) {
            errors.push(format!(
                "recording input `{}` is not a canonical relative path",
                input.path
            ));
        }
        if !is_blake3(&input.blake3) {
            errors.push(format!(
                "recording input `{}` must have a lowercase 64-character BLAKE3 digest",
                input.path
            ));
        }
        if !is_full_sha(&input.git_blob) {
            errors.push(format!(
                "recording input `{}` must have a lowercase 40-character Git blob ID",
                input.path
            ));
        }
    }
}

fn validate_artifacts(artifacts: &[GeneratedArtifact], errors: &mut Vec<String>) {
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut case_folded_paths = BTreeSet::new();
    let mut coordinates = BTreeSet::new();
    let mut kinds = BTreeMap::<ArtifactKind, usize>::new();
    for artifact in artifacts {
        *kinds.entry(artifact.kind).or_default() += 1;
        if !is_artifact_name(&artifact.name) {
            errors.push(format!(
                "artifact name `{}` is not kebab-case ASCII",
                artifact.name
            ));
        }
        if !names.insert(&artifact.name) {
            errors.push(format!("duplicate artifact name `{}`", artifact.name));
        }
        if !paths.insert(&artifact.path) {
            errors.push(format!("duplicate artifact path `{}`", artifact.path));
        }
        validate_artifact_path_reservation(&artifact.path, &mut case_folded_paths, errors);
        if let Some(candidate) = &artifact.candidate_path {
            if !paths.insert(candidate) {
                errors.push(format!("duplicate artifact/candidate path `{candidate}`"));
            }
            if !is_canonical_manifest_path(candidate) {
                errors.push(format!(
                    "artifact candidate path `{candidate}` is not a canonical relative path"
                ));
            }
            validate_artifact_path_reservation(candidate, &mut case_folded_paths, errors);
        }
        let coordinate = (
            artifact.kind,
            artifact.precision,
            artifact.target,
            artifact.provider,
        );
        if !coordinates.insert(coordinate) {
            errors.push(format!(
                "duplicate artifact coordinate {:?}/{:?}/{:?}/{:?}",
                artifact.kind, artifact.precision, artifact.target, artifact.provider
            ));
        }
        if !is_canonical_manifest_path(&artifact.path) {
            errors.push(format!(
                "artifact path `{}` is not a canonical relative path",
                artifact.path
            ));
        }
        match (artifact.kind, artifact.precision) {
            (
                ArtifactKind::Bindings | ArtifactKind::AbiMetadata | ArtifactKind::ProviderIdentity,
                Some(_),
            ) => {}
            (
                ArtifactKind::Bindings | ArtifactKind::AbiMetadata | ArtifactKind::ProviderIdentity,
                None,
            ) => {
                errors.push(format!(
                    "precision-specific artifact `{}` has no precision",
                    artifact.name
                ));
            }
            (_, Some(_)) => errors.push(format!(
                "non-bindings artifact `{}` cannot declare precision",
                artifact.name
            )),
            (_, None) => {}
        }
        match (artifact.kind, artifact.producer) {
            (ArtifactKind::Bindings, ArtifactProducer::Bindgen)
            | (ArtifactKind::ApiContract, ArtifactProducer::Reviewed)
            | (ArtifactKind::RecordingWire, ArtifactProducer::ApiCoverage)
            | (ArtifactKind::ApiCoverageReport, ArtifactProducer::ApiCoverage)
            | (ArtifactKind::AbiMetadata, ArtifactProducer::AbiProbe)
            | (ArtifactKind::ProviderIdentity, ArtifactProducer::ProviderAttestation) => {}
            (kind, producer) => errors.push(format!(
                "artifact `{}` has incompatible kind {kind:?} and producer {producer:?}",
                artifact.name
            )),
        }
        if artifact.candidate_path.is_some() && artifact.producer != ArtifactProducer::Reviewed {
            errors.push(format!(
                "only reviewed artifact `{}` may declare candidate_path",
                artifact.name
            ));
        }
        if !is_blake3(&artifact.content_blake3) {
            errors.push(format!(
                "artifact `{}` content_blake3 must be a lowercase 64-character BLAKE3 digest",
                artifact.name
            ));
        }
        match (&artifact.candidate_path, &artifact.candidate_blake3) {
            (Some(_), Some(digest)) if !is_blake3(digest) => errors.push(format!(
                "artifact `{}` candidate_blake3 must be a lowercase 64-character BLAKE3 digest",
                artifact.name
            )),
            (Some(_), None) => errors.push(format!(
                "artifact `{}` has candidate_path without candidate_blake3",
                artifact.name
            )),
            (None, Some(_)) => errors.push(format!(
                "artifact `{}` has candidate_blake3 without candidate_path",
                artifact.name
            )),
            _ => {}
        }
        if matches!(
            artifact.provider,
            ArtifactProvider::WasmRuntime | ArtifactProvider::WasmCompileOnly
        ) && matches!(artifact.target, ArtifactTarget::Native)
        {
            errors.push(format!(
                "artifact `{}` assigns a WASM provider to a native target",
                artifact.name
            ));
        }
    }
    if kinds
        .get(&ArtifactKind::Bindings)
        .copied()
        .unwrap_or_default()
        == 0
    {
        errors.push("upstream manifest has no bindings artifacts".to_owned());
    }
    for required in [
        ArtifactKind::ApiContract,
        ArtifactKind::RecordingWire,
        ArtifactKind::ApiCoverageReport,
    ] {
        if kinds.get(&required).copied().unwrap_or_default() != 1 {
            errors.push(format!(
                "upstream manifest must contain exactly one {required:?} artifact"
            ));
        }
    }
    validate_provider_identity_topology(artifacts, errors);
}

fn validate_provider_identity_topology(artifacts: &[GeneratedArtifact], errors: &mut Vec<String>) {
    let observed = artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
        .map(|artifact| {
            (
                artifact.name.as_str(),
                artifact.path.as_str(),
                artifact.precision,
                artifact.target,
                artifact.provider,
                artifact.producer,
            )
        })
        .collect::<BTreeSet<_>>();
    if observed.is_empty() {
        return;
    }
    let expected = BTreeSet::from([
        (
            "wasm-provider-identity-single",
            "boxdd-sys/abi/wasm32-unknown-unknown-single.toml",
            Some(Precision::Single),
            ArtifactTarget::Wasm32UnknownUnknown,
            ArtifactProvider::WasmRuntime,
            ArtifactProducer::ProviderAttestation,
        ),
        (
            "wasm-provider-identity-double",
            "boxdd-sys/abi/wasm32-unknown-unknown-double.toml",
            Some(Precision::Double),
            ArtifactTarget::Wasm32UnknownUnknown,
            ArtifactProvider::WasmRuntime,
            ArtifactProducer::ProviderAttestation,
        ),
    ]);
    if observed != expected {
        errors.push(format!(
            "WASM provider identity topology must contain the exact single/double contract pair; observed {observed:?}, expected {expected:?}"
        ));
    }
}

pub(super) fn require_provider_identity_topology(manifest: &UpstreamManifest) -> Result<()> {
    let mut errors = Vec::new();
    validate_provider_identity_topology(&manifest.artifacts, &mut errors);
    if !manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
    {
        errors.push("upstream manifest has no WASM provider identity artifacts".to_owned());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn validate_artifact_path_reservation(
    path: &str,
    case_folded_paths: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let folded = path.to_ascii_lowercase();
    if !case_folded_paths.insert(folded) {
        errors.push(format!(
            "artifact paths collide on case-insensitive filesystems at `{path}`"
        ));
    }
    if path == "boxdd-sys/upstream.toml"
        || path == BOX2D_GITLINK
        || Path::new(path).starts_with(BOX2D_GITLINK)
    {
        errors.push(format!(
            "artifact path `{path}` overlaps the manifest or Box2D gitlink"
        ));
    }
}

fn validate_inventory_shape(inventory: &SourceInventory, errors: &mut Vec<String>) {
    if !is_full_sha(&inventory.tree) {
        errors.push("source inventory tree must be a lowercase 40-character Git SHA".to_owned());
    }
    validate_inventory_group("c_sources", &inventory.c_sources, "src", "c", true, errors);
    validate_inventory_group(
        "private_headers",
        &inventory.private_headers,
        "src",
        "h",
        true,
        errors,
    );
    validate_inventory_group(
        "inline_files",
        &inventory.inline_files,
        "src",
        "inl",
        false,
        errors,
    );
    validate_inventory_group(
        "public_headers",
        &inventory.public_headers,
        "include/box2d",
        "h",
        true,
        errors,
    );
    let mut globally_seen = BTreeMap::<&str, &str>::new();
    for (group, paths) in [
        ("c_sources", &inventory.c_sources),
        ("private_headers", &inventory.private_headers),
        ("inline_files", &inventory.inline_files),
        ("public_headers", &inventory.public_headers),
    ] {
        for path in paths {
            if let Some(previous_group) = globally_seen.insert(path, group) {
                errors.push(format!(
                    "source inventory path `{path}` appears in both {previous_group} and {group}"
                ));
            }
        }
    }
}

fn validate_inventory_group(
    label: &str,
    paths: &[String],
    parent: &str,
    extension: &str,
    required: bool,
    errors: &mut Vec<String>,
) {
    if required && paths.is_empty() {
        errors.push(format!("source inventory {label} is empty"));
    }
    if !paths.windows(2).all(|pair| pair[0] < pair[1]) {
        errors.push(format!(
            "source inventory {label} must be sorted and unique"
        ));
    }
    for path in paths {
        let candidate = Path::new(path);
        if !is_canonical_manifest_path(path)
            || !candidate.starts_with(parent)
            || candidate == Path::new(parent)
            || candidate.extension().and_then(|value| value.to_str()) != Some(extension)
        {
            errors.push(format!(
                "source inventory {label} has invalid path `{path}`"
            ));
        }
    }
}

fn apply_update(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let (manifest, baseline) = load_manifest_snapshot(paths, true)?;
    let target = manifest
        .next_revision
        .as_deref()
        .ok_or_else(|| Error::message("upstream manifest has no next_revision to apply"))?;
    require_provider_identity_topology(&manifest)?;
    super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
    validate_update_preconditions(paths, &manifest)?;

    let staging =
        IsolatedGeneration::create_at(paths, &baseline.generation.repository_revision, target)?;
    let staged_result = staging.prepare_update(&manifest, target);
    let cleanup_result = staging.finish();
    let staged = match (staged_result, cleanup_result) {
        (Ok(staged), Ok(())) => staged,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(cleanup)) => return Err(cleanup),
        (Err(error), Err(cleanup)) => {
            return Err(Error::message(format!(
                "artifact staging failed: {error}\nisolated worktree cleanup also failed: {cleanup}"
            )));
        }
    };

    // Close the race between isolated staging and the mutation phase.
    validate_update_preconditions(paths, &manifest)?;
    baseline.verify_all(paths)?;
    install_staged_update(paths, &manifest, &staged, &baseline, None)?;
    println!(
        "updated Box2D from {} to {target}; the gitlink, manifest, bindings, API contract, recording contract, and generated report agree",
        manifest.active_revision
    );
    Ok(())
}

fn validate_update_preconditions(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    validate_repository(paths, manifest, true)?;
    let _ = manifest
        .next_revision
        .as_deref()
        .ok_or_else(|| Error::message("upstream manifest has no next_revision to apply"))?;
    for artifact in manifest.reviewed_artifacts() {
        if artifact.candidate_path.is_none() {
            return Err(Error::message(format!(
                "reviewed artifact `{}` has no target candidate_path",
                artifact.name
            )));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct UpdateLock {
    path: PathBuf,
    file: Option<fs::File>,
}

impl UpdateLock {
    pub(crate) fn acquire(root: &Path) -> Result<Self> {
        let path = Self::lock_path(root)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| Error::io(&path, source))?;
        let verified_path = Self::lock_path(root)?;
        if verified_path != path {
            return Err(Error::message(format!(
                "upstream-sync lock path changed while opening it: expected {}, found {}",
                path.display(),
                verified_path.display()
            )));
        }
        file.try_lock().map_err(|source| {
            Error::message(format!(
                "could not acquire upstream-sync lock {}: {source}; another update may be running",
                path.display()
            ))
        })?;
        let mut lock = Self {
            path,
            file: Some(file),
        };
        let file = lock.file.as_mut().expect("newly acquired lock file");
        file.set_len(0)
            .map_err(|source| Error::io(&lock.path, source))?;
        writeln!(file, "pid={}", std::process::id())
            .map_err(|source| Error::io(&lock.path, source))?;
        file.sync_all()
            .map_err(|source| Error::io(&lock.path, source))?;
        recover_atomic_batches(root)?;
        cleanup_deferred_isolated_generations(root)?;
        Ok(lock)
    }

    fn lock_path(root: &Path) -> Result<PathBuf> {
        repository_lock_path(root, Path::new("boxdd-upstream-sync.lock")).map_err(Error::message)
    }
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

struct ManagedSnapshot {
    files: Vec<FileBackup>,
    gitlink_revision: String,
    git_index: GitIndexSnapshot,
    checkout: CheckoutState,
    generation: GenerationBaseline,
}

impl ManagedSnapshot {
    fn capture(paths: &WorkspacePaths, manifest: &UpstreamManifest) -> Result<Self> {
        Self::capture_with(paths, manifest, true)
    }

    fn capture_observed(paths: &WorkspacePaths, manifest: &UpstreamManifest) -> Result<Self> {
        Self::capture_with(paths, manifest, false)
    }

    fn capture_with(
        paths: &WorkspacePaths,
        manifest: &UpstreamManifest,
        require_clean_generator_inputs: bool,
    ) -> Result<Self> {
        let mut managed = BTreeSet::from([paths.upstream_manifest()]);
        for artifact in manifest.artifacts.iter().chain(&manifest.next_artifacts) {
            managed.insert(paths.root().join(&artifact.path));
            if let Some(candidate) = &artifact.candidate_path {
                managed.insert(paths.root().join(candidate));
            }
        }
        let files = managed
            .into_iter()
            .map(FileBackup::capture)
            .collect::<Result<Vec<_>>>()?;
        let git_index = GitIndexSnapshot::capture(paths.root())?;
        Ok(Self {
            files,
            gitlink_revision: indexed_gitlink_from_snapshot(paths.root(), &git_index)?,
            git_index,
            checkout: checkout_state(&paths.box2d())?,
            generation: GenerationBaseline::capture_with_policy(
                paths.root(),
                require_clean_generator_inputs,
            )?,
        })
    }

    fn verify_loaded_state(
        &self,
        paths: &WorkspacePaths,
        manifest_content: &[u8],
        loaded_generation: &GenerationBaseline,
    ) -> Result<()> {
        let captured_manifest = self.manifest_content(paths)?;
        if captured_manifest != manifest_content {
            return Err(Error::message(
                "upstream manifest changed between loading and preflight snapshot capture",
            ));
        }
        if self.generation.repository_revision != loaded_generation.repository_revision {
            return Err(Error::message(format!(
                "repository HEAD changed between manifest loading and preflight snapshot capture: expected {}, observed {}",
                loaded_generation.repository_revision, self.generation.repository_revision
            )));
        }
        if self.generation.input_tree != loaded_generation.input_tree
            || self.generation.worktree_blake3 != loaded_generation.worktree_blake3
        {
            return Err(Error::message(
                "upstream generator inputs changed between manifest loading and preflight snapshot capture",
            ));
        }
        Ok(())
    }

    fn manifest_content<'a>(&'a self, paths: &WorkspacePaths) -> Result<&'a [u8]> {
        let manifest_path = paths.upstream_manifest();
        self.files
            .iter()
            .find(|file| file.path == manifest_path)
            .and_then(|file| file.content.as_deref())
            .ok_or_else(|| Error::message("upstream manifest was absent from the snapshot"))
    }

    fn verify_all(&self, paths: &WorkspacePaths) -> Result<()> {
        for file in &self.files {
            self.verify_path(&file.path)?;
        }
        self.verify_gitlink(paths)?;
        self.verify_checkout(paths)?;
        self.generation.verify(paths.root())
    }

    fn verify_path(&self, path: &Path) -> Result<()> {
        let expected = self
            .files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| {
                Error::message(format!(
                    "managed path {} was absent from the preflight snapshot",
                    path.display()
                ))
            })?;
        let actual = FileBackup::capture(path.to_owned())?;
        if actual.content != expected.content {
            return Err(Error::message(format!(
                "managed path {} changed after upstream-sync preflight; refusing to overwrite it",
                path.display()
            )));
        }
        Ok(())
    }

    fn verify_gitlink(&self, paths: &WorkspacePaths) -> Result<()> {
        let actual_index = GitIndexSnapshot::capture(paths.root())?;
        if actual_index != self.git_index {
            return Err(Error::message(
                "root Git index changed after upstream-sync preflight; refusing to overwrite concurrent staged state",
            ));
        }
        let actual = indexed_gitlink_from_snapshot(paths.root(), &actual_index)?;
        if actual != self.gitlink_revision {
            return Err(Error::message(format!(
                "Box2D gitlink changed after upstream-sync preflight: expected {}, observed {actual}",
                self.gitlink_revision
            )));
        }
        Ok(())
    }

    fn verify_checkout(&self, paths: &WorkspacePaths) -> Result<()> {
        let actual = checkout_state(&paths.box2d())?;
        if actual != self.checkout {
            return Err(Error::message(format!(
                "Box2D checkout changed after upstream-sync preflight: expected {:?}, observed {actual:?}",
                self.checkout
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GenerationBaseline {
    repository_revision: String,
    input_tree: String,
    worktree_blake3: String,
    require_clean_inputs: bool,
}

impl GenerationBaseline {
    fn capture(root: &Path) -> Result<Self> {
        Self::capture_with_policy(root, true)
    }

    fn capture_with_policy(root: &Path, require_clean_inputs: bool) -> Result<Self> {
        let repository_revision = git_output(root, ["rev-parse", "HEAD"])?.trim().to_owned();
        let dirty = git_output_with_paths(
            root,
            &[
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
                "--ignored=matching",
                "--",
            ],
            GENERATOR_INPUT_PATHS,
        )?;
        if require_clean_inputs && !dirty.trim().is_empty() {
            return Err(Error::message(format!(
                "upstream generator inputs are dirty; commit generator changes before producing revision-coupled artifacts:\n{dirty}"
            )));
        }
        let input_tree = git_output_with_paths(
            root,
            &["ls-tree", "-r", "--full-tree", &repository_revision, "--"],
            GENERATOR_INPUT_PATHS,
        )?;
        Ok(Self {
            repository_revision,
            input_tree,
            worktree_blake3: generator_worktree_blake3(root)?,
            require_clean_inputs,
        })
    }

    fn verify(&self, root: &Path) -> Result<()> {
        let actual_revision = git_output(root, ["rev-parse", "HEAD"])?.trim().to_owned();
        if actual_revision != self.repository_revision {
            return Err(Error::message(format!(
                "repository HEAD changed during upstream generation: expected {}, observed {actual_revision}",
                self.repository_revision
            )));
        }
        let actual = Self::capture_with_policy(root, self.require_clean_inputs)?;
        if actual.input_tree != self.input_tree {
            return Err(Error::message(
                "upstream generator input identities changed during generation",
            ));
        }
        if actual.worktree_blake3 != self.worktree_blake3 {
            return Err(Error::message(
                "upstream generator input contents changed during generation",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GeneratorInputEntry {
    Directory,
    File(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratorInputSnapshot {
    repository_revision: String,
    entries: BTreeMap<String, GeneratorInputEntry>,
}

impl GeneratorInputSnapshot {
    fn capture(root: &Path) -> Result<Self> {
        let repository_revision = git_output(root, ["rev-parse", "HEAD"])?.trim().to_owned();
        let mut entries = BTreeMap::new();
        for relative in GENERATOR_INPUT_PATHS {
            collect_generator_snapshot_entries(root, &root.join(relative), &mut entries)?;
        }
        Ok(Self {
            repository_revision,
            entries,
        })
    }

    fn verify(&self, root: &Path) -> Result<()> {
        self.verify_excluding(root, &BTreeSet::new())
    }

    fn verify_excluding(&self, root: &Path, excluded: &BTreeSet<String>) -> Result<()> {
        let actual = Self::capture(root)?;
        if actual.repository_revision != self.repository_revision {
            return Err(Error::message(format!(
                "repository HEAD changed during route refresh: expected {}, observed {}",
                self.repository_revision, actual.repository_revision
            )));
        }
        let paths = self
            .entries
            .keys()
            .chain(actual.entries.keys())
            .filter(|path| !snapshot_path_is_excluded(path, excluded))
            .collect::<BTreeSet<_>>();
        for path in paths {
            let expected = self.entries.get(path);
            let observed = actual.entries.get(path);
            if expected != observed {
                return Err(Error::message(format!(
                    "controlled route generator input `{path}` changed after the refresh snapshot was captured: expected {}, observed {}",
                    describe_generator_input_entry(expected),
                    describe_generator_input_entry(observed)
                )));
            }
        }
        Ok(())
    }

    fn overlay(&self, destination_root: &Path) -> Result<()> {
        for relative in GENERATOR_INPUT_PATHS {
            remove_overlay_path(&destination_root.join(relative))?;
        }
        for (relative, entry) in &self.entries {
            let destination = destination_root.join(relative);
            match entry {
                GeneratorInputEntry::Directory => fs::create_dir_all(&destination)
                    .map_err(|source| Error::io(&destination, source))?,
                GeneratorInputEntry::File(content) => {
                    let parent = destination.parent().ok_or_else(|| {
                        Error::message(format!(
                            "controlled generator input {} has no parent",
                            destination.display()
                        ))
                    })?;
                    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
                    write_atomic_bytes(&destination, content)?;
                }
            }
        }
        Ok(())
    }
}

fn snapshot_path_is_excluded(path: &str, excluded: &BTreeSet<String>) -> bool {
    excluded.iter().any(|prefix| {
        path == prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn describe_generator_input_entry(entry: Option<&GeneratorInputEntry>) -> String {
    match entry {
        Some(GeneratorInputEntry::Directory) => "directory".to_owned(),
        Some(GeneratorInputEntry::File(content)) => {
            format!("file blake3 {}", blake3_bytes(content))
        }
        None => "missing".to_owned(),
    }
}

fn collect_generator_snapshot_entries(
    root: &Path,
    path: &Path,
    entries: &mut BTreeMap<String, GeneratorInputEntry>,
) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(path, error)),
    };
    let relative = path.strip_prefix(root).map_err(|_| {
        Error::message(format!(
            "controlled generator input {} is outside repository root {}",
            path.display(),
            root.display()
        ))
    })?;
    let relative = canonical_manifest_path(relative).ok_or_else(|| {
        Error::message(format!(
            "controlled generator input {} is not canonical UTF-8",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "controlled generator input {} is a symlink; route refresh accepts only regular files and directories",
            path.display()
        )));
    }
    if metadata.is_file() {
        let content = fs::read(path).map_err(|source| Error::io(path, source))?;
        entries.insert(relative, GeneratorInputEntry::File(content));
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(Error::message(format!(
            "controlled generator input {} is neither a regular file nor a directory",
            path.display()
        )));
    }

    entries.insert(relative, GeneratorInputEntry::Directory);
    let mut children = fs::read_dir(path)
        .map_err(|source| Error::io(path, source))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|source| Error::io(path, source))?;
    children.sort();
    for child in children {
        collect_generator_snapshot_entries(root, &child, entries)?;
    }
    Ok(())
}

fn remove_overlay_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(path, error)),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path).map_err(|source| Error::io(path, source))
    } else {
        fs::remove_file(path).map_err(|source| Error::io(path, source))
    }
}

fn generator_worktree_blake3(root: &Path) -> Result<String> {
    let mut entries = Vec::<(String, PathBuf, u8)>::new();
    for relative in GENERATOR_INPUT_PATHS {
        collect_generator_input_entries(root, &root.join(relative), &mut entries)?;
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"boxdd-upstream-generator-worktree-v1\0");
    for (relative, path, kind) in entries {
        hasher.update(&[kind]);
        hasher.update(&(relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        match kind {
            b'f' => {
                let content = fs::read(&path).map_err(|source| Error::io(&path, source))?;
                hasher.update(&(content.len() as u64).to_le_bytes());
                hasher.update(&content);
            }
            b'l' => {
                let target = fs::read_link(&path).map_err(|source| Error::io(&path, source))?;
                let target = target.to_string_lossy();
                hasher.update(&(target.len() as u64).to_le_bytes());
                hasher.update(target.as_bytes());
            }
            b'd' | b'm' => {}
            _ => unreachable!("generator input kind is controlled by the collector"),
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn collect_generator_input_entries(
    root: &Path,
    path: &Path,
    entries: &mut Vec<(String, PathBuf, u8)>,
) -> Result<()> {
    let relative = path.strip_prefix(root).map_err(|_| {
        Error::message(format!(
            "generator input {} is outside repository root {}",
            path.display(),
            root.display()
        ))
    })?;
    let relative = canonical_manifest_path(relative).ok_or_else(|| {
        Error::message(format!(
            "generator input path {} is not canonical UTF-8",
            path.display()
        ))
    })?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            entries.push((relative, path.to_owned(), b'm'));
            return Ok(());
        }
        Err(error) => return Err(Error::io(path, error)),
    };
    let kind = if metadata.file_type().is_file() {
        b'f'
    } else if metadata.file_type().is_dir() {
        b'd'
    } else if metadata.file_type().is_symlink() {
        b'l'
    } else {
        return Err(Error::message(format!(
            "generator input {} has an unsupported file type",
            path.display()
        )));
    };
    entries.push((relative, path.to_owned(), kind));
    if kind == b'd' {
        for entry in fs::read_dir(path).map_err(|source| Error::io(path, source))? {
            let entry = entry.map_err(|source| Error::io(path, source))?;
            collect_generator_input_entries(root, &entry.path(), entries)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct StagedFile {
    relative_path: String,
    content: Vec<u8>,
}

#[derive(Debug)]
struct StagedUpdate {
    manifest: UpstreamManifest,
    artifacts: Vec<StagedFile>,
    candidate_paths: Vec<String>,
}

#[derive(Debug)]
struct RouteRefreshStaging {
    manifest: UpstreamManifest,
    files: Vec<StagedFile>,
    removals: Vec<String>,
}

struct RouteRefreshBaseline {
    inputs: GeneratorInputSnapshot,
    manifest: FileBackup,
    outputs: Vec<FileBackup>,
    gitlink_revision: String,
    checkout: CheckoutState,
}

impl RouteRefreshBaseline {
    fn capture(
        paths: &WorkspacePaths,
        inputs: GeneratorInputSnapshot,
        expected_manifest: &[u8],
        original: &UpstreamManifest,
        target: &UpstreamManifest,
    ) -> Result<Self> {
        let manifest = FileBackup::capture(paths.upstream_manifest())?;
        if manifest.content.as_deref() != Some(expected_manifest) {
            return Err(Error::message(
                "upstream manifest changed while the route refresh snapshot was being captured",
            ));
        }
        let mut relative_paths = original
            .artifacts
            .iter()
            .chain(&target.artifacts)
            .map(|artifact| artifact.path.clone())
            .collect::<BTreeSet<_>>();
        relative_paths.insert(super::api_coverage::RUNTIME_RECORDING_WIRE_PATH.to_owned());
        let outputs = relative_paths
            .into_iter()
            .map(|relative| {
                if !is_canonical_manifest_path(&relative) {
                    return Err(Error::message(format!(
                        "route refresh output `{relative}` is not a canonical relative path"
                    )));
                }
                let path = paths.root().join(relative);
                let parent = path.parent().ok_or_else(|| {
                    Error::message(format!(
                        "route refresh output {} has no parent directory",
                        path.display()
                    ))
                })?;
                let metadata =
                    fs::symlink_metadata(parent).map_err(|source| Error::io(parent, source))?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(Error::message(format!(
                        "route refresh output parent {} must be an existing non-symlink directory",
                        parent.display()
                    )));
                }
                FileBackup::capture(path)
            })
            .collect::<Result<Vec<_>>>()?;
        let baseline = Self {
            inputs,
            manifest,
            outputs,
            gitlink_revision: indexed_gitlink(paths.root())?,
            checkout: checkout_state(&paths.box2d())?,
        };
        baseline.verify_before_install(paths)?;
        Ok(baseline)
    }

    fn verify_before_install(&self, paths: &WorkspacePaths) -> Result<()> {
        self.inputs.verify(paths.root())?;
        self.verify_manifest_original()?;
        for output in &self.outputs {
            verify_file_backup(output)?;
        }
        self.verify_repository_coordinates(paths)
    }

    fn verify_stable_inputs(
        &self,
        paths: &WorkspacePaths,
        managed_paths: &BTreeSet<String>,
    ) -> Result<()> {
        self.inputs.verify_excluding(paths.root(), managed_paths)?;
        self.verify_repository_coordinates(paths)
    }

    fn verify_manifest_original(&self) -> Result<()> {
        verify_file_backup(&self.manifest).map_err(|error| {
            Error::message(format!(
                "upstream manifest changed after route refresh preflight: {error}"
            ))
        })
    }

    fn verify_repository_coordinates(&self, paths: &WorkspacePaths) -> Result<()> {
        let gitlink = indexed_gitlink(paths.root())?;
        if gitlink != self.gitlink_revision {
            return Err(Error::message(format!(
                "Box2D gitlink changed during route refresh: expected {}, observed {gitlink}",
                self.gitlink_revision
            )));
        }
        let checkout = checkout_state(&paths.box2d())?;
        if checkout != self.checkout {
            return Err(Error::message(format!(
                "Box2D checkout changed during route refresh: expected {:?}, observed {checkout:?}",
                self.checkout
            )));
        }
        Ok(())
    }

    fn output(&self, path: &Path) -> Result<&FileBackup> {
        self.outputs
            .iter()
            .find(|output| output.path == path)
            .ok_or_else(|| {
                Error::message(format!(
                    "route refresh path {} was absent from the output snapshot",
                    path.display()
                ))
            })
    }
}

fn verify_file_backup(expected: &FileBackup) -> Result<()> {
    let actual = FileBackup::capture(expected.path.clone())?;
    if actual.content != expected.content {
        return Err(Error::message(format!(
            "managed route refresh path {} changed after preflight",
            expected.path.display()
        )));
    }
    Ok(())
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub enum ManagedArtifactDestination {
    Active,
    ReviewedActive,
    ReviewedCandidate {
        path: String,
    },
    /// A generated file that participates in the same CAS/rollback transaction
    /// but is intentionally not represented as an upstream manifest artifact.
    Auxiliary {
        path: String,
    },
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct ManagedArtifactWrite {
    pub artifact_name: String,
    pub destination: ManagedArtifactDestination,
    pub content: Vec<u8>,
    reviewed_baseline_blake3: Option<String>,
}

impl ManagedArtifactWrite {
    pub fn active(artifact_name: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            artifact_name: artifact_name.into(),
            destination: ManagedArtifactDestination::Active,
            content,
            reviewed_baseline_blake3: None,
        }
    }

    pub fn reviewed_candidate(
        artifact_name: impl Into<String>,
        path: impl Into<String>,
        content: Vec<u8>,
    ) -> Self {
        Self {
            artifact_name: artifact_name.into(),
            destination: ManagedArtifactDestination::ReviewedCandidate { path: path.into() },
            content,
            reviewed_baseline_blake3: None,
        }
    }

    pub fn reviewed_active(artifact_name: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            artifact_name: artifact_name.into(),
            destination: ManagedArtifactDestination::ReviewedActive,
            content,
            reviewed_baseline_blake3: None,
        }
    }

    pub(crate) fn reviewed_active_with_baseline_blake3(
        artifact_name: impl Into<String>,
        content: Vec<u8>,
        reviewed_baseline_blake3: impl Into<String>,
    ) -> Self {
        Self {
            artifact_name: artifact_name.into(),
            destination: ManagedArtifactDestination::ReviewedActive,
            content,
            reviewed_baseline_blake3: Some(reviewed_baseline_blake3.into()),
        }
    }

    pub fn auxiliary(path: impl Into<String>, content: Vec<u8>) -> Self {
        let path = path.into();
        Self {
            artifact_name: format!("auxiliary:{path}"),
            destination: ManagedArtifactDestination::Auxiliary { path },
            content,
            reviewed_baseline_blake3: None,
        }
    }
}

#[doc(hidden)]
pub fn install_managed_artifact_writes<F>(
    paths: &WorkspacePaths,
    writes: &[ManagedArtifactWrite],
    terminal_validation: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let _lock = UpdateLock::acquire(paths.root())?;
    install_managed_artifact_writes_locked(paths, writes, None, terminal_validation)
}

/// Installs generated outputs while the caller holds `UpdateLock` across generation and commit.
pub(crate) fn install_managed_artifact_writes_locked<F>(
    paths: &WorkspacePaths,
    writes: &[ManagedArtifactWrite],
    expected_manifest_content: Option<&[u8]>,
    terminal_validation: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    if writes.is_empty() {
        return Err(Error::message(
            "managed artifact transaction requires at least one output",
        ));
    }
    let manifest_path = paths.upstream_manifest();
    let manifest_baseline = FileBackup::capture(manifest_path.clone())?;
    if let Some(expected) = expected_manifest_content
        && manifest_baseline.content.as_deref() != Some(expected)
    {
        return Err(Error::message(format!(
            "{} changed while managed artifacts were being generated",
            manifest_path.display()
        )));
    }
    let original_manifest = UpstreamManifest::load(paths)?;
    let bootstrap = !original_manifest.artifact_digests_initialized;
    let mut validation_manifest = original_manifest.clone();
    for write in writes {
        let Some(reviewed_baseline_blake3) = &write.reviewed_baseline_blake3 else {
            continue;
        };
        if bootstrap {
            return Err(Error::message(
                "artifact digest bootstrap cannot accept a reviewed active baseline digest",
            ));
        }
        if !matches!(
            write.destination,
            ManagedArtifactDestination::ReviewedActive
        ) {
            return Err(Error::message(format!(
                "managed artifact `{}` supplies a reviewed baseline digest without a reviewed-active destination",
                write.artifact_name
            )));
        }
        if !is_blake3(reviewed_baseline_blake3) {
            return Err(Error::message(format!(
                "managed artifact `{}` reviewed baseline must be a lowercase 64-character BLAKE3 digest",
                write.artifact_name
            )));
        }
        let artifact = validation_manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.name == write.artifact_name)
            .ok_or_else(|| {
                Error::message(format!(
                    "managed artifact transaction references unknown artifact `{}`",
                    write.artifact_name
                ))
            })?;
        if artifact.producer != ArtifactProducer::Reviewed {
            return Err(Error::message(format!(
                "artifact `{}` is produced by {}, not reviewed",
                artifact.name,
                artifact.producer.as_str()
            )));
        }
        artifact.content_blake3.clone_from(reviewed_baseline_blake3);
    }
    if bootstrap {
        if original_manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.candidate_path.is_some())
        {
            return Err(Error::message(
                "artifact digest bootstrap does not accept reviewed candidates",
            ));
        }
        let expected = original_manifest
            .artifacts
            .iter()
            .filter_map(|artifact| match artifact.producer {
                ArtifactProducer::Reviewed => Some((artifact.name.as_str(), "reviewed-active")),
                ArtifactProducer::ApiCoverage => Some((artifact.name.as_str(), "active")),
                ArtifactProducer::Bindgen
                | ArtifactProducer::AbiProbe
                | ArtifactProducer::ProviderAttestation => None,
            })
            .collect::<BTreeSet<_>>();
        let actual = writes
            .iter()
            .map(|write| {
                let destination = match &write.destination {
                    ManagedArtifactDestination::Active => "active",
                    ManagedArtifactDestination::ReviewedActive => "reviewed-active",
                    ManagedArtifactDestination::ReviewedCandidate { .. } => "reviewed-candidate",
                    ManagedArtifactDestination::Auxiliary { .. } => "auxiliary",
                };
                (write.artifact_name.as_str(), destination)
            })
            .filter(|(_, destination)| *destination != "auxiliary")
            .collect::<BTreeSet<_>>();
        let managed_write_count = writes
            .iter()
            .filter(|write| {
                !matches!(
                    write.destination,
                    ManagedArtifactDestination::Auxiliary { .. }
                )
            })
            .count();
        if actual != expected || actual.len() != managed_write_count {
            return Err(Error::message(
                "artifact digest bootstrap must write exactly every reviewed active and api-coverage artifact with the matching destination",
            ));
        }
        reject_bootstrap_artifact_changes_if_present(paths, &original_manifest)?;
        validate_repository_without_artifact_digests(paths, &original_manifest)?;
    } else {
        validate_artifact_identities(paths, &validation_manifest)?;
        validate_candidate_identities(paths, &original_manifest)?;
    }

    let mut updated_manifest = original_manifest.clone();
    if bootstrap {
        for artifact in &mut updated_manifest.artifacts {
            artifact.content_blake3 = file_blake3(&paths.root().join(&artifact.path))?;
        }
        updated_manifest.artifact_digests_initialized = true;
    }
    let mut output_names = BTreeSet::new();
    let mut output_paths = BTreeSet::new();
    let mut baseline_digests = BTreeMap::new();
    let mut staged_files = Vec::with_capacity(writes.len());
    for write in writes {
        if !output_names.insert(write.artifact_name.as_str()) {
            return Err(Error::message(format!(
                "managed artifact transaction contains duplicate output `{}`",
                write.artifact_name
            )));
        }
        let digest = blake3::hash(&write.content).to_hex().to_string();
        let relative_path = match &write.destination {
            ManagedArtifactDestination::Active => {
                let artifact = updated_manifest
                    .artifacts
                    .iter_mut()
                    .find(|artifact| artifact.name == write.artifact_name)
                    .ok_or_else(|| {
                        Error::message(format!(
                            "managed artifact transaction references unknown artifact `{}`",
                            write.artifact_name
                        ))
                    })?;
                if artifact.producer != ArtifactProducer::ApiCoverage {
                    return Err(Error::message(format!(
                        "active artifact `{}` is produced by {}, not api-coverage",
                        artifact.name,
                        artifact.producer.as_str()
                    )));
                }
                let path = artifact.path.clone();
                baseline_digests.insert(path.clone(), Some(artifact.content_blake3.clone()));
                artifact.content_blake3 = digest;
                path
            }
            ManagedArtifactDestination::ReviewedActive => {
                let artifact = updated_manifest
                    .artifacts
                    .iter_mut()
                    .find(|artifact| artifact.name == write.artifact_name)
                    .ok_or_else(|| {
                        Error::message(format!(
                            "managed artifact transaction references unknown artifact `{}`",
                            write.artifact_name
                        ))
                    })?;
                if artifact.producer != ArtifactProducer::Reviewed {
                    return Err(Error::message(format!(
                        "active artifact `{}` is produced by {}, not reviewed",
                        artifact.name,
                        artifact.producer.as_str()
                    )));
                }
                let path = artifact.path.clone();
                baseline_digests.insert(
                    path.clone(),
                    Some(
                        write
                            .reviewed_baseline_blake3
                            .clone()
                            .unwrap_or_else(|| artifact.content_blake3.clone()),
                    ),
                );
                artifact.content_blake3 = digest;
                path
            }
            ManagedArtifactDestination::ReviewedCandidate { path } => {
                let artifact = updated_manifest
                    .artifacts
                    .iter_mut()
                    .find(|artifact| artifact.name == write.artifact_name)
                    .ok_or_else(|| {
                        Error::message(format!(
                            "managed artifact transaction references unknown artifact `{}`",
                            write.artifact_name
                        ))
                    })?;
                if artifact.producer != ArtifactProducer::Reviewed {
                    return Err(Error::message(format!(
                        "artifact `{}` is not a reviewed artifact and cannot receive a candidate",
                        artifact.name
                    )));
                }
                if updated_manifest.next_revision.is_none() {
                    return Err(Error::message(format!(
                        "reviewed artifact `{}` cannot receive a candidate without next_revision",
                        artifact.name
                    )));
                }
                if !is_canonical_manifest_path(path) {
                    return Err(Error::message(format!(
                        "reviewed artifact candidate path `{path}` is not a canonical relative path"
                    )));
                }
                let baseline_digest = match artifact.candidate_path.as_deref() {
                    Some(existing) if existing != path => {
                        return Err(Error::message(format!(
                            "reviewed artifact `{}` already declares candidate `{existing}`, not `{path}`",
                            artifact.name
                        )));
                    }
                    Some(_) => Some(artifact.candidate_blake3.clone().ok_or_else(|| {
                        Error::message(format!(
                            "reviewed artifact `{}` has candidate_path without candidate_blake3",
                            artifact.name
                        ))
                    })?),
                    None => {
                        let candidate_path = paths.root().join(path);
                        match fs::symlink_metadata(&candidate_path) {
                            Ok(_) => {
                                return Err(Error::message(format!(
                                    "undeclared candidate path {} already exists; refusing to overwrite it",
                                    candidate_path.display()
                                )));
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                            Err(error) => return Err(Error::io(&candidate_path, error)),
                        }
                        None
                    }
                };
                baseline_digests.insert(path.clone(), baseline_digest);
                artifact.candidate_path = Some(path.clone());
                artifact.candidate_blake3 = Some(digest);
                path.clone()
            }
            ManagedArtifactDestination::Auxiliary { path } => {
                if !is_canonical_manifest_path(path) {
                    return Err(Error::message(format!(
                        "auxiliary managed path `{path}` is not a canonical relative path"
                    )));
                }
                let absolute = paths.root().join(path);
                let baseline_digest = match fs::symlink_metadata(&absolute) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(Error::message(format!(
                            "auxiliary managed path {} is a symlink",
                            absolute.display()
                        )));
                    }
                    Ok(_) => Some(file_blake3(&absolute)?),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(Error::io(&absolute, error)),
                };
                baseline_digests.insert(path.clone(), baseline_digest);
                path.clone()
            }
        };
        if !output_paths.insert(relative_path.clone()) {
            return Err(Error::message(format!(
                "managed artifact transaction resolves multiple outputs to `{relative_path}`"
            )));
        }
        staged_files.push(StagedFile {
            relative_path,
            content: write.content.clone(),
        });
    }
    validate_manifest(&updated_manifest)?;
    validate_binding_route_feature_catalog(paths, &updated_manifest.binding_routes)?;

    let mut backup_paths = staged_files
        .iter()
        .map(|file| paths.root().join(&file.relative_path))
        .collect::<BTreeSet<_>>();
    backup_paths.insert(manifest_path.clone());
    let backups = backup_paths
        .into_iter()
        .map(FileBackup::capture)
        .collect::<Result<Vec<_>>>()?;
    let manifest_backup = backups
        .iter()
        .find(|backup| backup.path == manifest_path)
        .expect("manifest path was included in transaction backups");
    if manifest_backup.content != manifest_baseline.content {
        return Err(Error::message(format!(
            "{} changed while preparing the managed artifact transaction",
            manifest_path.display()
        )));
    }
    for (relative_path, expected_digest) in &baseline_digests {
        let path = paths.root().join(relative_path);
        let backup = backups
            .iter()
            .find(|backup| backup.path == path)
            .expect("managed output path was included in transaction backups");
        let matches = match (backup.content.as_deref(), expected_digest.as_deref()) {
            (Some(content), Some(expected)) => blake3::hash(content).to_hex().as_str() == expected,
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
        };
        if !matches {
            return Err(Error::message(format!(
                "managed artifact path {} changed while preparing the transaction",
                path.display()
            )));
        }
    }
    let mut progress = TransitionProgress::default();

    let transition = (|| {
        for file in &staged_files {
            let path = paths.root().join(&file.relative_path);
            let backup = backups
                .iter()
                .find(|backup| backup.path == path)
                .expect("managed output path was included in transaction backups");
            let replacement = ManagedReplacement::capture(path.clone(), file.content.clone())?;
            progress.replaced_files.insert(path.clone(), replacement);
            let replacement = progress
                .replaced_files
                .get_mut(&path)
                .expect("captured managed output replacement");
            replacement.validate_original(backup)?;
            replacement.install()?;
        }

        let manifest_content = render_toml(&updated_manifest)?.into_bytes();
        let replacement = ManagedReplacement::capture(manifest_path.clone(), manifest_content)?;
        progress
            .replaced_files
            .insert(manifest_path.clone(), replacement);
        let replacement = progress
            .replaced_files
            .get_mut(&manifest_path)
            .expect("captured manifest replacement");
        replacement.validate_original(manifest_backup)?;
        replacement.install()?;

        validate_artifact_identities(paths, &updated_manifest)?;
        validate_candidate_identities(paths, &updated_manifest)?;
        if bootstrap {
            validate_repository(paths, &updated_manifest, false)?;
            super::api_coverage::check(paths)?;
        }
        terminal_validation()
    })();

    if let Err(error) = transition {
        let rollback_errors = rollback_file_changes(&backups, &mut progress);
        rollback_result(error, rollback_errors)
    } else {
        progress.finalize().map_err(|errors| {
            Error::message(format!(
                "managed artifacts installed successfully but quarantine cleanup failed:\n{}",
                errors.join("\n")
            ))
        })
    }
}

fn validate_repository_without_artifact_digests(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    validate_repository_core(paths, manifest)?;
    validate_artifact_revision_identities(paths, manifest)?;
    validate_recording_input_identities(paths, manifest)?;
    validate_recording_operations(paths, manifest)?;
    validate_bootstrap_bindings(paths, manifest)
}

fn validate_bootstrap_bindings(paths: &WorkspacePaths, manifest: &UpstreamManifest) -> Result<()> {
    let baseline = GenerationBaseline::capture(paths.root())?;
    let generation = IsolatedGeneration::create_at(
        paths,
        &baseline.repository_revision,
        &manifest.active_revision,
    )?;
    let validation = (|| {
        generation.generate_bindings(manifest)?;
        generation.generate_abi_metadata(manifest)?;
        let has_provider_identities = manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::ProviderIdentity);
        if has_provider_identities {
            let provider_sdk = super::provider::qualified_provider_sdk_for(&generation.worktree)?;
            super::provider::refresh_wasm_provider_contracts_unlocked(
                &generation.worktree,
                &generation.target_dir.join("wasm-provider-contracts"),
                &provider_sdk,
            )?;
        }
        compare_binding_artifacts(paths.root(), &generation.worktree, manifest)?;
        compare_abi_metadata_artifacts(paths.root(), &generation.worktree, manifest)?;
        if has_provider_identities {
            compare_provider_identity_artifacts(paths.root(), &generation.worktree, manifest)?;
        }
        Ok(())
    })();
    let cleanup = generation.finish();
    match (validation, cleanup) {
        (Ok(()), Ok(())) => baseline.verify(paths.root()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Err(error), Err(cleanup)) => Err(Error::message(format!(
            "bootstrap bindings validation failed: {error}\nisolated worktree cleanup also failed: {cleanup}"
        ))),
    }
}

fn compare_provider_identity_artifacts(
    installed_root: &Path,
    generated_root: &Path,
    manifest: &UpstreamManifest,
) -> Result<()> {
    for artifact in manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
    {
        let installed_path = installed_root.join(&artifact.path);
        let generated_path = generated_root.join(&artifact.path);
        let installed =
            fs::read(&installed_path).map_err(|source| Error::io(&installed_path, source))?;
        let generated =
            fs::read(&generated_path).map_err(|source| Error::io(&generated_path, source))?;
        if installed != generated {
            return Err(Error::message(format!(
                "provider identity artifact `{}` does not match isolated regeneration",
                artifact.name
            )));
        }
    }
    Ok(())
}

fn compare_abi_metadata_artifacts(
    installed_root: &Path,
    generated_root: &Path,
    manifest: &UpstreamManifest,
) -> Result<()> {
    for artifact in manifest.abi_metadata_artifacts() {
        let installed_path = installed_root.join(&artifact.path);
        let generated_path = generated_root.join(&artifact.path);
        let installed =
            fs::read(&installed_path).map_err(|source| Error::io(&installed_path, source))?;
        let generated =
            fs::read(&generated_path).map_err(|source| Error::io(&generated_path, source))?;
        if installed != generated {
            return Err(Error::message(format!(
                "bootstrap refuses to trust ABI metadata artifact `{}`: {} is not byte-for-byte reproducible from root revision {} and the native C probe",
                artifact.name,
                installed_path.display(),
                manifest.active_revision
            )));
        }
    }
    Ok(())
}

fn compare_binding_artifacts(
    installed_root: &Path,
    generated_root: &Path,
    manifest: &UpstreamManifest,
) -> Result<()> {
    for artifact in manifest.binding_artifacts() {
        let installed_path = installed_root.join(&artifact.path);
        let generated_path = generated_root.join(&artifact.path);
        let installed =
            fs::read(&installed_path).map_err(|source| Error::io(&installed_path, source))?;
        let generated =
            fs::read(&generated_path).map_err(|source| Error::io(&generated_path, source))?;
        if installed != generated {
            return Err(Error::message(format!(
                "bootstrap refuses to trust bindings artifact `{}`: {} is not byte-for-byte reproducible from root revision {} and manifest target coordinates",
                artifact.name,
                installed_path.display(),
                manifest.active_revision
            )));
        }
    }
    Ok(())
}

fn render_abi_probe_metadata(
    root: &Path,
    manifest: &UpstreamManifest,
    artifact: &GeneratedArtifact,
) -> Result<Vec<u8>> {
    let precision = artifact.precision.ok_or_else(|| {
        Error::message(format!(
            "ABI metadata artifact `{}` has no precision",
            artifact.name
        ))
    })?;
    if artifact.target != ArtifactTarget::Native
        || artifact.provider != ArtifactProvider::Source
        || artifact.producer != ArtifactProducer::AbiProbe
    {
        return Err(Error::message(format!(
            "ABI metadata artifact `{}` is not a native source ABI probe",
            artifact.name
        )));
    }
    let route = manifest
        .binding_routes
        .iter()
        .find(|route| route.mode == precision && route.provider == artifact.provider)
        .ok_or_else(|| {
            Error::message(format!(
                "ABI metadata artifact `{}` has no matching {}/{} binding route",
                artifact.name,
                precision.as_str(),
                artifact.provider.as_str()
            ))
        })?;
    let binding = manifest
        .artifacts
        .iter()
        .find(|candidate| candidate.name == route.artifact)
        .ok_or_else(|| {
            Error::message(format!(
                "ABI metadata artifact `{}` references missing bindings artifact `{}`",
                artifact.name, route.artifact
            ))
        })?;
    let generated = generate_workspace_probe(root, abi_probe_precision(precision))?;
    let inventory = render_toml(&manifest.source_inventory)?;
    let metadata = AbiProbeMetadata {
        schema_version: ABI_PROBE_METADATA_SCHEMA,
        upstream_sha: manifest.active_revision.clone(),
        repository: manifest.repository.clone(),
        source_tree: manifest.source_inventory.tree.clone(),
        source_inventory_blake3: blake3_bytes(inventory.as_bytes()),
        source_provenance: "official-gitlink".to_owned(),
        provider_source_sha: manifest.active_revision.clone(),
        precision,
        target: artifact.target,
        provider: artifact.provider,
        producer: artifact.producer,
        bindings_generation_target: route.rust_target,
        binding_artifact: binding.name.clone(),
        binding_blake3: file_blake3(&root.join(&binding.path))?,
        probe_content_blake3: abi_probe_content_blake3(&generated),
        c_probe_blake3: blake3_bytes(generated.c_source.as_bytes()),
        mixed_precision_c_probe_blake3: blake3_bytes(generated.mixed_precision_c_source.as_bytes()),
        rust_cases_blake3: blake3_bytes(generated.rust_source.as_bytes()),
        structure_count: generated.struct_count,
        field_count: generated.field_count,
        layout_case_count: generated.layout_case_count,
        symbol_count: generated.symbol_count,
        callback_count: generated.callback_count,
        callable_callback_count: generated.callable_callback_count,
    };
    Ok(render_toml(&metadata)?.into_bytes())
}

const fn abi_probe_precision(precision: Precision) -> AbiProbePrecision {
    match precision {
        Precision::Single => AbiProbePrecision::Single,
        Precision::Double => AbiProbePrecision::Double,
    }
}

fn blake3_bytes(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

fn abi_probe_content_blake3(generated: &GeneratedAbiProbe) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"boxdd-abi-probe-content-v1\0");
    for (name, content) in [
        ("c-probe", generated.c_source.as_bytes()),
        (
            "mixed-precision-c-probe",
            generated.mixed_precision_c_source.as_bytes(),
        ),
        ("rust-cases", generated.rust_source.as_bytes()),
    ] {
        hasher.update(name.as_bytes());
        hasher.update(&[0]);
        hasher.update(&(content.len() as u64).to_le_bytes());
        hasher.update(content);
    }
    hasher.finalize().to_hex().to_string()
}

fn run_abi_probe_test(
    cargo: &QualifiedCargo,
    target_dir: &Path,
    precision: Precision,
) -> Result<()> {
    let mut command = cargo.command_at_working_root(&target_dir.join(precision.as_str()))?;
    command.args([
        "test",
        "--locked",
        "-p",
        "boxdd-abi-probe",
        "--test",
        "abi",
        "--no-default-features",
    ]);
    if precision == Precision::Double {
        command.args(["--features", "double-precision"]);
    }
    command_success_with_output(
        &mut command,
        &format!("run {}-precision C ABI probe", precision.as_str()),
    )
}

fn qualify_generation_cargo(
    anchor_root: &Path,
    working_root: &Path,
    cargo_home: &Path,
    output_root: &Path,
) -> Result<QualifiedCargo> {
    QualifiedCargo::qualify_isolated_scoped(anchor_root, working_root, cargo_home, output_root)
}

fn command_success_with_output(command: &mut Command, label: &str) -> Result<()> {
    let output = command
        .output()
        .map_err(|source| Error::io(label, source))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with {}:\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn validate_abi_probe_artifacts(paths: &WorkspacePaths, manifest: &UpstreamManifest) -> Result<()> {
    let artifacts = manifest.abi_metadata_artifacts().collect::<Vec<_>>();
    if artifacts.is_empty() {
        #[cfg(test)]
        return Ok(());
        #[cfg(not(test))]
        return Err(Error::message(
            "upstream manifest has no C ABI metadata artifacts; single- and double-precision source routes must be qualified",
        ));
    }

    let mut topology_errors = Vec::new();
    validate_abi_metadata_topology(
        &manifest.binding_routes,
        &manifest.artifacts,
        true,
        &mut topology_errors,
    );
    if !topology_errors.is_empty() {
        return Err(Error::message(topology_errors.join("\n")));
    }

    let mut precisions = BTreeSet::new();
    for artifact in artifacts {
        let path = paths.root().join(&artifact.path);
        let observed = fs::read(&path).map_err(|source| Error::io(&path, source))?;
        let expected = render_abi_probe_metadata(paths.root(), manifest, artifact)?;
        if observed != expected {
            return Err(Error::message(format!(
                "ABI metadata artifact `{}` is not reproducible from revision {}: expected blake3 {}, observed blake3 {}; regenerate it through upstream-sync",
                artifact.name,
                manifest.active_revision,
                blake3_bytes(&expected),
                blake3_bytes(&observed)
            )));
        }
        let source = std::str::from_utf8(&observed).map_err(|error| {
            Error::message(format!(
                "ABI metadata artifact `{}` is not UTF-8: {error}",
                artifact.name
            ))
        })?;
        let metadata: AbiProbeMetadata = toml::from_str(source).map_err(|error| {
            Error::message(format!(
                "ABI metadata artifact `{}` is invalid TOML: {error}",
                artifact.name
            ))
        })?;
        if metadata.schema_version != ABI_PROBE_METADATA_SCHEMA {
            return Err(Error::message(format!(
                "ABI metadata artifact `{}` schema {} does not match supported schema {ABI_PROBE_METADATA_SCHEMA}",
                artifact.name, metadata.schema_version
            )));
        }
        precisions.insert(metadata.precision);
    }

    let output_root = controlled_child_directory(
        paths.root(),
        Path::new("target/upstream-abi-verification"),
        "upstream ABI verification root",
    )?;
    let cargo_home = controlled_child_directory(
        &output_root,
        Path::new("cargo-home"),
        "upstream ABI verification Cargo home",
    )?;
    let target_dir = output_root.join("cargo-target");
    let cargo = qualify_generation_cargo(paths.root(), paths.root(), &cargo_home, &output_root)?;
    for precision in precisions {
        run_abi_probe_test(&cargo, &target_dir, precision)?;
    }
    Ok(())
}

fn install_staged_update(
    paths: &WorkspacePaths,
    original: &UpstreamManifest,
    staged: &StagedUpdate,
    baseline: &ManagedSnapshot,
    fail_after_operations: Option<usize>,
) -> Result<()> {
    install_staged_update_with(
        paths,
        original,
        staged,
        Some(baseline),
        fail_after_operations,
        || {
            validate_repository(paths, &staged.manifest, false)?;
            super::provider::validate_checked_wasm_provider_contracts(paths.root())?;
            super::api_coverage::check(paths)
        },
    )
}

fn install_staged_update_with<F>(
    paths: &WorkspacePaths,
    original: &UpstreamManifest,
    staged: &StagedUpdate,
    baseline: Option<&ManagedSnapshot>,
    fail_after_operations: Option<usize>,
    terminal_validation: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    install_staged_update_with_finalize(
        paths,
        original,
        staged,
        baseline,
        fail_after_operations,
        terminal_validation,
        TransitionProgress::finalize,
    )
}

#[allow(clippy::too_many_arguments)]
fn install_staged_update_with_finalize<F, C>(
    paths: &WorkspacePaths,
    original: &UpstreamManifest,
    staged: &StagedUpdate,
    baseline: Option<&ManagedSnapshot>,
    fail_after_operations: Option<usize>,
    terminal_validation: F,
    finalize: C,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
    C: FnOnce(&mut TransitionProgress) -> std::result::Result<(), Vec<String>>,
{
    if let Some(baseline) = baseline {
        baseline.verify_all(paths)?;
    }
    let mut backup_paths = BTreeSet::new();
    backup_paths.insert(paths.upstream_manifest());
    for artifact in &staged.artifacts {
        backup_paths.insert(paths.root().join(&artifact.relative_path));
    }
    for candidate in &staged.candidate_paths {
        backup_paths.insert(paths.root().join(candidate));
    }
    let backups = backup_paths
        .into_iter()
        .map(FileBackup::capture)
        .collect::<Result<Vec<_>>>()?;
    let git_backup = GitBackup {
        index: GitIndexSnapshot::capture(paths.root())?,
        checkout: checkout_state(&paths.box2d())?,
    };
    let mut progress = TransitionProgress::default();

    let transition = (|| {
        let mut completed = 0usize;
        for artifact in &staged.artifacts {
            maybe_inject_transition_failure(fail_after_operations, completed)?;
            let path = paths.root().join(&artifact.relative_path);
            if let Some(baseline) = baseline {
                baseline.verify_path(&path)?;
            }
            let backup = backups
                .iter()
                .find(|backup| backup.path == path)
                .expect("artifact path was included in transaction backups");
            let replacement = ManagedReplacement::capture(path.clone(), artifact.content.clone())?;
            progress.replaced_files.insert(path.clone(), replacement);
            let replacement = progress
                .replaced_files
                .get_mut(&path)
                .expect("captured replacement");
            replacement.validate_original(backup)?;
            replacement.install()?;
            completed += 1;
        }
        for candidate in &staged.candidate_paths {
            maybe_inject_transition_failure(fail_after_operations, completed)?;
            let path = paths.root().join(candidate);
            if let Some(baseline) = baseline {
                baseline.verify_path(&path)?;
            }
            let expected = backups
                .iter()
                .find(|backup| backup.path == path)
                .and_then(|backup| backup.content.as_deref());
            if let Some(expected) = expected {
                let removed = RemovedFile::capture(path.clone())?;
                let content_matches = removed.captured_content.as_deref() == Some(expected);
                let capture_error = removed.capture_error.clone();
                progress.removed_files.insert(path.clone(), removed);
                if let Some(capture_error) = capture_error {
                    return Err(Error::message(format!(
                        "could not validate atomically quarantined candidate {}: {capture_error}",
                        path.display()
                    )));
                }
                if !content_matches {
                    return Err(Error::message(format!(
                        "managed candidate {} changed while it was atomically quarantined; refusing to delete it",
                        path.display()
                    )));
                }
            } else if fs::symlink_metadata(&path).is_ok() {
                return Err(Error::message(format!(
                    "managed candidate {} appeared while the transaction was running; refusing to delete it",
                    path.display()
                )));
            }
            completed += 1;
        }
        maybe_inject_transition_failure(fail_after_operations, completed)?;
        if let Some(baseline) = baseline {
            baseline.verify_checkout(paths)?;
        }
        progress.checkout_poststate = Some(CheckoutState::detached(
            staged.manifest.active_revision.clone(),
        ));
        checkout_detached(&paths.box2d(), &staged.manifest.active_revision)?;
        completed += 1;
        maybe_inject_transition_failure(fail_after_operations, completed)?;
        if let Some(baseline) = baseline {
            baseline.verify_gitlink(paths)?;
        }
        let replacement = GitIndexReplacement::prepare(
            paths.root(),
            git_backup.index.clone(),
            &staged.manifest.active_revision,
        )?;
        progress.git_index = Some(replacement);
        progress
            .git_index
            .as_mut()
            .expect("prepared Git index replacement")
            .install()?;
        completed += 1;
        maybe_inject_transition_failure(fail_after_operations, completed)?;
        if let Some(baseline) = baseline {
            baseline.verify_path(&paths.upstream_manifest())?;
        }
        let manifest_path = paths.upstream_manifest();
        let manifest_content = render_toml(&staged.manifest)?.into_bytes();
        let backup = backups
            .iter()
            .find(|backup| backup.path == manifest_path)
            .expect("manifest path was included in transaction backups");
        let replacement = ManagedReplacement::capture(manifest_path.clone(), manifest_content)?;
        progress
            .replaced_files
            .insert(manifest_path.clone(), replacement);
        let replacement = progress
            .replaced_files
            .get_mut(&manifest_path)
            .expect("captured manifest replacement");
        replacement.validate_original(backup)?;
        replacement.install()?;
        completed += 1;
        maybe_inject_transition_failure(fail_after_operations, completed)?;
        terminal_validation()
    })();

    if let Err(error) = transition {
        rollback_update(paths, original, &backups, &git_backup, &mut progress, error)
    } else {
        finalize(&mut progress).map_err(|errors| {
            Error::message(format!(
                "upstream update installed successfully but quarantine cleanup failed:\n{}",
                errors.join("\n")
            ))
        })
    }
}

fn validate_route_refresh_staging(
    original: &UpstreamManifest,
    staged: &RouteRefreshStaging,
) -> Result<()> {
    validate_manifest(&staged.manifest)?;
    validate_route_refresh_topology(&staged.manifest)?;
    if !staged.manifest.artifact_digests_initialized {
        return Err(Error::message(
            "route refresh staging must initialize every artifact digest",
        ));
    }
    if staged.manifest.active_revision != original.active_revision
        || staged.manifest.recording_revision != original.recording_revision
        || staged.manifest.repository != original.repository
        || staged.manifest.source_inventory != original.source_inventory
        || staged.manifest.recording_inputs != original.recording_inputs
    {
        return Err(Error::message(
            "route refresh staging changed revision-coupled upstream identity",
        ));
    }

    if !staged
        .files
        .windows(2)
        .all(|pair| pair[0].relative_path < pair[1].relative_path)
    {
        return Err(Error::message(
            "route refresh staged files must be sorted and unique",
        ));
    }
    let expected_files = staged
        .manifest
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .chain(std::iter::once(
            super::api_coverage::RUNTIME_RECORDING_WIRE_PATH.to_owned(),
        ))
        .collect::<BTreeSet<_>>();
    if expected_files.contains("boxdd-sys/upstream.toml") {
        return Err(Error::message(
            "route refresh outputs cannot alias the upstream manifest",
        ));
    }
    if expected_files.len() != staged.manifest.artifacts.len() + 1 {
        return Err(Error::message(
            "route refresh artifact paths collide with the runtime recording contract",
        ));
    }
    let observed_files = staged
        .files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<BTreeSet<_>>();
    if observed_files != expected_files || observed_files.len() != staged.files.len() {
        return Err(Error::message(format!(
            "route refresh staged output set is incomplete: observed {observed_files:?}, expected {expected_files:?}"
        )));
    }
    for artifact in &staged.manifest.artifacts {
        let file = staged
            .files
            .iter()
            .find(|file| file.relative_path == artifact.path)
            .expect("validated route refresh output set contains every artifact");
        let digest = blake3_bytes(&file.content);
        if digest != artifact.content_blake3 {
            return Err(Error::message(format!(
                "route refresh artifact `{}` staged digest {digest} does not match manifest digest {}",
                artifact.name, artifact.content_blake3
            )));
        }
    }

    let target_paths = staged
        .manifest
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    let expected_removals = original
        .artifacts
        .iter()
        .filter(|artifact| !target_paths.contains(artifact.path.as_str()))
        .map(|artifact| artifact.path.clone())
        .collect::<BTreeSet<_>>();
    let observed_removals = staged.removals.iter().cloned().collect::<BTreeSet<_>>();
    if observed_removals != expected_removals
        || observed_removals.len() != staged.removals.len()
        || observed_removals
            .iter()
            .any(|path| observed_files.contains(path))
    {
        return Err(Error::message(format!(
            "route refresh stale artifact removal set is incorrect: observed {observed_removals:?}, expected {expected_removals:?}"
        )));
    }
    Ok(())
}

fn install_route_refresh<F>(
    paths: &WorkspacePaths,
    original: &UpstreamManifest,
    staged: &RouteRefreshStaging,
    baseline: &RouteRefreshBaseline,
    fail_after_operations: Option<usize>,
    terminal_validation: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    validate_route_refresh_staging(original, staged)?;
    baseline.verify_before_install(paths)?;
    let managed_paths = staged
        .files
        .iter()
        .map(|file| file.relative_path.clone())
        .chain(staged.removals.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut backups = baseline.outputs.clone();
    backups.push(baseline.manifest.clone());
    let mut progress = TransitionProgress::default();

    let transition = (|| {
        let mut completed = 0usize;
        for file in &staged.files {
            maybe_inject_transition_failure(fail_after_operations, completed)?;
            let path = paths.root().join(&file.relative_path);
            let backup = baseline.output(&path)?;
            let replacement = ManagedReplacement::capture(path.clone(), file.content.clone())?;
            progress.replaced_files.insert(path.clone(), replacement);
            let replacement = progress
                .replaced_files
                .get_mut(&path)
                .expect("captured route refresh replacement");
            replacement.validate_original(backup)?;
            replacement.install()?;
            completed += 1;
        }
        for relative in &staged.removals {
            maybe_inject_transition_failure(fail_after_operations, completed)?;
            let path = paths.root().join(relative);
            let backup = baseline.output(&path)?;
            match backup.content.as_deref() {
                Some(expected) => {
                    let removed = RemovedFile::capture(path.clone())?;
                    let content_matches = removed.captured_content.as_deref() == Some(expected);
                    let capture_error = removed.capture_error.clone();
                    progress.removed_files.insert(path.clone(), removed);
                    if let Some(capture_error) = capture_error {
                        return Err(Error::message(format!(
                            "could not quarantine stale route artifact {}: {capture_error}",
                            path.display()
                        )));
                    }
                    if !content_matches {
                        return Err(Error::message(format!(
                            "stale route artifact {} changed while it was quarantined",
                            path.display()
                        )));
                    }
                }
                None if fs::symlink_metadata(&path).is_ok() => {
                    return Err(Error::message(format!(
                        "stale route artifact {} appeared during installation",
                        path.display()
                    )));
                }
                None => {}
            }
            completed += 1;
        }

        maybe_inject_transition_failure(fail_after_operations, completed)?;
        let snapshot_exclusions =
            route_refresh_snapshot_exclusions(paths.root(), &managed_paths, &progress)?;
        baseline.verify_stable_inputs(paths, &snapshot_exclusions)?;
        baseline.verify_manifest_original()?;
        let manifest_content = render_toml(&staged.manifest)?.into_bytes();
        let manifest_path = paths.upstream_manifest();
        let replacement =
            ManagedReplacement::capture(manifest_path.clone(), manifest_content.clone())?;
        progress
            .replaced_files
            .insert(manifest_path.clone(), replacement);
        let replacement = progress
            .replaced_files
            .get_mut(&manifest_path)
            .expect("captured route refresh manifest replacement");
        replacement.validate_original(&baseline.manifest)?;
        replacement.install()?;
        completed += 1;
        maybe_inject_transition_failure(fail_after_operations, completed)?;

        terminal_validation()?;
        let snapshot_exclusions =
            route_refresh_snapshot_exclusions(paths.root(), &managed_paths, &progress)?;
        baseline.verify_stable_inputs(paths, &snapshot_exclusions)?;
        validate_installed_route_refresh(paths, staged, &manifest_content)
    })();

    if let Err(error) = transition {
        let rollback_errors = rollback_file_changes(&backups, &mut progress);
        rollback_result(error, rollback_errors)
    } else {
        progress.finalize().map_err(|errors| {
            Error::message(format!(
                "route refresh installed successfully but quarantine cleanup failed:\n{}",
                errors.join("\n")
            ))
        })
    }
}

fn route_refresh_snapshot_exclusions(
    root: &Path,
    managed_paths: &BTreeSet<String>,
    progress: &TransitionProgress,
) -> Result<BTreeSet<String>> {
    let mut excluded = managed_paths.clone();
    let directories = progress
        .replaced_files
        .values()
        .filter_map(|replacement| replacement.original.as_ref())
        .chain(progress.removed_files.values())
        .filter_map(|removed| removed.directory.as_ref())
        .map(TempDir::path);
    for directory in directories {
        let relative = directory.strip_prefix(root).map_err(|_| {
            Error::message(format!(
                "route refresh quarantine {} is outside repository root {}",
                directory.display(),
                root.display()
            ))
        })?;
        let relative = canonical_manifest_path(relative).ok_or_else(|| {
            Error::message(format!(
                "route refresh quarantine {} is not a canonical UTF-8 path",
                directory.display()
            ))
        })?;
        excluded.insert(relative);
    }
    Ok(excluded)
}

fn validate_installed_route_refresh(
    paths: &WorkspacePaths,
    staged: &RouteRefreshStaging,
    manifest_content: &[u8],
) -> Result<()> {
    for file in &staged.files {
        let path = paths.root().join(&file.relative_path);
        let observed = fs::read(&path).map_err(|source| Error::io(&path, source))?;
        if observed != file.content {
            return Err(Error::message(format!(
                "installed route refresh output {} changed during terminal validation",
                path.display()
            )));
        }
    }
    for relative in &staged.removals {
        let path = paths.root().join(relative);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => {
                return Err(Error::message(format!(
                    "stale route artifact {} reappeared during terminal validation",
                    path.display()
                )));
            }
            Err(error) => return Err(Error::io(&path, error)),
        }
    }
    let manifest_path = paths.upstream_manifest();
    let observed = fs::read(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    if observed != manifest_content {
        return Err(Error::message(
            "upstream manifest changed during route refresh terminal validation",
        ));
    }
    Ok(())
}

#[derive(Default)]
struct TransitionProgress {
    replaced_files: BTreeMap<PathBuf, ManagedReplacement>,
    removed_files: BTreeMap<PathBuf, RemovedFile>,
    checkout_poststate: Option<CheckoutState>,
    git_index: Option<GitIndexReplacement>,
}

impl TransitionProgress {
    fn finalize(&mut self) -> std::result::Result<(), Vec<String>> {
        self.finalize_with(TempDir::close)
    }

    fn finalize_with<F>(&mut self, mut close: F) -> std::result::Result<(), Vec<String>>
    where
        F: FnMut(TempDir) -> std::io::Result<()>,
    {
        let mut errors = Vec::new();
        for replacement in self.replaced_files.values_mut() {
            if let Err(error) = replacement.finalize_with(&mut close) {
                errors.push(error);
            }
        }
        for removed in self.removed_files.values_mut() {
            if let Err(error) = removed.finalize_with(&mut close) {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

struct GitBackup {
    index: GitIndexSnapshot,
    checkout: CheckoutState,
}

fn maybe_inject_transition_failure(fail_after: Option<usize>, completed: usize) -> Result<()> {
    if fail_after == Some(completed) {
        Err(Error::message(format!(
            "injected transition failure after {completed} operations"
        )))
    } else {
        Ok(())
    }
}

fn rollback_update(
    paths: &WorkspacePaths,
    _manifest: &UpstreamManifest,
    backups: &[FileBackup],
    git_backup: &GitBackup,
    progress: &mut TransitionProgress,
    original: Error,
) -> Result<()> {
    let mut rollback_errors = rollback_file_changes(backups, progress);
    if let Some(replacement) = progress.git_index.as_mut()
        && let Err(error) = replacement.rollback()
    {
        rollback_errors.push(format!("gitlink: {error}"));
    }
    if let Some(poststate) = &progress.checkout_poststate {
        match checkout_state(&paths.box2d()) {
            Ok(actual) if actual == *poststate => {
                if let Err(error) = restore_checkout_state(&paths.box2d(), &git_backup.checkout) {
                    rollback_errors.push(format!("submodule: {error}"));
                }
            }
            Ok(actual) if actual == git_backup.checkout => {}
            Ok(actual) => rollback_errors.push(format!(
                "rollback conflict at submodule checkout: observed {actual:?}, expected transaction state {poststate:?}; preserving the concurrent state"
            )),
            Err(error) => rollback_errors.push(format!(
                "rollback conflict at submodule checkout: {error}; preserving the concurrent state"
            )),
        }
    }
    rollback_result(original, rollback_errors)
}

fn rollback_file_changes(backups: &[FileBackup], progress: &mut TransitionProgress) -> Vec<String> {
    let mut rollback_errors = Vec::new();
    for backup in backups.iter().rev() {
        if let Some(replacement) = progress.replaced_files.get_mut(&backup.path)
            && let Some(error) = replacement.rollback(backup)
        {
            rollback_errors.push(error);
        }
    }
    for backup in backups.iter().rev() {
        let Some(removed) = progress.removed_files.get_mut(&backup.path) else {
            continue;
        };
        if let Some(error) = removed.rollback(backup) {
            rollback_errors.push(error);
        }
    }
    rollback_errors
}

fn rollback_result(original: Error, rollback_errors: Vec<String>) -> Result<()> {
    if rollback_errors.is_empty() {
        Err(original)
    } else {
        Err(Error::message(format!(
            "upstream update failed: {original}\nrollback also failed:\n{}",
            rollback_errors.join("\n")
        )))
    }
}

#[derive(Clone)]
struct FileBackup {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

struct ManagedReplacement {
    path: PathBuf,
    original: Option<RemovedFile>,
    installed_content: Vec<u8>,
    installed: bool,
}

impl ManagedReplacement {
    fn capture(path: PathBuf, installed_content: Vec<u8>) -> Result<Self> {
        let original = match fs::symlink_metadata(&path) {
            Ok(_) => Some(RemovedFile::capture(path.clone())?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(Error::io(&path, error)),
        };
        Ok(Self {
            path,
            original,
            installed_content,
            installed: false,
        })
    }

    fn validate_original(&self, backup: &FileBackup) -> Result<()> {
        if let Some(error) = self
            .original
            .as_ref()
            .and_then(|original| original.capture_error.as_deref())
        {
            return Err(Error::message(format!(
                "could not validate quarantined managed file {}: {error}",
                self.path.display()
            )));
        }
        let observed = self
            .original
            .as_ref()
            .and_then(|original| original.captured_content.as_ref());
        if observed != backup.content.as_ref() {
            return Err(Error::message(format!(
                "managed path {} changed while it was atomically quarantined; refusing to overwrite it",
                self.path.display()
            )));
        }
        Ok(())
    }

    fn install(&mut self) -> Result<()> {
        let permissions = self
            .original
            .as_ref()
            .map(|original| {
                fs::metadata(&original.quarantine_path)
                    .map(|metadata| metadata.permissions())
                    .map_err(|source| Error::io(&original.quarantine_path, source))
            })
            .transpose()?;
        write_new_bytes_noclobber(&self.path, &self.installed_content, permissions)?;
        self.installed = true;
        Ok(())
    }

    fn rollback(&mut self, backup: &FileBackup) -> Option<String> {
        if !self.installed {
            return match &mut self.original {
                Some(original) => match FileBackup::capture(self.path.clone()) {
                    Ok(actual) if actual.content.is_none() => original.restore_to_empty().err(),
                    Ok(actual)
                        if original.captured_content == backup.content
                            && actual.content == backup.content =>
                    {
                        original.finalize().err()
                    }
                    Ok(_) => Some(original.keep_conflict(
                        "installation did not complete because a concurrent state appeared; preserving both states"
                            .to_owned(),
                    )),
                    Err(error) => Some(original.keep_conflict(format!(
                        "could not inspect the path after installation failed: {error}"
                    ))),
                },
                None => match FileBackup::capture(self.path.clone()) {
                    Ok(actual) if actual.content.is_none() => None,
                    Ok(_) => Some(format!(
                        "rollback conflict at {}: a concurrent state appeared before installation",
                        self.path.display()
                    )),
                    Err(error) => Some(format!(
                        "rollback conflict at {}: {error}; preserving the concurrent state",
                        self.path.display()
                    )),
                },
            };
        }

        let actual = match FileBackup::capture(self.path.clone()) {
            Ok(actual) => actual,
            Err(error) => {
                return Some(self.keep_original_conflict(format!(
                    "could not inspect installed state during rollback: {error}"
                )));
            }
        };
        if actual.content == backup.content {
            return self.finalize_original().err();
        }
        if actual.content.as_ref() != Some(&self.installed_content) {
            return Some(self.keep_original_conflict(
                "content changed after installation; preserving the concurrent state".to_owned(),
            ));
        }

        let mut installed = match RemovedFile::capture(self.path.clone()) {
            Ok(installed) => installed,
            Err(error) => {
                return Some(self.keep_original_conflict(format!(
                    "could not quarantine the installed state during rollback: {error}"
                )));
            }
        };
        if installed.capture_error.is_some()
            || installed.captured_content.as_ref() != Some(&self.installed_content)
        {
            let restore_error = installed.restore_to_empty().err();
            let detail = match restore_error {
                Some(error) => format!(
                    "installed path changed during rollback and could not be restored: {error}"
                ),
                None => {
                    "installed path changed during rollback; concurrent state restored".to_owned()
                }
            };
            return Some(self.keep_original_conflict(detail));
        }

        if let Some(original) = &mut self.original
            && let Err(error) = original.restore_to_empty()
        {
            let installed_location = installed.keep_conflict(
                "transaction-installed content retained after original restore failed".to_owned(),
            );
            return Some(self.keep_original_conflict(format!(
                "could not restore original content: {error}; {installed_location}"
            )));
        }
        installed.finalize().err()
    }

    fn finalize_with<F>(&mut self, close: &mut F) -> std::result::Result<(), String>
    where
        F: FnMut(TempDir) -> std::io::Result<()>,
    {
        match &mut self.original {
            Some(original) => original.finalize_with(close),
            None => Ok(()),
        }
    }

    fn finalize_original(&mut self) -> std::result::Result<(), String> {
        match &mut self.original {
            Some(original) => original.finalize(),
            None => Ok(()),
        }
    }

    fn keep_original_conflict(&mut self, detail: String) -> String {
        match &mut self.original {
            Some(original) => original.keep_conflict(detail),
            None => format!("rollback conflict at {}: {detail}", self.path.display()),
        }
    }
}

struct RemovedFile {
    original_path: PathBuf,
    quarantine_path: PathBuf,
    directory: Option<TempDir>,
    captured_content: Option<Vec<u8>>,
    capture_error: Option<String>,
}

impl RemovedFile {
    fn capture(original_path: PathBuf) -> Result<Self> {
        let parent = original_path.parent().ok_or_else(|| {
            Error::message(format!(
                "{} has no parent directory",
                original_path.display()
            ))
        })?;
        let directory = tempfile::tempdir_in(parent).map_err(|source| Error::io(parent, source))?;
        let quarantine_path = directory.path().join("removed");
        fs::rename(&original_path, &quarantine_path)
            .map_err(|source| Error::io(&original_path, source))?;
        let (captured_content, capture_error) = match fs::read(&quarantine_path) {
            Ok(content) => (Some(content), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Ok(Self {
            original_path,
            quarantine_path,
            directory: Some(directory),
            captured_content,
            capture_error,
        })
    }

    fn rollback(&mut self, backup: &FileBackup) -> Option<String> {
        let captured_matches_backup = self.captured_content == backup.content;
        let actual = FileBackup::capture(self.original_path.clone());
        match actual {
            Ok(actual) if actual.content.is_none() => {
                match fs::hard_link(&self.quarantine_path, &self.original_path) {
                    Ok(()) => {
                        self.finalize().err()
                    }
                    Err(error) => Some(self.keep_conflict(format!(
                        "could not restore the quarantined candidate without overwriting a concurrent path: {error}"
                    ))),
                }
            }
            Ok(actual) if captured_matches_backup && actual.content == backup.content => {
                self.finalize().err()
            }
            Ok(_) if captured_matches_backup => {
                let cleanup = self.finalize().err();
                Some(format!(
                    "rollback conflict at {}: the candidate was recreated after quarantine; preserving the concurrent state",
                    self.original_path.display()
                ) + &cleanup.map(|error| format!("; {error}")).unwrap_or_default())
            }
            Ok(_) => Some(self.keep_conflict(
                "both the quarantined candidate and its original path contain concurrent states"
                    .to_owned(),
            )),
            Err(error) if captured_matches_backup => {
                let cleanup = self.finalize().err();
                Some(format!(
                    "rollback conflict at {}: {error}; preserving the concurrent state",
                    self.original_path.display()
                ) + &cleanup.map(|error| format!("; {error}")).unwrap_or_default())
            }
            Err(error) => Some(self.keep_conflict(format!(
                "could not inspect the candidate path during rollback: {error}"
            ))),
        }
    }

    fn restore_to_empty(&mut self) -> std::result::Result<(), String> {
        fs::hard_link(&self.quarantine_path, &self.original_path).map_err(|error| {
            format!(
                "could not restore {} without overwriting a concurrent path: {error}",
                self.original_path.display()
            )
        })?;
        self.finalize()
    }

    fn finalize(&mut self) -> std::result::Result<(), String> {
        self.finalize_with(TempDir::close)
    }

    fn finalize_with<F>(&mut self, close: F) -> std::result::Result<(), String>
    where
        F: FnOnce(TempDir) -> std::io::Result<()>,
    {
        let Some(directory) = self.directory.take() else {
            return Ok(());
        };
        let path = directory.path().to_owned();
        close(directory).map_err(|error| {
            format!(
                "could not remove quarantine directory {}: {error}",
                path.display()
            )
        })
    }

    fn keep_conflict(&mut self, detail: String) -> String {
        let kept = self.directory.take().map(TempDir::keep).unwrap_or_else(|| {
            self.quarantine_path
                .parent()
                .expect("quarantine path has a parent")
                .to_owned()
        });
        format!(
            "rollback conflict at {}: {detail}; quarantined content preserved at {}",
            self.original_path.display(),
            kept.display()
        )
    }
}

impl FileBackup {
    fn capture(path: PathBuf) -> Result<Self> {
        let content = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                Some(fs::read(&path).map_err(|source| Error::io(&path, source))?)
            }
            Ok(_) => {
                return Err(Error::message(format!(
                    "managed path {} is not a regular file",
                    path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(Error::io(&path, error)),
        };
        Ok(Self { path, content })
    }
}

struct IsolatedGeneration {
    repository_root: PathBuf,
    source_directory: Option<TempDir>,
    source_root: PathBuf,
    worktree: PathBuf,
    target_dir: PathBuf,
    cargo_home: PathBuf,
    cargo: Option<QualifiedCargo>,
    repository_worktree_added: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct IsolatedGenerationMarker {
    schema: u32,
    common_directory: PathBuf,
    source_root: PathBuf,
    worktree: PathBuf,
}

impl IsolatedGeneration {
    #[cfg(test)]
    fn create(paths: &WorkspacePaths, revision: &str) -> Result<Self> {
        let repository_revision = git_output(paths.root(), ["rev-parse", "HEAD"])?;
        Self::create_at(paths, repository_revision.trim(), revision)
    }

    fn create_at(
        paths: &WorkspacePaths,
        repository_revision: &str,
        revision: &str,
    ) -> Result<Self> {
        Self::create_at_with_cleanup(paths, repository_revision, revision, Self::cleanup)
    }

    fn create_at_with_cleanup<F>(
        paths: &WorkspacePaths,
        repository_revision: &str,
        revision: &str,
        cleanup: F,
    ) -> Result<Self>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        let isolation_parent = isolated_generation_parent(paths.root())?;
        let source_directory = tempfile::Builder::new()
            .prefix(&format!(
                "{ISOLATED_GENERATION_DIRECTORY_PREFIX}{}-",
                std::process::id()
            ))
            .tempdir_in(&isolation_parent)
            .map_err(|source| Error::io(&isolation_parent, source))?;
        let source_root = fs::canonicalize(source_directory.path())
            .map_err(|source| Error::io(source_directory.path(), source))?;
        let worktree = source_root.join("workspace");
        let target_dir = source_root.join("cargo-target");
        let cargo_home = source_root.join("cargo-home");
        write_isolated_generation_marker(&isolation_parent, &source_root, &worktree)?;
        let mut generation = Self {
            repository_root: paths.root().to_owned(),
            source_directory: Some(source_directory),
            source_root,
            worktree,
            target_dir,
            cargo_home,
            cargo: None,
            repository_worktree_added: false,
        };
        let initialization = (|| {
            generation.cargo_home = controlled_child_directory(
                &generation.source_root,
                Path::new("cargo-home"),
                "isolated upstream Cargo home",
            )?;
            command_success(
                git_command()?
                    .current_dir(&generation.repository_root)
                    .args(["worktree", "add", "--detach"])
                    .arg(&generation.worktree)
                    .arg(repository_revision),
                "create isolated repository worktree",
            )?;
            generation.repository_worktree_added = true;

            let isolated_submodule = generation.worktree.join(BOX2D_GITLINK);
            if isolated_submodule.exists() {
                fs::remove_dir_all(&isolated_submodule)
                    .map_err(|source| Error::io(&isolated_submodule, source))?;
            }
            let parent = isolated_submodule
                .parent()
                .ok_or_else(|| Error::message("isolated submodule path has no parent"))?;
            fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
            command_success(
                git_command()?
                    .args(["clone", "--no-hardlinks", "--no-checkout"])
                    .arg(paths.box2d())
                    .arg(&isolated_submodule),
                "clone local Box2D object store into isolated worktree",
            )?;
            checkout_detached(&isolated_submodule, revision)?;
            generation.cargo = Some(qualify_generation_cargo(
                &generation.worktree,
                &generation.worktree,
                &generation.cargo_home,
                &generation.source_root,
            )?);
            Ok(())
        })();
        if let Err(error) = initialization {
            let cleanup_result = cleanup(&mut generation);
            return Err(merge_isolated_initialization_failure(error, cleanup_result));
        }
        Ok(generation)
    }

    fn qualified_cargo(&self) -> Result<&QualifiedCargo> {
        self.cargo
            .as_ref()
            .ok_or_else(|| Error::message("isolated upstream Cargo was not qualified"))
    }

    fn prepare_update(&self, manifest: &UpstreamManifest, target: &str) -> Result<StagedUpdate> {
        let mut target_manifest = manifest.promoted_for_generation(target)?;

        write_atomic(
            &self.worktree.join("boxdd-sys/upstream.toml"),
            &render_toml(&target_manifest)?,
        )?;
        set_indexed_gitlink(&self.worktree, target)?;
        self.generate_bindings(&target_manifest)?;
        self.generate_abi_metadata(&target_manifest)?;
        if target_manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
        {
            let provider_sdk = super::provider::qualified_provider_sdk_for(&self.worktree)?;
            super::provider::refresh_wasm_provider_contracts_unlocked(
                &self.worktree,
                &self.target_dir.join("wasm-provider-contracts"),
                &provider_sdk,
            )?;
            super::provider::validate_checked_wasm_provider_contracts(&self.worktree)?;
        }

        let isolated_paths = WorkspacePaths::new(&self.worktree);
        let rendered = super::api_coverage::render_refreshed_contract_candidate(&isolated_paths)?;
        validate_promotion_candidate(&self.worktree, manifest, &rendered)?;

        for artifact in manifest.reviewed_artifacts() {
            let candidate = artifact.candidate_path.as_deref().ok_or_else(|| {
                Error::message(format!(
                    "reviewed artifact `{}` has no target candidate_path",
                    artifact.name
                ))
            })?;
            let source = self.worktree.join(candidate);
            let content = fs::read(&source).map_err(|error| Error::io(&source, error))?;
            write_atomic_bytes(&self.worktree.join(&artifact.path), &content)?;
        }

        let generated = super::api_coverage::render_generated_outputs(&isolated_paths)?;
        let recording_wire =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::RecordingWire)?;
        let report =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::ApiCoverageReport)?;
        write_atomic_bytes(&recording_wire, &generated.recording_wire)?;
        write_atomic_bytes(
            &self
                .worktree
                .join(super::api_coverage::RUNTIME_RECORDING_WIRE_PATH),
            &generated.runtime_recording_wire,
        )?;
        write_atomic_bytes(&report, &generated.report)?;
        for artifact in &mut target_manifest.artifacts {
            artifact.content_blake3 = file_blake3(&self.worktree.join(&artifact.path))?;
        }
        target_manifest.artifact_digests_initialized = true;
        write_atomic(
            &self.worktree.join("boxdd-sys/upstream.toml"),
            &render_toml(&target_manifest)?,
        )?;
        super::api_coverage::check(&isolated_paths)?;
        validate_repository(&isolated_paths, &target_manifest, false)?;

        let artifacts = target_manifest
            .artifacts
            .iter()
            .map(|artifact| {
                let path = self.worktree.join(&artifact.path);
                let content = fs::read(&path).map_err(|error| Error::io(&path, error))?;
                Ok(StagedFile {
                    relative_path: artifact.path.clone(),
                    content,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let candidate_paths = manifest
            .artifacts
            .iter()
            .filter_map(|artifact| artifact.candidate_path.clone())
            .collect();
        Ok(StagedUpdate {
            manifest: target_manifest,
            artifacts,
            candidate_paths,
        })
    }

    fn prepare_route_refresh(
        &self,
        original: &UpstreamManifest,
        target: &UpstreamManifest,
    ) -> Result<RouteRefreshStaging> {
        let mut target_manifest = target.clone();
        validate_route_refresh_topology(&target_manifest)?;
        write_atomic(
            &self.worktree.join("boxdd-sys/upstream.toml"),
            &render_toml(&target_manifest)?,
        )?;
        set_indexed_gitlink(&self.worktree, &target_manifest.active_revision)?;

        self.generate_bindings(&target_manifest)?;
        self.generate_abi_metadata(&target_manifest)?;
        if target_manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::ProviderIdentity)
        {
            let provider_sdk = super::provider::qualified_provider_sdk_for(&self.worktree)?;
            super::provider::refresh_wasm_provider_contracts_unlocked(
                &self.worktree,
                &self.target_dir.join("wasm-provider-contracts"),
                &provider_sdk,
            )?;
            super::provider::validate_checked_wasm_provider_contracts(&self.worktree)?;
        }
        let isolated_paths = WorkspacePaths::new(&self.worktree);
        let contract = super::api_coverage::render_refreshed_contract_candidate(&isolated_paths)?;
        let contract_path =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::ApiContract)?;
        write_atomic_bytes(&contract_path, &contract)?;

        let generated = super::api_coverage::render_generated_outputs(&isolated_paths)?;
        let recording_wire =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::RecordingWire)?;
        let runtime_recording_wire = self
            .worktree
            .join(super::api_coverage::RUNTIME_RECORDING_WIRE_PATH);
        let report =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::ApiCoverageReport)?;
        write_atomic_bytes(&recording_wire, &generated.recording_wire)?;
        write_atomic_bytes(&runtime_recording_wire, &generated.runtime_recording_wire)?;
        write_atomic_bytes(&report, &generated.report)?;

        for artifact in &mut target_manifest.artifacts {
            artifact.content_blake3 = file_blake3(&self.worktree.join(&artifact.path))?;
        }
        target_manifest.artifact_digests_initialized = true;
        validate_manifest(&target_manifest)?;
        validate_route_refresh_topology(&target_manifest)?;
        write_atomic(
            &self.worktree.join("boxdd-sys/upstream.toml"),
            &render_toml(&target_manifest)?,
        )?;
        super::api_coverage::check(&isolated_paths)?;
        validate_repository(&isolated_paths, &target_manifest, false)?;

        let mut files = target_manifest
            .artifacts
            .iter()
            .map(|artifact| {
                let path = self.worktree.join(&artifact.path);
                let content = fs::read(&path).map_err(|source| Error::io(&path, source))?;
                Ok(StagedFile {
                    relative_path: artifact.path.clone(),
                    content,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        files.push(StagedFile {
            relative_path: super::api_coverage::RUNTIME_RECORDING_WIRE_PATH.to_owned(),
            content: fs::read(&runtime_recording_wire)
                .map_err(|source| Error::io(&runtime_recording_wire, source))?,
        });
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        let target_paths = target_manifest
            .artifacts
            .iter()
            .map(|artifact| artifact.path.as_str())
            .collect::<BTreeSet<_>>();
        let mut removals = original
            .artifacts
            .iter()
            .filter(|artifact| !target_paths.contains(artifact.path.as_str()))
            .map(|artifact| artifact.path.clone())
            .collect::<Vec<_>>();
        removals.sort();
        Ok(RouteRefreshStaging {
            manifest: target_manifest,
            files,
            removals,
        })
    }

    fn render_next_candidate(&self, manifest: &UpstreamManifest, target: &str) -> Result<Vec<u8>> {
        let target_manifest = manifest.promoted_for_generation(target)?;
        write_atomic(
            &self.worktree.join("boxdd-sys/upstream.toml"),
            &render_toml(&target_manifest)?,
        )?;
        set_indexed_gitlink(&self.worktree, target)?;
        let isolated_paths = WorkspacePaths::new(&self.worktree);
        validate_repository_core(&isolated_paths, &target_manifest)?;
        validate_recording_input_identities(&isolated_paths, &target_manifest)?;
        validate_recording_operations(&isolated_paths, &target_manifest)?;
        self.generate_bindings(&target_manifest)?;
        self.generate_abi_metadata(&target_manifest)?;
        super::api_coverage::render_refreshed_contract_candidate(&isolated_paths)
    }

    fn generate_bindings(&self, manifest: &UpstreamManifest) -> Result<()> {
        let cargo = self.qualified_cargo()?;
        let generation_targets = manifest
            .binding_artifacts()
            .map(|artifact| binding_generation_target(manifest, artifact))
            .collect::<Result<BTreeSet<_>>>()?;
        let wasi_sysroot = if generation_targets.contains(&RustTarget::Wasm32Wasip1) {
            binding_generation_wasi_sysroot(RustTarget::Wasm32Wasip1)?
        } else {
            None
        };
        let freestanding_headers = if generation_targets.contains(&RustTarget::Wasm32UnknownUnknown)
        {
            bindgen_contract::resolve_unknown_unknown_headers(
                &self.worktree.join("boxdd-sys"),
                RustTarget::Wasm32UnknownUnknown.as_str(),
                true,
            )
            .map_err(Error::message)?
        } else {
            None
        };
        for artifact in manifest.binding_artifacts() {
            let precision = artifact
                .precision
                .ok_or_else(|| Error::message(format!("{} has no precision", artifact.name)))?;
            let rust_target = binding_generation_target(manifest, artifact)?;
            let artifact_target = self.target_dir.join(&artifact.name);
            let features = match precision {
                Precision::Single => "bindgen",
                Precision::Double => "bindgen,double-precision",
            };
            bindgen_contract::validate_bindgen_target_override(
                rust_target.as_str(),
                Some(std::ffi::OsStr::new(rust_target.as_str())),
            )
            .map_err(Error::message)?;
            let mut command = cargo.command_at_working_root(&artifact_target)?;
            command
                .args(binding_generation_cargo_args(rust_target, features))
                .env("BOXDD_SYS_SKIP_CC", "1")
                .env("BOXDD_SYS_FORCE_BINDGEN", "1")
                .env("BOXDD_SYS_BINDGEN_TARGET", rust_target.as_str())
                .env(
                    "BOXDD_SYS_PROVIDER",
                    binding_generation_provider(rust_target),
                );
            if rust_target == RustTarget::Wasm32Wasip1
                && let Some(wasi_sysroot) = &wasi_sysroot
            {
                command.env("BOXDD_SYS_WASI_SYSROOT", &wasi_sysroot.canonical_path);
            }
            command_success(
                &mut command,
                &format!("generate {} bindings", artifact.name),
            )?;
            let mut candidates = Vec::new();
            collect_generated_bindings(
                &artifact_target.join(rust_target.as_str()),
                &mut candidates,
            )?;
            if candidates.len() != 1 {
                return Err(Error::message(format!(
                    "{} generation produced {} bindings.rs candidates; expected exactly one",
                    artifact.name,
                    candidates.len()
                )));
            }
            let content = fs::read_to_string(&candidates[0])
                .map_err(|source| Error::io(&candidates[0], source))?;
            write_atomic(
                &self.worktree.join(&artifact.path),
                &format!(
                    "{}\n{content}",
                    binding_provenance(
                        artifact,
                        &manifest.active_revision,
                        rust_target,
                        (rust_target == RustTarget::Wasm32Wasip1)
                            .then_some(wasi_sysroot.as_ref())
                            .flatten()
                            .map(bindgen_contract::ValidatedWasiSysroot::identity_sha256),
                        (rust_target == RustTarget::Wasm32UnknownUnknown)
                            .then_some(freestanding_headers.as_ref())
                            .flatten()
                            .map(bindgen_contract::ValidatedFreestandingHeaders::identity_sha256),
                    )?
                ),
            )?;
        }
        Ok(())
    }

    fn generate_abi_metadata(&self, manifest: &UpstreamManifest) -> Result<()> {
        let artifacts = manifest.abi_metadata_artifacts().collect::<Vec<_>>();
        if artifacts.is_empty() {
            #[cfg(test)]
            return Ok(());
            #[cfg(not(test))]
            return Err(Error::message(
                "upstream manifest has no C ABI metadata artifacts to generate",
            ));
        }

        let mut precisions = BTreeSet::new();
        for artifact in artifacts {
            let precision = artifact.precision.ok_or_else(|| {
                Error::message(format!(
                    "ABI metadata artifact `{}` has no precision",
                    artifact.name
                ))
            })?;
            let content = render_abi_probe_metadata(&self.worktree, manifest, artifact)?;
            write_atomic_bytes(&self.worktree.join(&artifact.path), &content)?;
            precisions.insert(precision);
        }
        let cargo = self.qualified_cargo()?;
        for precision in precisions {
            run_abi_probe_test(cargo, &self.target_dir.join("abi-probe"), precision)?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<()> {
        let result = self.cleanup();
        if result.is_ok() {
            self.repository_worktree_added = false;
        }
        result
    }

    fn cleanup(&mut self) -> Result<()> {
        if self.source_directory.is_none() {
            return Ok(());
        }
        if self.worktree.exists()
            && let Err(error) = ensure_no_pending_atomic_batches_for_workspace(&self.worktree)
        {
            let preserved = self
                .source_directory
                .take()
                .expect("checked isolated source directory")
                .keep();
            return Err(Error::message(format!(
                "{error}\nisolated source directory preserved at {} because atomic batch recovery is pending",
                preserved.display()
            )));
        }
        let registration = worktree_is_registered(&self.repository_root, &self.worktree)
            .map_err(|error| Error::message(format!("inspect isolated worktree: {error}")));
        self.cleanup_after_inspection(registration)
    }

    fn cleanup_after_inspection(&mut self, registration: Result<bool>) -> Result<()> {
        self.cleanup_after_inspection_with(registration, remove_repository_worktree)
    }

    fn cleanup_after_inspection_with<F>(
        &mut self,
        registration: Result<bool>,
        remove_worktree: F,
    ) -> Result<()>
    where
        F: FnOnce(&Path, &Path) -> Result<()>,
    {
        let mut errors = Vec::new();
        let should_attempt_registered_removal = match registration {
            Ok(registered) => registered,
            Err(error) => {
                errors.push(error.to_string());
                true
            }
        };
        if should_attempt_registered_removal {
            if let Err(error) = remove_worktree(&self.repository_root, &self.worktree) {
                errors.push(error.to_string());
                if let Some(directory) = self.source_directory.take() {
                    let preserved = directory.keep();
                    errors.push(format!(
                        "isolated source directory preserved at {} because its Git worktree registration could not be removed",
                        preserved.display()
                    ));
                }
                return Err(Error::message(errors.join("\n")));
            } else {
                self.repository_worktree_added = false;
            }
        } else {
            self.repository_worktree_added = false;
        }
        if let Some(directory) = self.source_directory.take() {
            let path = directory.path().to_owned();
            if let Err(source) = directory.close() {
                errors.push(Error::io(path, source).to_string());
            } else {
                self.repository_worktree_added = false;
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::message(errors.join("\n")))
        }
    }
}

fn remove_repository_worktree(repository: &Path, worktree: &Path) -> Result<()> {
    git_command().and_then(|mut command| {
        command
            .current_dir(repository)
            .args(["worktree", "remove", "--force"])
            .arg(worktree);
        command_success(&mut command, "remove isolated repository worktree")
    })
}

fn isolated_generation_parent(root: &Path) -> Result<PathBuf> {
    let isolation_anchor =
        repository_lock_path(root, Path::new("boxdd-isolated-generation.anchor"))
            .map_err(Error::message)?;
    isolation_anchor
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            Error::message(format!(
                "repository-owned isolation anchor has no parent: {}",
                isolation_anchor.display()
            ))
        })
}

fn write_isolated_generation_marker(
    common_directory: &Path,
    source_root: &Path,
    worktree: &Path,
) -> Result<()> {
    let marker = IsolatedGenerationMarker {
        schema: ISOLATED_GENERATION_MARKER_SCHEMA,
        common_directory: common_directory.to_path_buf(),
        source_root: source_root.to_path_buf(),
        worktree: worktree.to_path_buf(),
    };
    let path = source_root.join(ISOLATED_GENERATION_MARKER);
    write_atomic(&path, &render_toml(&marker)?)?;
    let installed = read_toml::<IsolatedGenerationMarker>(&path)?;
    if installed == marker {
        Ok(())
    } else {
        Err(Error::message(format!(
            "isolated generation ownership marker changed while it was installed: {}",
            path.display()
        )))
    }
}

fn cleanup_deferred_isolated_generations(root: &Path) -> Result<()> {
    let common_directory = isolated_generation_parent(root)?;
    let mut candidates = fs::read_dir(&common_directory)
        .map_err(|source| Error::io(&common_directory, source))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|source| Error::io(&common_directory, source))?;
    candidates.sort();

    for candidate in candidates {
        let Some(name) = candidate.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with(ISOLATED_GENERATION_DIRECTORY_PREFIX) {
            continue;
        }
        let candidate_metadata =
            fs::symlink_metadata(&candidate).map_err(|source| Error::io(&candidate, source))?;
        if !candidate_metadata.file_type().is_dir() || candidate_metadata.file_type().is_symlink() {
            return Err(Error::message(format!(
                "deferred isolated generation candidate is not a real directory; preserved at {}",
                candidate.display()
            )));
        }
        let marker_path = candidate.join(ISOLATED_GENERATION_MARKER);
        match fs::symlink_metadata(&marker_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(Error::io(&marker_path, error)),
            Ok(metadata)
                if !metadata.file_type().is_file() || metadata.file_type().is_symlink() =>
            {
                return Err(Error::message(format!(
                    "deferred isolated generation marker is not a regular non-symlink file: {}",
                    marker_path.display()
                )));
            }
            Ok(_) => {}
        }

        let source_root = candidate
            .canonicalize()
            .map_err(|source| Error::io(&candidate, source))?;
        let marker = read_toml::<IsolatedGenerationMarker>(&marker_path)?;
        let expected_worktree = source_root.join("workspace");
        if marker.schema != ISOLATED_GENERATION_MARKER_SCHEMA
            || marker.common_directory != common_directory
            || marker.source_root != source_root
            || marker.worktree != expected_worktree
            || source_root.parent() != Some(common_directory.as_path())
        {
            return Err(Error::message(format!(
                "deferred isolated generation marker does not own its directory; preserved at {}",
                source_root.display()
            )));
        }
        if worktree_is_registered(root, &expected_worktree)? {
            remove_repository_worktree(root, &expected_worktree)?;
        }
        fs::remove_dir_all(&source_root).map_err(|source| Error::io(&source_root, source))?;
    }
    Ok(())
}

fn merge_isolated_initialization_failure(error: Error, cleanup: Result<()>) -> Error {
    match cleanup {
        Ok(()) => error,
        Err(cleanup) => Error::message(format!(
            "isolated worktree initialization failed: {error}\nisolated worktree cleanup also failed: {cleanup}"
        )),
    }
}

fn binding_generation_target(
    manifest: &UpstreamManifest,
    artifact: &GeneratedArtifact,
) -> Result<RustTarget> {
    let targets = manifest
        .binding_routes
        .iter()
        .filter(|route| route.artifact == artifact.name)
        .map(|route| route.rust_target)
        .collect::<BTreeSet<_>>();
    let mut targets = targets.into_iter();
    let target = targets.next().ok_or_else(|| {
        Error::message(format!(
            "bindings artifact `{}` has no manifest route generation target",
            artifact.name
        ))
    })?;
    if targets.next().is_some() {
        return Err(Error::message(format!(
            "bindings artifact `{}` is shared by routes with multiple Rust targets; declare one generated artifact per target",
            artifact.name
        )));
    }
    Ok(target)
}

fn binding_generation_cargo_args(target: RustTarget, features: &str) -> Vec<String> {
    [
        "build",
        "--locked",
        "--target",
        target.as_str(),
        "-p",
        "boxdd-sys",
        "--features",
        features,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

const fn binding_generation_provider(target: RustTarget) -> &'static str {
    match target {
        RustTarget::X86_64UnknownLinuxGnu => "vendored",
        RustTarget::Wasm32UnknownUnknown | RustTarget::Wasm32Wasip1 => "wasm-compile-only",
    }
}

fn binding_generation_wasi_sysroot(
    target: RustTarget,
) -> Result<Option<bindgen_contract::ValidatedWasiSysroot>> {
    if target != RustTarget::Wasm32Wasip1 {
        return Ok(None);
    }
    let configured = std::env::var_os("BOXDD_SYS_WASI_SYSROOT").map(PathBuf::from);
    bindgen_contract::resolve_wasi_sysroot(target.as_str(), true, configured.as_deref())
        .map_err(Error::message)
}

fn worktree_is_registered(repository: &Path, worktree: &Path) -> Result<bool> {
    let output = git_output(repository, ["worktree", "list", "--porcelain"])?;
    let requested = fs::canonicalize(worktree).unwrap_or_else(|_| worktree.to_owned());
    Ok(output
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .map(|registered| fs::canonicalize(&registered).unwrap_or(registered))
        .any(|registered| registered == requested))
}

impl Drop for IsolatedGeneration {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn collect_generated_bindings(dir: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        if path.is_dir() {
            collect_generated_bindings(&path, output)?;
        } else if path.file_name().is_some_and(|name| name == "bindings.rs")
            && path.components().any(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .starts_with("boxdd-sys-")
            })
        {
            output.push(path);
        }
    }
    output.sort();
    Ok(())
}

fn validate_artifact_identities(paths: &WorkspacePaths, manifest: &UpstreamManifest) -> Result<()> {
    for artifact in &manifest.artifacts {
        let path = paths.root().join(&artifact.path);
        let metadata = fs::symlink_metadata(&path).map_err(|source| Error::io(&path, source))?;
        if !metadata.file_type().is_file() {
            return Err(Error::message(format!(
                "artifact `{}` is not a regular file: {}",
                artifact.name,
                path.display()
            )));
        }
        validate_file_blake3(&path, &artifact.content_blake3, &artifact.name)?;
    }
    validate_artifact_revision_identities(paths, manifest)
}

fn validate_artifact_revision_identities(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    for artifact in &manifest.artifacts {
        let path = paths.root().join(&artifact.path);
        let metadata = fs::symlink_metadata(&path).map_err(|source| Error::io(&path, source))?;
        if !metadata.file_type().is_file() {
            return Err(Error::message(format!(
                "artifact `{}` is not a regular file: {}",
                artifact.name,
                path.display()
            )));
        }
        match artifact.kind {
            ArtifactKind::Bindings => validate_binding_identity(&path, artifact, manifest)?,
            ArtifactKind::AbiMetadata | ArtifactKind::ProviderIdentity => {
                let identity = read_revision_identity(&path)?;
                if identity.upstream_sha != manifest.active_revision {
                    return Err(Error::message(format!(
                        "artifact `{}` revision {} does not match active revision {}",
                        artifact.name, identity.upstream_sha, manifest.active_revision
                    )));
                }
            }
            ArtifactKind::ApiContract
            | ArtifactKind::RecordingWire
            | ArtifactKind::ApiCoverageReport => {}
        }
    }
    let api_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let api = read_revision_identity(&api_path)?;
    if api.upstream_sha != manifest.active_revision {
        return Err(Error::message(format!(
            "API contract revision {} does not match active revision {}",
            api.upstream_sha, manifest.active_revision
        )));
    }
    let wire_path = manifest.artifact_path(paths.root(), ArtifactKind::RecordingWire)?;
    let wire = read_revision_identity(&wire_path)?;
    if wire.upstream_sha != manifest.recording_revision {
        return Err(Error::message(format!(
            "recording wire revision {} does not match recording revision {}",
            wire.upstream_sha, manifest.recording_revision
        )));
    }
    let report_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiCoverageReport)?;
    let report =
        fs::read_to_string(&report_path).map_err(|source| Error::io(&report_path, source))?;
    let marker = format!("Pinned active upstream: `{}`.", manifest.active_revision);
    if !report.contains(&marker) {
        return Err(Error::message(format!(
            "API coverage report does not contain active revision marker `{marker}`"
        )));
    }
    Ok(())
}

fn binding_provenance(
    artifact: &GeneratedArtifact,
    revision: &str,
    rust_target: RustTarget,
    wasi_headers_sha256: Option<&str>,
    freestanding_math_sha256: Option<&str>,
) -> Result<String> {
    let precision = artifact.precision.map(Precision::as_str).unwrap_or("none");
    let (wasi_libc_version, wasi_headers_sha256, freestanding_math_sha256) = match rust_target {
        RustTarget::Wasm32Wasip1 => {
            if freestanding_math_sha256.is_some() {
                return Err(Error::message(format!(
                    "WASI bindings artifact `{}` cannot claim freestanding headers",
                    artifact.name
                )));
            }
            let identity = wasi_headers_sha256.ok_or_else(|| {
                Error::message(format!(
                    "WASI bindings artifact `{}` has no validated wasi-libc header identity",
                    artifact.name
                ))
            })?;
            if identity != bindgen_contract::WASI_LIBC_HEADERS_SHA256 {
                return Err(Error::message(format!(
                    "WASI bindings artifact `{}` used header identity {identity}, expected wasi-libc {} identity {}",
                    artifact.name,
                    bindgen_contract::WASI_LIBC_VERSION,
                    bindgen_contract::WASI_LIBC_HEADERS_SHA256
                )));
            }
            (bindgen_contract::WASI_LIBC_VERSION, identity, "none")
        }
        RustTarget::Wasm32UnknownUnknown => {
            if wasi_headers_sha256.is_some() {
                return Err(Error::message(format!(
                    "freestanding bindings artifact `{}` cannot claim a wasi-libc identity",
                    artifact.name
                )));
            }
            let identity = freestanding_math_sha256.ok_or_else(|| {
                Error::message(format!(
                    "freestanding bindings artifact `{}` has no validated math header identity",
                    artifact.name
                ))
            })?;
            if identity != bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256 {
                return Err(Error::message(format!(
                    "freestanding bindings artifact `{}` used math header identity {identity}, expected {}",
                    artifact.name,
                    bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256
                )));
            }
            ("none", "none", identity)
        }
        RustTarget::X86_64UnknownLinuxGnu => {
            if wasi_headers_sha256.is_some() || freestanding_math_sha256.is_some() {
                return Err(Error::message(format!(
                    "native bindings artifact `{}` cannot claim WASM header identities",
                    artifact.name
                )));
            }
            ("none", "none", "none")
        }
    };
    Ok(format!(
        "// AUTOGENERATED: pregenerated bindings for docs.rs/offline builds\n\
// boxdd-upstream-revision: {revision}\n\
// boxdd-artifact-name: {}\n\
// boxdd-artifact-precision: {precision}\n\
// boxdd-artifact-target: {}\n\
// boxdd-artifact-provider: {}\n\
// boxdd-artifact-producer: {}\n\
// boxdd-artifact-rust-target: {}\n\
// boxdd-wasi-libc-version: {wasi_libc_version}\n\
// boxdd-wasi-headers-sha256: {wasi_headers_sha256}\n\
// boxdd-freestanding-math-header-sha256: {freestanding_math_sha256}\n\
// Authority: boxdd-sys/upstream.toml\n\
// Refresh with: cargo run -p xtask -- upstream-sync --refresh-routes\n",
        artifact.name,
        artifact.target.as_str(),
        artifact.provider.as_str(),
        artifact.producer.as_str(),
        rust_target.as_str(),
    ))
}

fn validate_binding_identity(
    path: &Path,
    artifact: &GeneratedArtifact,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let source = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let rust_target = binding_generation_target(manifest, artifact)?;
    let expected = binding_provenance(
        artifact,
        &manifest.active_revision,
        rust_target,
        (rust_target == RustTarget::Wasm32Wasip1)
            .then_some(bindgen_contract::WASI_LIBC_HEADERS_SHA256),
        (rust_target == RustTarget::Wasm32UnknownUnknown)
            .then_some(bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256),
    )?;
    if !source.starts_with(&expected) {
        return Err(Error::message(format!(
            "bindings artifact `{}` is missing exact manifest provenance for revision {}",
            artifact.name, manifest.active_revision
        )));
    }
    Ok(())
}

fn validate_candidate_identities(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    for artifact in manifest.reviewed_artifacts() {
        let Some(candidate) = &artifact.candidate_path else {
            continue;
        };
        let expected = manifest.next_revision.as_deref().ok_or_else(|| {
            Error::message(format!(
                "artifact `{}` has candidate_path but the manifest has no next_revision",
                artifact.name
            ))
        })?;
        let path = paths.root().join(candidate);
        let candidate_blake3 = artifact.candidate_blake3.as_deref().ok_or_else(|| {
            Error::message(format!(
                "artifact `{}` has no candidate_blake3",
                artifact.name
            ))
        })?;
        validate_file_blake3(
            &path,
            candidate_blake3,
            &format!("{} candidate", artifact.name),
        )?;
        let identity = read_revision_identity(&path)?;
        if identity.upstream_sha != expected {
            return Err(Error::message(format!(
                "artifact candidate `{}` revision {} does not match next revision {expected}",
                artifact.name, identity.upstream_sha
            )));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct RevisionIdentity {
    upstream_sha: String,
}

fn read_revision_identity(path: &Path) -> Result<RevisionIdentity> {
    read_toml(path)
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
            "artifact `{label}` content digest drifted: expected {expected}, observed {actual}"
        )))
    }
}

fn validate_recording_input_identities(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let mut errors = Vec::new();
    for input in &manifest.recording_inputs {
        let actual_blob =
            git_blob_identity(&paths.box2d(), &manifest.recording_revision, &input.path)?;
        if actual_blob != input.git_blob {
            errors.push(format!(
                "reviewed recording input {}:{} has blob {actual_blob}, expected {}; a new blob requires explicit wire review",
                manifest.recording_revision, input.path, input.git_blob
            ));
        }
        let actual = git_blob_blake3(&paths.box2d(), &manifest.recording_revision, &input.path)?;
        if actual != input.blake3 {
            errors.push(format!(
                "reviewed recording input {}:{} drifted: expected {}, observed {actual}; a new digest requires explicit wire review",
                manifest.recording_revision, input.path, input.blake3
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn git_blob_identity(repository: &Path, revision: &str, path: &str) -> Result<String> {
    git_output(repository, ["rev-parse", &format!("{revision}:{path}")])
        .map(|value| value.trim().to_owned())
}

fn git_blob_blake3(repository: &Path, revision: &str, path: &str) -> Result<String> {
    Ok(blake3::hash(&git_blob_bytes(repository, revision, path)?)
        .to_hex()
        .to_string())
}

fn git_blob_bytes(repository: &Path, revision: &str, path: &str) -> Result<Vec<u8>> {
    let object = format!("{revision}:{path}");
    let output = git_command()?
        .current_dir(repository)
        .args(["cat-file", "blob", &object])
        .output()
        .map_err(|source| Error::io("git cat-file", source))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "git cat-file blob {object} failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

fn validate_recording_operations(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let source = recording_operations_source(paths, manifest)?;
    recording_ops::parse(&source).map(|_| ())
}

pub(crate) fn reviewed_recording_operations_source(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<String> {
    validate_recording_input_identities(paths, manifest)?;
    recording_operations_source(paths, manifest)
}

fn recording_operations_source(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<String> {
    const PATH: &str = "src/recording_ops.inl";
    let bytes = git_blob_bytes(&paths.box2d(), &manifest.recording_revision, PATH)?;
    String::from_utf8(bytes).map_err(|error| {
        Error::message(format!(
            "{}:{PATH} is not UTF-8: {error}",
            manifest.recording_revision
        ))
    })
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
        let extension = candidate.extension().and_then(|value| value.to_str());
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

fn validate_exact_inventory(expected: &SourceInventory, actual: &SourceInventory) -> Result<()> {
    if expected == actual {
        return Ok(());
    }
    let mut details = Vec::new();
    if expected.tree != actual.tree {
        details.push(format!(
            "tree expected {}, observed {}",
            expected.tree, actual.tree
        ));
    }
    for (label, expected, actual) in [
        ("C sources", &expected.c_sources, &actual.c_sources),
        (
            "private headers",
            &expected.private_headers,
            &actual.private_headers,
        ),
        ("inline files", &expected.inline_files, &actual.inline_files),
        (
            "public headers",
            &expected.public_headers,
            &actual.public_headers,
        ),
    ] {
        let expected = expected.iter().collect::<BTreeSet<_>>();
        let actual = actual.iter().collect::<BTreeSet<_>>();
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        let unexpected = actual.difference(&expected).copied().collect::<Vec<_>>();
        if !missing.is_empty() || !unexpected.is_empty() {
            details.push(format!(
                "{label}: missing {missing:?}, unexpected {unexpected:?}"
            ));
        }
    }
    Err(Error::message(format!(
        "target source inventory drifted:\n{}",
        details.join("\n")
    )))
}

fn managed_status(root: &Path, manifest: &UpstreamManifest) -> Result<String> {
    let mut command = git_command()?;
    command
        .current_dir(root)
        .args(["status", "--porcelain=v1", "--untracked-files=all", "--"]);
    command.arg("boxdd-sys/upstream.toml");
    command.args(
        manifest
            .artifacts
            .iter()
            .chain(&manifest.next_artifacts)
            .map(|artifact| &artifact.path),
    );
    command.args(
        manifest
            .artifacts
            .iter()
            .filter_map(|artifact| artifact.candidate_path.as_ref()),
    );
    output_text(
        command
            .output()
            .map_err(|source| Error::io("git", source))?,
        "git status",
    )
}

fn reject_bootstrap_artifact_changes_if_present(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let mut command = git_command()?;
    command.current_dir(paths.root()).args([
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        "--",
    ]);
    command.args(manifest.artifacts.iter().map(|artifact| &artifact.path));
    let dirty = output_text(
        command
            .output()
            .map_err(|source| Error::io("git", source))?,
        "git status",
    )?;
    if dirty.trim().is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "artifact digest bootstrap refuses dirty generated artifacts; preserve and review them before bootstrapping:\n{dirty}"
        )))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GitIndexSnapshot {
    path: PathBuf,
    content: Vec<u8>,
}

impl GitIndexSnapshot {
    fn capture(root: &Path) -> Result<Self> {
        let path = repository_git_path(root, "index")?;
        let metadata = fs::symlink_metadata(&path).map_err(|source| Error::io(&path, source))?;
        if !metadata.file_type().is_file() {
            return Err(Error::message(format!(
                "Git index {} is not a regular file",
                path.display()
            )));
        }
        let content = fs::read(&path).map_err(|source| Error::io(&path, source))?;
        Ok(Self { path, content })
    }

    fn digest(&self) -> String {
        blake3::hash(&self.content).to_hex().to_string()
    }
}

#[derive(Debug)]
struct GitIndexReplacement {
    root: PathBuf,
    before: GitIndexSnapshot,
    after: GitIndexSnapshot,
    installed: bool,
}

impl GitIndexReplacement {
    fn prepare(root: &Path, before: GitIndexSnapshot, revision: &str) -> Result<Self> {
        let after = index_with_gitlink(root, &before, revision)?;
        Ok(Self {
            root: root.to_owned(),
            before,
            after,
            installed: false,
        })
    }

    fn install(&mut self) -> Result<()> {
        if self.installed {
            return Err(Error::message("Git index replacement is already installed"));
        }
        let outcome = compare_and_swap_index(&self.before, &self.after)?;
        self.installed = true;
        outcome.finish()
    }

    fn rollback(&mut self) -> Result<()> {
        if !self.installed {
            return Ok(());
        }
        let actual = GitIndexSnapshot::capture(&self.root)?;
        if actual == self.before {
            self.installed = false;
            return Ok(());
        }
        if actual != self.after {
            return Err(Error::message(format!(
                "rollback conflict at root Git index: observed {}, expected transaction state {}; preserving the concurrent state",
                actual.digest(),
                self.after.digest()
            )));
        }
        let outcome = compare_and_swap_index(&self.after, &self.before)?;
        self.installed = false;
        outcome.finish()
    }
}

struct GitIndexLock {
    path: PathBuf,
    file: Option<fs::File>,
}

impl GitIndexLock {
    fn acquire(index: &Path) -> Result<Self> {
        let mut lock_name = index.as_os_str().to_os_string();
        lock_name.push(".lock");
        let path = PathBuf::from(lock_name);
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                Error::message(format!(
                    "could not acquire root Git index lock {}: {source}",
                    path.display()
                ))
            })?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    fn release(mut self) -> Option<String> {
        self.file.take();
        match fs::remove_file(&self.path) {
            Ok(()) => None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => Some(format!(
                "could not remove root Git index lock {}: {error}",
                self.path.display()
            )),
        }
    }
}

impl Drop for GitIndexLock {
    fn drop(&mut self) {
        self.file.take();
        let _ = fs::remove_file(&self.path);
    }
}

struct IndexCasOutcome {
    cleanup_error: Option<String>,
}

impl IndexCasOutcome {
    fn finish(self) -> Result<()> {
        match self.cleanup_error {
            Some(error) => Err(Error::message(error)),
            None => Ok(()),
        }
    }
}

fn compare_and_swap_index(
    expected: &GitIndexSnapshot,
    replacement: &GitIndexSnapshot,
) -> Result<IndexCasOutcome> {
    if expected.path != replacement.path {
        return Err(Error::message("Git index CAS paths do not match"));
    }
    let parent = expected
        .path
        .parent()
        .ok_or_else(|| Error::message("root Git index has no parent directory"))?;
    let lock = GitIndexLock::acquire(&expected.path)?;
    let actual = GitIndexSnapshot {
        path: expected.path.clone(),
        content: fs::read(&expected.path).map_err(|source| Error::io(&expected.path, source))?,
    };
    if actual != *expected {
        return Err(Error::message(format!(
            "root Git index changed during transaction: expected {}, observed {}; preserving the concurrent state",
            expected.digest(),
            actual.digest()
        )));
    }
    let permissions = fs::metadata(&expected.path)
        .map_err(|source| Error::io(&expected.path, source))?
        .permissions();
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| Error::io(temporary.path(), source))?;
    temporary
        .as_file_mut()
        .write_all(&replacement.content)
        .map_err(|source| Error::io(temporary.path(), source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(temporary.path(), source))?;
    temporary
        .persist(&expected.path)
        .map_err(|error| Error::io(&expected.path, error.error))?;
    Ok(IndexCasOutcome {
        cleanup_error: lock.release(),
    })
}

fn index_with_gitlink(
    root: &Path,
    baseline: &GitIndexSnapshot,
    revision: &str,
) -> Result<GitIndexSnapshot> {
    let temporary = materialize_index_snapshot(baseline)?;
    let index = temporary.path();
    let cache_info = format!("160000,{revision},{BOX2D_GITLINK}");
    command_success(
        git_command()?
            .current_dir(root)
            .env("GIT_INDEX_FILE", index)
            .args(["update-index", "--add", "--cacheinfo", &cache_info]),
        &format!("prepare indexed Box2D gitlink {revision}"),
    )?;
    Ok(GitIndexSnapshot {
        path: baseline.path.clone(),
        content: fs::read(index).map_err(|source| Error::io(index, source))?,
    })
}

fn materialize_index_snapshot(snapshot: &GitIndexSnapshot) -> Result<NamedTempFile> {
    let parent = snapshot
        .path
        .parent()
        .ok_or_else(|| Error::message("root Git index has no parent directory"))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    temporary
        .as_file_mut()
        .write_all(&snapshot.content)
        .map_err(|source| Error::io(temporary.path(), source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(temporary.path(), source))?;
    Ok(temporary)
}

fn repository_git_path(root: &Path, name: &str) -> Result<PathBuf> {
    let raw = git_output(root, ["rev-parse", "--git-path", name])?;
    let path = PathBuf::from(raw.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::message(format!(
            "Git returned an empty path for {name}"
        )));
    }
    Ok(if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
}

fn indexed_gitlink(root: &Path) -> Result<String> {
    let output = git_output(root, ["ls-files", "--stage", "--", BOX2D_GITLINK])?;
    parse_indexed_gitlink(&output)
}

fn indexed_gitlink_from_snapshot(root: &Path, snapshot: &GitIndexSnapshot) -> Result<String> {
    let temporary = materialize_index_snapshot(snapshot)?;
    let index = temporary.path();
    let output = git_command()?
        .current_dir(root)
        .env("GIT_INDEX_FILE", index)
        .args(["ls-files", "--stage", "--", BOX2D_GITLINK])
        .output()
        .map_err(|source| Error::io("git ls-files", source))?;
    parse_indexed_gitlink(&output_text(output, "git ls-files")?)
}

fn parse_indexed_gitlink(output: &str) -> Result<String> {
    let lines = output.lines().collect::<Vec<_>>();
    let Some(line) = lines
        .as_slice()
        .first()
        .copied()
        .filter(|_| lines.len() == 1)
    else {
        return Err(Error::message(format!(
            "{BOX2D_GITLINK} must have exactly one stage-0 index entry"
        )));
    };
    let Some((metadata, indexed_path)) = line.split_once('\t') else {
        return Err(Error::message(format!(
            "{BOX2D_GITLINK} has an invalid index entry"
        )));
    };
    let fields = metadata.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3
        || fields[0] != "160000"
        || !is_full_sha(fields[1])
        || fields[2] != "0"
        || indexed_path != BOX2D_GITLINK
    {
        return Err(Error::message(format!(
            "{BOX2D_GITLINK} is not a single stage-0 indexed Git submodule"
        )));
    }
    Ok(fields[1].to_owned())
}

fn set_indexed_gitlink(root: &Path, revision: &str) -> Result<()> {
    let cache_info = format!("160000,{revision},{BOX2D_GITLINK}");
    command_success(
        git_command()?.current_dir(root).args([
            "update-index",
            "--add",
            "--cacheinfo",
            &cache_info,
        ]),
        &format!("set indexed Box2D gitlink to {revision}"),
    )
}

fn ensure_commit_object(repository: &Path, revision: &str) -> Result<()> {
    let object = format!("{revision}^{{commit}}");
    command_success(
        git_command()?
            .current_dir(repository)
            .args(["cat-file", "-e", &object]),
        &format!("verify commit object {revision}"),
    )
}

fn checkout_detached(repository: &Path, revision: &str) -> Result<()> {
    command_success(
        git_command()?.current_dir(repository).args([
            "checkout",
            "--no-overwrite-ignore",
            "--detach",
            revision,
        ]),
        &format!("checkout exact revision {revision}"),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CheckoutState {
    revision: String,
    symbolic_ref: Option<String>,
    worktree_status: String,
}

impl CheckoutState {
    fn detached(revision: String) -> Self {
        Self {
            revision,
            symbolic_ref: None,
            worktree_status: String::new(),
        }
    }
}

fn checkout_state(repository: &Path) -> Result<CheckoutState> {
    let revision = git_output(repository, ["rev-parse", "HEAD"])?
        .trim()
        .to_owned();
    let output = git_command()?
        .current_dir(repository)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .map_err(|source| Error::io("git symbolic-ref", source))?;
    let symbolic_ref = if output.status.success() {
        let value = String::from_utf8(output.stdout)
            .map_err(|error| {
                Error::message(format!("git symbolic-ref emitted non-UTF-8: {error}"))
            })?
            .trim()
            .to_owned();
        if value.is_empty() {
            return Err(Error::message("git symbolic-ref returned an empty ref"));
        }
        Some(value)
    } else if output.status.code() == Some(1) {
        None
    } else {
        return Err(Error::message(format!(
            "git symbolic-ref failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    };
    let worktree_status = git_output(
        repository,
        ["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    Ok(CheckoutState {
        revision,
        symbolic_ref,
        worktree_status,
    })
}

fn restore_checkout_state(repository: &Path, state: &CheckoutState) -> Result<()> {
    let Some(symbolic_ref) = &state.symbolic_ref else {
        return checkout_detached(repository, &state.revision);
    };
    let referenced_revision = git_output(repository, ["rev-parse", symbolic_ref])?;
    if referenced_revision.trim() != state.revision {
        return Err(Error::message(format!(
            "cannot restore {symbolic_ref}: it moved from {} to {}",
            state.revision,
            referenced_revision.trim()
        )));
    }
    let branch = symbolic_ref.strip_prefix("refs/heads/").ok_or_else(|| {
        Error::message(format!(
            "cannot restore unsupported symbolic HEAD {symbolic_ref}"
        ))
    })?;
    command_success(
        git_command()?
            .current_dir(repository)
            .args(["checkout", "--no-overwrite-ignore", branch]),
        &format!("restore submodule checkout {symbolic_ref}"),
    )?;
    let restored = checkout_state(repository)?;
    if restored != *state {
        return Err(Error::message(format!(
            "restoring submodule checkout produced {restored:?}, expected {state:?}"
        )));
    }
    Ok(())
}

fn git_command() -> Result<Command> {
    qualified_git_command().map_err(Error::message)
}

fn git_output<const N: usize>(repository: &Path, args: [&str; N]) -> Result<String> {
    let output = git_command()?
        .current_dir(repository)
        .args(args)
        .output()
        .map_err(|source| Error::io("git", source))?;
    output_text(output, "git")
}

fn git_output_with_paths(repository: &Path, args: &[&str], paths: &[&str]) -> Result<String> {
    let output = git_command()?
        .current_dir(repository)
        .args(args)
        .args(paths)
        .output()
        .map_err(|source| Error::io("git", source))?;
    output_text(output, "git")
}

fn command_success(command: &mut Command, label: &str) -> Result<()> {
    let output = command
        .output()
        .map_err(|source| Error::io(label, source))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn output_text(output: Output, label: &str) -> Result<String> {
    if !output.status.success() {
        return Err(Error::message(format!(
            "{label} failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| Error::message(format!("{label} emitted non-UTF-8 output: {error}")))
}

fn is_full_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_blake3(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_artifact_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_canonical_manifest_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && value.split('/').all(|component| !component.is_empty())
        && is_safe_relative_path(Path::new(value))
        && canonical_manifest_path(Path::new(value)).as_deref() == Some(value)
}

fn canonical_manifest_path(path: &Path) -> Option<String> {
    let mut components = Vec::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            return None;
        };
        components.push(value.to_str()?);
    }
    (!components.is_empty()).then(|| components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UnrelatedGitState {
        staged_path: PathBuf,
        staged_content: Vec<u8>,
        unstaged_path: PathBuf,
        unstaged_content: Vec<u8>,
        untracked_path: PathBuf,
        untracked_content: Vec<u8>,
        indexed_staged_content: String,
        cached_diff: String,
        status: String,
    }

    impl UnrelatedGitState {
        fn create(root: &Path) -> Self {
            let staged_path = root.join("unrelated-staged.txt");
            let staged_content = b"staged user content\n".to_vec();
            fs::write(&staged_path, &staged_content).expect("unrelated staged content");
            run_git(root, &["add", "unrelated-staged.txt"]);

            let unstaged_path = root.join("unrelated-tracked.txt");
            let unstaged_content = b"unstaged user content\n".to_vec();
            fs::write(&unstaged_path, &unstaged_content).expect("unrelated unstaged content");

            let untracked_path = root.join("unrelated-untracked.txt");
            let untracked_content = b"untracked user content\n".to_vec();
            fs::write(&untracked_path, &untracked_content).expect("unrelated untracked content");

            Self {
                staged_path,
                staged_content,
                unstaged_path,
                unstaged_content,
                untracked_path,
                untracked_content,
                indexed_staged_content: git_output(root, ["show", ":unrelated-staged.txt"])
                    .expect("unrelated staged blob"),
                cached_diff: git_output(root, ["diff", "--cached", "--binary", "--no-ext-diff"])
                    .expect("unrelated cached diff"),
                status: git_output(root, ["status", "--porcelain=v1", "--untracked-files=all"])
                    .expect("unrelated status"),
            }
        }

        fn assert_unchanged(&self, root: &Path) {
            assert_eq!(
                fs::read(&self.staged_path).expect("preserved staged file"),
                self.staged_content
            );
            assert_eq!(
                fs::read(&self.unstaged_path).expect("preserved unstaged file"),
                self.unstaged_content
            );
            assert_eq!(
                fs::read(&self.untracked_path).expect("preserved untracked file"),
                self.untracked_content
            );
            assert_eq!(
                git_output(root, ["show", ":unrelated-staged.txt"]).expect("preserved staged blob"),
                self.indexed_staged_content
            );
            assert_eq!(
                git_output(root, ["diff", "--cached", "--binary", "--no-ext-diff"],)
                    .expect("preserved cached diff"),
                self.cached_diff
            );
            assert_eq!(
                git_output(root, ["status", "--porcelain=v1", "--untracked-files=all"],)
                    .expect("preserved status"),
                self.status
            );
        }
    }

    struct TemporaryWorkspace {
        root: PathBuf,
        workspace: PathBuf,
        active_revision: String,
        next_revision: String,
    }

    impl TemporaryWorkspace {
        fn create() -> Self {
            let root = std::env::temp_dir().join(format!(
                "boxdd-upstream-repository-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            let upstream = root.join("upstream");
            let workspace = root.join("workspace");
            fs::create_dir_all(upstream.join("src")).expect("upstream src");
            fs::create_dir_all(upstream.join("include/box2d")).expect("upstream include");
            run_git(&upstream, &["init"]);
            configure_git(&upstream);
            fs::write(upstream.join("src/a.c"), "int a(void) { return 1; }\n")
                .expect("source fixture");
            fs::write(upstream.join("src/a.h"), "int a(void);\n").expect("private header");
            fs::write(
                upstream.join("src/recording_ops.inl"),
                "B2_REC_OP( 0x01, DestroyWorld, RET_NONE, ARG( WORLDID, world ) )\n",
            )
            .expect("recording fixture");
            fs::write(
                upstream.join("src/recording.c"),
                "int recording_writer(void) { return 1; }\n",
            )
            .expect("recording writer fixture");
            fs::write(
                upstream.join("src/recording_replay.c"),
                "int recording_reader(void) { return 1; }\n",
            )
            .expect("recording reader fixture");
            fs::write(
                upstream.join("src/recording.h"),
                "int recording_writer(void);\n",
            )
            .expect("recording writer header fixture");
            fs::write(
                upstream.join("src/recording_replay.h"),
                "int recording_reader(void);\n",
            )
            .expect("recording reader header fixture");
            fs::write(
                upstream.join("src/world_snapshot.c"),
                "int snapshot_wire(void) { return 1; }\n",
            )
            .expect("snapshot source fixture");
            fs::write(
                upstream.join("src/world_snapshot.h"),
                "int snapshot_wire(void);\n",
            )
            .expect("snapshot header fixture");
            fs::write(upstream.join("include/box2d/box2d.h"), "int a(void);\n")
                .expect("public header");
            run_git(&upstream, &["add", "."]);
            run_git(&upstream, &["commit", "-m", "active"]);
            let active_revision = git_output(&upstream, ["rev-parse", "HEAD"])
                .expect("active revision")
                .trim()
                .to_owned();
            fs::write(upstream.join("src/b.c"), "int b(void) { return 2; }\n")
                .expect("target source fixture");
            run_git(&upstream, &["add", "."]);
            run_git(&upstream, &["commit", "-m", "target"]);
            let next_revision = git_output(&upstream, ["rev-parse", "HEAD"])
                .expect("next revision")
                .trim()
                .to_owned();

            fs::create_dir_all(&workspace).expect("workspace fixture");
            fs::create_dir_all(workspace.join("boxdd")).expect("boxdd fixture directory");
            fs::write(
                workspace.join("boxdd/Cargo.toml"),
                "[package]\nname = \"boxdd-fixture\"\nversion = \"0.0.0\"\n\n[features]\ndefault = []\n",
            )
            .expect("boxdd fixture manifest");
            fs::write(
                workspace.join("unrelated-tracked.txt"),
                "tracked baseline\n",
            )
            .expect("unrelated tracked fixture");
            run_git(&workspace, &["init"]);
            configure_git(&workspace);
            let upstream_arg = upstream.to_string_lossy().into_owned();
            command_success(
                git_command()
                    .expect("qualified Git")
                    .current_dir(&workspace)
                    .args(["-c", "protocol.file.allow=always", "submodule", "add"])
                    .arg(&upstream_arg)
                    .arg(BOX2D_GITLINK),
                "add fixture submodule",
            )
            .expect("fixture submodule");
            let submodule = workspace.join(BOX2D_GITLINK);
            checkout_detached(&submodule, &active_revision).expect("active submodule checkout");

            let mut manifest = UpstreamManifest {
                schema_version: UPSTREAM_MANIFEST_SCHEMA,
                repository: "https://github.com/erincatto/box2d.git".to_owned(),
                active_revision: active_revision.clone(),
                next_revision: Some(next_revision.clone()),
                recording_revision: next_revision.clone(),
                artifact_digests_initialized: true,
                binding_routes: binding_routes(),
                next_binding_routes: Vec::new(),
                recording_inputs: reviewed_recording_inputs(&upstream, &next_revision),
                artifacts: artifacts(),
                next_artifacts: Vec::new(),
                source_inventory: source_inventory(&upstream, &active_revision)
                    .expect("active inventory"),
                next_inventory: Some(
                    source_inventory(&upstream, &next_revision).expect("target inventory"),
                ),
            };
            for artifact in &manifest.artifacts {
                let path = workspace.join(&artifact.path);
                fs::create_dir_all(path.parent().expect("artifact parent"))
                    .expect("artifact directory");
                let content = match artifact.kind {
                    ArtifactKind::Bindings => format!(
                        "{}\n// generated fixture\n",
                        binding_provenance(
                            artifact,
                            &active_revision,
                            RustTarget::X86_64UnknownLinuxGnu,
                            None,
                            None,
                        )
                        .expect("fixture binding provenance")
                    ),
                    ArtifactKind::ApiContract => {
                        format!("upstream_sha = \"{active_revision}\"\n")
                    }
                    ArtifactKind::RecordingWire => {
                        format!("upstream_sha = \"{next_revision}\"\n")
                    }
                    ArtifactKind::ApiCoverageReport => {
                        format!("Pinned active upstream: `{active_revision}`.\n")
                    }
                    ArtifactKind::ProviderIdentity => {
                        format!("upstream_sha = \"{active_revision}\"\n")
                    }
                    ArtifactKind::AbiMetadata => {
                        unreachable!("fixture does not declare optional ABI metadata")
                    }
                };
                fs::write(path, content).expect("artifact fixture");
            }
            for artifact in &mut manifest.artifacts {
                artifact.content_blake3 =
                    file_blake3(&workspace.join(&artifact.path)).expect("artifact digest");
            }
            fs::write(
                workspace.join("boxdd-sys/upstream.toml"),
                render_toml(&manifest).expect("manifest TOML"),
            )
            .expect("manifest fixture");
            run_git(&workspace, &["add", "."]);
            run_git(&workspace, &["commit", "-m", "workspace"]);
            assert_eq!(
                indexed_gitlink(&workspace).expect("fixture gitlink"),
                active_revision
            );

            Self {
                root,
                workspace,
                active_revision,
                next_revision,
            }
        }

        fn paths(&self) -> WorkspacePaths {
            WorkspacePaths::new(&self.workspace)
        }

        fn manifest(&self) -> UpstreamManifest {
            UpstreamManifest::load(&self.paths()).expect("fixture manifest")
        }

        fn commit_target_candidates(&self, manifest: &UpstreamManifest) -> UpstreamManifest {
            let mut updated = manifest.clone();
            let api = updated
                .artifacts
                .iter_mut()
                .find(|artifact| artifact.kind == ArtifactKind::ApiContract)
                .expect("API artifact");
            let candidate = "boxdd/tests/fixtures/api_contract.next.toml".to_owned();
            api.candidate_path = Some(candidate.clone());
            fs::write(
                self.workspace.join(&candidate),
                format!("upstream_sha = \"{}\"\n", self.next_revision),
            )
            .expect("target API identity");
            api.candidate_blake3 = Some(
                file_blake3(&self.workspace.join(&candidate)).expect("candidate artifact digest"),
            );
            fs::write(
                self.workspace.join("boxdd-sys/upstream.toml"),
                render_toml(&updated).expect("candidate manifest"),
            )
            .expect("candidate manifest fixture");
            run_git(&self.workspace, &["add", "."]);
            run_git(&self.workspace, &["commit", "-m", "target candidates"]);
            updated
        }
    }

    impl Drop for TemporaryWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn configure_git(repository: &Path) {
        run_git(repository, &["config", "user.name", "Boxdd Test"]);
        run_git(
            repository,
            &["config", "user.email", "boxdd@example.invalid"],
        );
    }

    fn run_git(repository: &Path, args: &[&str]) {
        command_success(
            git_command()
                .expect("qualified Git")
                .current_dir(repository)
                .args(args),
            &format!("git {}", args.join(" ")),
        )
        .expect("fixture git command");
    }

    fn inventory() -> SourceInventory {
        SourceInventory {
            tree: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            c_sources: vec!["src/a.c".to_owned(), "src/b.c".to_owned()],
            private_headers: vec!["src/a.h".to_owned()],
            inline_files: vec!["src/ops.inl".to_owned()],
            public_headers: vec!["include/box2d/box2d.h".to_owned()],
        }
    }

    fn artifacts() -> Vec<GeneratedArtifact> {
        vec![
            GeneratedArtifact {
                name: "bindings-single".to_owned(),
                kind: ArtifactKind::Bindings,
                path: "boxdd-sys/src/bindings_pregenerated.rs".to_owned(),
                precision: Some(Precision::Single),
                target: ArtifactTarget::Universal,
                provider: ArtifactProvider::Universal,
                producer: ArtifactProducer::Bindgen,
                content_blake3: "0".repeat(64),
                candidate_path: None,
                candidate_blake3: None,
            },
            GeneratedArtifact {
                name: "api-contract".to_owned(),
                kind: ArtifactKind::ApiContract,
                path: "boxdd/tests/fixtures/api_contract.toml".to_owned(),
                precision: None,
                target: ArtifactTarget::Universal,
                provider: ArtifactProvider::Universal,
                producer: ArtifactProducer::Reviewed,
                content_blake3: "0".repeat(64),
                candidate_path: None,
                candidate_blake3: None,
            },
            GeneratedArtifact {
                name: "recording-wire".to_owned(),
                kind: ArtifactKind::RecordingWire,
                path: "xtask/tests/fixtures/recording_wire.toml".to_owned(),
                precision: None,
                target: ArtifactTarget::Universal,
                provider: ArtifactProvider::Universal,
                producer: ArtifactProducer::ApiCoverage,
                content_blake3: "0".repeat(64),
                candidate_path: None,
                candidate_blake3: None,
            },
            GeneratedArtifact {
                name: "api-coverage-report".to_owned(),
                kind: ArtifactKind::ApiCoverageReport,
                path: "docs/api-coverage.md".to_owned(),
                precision: None,
                target: ArtifactTarget::Universal,
                provider: ArtifactProvider::Universal,
                producer: ArtifactProducer::ApiCoverage,
                content_blake3: "0".repeat(64),
                candidate_path: None,
                candidate_blake3: None,
            },
        ]
    }

    fn binding_routes() -> Vec<BindingRoute> {
        vec![BindingRoute {
            mode: Precision::Single,
            provider: ArtifactProvider::Source,
            artifact: "bindings-single".to_owned(),
            rust_target: RustTarget::X86_64UnknownLinuxGnu,
            rust_features: Vec::new(),
        }]
    }

    fn pending_double_bindings() -> GeneratedArtifact {
        GeneratedArtifact {
            name: "bindings-double".to_owned(),
            kind: ArtifactKind::Bindings,
            path: "boxdd-sys/src/bindings_double.rs".to_owned(),
            precision: Some(Precision::Double),
            target: ArtifactTarget::Universal,
            provider: ArtifactProvider::Universal,
            producer: ArtifactProducer::Bindgen,
            content_blake3: UNINITIALIZED_BLAKE3.to_owned(),
            candidate_path: None,
            candidate_blake3: None,
        }
    }

    fn dual_precision_routes() -> Vec<BindingRoute> {
        vec![
            BindingRoute {
                mode: Precision::Single,
                provider: ArtifactProvider::Source,
                artifact: "bindings-single".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            },
            BindingRoute {
                mode: Precision::Double,
                provider: ArtifactProvider::Source,
                artifact: "bindings-double".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: vec!["double-precision".to_owned()],
            },
        ]
    }

    fn reviewed_recording_inputs(repository: &Path, revision: &str) -> Vec<RecordingInputIdentity> {
        crate::recording_wire::REVIEWED_RECORDING_INPUT_PATHS
            .iter()
            .copied()
            .map(|path| RecordingInputIdentity {
                path: path.to_owned(),
                git_blob: git_blob_identity(repository, revision, path)
                    .expect("reviewed recording input blob"),
                blake3: git_blob_blake3(repository, revision, path)
                    .expect("reviewed recording input digest"),
            })
            .collect()
    }

    fn placeholder_recording_inputs() -> Vec<RecordingInputIdentity> {
        crate::recording_wire::REVIEWED_RECORDING_INPUT_PATHS
            .iter()
            .copied()
            .map(|path| RecordingInputIdentity {
                path: path.to_owned(),
                git_blob: "0".repeat(40),
                blake3: "0".repeat(64),
            })
            .collect()
    }

    fn manifest() -> UpstreamManifest {
        let mut artifacts = artifacts();
        for artifact in &mut artifacts {
            artifact.content_blake3 = "1".repeat(64);
        }
        UpstreamManifest {
            schema_version: UPSTREAM_MANIFEST_SCHEMA,
            repository: "https://github.com/erincatto/box2d.git".to_owned(),
            active_revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            next_revision: Some("89abcdef0123456789abcdef0123456789abcdef".to_owned()),
            recording_revision: "89abcdef0123456789abcdef0123456789abcdef".to_owned(),
            artifact_digests_initialized: true,
            binding_routes: binding_routes(),
            next_binding_routes: Vec::new(),
            recording_inputs: placeholder_recording_inputs(),
            artifacts,
            next_artifacts: Vec::new(),
            source_inventory: inventory(),
            next_inventory: Some(inventory()),
        }
    }

    fn route_refresh_transaction_fixture(
        fixture: &TemporaryWorkspace,
    ) -> (
        WorkspacePaths,
        UpstreamManifest,
        RouteRefreshStaging,
        Vec<u8>,
    ) {
        let paths = fixture.paths();
        fs::write(
            paths.root().join("boxdd/Cargo.toml"),
            "[package]\nname = \"boxdd-fixture\"\nversion = \"0.0.0\"\n\n[features]\ndefault = []\ndouble-precision = []\n",
        )
        .expect("route refresh feature catalog");
        let mut original = fixture.manifest();
        original.next_revision = None;
        original.recording_revision = fixture.active_revision.clone();
        original.next_binding_routes.clear();
        original.next_artifacts.clear();
        original.next_inventory = None;
        original.recording_inputs =
            reviewed_recording_inputs(&fixture.root.join("upstream"), &fixture.active_revision);
        let manifest_content = render_toml(&original)
            .expect("route refresh source manifest")
            .into_bytes();
        fs::write(paths.upstream_manifest(), &manifest_content)
            .expect("route refresh source manifest");

        let mut target =
            canonical_route_refresh_manifest(&paths, &original).expect("canonical route manifest");
        let mut files = target
            .artifacts
            .iter_mut()
            .map(|artifact| {
                let content = format!("route refresh output for {}\n", artifact.name).into_bytes();
                artifact.content_blake3 = blake3_bytes(&content);
                StagedFile {
                    relative_path: artifact.path.clone(),
                    content,
                }
            })
            .collect::<Vec<_>>();
        files.push(StagedFile {
            relative_path: super::super::api_coverage::RUNTIME_RECORDING_WIRE_PATH.to_owned(),
            content: b"route refresh runtime recording contract\n".to_vec(),
        });
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        target.artifact_digests_initialized = true;
        let staged = RouteRefreshStaging {
            manifest: target,
            files,
            removals: Vec::new(),
        };
        for file in &staged.files {
            let path = paths.root().join(&file.relative_path);
            fs::create_dir_all(path.parent().expect("route output parent"))
                .expect("route output directory");
        }
        validate_route_refresh_staging(&original, &staged).expect("valid route refresh staging");
        (paths, original, staged, manifest_content)
    }

    fn candidate_contract(revision: &str, modes: &[&str]) -> Vec<u8> {
        let modes_toml = modes
            .iter()
            .map(|mode| format!("\"{mode}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let link_symbols = modes
            .iter()
            .map(|mode| format!("{mode} = \"b2Fixture\""))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "schema_version = 4\n\
             upstream_sha = \"{revision}\"\n\
             classification_changes = []\n\
             evidence = []\n\
             \n\
             [migration_baseline]\n\
             total = 1\n\
             safe = 0\n\
             raw = 1\n\
             omitted = 0\n\
             deferred = 0\n\
             \n\
             [[functions]]\n\
             logical_name = \"b2Fixture\"\n\
             signature = \"void b2Fixture ( void )\"\n\
             fingerprint = \"fnv1a64:0000000000000000\"\n\
             classification = \"raw\"\n\
             area = \"Fixture\"\n\
             rust_paths = [\"boxdd_sys::ffi::b2Fixture\"]\n\
             rationale = \"Fixture raw route.\"\n\
             modes = [{modes_toml}]\n\
             providers = [\"source\"]\n\
             availability = [\"always\"]\n\
             evidence = []\n\
             \n\
             [functions.link_symbols]\n\
             {link_symbols}\n\
             \n\
             [abi]\n\
             policies = []\n\
             structs = []\n\
             callbacks = []\n"
        )
        .into_bytes()
    }

    fn register_candidate_bytes(manifest: &mut UpstreamManifest, bytes: &[u8]) {
        let artifact = manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == ArtifactKind::ApiContract)
            .expect("API artifact");
        artifact.candidate_path = Some("boxdd/tests/fixtures/api_contract.next.toml".to_owned());
        artifact.candidate_blake3 = Some(blake3::hash(bytes).to_hex().to_string());
    }

    #[test]
    fn manifest_requires_exact_shas_safe_named_artifacts_and_inventory() {
        validate_manifest(&manifest()).expect("valid manifest");
        let mut invalid = manifest();
        invalid.next_revision = Some("main".to_owned());
        invalid.artifacts[0].path = "../bindings.rs".to_owned();
        invalid.source_inventory.c_sources.reverse();
        let error = validate_manifest(&invalid).expect_err("invalid manifest must fail");
        assert!(error.to_string().contains("40-character"));
        assert!(error.to_string().contains("canonical relative path"));
        assert!(error.to_string().contains("sorted and unique"));
        assert_eq!(
            reviewed_candidate_path(
                manifest()
                    .artifact(ArtifactKind::ApiContract)
                    .expect("API artifact")
            )
            .expect("derived candidate path"),
            "boxdd/tests/fixtures/api_contract.next.toml"
        );
    }

    #[test]
    fn next_binding_topology_is_promoted_as_one_uninitialized_generation_set() {
        let mut pending = manifest();
        pending.next_binding_routes = dual_precision_routes();
        pending.next_artifacts = vec![pending_double_bindings()];
        pending
            .next_inventory
            .as_mut()
            .expect("next inventory")
            .tree = "b".repeat(40);
        validate_manifest(&pending).expect("valid pending topology");

        let target = pending.next_revision.clone().expect("next revision");
        let promoted = pending
            .promoted_for_generation(&target)
            .expect("promoted topology");
        assert_eq!(promoted.active_revision, target);
        assert_eq!(promoted.recording_revision, target);
        assert_eq!(promoted.next_revision, None);
        assert_eq!(promoted.binding_routes, dual_precision_routes());
        assert!(promoted.next_binding_routes.is_empty());
        assert!(promoted.next_artifacts.is_empty());
        assert!(promoted.next_inventory.is_none());
        assert_eq!(promoted.source_inventory.tree, "b".repeat(40));
        assert_eq!(promoted.binding_artifacts().count(), 2);
        assert!(!promoted.artifact_digests_initialized);
        assert!(promoted.artifacts.iter().all(|artifact| {
            artifact.content_blake3 == UNINITIALIZED_BLAKE3
                && artifact.candidate_path.is_none()
                && artifact.candidate_blake3.is_none()
        }));

        let mut without_revision = pending.clone();
        without_revision.next_revision = None;
        assert!(
            validate_manifest(&without_revision)
                .expect_err("pending topology without revision must fail")
                .to_string()
                .contains("requires next_revision")
        );

        let mut initialized = pending;
        initialized.next_artifacts[0].content_blake3 = "1".repeat(64);
        assert!(
            validate_manifest(&initialized)
                .expect_err("pending artifact with digest must fail")
                .to_string()
                .contains("uninitialized digest")
        );
    }

    #[test]
    fn next_candidate_gate_invalidates_a_candidate_when_target_topology_changes() {
        let mut pending = manifest();
        let target = pending.next_revision.clone().expect("next revision");
        let single_candidate = candidate_contract(&target, &["single"]);
        register_candidate_bytes(&mut pending, &single_candidate);
        let summary = validate_next_candidate_bytes(&pending, &single_candidate, &single_candidate)
            .expect("single candidate matches the original topology");
        assert_eq!(summary.routes, ["single/source"]);

        pending.next_binding_routes = dual_precision_routes();
        pending.next_artifacts = vec![pending_double_bindings()];
        let dual_candidate = candidate_contract(&target, &["double", "single"]);
        let error = validate_next_candidate_bytes(&pending, &single_candidate, &dual_candidate)
            .expect_err("single-only candidate must not survive a dual-route topology change");
        assert!(error.to_string().contains("covers routes"), "{error}");
        assert!(error.to_string().contains("double"), "{error}");
    }

    #[test]
    fn next_candidate_gate_requires_a_target_and_registered_candidate() {
        let missing_candidate = manifest();
        let error = next_candidate_registration(&missing_candidate)
            .expect_err("missing candidate must fail closed");
        assert!(error.to_string().contains("no target candidate_path"));

        let mut missing_target = manifest();
        let target = missing_target.next_revision.clone().expect("next revision");
        let candidate = candidate_contract(&target, &["single"]);
        register_candidate_bytes(&mut missing_target, &candidate);
        missing_target.next_revision = None;
        missing_target.recording_revision = missing_target.active_revision.clone();
        missing_target.next_inventory = None;
        let error = next_candidate_registration(&missing_target)
            .expect_err("missing target must fail closed");
        assert!(error.to_string().contains("no next_revision to check"));
    }

    #[test]
    fn next_candidate_gate_requires_exact_bytes_and_registered_digest() {
        let mut pending = manifest();
        let target = pending.next_revision.clone().expect("next revision");
        let candidate = candidate_contract(&target, &["single"]);
        register_candidate_bytes(&mut pending, &candidate);

        let mut regenerated = candidate.clone();
        regenerated.extend_from_slice(b"# generator drift\n");
        let error = validate_next_candidate_bytes(&pending, &candidate, &regenerated)
            .expect_err("byte drift must fail closed");
        assert!(error.to_string().contains("candidate is stale"), "{error}");

        let api = pending
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == ArtifactKind::ApiContract)
            .expect("API artifact");
        api.candidate_blake3 = Some("f".repeat(64));
        let error = validate_next_candidate_bytes(&pending, &candidate, &candidate)
            .expect_err("registered digest drift must fail closed");
        assert!(error.to_string().contains("digest drifted"), "{error}");
    }

    #[test]
    fn promotion_candidate_gate_rejects_legal_byte_drift_without_mutation() {
        let root = TempDir::new().expect("candidate fixture root");
        let mut pending = manifest();
        let target = pending.next_revision.clone().expect("next revision");
        let rendered = candidate_contract(&target, &["single"]);
        let mut registered = rendered.clone();
        registered.extend_from_slice(b"# reviewed formatting drift\n");
        register_candidate_bytes(&mut pending, &registered);
        let registration = next_candidate_registration(&pending).expect("candidate registration");
        let path = root.path().join(registration.path);
        fs::create_dir_all(path.parent().expect("candidate parent")).expect("candidate directory");
        fs::write(&path, &registered).expect("registered candidate");

        let error = validate_promotion_candidate(root.path(), &pending, &rendered)
            .expect_err("promotion must reject structurally legal but non-reproducible bytes");

        assert!(error.to_string().contains("candidate is stale"), "{error}");
        assert_eq!(
            fs::read(&path).expect("candidate after rejection"),
            registered
        );
    }

    #[test]
    fn manifest_rejects_lexical_path_aliases_before_transaction_keying() {
        for alias in [
            "boxdd-sys/src//bindings_pregenerated.rs",
            "boxdd-sys/src/./bindings_pregenerated.rs",
            "boxdd-sys/src/bindings_pregenerated.rs/",
            "boxdd-sys\\src\\bindings_pregenerated.rs",
        ] {
            let mut invalid = manifest();
            invalid.artifacts[0].path = alias.to_owned();
            let error = validate_manifest(&invalid).expect_err("path alias must fail closed");
            assert!(
                error.to_string().contains("canonical relative path"),
                "{alias}: {error}"
            );
        }
    }

    #[test]
    fn inventory_rejects_duplicate_paths_within_and_across_groups() {
        let mut within = inventory();
        within.c_sources.insert(1, within.c_sources[0].clone());
        let mut errors = Vec::new();
        validate_inventory_shape(&within, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("sorted and unique"))
        );

        let mut across = inventory();
        across.private_headers = vec![across.c_sources[0].clone()];
        let mut errors = Vec::new();
        validate_inventory_shape(&across, &mut errors);
        assert!(errors.iter().any(|error| error.contains("appears in both")));
    }

    #[test]
    fn artifact_coordinates_support_precision_target_and_provider_variants() {
        let mut variants = artifacts();
        let mut double = variants[0].clone();
        double.name = "bindings-double-native-source".to_owned();
        double.path = "boxdd-sys/src/bindings_double.rs".to_owned();
        double.precision = Some(Precision::Double);
        double.target = ArtifactTarget::Native;
        double.provider = ArtifactProvider::Source;
        variants.push(double.clone());
        let mut errors = Vec::new();
        validate_artifacts(&variants, &mut errors);
        assert!(errors.is_empty(), "valid coordinate variants: {errors:?}");

        double.name = "bindings-double-duplicate".to_owned();
        double.path = "boxdd-sys/src/bindings_double_duplicate.rs".to_owned();
        variants.push(double);
        let mut errors = Vec::new();
        validate_artifacts(&variants, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate artifact coordinate"))
        );

        let manifest = manifest();
        assert_eq!(
            manifest
                .binding_artifact(
                    Precision::Single,
                    ArtifactTarget::Universal,
                    ArtifactProvider::Universal,
                )
                .expect("single universal bindings")
                .name,
            "bindings-single"
        );
        assert!(manifest.artifact(ArtifactKind::Bindings).is_err());
    }

    #[test]
    fn provider_identity_topology_requires_the_exact_single_double_pair() {
        assert!(require_provider_identity_topology(&manifest()).is_err());
        let mut canonical = artifacts();
        canonical.extend([
            GeneratedArtifact {
                name: "wasm-provider-identity-single".to_owned(),
                kind: ArtifactKind::ProviderIdentity,
                path: "boxdd-sys/abi/wasm32-unknown-unknown-single.toml".to_owned(),
                precision: Some(Precision::Single),
                target: ArtifactTarget::Wasm32UnknownUnknown,
                provider: ArtifactProvider::WasmRuntime,
                producer: ArtifactProducer::ProviderAttestation,
                content_blake3: "1".repeat(64),
                candidate_path: None,
                candidate_blake3: None,
            },
            GeneratedArtifact {
                name: "wasm-provider-identity-double".to_owned(),
                kind: ArtifactKind::ProviderIdentity,
                path: "boxdd-sys/abi/wasm32-unknown-unknown-double.toml".to_owned(),
                precision: Some(Precision::Double),
                target: ArtifactTarget::Wasm32UnknownUnknown,
                provider: ArtifactProvider::WasmRuntime,
                producer: ArtifactProducer::ProviderAttestation,
                content_blake3: "2".repeat(64),
                candidate_path: None,
                candidate_blake3: None,
            },
        ]);
        let mut errors = Vec::new();
        validate_provider_identity_topology(&canonical, &mut errors);
        assert!(errors.is_empty(), "canonical topology: {errors:?}");

        let mut missing = canonical.clone();
        missing.retain(|artifact| artifact.name != "wasm-provider-identity-double");
        let mut errors = Vec::new();
        validate_provider_identity_topology(&missing, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("exact single/double"))
        );

        let mut wrong_provider = canonical;
        wrong_provider
            .iter_mut()
            .find(|artifact| artifact.name == "wasm-provider-identity-single")
            .expect("single provider identity")
            .provider = ArtifactProvider::WasmCompileOnly;
        let mut errors = Vec::new();
        validate_provider_identity_topology(&wrong_provider, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("exact single/double"))
        );
    }

    #[test]
    fn abi_metadata_topology_exactly_covers_source_routes() {
        let routes = dual_precision_routes();
        let metadata = |precision: Precision, name: &str, path: &str| GeneratedArtifact {
            name: name.to_owned(),
            kind: ArtifactKind::AbiMetadata,
            path: path.to_owned(),
            precision: Some(precision),
            target: ArtifactTarget::Native,
            provider: ArtifactProvider::Source,
            producer: ArtifactProducer::AbiProbe,
            content_blake3: "1".repeat(64),
            candidate_path: None,
            candidate_blake3: None,
        };
        let artifacts = vec![
            metadata(
                Precision::Single,
                "abi-source-single",
                "boxdd-sys/abi/metadata/source-single.toml",
            ),
            metadata(
                Precision::Double,
                "abi-source-double",
                "boxdd-sys/abi/metadata/source-double.toml",
            ),
        ];
        let mut errors = Vec::new();
        validate_abi_metadata_topology(&routes, &artifacts, true, &mut errors);
        assert!(errors.is_empty(), "valid ABI metadata topology: {errors:?}");

        let mut missing = Vec::new();
        validate_abi_metadata_topology(&routes, &artifacts[..1], true, &mut missing);
        assert!(
            missing
                .iter()
                .any(|error| error.contains("do not match source binding routes"))
        );

        let mut wrong_provider = artifacts;
        wrong_provider[0].provider = ArtifactProvider::SystemStatic;
        let mut errors = Vec::new();
        validate_abi_metadata_topology(&routes, &wrong_provider, true, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must identify the vendored source provider"))
        );
    }

    #[test]
    fn generation_cargo_is_qualified_scoped_and_rejects_ambient_configs() {
        let root = TempDir::new().expect("generation workspace parent");
        let workspace = root.path().join("workspace");
        fs::create_dir(&workspace).expect("generation workspace");
        let workspace = fs::canonicalize(&workspace).expect("canonical generation workspace");
        let output_root = controlled_child_directory(
            &workspace,
            Path::new("isolated-output"),
            "test generation output root",
        )
        .expect("controlled generation output root");
        let cargo_home = controlled_child_directory(
            &output_root,
            Path::new("cargo-home"),
            "test generation Cargo home",
        )
        .expect("controlled generation Cargo home");
        let cargo = qualify_generation_cargo(&workspace, &workspace, &cargo_home, &output_root)
            .expect("isolated Cargo qualification");
        let command = cargo
            .command_at_working_root(&output_root.join("cargo-target"))
            .expect("qualified generation command");
        assert!(Path::new(command.get_program()).is_absolute());
        let configured_home = command
            .get_envs()
            .find(|(name, _)| *name == std::ffi::OsStr::new("CARGO_HOME"))
            .and_then(|(_, value)| value)
            .expect("explicit CARGO_HOME");
        assert_eq!(configured_home, cargo_home.as_os_str());
        let rustc = command
            .get_envs()
            .find(|(name, _)| *name == std::ffi::OsStr::new("RUSTC"))
            .and_then(|(_, value)| value)
            .expect("qualified RUSTC");
        assert!(Path::new(rustc).is_absolute());

        let ambient = root.path().join(".cargo");
        fs::create_dir(&ambient).expect("ambient Cargo config directory");
        fs::write(ambient.join("config.toml"), "[build]\nrustflags = []\n")
            .expect("ambient Cargo config");
        let error = qualify_generation_cargo(&workspace, &workspace, &cargo_home, &output_root)
            .expect_err("workspace ancestor Cargo config must fail closed");
        assert!(
            error
                .to_string()
                .contains("workspace ancestor Cargo config")
        );

        fs::remove_file(ambient.join("config.toml")).expect("remove fixture ancestor config");
        let workspace_config = workspace.join(".cargo");
        fs::create_dir(&workspace_config).expect("workspace Cargo config directory");
        fs::write(
            workspace_config.join("config.toml"),
            "[build]\nrustflags = []\n",
        )
        .expect("workspace Cargo config");
        let error = qualify_generation_cargo(&workspace, &workspace, &cargo_home, &output_root)
            .expect_err("workspace Cargo config must fail closed");
        assert!(
            error
                .to_string()
                .contains("workspace ancestor Cargo config")
        );

        fs::remove_dir_all(&workspace_config).expect("remove fixture workspace config");
        fs::write(cargo_home.join("config"), "[build]\nrustflags = []\n")
            .expect("isolated Cargo home config");
        let error = qualify_generation_cargo(&workspace, &workspace, &cargo_home, &output_root)
            .expect_err("isolated Cargo home config must fail closed");
        assert!(error.to_string().contains("Cargo home Cargo config"));
    }

    #[test]
    fn repository_abi_metadata_is_byte_reproducible() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let paths = WorkspacePaths::new(root);
        let manifest = UpstreamManifest::load(&paths).expect("repository manifest");
        let artifacts = manifest.abi_metadata_artifacts().collect::<Vec<_>>();
        assert_eq!(artifacts.len(), 2);
        for artifact in artifacts {
            let path = root.join(&artifact.path);
            let observed = fs::read(&path).expect("checked-in ABI metadata");
            let expected = render_abi_probe_metadata(root, &manifest, artifact)
                .expect("regenerated ABI metadata");
            assert_eq!(observed, expected, "{}", artifact.name);
            assert_eq!(blake3_bytes(&observed), artifact.content_blake3);
        }
    }

    #[test]
    fn binding_routes_are_exact_unique_and_coordinate_compatible() {
        let artifacts = artifacts();
        let routes = binding_routes();
        let mut errors = Vec::new();
        validate_binding_routes(&routes, &artifacts, &mut errors);
        assert!(errors.is_empty(), "valid route: {errors:?}");

        let mut duplicate = routes.clone();
        duplicate.push(routes[0].clone());
        let mut errors = Vec::new();
        validate_binding_routes(&duplicate, &artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate binding route"))
        );

        let shared_native_routes = vec![
            BindingRoute {
                mode: Precision::Single,
                provider: ArtifactProvider::Source,
                artifact: "bindings-single".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            },
            BindingRoute {
                mode: Precision::Single,
                provider: ArtifactProvider::SystemStatic,
                artifact: "bindings-single".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            },
        ];
        let mut errors = Vec::new();
        validate_binding_routes(&shared_native_routes, &artifacts, &mut errors);
        assert!(errors.is_empty(), "compatible route reuse: {errors:?}");

        let mut wrong_precision = routes.clone();
        wrong_precision[0].mode = Precision::Double;
        let mut errors = Vec::new();
        validate_binding_routes(&wrong_precision, &artifacts, &mut errors);
        assert!(errors.iter().any(|error| error.contains("precision")));

        let mut wrong_target_artifacts = artifacts.clone();
        wrong_target_artifacts[0].target = ArtifactTarget::Wasm32UnknownUnknown;
        let mut errors = Vec::new();
        validate_binding_routes(&routes, &wrong_target_artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("incompatible target"))
        );

        let mut wrong_rust_target = routes.clone();
        wrong_rust_target[0].rust_target = RustTarget::Wasm32UnknownUnknown;
        let mut errors = Vec::new();
        validate_binding_routes(&wrong_rust_target, &artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("incompatible target"))
        );

        let mut unused_artifacts = artifacts;
        let mut unused = unused_artifacts[0].clone();
        unused.name = "bindings-double".to_owned();
        unused.path = "boxdd-sys/src/bindings_double.rs".to_owned();
        unused.precision = Some(Precision::Double);
        unused_artifacts.push(unused);
        let mut errors = Vec::new();
        validate_binding_routes(&routes, &unused_artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("not used by an executable route"))
        );

        let mut duplicate_features = routes.clone();
        duplicate_features[0].rust_features = vec!["serde".to_owned(), "serde".to_owned()];
        let mut errors = Vec::new();
        validate_binding_routes(&duplicate_features, &unused_artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("rust_features must be sorted and unique"))
        );

        let mut single_with_double = routes.clone();
        single_with_double[0].rust_features = vec!["double-precision".to_owned()];
        let mut errors = Vec::new();
        validate_binding_routes(&single_with_double, &unused_artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must not enable `double-precision`"))
        );

        let mut double_without_feature = routes;
        double_without_feature[0].mode = Precision::Double;
        let mut errors = Vec::new();
        validate_binding_routes(&double_without_feature, &unused_artifacts, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must enable `double-precision`"))
        );
    }

    #[test]
    fn binding_generation_target_is_manifest_derived_and_unique_per_artifact() {
        let mut manifest = manifest();
        let artifact = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact")
            .clone();
        assert_eq!(
            binding_generation_target(&manifest, &artifact).expect("route target"),
            RustTarget::X86_64UnknownLinuxGnu
        );
        assert_eq!(
            binding_generation_cargo_args(RustTarget::X86_64UnknownLinuxGnu, "bindgen"),
            [
                "build",
                "--locked",
                "--target",
                "x86_64-unknown-linux-gnu",
                "-p",
                "boxdd-sys",
                "--features",
                "bindgen",
            ]
            .map(str::to_owned)
        );

        let mut shared = manifest.binding_routes[0].clone();
        shared.provider = ArtifactProvider::SystemStatic;
        manifest.binding_routes.push(shared);
        assert_eq!(
            binding_generation_target(&manifest, &artifact).expect("shared route target"),
            RustTarget::X86_64UnknownLinuxGnu
        );

        manifest.binding_routes[1].rust_target = RustTarget::Wasm32UnknownUnknown;
        let error = binding_generation_target(&manifest, &artifact)
            .expect_err("one artifact cannot encode multiple target layouts");
        assert!(error.to_string().contains("multiple Rust targets"));
    }

    #[test]
    fn binding_routes_reject_unknown_boxdd_features() {
        let fixture = TemporaryWorkspace::create();
        let mut routes = binding_routes();
        routes[0].rust_features = vec!["unknown-feature".to_owned()];

        let error = validate_binding_route_feature_catalog(&fixture.paths(), &routes)
            .expect_err("unknown feature must fail closed");

        assert!(error.to_string().contains("unknown boxdd Rust feature"));
    }

    #[test]
    fn binding_route_features_expand_local_aliases_but_not_dependency_features() {
        let fixture = TemporaryWorkspace::create();
        fs::write(
            fixture.paths().root().join("boxdd/Cargo.toml"),
            "[package]\nname = \"boxdd-fixture\"\nversion = \"0.0.0\"\n\n[features]\ndefault = []\nserde = [\"dep:serde\"]\nserialize = [\"serde\", \"boxdd-sys/serialize\"]\nall-data = [\"serialize\"]\n",
        )
        .expect("feature closure fixture");

        let expanded = expanded_binding_route_features(&fixture.paths(), &["all-data".to_owned()])
            .expect("expanded feature closure");

        assert_eq!(
            expanded,
            BTreeSet::from([
                "all-data".to_owned(),
                "serde".to_owned(),
                "serialize".to_owned(),
            ])
        );
    }

    #[test]
    fn recording_inputs_require_the_exact_reviewed_source_set() {
        let inputs = placeholder_recording_inputs();
        let mut errors = Vec::new();
        validate_recording_input_shape(&inputs, &mut errors);
        assert!(errors.is_empty(), "valid recording inputs: {errors:?}");

        let mut missing = inputs.clone();
        missing.pop();
        let mut errors = Vec::new();
        validate_recording_input_shape(&missing, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("exact sorted reviewed set"))
        );

        let mut malformed = inputs;
        malformed[0].git_blob = "main".to_owned();
        malformed[1].blake3 = "not-a-digest".to_owned();
        let mut errors = Vec::new();
        validate_recording_input_shape(&malformed, &mut errors);
        assert!(errors.iter().any(|error| error.contains("Git blob ID")));
        assert!(errors.iter().any(|error| error.contains("BLAKE3")));
    }

    #[test]
    fn recording_input_blob_drift_requires_explicit_review() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let mut manifest = fixture.manifest();
        manifest.recording_inputs[0].git_blob = "0".repeat(40);

        let error = validate_repository(&paths, &manifest, false)
            .expect_err("forged reviewed blob must fail");

        assert!(error.to_string().contains("explicit wire review"));
    }

    #[test]
    fn repository_recording_input_identities_match_the_reviewed_target() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let paths = WorkspacePaths::new(root);
        let manifest = UpstreamManifest::load(&paths).expect("repository manifest");
        validate_recording_input_identities(&paths, &manifest)
            .expect("reviewed recording source identities");
    }

    #[test]
    fn exact_inventory_rejects_same_count_substitution() {
        let expected = inventory();
        let mut observed = expected.clone();
        observed.c_sources[1] = "src/c.c".to_owned();
        let error = validate_exact_inventory(&expected, &observed)
            .expect_err("same-count substitution must fail");
        assert!(error.to_string().contains("src/b.c"));
        assert!(error.to_string().contains("src/c.c"));
    }

    #[test]
    fn repository_manifest_matches_every_declared_revision_inventory() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let paths = WorkspacePaths::new(root);
        let manifest = UpstreamManifest::load(&paths).expect("repository manifest");
        let observed_active = source_inventory(&paths.box2d(), &manifest.active_revision)
            .expect("repository active inventory");
        validate_exact_inventory(&manifest.source_inventory, &observed_active)
            .expect("exact active inventory");
        match (&manifest.next_revision, &manifest.next_inventory) {
            (Some(next_revision), Some(next_inventory)) => {
                let observed_next = source_inventory(&paths.box2d(), next_revision)
                    .expect("repository target inventory");
                validate_exact_inventory(next_inventory, &observed_next)
                    .expect("exact target inventory");
            }
            (None, None) => {}
            _ => panic!("manifest validation must pair next_revision and next_inventory"),
        }
    }

    #[test]
    fn source_inventory_tracks_nested_sources_compiled_by_box2d_sys() {
        let fixture = TemporaryWorkspace::create();
        let nested = fixture.root.join("upstream/src/generated/nested.c");
        fs::create_dir_all(nested.parent().expect("nested source parent"))
            .expect("nested source directory");
        fs::write(&nested, "int nested(void) { return 3; }\n").expect("nested source");
        run_git(
            &fixture.root.join("upstream"),
            &["add", "src/generated/nested.c"],
        );
        run_git(
            &fixture.root.join("upstream"),
            &["commit", "-m", "nested source"],
        );
        let revision = git_output(&fixture.root.join("upstream"), ["rev-parse", "HEAD"])
            .expect("nested revision")
            .trim()
            .to_owned();

        let inventory =
            source_inventory(&fixture.root.join("upstream"), &revision).expect("nested inventory");
        assert!(
            inventory
                .c_sources
                .contains(&"src/generated/nested.c".to_owned())
        );
    }

    #[test]
    fn bindings_artifact_requires_exact_manifest_provenance() {
        let forged = TemporaryWorkspace::create();
        let paths = forged.paths();
        let mut manifest = forged.manifest();
        let binding_path = manifest
            .binding_artifacts()
            .next()
            .map(|artifact| artifact.path.clone())
            .expect("binding artifact");
        fs::write(
            paths.root().join(&binding_path),
            "// plausible bindgen output without provenance\n",
        )
        .expect("forged bindings");
        let digest = file_blake3(&paths.root().join(&binding_path)).expect("forged digest");
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == ArtifactKind::Bindings)
            .expect("binding artifact")
            .content_blake3 = digest;
        let error = validate_repository(&paths, &manifest, false)
            .expect_err("forged bindings must fail check");
        assert!(
            error
                .to_string()
                .contains("missing exact manifest provenance")
        );

        let missing = TemporaryWorkspace::create();
        let paths = missing.paths();
        let manifest = missing.manifest();
        let binding = manifest
            .binding_artifacts()
            .next()
            .expect("binding artifact");
        fs::remove_file(paths.root().join(&binding.path)).expect("remove fixture bindings");
        let error = validate_repository(&paths, &manifest, false)
            .expect_err("missing bindings must fail check");
        assert!(error.to_string().contains("bindings_pregenerated.rs"));
    }

    #[test]
    fn bootstrap_bindings_require_exact_regenerated_bytes() {
        let fixture = TemporaryWorkspace::create();
        let manifest = fixture.manifest();
        let generated = tempfile::tempdir().expect("generated bindings root");
        for artifact in manifest.binding_artifacts() {
            let installed_path = fixture.workspace.join(&artifact.path);
            let generated_path = generated.path().join(&artifact.path);
            fs::create_dir_all(generated_path.parent().expect("generated parent"))
                .expect("generated bindings parent");
            fs::copy(&installed_path, &generated_path).expect("matching generated bindings");
        }
        compare_binding_artifacts(&fixture.workspace, generated.path(), &manifest)
            .expect("byte-identical regenerated bindings");

        let artifact = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact");
        fs::write(
            generated.path().join(&artifact.path),
            "forged generated bindings\n",
        )
        .expect("mismatched generated bindings");
        let error = compare_binding_artifacts(&fixture.workspace, generated.path(), &manifest)
            .expect_err("bootstrap must not hash-and-bless stale bindings");
        assert!(error.to_string().contains("not byte-for-byte reproducible"));
    }

    #[test]
    fn recording_operations_are_read_from_the_reviewed_git_object() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let expected = git_output(
            &fixture.root.join("upstream"),
            [
                "show",
                &format!("{}:src/recording_ops.inl", manifest.recording_revision),
            ],
        )
        .expect("reviewed recording operation object");

        fs::write(
            paths.box2d().join("src/recording_ops.inl"),
            "B2_REC_OP( 0xff, Forged, RET_NONE )\n",
        )
        .expect("dirty checkout operation file");

        assert_eq!(
            reviewed_recording_operations_source(&paths, &manifest)
                .expect("reviewed recording operations"),
            expected
        );
    }

    #[test]
    fn artifact_content_digests_reject_body_drift_beyond_identity_markers() {
        for kind in [
            ArtifactKind::Bindings,
            ArtifactKind::RecordingWire,
            ArtifactKind::ApiCoverageReport,
        ] {
            let fixture = TemporaryWorkspace::create();
            let paths = fixture.paths();
            let manifest = fixture.manifest();
            let artifact = if kind == ArtifactKind::Bindings {
                manifest
                    .binding_artifacts()
                    .next()
                    .expect("bindings artifact")
            } else {
                manifest.artifact(kind).expect("artifact")
            };
            let path = paths.root().join(&artifact.path);
            let mut content = fs::read(&path).expect("artifact content");
            content.extend_from_slice(b"\nidentity-preserving forged body\n");
            fs::write(&path, content).expect("forged artifact body");

            let error = validate_repository(&paths, &manifest, false)
                .expect_err("body drift must fail the digest gate");
            assert!(error.to_string().contains("content digest drifted"));
        }
    }

    #[test]
    fn dirty_submodule_and_revision_mismatch_do_not_mutate_repository_state() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        validate_repository(&paths, &manifest, true).expect("clean fixture");
        let original_gitlink = indexed_gitlink(paths.root()).expect("original gitlink");
        let original_checkout =
            git_output(&paths.box2d(), ["rev-parse", "HEAD"]).expect("original checkout");

        fs::write(
            paths.box2d().join("src/a.c"),
            "dirty and intentionally preserved\n",
        )
        .expect("dirty submodule");
        let error = validate_repository(&paths, &manifest, false)
            .expect_err("dirty submodule must be rejected");
        assert!(error.to_string().contains("submodule is dirty"));
        assert_eq!(
            indexed_gitlink(paths.root()).expect("gitlink after rejection"),
            original_gitlink
        );
        assert_eq!(
            git_output(&paths.box2d(), ["rev-parse", "HEAD"]).expect("checkout after rejection"),
            original_checkout
        );
        assert_eq!(
            fs::read_to_string(paths.box2d().join("src/a.c")).expect("dirty file preserved"),
            "dirty and intentionally preserved\n"
        );

        run_git(&paths.box2d(), &["checkout", "--", "src/a.c"]);
        let mut mismatched = manifest;
        mismatched.active_revision = fixture.next_revision.clone();
        mismatched.next_revision = None;
        mismatched.next_inventory = None;
        let error = validate_repository(&paths, &mismatched, false)
            .expect_err("checkout mismatch must be rejected");
        assert!(error.to_string().contains("submodule checkout"));
        assert_eq!(
            indexed_gitlink(paths.root()).expect("gitlink after mismatch"),
            fixture.active_revision
        );
        assert_eq!(
            git_output(&paths.box2d(), ["rev-parse", "HEAD"])
                .expect("checkout after mismatch")
                .trim(),
            fixture.active_revision
        );
    }

    #[test]
    fn write_preflight_accepts_committed_target_candidates_without_mutation() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let manifest = fixture.commit_target_candidates(&manifest);
        let original_gitlink = indexed_gitlink(paths.root()).expect("original gitlink");
        let original_checkout =
            git_output(&paths.box2d(), ["rev-parse", "HEAD"]).expect("original checkout");

        validate_update_preconditions(&paths, &manifest).expect("target candidate preflight");
        validate_repository(&paths, &manifest, true)
            .expect("active check remains valid with target candidate");

        assert_eq!(
            indexed_gitlink(paths.root()).expect("gitlink after preflight"),
            original_gitlink
        );
        assert_eq!(
            git_output(&paths.box2d(), ["rev-parse", "HEAD"]).expect("checkout after preflight"),
            original_checkout
        );
    }

    #[test]
    fn candidate_digest_and_revision_identity_both_fail_closed() {
        let forged = TemporaryWorkspace::create();
        let forged_paths = forged.paths();
        let manifest = forged.manifest();
        let mut manifest = forged.commit_target_candidates(&manifest);
        let candidate = forged_paths.root().join(
            manifest
                .artifact(ArtifactKind::ApiContract)
                .expect("API artifact")
                .candidate_path
                .as_deref()
                .expect("API candidate"),
        );
        fs::write(
            &candidate,
            "upstream_sha = \"0000000000000000000000000000000000000000\"\n",
        )
        .expect("forged candidate");
        let forged_digest = file_blake3(&candidate).expect("forged candidate digest");
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == ArtifactKind::ApiContract)
            .expect("API artifact")
            .candidate_blake3 = Some(forged_digest);
        let error = validate_repository(&forged_paths, &manifest, false)
            .expect_err("forged candidate identity must fail check");
        assert!(error.to_string().contains("does not match next revision"));

        let dirty = TemporaryWorkspace::create();
        let dirty_paths = dirty.paths();
        let manifest = dirty.manifest();
        let manifest = dirty.commit_target_candidates(&manifest);
        let candidate = dirty_paths.root().join(
            manifest
                .artifact(ArtifactKind::ApiContract)
                .expect("API artifact")
                .candidate_path
                .as_deref()
                .expect("API candidate"),
        );
        let mut content = fs::read_to_string(&candidate).expect("candidate content");
        content.push_str("# reviewed but not committed\n");
        fs::write(&candidate, content).expect("dirty candidate");
        let error = validate_repository(&dirty_paths, &manifest, false)
            .expect_err("unreviewed candidate content must fail check");
        assert!(error.to_string().contains("content digest drifted"));
    }

    #[test]
    fn isolated_generation_finish_removes_worktree_registration_and_directory() {
        let fixture = TemporaryWorkspace::create();
        let generation = IsolatedGeneration::create(&fixture.paths(), &fixture.next_revision)
            .expect("isolated generation");
        let worktree = generation.worktree.clone();
        let source_root = generation.source_root.clone();
        let common_anchor = repository_lock_path(
            &fixture.workspace,
            Path::new("boxdd-isolated-generation-test.anchor"),
        )
        .expect("repository-owned isolation anchor");
        let common_directory = common_anchor.parent().expect("Git common directory");
        assert_eq!(source_root.parent(), Some(common_directory));
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;

            assert_eq!(
                fs::metadata(&source_root).expect("source metadata").dev(),
                fs::metadata(common_directory)
                    .expect("Git common directory metadata")
                    .dev()
            );
            assert_eq!(
                fs::metadata(&worktree).expect("worktree metadata").dev(),
                fs::metadata(&source_root).expect("source metadata").dev()
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt as _;

            assert_eq!(
                fs::metadata(&source_root)
                    .expect("source metadata")
                    .volume_serial_number(),
                fs::metadata(common_directory)
                    .expect("Git common directory metadata")
                    .volume_serial_number()
            );
            assert_eq!(
                fs::metadata(&worktree)
                    .expect("worktree metadata")
                    .volume_serial_number(),
                fs::metadata(&source_root)
                    .expect("source metadata")
                    .volume_serial_number()
            );
        }
        let before = git_output(&fixture.workspace, ["worktree", "list", "--porcelain"])
            .expect("worktree list");
        assert!(before.contains(&worktree.to_string_lossy().into_owned()));

        generation.finish().expect("explicit generation cleanup");

        let after = git_output(&fixture.workspace, ["worktree", "list", "--porcelain"])
            .expect("worktree list after cleanup");
        assert!(
            !after.contains(&worktree.to_string_lossy().into_owned()),
            "unexpected registration after cleanup:\n{after}"
        );
        assert!(!source_root.exists());
    }

    #[test]
    fn generation_baseline_rejects_relevant_dirty_and_ignored_inputs() {
        let dirty = TemporaryWorkspace::create();
        let cargo_manifest = dirty.workspace.join("boxdd/Cargo.toml");
        fs::write(&cargo_manifest, "# uncommitted generator input\n")
            .expect("dirty generator input");
        let error = GenerationBaseline::capture(&dirty.workspace)
            .expect_err("dirty generator input must fail closed");
        assert!(error.to_string().contains("generator inputs are dirty"));
        assert!(error.to_string().contains("boxdd/Cargo.toml"));

        let ignored = TemporaryWorkspace::create();
        let exclude = git_output(
            &ignored.workspace,
            ["rev-parse", "--git-path", "info/exclude"],
        )
        .expect("root exclude path");
        let exclude = PathBuf::from(exclude.trim());
        let exclude = if exclude.is_absolute() {
            exclude
        } else {
            ignored.workspace.join(exclude)
        };
        fs::write(&exclude, "boxdd/src/local.rs\n").expect("ignored generator input rule");
        fs::create_dir_all(ignored.workspace.join("boxdd/src"))
            .expect("ignored generator input parent");
        fs::write(
            ignored.workspace.join("boxdd/src/local.rs"),
            "local input\n",
        )
        .expect("ignored generator input");
        let error = GenerationBaseline::capture(&ignored.workspace)
            .expect_err("ignored generator input must fail closed");
        assert!(error.to_string().contains("boxdd/src/local.rs"));
    }

    #[test]
    fn generation_baseline_allows_unrelated_dirty_paths() {
        let fixture = TemporaryWorkspace::create();
        fs::write(
            fixture.workspace.join("local-notes.txt"),
            "unrelated untracked content\n",
        )
        .expect("unrelated path");

        GenerationBaseline::capture(&fixture.workspace)
            .expect("unrelated paths are outside the generator authority");
    }

    #[test]
    fn route_refresh_input_gate_rejects_controlled_dirty_and_untracked_paths() {
        let dirty = TemporaryWorkspace::create();
        fs::write(
            dirty.workspace.join("boxdd/Cargo.toml"),
            "# intentionally dirty route generator input\n",
        )
        .expect("dirty controlled input");
        let error = capture_route_refresh_inputs(&dirty.workspace)
            .expect_err("tracked controlled input drift must fail closed");
        assert!(error.to_string().contains("generator inputs are dirty"));
        assert!(error.to_string().contains("boxdd/Cargo.toml"));

        let untracked = TemporaryWorkspace::create();
        let untracked_input = untracked.workspace.join("boxdd/src/local.rs");
        fs::create_dir_all(untracked_input.parent().expect("untracked input parent"))
            .expect("untracked input directory");
        fs::write(&untracked_input, "pub fn local() {}\n").expect("untracked controlled input");
        let error = capture_route_refresh_inputs(&untracked.workspace)
            .expect_err("untracked controlled input must fail closed");
        assert!(error.to_string().contains("generator inputs are dirty"));
        assert!(error.to_string().contains("boxdd/src/local.rs"));
    }

    #[test]
    fn route_input_snapshot_overlays_clean_inputs_and_ignores_unrelated_paths() {
        let fixture = TemporaryWorkspace::create();
        fs::write(
            fixture.workspace.join("local-notes.txt"),
            "unrelated dirty state\n",
        )
        .expect("unrelated dirty path");
        let snapshot = capture_route_refresh_inputs(&fixture.workspace)
            .expect("unrelated paths are outside the route generator authority");

        let destination = TempDir::new().expect("overlay destination");
        fs::create_dir_all(destination.path().join("xtask/src"))
            .expect("stale controlled directory");
        fs::write(destination.path().join("xtask/src/stale.rs"), "stale\n")
            .expect("stale controlled file");
        snapshot
            .overlay(destination.path())
            .expect("overlay clean controlled inputs");
        assert_eq!(
            fs::read(destination.path().join("boxdd/Cargo.toml"))
                .expect("overlaid controlled input"),
            fs::read(fixture.workspace.join("boxdd/Cargo.toml")).expect("source controlled input")
        );
        assert!(!destination.path().join("xtask/src/stale.rs").exists());

        fs::write(
            fixture.workspace.join("boxdd/Cargo.toml"),
            "# concurrent controlled drift\n",
        )
        .expect("concurrent controlled drift");
        let error = snapshot
            .verify(&fixture.workspace)
            .expect_err("controlled drift must invalidate the snapshot");
        assert!(
            error
                .to_string()
                .contains("controlled route generator input")
        );
    }

    #[test]
    fn canonical_route_topology_rejects_missing_and_misdirected_coordinates() {
        let mut topology = manifest();
        topology.binding_routes = canonical_binding_routes();
        topology
            .artifacts
            .retain(|artifact| artifact.kind != ArtifactKind::Bindings);
        let mut bindings = canonical_route_binding_artifacts();
        bindings.append(&mut topology.artifacts);
        topology.artifacts = bindings;
        validate_route_refresh_topology(&topology).expect("canonical route topology");
        assert_eq!(topology.binding_routes.len(), 10);
        assert_eq!(topology.binding_artifacts().count(), 6);

        let mut missing = topology.clone();
        missing.binding_routes.pop();
        let error = validate_route_refresh_topology(&missing)
            .expect_err("missing route coordinate must fail closed");
        assert!(error.to_string().contains("canonical 10-route matrix"));

        let mut wrong_target = topology;
        let wasm = wrong_target
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.name == "bindings-wasm32-wasip1-single")
            .expect("WASI binding artifact");
        wasm.target = ArtifactTarget::Wasm32UnknownUnknown;
        let error = validate_route_refresh_topology(&wrong_target)
            .expect_err("misdirected artifact must fail closed");
        assert!(error.to_string().contains("incomplete or incorrect"));
    }

    #[test]
    fn route_refresh_transaction_rolls_back_every_install_boundary() {
        let fixture = TemporaryWorkspace::create();
        let (paths, original, staged, manifest_content) =
            route_refresh_transaction_fixture(&fixture);
        let inputs = GeneratorInputSnapshot::capture(paths.root()).expect("route input snapshot");
        let baseline = RouteRefreshBaseline::capture(
            &paths,
            inputs,
            &manifest_content,
            &original,
            &staged.manifest,
        )
        .expect("route refresh baseline");
        let unrelated = paths.root().join("unrelated-route-notes.txt");
        fs::write(&unrelated, "preserve me\n").expect("unrelated route state");
        let operations = staged.files.len() + staged.removals.len() + 1;

        for boundary in 0..=operations {
            let error = install_route_refresh(
                &paths,
                &original,
                &staged,
                &baseline,
                Some(boundary),
                || Ok(()),
            )
            .expect_err("injected route refresh boundary must roll back");
            assert!(error.to_string().contains("injected transition failure"));
            baseline
                .verify_before_install(&paths)
                .unwrap_or_else(|error| {
                    panic!("route refresh boundary {boundary} did not restore baseline: {error}")
                });
            assert_eq!(
                fs::read_to_string(&unrelated).expect("preserved unrelated route state"),
                "preserve me\n"
            );
        }

        let error = install_route_refresh(&paths, &original, &staged, &baseline, None, || {
            Err(Error::message("injected route terminal validation failure"))
        })
        .expect_err("terminal route validation failure must roll back");
        assert!(
            error
                .to_string()
                .contains("route terminal validation failure")
        );
        baseline
            .verify_before_install(&paths)
            .expect("terminal failure restored the route baseline");
    }

    #[test]
    fn route_refresh_rejects_concurrent_controlled_drift_before_installation() {
        let fixture = TemporaryWorkspace::create();
        let (paths, original, staged, manifest_content) =
            route_refresh_transaction_fixture(&fixture);
        let inputs = GeneratorInputSnapshot::capture(paths.root()).expect("route input snapshot");
        let baseline = RouteRefreshBaseline::capture(
            &paths,
            inputs,
            &manifest_content,
            &original,
            &staged.manifest,
        )
        .expect("route refresh baseline");
        let original_binding =
            fs::read(paths.root().join("boxdd-sys/src/bindings_pregenerated.rs"))
                .expect("original binding");
        fs::write(
            paths.root().join("boxdd/Cargo.toml"),
            "# concurrent controlled edit\n",
        )
        .expect("concurrent controlled edit");

        let error = install_route_refresh(&paths, &original, &staged, &baseline, None, || Ok(()))
            .expect_err("concurrent controlled drift must fail before installation");
        assert!(
            error
                .to_string()
                .contains("controlled route generator input")
        );
        assert_eq!(
            fs::read(paths.root().join("boxdd-sys/src/bindings_pregenerated.rs"))
                .expect("unchanged binding"),
            original_binding
        );
        assert_eq!(
            fs::read(paths.upstream_manifest()).expect("unchanged manifest"),
            manifest_content
        );
        assert!(
            !paths
                .root()
                .join("boxdd-sys/src/bindings_wasm32_wasip1.rs")
                .exists()
        );
    }

    #[test]
    fn wasi_sysroot_and_binding_provenance_share_the_pinned_identity() {
        let sysroot = TempDir::new().expect("WASI sysroot");
        let missing = bindgen_contract::validate_wasi_sysroot(sysroot.path())
            .expect_err("missing math.h must fail closed");
        assert!(missing.contains("include/wasm32-wasip1"));
        let headers = sysroot.path().join("include/wasm32-wasip1");
        fs::create_dir_all(&headers).expect("WASI include directory");
        fs::write(headers.join("math.h"), "#pragma once\n").expect("WASI math header");
        let drift = bindgen_contract::validate_wasi_sysroot(sysroot.path())
            .expect_err("unpinned headers must fail closed");
        assert!(drift.contains("identity mismatch"));

        let artifact = canonical_route_binding_artifacts()
            .into_iter()
            .find(|artifact| artifact.name == "bindings-wasm32-wasip1-single")
            .expect("WASI binding artifact");
        let provenance = binding_provenance(
            &artifact,
            "0123456789abcdef0123456789abcdef01234567",
            RustTarget::Wasm32Wasip1,
            Some(bindgen_contract::WASI_LIBC_HEADERS_SHA256),
            None,
        )
        .expect("pinned WASI binding provenance");
        assert!(provenance.contains(bindgen_contract::WASI_LIBC_VERSION));
        assert!(provenance.contains(bindgen_contract::WASI_LIBC_HEADERS_SHA256));
        assert!(
            binding_provenance(
                &artifact,
                "0123456789abcdef0123456789abcdef01234567",
                RustTarget::Wasm32Wasip1,
                None,
                None,
            )
            .is_err()
        );

        let freestanding_artifact = canonical_route_binding_artifacts()
            .into_iter()
            .find(|artifact| artifact.name == "bindings-wasm32-unknown-unknown-single")
            .expect("freestanding binding artifact");
        let provenance = binding_provenance(
            &freestanding_artifact,
            "0123456789abcdef0123456789abcdef01234567",
            RustTarget::Wasm32UnknownUnknown,
            None,
            Some(bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256),
        )
        .expect("pinned freestanding binding provenance");
        assert!(provenance.contains(bindgen_contract::UNKNOWN_UNKNOWN_MATH_HEADER_SHA256));
        assert!(
            binding_provenance(
                &freestanding_artifact,
                "0123456789abcdef0123456789abcdef01234567",
                RustTarget::Wasm32UnknownUnknown,
                None,
                None,
            )
            .is_err()
        );
        assert_eq!(
            binding_generation_provider(RustTarget::X86_64UnknownLinuxGnu),
            "vendored"
        );
        assert_eq!(
            binding_generation_provider(RustTarget::Wasm32UnknownUnknown),
            "wasm-compile-only"
        );
        assert_eq!(
            binding_generation_provider(RustTarget::Wasm32Wasip1),
            "wasm-compile-only"
        );
    }

    #[test]
    fn isolated_generation_uses_the_pinned_root_revision_and_rejects_head_drift() {
        let fixture = TemporaryWorkspace::create();
        let baseline =
            GenerationBaseline::capture(&fixture.workspace).expect("generation baseline");
        fs::write(
            fixture.workspace.join("unrelated-tracked.txt"),
            "new committed state\n",
        )
        .expect("new root state");
        run_git(&fixture.workspace, &["add", "unrelated-tracked.txt"]);
        run_git(&fixture.workspace, &["commit", "-m", "move root head"]);

        let generation = IsolatedGeneration::create_at(
            &fixture.paths(),
            &baseline.repository_revision,
            &fixture.next_revision,
        )
        .expect("generation at captured root revision");
        assert_eq!(
            git_output(&generation.worktree, ["rev-parse", "HEAD"])
                .expect("isolated root revision")
                .trim(),
            baseline.repository_revision
        );
        generation.finish().expect("generation cleanup");

        let error = baseline
            .verify(&fixture.workspace)
            .expect_err("root HEAD drift must fail before installation");
        assert!(error.to_string().contains("repository HEAD changed"));
    }

    #[test]
    fn failed_staging_drop_removes_worktree_registration_and_directory() {
        let fixture = TemporaryWorkspace::create();
        let manifest = fixture.manifest();
        let mut generation = IsolatedGeneration::create(&fixture.paths(), &fixture.next_revision)
            .expect("isolated generation");
        let worktree = generation.worktree.clone();
        let source_root = generation.source_root.clone();

        generation
            .prepare_update(&manifest, &fixture.next_revision)
            .expect_err("missing reviewed candidate must fail staging");
        generation.repository_worktree_added = false;
        drop(generation);

        let registrations = git_output(&fixture.workspace, ["worktree", "list", "--porcelain"])
            .expect("worktree list after staging failure");
        assert!(
            !registrations.contains(&worktree.to_string_lossy().into_owned()),
            "unexpected registration after drop:\n{registrations}"
        );
        assert!(!source_root.exists());
    }

    #[test]
    fn initialization_failure_reports_a_concurrent_cleanup_failure() {
        let fixture = TemporaryWorkspace::create();
        let cargo_config = fixture.workspace.join(".cargo");
        fs::create_dir(&cargo_config).expect("fixture Cargo config directory");
        fs::write(
            cargo_config.join("config.toml"),
            "[build]\nrustflags = []\n",
        )
        .expect("fixture Cargo config");
        run_git(&fixture.workspace, &["add", ".cargo/config.toml"]);
        run_git(&fixture.workspace, &["commit", "-m", "inject Cargo config"]);
        let repository_revision = git_output(&fixture.workspace, ["rev-parse", "HEAD"])
            .expect("fixture repository revision");

        let error = match IsolatedGeneration::create_at_with_cleanup(
            &fixture.paths(),
            repository_revision.trim(),
            &fixture.next_revision,
            |generation| {
                assert!(generation.repository_worktree_added);
                assert!(
                    worktree_is_registered(&generation.repository_root, &generation.worktree)
                        .expect("registered worktree before injected cleanup failure")
                );
                Err(Error::message("injected cleanup failure"))
            },
        ) {
            Ok(_) => panic!("worktree-local Cargo config must fail qualification"),
            Err(error) => error,
        };
        let message = error.to_string();
        assert!(message.contains("workspace ancestor Cargo config"));
        assert!(message.contains("isolated worktree cleanup also failed"));
        assert!(message.contains("injected cleanup failure"));
    }

    #[test]
    fn failed_cargo_qualification_removes_the_isolated_worktree_registration() {
        let fixture = TemporaryWorkspace::create();
        let cargo_config = fixture.workspace.join(".cargo");
        fs::create_dir(&cargo_config).expect("fixture Cargo config directory");
        fs::write(
            cargo_config.join("config.toml"),
            "[build]\nrustflags = []\n",
        )
        .expect("fixture Cargo config");
        run_git(&fixture.workspace, &["add", ".cargo/config.toml"]);
        run_git(&fixture.workspace, &["commit", "-m", "inject Cargo config"]);
        let repository_revision = git_output(&fixture.workspace, ["rev-parse", "HEAD"])
            .expect("fixture repository revision");
        let registrations_before =
            git_output(&fixture.workspace, ["worktree", "list", "--porcelain"])
                .expect("worktree list before qualification failure");

        let error = match IsolatedGeneration::create_at(
            &fixture.paths(),
            repository_revision.trim(),
            &fixture.next_revision,
        ) {
            Ok(_) => panic!("worktree-local Cargo config must fail qualification"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("workspace ancestor Cargo config")
        );
        let registrations_after =
            git_output(&fixture.workspace, ["worktree", "list", "--porcelain"])
                .expect("worktree list after qualification failure");
        assert_eq!(registrations_after, registrations_before);
    }

    #[test]
    fn worktree_cleanup_continues_after_registration_inspection_failure() {
        let fixture = TemporaryWorkspace::create();
        let mut generation = IsolatedGeneration::create(&fixture.paths(), &fixture.next_revision)
            .expect("isolated generation");
        let worktree = generation.worktree.clone();
        let source_root = generation.source_root.clone();

        let error = generation
            .cleanup_after_inspection(Err(Error::message("injected inspection failure")))
            .expect_err("inspection failure remains observable");

        assert!(error.to_string().contains("injected inspection failure"));
        assert!(!source_root.exists());
        assert!(
            !worktree_is_registered(&fixture.workspace, &worktree)
                .expect("registration after best-effort cleanup")
        );
    }

    #[test]
    fn worktree_cleanup_preserves_the_directory_when_registration_removal_fails() {
        let fixture = TemporaryWorkspace::create();
        let mut generation = IsolatedGeneration::create(&fixture.paths(), &fixture.next_revision)
            .expect("isolated generation");
        let worktree = generation.worktree.clone();
        let source_root = generation.source_root.clone();

        let error = generation
            .cleanup_after_inspection_with(Ok(true), |_, _| {
                Err(Error::message("injected worktree removal failure"))
            })
            .expect_err("worktree removal failure must be reported");

        let message = error.to_string();
        assert!(message.contains("injected worktree removal failure"));
        assert!(message.contains("isolated source directory preserved at"));
        assert!(source_root.exists());
        assert!(worktree.exists());
        assert!(
            worktree_is_registered(&fixture.workspace, &worktree)
                .expect("registration after failed removal")
        );

        remove_repository_worktree(&fixture.workspace, &worktree)
            .expect("remove preserved fixture worktree registration");
        fs::remove_dir_all(&source_root).expect("remove preserved fixture source directory");
        generation.repository_worktree_added = false;
    }

    #[test]
    fn worktree_cleanup_preserves_workspace_needed_by_pending_atomic_recovery() {
        let fixture = TemporaryWorkspace::create();
        let mut generation = IsolatedGeneration::create(&fixture.paths(), &fixture.next_revision)
            .expect("isolated generation");
        let worktree = generation.worktree.clone();
        let source_root = generation.source_root.clone();
        let recovery_root = crate::config::atomic_batch_recovery_root(&worktree)
            .expect("atomic batch recovery root");
        let transaction = recovery_root.join("transaction-incomplete-publish");
        fs::create_dir(&transaction).expect("published transaction without journal");

        let error = generation
            .cleanup()
            .expect_err("pending recovery must preserve its workspace");

        let message = error.to_string();
        assert!(message.contains("atomic batch recovery"));
        assert!(message.contains("isolated source directory preserved at"));
        assert!(source_root.exists());
        assert!(worktree.exists());
        assert!(
            worktree_is_registered(&fixture.workspace, &worktree)
                .expect("registration after deferred cleanup")
        );

        fs::remove_dir(&transaction).expect("remove incomplete transaction fixture");
        cleanup_deferred_isolated_generations(&fixture.workspace)
            .expect("finish deferred isolated generation cleanup");
        assert!(!source_root.exists());
        assert!(
            !worktree_is_registered(&fixture.workspace, &worktree)
                .expect("registration after deferred cleanup completes")
        );
        generation.repository_worktree_added = false;
    }

    #[test]
    fn deferred_isolated_cleanup_requires_an_exact_ownership_marker() {
        let fixture = TemporaryWorkspace::create();
        let common_directory = isolated_generation_parent(&fixture.workspace)
            .expect("isolated generation common directory");
        let unrelated = common_directory.join(format!(
            "{ISOLATED_GENERATION_DIRECTORY_PREFIX}unowned-fixture"
        ));
        fs::create_dir(&unrelated).expect("unowned prefix directory");

        cleanup_deferred_isolated_generations(&fixture.workspace)
            .expect("unowned prefix directory must be ignored");

        assert!(unrelated.is_dir());
        fs::remove_dir(&unrelated).expect("remove unowned prefix directory fixture");
    }

    #[test]
    fn deferred_isolated_cleanup_removes_a_half_initialized_worktree() {
        let fixture = TemporaryWorkspace::create();
        let common_directory = isolated_generation_parent(&fixture.workspace)
            .expect("isolated generation common directory");
        let source = tempfile::Builder::new()
            .prefix(ISOLATED_GENERATION_DIRECTORY_PREFIX)
            .tempdir_in(&common_directory)
            .expect("half-initialized source directory");
        let source_root = source.keep().canonicalize().expect("canonical source root");
        let worktree = source_root.join("workspace");
        fs::create_dir(&worktree).expect("half-initialized worktree directory");
        write_isolated_generation_marker(&common_directory, &source_root, &worktree)
            .expect("isolated generation marker");

        cleanup_deferred_isolated_generations(&fixture.workspace)
            .expect("half-initialized worktree cleanup");

        assert!(!source_root.exists());
        assert!(
            !worktree_is_registered(&fixture.workspace, &worktree)
                .expect("half-initialized worktree registration")
        );
    }

    #[test]
    fn isolated_cleanup_does_not_prune_unrelated_worktree_registrations() {
        let fixture = TemporaryWorkspace::create();
        let unrelated = fixture.root.join("unrelated-worktree");
        command_success(
            git_command()
                .expect("qualified Git")
                .current_dir(&fixture.workspace)
                .args(["worktree", "add", "--detach"])
                .arg(&unrelated)
                .arg("HEAD"),
            "create unrelated fixture worktree",
        )
        .expect("unrelated worktree");
        fs::remove_dir_all(&unrelated).expect("make unrelated registration prunable");

        let generation = IsolatedGeneration::create(&fixture.paths(), &fixture.next_revision)
            .expect("isolated generation");
        generation.finish().expect("generation cleanup");

        let registrations = git_output(&fixture.workspace, ["worktree", "list", "--porcelain"])
            .expect("worktree registrations");
        assert!(registrations.contains(&unrelated.to_string_lossy().into_owned()));
    }

    #[test]
    fn update_lock_is_exclusive_and_released_on_drop() {
        let fixture = TemporaryWorkspace::create();
        let first = UpdateLock::acquire(&fixture.workspace).expect("first update lock");
        let error = UpdateLock::acquire(&fixture.workspace)
            .expect_err("second update lock must fail")
            .to_string();
        assert!(error.contains("another update may be running"));
        drop(first);
        UpdateLock::acquire(&fixture.workspace).expect("lock after release");
    }

    #[test]
    fn update_lock_uses_the_common_directory_across_worktrees() {
        let fixture = TemporaryWorkspace::create();
        let worktree = fixture.root.join("lock-worktree");
        command_success(
            git_command()
                .expect("qualified Git")
                .current_dir(&fixture.workspace)
                .args(["worktree", "add", "--detach"])
                .arg(&worktree)
                .arg("HEAD"),
            "create lock fixture worktree",
        )
        .expect("fixture worktree");

        assert_eq!(
            UpdateLock::lock_path(&fixture.workspace).expect("main worktree lock path"),
            UpdateLock::lock_path(&worktree).expect("linked worktree lock path")
        );
        let first = UpdateLock::acquire(&fixture.workspace).expect("main worktree lock");
        let error = UpdateLock::acquire(&worktree)
            .expect_err("linked worktree must observe the common lock")
            .to_string();
        assert!(error.contains("another update may be running"));
        drop(first);
        command_success(
            git_command()
                .expect("qualified Git")
                .current_dir(&fixture.workspace)
                .args(["worktree", "remove", "--force"])
                .arg(&worktree),
            "remove lock fixture worktree",
        )
        .expect("fixture worktree cleanup");
    }

    #[test]
    fn git_index_replacement_preserves_staged_state_and_rolls_back_exactly() {
        let fixture = TemporaryWorkspace::create();
        fs::write(
            fixture.workspace.join("unrelated-tracked.txt"),
            "user staged state\n",
        )
        .expect("unrelated staged state");
        run_git(&fixture.workspace, &["add", "unrelated-tracked.txt"]);
        let before = GitIndexSnapshot::capture(&fixture.workspace).expect("index baseline");
        let mut replacement = GitIndexReplacement::prepare(
            &fixture.workspace,
            before.clone(),
            &fixture.next_revision,
        )
        .expect("prepared index replacement");

        replacement.install().expect("install target gitlink");

        assert_eq!(
            indexed_gitlink(&fixture.workspace).expect("target gitlink"),
            fixture.next_revision
        );
        assert_eq!(
            git_output(
                &fixture.workspace,
                [
                    "diff",
                    "--cached",
                    "--name-only",
                    "--",
                    "unrelated-tracked.txt"
                ]
            )
            .expect("staged unrelated path")
            .trim(),
            "unrelated-tracked.txt"
        );

        replacement.rollback().expect("exact index rollback");

        assert_eq!(
            GitIndexSnapshot::capture(&fixture.workspace).expect("rolled back index"),
            before
        );
        assert_eq!(
            indexed_gitlink(&fixture.workspace).expect("active gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn git_index_cas_preserves_a_third_state_created_before_install() {
        let fixture = TemporaryWorkspace::create();
        let before = GitIndexSnapshot::capture(&fixture.workspace).expect("index baseline");
        let mut replacement =
            GitIndexReplacement::prepare(&fixture.workspace, before, &fixture.next_revision)
                .expect("prepared index replacement");
        fs::write(
            fixture.workspace.join("unrelated-tracked.txt"),
            "concurrent staged state\n",
        )
        .expect("concurrent staged state");
        run_git(&fixture.workspace, &["add", "unrelated-tracked.txt"]);
        let concurrent = GitIndexSnapshot::capture(&fixture.workspace).expect("concurrent index");

        let error = replacement
            .install()
            .expect_err("CAS must reject the third index state");

        assert!(
            error
                .to_string()
                .contains("index changed during transaction")
        );
        assert_eq!(
            GitIndexSnapshot::capture(&fixture.workspace).expect("preserved concurrent index"),
            concurrent
        );
        assert_eq!(
            indexed_gitlink(&fixture.workspace).expect("preserved active gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn public_mutating_commands_lock_before_loading_the_manifest() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let original_manifest = fs::read(paths.upstream_manifest()).expect("original manifest");
        let lock = UpdateLock::acquire(paths.root()).expect("held update lock");
        fs::write(paths.upstream_manifest(), "not valid TOML")
            .expect("temporarily invalid manifest");

        for args in [
            ["upstream-sync", "--write"],
            ["upstream-sync", "--refresh-routes"],
            ["upstream-sync", "--prepare-next"],
            ["api-coverage", "--write"],
            ["api-coverage", "--refresh-abi"],
        ] {
            let error = crate::run_in(&paths, args.map(str::to_owned))
                .expect_err("held update lock must reject every mutating command");
            assert!(
                error.to_string().contains("another update may be running"),
                "{args:?} loaded mutable state before acquiring the lock: {error}"
            );
        }

        fs::write(paths.upstream_manifest(), original_manifest).expect("restore fixture manifest");
        drop(lock);
    }

    #[test]
    fn preflight_snapshot_rejects_and_preserves_concurrent_managed_edits() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let manifest = fixture.commit_target_candidates(&manifest);
        let baseline = ManagedSnapshot::capture(&paths, &manifest).expect("managed baseline");
        let bindings = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact");
        let bindings_path = paths.root().join(&bindings.path);
        let concurrent_content = b"user edit after preflight\n";
        fs::write(&bindings_path, concurrent_content).expect("concurrent edit");

        let staged = StagedUpdate {
            manifest: manifest.clone(),
            artifacts: vec![StagedFile {
                relative_path: bindings.path.clone(),
                content: b"generated replacement\n".to_vec(),
            }],
            candidate_paths: Vec::new(),
        };
        let error = install_staged_update_with(
            &paths,
            &manifest,
            &staged,
            Some(&baseline),
            None,
            || Ok(()),
        )
        .expect_err("concurrent managed edit must fail closed");

        assert!(
            error
                .to_string()
                .contains("changed after upstream-sync preflight")
        );
        assert_eq!(
            fs::read(&bindings_path).expect("preserved concurrent edit"),
            concurrent_content
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("unchanged gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn loaded_state_rejects_manifest_and_head_changes_before_snapshot_capture() {
        let manifest_drift = TemporaryWorkspace::create();
        let paths = manifest_drift.paths();
        let manifest = manifest_drift.manifest();
        let loaded_manifest = fs::read(paths.upstream_manifest()).expect("loaded manifest");
        let loaded_generation = GenerationBaseline::capture_with_policy(paths.root(), false)
            .expect("loaded generation state");
        let mut changed_manifest = loaded_manifest.clone();
        changed_manifest.extend_from_slice(b"\n# concurrent committed review\n");
        fs::write(paths.upstream_manifest(), changed_manifest).expect("concurrent manifest edit");
        let baseline = ManagedSnapshot::capture_observed(&paths, &manifest)
            .expect("snapshot after manifest drift");
        let error = baseline
            .verify_loaded_state(&paths, &loaded_manifest, &loaded_generation)
            .expect_err("pre-snapshot manifest drift must fail closed");
        assert!(
            error
                .to_string()
                .contains("manifest changed between loading and preflight")
        );

        let head_drift = TemporaryWorkspace::create();
        let paths = head_drift.paths();
        let manifest = head_drift.manifest();
        let loaded_manifest = fs::read(paths.upstream_manifest()).expect("loaded manifest");
        let loaded_generation = GenerationBaseline::capture_with_policy(paths.root(), false)
            .expect("loaded generation state");
        fs::write(
            paths.root().join("unrelated-tracked.txt"),
            "concurrent committed state\n",
        )
        .expect("concurrent root edit");
        run_git(paths.root(), &["add", "unrelated-tracked.txt"]);
        run_git(paths.root(), &["commit", "-m", "concurrent root commit"]);
        let baseline = ManagedSnapshot::capture_observed(&paths, &manifest)
            .expect("snapshot after HEAD drift");
        let error = baseline
            .verify_loaded_state(&paths, &loaded_manifest, &loaded_generation)
            .expect_err("pre-snapshot HEAD drift must fail closed");
        assert!(
            error
                .to_string()
                .contains("HEAD changed between manifest loading and preflight")
        );
    }

    #[test]
    fn observed_snapshot_detects_generator_edits_during_check() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let baseline =
            ManagedSnapshot::capture_observed(&paths, &manifest).expect("observed snapshot");
        let generator_input = paths.root().join("boxdd/Cargo.toml");
        fs::write(&generator_input, "# concurrent generator edit\n")
            .expect("concurrent generator edit");

        let error = baseline
            .verify_all(&paths)
            .expect_err("check snapshot must reject generator input drift");
        assert!(
            error
                .to_string()
                .contains("generator input contents changed during generation")
        );
    }

    #[test]
    fn every_failed_transition_boundary_restores_only_transaction_changes() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let manifest = fixture.commit_target_candidates(&manifest);
        let bindings = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact");
        let bindings_path = paths.root().join(&bindings.path);
        let candidate = manifest
            .artifact(ArtifactKind::ApiContract)
            .expect("API artifact")
            .candidate_path
            .clone()
            .expect("API candidate");
        let candidate_path = paths.root().join(&candidate);
        let original_bindings = fs::read(&bindings_path).expect("original bindings");
        let original_candidate = fs::read(&candidate_path).expect("original candidate");
        let original_manifest = fs::read(paths.upstream_manifest()).expect("original manifest");
        let unrelated = UnrelatedGitState::create(paths.root());
        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = fixture.next_revision.clone();
        target_manifest.next_revision = None;
        target_manifest.source_inventory = target_manifest
            .next_inventory
            .take()
            .expect("target inventory");
        for artifact in &mut target_manifest.artifacts {
            artifact.candidate_path = None;
            artifact.candidate_blake3 = None;
        }
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: vec![StagedFile {
                relative_path: bindings.path.clone(),
                content: vec![0, 1, 2, 0xff],
            }],
            candidate_paths: vec![candidate],
        };

        for boundary in 0..=5 {
            let error = install_staged_update_with(
                &paths,
                &manifest,
                &staged,
                None,
                Some(boundary),
                || Ok(()),
            )
            .expect_err("transition must remain failed after rollback");
            assert!(error.to_string().contains("injected transition failure"));
            assert_transition_restored(
                &fixture,
                &paths,
                &bindings_path,
                &original_bindings,
                &candidate_path,
                &original_candidate,
                &original_manifest,
                &unrelated,
            );
        }

        let error = install_staged_update_with(&paths, &manifest, &staged, None, None, || {
            Err(Error::message("injected terminal validation failure"))
        })
        .expect_err("terminal validation failure must roll back");
        assert!(error.to_string().contains("terminal validation failure"));
        assert_transition_restored(
            &fixture,
            &paths,
            &bindings_path,
            &original_bindings,
            &candidate_path,
            &original_candidate,
            &original_manifest,
            &unrelated,
        );
    }

    #[test]
    fn rollback_restores_the_original_symbolic_submodule_checkout() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        run_git(&paths.box2d(), &["checkout", "-b", "original-work"]);
        let original_checkout = checkout_state(&paths.box2d()).expect("symbolic checkout");
        assert_eq!(
            original_checkout.symbolic_ref.as_deref(),
            Some("refs/heads/original-work")
        );
        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = fixture.next_revision.clone();
        target_manifest.next_revision = None;
        target_manifest.source_inventory = target_manifest
            .next_inventory
            .take()
            .expect("target inventory");
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: Vec::new(),
            candidate_paths: Vec::new(),
        };

        let error =
            install_staged_update_with(&paths, &manifest, &staged, None, Some(1), || Ok(()))
                .expect_err("failure after checkout must roll back");

        assert!(error.to_string().contains("injected transition failure"));
        assert_eq!(
            checkout_state(&paths.box2d()).expect("restored symbolic checkout"),
            original_checkout
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("unchanged gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn transaction_refuses_to_overwrite_ignored_content_tracked_by_the_target() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let upstream = fixture.root.join("upstream");
        let collision_path = "src/target-added.c";
        fs::write(
            upstream.join(collision_path),
            "int target_added(void) { return 5; }\n",
        )
        .expect("target collision fixture");
        run_git(&upstream, &["add", "-f", collision_path]);
        run_git(
            &upstream,
            &["commit", "-m", "track previously ignored path"],
        );
        let target_revision = git_output(&upstream, ["rev-parse", "HEAD"])
            .expect("target collision revision")
            .trim()
            .to_owned();
        run_git(&paths.box2d(), &["fetch", "origin"]);

        let exclude = git_output(&paths.box2d(), ["rev-parse", "--git-path", "info/exclude"])
            .expect("submodule exclude path");
        let exclude = PathBuf::from(exclude.trim());
        let exclude = if exclude.is_absolute() {
            exclude
        } else {
            paths.box2d().join(exclude)
        };
        let mut exclusions = fs::read_to_string(&exclude).expect("submodule exclusions");
        exclusions.push_str("\nsrc/target-added.c\n");
        fs::write(&exclude, exclusions).expect("ignore collision path");
        let user_content = "user-owned ignored content\n";
        fs::write(paths.box2d().join(collision_path), user_content).expect("ignored user content");
        assert!(
            git_output(
                &paths.box2d(),
                ["status", "--porcelain=v1", "--untracked-files=all"]
            )
            .expect("ignored worktree status")
            .trim()
            .is_empty(),
            "the regression requires a path hidden from the clean preflight"
        );

        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = target_revision;
        target_manifest.next_revision = None;
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: Vec::new(),
            candidate_paths: Vec::new(),
        };

        let error = install_staged_update_with(&paths, &manifest, &staged, None, None, || {
            panic!("terminal validation must not run after a checkout collision")
        })
        .expect_err("ignored user content must block the target checkout");

        assert!(error.to_string().contains("checkout exact revision"));
        assert_eq!(
            fs::read_to_string(paths.box2d().join(collision_path))
                .expect("preserved ignored user content"),
            user_content
        );
        assert_eq!(
            checkout_state(&paths.box2d())
                .expect("preserved checkout")
                .revision,
            fixture.active_revision
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("preserved gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn transaction_checkout_preserves_dirty_tracked_content() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let upstream = fixture.root.join("upstream");
        fs::write(upstream.join("src/a.c"), "int a(void) { return 9; }\n")
            .expect("target tracked change");
        run_git(&upstream, &["add", "src/a.c"]);
        run_git(&upstream, &["commit", "-m", "change tracked source"]);
        let target_revision = git_output(&upstream, ["rev-parse", "HEAD"])
            .expect("dirty collision target")
            .trim()
            .to_owned();
        run_git(&paths.box2d(), &["fetch", "origin"]);
        let user_content = "user-owned dirty tracked content\n";
        fs::write(paths.box2d().join("src/a.c"), user_content).expect("dirty user content");

        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = target_revision;
        target_manifest.next_revision = None;
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: Vec::new(),
            candidate_paths: Vec::new(),
        };

        let error = install_staged_update_with(&paths, &manifest, &staged, None, None, || {
            panic!("terminal validation must not run after a dirty checkout collision")
        })
        .expect_err("dirty tracked content must block target checkout");

        assert!(error.to_string().contains("checkout exact revision"));
        assert_eq!(
            fs::read_to_string(paths.box2d().join("src/a.c"))
                .expect("preserved dirty user content"),
            user_content
        );
        assert_eq!(
            checkout_state(&paths.box2d())
                .expect("preserved dirty checkout")
                .revision,
            fixture.active_revision
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("preserved dirty gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn terminal_concurrent_states_are_preserved_instead_of_rolled_back() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let manifest = fixture.commit_target_candidates(&manifest);
        let bindings = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact");
        let bindings_path = paths.root().join(&bindings.path);
        let candidate = manifest
            .artifact(ArtifactKind::ApiContract)
            .expect("API artifact")
            .candidate_path
            .clone()
            .expect("API candidate");
        let candidate_path = paths.root().join(&candidate);
        let manifest_path = paths.upstream_manifest();
        let user_bindings = b"concurrent bindings state\n";
        let user_candidate = b"concurrent candidate state\n";
        let user_manifest = b"concurrent manifest state\n";
        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = fixture.next_revision.clone();
        target_manifest.next_revision = None;
        for artifact in &mut target_manifest.artifacts {
            artifact.candidate_path = None;
            artifact.candidate_blake3 = None;
        }
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: vec![StagedFile {
                relative_path: bindings.path.clone(),
                content: b"transaction bindings state\n".to_vec(),
            }],
            candidate_paths: vec![candidate],
        };
        let mut concurrent_revision = None;

        let error = install_staged_update_with(&paths, &manifest, &staged, None, None, || {
            fs::write(&bindings_path, user_bindings)
                .map_err(|source| Error::io(&bindings_path, source))?;
            fs::write(&candidate_path, user_candidate)
                .map_err(|source| Error::io(&candidate_path, source))?;
            fs::write(&manifest_path, user_manifest)
                .map_err(|source| Error::io(&manifest_path, source))?;
            configure_git(&paths.box2d());
            let concurrent_source = paths.box2d().join("src/concurrent.c");
            fs::write(&concurrent_source, "int concurrent(void) { return 3; }\n")
                .map_err(|source| Error::io(&concurrent_source, source))?;
            run_git(&paths.box2d(), &["add", "."]);
            run_git(&paths.box2d(), &["commit", "-m", "concurrent"]);
            let revision = git_output(&paths.box2d(), ["rev-parse", "HEAD"])?
                .trim()
                .to_owned();
            set_indexed_gitlink(paths.root(), &revision)?;
            concurrent_revision = Some(revision);
            Err(Error::message("injected terminal validation failure"))
        })
        .expect_err("terminal failure with concurrent states must report conflicts");

        let concurrent_revision = concurrent_revision.expect("concurrent revision");
        let message = error.to_string();
        assert!(message.contains("rollback also failed"));
        assert!(message.contains("rollback conflict"));
        assert_eq!(fs::read(&bindings_path).expect("bindings"), user_bindings);
        assert_eq!(
            fs::read(&candidate_path).expect("candidate"),
            user_candidate
        );
        assert_eq!(fs::read(&manifest_path).expect("manifest"), user_manifest);
        assert_eq!(
            indexed_gitlink(paths.root()).expect("concurrent gitlink"),
            concurrent_revision
        );
        assert_eq!(
            checkout_state(&paths.box2d())
                .expect("concurrent checkout")
                .revision,
            concurrent_revision
        );
    }

    #[test]
    fn terminal_dirty_submodule_state_is_not_overwritten_by_checkout_rollback() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = fixture.next_revision.clone();
        target_manifest.next_revision = None;
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: Vec::new(),
            candidate_paths: Vec::new(),
        };
        let concurrent_source = paths.box2d().join("src/user-edit.c");

        let error = install_staged_update_with(&paths, &manifest, &staged, None, None, || {
            fs::write(&concurrent_source, "int user_edit(void) { return 4; }\n")
                .map_err(|source| Error::io(&concurrent_source, source))?;
            Err(Error::message("injected terminal validation failure"))
        })
        .expect_err("dirty checkout must be preserved");

        assert!(error.to_string().contains("submodule checkout"));
        assert_eq!(
            checkout_state(&paths.box2d())
                .expect("dirty checkout")
                .revision,
            fixture.next_revision
        );
        assert_eq!(
            fs::read_to_string(&concurrent_source).expect("preserved user edit"),
            "int user_edit(void) { return 4; }\n"
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("rolled back gitlink"),
            fixture.active_revision
        );
    }

    #[test]
    fn quarantined_candidate_cas_restores_the_captured_concurrent_file() {
        let fixture = TemporaryWorkspace::create();
        let path = fixture.workspace.join("candidate.toml");
        fs::write(&path, "reviewed baseline\n").expect("baseline candidate");
        let backup = FileBackup::capture(path.clone()).expect("candidate backup");
        fs::write(&path, "concurrent replacement\n").expect("concurrent replacement");
        let mut removed = RemovedFile::capture(path.clone()).expect("atomic quarantine");

        let rollback_error = removed.rollback(&backup);

        assert!(rollback_error.is_none());
        assert_eq!(
            fs::read_to_string(&path).expect("restored concurrent candidate"),
            "concurrent replacement\n"
        );
    }

    #[test]
    fn managed_replacement_noclobber_preserves_concurrent_and_original_states() {
        let fixture = TemporaryWorkspace::create();
        let path = fixture.workspace.join("managed.txt");
        fs::write(&path, "original\n").expect("original managed file");
        let backup = FileBackup::capture(path.clone()).expect("managed backup");
        let mut replacement = ManagedReplacement::capture(path.clone(), b"transaction\n".to_vec())
            .expect("replacement capture");
        replacement
            .validate_original(&backup)
            .expect("captured baseline");
        let quarantine_root = replacement
            .original
            .as_ref()
            .and_then(|original| original.directory.as_ref())
            .expect("original quarantine")
            .path()
            .to_owned();
        fs::write(&path, "concurrent\n").expect("concurrent recreation");

        let install_error = replacement
            .install()
            .expect_err("noclobber install must reject concurrent recreation");
        let rollback_error = replacement
            .rollback(&backup)
            .expect("both states require a reported rollback conflict");

        assert!(install_error.to_string().contains("refusing to overwrite"));
        assert!(rollback_error.contains("preserving both states"));
        assert_eq!(
            fs::read_to_string(&path).expect("concurrent state"),
            "concurrent\n"
        );
        assert_eq!(
            fs::read_to_string(quarantine_root.join("removed")).expect("original quarantine"),
            "original\n"
        );
        fs::remove_dir_all(quarantine_root).expect("quarantine cleanup");
    }

    #[test]
    fn managed_replacement_propagates_quarantined_permission_lookup_failures() {
        let fixture = TemporaryWorkspace::create();
        let path = fixture.workspace.join("managed.txt");
        fs::write(&path, "original\n").expect("original managed file");
        let backup = FileBackup::capture(path.clone()).expect("managed backup");
        let mut replacement = ManagedReplacement::capture(path.clone(), b"transaction\n".to_vec())
            .expect("replacement capture");
        let original = replacement.original.as_mut().expect("original quarantine");
        let quarantine_path = original.quarantine_path.clone();
        original.quarantine_path = quarantine_path.with_file_name("missing-quarantine-file");

        let error = replacement
            .install()
            .expect_err("permission lookup failure must abort installation");

        assert!(error.to_string().contains("missing-quarantine-file"));
        assert!(
            !path.exists(),
            "failed installation must not create a target"
        );
        replacement
            .original
            .as_mut()
            .expect("original quarantine")
            .quarantine_path = quarantine_path;
        assert!(replacement.rollback(&backup).is_none());
        assert_eq!(
            fs::read_to_string(path).expect("restored original"),
            "original\n"
        );
    }

    #[test]
    fn quarantine_finalize_propagates_cleanup_failures() {
        let fixture = TemporaryWorkspace::create();
        let path = fixture.workspace.join("managed.txt");
        fs::write(&path, "original\n").expect("managed file");
        let mut removed = RemovedFile::capture(path).expect("quarantine");
        let mut kept = None;

        let error = removed
            .finalize_with(|directory| {
                kept = Some(directory.keep());
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected remove failure",
                ))
            })
            .expect_err("cleanup failure must be reported");

        assert!(error.contains("injected remove failure"));
        fs::remove_dir_all(kept.expect("kept failed quarantine"))
            .expect("failed quarantine cleanup");
    }

    #[test]
    fn successful_install_reports_cleanup_failure_without_rolling_back() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let manifest = fixture.commit_target_candidates(&manifest);
        let bindings = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact");
        let candidate = manifest
            .artifact(ArtifactKind::ApiContract)
            .expect("API artifact")
            .candidate_path
            .clone()
            .expect("API candidate");
        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = fixture.next_revision.clone();
        target_manifest.next_revision = None;
        target_manifest.source_inventory = target_manifest
            .next_inventory
            .take()
            .expect("target inventory");
        for artifact in &mut target_manifest.artifacts {
            artifact.candidate_path = None;
            artifact.candidate_blake3 = None;
        }
        let installed_bindings = b"transaction bindings state\n".to_vec();
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: vec![StagedFile {
                relative_path: bindings.path.clone(),
                content: installed_bindings.clone(),
            }],
            candidate_paths: vec![candidate.clone()],
        };
        let mut kept_quarantine = None;

        let error = install_staged_update_with_finalize(
            &paths,
            &manifest,
            &staged,
            None,
            None,
            || Ok(()),
            |progress| {
                progress.finalize_with(|directory| {
                    if kept_quarantine.is_none() {
                        kept_quarantine = Some(directory.keep());
                        Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "injected quarantine cleanup failure",
                        ))
                    } else {
                        directory.close()
                    }
                })
            },
        )
        .expect_err("cleanup failure must be reported after installation");

        let message = error.to_string();
        assert!(message.contains("installed successfully but quarantine cleanup failed"));
        assert!(message.contains("injected quarantine cleanup failure"));
        assert_eq!(
            fs::read(paths.root().join(&bindings.path)).expect("installed bindings"),
            installed_bindings
        );
        assert!(!paths.root().join(candidate).exists());
        assert_eq!(
            UpstreamManifest::load(&paths)
                .expect("installed manifest")
                .active_revision,
            fixture.next_revision
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("installed gitlink"),
            fixture.next_revision
        );
        fs::remove_dir_all(kept_quarantine.expect("retained failed quarantine"))
            .expect("retained quarantine cleanup");
    }

    #[test]
    fn managed_artifact_write_transaction_updates_manifest_last_and_rolls_back_together() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let wire_path = paths
            .root()
            .join("xtask/tests/fixtures/recording_wire.toml");
        let report_path = paths.root().join("docs/api-coverage.md");
        let manifest_path = paths.upstream_manifest();
        let original_wire = fs::read(&wire_path).expect("original recording wire");
        let original_report = fs::read(&report_path).expect("original report");
        let original_manifest = fs::read(&manifest_path).expect("original manifest");
        let wire = format!("upstream_sha = \"{}\"\n", fixture.next_revision).into_bytes();
        let report = format!(
            "Pinned active upstream: `{}`.\ngenerated report\n",
            fixture.active_revision
        )
        .into_bytes();
        let writes = vec![
            ManagedArtifactWrite::active("recording-wire", wire.clone()),
            ManagedArtifactWrite::active("api-coverage-report", report.clone()),
        ];

        let error = install_managed_artifact_writes(&paths, &writes, || {
            Err(Error::message(
                "injected generated-output validation failure",
            ))
        })
        .expect_err("terminal failure must roll back outputs and manifest");

        assert!(
            error
                .to_string()
                .contains("generated-output validation failure")
        );
        assert_eq!(
            fs::read(&wire_path).expect("rolled back wire"),
            original_wire
        );
        assert_eq!(
            fs::read(&report_path).expect("rolled back report"),
            original_report
        );
        assert_eq!(
            fs::read(&manifest_path).expect("rolled back manifest"),
            original_manifest
        );

        install_managed_artifact_writes(&paths, &writes, || Ok(()))
            .expect("generated outputs transaction");

        assert_eq!(fs::read(&wire_path).expect("installed wire"), wire);
        assert_eq!(fs::read(&report_path).expect("installed report"), report);
        let installed = UpstreamManifest::load(&paths).expect("updated manifest");
        assert_eq!(
            installed
                .artifact(ArtifactKind::RecordingWire)
                .expect("wire artifact")
                .content_blake3,
            file_blake3(&wire_path).expect("wire digest")
        );
        assert_eq!(
            installed
                .artifact(ArtifactKind::ApiCoverageReport)
                .expect("report artifact")
                .content_blake3,
            file_blake3(&report_path).expect("report digest")
        );
    }

    #[test]
    fn artifact_digest_bootstrap_rejects_mixed_or_partial_output_sets() {
        let mixed = TemporaryWorkspace::create();
        let mixed_paths = mixed.paths();
        let mut mixed_manifest = mixed.manifest();
        mixed_manifest.artifacts[0].content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
        fs::write(
            mixed_paths.upstream_manifest(),
            render_toml(&mixed_manifest).expect("mixed manifest"),
        )
        .expect("write mixed manifest");
        let error = install_managed_artifact_writes(
            &mixed_paths,
            &[ManagedArtifactWrite::active(
                "recording-wire",
                fs::read(
                    mixed_paths
                        .root()
                        .join("xtask/tests/fixtures/recording_wire.toml"),
                )
                .expect("wire fixture"),
            )],
            || Ok(()),
        )
        .expect_err("mixed initialized and zero digests must fail closed");
        assert!(
            error
                .to_string()
                .contains("initialized artifact digest manifest cannot contain zero")
        );

        let partial = TemporaryWorkspace::create();
        let partial_paths = partial.paths();
        let mut zero_manifest = partial.manifest();
        zero_manifest.artifact_digests_initialized = false;
        for artifact in &mut zero_manifest.artifacts {
            artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
        }
        fs::write(
            partial_paths.upstream_manifest(),
            render_toml(&zero_manifest).expect("zero manifest"),
        )
        .expect("write zero manifest");
        let error = install_managed_artifact_writes(
            &partial_paths,
            &[ManagedArtifactWrite::reviewed_active(
                "api-contract",
                fs::read(
                    partial_paths
                        .root()
                        .join("boxdd/tests/fixtures/api_contract.toml"),
                )
                .expect("API fixture"),
            )],
            || Ok(()),
        )
        .expect_err("partial bootstrap output set must fail closed");
        assert!(
            error
                .to_string()
                .contains("must write exactly every reviewed active and api-coverage artifact")
        );
    }

    #[test]
    fn artifact_digest_bootstrap_validates_repository_inputs_before_installing() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let mut manifest = fixture.manifest();
        manifest.artifact_digests_initialized = false;
        for artifact in &mut manifest.artifacts {
            artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
        }
        manifest.recording_inputs[0].blake3 = "f".repeat(64);
        fs::write(
            paths.upstream_manifest(),
            render_toml(&manifest).expect("uninitialized manifest"),
        )
        .expect("write uninitialized manifest");
        let writes = exact_bootstrap_writes(&paths);
        let observed_paths = writes
            .iter()
            .map(|write| {
                let artifact = manifest
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.name == write.artifact_name)
                    .expect("bootstrap artifact");
                paths.root().join(&artifact.path)
            })
            .chain(std::iter::once(paths.upstream_manifest()))
            .collect::<Vec<_>>();
        let before = observed_paths
            .iter()
            .map(|path| (path.clone(), fs::read(path).expect("bootstrap baseline")))
            .collect::<BTreeMap<_, _>>();

        let error = install_managed_artifact_writes(&paths, &writes, || Ok(()))
            .expect_err("forged reviewed recording identity must fail before installation");

        assert!(error.to_string().contains("reviewed recording input"));
        for (path, content) in before {
            assert_eq!(fs::read(path).expect("unchanged bootstrap path"), content);
        }
    }

    #[test]
    fn artifact_digest_bootstrap_refuses_dirty_generated_outputs() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let mut manifest = fixture.manifest();
        manifest.artifact_digests_initialized = false;
        for artifact in &mut manifest.artifacts {
            artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
        }
        fs::write(
            paths.upstream_manifest(),
            render_toml(&manifest).expect("uninitialized manifest"),
        )
        .expect("write uninitialized manifest");
        let report = paths.root().join("docs/api-coverage.md");
        let dirty = b"Pinned active upstream: dirty user edit\n".to_vec();
        fs::write(&report, &dirty).expect("dirty generated report");
        let writes = exact_bootstrap_writes(&paths);

        let error = install_managed_artifact_writes(&paths, &writes, || Ok(()))
            .expect_err("bootstrap must preserve dirty generated output");

        assert!(
            error
                .to_string()
                .contains("refuses dirty generated artifacts")
        );
        assert_eq!(fs::read(report).expect("preserved dirty report"), dirty);
    }

    #[test]
    fn artifact_digest_bootstrap_rejects_wrong_destinations_and_duplicates() {
        for mutation in ["wrong-destination", "duplicate"] {
            let fixture = TemporaryWorkspace::create();
            let paths = fixture.paths();
            let mut manifest = fixture.manifest();
            manifest.artifact_digests_initialized = false;
            for artifact in &mut manifest.artifacts {
                artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
            }
            fs::write(
                paths.upstream_manifest(),
                render_toml(&manifest).expect("uninitialized manifest"),
            )
            .expect("write uninitialized manifest");
            let original_manifest = fs::read(paths.upstream_manifest()).expect("manifest baseline");
            let mut writes = exact_bootstrap_writes(&paths);
            match mutation {
                "wrong-destination" => {
                    writes[0].destination = ManagedArtifactDestination::Active;
                }
                "duplicate" => writes.push(writes[0].clone()),
                _ => unreachable!("closed mutation table"),
            }

            let error = install_managed_artifact_writes(&paths, &writes, || Ok(()))
                .expect_err("bootstrap output set must be exact");

            assert!(
                error
                    .to_string()
                    .contains("must write exactly every reviewed active and api-coverage artifact")
            );
            assert_eq!(
                fs::read(paths.upstream_manifest()).expect("unchanged manifest"),
                original_manifest
            );
        }
    }

    #[test]
    fn artifact_digest_bootstrap_cannot_bypass_the_internal_contract_gate() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let mut manifest = fixture.manifest();
        manifest.artifact_digests_initialized = false;
        for artifact in &mut manifest.artifacts {
            artifact.content_blake3 = UNINITIALIZED_BLAKE3.to_owned();
        }
        fs::write(
            paths.upstream_manifest(),
            render_toml(&manifest).expect("uninitialized manifest"),
        )
        .expect("write uninitialized manifest");
        let writes = exact_bootstrap_writes(&paths);
        let observed_paths = writes
            .iter()
            .map(|write| {
                let artifact = manifest
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.name == write.artifact_name)
                    .expect("bootstrap artifact");
                paths.root().join(&artifact.path)
            })
            .chain(std::iter::once(paths.upstream_manifest()))
            .collect::<Vec<_>>();
        let before = observed_paths
            .iter()
            .map(|path| (path.clone(), fs::read(path).expect("bootstrap baseline")))
            .collect::<BTreeMap<_, _>>();
        let terminal_called = std::cell::Cell::new(false);

        install_managed_artifact_writes(&paths, &writes, || {
            terminal_called.set(true);
            Ok(())
        })
        .expect_err("minimal fixture contract must fail the internal API gate");

        assert!(
            !terminal_called.get(),
            "the caller terminal must run only after the mandatory bootstrap gate"
        );
        for (path, content) in before {
            assert_eq!(fs::read(path).expect("rolled back bootstrap path"), content);
        }
    }

    fn exact_bootstrap_writes(paths: &WorkspacePaths) -> Vec<ManagedArtifactWrite> {
        let manifest = UpstreamManifest::load(paths).expect("bootstrap manifest");
        [
            ("api-contract", ManagedArtifactDestination::ReviewedActive),
            ("recording-wire", ManagedArtifactDestination::Active),
            ("api-coverage-report", ManagedArtifactDestination::Active),
        ]
        .into_iter()
        .map(|(name, destination)| {
            let artifact = manifest
                .artifacts
                .iter()
                .find(|artifact| artifact.name == name)
                .expect("bootstrap artifact");
            ManagedArtifactWrite {
                artifact_name: name.to_owned(),
                destination,
                content: fs::read(paths.root().join(&artifact.path))
                    .expect("bootstrap artifact content"),
                reviewed_baseline_blake3: None,
            }
        })
        .collect()
    }

    #[test]
    fn generated_candidate_transaction_rejects_manifest_changes_during_generation() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let report_path = paths.root().join("docs/api-coverage.md");
        let original_report = fs::read(&report_path).expect("original report");
        let original_manifest = fs::read(paths.upstream_manifest()).expect("original manifest");
        let report = format!(
            "Pinned active upstream: `{}`.\nnew report\n",
            fixture.active_revision
        )
        .into_bytes();

        let error = install_managed_artifact_writes_locked(
            &paths,
            &[ManagedArtifactWrite::active("api-coverage-report", report)],
            Some(b"different generation baseline"),
            || Ok(()),
        )
        .expect_err("generation baseline mismatch must fail closed");

        assert!(
            error
                .to_string()
                .contains("changed while managed artifacts were being generated")
        );
        assert_eq!(
            fs::read(report_path).expect("unchanged report"),
            original_report
        );
        assert_eq!(
            fs::read(paths.upstream_manifest()).expect("unchanged manifest"),
            original_manifest
        );
    }

    #[test]
    fn reviewed_active_baseline_updates_reviewed_bytes_and_manifest_atomically() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let original_manifest = fixture.manifest();
        let contract_path = original_manifest
            .artifact_path(paths.root(), ArtifactKind::ApiContract)
            .expect("API contract path");
        let reviewed = format!(
            "upstream_sha = \"{}\"\nreview_state = \"accepted\"\n",
            fixture.active_revision
        )
        .into_bytes();
        fs::write(&contract_path, &reviewed).expect("manually reviewed contract");
        let reviewed_blake3 = blake3::hash(&reviewed).to_hex().to_string();
        let generated = format!(
            "upstream_sha = \"{}\"\nreview_state = \"canonical\"\n",
            fixture.active_revision
        )
        .into_bytes();

        install_managed_artifact_writes(
            &paths,
            &[ManagedArtifactWrite::reviewed_active_with_baseline_blake3(
                "api-contract",
                generated.clone(),
                reviewed_blake3,
            )],
            || Ok(()),
        )
        .expect("reviewed active transaction");

        let updated_manifest = UpstreamManifest::load(&paths).expect("updated manifest");
        assert_eq!(
            fs::read(&contract_path).expect("installed reviewed contract"),
            generated
        );
        assert_eq!(
            updated_manifest
                .artifact(ArtifactKind::ApiContract)
                .expect("API contract artifact")
                .content_blake3,
            blake3::hash(&generated).to_hex().as_str()
        );
        assert_eq!(
            updated_manifest.binding_routes,
            original_manifest.binding_routes
        );
    }

    #[test]
    fn reviewed_candidate_write_pairs_path_and_digest_atomically() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest_path = paths.upstream_manifest();
        let original_manifest = fs::read(&manifest_path).expect("original manifest");
        let candidate = "boxdd/tests/fixtures/api_contract.next.toml";
        let candidate_path = paths.root().join(candidate);
        let content = format!("upstream_sha = \"{}\"\n", fixture.next_revision).into_bytes();
        let writes = [ManagedArtifactWrite::reviewed_candidate(
            "api-contract",
            candidate,
            content.clone(),
        )];

        install_managed_artifact_writes(&paths, &writes, || {
            Err(Error::message("injected candidate review failure"))
        })
        .expect_err("candidate failure must roll back candidate and manifest");

        assert!(!candidate_path.exists());
        assert_eq!(
            fs::read(&manifest_path).expect("rolled back manifest"),
            original_manifest
        );

        install_managed_artifact_writes(&paths, &writes, || Ok(()))
            .expect("reviewed candidate transaction");

        assert_eq!(
            fs::read(&candidate_path).expect("installed candidate"),
            content
        );
        let installed = UpstreamManifest::load(&paths).expect("candidate manifest");
        let artifact = installed
            .artifact(ArtifactKind::ApiContract)
            .expect("API contract artifact");
        assert_eq!(artifact.candidate_path.as_deref(), Some(candidate));
        assert_eq!(
            artifact.candidate_blake3.as_deref(),
            Some(
                file_blake3(&candidate_path)
                    .expect("candidate digest")
                    .as_str()
            )
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_transition_restored(
        fixture: &TemporaryWorkspace,
        paths: &WorkspacePaths,
        bindings_path: &Path,
        original_bindings: &[u8],
        candidate_path: &Path,
        original_candidate: &[u8],
        original_manifest: &[u8],
        unrelated: &UnrelatedGitState,
    ) {
        assert_eq!(
            fs::read(bindings_path).expect("restored bindings"),
            original_bindings
        );
        assert_eq!(
            fs::read(candidate_path).expect("restored candidate"),
            original_candidate
        );
        assert_eq!(
            fs::read(paths.upstream_manifest()).expect("restored manifest"),
            original_manifest
        );
        assert_eq!(
            indexed_gitlink(paths.root()).expect("restored gitlink"),
            fixture.active_revision
        );
        assert_eq!(
            git_output(&paths.box2d(), ["rev-parse", "HEAD"])
                .expect("restored checkout")
                .trim(),
            fixture.active_revision
        );
        unrelated.assert_unchanged(paths.root());
    }

    #[test]
    fn successful_transition_installs_binary_artifact_and_removes_candidate() {
        let fixture = TemporaryWorkspace::create();
        let paths = fixture.paths();
        let manifest = fixture.manifest();
        let manifest = fixture.commit_target_candidates(&manifest);
        let bindings = manifest
            .binding_artifacts()
            .next()
            .expect("bindings artifact");
        let candidate = manifest
            .artifact(ArtifactKind::ApiContract)
            .expect("API artifact")
            .candidate_path
            .clone()
            .expect("API candidate");
        let mut target_manifest = manifest.clone();
        target_manifest.active_revision = fixture.next_revision.clone();
        target_manifest.next_revision = None;
        target_manifest.source_inventory = target_manifest
            .next_inventory
            .take()
            .expect("target inventory");
        for artifact in &mut target_manifest.artifacts {
            artifact.candidate_path = None;
            artifact.candidate_blake3 = None;
        }
        let staged = StagedUpdate {
            manifest: target_manifest,
            artifacts: vec![StagedFile {
                relative_path: bindings.path.clone(),
                content: vec![0, 1, 2, 0xff],
            }],
            candidate_paths: vec![candidate.clone()],
        };

        install_staged_update_with(&paths, &manifest, &staged, None, None, || Ok(()))
            .expect("successful fixture transition");

        assert_eq!(
            fs::read(paths.root().join(&bindings.path)).expect("binary artifact"),
            [0, 1, 2, 0xff]
        );
        assert!(!paths.root().join(candidate).exists());
        assert_eq!(
            indexed_gitlink(paths.root()).expect("target gitlink"),
            fixture.next_revision
        );
        assert_eq!(
            git_output(&paths.box2d(), ["rev-parse", "HEAD"])
                .expect("target checkout")
                .trim(),
            fixture.next_revision
        );
        let installed = UpstreamManifest::load(&paths).expect("installed manifest");
        assert_eq!(installed.active_revision, fixture.next_revision);
        assert!(installed.next_revision.is_none());
        assert!(
            installed
                .artifacts
                .iter()
                .all(|artifact| artifact.candidate_path.is_none())
        );
    }

    #[test]
    fn binding_discovery_is_scoped_and_rejects_ambiguity_by_contract() {
        let root = std::env::temp_dir().join(format!(
            "boxdd-upstream-bindings-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let expected = root.join("debug/build/boxdd-sys-one/out/bindings.rs");
        let stale = root.join("shared-target/debug/build/boxdd-sys-old/out/bindings.rs");
        fs::create_dir_all(expected.parent().expect("expected parent")).expect("expected dir");
        fs::create_dir_all(stale.parent().expect("stale parent")).expect("stale dir");
        fs::write(&expected, "expected").expect("expected binding");
        fs::write(&stale, "stale").expect("stale binding");
        let mut found = Vec::new();
        collect_generated_bindings(&root.join("debug"), &mut found).expect("scoped scan");
        assert_eq!(found, [expected]);
        fs::remove_dir_all(root).expect("fixture cleanup");
    }
}
