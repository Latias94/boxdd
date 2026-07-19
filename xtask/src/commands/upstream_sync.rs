use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write as _,
    path::{Component, Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, TempDir};

use crate::{
    Error, Result,
    commands::{UpdateMode, parse_update_mode},
    config::{
        UPSTREAM_MANIFEST_SCHEMA, read_toml, render_toml, write_atomic, write_atomic_bytes,
        write_new_bytes_noclobber,
    },
    paths::WorkspacePaths,
    recording_ops,
};

const BOX2D_GITLINK: &str = "boxdd-sys/third-party/box2d";
const UNINITIALIZED_BLAKE3: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
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
    "boxdd-sys/src",
    "xtask/Cargo.toml",
    "xtask/src",
    "xtask/tests",
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
        let manifest: Self = read_toml(&paths.upstream_manifest())?;
        validate_manifest(&manifest)?;
        validate_binding_route_feature_catalog(paths, &manifest.binding_routes)?;
        validate_binding_route_feature_catalog(paths, &manifest.next_binding_routes)?;
        Ok(manifest)
    }

    pub fn artifact(&self, kind: ArtifactKind) -> Result<&GeneratedArtifact> {
        if kind == ArtifactKind::Bindings {
            return Err(Error::message(
                "bindings artifacts must be selected by precision, target, and provider",
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

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    if matches!(args, [argument] if argument == "--prepare-next") {
        return prepare_next_candidate(paths);
    }
    let mode = parse_update_mode("upstream-sync", args)?;
    match mode {
        UpdateMode::Check => {
            let manifest = UpstreamManifest::load(paths)?;
            let snapshot = validate_repository(paths, &manifest, false)?;
            super::api_coverage::check(paths)?;
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

fn prepare_next_candidate(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let manifest = UpstreamManifest::load(paths)?;
    let manifest_baseline = fs::read(paths.upstream_manifest())
        .map_err(|source| Error::io(paths.upstream_manifest(), source))?;
    validate_repository(paths, &manifest, true)?;
    super::api_coverage::check(paths)?;
    let generation_baseline = GenerationBaseline::capture(paths.root())?;
    let target = manifest
        .next_revision
        .as_deref()
        .ok_or_else(|| Error::message("upstream manifest has no next_revision to prepare"))?;
    let generation =
        IsolatedGeneration::create_at(paths, &generation_baseline.repository_revision, target)?;
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
    generation_baseline.verify(paths.root())?;
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
    validate_repository(paths, &manifest, false)
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
        if artifact.kind != ArtifactKind::Bindings || artifact.producer != ArtifactProducer::Bindgen
        {
            errors.push(format!(
                "next artifact `{}` must be a bindgen-produced bindings artifact",
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
    let manifest = UpstreamManifest::load(paths)?;
    let target = manifest
        .next_revision
        .as_deref()
        .ok_or_else(|| Error::message("upstream manifest has no next_revision to apply"))?;
    validate_update_preconditions(paths, &manifest)?;
    let baseline = ManagedSnapshot::capture(paths, &manifest)?;

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
        let git_path = git_output(
            root,
            ["rev-parse", "--git-path", "boxdd-upstream-sync.lock"],
        )?;
        let git_path = PathBuf::from(git_path.trim());
        let path = if git_path.is_absolute() {
            git_path
        } else {
            root.join(git_path)
        };
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| Error::io(&path, source))?;
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
        Ok(lock)
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
            generation: GenerationBaseline::capture(paths.root())?,
        })
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
}

impl GenerationBaseline {
    fn capture(root: &Path) -> Result<Self> {
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
        if !dirty.trim().is_empty() {
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
        let actual = Self::capture(root)?;
        if actual.input_tree != self.input_tree {
            return Err(Error::message(
                "upstream generator input identities changed during generation",
            ));
        }
        Ok(())
    }
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

#[derive(Clone, Debug)]
#[doc(hidden)]
pub enum ManagedArtifactDestination {
    Active,
    ReviewedActive,
    ReviewedCandidate { path: String },
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct ManagedArtifactWrite {
    pub artifact_name: String,
    pub destination: ManagedArtifactDestination,
    pub content: Vec<u8>,
}

impl ManagedArtifactWrite {
    pub fn active(artifact_name: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            artifact_name: artifact_name.into(),
            destination: ManagedArtifactDestination::Active,
            content,
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
        }
    }

    pub fn reviewed_active(artifact_name: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            artifact_name: artifact_name.into(),
            destination: ManagedArtifactDestination::ReviewedActive,
            content,
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
                };
                (write.artifact_name.as_str(), destination)
            })
            .collect::<BTreeSet<_>>();
        if actual != expected || actual.len() != writes.len() {
            return Err(Error::message(
                "artifact digest bootstrap must write exactly every reviewed active and api-coverage artifact with the matching destination",
            ));
        }
        reject_bootstrap_artifact_changes_if_present(paths, &original_manifest)?;
        validate_repository_without_artifact_digests(paths, &original_manifest)?;
    } else {
        validate_artifact_identities(paths, &original_manifest)?;
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
        let digest = blake3::hash(&write.content).to_hex().to_string();
        let relative_path = match &write.destination {
            ManagedArtifactDestination::Active => {
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
                if artifact.producer != ArtifactProducer::Reviewed {
                    return Err(Error::message(format!(
                        "active artifact `{}` is produced by {}, not reviewed",
                        artifact.name,
                        artifact.producer.as_str()
                    )));
                }
                let path = artifact.path.clone();
                baseline_digests.insert(path.clone(), Some(artifact.content_blake3.clone()));
                artifact.content_blake3 = digest;
                path
            }
            ManagedArtifactDestination::ReviewedCandidate { path } => {
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
    if let Some(artifact) = manifest.artifacts.iter().find(|artifact| {
        matches!(
            artifact.producer,
            ArtifactProducer::AbiProbe | ArtifactProducer::ProviderAttestation
        )
    }) {
        return Err(Error::message(format!(
            "artifact digest bootstrap has no reproducible generator for target-native artifact `{}` produced by {}; bootstrap refuses to hash-and-bless it",
            artifact.name,
            artifact.producer.as_str()
        )));
    }
    let baseline = GenerationBaseline::capture(paths.root())?;
    let generation = IsolatedGeneration::create_at(
        paths,
        &baseline.repository_revision,
        &manifest.active_revision,
    )?;
    let validation = (|| {
        generation.generate_bindings(manifest)?;
        compare_binding_artifacts(paths.root(), &generation.worktree, manifest)
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
    source_root: PathBuf,
    worktree: PathBuf,
    target_dir: PathBuf,
    repository_worktree_added: bool,
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
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| Error::message(format!("system clock is before Unix epoch: {error}")))?
            .as_nanos();
        let source_root = std::env::temp_dir().join(format!(
            "boxdd-upstream-sync-{}-{nonce}",
            std::process::id()
        ));
        let worktree = source_root.join("workspace");
        let target_dir = source_root.join("cargo-target");
        fs::create_dir_all(&source_root).map_err(|source| Error::io(&source_root, source))?;
        let mut generation = Self {
            repository_root: paths.root().to_owned(),
            source_root,
            worktree,
            target_dir,
            repository_worktree_added: false,
        };
        command_success(
            Command::new("git")
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
            Command::new("git")
                .args(["clone", "--no-hardlinks", "--no-checkout"])
                .arg(paths.box2d())
                .arg(&isolated_submodule),
            "clone local Box2D object store into isolated worktree",
        )?;
        checkout_detached(&isolated_submodule, revision)?;
        Ok(generation)
    }

    fn prepare_update(&self, manifest: &UpstreamManifest, target: &str) -> Result<StagedUpdate> {
        let mut target_manifest = manifest.promoted_for_generation(target)?;

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

        write_atomic(
            &self.worktree.join("boxdd-sys/upstream.toml"),
            &render_toml(&target_manifest)?,
        )?;
        set_indexed_gitlink(&self.worktree, target)?;
        self.generate_bindings(&target_manifest)?;
        if let Some(artifact) = target_manifest.artifacts.iter().find(|artifact| {
            matches!(
                artifact.kind,
                ArtifactKind::AbiMetadata | ArtifactKind::ProviderIdentity
            )
        }) {
            return Err(Error::message(format!(
                "artifact `{}` requires its target-native {:?} generator before upstream-sync may stage an update",
                artifact.name, artifact.producer
            )));
        }

        let isolated_paths = WorkspacePaths::new(&self.worktree);
        let generated = super::api_coverage::render_generated_outputs(&isolated_paths)?;
        let recording_wire =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::RecordingWire)?;
        let report =
            target_manifest.artifact_path(&self.worktree, ArtifactKind::ApiCoverageReport)?;
        write_atomic_bytes(&recording_wire, &generated.recording_wire)?;
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
        super::api_coverage::render_refreshed_contract_candidate(&isolated_paths)
    }

    fn generate_bindings(&self, manifest: &UpstreamManifest) -> Result<()> {
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
            if !matches!(
                artifact.target,
                ArtifactTarget::Universal | ArtifactTarget::Native
            ) {
                return Err(Error::message(format!(
                    "bindings generation for target {:?} is not implemented for artifact `{}`",
                    artifact.target, artifact.name
                )));
            }
            let mut command = Command::new("cargo");
            command
                .current_dir(&self.worktree)
                .args(binding_generation_cargo_args(rust_target, features))
                .env("CARGO_TARGET_DIR", &artifact_target)
                .env("BOXDD_SYS_SKIP_CC", "1")
                .env("BOXDD_SYS_FORCE_BINDGEN", "1")
                .env("BOXDD_SYS_BINDGEN_TARGET", rust_target.as_str());
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
                    binding_provenance(artifact, &manifest.active_revision, rust_target)
                ),
            )?;
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
        let registration = worktree_is_registered(&self.repository_root, &self.worktree)
            .map_err(|error| Error::message(format!("inspect isolated worktree: {error}")));
        self.cleanup_after_inspection(registration)
    }

    fn cleanup_after_inspection(&mut self, registration: Result<bool>) -> Result<()> {
        let mut errors = Vec::new();
        let should_attempt_registered_removal = match registration {
            Ok(registered) => registered,
            Err(error) => {
                errors.push(error.to_string());
                true
            }
        };
        if should_attempt_registered_removal {
            if let Err(error) = command_success(
                Command::new("git")
                    .current_dir(&self.repository_root)
                    .args(["worktree", "remove", "--force"])
                    .arg(&self.worktree),
                "remove isolated repository worktree",
            ) {
                errors.push(error.to_string());
            } else {
                self.repository_worktree_added = false;
            }
        } else {
            self.repository_worktree_added = false;
        }
        if self.source_root.exists() {
            if let Err(source) = fs::remove_dir_all(&self.source_root) {
                errors.push(Error::io(&self.source_root, source).to_string());
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
) -> String {
    let precision = artifact.precision.map(Precision::as_str).unwrap_or("none");
    format!(
        "// AUTOGENERATED: pregenerated bindings for docs.rs/offline builds\n\
// boxdd-upstream-revision: {revision}\n\
// boxdd-artifact-name: {}\n\
// boxdd-artifact-precision: {precision}\n\
// boxdd-artifact-target: {}\n\
// boxdd-artifact-provider: {}\n\
// boxdd-artifact-producer: {}\n\
// boxdd-artifact-rust-target: {}\n\
// Authority: boxdd-sys/upstream.toml\n\
// Refresh with: cargo run -p xtask -- upstream-sync --write\n",
        artifact.name,
        artifact.target.as_str(),
        artifact.provider.as_str(),
        artifact.producer.as_str(),
        rust_target.as_str(),
    )
}

fn validate_binding_identity(
    path: &Path,
    artifact: &GeneratedArtifact,
    manifest: &UpstreamManifest,
) -> Result<()> {
    let source = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let expected = binding_provenance(
        artifact,
        &manifest.active_revision,
        binding_generation_target(manifest, artifact)?,
    );
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
    let output = Command::new("git")
        .current_dir(repository)
        .args(["cat-file", "blob", &object])
        .env("GIT_OPTIONAL_LOCKS", "0")
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
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
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
    let mut command = Command::new("git");
    command
        .current_dir(paths.root())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["status", "--porcelain=v1", "--untracked-files=all", "--"]);
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
        Command::new("git")
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
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
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
        Command::new("git").current_dir(root).args([
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
        Command::new("git")
            .current_dir(repository)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args(["cat-file", "-e", &object]),
        &format!("verify commit object {revision}"),
    )
}

fn checkout_detached(repository: &Path, revision: &str) -> Result<()> {
    command_success(
        Command::new("git").current_dir(repository).args([
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
    let output = Command::new("git")
        .current_dir(repository)
        .env("GIT_OPTIONAL_LOCKS", "0")
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
        Command::new("git").current_dir(repository).args([
            "checkout",
            "--no-overwrite-ignore",
            branch,
        ]),
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

fn git_output<const N: usize>(repository: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .output()
        .map_err(|source| Error::io("git", source))?;
    output_text(output, "git")
}

fn git_output_with_paths(repository: &Path, args: &[&str], paths: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .env("GIT_OPTIONAL_LOCKS", "0")
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
                Command::new("git")
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
                        )
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
                    ArtifactKind::AbiMetadata | ArtifactKind::ProviderIdentity => {
                        unreachable!("fixture does not declare optional artifacts")
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
            Command::new("git").current_dir(repository).args(args),
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
    fn target_manifest_matches_exact_target_commit_inventory() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let paths = WorkspacePaths::new(root);
        let manifest = UpstreamManifest::load(&paths).expect("repository manifest");
        let observed_active = source_inventory(&paths.box2d(), &manifest.active_revision)
            .expect("repository active inventory");
        validate_exact_inventory(&manifest.source_inventory, &observed_active)
            .expect("exact active inventory");
        let next_revision = manifest.next_revision.as_deref().expect("next revision");
        let observed_next =
            source_inventory(&paths.box2d(), next_revision).expect("repository target inventory");
        validate_exact_inventory(
            manifest.next_inventory.as_ref().expect("next inventory"),
            &observed_next,
        )
        .expect("exact target inventory");
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

        let mut unsupported = manifest.clone();
        let mut abi_artifact = artifact.clone();
        abi_artifact.name = "abi-single".to_owned();
        abi_artifact.kind = ArtifactKind::AbiMetadata;
        abi_artifact.path = "boxdd-sys/abi-single.toml".to_owned();
        abi_artifact.target = ArtifactTarget::Native;
        abi_artifact.provider = ArtifactProvider::Source;
        abi_artifact.producer = ArtifactProducer::AbiProbe;
        unsupported.artifacts.push(abi_artifact);
        let error = validate_bootstrap_bindings(&fixture.paths(), &unsupported)
            .expect_err("unsupported native artifacts must not be hash-and-blessed");
        assert!(error.to_string().contains("no reproducible generator"));
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
    fn isolated_cleanup_does_not_prune_unrelated_worktree_registrations() {
        let fixture = TemporaryWorkspace::create();
        let unrelated = fixture.root.join("unrelated-worktree");
        command_success(
            Command::new("git")
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
