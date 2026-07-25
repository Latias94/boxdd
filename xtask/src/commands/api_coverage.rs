use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    process::Command,
};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    abi_contract::{
        ABI_BINDING_EVIDENCE_ID, ABI_HEADER_EVIDENCE_ID, ABI_VALIDATOR_EVIDENCE_ID,
        AbiBindingIndex, AbiBindingIndexes, AbiBindingRoute, AbiBindingRoutes, AbiContract,
        AbiFunctionSymbols, AbiPrecisionInventories, AbiRustIndexes, AbiValidationContext,
        bootstrap_legacy_precision_proofs, discard_unproven_reviewed_exposure,
        map_precision_inventory, preserve_reviewed_exposure, promote_proven_deferred_exposure,
        validate as validate_abi, validate_reviewed_deferred_migration_invariant,
    },
    c_api::{CAbiPrecision, CApiInventory, parse_headers, parse_headers_for_precision},
    commands::upstream_sync::{
        ArtifactKind, ArtifactTarget, GeneratedArtifact, ManagedArtifactWrite, RustTarget,
        UpdateLock, UpstreamManifest, canonical_binding_routes, canonical_route_binding_artifacts,
        expanded_binding_route_features, install_managed_artifact_writes_locked,
        reviewed_recording_operations_source, validate_repository,
    },
    commands::{UpdateMode, parse_update_mode},
    config::{API_CONTRACT_SCHEMA, read_toml, render_toml},
    paths::WorkspacePaths,
    recording_ops::parse as parse_recording_ops,
    recording_wire::{
        RecordingWireContract, generate_wire_contract, render_runtime_parser,
        reviewed_sources_aggregate_blake3, validate_wire_contract,
    },
    rust_index::{
        RustFfiTypeHints, RustIndex, RustIndexCoordinate, TestEvidenceIndex,
        discover_test_evidence_items, index_boxdd_routes_with_ffi_hints,
        index_test_evidence_for_gate_at_coordinate,
    },
    source_overlay::effective_source_identity,
};

const AVAILABILITY: &[&str] = &[
    "always",
    "debug-profile",
    "assertions-enabled",
    "validation-enabled",
];
const API_CLASSIFICATION_EVIDENCE_ID: &str = "api-classification-validator";
const BOX2D_3_2_TARGET_REVISION: &str = "56edae79f2949d86142b03450d5d60f63bcf5a6f";
pub(crate) const RUNTIME_RECORDING_WIRE_PATH: &str = "boxdd/src/generated/recording_wire.rs";
const BOX2D_3_2_EXPORTED_FUNCTION_COUNT: usize = 478;
const BOX2D_3_2_LOGICAL_FUNCTIONS_BLAKE3: &str =
    "73314b5a229524dc25da96da268e950e57ebe7f3701da743057a10525c5a410d";
const BOX2D_3_2_SINGLE_FUNCTIONS_BLAKE3: &str =
    "e10b037454ac768bcea839b175a842091b49ea119dc7b2c6d16c439af6722273";
const BOX2D_3_2_DOUBLE_FUNCTIONS_BLAKE3: &str =
    "bdc19edc4fd8c005d570c18e833d2c1e7c27fc034fb098e1c1287c9c97518cdf";
const FUNCTION_INVENTORY_DIGEST_DOMAIN: &[u8] = b"boxdd-api-function-inventory-v1\0";
const REVIEWED_MIGRATION_OVERRIDES_PATH: &str =
    "xtask/fixtures/api-contract-3.2-reviewed-migration.toml";
const REVIEWED_MIGRATION_SCHEMA: u32 = 2;
const SAFE_CALL_EVIDENCE_POLICY: &str = "route-conditioned-safe-call-v2";

fn effective_source_sha256(paths: &WorkspacePaths) -> Result<String> {
    effective_source_identity(&paths.root().join("boxdd-sys"))
        .map(|identity| identity.effective_source_sha256)
        .map_err(|error| Error::message(format!("validate effective source identity: {error}")))
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    Safe,
    Raw,
    Omitted,
    Deferred,
}

impl Classification {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Raw => "raw",
            Self::Omitted => "omitted",
            Self::Deferred => "deferred",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageCounts {
    pub total: usize,
    pub safe: usize,
    pub raw: usize,
    pub omitted: usize,
    pub deferred: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionInventoryDigests {
    pub logical: String,
    pub single: String,
    pub double: String,
}

impl CoverageCounts {
    fn add(&mut self, classification: Classification) {
        self.total += 1;
        match classification {
            Classification::Safe => self.safe += 1,
            Classification::Raw => self.raw += 1,
            Classification::Omitted => self.omitted += 1,
            Classification::Deferred => self.deferred += 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationChange {
    pub logical_name: String,
    pub from: Classification,
    pub to: Classification,
    pub unit: String,
    pub rationale: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestEvidence {
    pub id: String,
    pub file: String,
    pub item: String,
    pub package: String,
    pub gate: String,
    #[serde(default)]
    pub role: TestEvidenceRole,
    #[serde(default)]
    pub fingerprint: String,
    /// Exact precision modes in which this evidence is indexed.
    #[serde(default)]
    pub modes: Vec<String>,
    /// Exact providers in which this evidence is indexed.
    #[serde(default)]
    pub providers: Vec<String>,
    /// Exact public calls proven by an executable straight-line Rust test source.
    #[serde(default, alias = "runtime_witnesses")]
    pub call_witnesses: Vec<SafeCallWitness>,
    #[serde(default)]
    pub classification_witnesses: Vec<FunctionClassificationWitness>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestEvidenceRole {
    #[default]
    #[serde(alias = "runtime")]
    SafeCall,
    FunctionClassificationValidator,
    AbiHeaderInventory,
    AbiBindingAst,
    AbiContractValidator,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafeCallWitness {
    /// Logical C API row reached by the reviewed Rust call.
    pub function: String,
    /// Canonical Safe Rust callable, proven through the contract's evidence policy.
    pub rust_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionClassificationWitness {
    pub function: String,
    pub classification: Classification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionProviderOverride {
    pub providers: Vec<String>,
    pub classification: Classification,
    pub rust_paths: Vec<String>,
    pub rationale: String,
    pub evidence: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingClass {
    PureWorldless,
    FoundationInitialization,
    ReadOnly,
    LoggedMutation,
    LoggedQuery,
    RecordingLifecycle,
    SnapshotLifecycle,
    ReplayLifecycle,
    ReplayMixerLifecycle,
    CallbackInstallUnsupported,
    UnloggedMutationForbidden,
    WorldDestroyTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingCoverage {
    pub class: RecordingClass,
    pub opcode: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FunctionExposureKind {
    #[default]
    Callable,
    RaiiDrop,
}

impl FunctionExposureKind {
    fn is_callable(&self) -> bool {
        *self == Self::Callable
    }

    fn path_kind(self) -> &'static str {
        match self {
            Self::Callable => "public safe callable",
            Self::RaiiDrop => "public RAII owner type",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Callable => "callable",
            Self::RaiiDrop => "raii-drop",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionContract {
    pub logical_name: String,
    pub signature: String,
    pub fingerprint: String,
    #[serde(default)]
    pub abi_fingerprints: BTreeMap<String, String>,
    pub link_symbols: BTreeMap<String, String>,
    pub classification: Classification,
    #[serde(default, skip_serializing_if = "FunctionExposureKind::is_callable")]
    pub exposure: FunctionExposureKind,
    pub area: String,
    pub rust_paths: Vec<String>,
    pub rationale: String,
    pub modes: Vec<String>,
    pub providers: Vec<String>,
    pub availability: Vec<String>,
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_overrides: Vec<FunctionProviderOverride>,
    pub recording: Option<RecordingCoverage>,
}

#[derive(Clone, Copy)]
enum FunctionExposureReview<'a> {
    Default(&'a FunctionContract),
    Override(&'a FunctionProviderOverride),
}

impl<'a> FunctionExposureReview<'a> {
    fn classification(self) -> Classification {
        match self {
            Self::Default(function) => function.classification,
            Self::Override(provider_override) => provider_override.classification,
        }
    }

    fn exposure(self) -> FunctionExposureKind {
        match self {
            Self::Default(function) => function.exposure,
            Self::Override(_) => FunctionExposureKind::Callable,
        }
    }

    fn rust_paths(self) -> &'a [String] {
        match self {
            Self::Default(function) => &function.rust_paths,
            Self::Override(provider_override) => &provider_override.rust_paths,
        }
    }

    fn rationale(self) -> &'a str {
        match self {
            Self::Default(function) => &function.rationale,
            Self::Override(provider_override) => &provider_override.rationale,
        }
    }

    fn evidence(self) -> &'a [String] {
        match self {
            Self::Default(function) => &function.evidence,
            Self::Override(provider_override) => &provider_override.evidence,
        }
    }
}

fn function_exposure_for_provider<'a>(
    function: &'a FunctionContract,
    provider: &str,
) -> FunctionExposureReview<'a> {
    function
        .provider_overrides
        .iter()
        .find(|provider_override| {
            provider_override
                .providers
                .iter()
                .any(|candidate| candidate == provider)
        })
        .map_or(
            FunctionExposureReview::Default(function),
            FunctionExposureReview::Override,
        )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApiContract {
    pub schema_version: u32,
    #[serde(default = "default_safe_call_evidence_policy")]
    pub evidence_policy: String,
    pub upstream_sha: String,
    #[serde(default)]
    pub function_inventory_digests: FunctionInventoryDigests,
    pub migration_baseline: CoverageCounts,
    pub classification_changes: Vec<ClassificationChange>,
    pub evidence: Vec<TestEvidence>,
    pub functions: Vec<FunctionContract>,
    pub abi: AbiContract,
}

fn default_safe_call_evidence_policy() -> String {
    SAFE_CALL_EVIDENCE_POLICY.to_owned()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReviewedMigrationOverrides {
    schema_version: u32,
    reviewed_revision: String,
    active_revision: String,
    expected_counts: CoverageCounts,
    functions: Vec<ReviewedFunctionOverride>,
    #[serde(default)]
    canonical_refreshes: Vec<ReviewedCanonicalRefresh>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReviewedFunctionOverride {
    logical_name: String,
    classification: Classification,
    #[serde(default)]
    exposure: FunctionExposureKind,
    area: String,
    #[serde(default)]
    rust_paths: Vec<String>,
    rationale: String,
    #[serde(default)]
    previous_classification: Option<Classification>,
    #[serde(default)]
    transition_unit: Option<String>,
    #[serde(default)]
    revalidated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReviewedCanonicalRefresh {
    logical_name: String,
    rust_paths: Vec<String>,
    rationale: String,
}

pub fn run(paths: &WorkspacePaths, args: &[String]) -> Result<()> {
    if matches!(args, [argument] if matches!(argument.as_str(), "help" | "--help" | "-h")) {
        print_help();
        return Ok(());
    }
    if matches!(args, [argument] if argument == "--audit-evidence") {
        return audit_runtime_evidence(paths);
    }
    if matches!(args, [argument] if argument == "--audit-canonical-paths") {
        return audit_canonical_paths(paths);
    }
    if let [argument, revision] = args
        && argument == "--audit-reviewed-migration"
    {
        return audit_reviewed_migration(paths, revision);
    }
    if let [argument, revision] = args
        && argument == "--migrate-reviewed-contract"
    {
        return migrate_reviewed_contract(paths, revision);
    }
    if matches!(args, [argument] if argument == "--refresh-abi") {
        return refresh_abi_contract(paths, None);
    }
    if let [argument, digest_flag, digest] = args
        && argument == "--refresh-abi"
        && digest_flag == "--reviewed-contract-blake3"
    {
        return refresh_abi_contract(paths, Some(digest));
    }
    match parse_update_mode("api-coverage", args)? {
        UpdateMode::Check => check(paths),
        UpdateMode::Write => write(paths),
    }
}

fn print_help() {
    println!(
        "\
Usage:
  cargo run -p xtask -- api-coverage --check
  cargo run -p xtask -- api-coverage --write
  cargo run -p xtask -- api-coverage --refresh-abi
  cargo run -p xtask -- api-coverage --refresh-abi --reviewed-contract-blake3 <64-hex-blake3>
  cargo run -p xtask -- api-coverage --audit-evidence
  cargo run -p xtask -- api-coverage --audit-canonical-paths
  cargo run -p xtask -- api-coverage --audit-reviewed-migration <40-hex-commit>
  cargo run -p xtask -- api-coverage --migrate-reviewed-contract <40-hex-commit>
"
    );
}

pub fn check(paths: &WorkspacePaths) -> Result<()> {
    let manifest = UpstreamManifest::load(paths)?;
    let snapshot = validate_repository(paths, &manifest, false)?;
    validate_authenticated_revision(
        &manifest.active_revision,
        &snapshot.gitlink_revision,
        &snapshot.worktree_revision,
    )?;
    let validated = load_validated_coverage_with_manifest(paths, manifest)?;
    let recording_wire_path = validated
        .manifest
        .artifact_path(paths.root(), ArtifactKind::RecordingWire)?;
    validate_recording_contract(
        &recording_wire_path,
        &validated.recording_operations,
        &validated.manifest.recording_revision,
        &validated.recording_source_git_blobs,
        &validated.recording_sources_aggregate,
    )?;
    validate_runtime_recording_parser(paths, &recording_wire_path)?;
    let report_path = validated
        .manifest
        .artifact_path(paths.root(), ArtifactKind::ApiCoverageReport)?;
    let actual =
        fs::read_to_string(&report_path).map_err(|source| Error::io(&report_path, source))?;
    if normalize_newlines(&actual) != normalize_newlines(&validated.report) {
        return Err(Error::message(format!(
            "{} is stale; run `cargo run -p xtask -- api-coverage --write`",
            report_path.display()
        )));
    }
    let counts = counts(&validated.contract.functions);
    println!(
        "api contract ok: {} functions, {} ABI structs, {} callbacks",
        counts.total,
        validated.contract.abi.structs.len(),
        validated.contract.abi.callbacks.len()
    );
    Ok(())
}

fn write(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let manifest_baseline = fs::read(paths.upstream_manifest())
        .map_err(|source| Error::io(paths.upstream_manifest(), source))?;
    let manifest = UpstreamManifest::load(paths)?;
    let snapshot = validate_repository(paths, &manifest, false)?;
    validate_authenticated_revision(
        &manifest.active_revision,
        &snapshot.gitlink_revision,
        &snapshot.worktree_revision,
    )?;
    let outputs = render_generated_outputs_with_manifest(paths, manifest)?;
    let writes = [
        ManagedArtifactWrite::active("recording-wire", outputs.recording_wire),
        ManagedArtifactWrite::active("api-coverage-report", outputs.report),
        ManagedArtifactWrite::auxiliary(
            RUNTIME_RECORDING_WIRE_PATH,
            outputs.runtime_recording_wire,
        ),
    ];
    install_managed_artifact_writes_locked(paths, &writes, Some(&manifest_baseline), || {
        validate_managed_repository_and_api(paths)
    })?;
    println!("wrote generated recording contract, runtime parser, and API coverage report");
    Ok(())
}

pub(crate) struct GeneratedApiCoverageOutputs {
    pub(crate) recording_wire: Vec<u8>,
    pub(crate) runtime_recording_wire: Vec<u8>,
    pub(crate) report: Vec<u8>,
}

pub(crate) fn render_generated_outputs(
    paths: &WorkspacePaths,
) -> Result<GeneratedApiCoverageOutputs> {
    let manifest = UpstreamManifest::load(paths)?;
    render_generated_outputs_with_manifest(paths, manifest)
}

fn render_generated_outputs_with_manifest(
    paths: &WorkspacePaths,
    manifest: UpstreamManifest,
) -> Result<GeneratedApiCoverageOutputs> {
    let validated = load_validated_coverage_with_manifest(paths, manifest)?;
    let wire = generate_wire_contract(
        &validated.manifest.recording_revision,
        &validated.recording_operations,
        &validated.recording_source_git_blobs,
        &validated.recording_sources_aggregate,
    )?;
    let recording_wire = render_toml(&wire)?.into_bytes();
    let contract_blake3 = blake3::hash(&recording_wire).to_hex().to_string();
    let effective_source_sha256 = effective_source_sha256(paths)?;
    let runtime_recording_wire =
        render_runtime_parser(&wire, &contract_blake3, &effective_source_sha256)?.into_bytes();
    Ok(GeneratedApiCoverageOutputs {
        recording_wire,
        runtime_recording_wire,
        report: validated.report.into_bytes(),
    })
}

fn validate_runtime_recording_parser(
    paths: &WorkspacePaths,
    recording_wire_path: &std::path::Path,
) -> Result<()> {
    let contract_bytes =
        fs::read(recording_wire_path).map_err(|source| Error::io(recording_wire_path, source))?;
    let contract: RecordingWireContract = read_toml(recording_wire_path)?;
    let digest = blake3::hash(&contract_bytes).to_hex().to_string();
    let effective_source_sha256 = effective_source_sha256(paths)?;
    let expected = render_runtime_parser(&contract, &digest, &effective_source_sha256)?;
    let runtime_path = paths.root().join(RUNTIME_RECORDING_WIRE_PATH);
    let actual =
        fs::read_to_string(&runtime_path).map_err(|source| Error::io(&runtime_path, source))?;
    if normalize_newlines(&actual) != normalize_newlines(&expected) {
        return Err(Error::message(format!(
            "{} is stale; run cargo run -p xtask -- api-coverage --write",
            runtime_path.display()
        )));
    }
    Ok(())
}

struct ValidatedCoverage {
    manifest: UpstreamManifest,
    contract: ApiContract,
    recording_operations: Vec<crate::recording_ops::RecordingOp>,
    recording_source_git_blobs: BTreeMap<String, String>,
    recording_sources_aggregate: String,
    report: String,
}

fn load_validated_coverage_with_manifest(
    paths: &WorkspacePaths,
    manifest: UpstreamManifest,
) -> Result<ValidatedCoverage> {
    let api_contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let contract: ApiContract = read_toml(&api_contract_path)?;
    if manifest.active_revision == BOX2D_3_2_TARGET_REVISION
        && contract.schema_version == API_CONTRACT_SCHEMA
    {
        let mut errors = Vec::new();
        validate_pinned_box2d_3_2_binding_artifacts(&manifest.artifacts, &mut errors);
        if !errors.is_empty() {
            return Err(Error::message(errors.join("\n")));
        }
    }
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let precision_inventories = load_precision_inventories(paths, &binding_routes)?;
    let rust_indexes =
        load_rust_indexes_with_inventory(paths, &binding_routes, &binding_indexes, &inventory)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let recording_source_git_blobs = manifest.recording_source_git_blobs();
    let recording_sources_aggregate =
        reviewed_sources_aggregate_blake3(&recording_source_git_blobs)?;
    let safe_functions = contract
        .functions
        .iter()
        .filter(|function| function.classification == Classification::Safe)
        .map(|function| function.logical_name.as_str())
        .collect::<BTreeSet<_>>();
    let known_functions = contract
        .functions
        .iter()
        .map(|function| function.logical_name.as_str())
        .collect::<BTreeSet<_>>();
    super::api_recording::validate_registry(
        &safe_functions,
        &known_functions,
        &recording_operations,
    )?;
    validate_contract(
        paths,
        &contract,
        &inventory,
        Some(&precision_inventories),
        &rust_indexes,
        &binding_routes,
        &binding_indexes,
        &manifest.active_revision,
        &recording_operations,
    )?;
    let report = render_report(&contract);
    Ok(ValidatedCoverage {
        manifest,
        contract,
        recording_operations,
        recording_source_git_blobs,
        recording_sources_aggregate,
        report,
    })
}

fn validate_authenticated_revision(
    expected_active_revision: &str,
    gitlink_revision: &str,
    worktree_revision: &str,
) -> Result<()> {
    if gitlink_revision == expected_active_revision && worktree_revision == expected_active_revision
    {
        return Ok(());
    }
    Err(Error::message(format!(
        "API coverage manifest revision `{expected_active_revision}` is not authenticated: authenticated gitlink `{gitlink_revision}`, authenticated worktree `{worktree_revision}`"
    )))
}

fn compute_function_inventory_digests(
    inventory: &CApiInventory,
) -> Result<FunctionInventoryDigests> {
    let mut logical = BTreeMap::new();
    let mut single = BTreeMap::new();
    let mut double = BTreeMap::new();
    for function in &inventory.functions {
        if logical
            .insert(function.name.clone(), String::new())
            .is_some()
        {
            return Err(Error::message(format!(
                "duplicate logical function `{}` in C header inventory",
                function.name
            )));
        }
        for (mode, symbols) in [("single", &mut single), ("double", &mut double)] {
            let symbol = function.physical_symbols.get(mode).ok_or_else(|| {
                Error::message(format!(
                    "logical function `{}` has no `{mode}` physical symbol",
                    function.name
                ))
            })?;
            symbols.insert(function.name.clone(), symbol.clone());
        }
    }
    Ok(FunctionInventoryDigests {
        logical: digest_function_inventory("logical", &logical),
        single: digest_function_inventory("single", &single),
        double: digest_function_inventory("double", &double),
    })
}

fn digest_function_inventory(kind: &str, entries: &BTreeMap<String, String>) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(FUNCTION_INVENTORY_DIGEST_DOMAIN);
    update_function_inventory_digest(&mut hasher, kind);
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for (logical, physical) in entries {
        update_function_inventory_digest(&mut hasher, logical);
        update_function_inventory_digest(&mut hasher, physical);
    }
    hasher.finalize().to_hex().to_string()
}

fn update_function_inventory_digest(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn validate_function_inventory_digests(
    expected: &FunctionInventoryDigests,
    inventory: &CApiInventory,
    errors: &mut Vec<String>,
) -> Option<FunctionInventoryDigests> {
    let observed = match compute_function_inventory_digests(inventory) {
        Ok(observed) => observed,
        Err(error) => {
            errors.push(format!("cannot authenticate function inventory: {error}"));
            return None;
        }
    };
    compare_function_inventory_digests("reviewed API contract", expected, &observed, errors);
    Some(observed)
}

fn compare_function_inventory_digests(
    expectation: &str,
    expected: &FunctionInventoryDigests,
    observed: &FunctionInventoryDigests,
    errors: &mut Vec<String>,
) {
    for (kind, expected, observed) in [
        ("logical", &expected.logical, &observed.logical),
        ("single physical", &expected.single, &observed.single),
        ("double physical", &expected.double, &observed.double),
    ] {
        if expected != observed {
            errors.push(format!(
                "{kind} function inventory digest does not match {expectation}: expected `{expected}`, observed `{observed}`"
            ));
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the top-level validator composes independently indexed contract domains"
)]
pub fn validate_contract(
    paths: &WorkspacePaths,
    contract: &ApiContract,
    inventory: &CApiInventory,
    precision_inventories: Option<&AbiPrecisionInventories>,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    expected_active_revision: &str,
    recording_operations: &[crate::recording_ops::RecordingOp],
) -> Result<()> {
    let mut errors = Vec::new();
    let expected_function_count = (expected_active_revision == BOX2D_3_2_TARGET_REVISION)
        .then_some(BOX2D_3_2_EXPORTED_FUNCTION_COUNT);
    if let Some(expected) = expected_function_count
        && inventory.functions.len() != expected
    {
        errors.push(format!(
            "active Box2D target `{expected_active_revision}` exposes {} header functions, expected exactly {expected}",
            inventory.functions.len()
        ));
    }
    let observed_function_digests = validate_function_inventory_digests(
        &contract.function_inventory_digests,
        inventory,
        &mut errors,
    );
    if expected_active_revision == BOX2D_3_2_TARGET_REVISION
        && let Some(observed) = observed_function_digests
    {
        let pinned = FunctionInventoryDigests {
            logical: BOX2D_3_2_LOGICAL_FUNCTIONS_BLAKE3.to_owned(),
            single: BOX2D_3_2_SINGLE_FUNCTIONS_BLAKE3.to_owned(),
            double: BOX2D_3_2_DOUBLE_FUNCTIONS_BLAKE3.to_owned(),
        };
        compare_function_inventory_digests(
            "pinned Box2D 3.2 target",
            &pinned,
            &observed,
            &mut errors,
        );
    }
    if contract.schema_version != API_CONTRACT_SCHEMA {
        errors.push(format!(
            "API contract schema {} does not match supported schema {API_CONTRACT_SCHEMA}",
            contract.schema_version
        ));
    }
    if contract.evidence_policy != SAFE_CALL_EVIDENCE_POLICY {
        errors.push(format!(
            "API contract evidence policy `{}` does not match supported policy `{SAFE_CALL_EVIDENCE_POLICY}`",
            contract.evidence_policy
        ));
    }
    if contract.upstream_sha != expected_active_revision {
        errors.push(format!(
            "API contract upstream {} does not match active revision {expected_active_revision}",
            contract.upstream_sha,
        ));
    }
    if expected_active_revision == BOX2D_3_2_TARGET_REVISION
        && contract.schema_version == API_CONTRACT_SCHEMA
    {
        validate_pinned_box2d_3_2_binding_routes(binding_routes, &mut errors);
        validate_pinned_box2d_3_2_no_deferred_functions(contract, &mut errors);
    }

    let inventory_by_name = inventory
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let route_coordinates = binding_routes.keys().cloned().collect::<BTreeSet<_>>();
    let route_modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.as_str())
        .collect::<BTreeSet<_>>();
    let route_providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.as_str())
        .collect::<BTreeSet<_>>();
    let mut evidence_by_id = BTreeMap::new();
    let mut evidence_indexes = BTreeMap::new();
    for evidence in &contract.evidence {
        if evidence_by_id
            .insert(evidence.id.as_str(), evidence)
            .is_some()
        {
            errors.push(format!("duplicate evidence id `{}`", evidence.id));
        }
        validate_evidence_scope(
            evidence,
            &route_coordinates,
            &route_modes,
            &route_providers,
            &mut errors,
        );
        match index_evidence_across_routes(paths, evidence, rust_indexes, binding_routes) {
            Ok(actual) => {
                let fingerprint = aggregate_evidence_fingerprint(&actual);
                if fingerprint != evidence.fingerprint {
                    errors.push(format!(
                        "evidence `{}` fingerprint drifted: reviewed `{}`, normalized AST `{fingerprint}`",
                        evidence.id, evidence.fingerprint
                    ));
                }
                evidence_indexes.insert(evidence.id.as_str(), actual);
            }
            Err(error) => errors.push(format!("evidence `{}`: {error}", evidence.id)),
        }
        if !matches!(evidence.package.as_str(), "boxdd" | "xtask")
            || !evidence.file.starts_with(&format!("{}/", evidence.package))
        {
            errors.push(format!(
                "evidence `{}` must name the package that owns its repository-relative test file",
                evidence.id
            ));
        }
        if evidence.gate != "nextest" {
            errors.push(format!(
                "evidence `{}` must declare the executable `nextest` gate",
                evidence.id
            ));
        }
        validate_evidence_role(
            evidence,
            evidence_indexes.get(evidence.id.as_str()),
            &mut errors,
        );
    }
    let availability_registry = AVAILABILITY.iter().copied().collect::<BTreeSet<_>>();
    let mut rows = BTreeSet::new();
    for function in &contract.functions {
        if !rows.insert(function.logical_name.as_str()) {
            errors.push(format!(
                "duplicate API contract function `{}`",
                function.logical_name
            ));
        }
        let Some(declaration) = inventory_by_name.get(function.logical_name.as_str()) else {
            errors.push(format!(
                "contract function `{}` is absent from active headers",
                function.logical_name
            ));
            continue;
        };
        if function.signature != declaration.signature
            || function.fingerprint != declaration.fingerprint
        {
            errors.push(format!(
                "signature drift for `{}`: expected `{}`, parsed `{}`",
                function.logical_name, function.signature, declaration.signature
            ));
        }
        if function.availability != declaration.availability {
            errors.push(format!(
                "availability drift for `{}`: contract {:?}, header-derived {:?}",
                function.logical_name, function.availability, declaration.availability
            ));
        }
        validate_registry_values(
            &function.logical_name,
            "mode",
            &function.modes,
            &route_modes,
            &mut errors,
        );
        let function_coordinates = function
            .modes
            .iter()
            .flat_map(|mode| {
                function
                    .providers
                    .iter()
                    .map(move |provider| (mode.clone(), provider.clone()))
            })
            .collect::<BTreeSet<_>>();
        if function_coordinates != route_coordinates {
            errors.push(format!(
                "`{}` must cover exactly the current executable mode/provider matrix",
                function.logical_name
            ));
        }
        validate_registry_values(
            &function.logical_name,
            "provider",
            &function.providers,
            &route_providers,
            &mut errors,
        );
        validate_registry_values(
            &function.logical_name,
            "availability",
            &function.availability,
            &availability_registry,
            &mut errors,
        );
        if function.modes.is_empty()
            || function.providers.is_empty()
            || function.availability.is_empty()
        {
            errors.push(format!(
                "`{}` must declare modes, providers, and availability",
                function.logical_name
            ));
        }
        if function.availability.contains(&"always".to_owned()) && function.availability.len() != 1
        {
            errors.push(format!(
                "`{}` availability `always` cannot be combined with conditions",
                function.logical_name
            ));
        }
        let link_modes = function
            .link_symbols
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let required_modes = route_modes.clone();
        if link_modes != required_modes {
            errors.push(format!(
                "`{}` must declare physical link symbols for the exact manifest mode set {:?}",
                function.logical_name, required_modes
            ));
        }
        for (mode, symbol) in &function.link_symbols {
            if !is_c_identifier(symbol) {
                errors.push(format!(
                    "`{}` has invalid {mode} link symbol `{symbol}`",
                    function.logical_name
                ));
            }
            match declaration.physical_symbols.get(mode) {
                Some(expected) if symbol == expected => {}
                Some(expected) => errors.push(format!(
                    "`{}` {mode} link symbol `{symbol}` does not match header-derived physical symbol `{expected}`",
                    function.logical_name
                )),
                None => errors.push(format!(
                    "`{}` has no header-derived physical symbol for mode `{mode}`",
                    function.logical_name
                )),
            }
        }
        if let Some(precision_inventories) = precision_inventories {
            validate_function_abi_fingerprints(
                function,
                declaration,
                precision_inventories,
                binding_routes,
                binding_indexes,
                &route_modes,
                &mut errors,
            );
        }
        if function.area.trim().is_empty() {
            errors.push(format!("`{}` has no explicit area", function.logical_name));
        }
        let default_providers =
            validate_function_provider_overrides(function, &route_providers, &mut errors);
        let default_coordinates = route_coordinates
            .iter()
            .filter(|(_, provider)| default_providers.contains(provider.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        validate_function_exposure_review(
            function,
            "default",
            FunctionExposureReview::Default(function),
            &default_coordinates,
            declaration,
            rust_indexes,
            &evidence_by_id,
            &mut errors,
        );
        for provider_override in &function.provider_overrides {
            let coordinates = route_coordinates
                .iter()
                .filter(|(_, provider)| provider_override.providers.contains(provider))
                .cloned()
                .collect::<BTreeSet<_>>();
            validate_function_exposure_review(
                function,
                &format!("provider override {:?}", provider_override.providers),
                FunctionExposureReview::Override(provider_override),
                &coordinates,
                declaration,
                rust_indexes,
                &evidence_by_id,
                &mut errors,
            );
            if provider_override
                .providers
                .iter()
                .any(|provider| !is_wasm_provider(provider))
            {
                errors.push(format!(
                    "function `{}` may only use conservative provider overrides for WASM providers",
                    function.logical_name
                ));
            }
            for coordinate in &coordinates {
                if safe_function_review_matches_coordinate(
                    &function.rust_paths,
                    function.exposure,
                    declaration,
                    coordinate,
                    rust_indexes,
                ) {
                    errors.push(format!(
                        "function `{}` provider override unnecessarily hides a proven Safe path at route `{}/{}`",
                        function.logical_name, coordinate.0, coordinate.1
                    ));
                }
            }
        }
        let expected_recording = super::api_recording::expected(
            &function.logical_name,
            function.classification,
            recording_operations,
        );
        if !super::api_recording::is_explicitly_classified(
            &function.logical_name,
            function.classification,
        ) {
            errors.push(format!(
                "safe function `{}` has no explicit recording capability classification",
                function.logical_name
            ));
        }
        if function.recording != expected_recording {
            errors.push(format!(
                "`{}` recording class {:?} does not match expected {:?}",
                function.logical_name, function.recording, expected_recording
            ));
        }
    }
    for declaration in &inventory.functions {
        if !rows.contains(declaration.name.as_str()) {
            errors.push(format!(
                "active header function `{}` has no contract row",
                declaration.name
            ));
        }
    }

    validate_typed_evidence_v8(
        contract,
        binding_routes,
        &evidence_by_id,
        &evidence_indexes,
        &mut errors,
    );

    validate_migration(contract, &mut errors);
    let evidence_ids = evidence_by_id
        .keys()
        .map(|id| (*id).to_owned())
        .collect::<BTreeSet<_>>();
    let function_symbols = abi_function_symbols(inventory, binding_routes);
    if let Some(precision_inventories) = precision_inventories {
        let abi_context = AbiValidationContext::new_precision(
            inventory,
            precision_inventories,
            binding_routes,
            binding_indexes,
            &function_symbols,
            rust_indexes,
            &evidence_ids,
        )
        .with_expected_function_count(expected_function_count);
        validate_abi(&contract.abi, &abi_context, &mut errors);
    } else {
        let abi_context = AbiValidationContext::new(
            inventory,
            binding_routes,
            binding_indexes,
            &function_symbols,
            rust_indexes,
            &evidence_ids,
        )
        .with_expected_function_count(expected_function_count);
        validate_abi(&contract.abi, &abi_context, &mut errors);
    }
    if expected_active_revision == BOX2D_3_2_TARGET_REVISION
        && contract.schema_version == API_CONTRACT_SCHEMA
    {
        validate_reviewed_deferred_migration_invariant(&contract.abi, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn pinned_box2d_3_2_binding_routes() -> AbiBindingRoutes {
    canonical_binding_routes()
        .into_iter()
        .map(|route| {
            let mode = route.mode.as_str().to_owned();
            let provider = route.provider.as_str().to_owned();
            (
                (mode.clone(), provider.clone()),
                AbiBindingRoute {
                    mode,
                    provider,
                    artifact: route.artifact,
                    rust_target: route.rust_target,
                    rust_features: route.rust_features,
                },
            )
        })
        .collect()
}

fn validate_pinned_box2d_3_2_binding_routes(
    binding_routes: &AbiBindingRoutes,
    errors: &mut Vec<String>,
) {
    let expected = pinned_box2d_3_2_binding_routes();
    if binding_routes != &expected {
        errors.push(format!(
            "pinned Box2D 3.2 schema-8 contract requires the exact canonical 10-route matrix; observed {binding_routes:?}, expected {expected:?}"
        ));
    }
}

fn validate_pinned_box2d_3_2_binding_artifacts(
    artifacts: &[GeneratedArtifact],
    errors: &mut Vec<String>,
) {
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
    let observed = artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::Bindings)
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
        errors.push(format!(
            "pinned Box2D 3.2 schema-8 contract requires the exact canonical six binding artifact identities; observed {observed:?}, expected {expected:?}"
        ));
    }
}

fn validate_pinned_box2d_3_2_no_deferred_functions(
    contract: &ApiContract,
    errors: &mut Vec<String>,
) {
    for function in &contract.functions {
        if function.classification == Classification::Deferred {
            errors.push(format!(
                "pinned Box2D 3.2 schema-8 function `{}` reintroduces Deferred",
                function.logical_name
            ));
        }
        for provider_override in &function.provider_overrides {
            if provider_override.classification == Classification::Deferred {
                errors.push(format!(
                    "pinned Box2D 3.2 schema-8 function `{}` provider override {:?} reintroduces Deferred",
                    function.logical_name, provider_override.providers
                ));
            }
        }
    }
}

fn validate_function_provider_overrides(
    function: &FunctionContract,
    route_providers: &BTreeSet<&str>,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    if !function
        .provider_overrides
        .windows(2)
        .all(|pair| pair[0].providers < pair[1].providers)
    {
        errors.push(format!(
            "function `{}` provider overrides must be strictly sorted by provider scope",
            function.logical_name
        ));
    }
    let mut overridden = BTreeSet::new();
    for provider_override in &function.provider_overrides {
        if function.classification != Classification::Safe
            || provider_override.classification != Classification::Raw
        {
            errors.push(format!(
                "function `{}` provider overrides may only conservatively narrow a default Safe exposure to Raw",
                function.logical_name
            ));
        }
        if provider_override.providers.is_empty() {
            errors.push(format!(
                "function `{}` has an empty provider override",
                function.logical_name
            ));
        }
        if !provider_override
            .providers
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        {
            errors.push(format!(
                "function `{}` provider override {:?} must be strictly sorted and unique",
                function.logical_name, provider_override.providers
            ));
        }
        for provider in &provider_override.providers {
            if !route_providers.contains(provider.as_str())
                || !function.providers.contains(provider)
            {
                errors.push(format!(
                    "function `{}` override references unregistered provider `{provider}`",
                    function.logical_name
                ));
            }
            if !overridden.insert(provider.clone()) {
                errors.push(format!(
                    "function `{}` provider `{provider}` is covered by multiple overrides",
                    function.logical_name
                ));
            }
        }
    }
    let defaults = function
        .providers
        .iter()
        .filter(|provider| !overridden.contains(*provider))
        .cloned()
        .collect::<BTreeSet<_>>();
    if defaults.is_empty() {
        errors.push(format!(
            "function `{}` provider overrides consume the entire default exposure",
            function.logical_name
        ));
    }
    defaults
}

#[allow(clippy::too_many_arguments)]
fn validate_function_exposure_review(
    function: &FunctionContract,
    label: &str,
    review: FunctionExposureReview<'_>,
    coordinates: &BTreeSet<(String, String)>,
    declaration: &crate::c_api::FunctionDecl,
    rust_indexes: &AbiRustIndexes,
    evidence_by_id: &BTreeMap<&str, &TestEvidence>,
    errors: &mut Vec<String>,
) {
    let subject = format!("function `{}` {label}", function.logical_name);
    if coordinates.is_empty() {
        errors.push(format!("{subject} has no executable route"));
    }
    if !has_rationale(review.rationale()) {
        errors.push(format!("{subject} needs a specific rationale"));
    }
    if review.evidence().is_empty() {
        errors.push(format!("{subject} has no test evidence"));
    }
    let expected_role = match review.classification() {
        Classification::Safe => TestEvidenceRole::SafeCall,
        Classification::Raw | Classification::Omitted | Classification::Deferred => {
            TestEvidenceRole::FunctionClassificationValidator
        }
    };
    let mut unique_evidence = BTreeSet::new();
    for evidence in review.evidence() {
        if !unique_evidence.insert(evidence.as_str()) {
            errors.push(format!("{subject} repeats evidence `{evidence}`"));
        }
        match evidence_by_id.get(evidence.as_str()) {
            None => errors.push(format!("{subject} references unknown evidence `{evidence}`")),
            Some(row) if row.role != expected_role => errors.push(format!(
                "{subject} references evidence `{evidence}` with role {:?}, expected {:?}",
                row.role, expected_role
            )),
            Some(row) if test_evidence_coordinates(row) != *coordinates => errors.push(format!(
                "{subject} evidence `{evidence}` scope {:?} must exactly match effective routes {:?}",
                test_evidence_coordinates(row),
                coordinates
            )),
            Some(_) => {}
        }
    }
    if !review.rust_paths().windows(2).all(|pair| pair[0] < pair[1]) {
        errors.push(format!(
            "{subject} Rust paths must be strictly sorted and unique"
        ));
    }
    if review.classification() != Classification::Safe
        && review.exposure() != FunctionExposureKind::Callable
    {
        errors.push(format!(
            "{subject} is {} and cannot claim the {:?} Safe Rust exposure kind",
            review.classification().as_str(),
            review.exposure()
        ));
    }
    match review.classification() {
        Classification::Safe => {
            if review.rust_paths().is_empty() {
                errors.push(format!("{subject} has no canonical Rust path"));
            }
            for coordinate in coordinates {
                let Some(index) = rust_indexes.get(coordinate) else {
                    errors.push(format!(
                        "{subject} has no Rust index for route `{}/{}`",
                        coordinate.0, coordinate.1
                    ));
                    continue;
                };
                let Some(symbol) = declaration.physical_symbols.get(&coordinate.0) else {
                    continue;
                };
                for path in review.rust_paths() {
                    if !safe_exposure_path_exists(index, review.exposure(), path) {
                        errors.push(format!(
                            "{subject} references nonexistent {} `{path}` at route `{}/{}`",
                            review.exposure().path_kind(),
                            coordinate.0,
                            coordinate.1
                        ));
                    } else if !index.path_reaches_symbol(path, symbol) {
                        errors.push(format!(
                            "{} `{path}` does not reach physical symbol `{symbol}` through the Rust AST call graph at route `{}/{}`",
                            review.exposure().path_kind(),
                            coordinate.0,
                            coordinate.1
                        ));
                    }
                }
            }
        }
        Classification::Raw => {
            let expected_paths = coordinates
                .iter()
                .filter_map(|(mode, _)| declaration.physical_symbols.get(mode))
                .map(|symbol| format!("boxdd_sys::ffi::{symbol}"))
                .collect::<BTreeSet<_>>();
            let actual_paths = review.rust_paths().iter().cloned().collect::<BTreeSet<_>>();
            if actual_paths != expected_paths || review.rust_paths().is_empty() {
                errors.push(format!(
                    "{subject} must name exactly its route-derived boxdd_sys::ffi physical paths"
                ));
            }
        }
        Classification::Omitted | Classification::Deferred => {
            if !review.rust_paths().is_empty() {
                errors.push(format!(
                    "{subject} is {} and cannot claim a Rust path",
                    review.classification().as_str()
                ));
            }
        }
    }
}

fn validate_function_abi_fingerprints(
    function: &FunctionContract,
    declaration: &crate::c_api::FunctionDecl,
    precision_inventories: &AbiPrecisionInventories,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    route_modes: &BTreeSet<&str>,
    errors: &mut Vec<String>,
) {
    let reviewed_modes = function
        .abi_fingerprints
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if reviewed_modes != *route_modes {
        errors.push(format!(
            "`{}` must declare recursive ABI fingerprints for the exact manifest mode set {:?}",
            function.logical_name, route_modes
        ));
    }

    for mode in route_modes {
        let Some(precision_inventory) = precision_inventories.get(*mode) else {
            errors.push(format!(
                "`{}` has no effective C ABI inventory for mode `{mode}`",
                function.logical_name
            ));
            continue;
        };
        if precision_inventory.precision.as_str() != *mode {
            errors.push(format!(
                "effective C ABI inventory registered as `{mode}` reports precision `{}`",
                precision_inventory.precision.as_str()
            ));
            continue;
        }
        let Some(c_function) = precision_inventory.function(&function.logical_name) else {
            errors.push(format!(
                "function `{}` is absent from the effective `{mode}` C ABI inventory",
                function.logical_name
            ));
            continue;
        };
        let Some(reviewed_fingerprint) = function.abi_fingerprints.get(*mode) else {
            continue;
        };
        if reviewed_fingerprint != &c_function.fingerprint {
            errors.push(format!(
                "function `{}` `{mode}` recursive ABI fingerprint drifted: contract `{reviewed_fingerprint}`, C header `{}`",
                function.logical_name, c_function.fingerprint
            ));
        }

        let Some(symbol) = function.link_symbols.get(*mode) else {
            continue;
        };
        if declaration.physical_symbols.get(*mode) != Some(symbol) {
            continue;
        }
        let physical_path = format!("boxdd_sys::ffi::{symbol}");
        for ((route_mode, provider), route) in binding_routes {
            if route_mode != *mode {
                continue;
            }
            let Some(binding) = binding_indexes.get(&route.artifact) else {
                errors.push(format!(
                    "function `{}` route `{route_mode}/{provider}` references missing binding artifact `{}`",
                    function.logical_name, route.artifact
                ));
                continue;
            };
            match binding.index.function_abi_fingerprint(&physical_path) {
                Ok(Some(rust_fingerprint)) if rust_fingerprint == c_function.fingerprint => {
                    if &rust_fingerprint != reviewed_fingerprint {
                        errors.push(format!(
                            "function `{}` route `{route_mode}/{provider}` Rust ABI fingerprint `{rust_fingerprint}` does not match contract `{reviewed_fingerprint}`",
                            function.logical_name
                        ));
                    }
                }
                Ok(Some(rust_fingerprint)) => errors.push(format!(
                    "function `{}` route `{route_mode}/{provider}` physical symbol `{physical_path}` has Rust ABI fingerprint `{rust_fingerprint}`, but the effective C header has `{}`",
                    function.logical_name, c_function.fingerprint
                )),
                Ok(None) => errors.push(format!(
                    "function `{}` route `{route_mode}/{provider}` physical symbol `{physical_path}` is absent from the generated Rust binding ABI index",
                    function.logical_name
                )),
                Err(error) => errors.push(format!(
                    "function `{}` route `{route_mode}/{provider}` physical symbol `{physical_path}` cannot be ABI-indexed: {error}",
                    function.logical_name
                )),
            }
        }
    }
}

fn index_evidence_across_routes(
    paths: &WorkspacePaths,
    evidence: &TestEvidence,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<BTreeMap<(String, String), TestEvidenceIndex>> {
    let requested = evidence
        .modes
        .iter()
        .flat_map(|mode| {
            evidence
                .providers
                .iter()
                .map(move |provider| (mode.clone(), provider.clone()))
        })
        .collect::<BTreeSet<_>>();
    if requested.is_empty() {
        return Err(Error::message(format!(
            "evidence `{}` must declare a non-empty mode/provider scope",
            evidence.id
        )));
    }
    let mut indexed_routes = BTreeMap::new();
    for coordinate in requested {
        let rust_index = rust_indexes.get(&coordinate).ok_or_else(|| {
            Error::message(format!(
                "evidence `{}` scope references missing Rust route `{}/{}`",
                evidence.id, coordinate.0, coordinate.1
            ))
        })?;
        let route = binding_routes.get(&coordinate).ok_or_else(|| {
            Error::message(format!(
                "evidence route `{}/{}` has no binding route",
                coordinate.0, coordinate.1
            ))
        })?;
        let expanded_features = expanded_binding_route_features(paths, &route.rust_features)?;
        let rust_coordinate = rust_index_coordinate(route.rust_target)
            .with_cfg_values("feature", expanded_features.iter());
        let indexed = index_test_evidence_for_gate_at_coordinate(
            paths.root(),
            &evidence.file,
            &evidence.item,
            &evidence.package,
            &evidence.gate,
            rust_index,
            &rust_coordinate,
        )?;
        indexed_routes.insert(coordinate, indexed);
    }
    if indexed_routes.is_empty() {
        Err(Error::message(format!(
            "evidence `{}` cannot be indexed without an executable Rust route",
            evidence.id
        )))
    } else {
        Ok(indexed_routes)
    }
}

fn validate_evidence_scope(
    evidence: &TestEvidence,
    route_coordinates: &BTreeSet<(String, String)>,
    route_modes: &BTreeSet<&str>,
    route_providers: &BTreeSet<&str>,
    errors: &mut Vec<String>,
) {
    validate_registry_values(
        &format!("evidence `{}`", evidence.id),
        "mode",
        &evidence.modes,
        route_modes,
        errors,
    );
    validate_registry_values(
        &format!("evidence `{}`", evidence.id),
        "provider",
        &evidence.providers,
        route_providers,
        errors,
    );
    if !evidence.modes.windows(2).all(|pair| pair[0] < pair[1]) {
        errors.push(format!(
            "evidence `{}` modes must be strictly sorted and unique",
            evidence.id
        ));
    }
    if !evidence.providers.windows(2).all(|pair| pair[0] < pair[1]) {
        errors.push(format!(
            "evidence `{}` providers must be strictly sorted and unique",
            evidence.id
        ));
    }
    let coordinates = test_evidence_coordinates(evidence);
    if coordinates.is_empty() {
        errors.push(format!(
            "evidence `{}` must declare a non-empty mode/provider scope",
            evidence.id
        ));
    }
    for coordinate in coordinates.difference(route_coordinates) {
        errors.push(format!(
            "evidence `{}` scope contains unregistered route `{}/{}`",
            evidence.id, coordinate.0, coordinate.1
        ));
    }
    if matches!(
        evidence.role,
        TestEvidenceRole::AbiHeaderInventory
            | TestEvidenceRole::AbiBindingAst
            | TestEvidenceRole::AbiContractValidator
    ) && coordinates != *route_coordinates
    {
        errors.push(format!(
            "ABI evidence `{}` scope {:?} must exactly cover every binding route {:?}",
            evidence.id, coordinates, route_coordinates
        ));
    }
}

fn test_evidence_coordinates(evidence: &TestEvidence) -> BTreeSet<(String, String)> {
    evidence
        .modes
        .iter()
        .flat_map(|mode| {
            evidence
                .providers
                .iter()
                .map(move |provider| (mode.clone(), provider.clone()))
        })
        .collect()
}

fn aggregate_evidence_fingerprint(
    indexed_routes: &BTreeMap<(String, String), TestEvidenceIndex>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"boxdd-test-evidence-route-fingerprint-v2\0");
    for ((mode, provider), index) in indexed_routes {
        for component in [mode.as_str(), provider.as_str(), index.fingerprint.as_str()] {
            hasher.update(&(component.len() as u64).to_le_bytes());
            hasher.update(component.as_bytes());
        }
    }
    format!("blake3-routes-v2:{}", hasher.finalize().to_hex())
}

fn validate_evidence_role(
    evidence: &TestEvidence,
    route_indexes: Option<&BTreeMap<(String, String), TestEvidenceIndex>>,
    errors: &mut Vec<String>,
) {
    let (expected, required_local_path) = match evidence.role {
        TestEvidenceRole::SafeCall => {
            if evidence.package != "boxdd" || !evidence.file.starts_with("boxdd/tests/") {
                errors.push(format!(
                    "Safe-call evidence `{}` must be an executable boxdd integration test source",
                    evidence.id
                ));
            }
            if !evidence.classification_witnesses.is_empty() {
                errors.push(format!(
                    "Safe-call evidence `{}` cannot contain classification witnesses",
                    evidence.id
                ));
            }
            return;
        }
        TestEvidenceRole::FunctionClassificationValidator => (
            (
                API_CLASSIFICATION_EVIDENCE_ID,
                "xtask/src/commands/api_coverage.rs",
                "typed_function_classification_evidence_rejects_unrelated_subjects",
            ),
            "validate_contract",
        ),
        TestEvidenceRole::AbiHeaderInventory => (
            (
                ABI_HEADER_EVIDENCE_ID,
                "xtask/src/c_api.rs",
                "vendored_headers_build_precision_abi_inventories",
            ),
            "parse_headers_for_precision",
        ),
        TestEvidenceRole::AbiBindingAst => (
            (
                ABI_BINDING_EVIDENCE_ID,
                "xtask/src/sys_abi_index.rs",
                "indexes_the_checked_in_pregenerated_bindings",
            ),
            "index_bindings",
        ),
        TestEvidenceRole::AbiContractValidator => (
            (
                ABI_VALIDATOR_EVIDENCE_ID,
                "xtask/src/commands/api_coverage.rs",
                "abi_capability_mapping_rejects_deleted_forged_and_unknown_references",
            ),
            "validate_contract",
        ),
    };
    let id_matches = if evidence.role == TestEvidenceRole::FunctionClassificationValidator {
        evidence.id == API_CLASSIFICATION_EVIDENCE_ID
            || evidence.id.starts_with("api-classification-")
    } else {
        evidence.id == expected.0
    };
    if !id_matches
        || (
            evidence.file.as_str(),
            evidence.item.as_str(),
            evidence.package.as_str(),
            evidence.gate.as_str(),
        ) != (expected.1, expected.2, "xtask", "nextest")
    {
        errors.push(format!(
            "evidence role {:?} must point to the reviewed production validator `{}` in `{}`",
            evidence.role, expected.2, expected.1
        ));
    }
    if !route_indexes.is_some_and(|indexes| {
        !indexes.is_empty()
            && indexes.values().all(|index| {
                index.called_local_paths.iter().any(|path| {
                    path == required_local_path
                        || path.ends_with(&format!("::{required_local_path}"))
                })
            })
    }) {
        errors.push(format!(
            "evidence `{}` role {:?} does not invoke required production entry `{required_local_path}` on every route",
            evidence.id, evidence.role
        ));
    }
    if !evidence.call_witnesses.is_empty() {
        errors.push(format!(
            "non-Safe-call evidence `{}` cannot contain Safe-call witnesses",
            evidence.id
        ));
    }
    if evidence.role != TestEvidenceRole::FunctionClassificationValidator
        && !evidence.classification_witnesses.is_empty()
    {
        errors.push(format!(
            "ABI evidence `{}` cannot contain function classification witnesses",
            evidence.id
        ));
    }
}

fn validate_typed_evidence_v8(
    contract: &ApiContract,
    binding_routes: &AbiBindingRoutes,
    evidence_by_id: &BTreeMap<&str, &TestEvidence>,
    evidence_indexes: &BTreeMap<&str, BTreeMap<(String, String), TestEvidenceIndex>>,
    errors: &mut Vec<String>,
) {
    let functions = contract
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let mut referenced = BTreeSet::new();
    for function in &contract.functions {
        referenced.extend(function.evidence.iter().map(String::as_str));
        for provider_override in &function.provider_overrides {
            referenced.extend(provider_override.evidence.iter().map(String::as_str));
        }
    }
    validate_abi_typed_evidence(contract, evidence_by_id, &mut referenced, errors);

    type WitnessCoordinate = (String, String, String);
    let mut safe_calls = BTreeMap::<WitnessCoordinate, BTreeSet<&str>>::new();
    let mut classifications = BTreeMap::<WitnessCoordinate, BTreeSet<&str>>::new();
    for evidence in &contract.evidence {
        if !referenced.contains(evidence.id.as_str()) {
            errors.push(format!(
                "evidence `{}` is orphaned because no function or ABI policy references it",
                evidence.id
            ));
        }
        if evidence
            .call_witnesses
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            errors.push(format!(
                "evidence `{}` Safe-call witnesses must be unique and strictly sorted",
                evidence.id
            ));
        }
        if evidence
            .classification_witnesses
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            errors.push(format!(
                "evidence `{}` classification witnesses must be unique and strictly sorted",
                evidence.id
            ));
        }
        if evidence.role == TestEvidenceRole::SafeCall && evidence.call_witnesses.is_empty() {
            errors.push(format!(
                "Safe-call evidence `{}` has no exact executable call witness",
                evidence.id
            ));
        }
        if evidence.role == TestEvidenceRole::FunctionClassificationValidator
            && evidence.classification_witnesses.is_empty()
        {
            errors.push(format!(
                "classification evidence `{}` has no exact function classification witness",
                evidence.id
            ));
        }
        let Some(indexed_routes) = evidence_indexes.get(evidence.id.as_str()) else {
            continue;
        };
        for witness in &evidence.call_witnesses {
            let Some(function) = functions.get(witness.function.as_str()) else {
                errors.push(format!(
                    "evidence `{}` witnesses unknown function `{}`",
                    evidence.id, witness.function
                ));
                continue;
            };
            if evidence.role != TestEvidenceRole::SafeCall {
                errors.push(format!(
                    "non-Safe-call evidence `{}` cannot witness executable calls",
                    evidence.id
                ));
                continue;
            }
            for ((mode, provider), index) in indexed_routes {
                let review = function_exposure_for_provider(function, provider);
                let key = (
                    function.logical_name.clone(),
                    mode.clone(),
                    provider.clone(),
                );
                let mut valid = true;
                if review.classification() != Classification::Safe {
                    errors.push(format!(
                        "evidence `{}` Safe-call witness targets {} function `{}` at route `{mode}/{provider}`",
                        evidence.id,
                        review.classification().as_str(),
                        witness.function
                    ));
                    valid = false;
                }
                if !review.evidence().contains(&evidence.id) {
                    errors.push(format!(
                        "evidence `{}` is not referenced by function `{}` at route `{mode}/{provider}`",
                        evidence.id, witness.function
                    ));
                    valid = false;
                }
                if !review.rust_paths().contains(&witness.rust_path) {
                    errors.push(format!(
                        "evidence `{}` witnesses `{}` through unreviewed Rust path `{}` at route `{mode}/{provider}`",
                        evidence.id, witness.function, witness.rust_path
                    ));
                    valid = false;
                } else if !evidence_invokes_exposure(index, review.exposure(), &witness.rust_path)
                    || !function.link_symbols.get(mode).is_some_and(|symbol| {
                        index.implementation_reachable_symbols.contains(symbol)
                    })
                {
                    errors.push(format!(
                        "evidence `{}` does not establish a must-invoke relation from `{}` to the reviewed physical symbol for `{}` at route `{mode}/{provider}`",
                        evidence.id, witness.rust_path, witness.function
                    ));
                    valid = false;
                }
                if valid {
                    safe_calls
                        .entry(key)
                        .or_default()
                        .insert(evidence.id.as_str());
                }
            }
        }
        for witness in &evidence.classification_witnesses {
            let Some(function) = functions.get(witness.function.as_str()) else {
                errors.push(format!(
                    "evidence `{}` classifies unknown function `{}`",
                    evidence.id, witness.function
                ));
                continue;
            };
            if evidence.role != TestEvidenceRole::FunctionClassificationValidator {
                errors.push(format!(
                    "evidence `{}` has classification witnesses without the classification-validator role",
                    evidence.id
                ));
                continue;
            }
            for (mode, provider) in indexed_routes.keys() {
                let review = function_exposure_for_provider(function, provider);
                let key = (
                    function.logical_name.clone(),
                    mode.clone(),
                    provider.clone(),
                );
                let mut valid = true;
                if review.classification() == Classification::Safe {
                    errors.push(format!(
                        "classification evidence `{}` cannot replace a Safe-call witness for function `{}` at route `{mode}/{provider}`",
                        evidence.id, witness.function
                    ));
                    valid = false;
                } else if witness.classification != review.classification() {
                    errors.push(format!(
                        "classification evidence `{}` records `{}` as {}, but route `{mode}/{provider}` is {}",
                        evidence.id,
                        witness.function,
                        witness.classification.as_str(),
                        review.classification().as_str()
                    ));
                    valid = false;
                }
                if !review.evidence().contains(&evidence.id) {
                    errors.push(format!(
                        "classification evidence `{}` is not referenced by function `{}` at route `{mode}/{provider}`",
                        evidence.id, witness.function
                    ));
                    valid = false;
                }
                if valid {
                    classifications
                        .entry(key)
                        .or_default()
                        .insert(evidence.id.as_str());
                }
            }
        }
    }

    for function in &contract.functions {
        for (mode, provider) in binding_routes.keys() {
            let review = function_exposure_for_provider(function, provider);
            let key = (
                function.logical_name.clone(),
                mode.clone(),
                provider.clone(),
            );
            let declared = review
                .evidence()
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let witnessed = match review.classification() {
                Classification::Safe => safe_calls.get(&key),
                Classification::Raw | Classification::Omitted | Classification::Deferred => {
                    classifications.get(&key)
                }
            }
            .cloned()
            .unwrap_or_default();
            if declared != witnessed || review.evidence().len() != declared.len() {
                errors.push(format!(
                    "{} function `{}` route `{mode}/{provider}` evidence references {:?} do not exactly match typed witnesses {:?}",
                    review.classification().as_str(),
                    function.logical_name,
                    declared,
                    witnessed
                ));
            }
            if witnessed.is_empty() {
                errors.push(format!(
                    "{} function `{}` has no exact route-conditioned witness at `{mode}/{provider}`",
                    review.classification().as_str(),
                    function.logical_name
                ));
            }
        }
    }
}

fn validate_abi_typed_evidence<'a>(
    contract: &'a ApiContract,
    evidence_by_id: &BTreeMap<&str, &TestEvidence>,
    referenced: &mut BTreeSet<&'a str>,
    errors: &mut Vec<String>,
) {
    for policy in &contract.abi.policies {
        referenced.extend(policy.evidence.iter().map(String::as_str));
        let roles = policy
            .evidence
            .iter()
            .filter_map(|id| evidence_by_id.get(id.as_str()))
            .map(|evidence| evidence.role)
            .collect::<BTreeSet<_>>();
        let required = BTreeSet::from([
            TestEvidenceRole::AbiHeaderInventory,
            TestEvidenceRole::AbiBindingAst,
            TestEvidenceRole::AbiContractValidator,
        ]);
        if roles != required {
            errors.push(format!(
                "ABI policy `{}` must reference exactly the header inventory, binding AST, and contract validator evidence roles",
                policy.id
            ));
        }
        let ids = policy
            .evidence
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let required_ids = BTreeSet::from([
            ABI_HEADER_EVIDENCE_ID,
            ABI_BINDING_EVIDENCE_ID,
            ABI_VALIDATOR_EVIDENCE_ID,
        ]);
        if ids != required_ids || policy.evidence.len() != required_ids.len() {
            errors.push(format!(
                "ABI policy `{}` must reference exactly the reviewed ABI evidence rows",
                policy.id
            ));
        }
    }
}

#[allow(
    dead_code,
    reason = "retained temporarily as a schema-7 migration oracle"
)]
fn validate_typed_evidence(
    contract: &ApiContract,
    evidence_by_id: &BTreeMap<&str, &TestEvidence>,
    evidence_indexes: &BTreeMap<&str, BTreeMap<(String, String), TestEvidenceIndex>>,
    errors: &mut Vec<String>,
) {
    let functions = contract
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let mut referenced = BTreeSet::new();
    let mut runtime_evidence_by_function = BTreeMap::<&str, BTreeSet<&str>>::new();
    let mut classification_evidence_by_function = BTreeMap::<&str, BTreeSet<&str>>::new();

    for function in &contract.functions {
        referenced.extend(function.evidence.iter().map(String::as_str));
    }
    for policy in &contract.abi.policies {
        referenced.extend(policy.evidence.iter().map(String::as_str));
        let roles = policy
            .evidence
            .iter()
            .filter_map(|id| evidence_by_id.get(id.as_str()))
            .map(|evidence| evidence.role)
            .collect::<BTreeSet<_>>();
        let required = BTreeSet::from([
            TestEvidenceRole::AbiHeaderInventory,
            TestEvidenceRole::AbiBindingAst,
            TestEvidenceRole::AbiContractValidator,
        ]);
        if roles != required {
            errors.push(format!(
                "ABI policy `{}` must reference exactly the header inventory, binding AST, and contract validator evidence roles",
                policy.id
            ));
        }
        let ids = policy
            .evidence
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let required_ids = BTreeSet::from([
            ABI_HEADER_EVIDENCE_ID,
            ABI_BINDING_EVIDENCE_ID,
            ABI_VALIDATOR_EVIDENCE_ID,
        ]);
        if ids != required_ids || policy.evidence.len() != required_ids.len() {
            errors.push(format!(
                "ABI policy `{}` must reference exactly the reviewed ABI evidence rows",
                policy.id
            ));
        }
    }

    for evidence in &contract.evidence {
        if !referenced.contains(evidence.id.as_str()) {
            errors.push(format!(
                "evidence `{}` is orphaned because no function or ABI policy references it",
                evidence.id
            ));
        }
        if evidence
            .call_witnesses
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            errors.push(format!(
                "evidence `{}` runtime witnesses must be unique and strictly sorted",
                evidence.id
            ));
        }
        if evidence
            .classification_witnesses
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            errors.push(format!(
                "evidence `{}` classification witnesses must be unique and strictly sorted",
                evidence.id
            ));
        }
        if evidence.role == TestEvidenceRole::SafeCall && evidence.call_witnesses.is_empty() {
            errors.push(format!(
                "runtime evidence `{}` has no exact executable call witness",
                evidence.id
            ));
        }
        if evidence.role == TestEvidenceRole::FunctionClassificationValidator
            && evidence.classification_witnesses.is_empty()
        {
            errors.push(format!(
                "classification evidence `{}` has no exact function classification witness",
                evidence.id
            ));
        }
        let indexed = evidence_indexes.get(evidence.id.as_str());
        for witness in &evidence.call_witnesses {
            let Some(function) = functions.get(witness.function.as_str()) else {
                errors.push(format!(
                    "evidence `{}` witnesses unknown function `{}`",
                    evidence.id, witness.function
                ));
                continue;
            };
            if evidence.role != TestEvidenceRole::SafeCall {
                errors.push(format!(
                    "non-runtime evidence `{}` cannot witness executable calls",
                    evidence.id
                ));
                continue;
            }
            if function.classification != Classification::Safe {
                errors.push(format!(
                    "evidence `{}` runtime witness targets non-safe function `{}`",
                    evidence.id, witness.function
                ));
            }
            if !function.evidence.contains(&evidence.id) {
                errors.push(format!(
                    "evidence `{}` is not referenced by witnessed function `{}`",
                    evidence.id, witness.function
                ));
            }
            if !function.rust_paths.contains(&witness.rust_path) {
                errors.push(format!(
                    "evidence `{}` witnesses `{}` through unreviewed Rust path `{}`",
                    evidence.id, witness.function, witness.rust_path
                ));
            } else if !indexed.is_some_and(|routes| {
                !routes.is_empty()
                    && routes.iter().all(|((mode, _provider), indexed)| {
                        evidence_invokes_exposure(indexed, function.exposure, &witness.rust_path)
                            && function.link_symbols.get(mode).is_some_and(|symbol| {
                                indexed.implementation_reachable_symbols.contains(symbol)
                            })
                    })
            }) {
                errors.push(format!(
                    "evidence `{}` does not establish a must-invoke {:?} relation to canonical path `{}` whose implementation reaches the reviewed physical symbol for `{}` on every route",
                    evidence.id, function.exposure, witness.rust_path, witness.function
                ));
            } else {
                runtime_evidence_by_function
                    .entry(function.logical_name.as_str())
                    .or_default()
                    .insert(evidence.id.as_str());
            }
        }
        for witness in &evidence.classification_witnesses {
            let Some(function) = functions.get(witness.function.as_str()) else {
                errors.push(format!(
                    "evidence `{}` classifies unknown function `{}`",
                    evidence.id, witness.function
                ));
                continue;
            };
            if evidence.role != TestEvidenceRole::FunctionClassificationValidator {
                errors.push(format!(
                    "evidence `{}` has classification witnesses without the classification-validator role",
                    evidence.id
                ));
                continue;
            }
            if function.classification == Classification::Safe {
                errors.push(format!(
                    "classification evidence `{}` cannot replace runtime evidence for safe function `{}`",
                    evidence.id, witness.function
                ));
            } else if witness.classification != function.classification {
                errors.push(format!(
                    "classification evidence `{}` records `{}` as {}, but the contract classifies it as {}",
                    evidence.id,
                    witness.function,
                    witness.classification.as_str(),
                    function.classification.as_str()
                ));
            } else if !function.evidence.contains(&evidence.id) {
                errors.push(format!(
                    "classification evidence `{}` is not referenced by function `{}`",
                    evidence.id, witness.function
                ));
            } else {
                classification_evidence_by_function
                    .entry(function.logical_name.as_str())
                    .or_default()
                    .insert(evidence.id.as_str());
            }
        }
    }

    for function in &contract.functions {
        let declared = function
            .evidence
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let witnessed = match function.classification {
            Classification::Safe => runtime_evidence_by_function
                .get(function.logical_name.as_str())
                .cloned()
                .unwrap_or_default(),
            Classification::Raw | Classification::Omitted | Classification::Deferred => {
                classification_evidence_by_function
                    .get(function.logical_name.as_str())
                    .cloned()
                    .unwrap_or_default()
            }
        };
        if declared != witnessed || function.evidence.len() != declared.len() {
            errors.push(format!(
                "{} function `{}` evidence references {:?} do not exactly match typed witnesses {:?}",
                function.classification.as_str(),
                function.logical_name,
                declared,
                witnessed
            ));
        }
        if witnessed.is_empty() {
            match function.classification {
                Classification::Safe => errors.push(format!(
                    "safe function `{}` has no exact executable runtime witness for canonical paths {:?}",
                    function.logical_name, function.rust_paths
                )),
                _ => errors.push(format!(
                    "{} function `{}` has no exact classification witness",
                    function.classification.as_str(),
                    function.logical_name
                )),
            }
        }
    }
}

fn validate_migration(contract: &ApiContract, errors: &mut Vec<String>) {
    let mut reconstructed = contract
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function.classification))
        .collect::<BTreeMap<_, _>>();
    let mut changed = BTreeSet::new();
    for change in &contract.classification_changes {
        if !changed.insert(change.logical_name.as_str()) {
            errors.push(format!(
                "duplicate classification change for `{}`",
                change.logical_name
            ));
        }
        let Some(current) = reconstructed.get_mut(change.logical_name.as_str()) else {
            errors.push(format!(
                "classification change references unknown `{}`",
                change.logical_name
            ));
            continue;
        };
        if *current != change.to {
            errors.push(format!(
                "classification change for `{}` ends at {}, but row is {}",
                change.logical_name,
                change.to.as_str(),
                current.as_str()
            ));
        }
        if change.unit.trim().is_empty() || !has_rationale(&change.rationale) {
            errors.push(format!(
                "classification change for `{}` needs a unit and rationale",
                change.logical_name
            ));
        }
        *current = change.from;
    }
    let mut baseline = CoverageCounts::default();
    for classification in reconstructed.values() {
        baseline.add(*classification);
    }
    if baseline != contract.migration_baseline {
        errors.push(format!(
            "reconstructed migration baseline {baseline:?} does not match {:?}",
            contract.migration_baseline
        ));
    }
}

fn audit_runtime_evidence(paths: &WorkspacePaths) -> Result<()> {
    let manifest = UpstreamManifest::load(paths)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let precision_inventories = load_precision_inventories(paths, &binding_routes)?;
    let rust_indexes =
        load_rust_indexes_with_inventory(paths, &binding_routes, &binding_indexes, &inventory)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let mut contract: ApiContract = read_toml(&contract_path)?;
    reconcile_functions(
        &mut contract,
        &inventory,
        Some(&precision_inventories),
        &binding_routes,
        &rust_indexes,
        &recording_operations,
    );
    let mut gaps =
        synchronize_runtime_evidence(paths, &mut contract, &rust_indexes, &binding_routes)?;
    gaps.sort_by(|left, right| {
        (&left.area, &left.function, &left.rust_paths).cmp(&(
            &right.area,
            &right.function,
            &right.rust_paths,
        ))
    });
    let safe_total = contract
        .functions
        .iter()
        .filter(|function| function.classification == Classification::Safe)
        .count();
    println!(
        "Safe-call witness audit: {} proven, {} gaps, {} Safe functions",
        safe_total - gaps.len(),
        gaps.len(),
        safe_total
    );
    for change in contract
        .classification_changes
        .iter()
        .filter(|change| change.unit == "upstream-contract-transition")
    {
        println!(
            "DOWNGRADED\t{}\t{}\t{}",
            change.logical_name,
            change.from.as_str(),
            change.to.as_str()
        );
    }
    for evidence in contract
        .evidence
        .iter()
        .filter(|evidence| evidence.role == TestEvidenceRole::SafeCall)
    {
        for witness in &evidence.call_witnesses {
            println!(
                "PROVEN\t{}\t{}\t{}\t{}\t{}",
                witness.function, witness.rust_path, evidence.id, evidence.file, evidence.item
            );
        }
    }
    for gap in gaps {
        println!(
            "GAP\t{}\t{}\t{}",
            gap.area,
            gap.function,
            gap.rust_paths.join(",")
        );
    }
    Ok(())
}

fn audit_canonical_paths(paths: &WorkspacePaths) -> Result<()> {
    let manifest = UpstreamManifest::load(paths)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let rust_indexes =
        load_rust_indexes_with_inventory(paths, &binding_routes, &binding_indexes, &inventory)?;
    let contract: ApiContract = read_toml(&contract_path)?;

    for function in contract
        .functions
        .iter()
        .filter(|function| function.classification == Classification::Safe)
    {
        let mut common_candidates: Option<BTreeSet<String>> = None;
        for ((mode, provider), index) in &rust_indexes {
            let Some(symbol) = function.link_symbols.get(mode) else {
                continue;
            };
            let route_candidates = index
                .paths_for_symbol(symbol)
                .filter(|path| safe_exposure_path_exists(index, function.exposure, path))
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            common_candidates = Some(match common_candidates {
                None => route_candidates,
                Some(candidates) => candidates
                    .intersection(&route_candidates)
                    .cloned()
                    .collect(),
            });
            if common_candidates.as_ref().is_some_and(BTreeSet::is_empty) {
                eprintln!(
                    "no common canonical path for `{}` after route `{mode}/{provider}`",
                    function.logical_name
                );
            }
        }
        let candidates = common_candidates.unwrap_or_default();
        let reviewed = function.rust_paths.iter().cloned().collect::<BTreeSet<_>>();
        let status = if !reviewed.is_empty() && reviewed.is_subset(&candidates) {
            "valid"
        } else {
            "invalid"
        };
        println!(
            "CANONICAL\t{}\t{}\t{}\t{}\t{}",
            function.logical_name,
            function.exposure.as_str(),
            status,
            function.rust_paths.join(","),
            candidates.into_iter().collect::<Vec<_>>().join(",")
        );
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReviewedMigrationGap {
    function: String,
    kind: ReviewedMigrationGapKind,
    reviewed_paths: Vec<String>,
    callable_candidates: Vec<String>,
    raii_drop_candidates: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReviewedMigrationGapKind {
    DeclarationDrift,
    Route,
}

fn reviewed_classification_transitions(
    reviewed: &ApiContract,
    active: &ApiContract,
) -> BTreeMap<String, (Classification, Classification)> {
    let active_classifications = active
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function.classification))
        .collect::<BTreeMap<_, _>>();
    reviewed
        .functions
        .iter()
        .filter_map(|historical| {
            let active_classification = active_classifications
                .get(historical.logical_name.as_str())
                .copied()?;
            (active_classification != historical.classification).then(|| {
                (
                    historical.logical_name.clone(),
                    (historical.classification, active_classification),
                )
            })
        })
        .collect()
}

fn audit_reviewed_migration(paths: &WorkspacePaths, revision: &str) -> Result<()> {
    let manifest = UpstreamManifest::load(paths)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let precision_inventories = load_precision_inventories(paths, &binding_routes)?;
    let rust_indexes =
        load_rust_indexes_with_inventory(paths, &binding_routes, &binding_indexes, &inventory)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let mut current: ApiContract = read_toml(&contract_path)?;
    let reviewed = read_api_contract_from_git(paths, &contract_path, revision)?;
    let classification_transitions = reviewed_classification_transitions(&reviewed, &current);

    reconcile_functions(
        &mut current,
        &inventory,
        Some(&precision_inventories),
        &binding_routes,
        &rust_indexes,
        &recording_operations,
    );
    let review_gaps = inherit_reviewed_function_semantics(
        &mut current,
        &reviewed,
        &inventory,
        Some(&precision_inventories),
        &binding_routes,
        &rust_indexes,
        &recording_operations,
    );
    synchronize_classification_evidence(&mut current);
    let mut evidence_gaps =
        synchronize_runtime_evidence(paths, &mut current, &rust_indexes, &binding_routes)?;
    evidence_gaps.sort_by(|left, right| left.function.cmp(&right.function));

    let reviewed_names = reviewed
        .functions
        .iter()
        .map(|function| function.logical_name.as_str())
        .collect::<BTreeSet<_>>();
    let current_names = current
        .functions
        .iter()
        .map(|function| function.logical_name.as_str())
        .collect::<BTreeSet<_>>();
    let classifications = counts(&current.functions);
    println!(
        "reviewed migration audit: {} candidate functions ({} safe, {} raw, {} omitted), {} classification transitions, {} declaration gaps, {} route gaps, {} runtime-evidence gaps, {} added, {} removed",
        classifications.total,
        classifications.safe,
        classifications.raw,
        classifications.omitted,
        classification_transitions.len(),
        review_gaps
            .iter()
            .filter(|gap| gap.kind == ReviewedMigrationGapKind::DeclarationDrift)
            .count(),
        review_gaps
            .iter()
            .filter(|gap| gap.kind == ReviewedMigrationGapKind::Route)
            .count(),
        evidence_gaps.len(),
        current_names.difference(&reviewed_names).count(),
        reviewed_names.difference(&current_names).count(),
    );
    for (logical_name, (historical, active)) in classification_transitions {
        println!(
            "CLASSIFICATION-TRANSITION\t{}\t{}\t{}",
            logical_name,
            historical.as_str(),
            active.as_str()
        );
    }
    let review_gap_names = review_gaps
        .iter()
        .map(|gap| gap.function.as_str())
        .collect::<BTreeSet<_>>();
    for gap in &review_gaps {
        println!(
            "{}\t{}\t{}\tcallable={}\traii-drop={}",
            match gap.kind {
                ReviewedMigrationGapKind::DeclarationDrift => "DECLARATION-GAP",
                ReviewedMigrationGapKind::Route => "ROUTE-GAP",
            },
            gap.function,
            gap.reviewed_paths.join(","),
            gap.callable_candidates.join(","),
            gap.raii_drop_candidates.join(",")
        );
    }
    for function in current.functions.iter().filter(|function| {
        reviewed_names.contains(function.logical_name.as_str())
            && !review_gap_names.contains(function.logical_name.as_str())
            && function.classification == Classification::Safe
            && matches!(
                function.recording,
                Some(RecordingCoverage {
                    class: RecordingClass::LoggedMutation,
                    ..
                })
            )
    }) {
        let declaration = inventory
            .functions
            .iter()
            .find(|declaration| declaration.name == function.logical_name)
            .expect("current function has an inventory declaration");
        let session_paths = common_safe_paths_for_symbol(
            declaration,
            FunctionExposureKind::Callable,
            &binding_routes,
            &rust_indexes,
        )
        .into_iter()
        .filter(|path| is_recording_session_path(path))
        .collect::<Vec<_>>();
        println!(
            "CANONICAL-REFRESH\t{}\t{}",
            function.logical_name,
            session_paths.join(",")
        );
    }
    for gap in evidence_gaps {
        println!(
            "EVIDENCE-GAP\t{}\t{}\t{}",
            gap.area,
            gap.function,
            gap.rust_paths.join(",")
        );
    }
    for function in current
        .functions
        .iter()
        .filter(|function| !reviewed_names.contains(function.logical_name.as_str()))
    {
        let declaration = inventory
            .functions
            .iter()
            .find(|declaration| declaration.name == function.logical_name)
            .expect("current function has an inventory declaration");
        println!(
            "ADDED\t{}\t{}\t{}\tcallable={}\traii-drop={}",
            function.logical_name,
            function.classification.as_str(),
            function.rust_paths.join(","),
            common_safe_paths_for_symbol(
                declaration,
                FunctionExposureKind::Callable,
                &binding_routes,
                &rust_indexes,
            )
            .join(","),
            common_safe_paths_for_symbol(
                declaration,
                FunctionExposureKind::RaiiDrop,
                &binding_routes,
                &rust_indexes,
            )
            .join(",")
        );
    }
    for function in reviewed
        .functions
        .iter()
        .filter(|function| !current_names.contains(function.logical_name.as_str()))
    {
        println!(
            "REMOVED\t{}\t{}\t{}",
            function.logical_name,
            function.classification.as_str(),
            function.rust_paths.join(",")
        );
    }
    Ok(())
}

fn migrate_reviewed_contract(paths: &WorkspacePaths, revision: &str) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let manifest_baseline = fs::read(paths.upstream_manifest())
        .map_err(|source| Error::io(paths.upstream_manifest(), source))?;
    let manifest = UpstreamManifest::load(paths)?;
    let snapshot = validate_repository(paths, &manifest, false)?;
    validate_authenticated_revision(
        &manifest.active_revision,
        &snapshot.gitlink_revision,
        &snapshot.worktree_revision,
    )?;

    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let precision_inventories = load_precision_inventories(paths, &binding_routes)?;
    let rust_indexes =
        load_rust_indexes_with_inventory(paths, &binding_routes, &binding_indexes, &inventory)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let mut contract: ApiContract = read_toml(&contract_path)?;
    let active_contract = contract.clone();
    let reviewed = read_api_contract_from_git(paths, &contract_path, revision)?;
    let override_path = paths.root().join(REVIEWED_MIGRATION_OVERRIDES_PATH);
    let overrides: ReviewedMigrationOverrides = read_toml(&override_path)?;

    if overrides.schema_version != REVIEWED_MIGRATION_SCHEMA {
        return Err(Error::message(format!(
            "{} has unsupported reviewed migration schema {}; expected {}",
            override_path.display(),
            overrides.schema_version,
            REVIEWED_MIGRATION_SCHEMA
        )));
    }
    if overrides.reviewed_revision != revision {
        return Err(Error::message(format!(
            "{} is pinned to reviewed revision `{}`, not requested immutable revision `{revision}`",
            override_path.display(),
            overrides.reviewed_revision
        )));
    }
    if overrides.active_revision != manifest.active_revision {
        return Err(Error::message(format!(
            "{} is pinned to active revision `{}`, not authenticated revision `{}`",
            override_path.display(),
            overrides.active_revision,
            manifest.active_revision
        )));
    }

    reconcile_functions(
        &mut contract,
        &inventory,
        Some(&precision_inventories),
        &binding_routes,
        &rust_indexes,
        &recording_operations,
    );
    let review_gaps = inherit_reviewed_function_semantics(
        &mut contract,
        &reviewed,
        &inventory,
        Some(&precision_inventories),
        &binding_routes,
        &rust_indexes,
        &recording_operations,
    );
    apply_reviewed_migration_overrides(
        &mut contract,
        &reviewed,
        &active_contract,
        &overrides,
        &review_gaps,
        &inventory,
        &binding_routes,
        &rust_indexes,
        &recording_operations,
    )?;

    contract.schema_version = API_CONTRACT_SCHEMA;
    contract.evidence_policy = SAFE_CALL_EVIDENCE_POLICY.to_owned();
    set_active_refresh_identity(&mut contract, &manifest.active_revision);
    contract.function_inventory_digests = compute_function_inventory_digests(&inventory)?;
    recompute_migration_baseline(&mut contract);
    synchronize_classification_evidence(&mut contract);
    let mut safe_call_gaps =
        synchronize_runtime_evidence(paths, &mut contract, &rust_indexes, &binding_routes)?;
    safe_call_gaps.sort_by(|left, right| left.function.cmp(&right.function));
    if !safe_call_gaps.is_empty() {
        return Err(Error::message(format!(
            "reviewed migration has Safe Rust functions without route-conditioned Safe-call witnesses:\n{}",
            safe_call_gaps
                .iter()
                .map(|gap| format!(
                    "{}: {} ({})",
                    gap.area,
                    gap.function,
                    gap.rust_paths.join(", ")
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )));
    }

    let previous_abi = contract.abi.clone();
    let mut generated_abi = map_precision_inventory(
        &inventory,
        &precision_inventories,
        &binding_routes,
        &binding_indexes,
    )?;
    preserve_reviewed_exposure(&previous_abi, &mut generated_abi);
    promote_proven_deferred_exposure(
        &mut generated_abi,
        &inventory,
        &binding_routes,
        &rust_indexes,
    )?;
    discard_unproven_reviewed_exposure(
        &mut generated_abi,
        &inventory,
        &binding_routes,
        &rust_indexes,
    );
    contract.abi = generated_abi;
    refresh_route_scoped_evidence_metadata(paths, &mut contract, &rust_indexes, &binding_routes)?;
    validate_contract(
        paths,
        &contract,
        &inventory,
        Some(&precision_inventories),
        &rust_indexes,
        &binding_routes,
        &binding_indexes,
        &manifest.active_revision,
        &recording_operations,
    )?;

    let observed_counts = counts(&contract.functions);
    if observed_counts != overrides.expected_counts {
        return Err(Error::message(format!(
            "reviewed migration coverage counts drifted: expected {:?}, observed {:?}",
            overrides.expected_counts, observed_counts
        )));
    }
    let recording_source_git_blobs = manifest.recording_source_git_blobs();
    let recording_sources_aggregate =
        reviewed_sources_aggregate_blake3(&recording_source_git_blobs)?;
    let wire = generate_wire_contract(
        &manifest.recording_revision,
        &recording_operations,
        &recording_source_git_blobs,
        &recording_sources_aggregate,
    )?;
    let recording_wire = render_toml(&wire)?.into_bytes();
    let wire_digest = blake3::hash(&recording_wire).to_hex().to_string();
    let effective_source_sha256 = effective_source_sha256(paths)?;
    let runtime_recording_wire =
        render_runtime_parser(&wire, &wire_digest, &effective_source_sha256)?.into_bytes();
    let writes = [
        ManagedArtifactWrite::reviewed_active("api-contract", render_toml(&contract)?.into_bytes()),
        ManagedArtifactWrite::active("recording-wire", recording_wire),
        ManagedArtifactWrite::active("api-coverage-report", render_report(&contract).into_bytes()),
        ManagedArtifactWrite::auxiliary(RUNTIME_RECORDING_WIRE_PATH, runtime_recording_wire),
    ];
    install_managed_artifact_writes_locked(paths, &writes, Some(&manifest_baseline), || {
        validate_managed_repository_and_api(paths)
    })?;
    println!(
        "migrated reviewed API contract: {} functions ({} safe, {} raw, {} omitted)",
        observed_counts.total, observed_counts.safe, observed_counts.raw, observed_counts.omitted
    );
    Ok(())
}

fn synchronize_abi_evidence_scopes(contract: &mut ApiContract, binding_routes: &AbiBindingRoutes) {
    let modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for evidence in &mut contract.evidence {
        if matches!(
            evidence.role,
            TestEvidenceRole::AbiHeaderInventory
                | TestEvidenceRole::AbiBindingAst
                | TestEvidenceRole::AbiContractValidator
        ) {
            evidence.modes.clone_from(&modes);
            evidence.providers.clone_from(&providers);
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the migration joins immutable review history, current ABI facts, and route proofs"
)]
fn apply_reviewed_migration_overrides(
    contract: &mut ApiContract,
    reviewed: &ApiContract,
    active: &ApiContract,
    overrides: &ReviewedMigrationOverrides,
    review_gaps: &[ReviewedMigrationGap],
    inventory: &CApiInventory,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
    recording_operations: &[crate::recording_ops::RecordingOp],
) -> Result<()> {
    let reviewed_by_name = reviewed
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let historical_changes = reviewed.classification_changes.clone();
    let current_names = contract
        .functions
        .iter()
        .map(|function| function.logical_name.clone())
        .collect::<BTreeSet<_>>();
    let reviewed_names = reviewed_by_name
        .keys()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    let active_classifications = active
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function.classification))
        .collect::<BTreeMap<_, _>>();
    let active_changes = active
        .classification_changes
        .iter()
        .map(|change| (change.logical_name.as_str(), change))
        .collect::<BTreeMap<_, _>>();
    let active_transitions = reviewed_classification_transitions(reviewed, active);
    let mut expected_overrides = review_gaps
        .iter()
        .map(|gap| gap.function.clone())
        .collect::<BTreeSet<_>>();
    let declaration_gaps = review_gaps
        .iter()
        .filter(|gap| gap.kind == ReviewedMigrationGapKind::DeclarationDrift)
        .map(|gap| gap.function.as_str())
        .collect::<BTreeSet<_>>();
    expected_overrides.extend(current_names.difference(&reviewed_names).cloned());
    expected_overrides.extend(
        active_transitions
            .keys()
            .filter(|logical_name| current_names.contains(*logical_name))
            .cloned(),
    );

    let mut override_names = BTreeSet::new();
    for function in &overrides.functions {
        if !override_names.insert(function.logical_name.clone()) {
            return Err(Error::message(format!(
                "duplicate reviewed migration override `{}`",
                function.logical_name
            )));
        }
    }
    let missing = expected_overrides
        .difference(&override_names)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = override_names
        .difference(&expected_overrides)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(Error::message(format!(
            "reviewed migration overrides must exactly cover classification transitions, declaration gaps, route gaps, and added functions; missing={missing:?}, unexpected={unexpected:?}"
        )));
    }

    let declarations = inventory
        .functions
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    for reviewed_override in &overrides.functions {
        if let Some((historical, active)) =
            active_transitions.get(reviewed_override.logical_name.as_str())
            && (reviewed_override.previous_classification != Some(*historical)
                || reviewed_override.classification != *active)
        {
            return Err(Error::message(format!(
                "active classification transition for `{}` requires previous_classification={} and classification={}",
                reviewed_override.logical_name,
                historical.as_str(),
                active.as_str()
            )));
        }
        let requires_revalidation =
            declaration_gaps.contains(reviewed_override.logical_name.as_str());
        if reviewed_override.revalidated != requires_revalidation {
            return Err(Error::message(format!(
                "reviewed migration override `{}` must set revalidated={} for its authenticated declaration state",
                reviewed_override.logical_name, requires_revalidation
            )));
        }
        if reviewed_override.area.trim().is_empty() || reviewed_override.rationale.trim().is_empty()
        {
            return Err(Error::message(format!(
                "reviewed migration override `{}` requires non-empty area and rationale",
                reviewed_override.logical_name
            )));
        }
        if reviewed_override
            .transition_unit
            .as_ref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(Error::message(format!(
                "reviewed migration override `{}` has an empty classification transition unit",
                reviewed_override.logical_name
            )));
        }
        if reviewed_override.transition_unit.is_some()
            && !reviewed_override
                .previous_classification
                .is_some_and(|previous| previous != reviewed_override.classification)
        {
            return Err(Error::message(format!(
                "reviewed migration override `{}` may set a transition unit only for a classification change",
                reviewed_override.logical_name
            )));
        }
        if reviewed_override.area.contains("Unreviewed upstream")
            || reviewed_override
                .rationale
                .contains("until its Safe Rust semantics are reviewed")
        {
            return Err(Error::message(format!(
                "reviewed migration override `{}` contains placeholder review text",
                reviewed_override.logical_name
            )));
        }
        let declaration = declarations
            .get(reviewed_override.logical_name.as_str())
            .ok_or_else(|| {
                Error::message(format!(
                    "reviewed migration override `{}` is absent from the current header inventory",
                    reviewed_override.logical_name
                ))
            })?;
        let function = contract
            .functions
            .iter_mut()
            .find(|function| function.logical_name == reviewed_override.logical_name)
            .expect("override names were proven to be current functions");
        function.classification = reviewed_override.classification;
        function.exposure = reviewed_override.exposure;
        function.area.clone_from(&reviewed_override.area);
        function.rationale.clone_from(&reviewed_override.rationale);
        function
            .rust_paths
            .clone_from(&reviewed_override.rust_paths);
        function.rust_paths.sort();
        function.rust_paths.dedup();
        synchronize_wasm_function_overrides(function, declaration, binding_routes, rust_indexes);
        match function.classification {
            Classification::Safe => {
                if !safe_function_review_matches_routes(
                    function,
                    declaration,
                    binding_routes,
                    rust_indexes,
                ) {
                    return Err(Error::message(format!(
                        "reviewed Safe Rust override `{}` does not prove every configured route",
                        function.logical_name
                    )));
                }
            }
            Classification::Raw => {
                if !reviewed_override.rust_paths.is_empty()
                    || reviewed_override.exposure != FunctionExposureKind::Callable
                {
                    return Err(Error::message(format!(
                        "raw override `{}` must omit Rust paths and use callable exposure",
                        function.logical_name
                    )));
                }
                function.rust_paths = function
                    .link_symbols
                    .values()
                    .map(|symbol| format!("boxdd_sys::ffi::{symbol}"))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
            }
            Classification::Omitted | Classification::Deferred => {
                return Err(Error::message(format!(
                    "reviewed migration override `{}` cannot introduce {} coverage",
                    function.logical_name,
                    function.classification.as_str()
                )));
            }
        }
        function.recording = super::api_recording::expected(
            &function.logical_name,
            function.classification,
            recording_operations,
        );
        if matches!(
            function.recording,
            Some(RecordingCoverage {
                class: RecordingClass::LoggedMutation,
                ..
            })
        ) && !function
            .rust_paths
            .iter()
            .any(|path| is_recording_session_path(path))
        {
            return Err(Error::message(format!(
                "reviewed logged-mutation override `{}` must use a RecordingSession canonical path",
                function.logical_name
            )));
        }

        if requires_revalidation && reviewed_override.previous_classification.is_none() {
            return Err(Error::message(format!(
                "declaration revalidation for `{}` must authenticate its previous classification",
                function.logical_name
            )));
        }
        if let Some(previous) = reviewed_override.previous_classification {
            let historical = reviewed_by_name
                .get(function.logical_name.as_str())
                .ok_or_else(|| {
                    Error::message(format!(
                        "added function `{}` cannot claim a previous classification",
                        function.logical_name
                    ))
                })?;
            if historical.classification != previous
                || (previous == function.classification && !reviewed_override.revalidated)
            {
                return Err(Error::message(format!(
                    "classification transition for `{}` is not authenticated by the immutable reviewed contract",
                    function.logical_name
                )));
            }
            if previous != function.classification {
                let active_classification = active_classifications
                    .get(function.logical_name.as_str())
                    .ok_or_else(|| {
                        Error::message(format!(
                            "classification transition for `{}` is absent from the active reviewed contract",
                            function.logical_name
                        ))
                    })?;
                if *active_classification != function.classification {
                    return Err(Error::message(format!(
                        "classification transition for `{}` targets {}, but the active reviewed contract authenticates {}",
                        function.logical_name,
                        function.classification.as_str(),
                        active_classification.as_str()
                    )));
                }
                let preserved_change =
                    active_changes
                        .get(function.logical_name.as_str())
                        .filter(|change| {
                            change.from == previous && change.to == function.classification
                        });
                let change = if let Some(unit) = &reviewed_override.transition_unit {
                    ClassificationChange {
                        logical_name: function.logical_name.clone(),
                        from: previous,
                        to: function.classification,
                        unit: unit.clone(),
                        rationale: function.rationale.clone(),
                    }
                } else if let Some(change) = preserved_change {
                    (*change).clone()
                } else {
                    ClassificationChange {
                        logical_name: function.logical_name.clone(),
                        from: previous,
                        to: function.classification,
                        unit: "box2d-3.2-reviewed-migration".to_owned(),
                        rationale: function.rationale.clone(),
                    }
                };
                changes.push(change);
            }
        }
    }
    changes.sort_by(|left, right| left.logical_name.cmp(&right.logical_name));
    apply_reviewed_canonical_refreshes(
        contract,
        &reviewed_by_name,
        overrides,
        &expected_overrides,
        &declarations,
        binding_routes,
        rust_indexes,
        recording_operations,
    )?;

    let final_classifications = contract
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function.classification))
        .collect::<BTreeMap<_, _>>();
    let mut all_changes = historical_changes
        .into_iter()
        .filter(|change| {
            final_classifications
                .get(change.logical_name.as_str())
                .is_some_and(|classification| *classification == change.to)
        })
        .map(|change| (change.logical_name.clone(), change))
        .collect::<BTreeMap<_, _>>();
    for change in changes {
        all_changes.insert(change.logical_name.clone(), change);
    }
    contract.classification_changes = all_changes.into_values().collect();

    for function in &contract.functions {
        if function.area.contains("Unreviewed upstream")
            || function
                .rationale
                .contains("until its Safe Rust semantics are reviewed")
        {
            return Err(Error::message(format!(
                "function `{}` still contains placeholder review text after migration",
                function.logical_name
            )));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "canonical refresh validation joins historical review, current headers, route proofs, and canonical semantics"
)]
fn apply_reviewed_canonical_refreshes(
    contract: &mut ApiContract,
    reviewed_by_name: &BTreeMap<&str, &FunctionContract>,
    overrides: &ReviewedMigrationOverrides,
    required_overrides: &BTreeSet<String>,
    declarations: &BTreeMap<&str, &crate::c_api::FunctionDecl>,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
    recording_operations: &[crate::recording_ops::RecordingOp],
) -> Result<()> {
    let unchanged_safe_functions = contract
        .functions
        .iter()
        .filter(|function| !required_overrides.contains(&function.logical_name))
        .filter(|function| function.classification == Classification::Safe)
        .filter(|function| {
            reviewed_by_name
                .get(function.logical_name.as_str())
                .is_some_and(|historical| historical.classification == Classification::Safe)
        });
    let expected_recording_refresh_names = unchanged_safe_functions
        .clone()
        .filter(|function| {
            matches!(
                super::api_recording::expected(
                    &function.logical_name,
                    Classification::Safe,
                    recording_operations,
                ),
                Some(RecordingCoverage {
                    class: RecordingClass::LoggedMutation,
                    ..
                })
            )
        })
        .map(|function| function.logical_name.clone())
        .collect::<BTreeSet<_>>();
    let expected_default_refresh_owners = unchanged_safe_functions
        .filter_map(|function| {
            let historical = reviewed_by_name.get(function.logical_name.as_str())?;
            reviewed_default_owner(historical)
                .map(|owner| (function.logical_name.clone(), owner.to_owned()))
        })
        .collect::<BTreeMap<_, _>>();
    let expected_refresh_names = expected_recording_refresh_names
        .iter()
        .cloned()
        .chain(expected_default_refresh_owners.keys().cloned())
        .collect::<BTreeSet<_>>();
    let supplied_refresh_names = overrides
        .canonical_refreshes
        .iter()
        .map(|refresh| refresh.logical_name.clone())
        .collect::<BTreeSet<_>>();
    if supplied_refresh_names.len() != overrides.canonical_refreshes.len() {
        return Err(Error::message(
            "reviewed canonical refresh list contains duplicate logical names",
        ));
    }
    let missing = expected_refresh_names
        .difference(&supplied_refresh_names)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = supplied_refresh_names
        .difference(&expected_refresh_names)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(Error::message(format!(
            "reviewed canonical refreshes must exactly cover unchanged Safe logged mutations and default constructors; missing={missing:?}, unexpected={unexpected:?}"
        )));
    }

    for refresh in &overrides.canonical_refreshes {
        if required_overrides.contains(refresh.logical_name.as_str()) {
            return Err(Error::message(format!(
                "canonical refresh `{}` must be an unchanged historical function, not a required migration override",
                refresh.logical_name
            )));
        }
        if refresh.rationale.trim().is_empty()
            || refresh
                .rationale
                .contains("until its Safe Rust semantics are reviewed")
            || refresh.rust_paths.is_empty()
        {
            return Err(Error::message(format!(
                "canonical refresh `{}` requires reviewed rationale and at least one Rust path",
                refresh.logical_name
            )));
        }
        let historical = reviewed_by_name
            .get(refresh.logical_name.as_str())
            .ok_or_else(|| {
                Error::message(format!(
                    "canonical refresh `{}` is absent from the immutable reviewed contract",
                    refresh.logical_name
                ))
            })?;
        let declaration = declarations
            .get(refresh.logical_name.as_str())
            .ok_or_else(|| {
                Error::message(format!(
                    "canonical refresh `{}` is absent from the current header inventory",
                    refresh.logical_name
                ))
            })?;
        let function = contract
            .functions
            .iter_mut()
            .find(|function| function.logical_name == refresh.logical_name)
            .expect("canonical refresh was proven to be a current function");
        let recording = super::api_recording::expected(
            &refresh.logical_name,
            Classification::Safe,
            recording_operations,
        );
        if historical.classification != Classification::Safe
            || function.classification != Classification::Safe
        {
            return Err(Error::message(format!(
                "canonical refresh `{}` must preserve historical and current Safe classification",
                refresh.logical_name
            )));
        }
        let is_recording_refresh =
            expected_recording_refresh_names.contains(refresh.logical_name.as_str());
        let default_owner = expected_default_refresh_owners.get(refresh.logical_name.as_str());
        match (is_recording_refresh, default_owner) {
            (true, None) => {
                if !refresh
                    .rust_paths
                    .iter()
                    .all(|path| is_recording_session_path(path))
                {
                    return Err(Error::message(format!(
                        "logged-mutation canonical refresh `{}` requires RecordingSession paths",
                        refresh.logical_name
                    )));
                }
            }
            (false, Some(owner)) => {
                if refresh.rust_paths.len() != 1
                    || !is_reviewed_default_constructor_path(&refresh.rust_paths[0], owner)
                {
                    return Err(Error::message(format!(
                        "default canonical refresh `{}` must replace `{owner}::default` with exactly one same-owner `new` or `builder` path",
                        refresh.logical_name
                    )));
                }
            }
            (true, Some(_)) => {
                return Err(Error::message(format!(
                    "canonical refresh `{}` is ambiguously classified as both a logged mutation and a default constructor",
                    refresh.logical_name
                )));
            }
            (false, None) => {
                return Err(Error::message(format!(
                    "canonical refresh `{}` is outside the computed reviewed refresh set",
                    refresh.logical_name
                )));
            }
        }

        function.exposure = FunctionExposureKind::Callable;
        function.rust_paths.clone_from(&refresh.rust_paths);
        function.rust_paths.sort();
        function.rust_paths.dedup();
        function.rationale.clone_from(&refresh.rationale);
        function.recording = recording;
        if !safe_function_review_matches_routes(function, declaration, binding_routes, rust_indexes)
        {
            return Err(Error::message(format!(
                "reviewed canonical refresh `{}` does not prove every configured route",
                refresh.logical_name
            )));
        }
    }
    Ok(())
}

fn reviewed_default_owner(function: &FunctionContract) -> Option<&str> {
    let native_owner = function.logical_name.strip_prefix("b2Default")?;
    let [rust_path] = function.rust_paths.as_slice() else {
        return None;
    };
    let owner = rust_path.strip_suffix("::default")?;
    (owner.starts_with("boxdd::")
        && !native_owner.is_empty()
        && owner.rsplit("::").next() == Some(native_owner))
    .then_some(owner)
}

fn is_reviewed_default_constructor_path(path: &str, owner: &str) -> bool {
    path.strip_prefix(owner)
        .and_then(|suffix| suffix.strip_prefix("::"))
        .is_some_and(|constructor| matches!(constructor, "new" | "builder"))
}

fn is_recording_session_path(path: &str) -> bool {
    path.starts_with("boxdd::RecordingSession::")
        || path.starts_with("boxdd::recording::RecordingSession::")
}

fn read_api_contract_from_git(
    paths: &WorkspacePaths,
    contract_path: &std::path::Path,
    revision: &str,
) -> Result<ApiContract> {
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::message(
            "reviewed migration revision must be an immutable 40-hex commit",
        ));
    }
    let object_type = Command::new("git")
        .current_dir(paths.root())
        .args(["--no-replace-objects", "cat-file", "-t", revision])
        .output()
        .map_err(|source| Error::io(paths.root().join(".git"), source))?;
    if !object_type.status.success()
        || String::from_utf8_lossy(&object_type.stdout).trim() != "commit"
    {
        return Err(Error::message(format!(
            "reviewed migration revision `{revision}` is not an immutable commit object"
        )));
    }
    let verified_revision = format!("{revision}^{{commit}}");
    let verification = Command::new("git")
        .current_dir(paths.root())
        .args([
            "--no-replace-objects",
            "rev-parse",
            "--verify",
            "--end-of-options",
            &verified_revision,
        ])
        .output()
        .map_err(|source| Error::io(paths.root().join(".git"), source))?;
    if !verification.status.success()
        || String::from_utf8_lossy(&verification.stdout).trim() != revision
    {
        return Err(Error::message(format!(
            "reviewed migration revision `{revision}` could not be authenticated as an exact commit"
        )));
    }
    let relative = contract_path.strip_prefix(paths.root()).map_err(|_| {
        Error::message(format!(
            "API contract {} is outside the workspace root {}",
            contract_path.display(),
            paths.root().display()
        ))
    })?;
    let object = format!(
        "{revision}:{}",
        relative.to_string_lossy().replace('\\', "/")
    );
    let output = Command::new("git")
        .current_dir(paths.root())
        .args(["--no-replace-objects", "show", "--no-textconv", &object])
        .output()
        .map_err(|source| Error::io(paths.root().join(".git"), source))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "could not read reviewed API contract `{object}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let source = std::str::from_utf8(&output.stdout).map_err(|error| {
        Error::message(format!(
            "reviewed API contract `{object}` is not UTF-8: {error}"
        ))
    })?;
    let contract = toml::from_str::<ApiContract>(source).map_err(|error| {
        Error::message(format!(
            "reviewed API contract `{object}` is invalid TOML: {error}"
        ))
    })?;
    if contract.schema_version == 0 || contract.schema_version > API_CONTRACT_SCHEMA {
        return Err(Error::message(format!(
            "reviewed API contract `{object}` has unsupported schema {}",
            contract.schema_version
        )));
    }
    let mut names = BTreeSet::new();
    for function in &contract.functions {
        if !names.insert(function.logical_name.as_str()) {
            return Err(Error::message(format!(
                "reviewed API contract `{object}` contains duplicate function `{}`",
                function.logical_name
            )));
        }
    }
    if names.is_empty() {
        return Err(Error::message(format!(
            "reviewed API contract `{object}` contains no functions"
        )));
    }
    Ok(contract)
}

fn inherit_reviewed_function_semantics(
    current: &mut ApiContract,
    reviewed: &ApiContract,
    inventory: &CApiInventory,
    precision_inventories: Option<&AbiPrecisionInventories>,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
    recording_operations: &[crate::recording_ops::RecordingOp],
) -> Vec<ReviewedMigrationGap> {
    let declarations = inventory
        .functions
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect::<BTreeMap<_, _>>();
    let reviewed_by_name = reviewed
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let mut review_gaps = Vec::new();

    for function in &mut current.functions {
        let Some(previous) = reviewed_by_name.get(function.logical_name.as_str()) else {
            continue;
        };
        let Some(declaration) = declarations.get(function.logical_name.as_str()) else {
            continue;
        };
        if !reviewed_function_metadata_matches(previous, function, precision_inventories) {
            review_gaps.push(ReviewedMigrationGap {
                function: function.logical_name.clone(),
                kind: ReviewedMigrationGapKind::DeclarationDrift,
                reviewed_paths: previous.rust_paths.clone(),
                callable_candidates: common_safe_paths_for_symbol(
                    declaration,
                    FunctionExposureKind::Callable,
                    binding_routes,
                    rust_indexes,
                ),
                raii_drop_candidates: common_safe_paths_for_symbol(
                    declaration,
                    FunctionExposureKind::RaiiDrop,
                    binding_routes,
                    rust_indexes,
                ),
            });
            continue;
        }
        let mut candidate = function.clone();
        candidate.classification = previous.classification;
        candidate.exposure = previous.exposure;
        candidate.area.clone_from(&previous.area);
        candidate.rust_paths.clone_from(&previous.rust_paths);
        candidate.rationale.clone_from(&previous.rationale);
        candidate.recording = super::api_recording::expected(
            &candidate.logical_name,
            candidate.classification,
            recording_operations,
        );
        if candidate.classification == Classification::Raw {
            candidate.rust_paths = candidate
                .link_symbols
                .values()
                .map(|symbol| format!("boxdd_sys::ffi::{symbol}"))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        } else if matches!(
            candidate.classification,
            Classification::Omitted | Classification::Deferred
        ) {
            candidate.rust_paths.clear();
        }
        synchronize_wasm_function_overrides(
            &mut candidate,
            declaration,
            binding_routes,
            rust_indexes,
        );

        if candidate.classification == Classification::Safe
            && !safe_function_review_matches_routes(
                &candidate,
                declaration,
                binding_routes,
                rust_indexes,
            )
        {
            review_gaps.push(ReviewedMigrationGap {
                function: candidate.logical_name.clone(),
                kind: ReviewedMigrationGapKind::Route,
                reviewed_paths: candidate.rust_paths.clone(),
                callable_candidates: common_safe_paths_for_symbol(
                    declaration,
                    FunctionExposureKind::Callable,
                    binding_routes,
                    rust_indexes,
                ),
                raii_drop_candidates: common_safe_paths_for_symbol(
                    declaration,
                    FunctionExposureKind::RaiiDrop,
                    binding_routes,
                    rust_indexes,
                ),
            });
            continue;
        }
        *function = candidate;
    }
    review_gaps.sort_by(|left, right| left.function.cmp(&right.function));
    review_gaps
}

fn reviewed_function_metadata_matches(
    reviewed: &FunctionContract,
    current: &FunctionContract,
    precision_inventories: Option<&AbiPrecisionInventories>,
) -> bool {
    reviewed.signature == current.signature
        && reviewed.fingerprint == current.fingerprint
        && reviewed.availability == current.availability
        && reviewed
            .link_symbols
            .iter()
            .all(|(mode, symbol)| current.link_symbols.get(mode) == Some(symbol))
        && (precision_inventories.is_none()
            || reviewed
                .abi_fingerprints
                .iter()
                .all(|(mode, fingerprint)| current.abi_fingerprints.get(mode) == Some(fingerprint)))
}

fn common_safe_paths_for_symbol(
    declaration: &crate::c_api::FunctionDecl,
    exposure: FunctionExposureKind,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
) -> Vec<String> {
    let mut common: Option<BTreeSet<String>> = None;
    for coordinate in binding_routes.keys() {
        let Some(index) = rust_indexes.get(coordinate) else {
            return Vec::new();
        };
        let Some(symbol) = declaration.physical_symbols.get(&coordinate.0) else {
            return Vec::new();
        };
        let candidates = index
            .paths_for_symbol(symbol)
            .filter(|path| safe_exposure_path_exists(index, exposure, path))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        common = Some(match common {
            Some(previous) => previous.intersection(&candidates).cloned().collect(),
            None => candidates,
        });
    }
    common.unwrap_or_default().into_iter().collect()
}

fn refresh_abi_contract(
    paths: &WorkspacePaths,
    reviewed_contract_blake3: Option<&str>,
) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let manifest_baseline = fs::read(paths.upstream_manifest())
        .map_err(|source| Error::io(paths.upstream_manifest(), source))?;
    let manifest = UpstreamManifest::load(paths)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let reviewed_contract = read_reviewed_contract_snapshot(&contract_path)?;
    if manifest.artifact_digests_initialized {
        let expected = match reviewed_contract_blake3 {
            Some(expected) => expected,
            None => manifest
                .artifact(ArtifactKind::ApiContract)?
                .content_blake3
                .as_str(),
        };
        let preflight_manifest =
            reviewed_contract_preflight_manifest(&manifest, &reviewed_contract.bytes, expected)?;
        validate_repository(paths, &preflight_manifest, false)?;
    } else if reviewed_contract_blake3.is_some() {
        return Err(Error::message(
            "--reviewed-contract-blake3 is only valid for an initialized artifact manifest",
        ));
    }
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let contract = build_refreshed_contract(
        paths,
        &manifest,
        &recording_operations,
        reviewed_contract.contract,
    )?;
    let recording_source_git_blobs = manifest.recording_source_git_blobs();
    let recording_sources_aggregate =
        reviewed_sources_aggregate_blake3(&recording_source_git_blobs)?;
    let wire = generate_wire_contract(
        &manifest.recording_revision,
        &recording_operations,
        &recording_source_git_blobs,
        &recording_sources_aggregate,
    )?;
    let recording_wire = render_toml(&wire)?.into_bytes();
    let wire_digest = blake3::hash(&recording_wire).to_hex().to_string();
    let effective_source_sha256 = effective_source_sha256(paths)?;
    let runtime_recording_wire =
        render_runtime_parser(&wire, &wire_digest, &effective_source_sha256)?.into_bytes();
    let contract_content = render_toml(&contract)?.into_bytes();
    let contract_write = match reviewed_contract_blake3 {
        Some(baseline_blake3) => ManagedArtifactWrite::reviewed_active_with_baseline_blake3(
            "api-contract",
            contract_content,
            baseline_blake3,
        ),
        None => ManagedArtifactWrite::reviewed_active("api-contract", contract_content),
    };
    let writes = [
        contract_write,
        ManagedArtifactWrite::active("recording-wire", recording_wire),
        ManagedArtifactWrite::active("api-coverage-report", render_report(&contract).into_bytes()),
        ManagedArtifactWrite::auxiliary(RUNTIME_RECORDING_WIRE_PATH, runtime_recording_wire),
    ];
    install_managed_artifact_writes_locked(paths, &writes, Some(&manifest_baseline), || {
        validate_managed_repository_and_api(paths)
    })?;
    println!(
        "refreshed active reviewed ABI contract {}: {} structs and {} callbacks",
        contract_path.display(),
        contract.abi.structs.len(),
        contract.abi.callbacks.len()
    );
    Ok(())
}

struct ReviewedContractSnapshot {
    bytes: Vec<u8>,
    contract: ApiContract,
}

fn read_reviewed_contract_snapshot(path: &std::path::Path) -> Result<ReviewedContractSnapshot> {
    let bytes = fs::read(path).map_err(|source| Error::io(path, source))?;
    let source = std::str::from_utf8(&bytes)
        .map_err(|error| Error::message(format!("{} is not UTF-8: {error}", path.display())))?;
    let contract = toml::from_str(source)
        .map_err(|error| Error::message(format!("{}: invalid TOML: {error}", path.display())))?;
    Ok(ReviewedContractSnapshot { bytes, contract })
}

fn reviewed_contract_preflight_manifest(
    manifest: &UpstreamManifest,
    reviewed_contract: &[u8],
    expected_blake3: &str,
) -> Result<UpstreamManifest> {
    if expected_blake3.len() != 64
        || !expected_blake3
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::message(
            "--reviewed-contract-blake3 must be a lowercase 64-character BLAKE3 digest",
        ));
    }

    let observed_blake3 = blake3::hash(reviewed_contract).to_hex().to_string();
    if observed_blake3 != expected_blake3 {
        return Err(Error::message(format!(
            "reviewed API contract BLAKE3 mismatch: expected {expected_blake3}, observed {observed_blake3}"
        )));
    }

    manifest.artifact(ArtifactKind::ApiContract)?;
    let mut preflight_manifest = manifest.clone();
    let artifact = preflight_manifest
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.kind == ArtifactKind::ApiContract)
        .expect("validated API contract artifact exists");
    artifact.content_blake3 = observed_blake3;
    Ok(preflight_manifest)
}

fn validate_managed_repository_and_api(paths: &WorkspacePaths) -> Result<()> {
    let manifest = UpstreamManifest::load(paths)?;
    validate_repository(paths, &manifest, false)?;
    check(paths)
}

pub(crate) fn render_refreshed_contract_candidate(paths: &WorkspacePaths) -> Result<Vec<u8>> {
    let manifest = UpstreamManifest::load(paths)?;
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let reviewed_contract = read_reviewed_contract_snapshot(&contract_path)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let contract = build_refreshed_contract(
        paths,
        &manifest,
        &recording_operations,
        reviewed_contract.contract,
    )?;
    Ok(render_toml(&contract)?.into_bytes())
}

fn build_refreshed_contract(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
    recording_operations: &[crate::recording_ops::RecordingOp],
    mut contract: ApiContract,
) -> Result<ApiContract> {
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, manifest)?;
    let binding_routes = load_binding_routes(manifest)?;
    let precision_inventories = load_precision_inventories(paths, &binding_routes)?;
    let rust_indexes =
        load_rust_indexes_with_inventory(paths, &binding_routes, &binding_indexes, &inventory)?;
    let bootstrap_legacy_precision =
        contract.schema_version == 4 && contract.upstream_sha == manifest.active_revision;
    if !matches!(contract.schema_version, 4 | 5 | 6 | 7 | API_CONTRACT_SCHEMA) {
        return Err(Error::message(format!(
            "cannot refresh API contract schema {} into schema {API_CONTRACT_SCHEMA}",
            contract.schema_version
        )));
    }
    if bootstrap_legacy_precision {
        bootstrap_legacy_function_precision_proofs(
            &mut contract,
            &inventory,
            &precision_inventories,
            &binding_routes,
        );
    }

    contract.schema_version = API_CONTRACT_SCHEMA;
    contract.evidence_policy = SAFE_CALL_EVIDENCE_POLICY.to_owned();
    set_active_refresh_identity(&mut contract, &manifest.active_revision);
    reconcile_functions(
        &mut contract,
        &inventory,
        Some(&precision_inventories),
        &binding_routes,
        &rust_indexes,
        recording_operations,
    );
    contract.function_inventory_digests = compute_function_inventory_digests(&inventory)?;
    synchronize_classification_evidence(&mut contract);
    let _runtime_gaps =
        synchronize_runtime_evidence(paths, &mut contract, &rust_indexes, &binding_routes)?;
    let mut previous_abi = contract.abi.clone();
    let mut generated_abi = map_precision_inventory(
        &inventory,
        &precision_inventories,
        &binding_routes,
        &binding_indexes,
    )?;
    if bootstrap_legacy_precision {
        bootstrap_legacy_precision_proofs(&mut previous_abi, &generated_abi);
    }
    preserve_reviewed_exposure(&previous_abi, &mut generated_abi);
    promote_proven_deferred_exposure(
        &mut generated_abi,
        &inventory,
        &binding_routes,
        &rust_indexes,
    )?;
    discard_unproven_reviewed_exposure(
        &mut generated_abi,
        &inventory,
        &binding_routes,
        &rust_indexes,
    );
    contract.abi = generated_abi;
    refresh_route_scoped_evidence_metadata(paths, &mut contract, &rust_indexes, &binding_routes)?;
    validate_contract(
        paths,
        &contract,
        &inventory,
        Some(&precision_inventories),
        &rust_indexes,
        &binding_routes,
        &binding_indexes,
        &manifest.active_revision,
        recording_operations,
    )?;
    Ok(contract)
}

fn bootstrap_legacy_function_precision_proofs(
    contract: &mut ApiContract,
    inventory: &CApiInventory,
    precision_inventories: &AbiPrecisionInventories,
    binding_routes: &AbiBindingRoutes,
) {
    let declarations = inventory
        .functions
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect::<BTreeMap<_, _>>();
    let modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.clone())
        .collect::<BTreeSet<_>>();
    let providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.clone())
        .collect::<BTreeSet<_>>();

    for function in &mut contract.functions {
        if !function.abi_fingerprints.is_empty()
            || function.modes.iter().cloned().collect::<BTreeSet<_>>() != modes
            || function.providers.iter().cloned().collect::<BTreeSet<_>>() != providers
        {
            continue;
        }
        let Some(declaration) = declarations.get(function.logical_name.as_str()) else {
            continue;
        };
        let link_symbols = modes
            .iter()
            .filter_map(|mode| {
                declaration
                    .physical_symbols
                    .get(mode)
                    .map(|symbol| (mode.clone(), symbol.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        if function.signature != declaration.signature
            || function.fingerprint != declaration.fingerprint
            || function.availability != declaration.availability
            || function.link_symbols != link_symbols
        {
            continue;
        }
        let fingerprints = modes
            .iter()
            .filter_map(|mode| {
                precision_inventories
                    .get(mode)
                    .and_then(|inventory| inventory.function(&function.logical_name))
                    .map(|declaration| (mode.clone(), declaration.fingerprint.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        if fingerprints.len() == modes.len() {
            function.abi_fingerprints = fingerprints;
        }
    }
}

fn reconcile_functions(
    contract: &mut ApiContract,
    inventory: &CApiInventory,
    precision_inventories: Option<&AbiPrecisionInventories>,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
    recording_operations: &[crate::recording_ops::RecordingOp],
) {
    let modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.clone())
        .collect::<BTreeSet<_>>();
    let providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.clone())
        .collect::<BTreeSet<_>>();
    let mut previous = std::mem::take(&mut contract.functions)
        .into_iter()
        .map(|function| (function.logical_name.clone(), function))
        .collect::<BTreeMap<_, _>>();
    let mut exact_reviewed_rows = BTreeSet::new();
    let mut generated_changes = Vec::new();
    let mut reconciled = Vec::with_capacity(inventory.functions.len());

    for declaration in &inventory.functions {
        let existing = previous.remove(&declaration.name);
        let current_abi_fingerprints = precision_inventories
            .map(|inventories| {
                modes
                    .iter()
                    .filter_map(|mode| {
                        inventories
                            .get(mode)
                            .and_then(|inventory| inventory.function(&declaration.name))
                            .map(|function| (mode.clone(), function.fingerprint.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        let current_link_symbols = modes
            .iter()
            .filter_map(|mode| {
                declaration
                    .physical_symbols
                    .get(mode)
                    .map(|symbol| (mode.clone(), symbol.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let exact = existing.as_ref().is_some_and(|function| {
            function.signature == declaration.signature
                && function.fingerprint == declaration.fingerprint
                && function.availability == declaration.availability
                && function.link_symbols == current_link_symbols
                && (function.classification != Classification::Safe
                    || precision_inventories.is_none()
                    || (current_abi_fingerprints.len() == modes.len()
                        && function.abi_fingerprints == current_abi_fingerprints))
                && safe_function_review_matches_native_routes(
                    function,
                    declaration,
                    binding_routes,
                    rust_indexes,
                )
        });
        let mut function = if exact {
            exact_reviewed_rows.insert(declaration.name.clone());
            existing.expect("exact reviewed row exists")
        } else {
            if let Some(previous) = existing
                && previous.classification != Classification::Raw
            {
                generated_changes.push(ClassificationChange {
                    logical_name: declaration.name.clone(),
                    from: previous.classification,
                    to: Classification::Raw,
                    unit: "upstream-contract-transition".to_owned(),
                    rationale: format!(
                        "The upstream declaration, availability, physical symbol, or Safe Rust route proof for `{}` changed, so its previous review no longer applies and the capability is conservatively raw.",
                        declaration.name
                    ),
                });
            }
            FunctionContract {
                logical_name: declaration.name.clone(),
                signature: declaration.signature.clone(),
                fingerprint: declaration.fingerprint.clone(),
                abi_fingerprints: BTreeMap::new(),
                link_symbols: BTreeMap::new(),
                classification: Classification::Raw,
                exposure: FunctionExposureKind::Callable,
                area: format!("Unreviewed upstream {}", declaration.header),
                rust_paths: Vec::new(),
                rationale: format!(
                    "The new or changed upstream function `{}` is conservatively exposed only through raw FFI until its Safe Rust semantics are reviewed.",
                    declaration.name
                ),
                modes: Vec::new(),
                providers: Vec::new(),
                availability: Vec::new(),
                evidence: vec![API_CLASSIFICATION_EVIDENCE_ID.to_owned()],
                provider_overrides: Vec::new(),
                recording: None,
            }
        };

        function.signature.clone_from(&declaration.signature);
        function.fingerprint.clone_from(&declaration.fingerprint);
        if precision_inventories.is_some() {
            function.abi_fingerprints = current_abi_fingerprints;
        }
        function.modes = modes.iter().cloned().collect();
        function.providers = providers.iter().cloned().collect();
        function.availability.clone_from(&declaration.availability);
        function.link_symbols = current_link_symbols;
        if function.classification == Classification::Raw {
            function.rust_paths = function
                .link_symbols
                .values()
                .map(|symbol| format!("boxdd_sys::ffi::{symbol}"))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
        synchronize_wasm_function_overrides(
            &mut function,
            declaration,
            binding_routes,
            rust_indexes,
        );
        function.recording = super::api_recording::expected(
            &function.logical_name,
            function.classification,
            recording_operations,
        );
        reconciled.push(function);
    }

    contract.functions = reconciled;
    let reconciled_classifications = contract
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function.classification))
        .collect::<BTreeMap<_, _>>();
    contract.classification_changes.retain(|change| {
        exact_reviewed_rows.contains(&change.logical_name)
            && reconciled_classifications.get(change.logical_name.as_str()) == Some(&change.to)
    });
    contract.classification_changes.extend(generated_changes);
    recompute_migration_baseline(contract);
}

fn safe_function_review_matches_routes(
    function: &FunctionContract,
    declaration: &crate::c_api::FunctionDecl,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
) -> bool {
    if function.classification != Classification::Safe {
        return true;
    }
    binding_routes.keys().all(|coordinate| {
        let review = function_exposure_for_provider(function, &coordinate.1);
        review.classification() != Classification::Safe
            || safe_function_review_matches_coordinate(
                review.rust_paths(),
                review.exposure(),
                declaration,
                coordinate,
                rust_indexes,
            )
    })
}

fn safe_function_review_matches_native_routes(
    function: &FunctionContract,
    declaration: &crate::c_api::FunctionDecl,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
) -> bool {
    function.classification != Classification::Safe
        || binding_routes
            .keys()
            .filter(|(_, provider)| !is_wasm_provider(provider))
            .all(|coordinate| {
                safe_function_review_matches_coordinate(
                    &function.rust_paths,
                    function.exposure,
                    declaration,
                    coordinate,
                    rust_indexes,
                )
            })
}

fn safe_function_review_matches_coordinate(
    rust_paths: &[String],
    exposure: FunctionExposureKind,
    declaration: &crate::c_api::FunctionDecl,
    coordinate: &(String, String),
    rust_indexes: &AbiRustIndexes,
) -> bool {
    !rust_paths.is_empty()
        && rust_indexes.get(coordinate).is_some_and(|index| {
            let Some(symbol) = declaration.physical_symbols.get(&coordinate.0) else {
                return false;
            };
            rust_paths.iter().all(|path| {
                safe_exposure_path_exists(index, exposure, path)
                    && index.path_reaches_symbol(path, symbol)
            })
        })
}

fn synchronize_wasm_function_overrides(
    function: &mut FunctionContract,
    declaration: &crate::c_api::FunctionDecl,
    binding_routes: &AbiBindingRoutes,
    rust_indexes: &AbiRustIndexes,
) {
    if function.classification != Classification::Safe {
        function.provider_overrides.clear();
        return;
    }
    let downgraded = binding_routes
        .keys()
        .filter(|(_, provider)| is_wasm_provider(provider))
        .filter(|coordinate| {
            !safe_function_review_matches_coordinate(
                &function.rust_paths,
                function.exposure,
                declaration,
                coordinate,
                rust_indexes,
            )
        })
        .map(|(_, provider)| provider.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if downgraded.is_empty() {
        function.provider_overrides.clear();
        return;
    }
    let rust_paths = function
        .modes
        .iter()
        .filter_map(|mode| declaration.physical_symbols.get(mode))
        .map(|symbol| format!("boxdd_sys::ffi::{symbol}"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    function.provider_overrides = vec![FunctionProviderOverride {
        providers: downgraded,
        classification: Classification::Raw,
        rust_paths,
        rationale: "The route-conditioned Rust index proves this Safe adapter on native providers, but its callback-bearing path is cfg-disabled on these WASM providers, where only exact raw FFI remains available."
            .to_owned(),
        evidence: Vec::new(),
    }];
}

fn is_wasm_provider(provider: &str) -> bool {
    matches!(provider, "wasm-runtime" | "wasm-compile-only")
}

fn safe_exposure_path_exists(
    index: &RustIndex,
    exposure: FunctionExposureKind,
    path: &str,
) -> bool {
    match exposure {
        FunctionExposureKind::Callable => index.contains_public_safe_callable_path(path),
        FunctionExposureKind::RaiiDrop => index.contains_public_type_path(path),
    }
}

fn evidence_invokes_exposure(
    index: &TestEvidenceIndex,
    exposure: FunctionExposureKind,
    path: &str,
) -> bool {
    match exposure {
        FunctionExposureKind::Callable => index.called_public_paths.contains(path),
        FunctionExposureKind::RaiiDrop => index.dropped_public_types.contains(path),
    }
}

fn synchronize_classification_evidence(contract: &mut ApiContract) {
    type Scope = (Vec<String>, Vec<String>);
    let all_providers = contract
        .functions
        .first()
        .map_or_else(Vec::new, |function| function.providers.clone());
    let mut grouped = BTreeMap::<Scope, Vec<FunctionClassificationWitness>>::new();
    for function in &mut contract.functions {
        let overridden = function
            .provider_overrides
            .iter()
            .flat_map(|provider_override| provider_override.providers.iter().cloned())
            .collect::<BTreeSet<_>>();
        let default_providers = function
            .providers
            .iter()
            .filter(|provider| !overridden.contains(*provider))
            .cloned()
            .collect::<Vec<_>>();
        if function.classification != Classification::Safe {
            let scope = (function.modes.clone(), default_providers);
            let evidence_id = classification_evidence_id(&scope, &all_providers);
            function.evidence = vec![evidence_id];
            grouped
                .entry(scope)
                .or_default()
                .push(FunctionClassificationWitness {
                    function: function.logical_name.clone(),
                    classification: function.classification,
                });
        }
        for provider_override in &mut function.provider_overrides {
            if provider_override.classification == Classification::Safe {
                continue;
            }
            let scope = (function.modes.clone(), provider_override.providers.clone());
            let evidence_id = classification_evidence_id(&scope, &all_providers);
            provider_override.evidence = vec![evidence_id];
            grouped
                .entry(scope)
                .or_default()
                .push(FunctionClassificationWitness {
                    function: function.logical_name.clone(),
                    classification: provider_override.classification,
                });
        }
    }

    let previous_fingerprints = contract
        .evidence
        .iter()
        .filter(|evidence| evidence.role == TestEvidenceRole::FunctionClassificationValidator)
        .map(|evidence| (evidence.id.clone(), evidence.fingerprint.clone()))
        .collect::<BTreeMap<_, _>>();
    contract
        .evidence
        .retain(|evidence| evidence.role != TestEvidenceRole::FunctionClassificationValidator);
    for ((modes, providers), mut witnesses) in grouped {
        witnesses.sort();
        witnesses.dedup();
        let scope = (modes, providers);
        let id = classification_evidence_id(&scope, &all_providers);
        contract.evidence.push(TestEvidence {
            fingerprint: previous_fingerprints.get(&id).cloned().unwrap_or_default(),
            id,
            file: "xtask/src/commands/api_coverage.rs".to_owned(),
            item: "typed_function_classification_evidence_rejects_unrelated_subjects".to_owned(),
            package: "xtask".to_owned(),
            gate: "nextest".to_owned(),
            role: TestEvidenceRole::FunctionClassificationValidator,
            modes: scope.0,
            providers: scope.1,
            call_witnesses: Vec::new(),
            classification_witnesses: witnesses,
        });
    }
}

fn classification_evidence_id(
    scope: &(Vec<String>, Vec<String>),
    all_providers: &[String],
) -> String {
    if scope.1 == all_providers {
        return API_CLASSIFICATION_EVIDENCE_ID.to_owned();
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"boxdd-function-classification-scope-v1\0");
    for value in scope.0.iter().chain(&scope.1) {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    format!("api-classification-{}", &hasher.finalize().to_hex()[..16])
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeEvidenceGap {
    area: String,
    function: String,
    rust_paths: Vec<String>,
}

fn synchronize_runtime_evidence(
    paths: &WorkspacePaths,
    contract: &mut ApiContract,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<Vec<RuntimeEvidenceGap>> {
    #[derive(Clone)]
    struct SafeReview {
        function: FunctionContract,
        modes: Vec<String>,
        providers: Vec<String>,
    }

    let all_providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let safe_reviews = contract
        .functions
        .iter()
        .filter(|function| function.classification == Classification::Safe)
        .map(|function| {
            let overridden = function
                .provider_overrides
                .iter()
                .flat_map(|provider_override| provider_override.providers.iter().cloned())
                .collect::<BTreeSet<_>>();
            SafeReview {
                function: function.clone(),
                modes: function.modes.clone(),
                providers: function
                    .providers
                    .iter()
                    .filter(|provider| !overridden.contains(*provider))
                    .cloned()
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let scopes = safe_reviews
        .iter()
        .map(|review| (review.modes.clone(), review.providers.clone()))
        .collect::<BTreeSet<_>>();
    let mut candidates = Vec::<TestEvidence>::new();
    let mut matches = BTreeMap::<String, Vec<(usize, SafeCallWitness)>>::new();

    for discovered in discover_test_evidence_items(paths.root())?
        .into_iter()
        .filter(|item| item.package == "boxdd")
    {
        for (modes, providers) in &scopes {
            let mut evidence = TestEvidence {
                id: safe_call_evidence_id(
                    &discovered.file,
                    &discovered.item,
                    modes,
                    providers,
                    &all_providers,
                ),
                file: discovered.file.clone(),
                item: discovered.item.clone(),
                package: discovered.package.clone(),
                gate: discovered.gate.clone(),
                role: TestEvidenceRole::SafeCall,
                fingerprint: String::new(),
                modes: modes.clone(),
                providers: providers.clone(),
                call_witnesses: Vec::new(),
                classification_witnesses: Vec::new(),
            };
            let Ok(indexed_routes) =
                index_evidence_across_routes(paths, &evidence, rust_indexes, binding_routes)
            else {
                continue;
            };
            evidence.fingerprint = aggregate_evidence_fingerprint(&indexed_routes);
            let candidate_index = candidates.len();
            for review in safe_reviews
                .iter()
                .filter(|review| review.modes == *modes && review.providers == *providers)
            {
                let function = &review.function;
                let Some(rust_path) = function.rust_paths.iter().find(|rust_path| {
                    indexed_routes.iter().all(|((mode, _provider), indexed)| {
                        evidence_invokes_exposure(indexed, function.exposure, rust_path)
                            && function.link_symbols.get(mode).is_some_and(|symbol| {
                                indexed.implementation_reachable_symbols.contains(symbol)
                            })
                    })
                }) else {
                    continue;
                };
                matches
                    .entry(function.logical_name.clone())
                    .or_default()
                    .push((
                        candidate_index,
                        SafeCallWitness {
                            function: function.logical_name.clone(),
                            rust_path: rust_path.clone(),
                        },
                    ));
            }
            candidates.push(evidence);
        }
    }

    let existing_references = contract
        .functions
        .iter()
        .map(|function| {
            (
                function.logical_name.clone(),
                function.evidence.iter().cloned().collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut selected = BTreeMap::<String, (usize, SafeCallWitness)>::new();
    for review in &safe_reviews {
        let function = &review.function;
        let Some(options) = matches.get_mut(&function.logical_name) else {
            continue;
        };
        let reviewed = existing_references
            .get(&function.logical_name)
            .expect("reviewed function evidence");
        options.sort_by_key(|(index, witness)| {
            (
                !reviewed.contains(&candidates[*index].id),
                candidates[*index].id.clone(),
                witness.rust_path.clone(),
            )
        });
        selected.insert(function.logical_name.clone(), options[0].clone());
    }

    let mut witnesses_by_candidate = BTreeMap::<usize, Vec<SafeCallWitness>>::new();
    for (candidate, witness) in selected.values() {
        witnesses_by_candidate
            .entry(*candidate)
            .or_default()
            .push(witness.clone());
    }
    let evidence_id_by_function = selected
        .iter()
        .map(|(function, (candidate, _))| (function.as_str(), candidates[*candidate].id.clone()))
        .collect::<BTreeMap<_, _>>();
    for function in &mut contract.functions {
        if function.classification == Classification::Safe {
            function.evidence = evidence_id_by_function
                .get(function.logical_name.as_str())
                .cloned()
                .into_iter()
                .collect();
        }
    }

    let mut rows = witnesses_by_candidate
        .into_iter()
        .map(|(candidate, mut witnesses)| {
            witnesses.sort();
            witnesses.dedup();
            let mut evidence = candidates[candidate].clone();
            evidence.call_witnesses = witnesses;
            evidence
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    contract
        .evidence
        .retain(|evidence| evidence.role != TestEvidenceRole::SafeCall);
    contract.evidence.extend(rows);

    Ok(safe_reviews
        .into_iter()
        .filter(|review| !selected.contains_key(&review.function.logical_name))
        .map(|review| RuntimeEvidenceGap {
            area: review.function.area,
            function: review.function.logical_name,
            rust_paths: review.function.rust_paths,
        })
        .collect())
}

fn safe_call_evidence_id(
    file: &str,
    item: &str,
    modes: &[String],
    providers: &[String],
    all_providers: &[String],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"boxdd-safe-call-evidence-v2\0");
    for value in std::iter::once(file)
        .chain(std::iter::once(item))
        .chain(modes.iter().map(String::as_str))
        .chain(providers.iter().map(String::as_str))
    {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    if providers == all_providers {
        let legacy = blake3::hash(format!("{file}\0{item}").as_bytes());
        format!("runtime-auto-{}", &legacy.to_hex()[..16])
    } else {
        format!("safe-call-auto-{}", &hasher.finalize().to_hex()[..16])
    }
}

#[allow(
    dead_code,
    reason = "retained temporarily as a schema-7 migration oracle"
)]
fn synchronize_runtime_evidence_legacy(
    paths: &WorkspacePaths,
    contract: &mut ApiContract,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<Vec<RuntimeEvidenceGap>> {
    let safe_functions = contract
        .functions
        .iter()
        .filter(|function| function.classification == Classification::Safe)
        .cloned()
        .collect::<Vec<_>>();
    let reviewed_ids = contract
        .evidence
        .iter()
        .filter(|evidence| evidence.role == TestEvidenceRole::SafeCall)
        .map(|evidence| {
            (
                (evidence.file.clone(), evidence.item.clone()),
                evidence.id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates = Vec::<TestEvidence>::new();
    let mut matches = BTreeMap::<String, Vec<(usize, SafeCallWitness)>>::new();

    for discovered in discover_test_evidence_items(paths.root())?
        .into_iter()
        .filter(|item| item.package == "boxdd")
    {
        let id = reviewed_ids
            .get(&(discovered.file.clone(), discovered.item.clone()))
            .cloned()
            .unwrap_or_else(|| runtime_evidence_id(&discovered.file, &discovered.item));
        let mut evidence = TestEvidence {
            id,
            file: discovered.file,
            item: discovered.item,
            package: discovered.package,
            gate: discovered.gate,
            role: TestEvidenceRole::SafeCall,
            fingerprint: String::new(),
            modes: binding_routes
                .keys()
                .map(|(mode, _)| mode.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            providers: binding_routes
                .keys()
                .map(|(_, provider)| provider.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            call_witnesses: Vec::new(),
            classification_witnesses: Vec::new(),
        };
        let Ok(indexed_routes) =
            index_evidence_across_routes(paths, &evidence, rust_indexes, binding_routes)
        else {
            continue;
        };
        evidence.fingerprint = aggregate_evidence_fingerprint(&indexed_routes);
        let candidate_index = candidates.len();
        for function in &safe_functions {
            let Some(rust_path) = function.rust_paths.iter().find(|rust_path| {
                indexed_routes.iter().all(|((mode, _provider), indexed)| {
                    evidence_invokes_exposure(indexed, function.exposure, rust_path)
                        && function.link_symbols.get(mode).is_some_and(|symbol| {
                            indexed.implementation_reachable_symbols.contains(symbol)
                        })
                })
            }) else {
                continue;
            };
            matches
                .entry(function.logical_name.clone())
                .or_default()
                .push((
                    candidate_index,
                    SafeCallWitness {
                        function: function.logical_name.clone(),
                        rust_path: rust_path.clone(),
                    },
                ));
        }
        candidates.push(evidence);
    }

    let existing_references = contract
        .functions
        .iter()
        .map(|function| {
            (
                function.logical_name.clone(),
                function.evidence.iter().cloned().collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut selected = BTreeMap::<String, (usize, SafeCallWitness)>::new();
    for function in &safe_functions {
        let Some(options) = matches.get_mut(&function.logical_name) else {
            continue;
        };
        let reviewed = existing_references
            .get(&function.logical_name)
            .expect("reviewed function evidence");
        options.sort_by_key(|(index, witness)| {
            (
                !reviewed.contains(&candidates[*index].id),
                candidates[*index].id.clone(),
                witness.rust_path.clone(),
            )
        });
        selected.insert(function.logical_name.clone(), options[0].clone());
    }

    let mut witnesses_by_candidate = BTreeMap::<usize, Vec<SafeCallWitness>>::new();
    for (candidate, witness) in selected.values() {
        witnesses_by_candidate
            .entry(*candidate)
            .or_default()
            .push(witness.clone());
    }
    let evidence_id_by_function = selected
        .iter()
        .map(|(function, (candidate, _))| (function.as_str(), candidates[*candidate].id.clone()))
        .collect::<BTreeMap<_, _>>();
    for function in &mut contract.functions {
        if function.classification == Classification::Safe {
            function.evidence = evidence_id_by_function
                .get(function.logical_name.as_str())
                .cloned()
                .into_iter()
                .collect();
        }
    }

    let mut runtime_rows = witnesses_by_candidate
        .into_iter()
        .map(|(candidate, mut witnesses)| {
            witnesses.sort();
            let mut evidence = candidates[candidate].clone();
            evidence.call_witnesses = witnesses;
            evidence
        })
        .collect::<Vec<_>>();
    runtime_rows.sort_by(|left, right| left.id.cmp(&right.id));
    contract
        .evidence
        .retain(|evidence| evidence.role != TestEvidenceRole::SafeCall);
    contract.evidence.extend(runtime_rows);

    Ok(safe_functions
        .into_iter()
        .filter(|function| !selected.contains_key(&function.logical_name))
        .map(|function| RuntimeEvidenceGap {
            area: function.area,
            function: function.logical_name,
            rust_paths: function.rust_paths,
        })
        .collect())
}

fn runtime_evidence_id(file: &str, item: &str) -> String {
    let digest = blake3::hash(format!("{file}\0{item}").as_bytes());
    format!("runtime-auto-{}", &digest.to_hex()[..16])
}

fn recompute_migration_baseline(contract: &mut ApiContract) {
    let mut classifications = contract
        .functions
        .iter()
        .map(|function| (function.logical_name.as_str(), function.classification))
        .collect::<BTreeMap<_, _>>();
    for change in &contract.classification_changes {
        if let Some(classification) = classifications.get_mut(change.logical_name.as_str()) {
            *classification = change.from;
        }
    }
    let mut baseline = CoverageCounts::default();
    for classification in classifications.values() {
        baseline.add(*classification);
    }
    contract.migration_baseline = baseline;
}

fn set_active_refresh_identity(contract: &mut ApiContract, active_revision: &str) {
    contract.upstream_sha = active_revision.to_owned();
}

fn refresh_route_scoped_evidence_metadata(
    paths: &WorkspacePaths,
    contract: &mut ApiContract,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<()> {
    synchronize_abi_evidence_scopes(contract, binding_routes);
    refresh_evidence_metadata(paths, contract, rust_indexes, binding_routes)
}

fn refresh_evidence_metadata(
    paths: &WorkspacePaths,
    contract: &mut ApiContract,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<()> {
    let mut expected = contract
        .functions
        .iter()
        .flat_map(|function| {
            function.evidence.iter().cloned().chain(
                function
                    .provider_overrides
                    .iter()
                    .flat_map(|provider_override| provider_override.evidence.iter().cloned()),
            )
        })
        .chain(
            contract
                .abi
                .policies
                .iter()
                .flat_map(|policy| policy.evidence.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for evidence in &mut contract.evidence {
        if !seen.insert(evidence.id.clone()) {
            return Err(Error::message(format!(
                "duplicate evidence id `{}`",
                evidence.id
            )));
        }
        if !expected.remove(&evidence.id) {
            return Err(Error::message(format!(
                "evidence `{}` is orphaned because no function or ABI policy references it",
                evidence.id
            )));
        }
        let indexed_routes =
            index_evidence_across_routes(paths, evidence, rust_indexes, binding_routes)?;
        evidence.fingerprint = aggregate_evidence_fingerprint(&indexed_routes);
    }
    if !expected.is_empty() {
        return Err(Error::message(format!(
            "contract references missing evidence rows: {:?}",
            expected.iter().collect::<Vec<_>>()
        )));
    }
    Ok(())
}

fn abi_function_symbols(
    inventory: &CApiInventory,
    routes: &AbiBindingRoutes,
) -> AbiFunctionSymbols {
    let mut symbols = AbiFunctionSymbols::new();
    for (mode, provider) in routes.keys() {
        for function in &inventory.functions {
            let Some(symbol) = function.physical_symbols.get(mode) else {
                continue;
            };
            symbols
                .entry((mode.clone(), provider.clone()))
                .or_default()
                .insert(function.name.clone(), symbol.clone());
        }
    }
    symbols
}

fn load_binding_indexes(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
) -> Result<AbiBindingIndexes> {
    let mut indexes = AbiBindingIndexes::new();
    for artifact in &manifest.artifacts {
        if artifact.kind != ArtifactKind::Bindings {
            continue;
        }
        let precision = artifact.precision.ok_or_else(|| {
            Error::message(format!(
                "binding artifact `{}` has no precision coordinate",
                artifact.name
            ))
        })?;
        let binding = AbiBindingIndex::from_path(
            artifact.name.clone(),
            precision,
            artifact.target,
            artifact.provider,
            &paths.root().join(&artifact.path),
        )?;
        if indexes.insert(artifact.name.clone(), binding).is_some() {
            return Err(Error::message(format!(
                "duplicate binding artifact `{}`",
                artifact.name
            )));
        }
    }
    if indexes.is_empty() {
        return Err(Error::message(
            "upstream manifest has no indexable binding artifacts",
        ));
    }
    Ok(indexes)
}

fn load_binding_routes(manifest: &UpstreamManifest) -> Result<AbiBindingRoutes> {
    let mut routes = AbiBindingRoutes::new();
    for manifest_route in &manifest.binding_routes {
        let artifact = manifest.binding_route_artifact(manifest_route)?;
        let route = AbiBindingRoute {
            mode: manifest_route.mode.as_str().to_owned(),
            provider: manifest_route.provider.as_str().to_owned(),
            artifact: artifact.name.clone(),
            rust_target: manifest_route.rust_target,
            rust_features: manifest_route.rust_features.clone(),
        };
        if routes
            .insert((route.mode.clone(), route.provider.clone()), route)
            .is_some()
        {
            return Err(Error::message(
                "upstream manifest has duplicate executable binding routes",
            ));
        }
    }
    Ok(routes)
}

fn load_precision_inventories(
    paths: &WorkspacePaths,
    binding_routes: &AbiBindingRoutes,
) -> Result<AbiPrecisionInventories> {
    let modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.as_str())
        .collect::<BTreeSet<_>>();
    let mut inventories = AbiPrecisionInventories::new();
    for mode in modes {
        let precision = match mode {
            "single" => CAbiPrecision::Single,
            "double" => CAbiPrecision::Double,
            other => {
                return Err(Error::message(format!(
                    "binding route has unsupported C ABI precision mode `{other}`"
                )));
            }
        };
        let inventory = parse_headers_for_precision(&paths.box2d_headers(), precision)?;
        inventories.insert(mode.to_owned(), inventory);
    }
    if inventories.is_empty() {
        return Err(Error::message(
            "cannot build precision-aware C ABI inventory without binding routes",
        ));
    }
    Ok(inventories)
}

fn load_rust_indexes(
    paths: &WorkspacePaths,
    routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    inventory: Option<&CApiInventory>,
) -> Result<AbiRustIndexes> {
    let mut coordinates = BTreeMap::new();
    let mut ffi_type_hints = BTreeMap::new();
    for ((mode, provider), route) in routes {
        let binding = binding_indexes.get(&route.artifact).ok_or_else(|| {
            Error::message(format!(
                "binding route `{mode}/{provider}` references unindexed artifact `{}`",
                route.artifact
            ))
        })?;
        let target_matches_artifact = match route.rust_target {
            RustTarget::X86_64UnknownLinuxGnu => matches!(
                binding.target,
                ArtifactTarget::Universal | ArtifactTarget::Native
            ),
            RustTarget::Wasm32UnknownUnknown => {
                binding.target == ArtifactTarget::Wasm32UnknownUnknown
            }
            RustTarget::Wasm32Wasip1 => binding.target == ArtifactTarget::Wasm32Wasip1,
        };
        if !target_matches_artifact {
            return Err(Error::message(format!(
                "binding route `{mode}/{provider}` Rust target {} is incompatible with artifact target {}",
                route.rust_target.as_str(),
                binding.target.as_str()
            )));
        }
        let expanded_features = expanded_binding_route_features(paths, &route.rust_features)?;
        let coordinate = rust_index_coordinate(route.rust_target)
            .with_cfg_values("feature", expanded_features.iter());
        if coordinates
            .insert((mode.clone(), provider.clone()), coordinate)
            .is_some()
        {
            return Err(Error::message(format!(
                "duplicate Safe Rust coordinate for binding route `{}/{}`",
                route.mode, route.provider
            )));
        }
        let mut route_hints = RustFfiTypeHints::default();
        if let Some(inventory) = inventory {
            for function in &inventory.functions {
                let Some(physical_symbol) = function.physical_symbols.get(mode) else {
                    continue;
                };
                let physical_path = format!("boxdd_sys::ffi::{physical_symbol}");
                if let Some(return_type) =
                    binding.index.function_return_type_path(&physical_path)?
                {
                    route_hints.insert_function_return(
                        format!("boxdd_sys::ffi::{}", function.name),
                        return_type,
                    );
                }
            }
        }
        ffi_type_hints.insert((mode.clone(), provider.clone()), route_hints);
    }
    index_boxdd_routes_with_ffi_hints(paths.root(), &coordinates, &ffi_type_hints)
}

fn load_rust_indexes_with_inventory(
    paths: &WorkspacePaths,
    routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    inventory: &CApiInventory,
) -> Result<AbiRustIndexes> {
    let mut indexes = load_rust_indexes(paths, routes, binding_indexes, Some(inventory))?;
    for ((mode, _provider), index) in &mut indexes {
        index.add_symbol_aliases(inventory.functions.iter().filter_map(|function| {
            function
                .physical_symbols
                .get(mode)
                .map(|physical| (function.name.clone(), physical.clone()))
        }));
    }
    Ok(indexes)
}

fn rust_index_coordinate(target: RustTarget) -> RustIndexCoordinate {
    match target {
        RustTarget::X86_64UnknownLinuxGnu => RustIndexCoordinate::source_for_target(
            "x86_64",
            "linux",
            ["unix"],
            "little",
            "64",
            "unwind",
        )
        .with_cfg_values("target_abi", [""])
        .with_cfg_values("target_env", ["gnu"])
        .with_cfg_values("target_vendor", ["unknown"]),
        RustTarget::Wasm32UnknownUnknown => RustIndexCoordinate::wasm32_unknown_unknown(),
        RustTarget::Wasm32Wasip1 => RustIndexCoordinate::wasm32_wasip1(),
    }
}

fn validate_recording_contract(
    path: &std::path::Path,
    operations: &[crate::recording_ops::RecordingOp],
    expected_revision: &str,
    expected_source_git_blobs: &BTreeMap<String, String>,
    expected_sources_aggregate_blake3: &str,
) -> Result<()> {
    let contract: RecordingWireContract = read_toml(path)?;
    validate_wire_contract(
        &contract,
        operations,
        expected_revision,
        expected_source_git_blobs,
        expected_sources_aggregate_blake3,
    )
}

fn validate_registry_values(
    function: &str,
    label: &str,
    values: &[String],
    allowed: &BTreeSet<&str>,
    errors: &mut Vec<String>,
) {
    if values.is_empty() {
        errors.push(format!("`{function}` declares no {label} values"));
        return;
    }
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            errors.push(format!("`{function}` repeats {label} `{value}`"));
        }
        if !allowed.contains(value.as_str()) {
            errors.push(format!("`{function}` has unknown {label} `{value}`"));
        }
    }
}

fn counts(functions: &[FunctionContract]) -> CoverageCounts {
    let mut counts = CoverageCounts::default();
    for function in functions {
        counts.add(function.classification);
    }
    counts
}

fn function_route_counts(
    functions: &[FunctionContract],
) -> BTreeMap<(String, String), CoverageCounts> {
    let coordinates = functions
        .iter()
        .flat_map(|function| {
            function.modes.iter().flat_map(|mode| {
                function
                    .providers
                    .iter()
                    .map(move |provider| (mode.clone(), provider.clone()))
            })
        })
        .collect::<BTreeSet<_>>();
    coordinates
        .into_iter()
        .map(|coordinate| {
            let mut counts = CoverageCounts::default();
            for function in functions.iter().filter(|function| {
                function.modes.contains(&coordinate.0) && function.providers.contains(&coordinate.1)
            }) {
                counts
                    .add(function_exposure_for_provider(function, &coordinate.1).classification());
            }
            (coordinate, counts)
        })
        .collect()
}

fn render_report(contract: &ApiContract) -> String {
    let counts = counts(&contract.functions);
    let mut by_area: BTreeMap<&str, CoverageCounts> = BTreeMap::new();
    for function in &contract.functions {
        by_area
            .entry(&function.area)
            .or_default()
            .add(function.classification);
    }
    let mut output = String::new();
    writeln!(output, "# Box2D API Coverage\n").expect("write to string");
    writeln!(
        output,
        "<!-- api-coverage: total={} safe={} raw={} omitted={} deferred={} -->\n",
        counts.total, counts.safe, counts.raw, counts.omitted, counts.deferred
    )
    .expect("write to string");
    writeln!(output, "This file is generated from the API artifact named by `boxdd-sys/upstream.toml`. The contract is validated against the exact vendored headers, canonical public Rust paths, real `#[test]` evidence, provider modes, precision-specific link symbols, explicit recording capability classes, and ABI struct/callback fingerprints.\n").expect("write to string");
    writeln!(
        output,
        "Pinned active upstream: `{}`.\n",
        contract.upstream_sha
    )
    .expect("write to string");
    writeln!(output, "## Safe-call Witness Policy\n").expect("write to string");
    writeln!(
        output,
        "Policy: `{}`. A Safe-call witness is a route-conditioned source proof: an executable `nextest` test must use a straight-line, unambiguous UFCS call to a unique Safe inherent callable, or an unambiguous Safe free function. Receiver-method syntax does not count as coverage. Standard `Result`/`Option` `unwrap`, `expect`, and continuation through `?` are accepted only when their standard wrapper provenance is proven. Macros, unknown attributes, external modules, ambiguous imports or traits, and non-linear control flow fail closed. Explicit `drop` proves RAII only for a directly owned, unwrapped public value. `ReplayPlayer::with_view` additionally requires a proven must-invoke inline closure and successful result consumption. These witnesses do not qualify a provider by themselves: native fresh-consumer gates, WASM Node/Chromium gates, and compile-only gates independently establish provider identity and execution or compilation support. Route aggregation never substitutes for running every declared verification target.\n",
        contract.evidence_policy
    )
    .expect("write to string");
    output.push_str("## Summary\n\n| Status | Count |\n|---|---:|\n");
    writeln!(output, "| `safe` | {} |", counts.safe).expect("write to string");
    writeln!(output, "| `raw` | {} |", counts.raw).expect("write to string");
    writeln!(output, "| `omitted` | {} |", counts.omitted).expect("write to string");
    writeln!(output, "| `deferred` | {} |", counts.deferred).expect("write to string");
    writeln!(output, "| Total | {} |\n", counts.total).expect("write to string");
    output.push_str("## Effective Function Exposure by Route\n\n| Precision | Provider | Safe | Raw | Omitted | Deferred | Total |\n|---|---|---:|---:|---:|---:|---:|\n");
    let route_counts = function_route_counts(&contract.functions);
    for ((mode, provider), counts) in &route_counts {
        writeln!(
            output,
            "| `{mode}` | `{provider}` | {} | {} | {} | {} | {} |",
            counts.safe, counts.raw, counts.omitted, counts.deferred, counts.total
        )
        .expect("write to string");
    }
    output.push('\n');
    output.push_str("## By Area\n\n| Area | Safe | Raw | Omitted | Deferred | Total |\n|---|---:|---:|---:|---:|---:|\n");
    for (area, counts) in by_area {
        writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} |",
            area, counts.safe, counts.raw, counts.omitted, counts.deferred, counts.total
        )
        .expect("write to string");
    }
    let abi_counts = abi_exposure_counts(&contract.abi);
    output.push_str("\n## ABI Safe Rust Exposure\n\n| Capability | Safe | Raw | Omitted | Deferred | Total |\n|---|---:|---:|---:|---:|---:|\n");
    for (kind, counts) in [
        ("Structs", abi_counts.structs),
        ("Fields", abi_counts.fields),
        ("Callbacks", abi_counts.callbacks),
    ] {
        writeln!(
            output,
            "| {kind} | {} | {} | {} | {} | {} |",
            counts.safe, counts.raw, counts.omitted, counts.deferred, counts.total
        )
        .expect("write to string");
    }
    output.push_str("\n### Effective ABI Exposure by Route\n\n| Precision | Provider | Capability | Safe | Raw | Omitted | Deferred | Total |\n|---|---|---|---:|---:|---:|---:|---:|\n");
    for (mode, provider) in route_counts.keys() {
        let effective = abi_exposure_counts_for_provider(&contract.abi, provider);
        for (kind, counts) in [
            ("Structs", effective.structs),
            ("Fields", effective.fields),
            ("Callbacks", effective.callbacks),
        ] {
            writeln!(
                output,
                "| `{mode}` | `{provider}` | {kind} | {} | {} | {} | {} | {} |",
                counts.safe, counts.raw, counts.omitted, counts.deferred, counts.total
            )
            .expect("write to string");
        }
    }
    output.push_str(
        "\n### Non-Safe ABI Capabilities\n\n| Capability | Status | Rationale |\n|---|---|---|\n",
    );
    let policies = contract
        .abi
        .policies
        .iter()
        .map(|policy| (policy.id.as_str(), policy))
        .collect::<BTreeMap<_, _>>();
    for structure in &contract.abi.structs {
        render_non_safe_abi_row(
            &mut output,
            &format!("struct {}", structure.name),
            &structure.policy,
            &structure.rationale,
            &policies,
        );
        render_non_safe_abi_overrides(
            &mut output,
            &format!("struct {}", structure.name),
            &structure.provider_overrides,
            &policies,
        );
        for field in &structure.fields {
            render_non_safe_abi_row(
                &mut output,
                &format!("{}::{}", structure.name, field.name),
                &field.policy,
                &field.rationale,
                &policies,
            );
            render_non_safe_abi_overrides(
                &mut output,
                &format!("{}::{}", structure.name, field.name),
                &field.provider_overrides,
                &policies,
            );
        }
    }
    for callback in &contract.abi.callbacks {
        render_non_safe_abi_row(
            &mut output,
            &format!("callback {}", callback.name),
            &callback.policy,
            &callback.rationale,
            &policies,
        );
        render_non_safe_abi_overrides(
            &mut output,
            &format!("callback {}", callback.name),
            &callback.provider_overrides,
            &policies,
        );
    }
    output.push_str("\n## Non-Safe Capabilities\n\n| Logical API | Status | Area | Rationale |\n|---|---|---|---|\n");
    for function in contract
        .functions
        .iter()
        .filter(|function| function.classification != Classification::Safe)
    {
        writeln!(
            output,
            "| `{}` | `{}` | {} | {} |",
            function.logical_name,
            function.classification.as_str(),
            escape_table(&function.area),
            escape_table(&function.rationale)
        )
        .expect("write to string");
    }
    for function in &contract.functions {
        for provider_override in &function.provider_overrides {
            writeln!(
                output,
                "| `{}` [{}] | `{}` | {} | {} |",
                function.logical_name,
                provider_override.providers.join(", "),
                provider_override.classification.as_str(),
                escape_table(&function.area),
                escape_table(&provider_override.rationale)
            )
            .expect("write to string");
        }
    }
    output.push_str("\n## Maintenance\n\n- Run `cargo run -p xtask -- api-coverage --check` to reject header, Rust path, evidence, recording-capability, wire-schema, ABI, or generated-document drift.\n- Run `cargo run -p xtask -- api-coverage --write` only to regenerate this report after the structured contract has been reviewed.\n- Run `cargo run -p xtask -- upstream-sync --check` to verify the manifest, gitlink, checkout, exact source-path inventory, reviewed recording-source Git objects, and all named artifacts.\n");
    output
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct AbiExposureCounts {
    structs: CoverageCounts,
    fields: CoverageCounts,
    callbacks: CoverageCounts,
}

fn abi_exposure_counts(contract: &AbiContract) -> AbiExposureCounts {
    let policies = contract
        .policies
        .iter()
        .map(|policy| (policy.id.as_str(), policy.classification))
        .collect::<BTreeMap<_, _>>();
    let mut counts = AbiExposureCounts::default();
    for structure in &contract.structs {
        if let Some(classification) = policies.get(structure.policy.as_str()) {
            counts.structs.add(*classification);
        }
        for field in &structure.fields {
            if let Some(classification) = policies.get(field.policy.as_str()) {
                counts.fields.add(*classification);
            }
        }
    }
    for callback in &contract.callbacks {
        if let Some(classification) = policies.get(callback.policy.as_str()) {
            counts.callbacks.add(*classification);
        }
    }
    counts
}

fn abi_exposure_counts_for_provider(contract: &AbiContract, provider: &str) -> AbiExposureCounts {
    let policies = contract
        .policies
        .iter()
        .map(|policy| (policy.id.as_str(), policy.classification))
        .collect::<BTreeMap<_, _>>();
    let mut counts = AbiExposureCounts::default();
    for structure in &contract.structs {
        if let Some(classification) = effective_abi_classification(
            &structure.policy,
            &structure.provider_overrides,
            provider,
            &policies,
        ) {
            counts.structs.add(classification);
        }
        for field in &structure.fields {
            if let Some(classification) = effective_abi_classification(
                &field.policy,
                &field.provider_overrides,
                provider,
                &policies,
            ) {
                counts.fields.add(classification);
            }
        }
    }
    for callback in &contract.callbacks {
        if let Some(classification) = effective_abi_classification(
            &callback.policy,
            &callback.provider_overrides,
            provider,
            &policies,
        ) {
            counts.callbacks.add(classification);
        }
    }
    counts
}

fn effective_abi_classification(
    policy_id: &str,
    provider_overrides: &[crate::abi_contract::AbiProviderOverride],
    provider: &str,
    policies: &BTreeMap<&str, Classification>,
) -> Option<Classification> {
    let effective_policy = provider_overrides
        .iter()
        .find(|provider_override| {
            provider_override
                .providers
                .iter()
                .any(|candidate| candidate == provider)
        })
        .map_or(policy_id, |provider_override| {
            provider_override.policy.as_str()
        });
    policies.get(effective_policy).copied()
}

fn render_non_safe_abi_row(
    output: &mut String,
    identifier: &str,
    policy_id: &str,
    rationale: &str,
    policies: &BTreeMap<&str, &crate::abi_contract::AbiCapabilityPolicy>,
) {
    let Some(policy) = policies.get(policy_id) else {
        return;
    };
    if policy.classification == Classification::Safe {
        return;
    }
    writeln!(
        output,
        "| `{}` | `{}` | {} |",
        escape_table(identifier),
        policy.classification.as_str(),
        escape_table(rationale)
    )
    .expect("write to string");
}

fn render_non_safe_abi_overrides(
    output: &mut String,
    identifier: &str,
    provider_overrides: &[crate::abi_contract::AbiProviderOverride],
    policies: &BTreeMap<&str, &crate::abi_contract::AbiCapabilityPolicy>,
) {
    for provider_override in provider_overrides {
        let Some(policy) = policies.get(provider_override.policy.as_str()) else {
            continue;
        };
        if policy.classification == Classification::Safe {
            continue;
        }
        writeln!(
            output,
            "| `{}` [{}] | `{}` | {} |",
            escape_table(identifier),
            provider_override.providers.join(", "),
            policy.classification.as_str(),
            escape_table(&provider_override.rationale)
        )
        .expect("write to string");
    }
}

fn has_rationale(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 24
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "todo" | "tbd" | "deferred"
        )
}

fn is_c_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        abi_contract::map_inventory,
        c_api::{
            AbiCallableShape, AbiPrimitive, AbiTypeShape, CallbackDecl, FieldDecl, OverlayDecl,
            PrecisionCApiInventory, StructDecl,
        },
        commands::upstream_sync::{ArtifactProducer, ArtifactProvider, Precision},
        rust_index::index_boxdd,
    };

    struct ContractFixture {
        root: std::path::PathBuf,
        inventory: CApiInventory,
        rust_indexes: AbiRustIndexes,
        binding_routes: AbiBindingRoutes,
        binding_indexes: AbiBindingIndexes,
        contract: ApiContract,
        operations: Vec<crate::recording_ops::RecordingOp>,
        active_revision: String,
    }

    fn workspace_manifest_and_contract() -> (UpstreamManifest, Vec<u8>) {
        let paths = WorkspacePaths::discover().expect("workspace paths");
        let manifest = UpstreamManifest::load(&paths).expect("upstream manifest");
        let contract_path = manifest
            .artifact_path(paths.root(), ArtifactKind::ApiContract)
            .expect("API contract path");
        let contract = fs::read(contract_path).expect("reviewed API contract");
        (manifest, contract)
    }

    #[test]
    fn reviewed_contract_preflight_rejects_malformed_digest() {
        let (manifest, contract) = workspace_manifest_and_contract();

        let error = reviewed_contract_preflight_manifest(&manifest, &contract, &"A".repeat(64))
            .expect_err("uppercase digest must fail closed");

        assert!(error.to_string().contains("lowercase 64-character"));
    }

    #[test]
    fn reviewed_contract_preflight_rejects_digest_mismatch() {
        let (manifest, contract) = workspace_manifest_and_contract();
        let observed = blake3::hash(&contract).to_hex().to_string();
        let mismatched = if observed == "0".repeat(64) {
            "1".repeat(64)
        } else {
            "0".repeat(64)
        };

        let error = reviewed_contract_preflight_manifest(&manifest, &contract, &mismatched)
            .expect_err("a digest for different bytes must fail closed");

        assert!(error.to_string().contains("BLAKE3 mismatch"));
        assert!(error.to_string().contains(&observed));
    }

    #[test]
    fn reviewed_contract_preflight_changes_only_contract_digest() {
        let (manifest, contract) = workspace_manifest_and_contract();
        let digest = blake3::hash(&contract).to_hex().to_string();
        let mut expected = manifest.clone();
        expected
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == ArtifactKind::ApiContract)
            .expect("API contract artifact")
            .content_blake3
            .clone_from(&digest);

        let preflight = reviewed_contract_preflight_manifest(&manifest, &contract, &digest)
            .expect("the exact reviewed bytes must be accepted");

        assert_eq!(preflight, expected);
    }

    #[test]
    fn reviewed_contract_snapshot_resists_path_aba_during_refresh() {
        let (manifest, approved_bytes) = workspace_manifest_and_contract();
        let temporary = tempfile::tempdir().expect("temporary contract directory");
        let contract_path = temporary.path().join("api_contract.toml");
        fs::write(&contract_path, &approved_bytes).expect("approved contract bytes");
        let approved = read_reviewed_contract_snapshot(&contract_path).expect("approved snapshot");
        let mut unreviewed_contract = approved.contract.clone();
        unreviewed_contract.upstream_sha = "0".repeat(40);
        let unreviewed_bytes = render_toml(&unreviewed_contract)
            .expect("unreviewed contract TOML")
            .into_bytes();

        fs::write(&contract_path, &unreviewed_bytes).expect("concurrent unreviewed contract");
        let reopened = read_reviewed_contract_snapshot(&contract_path)
            .expect("a vulnerable second read would observe the unreviewed contract");
        fs::write(&contract_path, &approved_bytes).expect("concurrent ABA restore");

        assert_eq!(approved.bytes, approved_bytes);
        assert_eq!(approved.contract.upstream_sha, manifest.active_revision);
        assert_eq!(reopened.contract.upstream_sha, "0".repeat(40));
        assert_eq!(
            fs::read(&contract_path).expect("restored contract bytes"),
            approved.bytes
        );
        assert_ne!(approved.contract, reopened.contract);
    }

    #[test]
    fn forged_active_revision_cannot_disable_the_target_inventory_gate() {
        let forged_manifest_revision = "0123456789abcdef0123456789abcdef01234567";
        let observed_revision = BOX2D_3_2_TARGET_REVISION;

        let error = validate_authenticated_revision(
            forged_manifest_revision,
            observed_revision,
            observed_revision,
        )
        .expect_err("a manifest revision cannot replace the authenticated repository identity");

        assert!(error.to_string().contains("authenticated gitlink"));
        assert!(error.to_string().contains(observed_revision));
        assert!(error.to_string().contains(forged_manifest_revision));
    }

    #[test]
    fn equal_count_function_substitution_changes_every_inventory_digest() {
        let function = crate::c_api::FunctionDecl {
            name: "b2Body_SetTransform".to_owned(),
            signature: "void b2Body_SetTransform ( void )".to_owned(),
            fingerprint: "fnv1a64:fixture".to_owned(),
            parameters: Vec::new(),
            physical_symbols: BTreeMap::from([
                ("single".to_owned(), "b2Body_SetTransform".to_owned()),
                ("double".to_owned(), "b2Body_SetTransform".to_owned()),
            ]),
            availability: vec!["always".to_owned()],
            header: "box2d.h".to_owned(),
            line: 1,
        };
        let mut second = function.clone();
        second.name = "b2World_Step".to_owned();
        second.physical_symbols = BTreeMap::from([
            ("single".to_owned(), "b2World_Step".to_owned()),
            ("double".to_owned(), "b2World_Step".to_owned()),
        ]);
        let reviewed_inventory = CApiInventory {
            functions: vec![function, second],
            structs: Vec::new(),
            callbacks: Vec::new(),
        };
        let reviewed = compute_function_inventory_digests(&reviewed_inventory)
            .expect("fixture function inventory digest");
        let mut reordered = reviewed_inventory.clone();
        reordered.functions.reverse();
        assert_eq!(
            compute_function_inventory_digests(&reordered)
                .expect("reordered function inventory digest"),
            reviewed
        );

        let mut substituted = reviewed_inventory.clone();
        let function = substituted
            .functions
            .first_mut()
            .expect("fixture function declaration");
        function.name = "b2World_Explode".to_owned();
        function.physical_symbols = BTreeMap::from([
            ("single".to_owned(), "b2World_Explode".to_owned()),
            ("double".to_owned(), "b2World_Explode_double".to_owned()),
        ]);

        assert_eq!(
            substituted.functions.len(),
            reviewed_inventory.functions.len()
        );
        let observed = compute_function_inventory_digests(&substituted)
            .expect("substituted function inventory digest");
        assert_ne!(observed.logical, reviewed.logical);
        assert_ne!(observed.single, reviewed.single);
        assert_ne!(observed.double, reviewed.double);

        let mut errors = Vec::new();
        validate_function_inventory_digests(&reviewed, &substituted, &mut errors);
        assert!(errors.iter().any(|error| error.contains("logical")));
        assert!(errors.iter().any(|error| error.contains("single")));
        assert!(errors.iter().any(|error| error.contains("double")));
    }

    #[test]
    fn pinned_box2d_target_function_inventory_digest_is_stable() {
        let paths = WorkspacePaths::discover().expect("workspace paths");
        let inventory = parse_headers(&paths.box2d_headers()).expect("vendored Box2D headers");
        let observed =
            compute_function_inventory_digests(&inventory).expect("function inventory digests");
        let pinned = FunctionInventoryDigests {
            logical: BOX2D_3_2_LOGICAL_FUNCTIONS_BLAKE3.to_owned(),
            single: BOX2D_3_2_SINGLE_FUNCTIONS_BLAKE3.to_owned(),
            double: BOX2D_3_2_DOUBLE_FUNCTIONS_BLAKE3.to_owned(),
        };

        assert_eq!(observed, pinned);
    }

    #[test]
    fn pinned_box2d_schema_8_route_topology_is_exact_and_persistent() {
        let routes = pinned_box2d_3_2_binding_routes();
        let coordinates = routes.keys().cloned().collect::<BTreeSet<_>>();
        let artifacts = routes
            .values()
            .map(|route| route.artifact.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(routes.len(), 10);
        assert_eq!(
            coordinates,
            BTreeSet::from([
                ("double".to_owned(), "prebuilt-static".to_owned()),
                ("double".to_owned(), "source".to_owned()),
                ("double".to_owned(), "system-static".to_owned()),
                ("double".to_owned(), "wasm-compile-only".to_owned()),
                ("double".to_owned(), "wasm-runtime".to_owned()),
                ("single".to_owned(), "prebuilt-static".to_owned()),
                ("single".to_owned(), "source".to_owned()),
                ("single".to_owned(), "system-static".to_owned()),
                ("single".to_owned(), "wasm-compile-only".to_owned()),
                ("single".to_owned(), "wasm-runtime".to_owned()),
            ])
        );
        assert_eq!(
            artifacts,
            BTreeSet::from([
                "bindings-double",
                "bindings-single",
                "bindings-wasm32-unknown-unknown-double",
                "bindings-wasm32-unknown-unknown-single",
                "bindings-wasm32-wasip1-double",
                "bindings-wasm32-wasip1-single",
            ])
        );
        let mut errors = Vec::new();
        validate_pinned_box2d_3_2_binding_routes(&routes, &mut errors);
        assert!(errors.is_empty(), "canonical route matrix: {errors:?}");

        let mut missing = routes.clone();
        missing.remove(&("single".to_owned(), "system-static".to_owned()));
        validate_pinned_box2d_3_2_binding_routes(&missing, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("exact canonical 10-route matrix")),
            "a self-consistent route subset must not bypass the pinned topology: {errors:?}"
        );

        errors.clear();
        let mut drifted = routes;
        drifted
            .get_mut(&("double".to_owned(), "wasm-runtime".to_owned()))
            .expect("double WASM runtime route")
            .rust_target = RustTarget::Wasm32Wasip1;
        validate_pinned_box2d_3_2_binding_routes(&drifted, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("exact canonical 10-route matrix")),
            "route target drift must fail closed: {errors:?}"
        );
    }

    #[test]
    fn pinned_box2d_schema_8_binding_artifact_identities_are_exact() {
        let canonical = canonical_route_binding_artifacts();
        assert_eq!(canonical.len(), 6);
        let mut errors = Vec::new();
        validate_pinned_box2d_3_2_binding_artifacts(&canonical, &mut errors);
        assert!(errors.is_empty(), "canonical binding artifacts: {errors:?}");

        let assert_rejected = |artifacts: &[GeneratedArtifact], mutation: &str| {
            let mut errors = Vec::new();
            validate_pinned_box2d_3_2_binding_artifacts(artifacts, &mut errors);
            assert!(
                errors
                    .iter()
                    .any(|error| error.contains("exact canonical six binding artifact identities")),
                "{mutation} must fail closed: {errors:?}"
            );
        };

        let mut drifted = canonical.clone();
        drifted[0].path = "boxdd-sys/src/bindings_decoy.rs".to_owned();
        assert_rejected(&drifted, "path drift");

        let mut drifted = canonical.clone();
        drifted[0].precision = Some(Precision::Double);
        assert_rejected(&drifted, "precision drift");

        let mut drifted = canonical.clone();
        drifted[0].target = ArtifactTarget::Wasm32UnknownUnknown;
        assert_rejected(&drifted, "target drift");

        let mut drifted = canonical.clone();
        drifted[0].provider = ArtifactProvider::Source;
        assert_rejected(&drifted, "provider drift");

        let mut drifted = canonical;
        drifted[0].producer = ArtifactProducer::Reviewed;
        assert_rejected(&drifted, "producer drift");
    }

    #[test]
    fn pinned_box2d_schema_8_rejects_function_deferred_reentry() {
        let mut fixture = ContractFixture::create();
        fixture.contract.functions[0].classification = Classification::Deferred;
        let mut errors = Vec::new();

        validate_pinned_box2d_3_2_no_deferred_functions(&fixture.contract, &mut errors);

        assert!(
            errors
                .iter()
                .any(|error| error.contains("reintroduces Deferred")),
            "function Deferred reentry must fail closed: {errors:?}"
        );
    }

    #[test]
    fn runtime_evidence_distinguishes_callable_and_raii_drop_witnesses() {
        let callable = "boxdd::World::step".to_owned();
        let owner = "boxdd::World".to_owned();
        let index = TestEvidenceIndex {
            fingerprint: "blake3-v2:fixture".to_owned(),
            called_public_paths: BTreeSet::from([callable.clone()]),
            called_local_paths: BTreeSet::new(),
            dropped_public_types: BTreeSet::from([owner.clone()]),
            implementation_reachable_symbols: BTreeSet::from([
                "b2World_Step".to_owned(),
                "b2DestroyWorld".to_owned(),
            ]),
            unresolved_calls: BTreeSet::new(),
        };

        assert!(evidence_invokes_exposure(
            &index,
            FunctionExposureKind::Callable,
            &callable
        ));
        assert!(!evidence_invokes_exposure(
            &index,
            FunctionExposureKind::Callable,
            &owner
        ));
        assert!(evidence_invokes_exposure(
            &index,
            FunctionExposureKind::RaiiDrop,
            &owner
        ));
        assert!(!evidence_invokes_exposure(
            &index,
            FunctionExposureKind::RaiiDrop,
            &callable
        ));
    }

    #[test]
    fn evidence_fingerprint_aggregates_route_identity_and_route_specific_ast() {
        fn route_index(fingerprint: &str) -> TestEvidenceIndex {
            TestEvidenceIndex {
                fingerprint: fingerprint.to_owned(),
                called_public_paths: BTreeSet::new(),
                called_local_paths: BTreeSet::new(),
                dropped_public_types: BTreeSet::new(),
                implementation_reachable_symbols: BTreeSet::new(),
                unresolved_calls: BTreeSet::new(),
            }
        }

        let routes = BTreeMap::from([
            (
                ("double".to_owned(), "source".to_owned()),
                route_index("blake3-v2:double-helper"),
            ),
            (
                ("single".to_owned(), "source".to_owned()),
                route_index("blake3-v2:single-helper"),
            ),
        ]);
        let aggregate = aggregate_evidence_fingerprint(&routes);
        assert!(aggregate.starts_with("blake3-routes-v2:"));
        assert_eq!(aggregate, aggregate_evidence_fingerprint(&routes));

        let mut changed_ast = routes.clone();
        changed_ast
            .get_mut(&("double".to_owned(), "source".to_owned()))
            .expect("double route")
            .fingerprint = "blake3-v2:changed-double-helper".to_owned();
        assert_ne!(aggregate, aggregate_evidence_fingerprint(&changed_ast));

        let mut changed_route = routes;
        let index = changed_route
            .remove(&("double".to_owned(), "source".to_owned()))
            .expect("double route");
        changed_route.insert(("double".to_owned(), "system-static".to_owned()), index);
        assert_ne!(aggregate, aggregate_evidence_fingerprint(&changed_route));
    }

    impl ContractFixture {
        fn create() -> Self {
            let root = std::env::temp_dir().join(format!(
                "boxdd-api-contract-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(root.join("boxdd/src")).expect("crate source directory");
            fs::create_dir_all(root.join("boxdd/tests")).expect("test directory");
            fs::create_dir_all(root.join("boxdd-sys/src")).expect("sys source directory");
            fs::write(
                root.join("boxdd/Cargo.toml"),
                "[package]\nname = \"boxdd\"\nversion = \"0.0.0\"\n[features]\ndefault = []\n[dependencies]\nboxdd-sys = \"0\"\n",
            )
            .expect("crate manifest");
            fs::write(
                root.join("boxdd/src/lib.rs"),
                r#"
                    pub fn set_transform() {
                        unsafe { boxdd_sys::ffi::b2Body_SetTransform(); }
                    }
                    pub struct RecordingSession;
                    impl RecordingSession {
                        pub fn try_set_transform(&mut self) {
                            unsafe { boxdd_sys::ffi::b2Body_SetTransform(); }
                        }
                    }
                    pub struct Example { pub count: i32 }
                    impl Example {
                        pub fn from_raw(raw: boxdd_sys::ffi::b2Example) -> Self {
                            Self { count: raw.count }
                        }
                    }
                "#,
            )
            .expect("crate source");
            fs::write(
                root.join("boxdd/tests/evidence.rs"),
                "#[test]\nfn covers_body_set_transform() { boxdd::set_transform(); }\n",
            )
            .expect("test evidence");
            fs::write(
                root.join("boxdd/tests/comment_only.rs"),
                "// #[test]\n// fn covers_body_set_transform() {}\n",
            )
            .expect("comment-only evidence");
            fs::write(
                root.join("boxdd-sys/src/bindings_pregenerated.rs"),
                r#"
                    pub struct b2Example { pub count: i32 }
                    pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
                    pub type b2Pos = b2Vec2;
                    pub struct b2Vec2 { pub x: f32 }
                    pub struct b2TreeNode {
                        pub __bindgen_anon_1: b2TreeNode__bindgen_ty_1,
                    }
                    pub union b2TreeNode__bindgen_ty_1 {
                        pub children: b2TreeNode__bindgen_ty_1__bindgen_ty_1,
                        pub userData: u64,
                    }
                    pub struct b2TreeNode__bindgen_ty_1__bindgen_ty_1 {
                        pub child1: i32,
                    }
                    unsafe extern "C" {
                        pub fn b2Body_SetTransform();
                    }
                "#,
            )
            .expect("generated binding fixture");

            let signature = "void b2Body_SetTransform ( void )".to_owned();
            let fingerprint = "fnv1a64:fixture".to_owned();
            let inventory = CApiInventory {
                functions: vec![crate::c_api::FunctionDecl {
                    name: "b2Body_SetTransform".to_owned(),
                    signature: signature.clone(),
                    fingerprint: fingerprint.clone(),
                    parameters: Vec::new(),
                    physical_symbols: BTreeMap::from([
                        ("single".to_owned(), "b2Body_SetTransform".to_owned()),
                        ("double".to_owned(), "b2Body_SetTransform".to_owned()),
                    ]),
                    availability: vec!["always".to_owned()],
                    header: "box2d.h".to_owned(),
                    line: 1,
                }],
                structs: Vec::new(),
                callbacks: Vec::new(),
            };
            let index = index_boxdd(&root).expect("Rust index");
            let evidence_index = index_test_evidence_for_gate_at_coordinate(
                &root,
                "boxdd/tests/evidence.rs",
                "covers_body_set_transform",
                "boxdd",
                "nextest",
                &index,
                &rust_index_coordinate(RustTarget::X86_64UnknownLinuxGnu),
            )
            .expect("test evidence index");
            let evidence_fingerprint = aggregate_evidence_fingerprint(&BTreeMap::from([(
                ("single".to_owned(), "source".to_owned()),
                evidence_index,
            )]));
            let rust_indexes =
                AbiRustIndexes::from([(("single".to_owned(), "source".to_owned()), index)]);
            let binding = AbiBindingIndex::from_path(
                "bindings-single",
                Precision::Single,
                ArtifactTarget::Universal,
                ArtifactProvider::Universal,
                &root.join("boxdd-sys/src/bindings_pregenerated.rs"),
            )
            .expect("sys ABI index");
            let binding_indexes = AbiBindingIndexes::from([(binding.artifact.clone(), binding)]);
            let route = AbiBindingRoute {
                mode: "single".to_owned(),
                provider: "source".to_owned(),
                artifact: "bindings-single".to_owned(),
                rust_target: RustTarget::X86_64UnknownLinuxGnu,
                rust_features: Vec::new(),
            };
            let binding_routes =
                AbiBindingRoutes::from([((route.mode.clone(), route.provider.clone()), route)]);
            let active_revision = "0123456789abcdef0123456789abcdef01234567".to_owned();
            let function_inventory_digests = compute_function_inventory_digests(&inventory)
                .expect("fixture function inventory digest");
            let contract = ApiContract {
                schema_version: API_CONTRACT_SCHEMA,
                evidence_policy: SAFE_CALL_EVIDENCE_POLICY.to_owned(),
                upstream_sha: active_revision.clone(),
                function_inventory_digests,
                migration_baseline: CoverageCounts {
                    total: 1,
                    safe: 1,
                    ..CoverageCounts::default()
                },
                classification_changes: Vec::new(),
                evidence: vec![TestEvidence {
                    id: "body-runtime".to_owned(),
                    file: "boxdd/tests/evidence.rs".to_owned(),
                    item: "covers_body_set_transform".to_owned(),
                    package: "boxdd".to_owned(),
                    gate: "nextest".to_owned(),
                    role: TestEvidenceRole::SafeCall,
                    fingerprint: evidence_fingerprint,
                    modes: vec!["single".to_owned()],
                    providers: vec!["source".to_owned()],
                    call_witnesses: vec![SafeCallWitness {
                        function: "b2Body_SetTransform".to_owned(),
                        rust_path: "boxdd::set_transform".to_owned(),
                    }],
                    classification_witnesses: Vec::new(),
                }],
                functions: vec![FunctionContract {
                    logical_name: "b2Body_SetTransform".to_owned(),
                    signature,
                    fingerprint,
                    abi_fingerprints: BTreeMap::new(),
                    link_symbols: BTreeMap::from([(
                        "single".to_owned(),
                        "b2Body_SetTransform".to_owned(),
                    )]),
                    classification: Classification::Safe,
                    exposure: FunctionExposureKind::Callable,
                    area: "Body".to_owned(),
                    rust_paths: vec!["boxdd::set_transform".to_owned()],
                    rationale: "The safe wrapper validates ownership before the native call."
                        .to_owned(),
                    modes: vec!["single".to_owned()],
                    providers: vec!["source".to_owned()],
                    availability: vec!["always".to_owned()],
                    evidence: vec!["body-runtime".to_owned()],
                    provider_overrides: Vec::new(),
                    recording: Some(RecordingCoverage {
                        class: RecordingClass::LoggedMutation,
                        opcode: Some(0x20),
                    }),
                }],
                abi: AbiContract::default(),
            };
            let operations = vec![crate::recording_ops::RecordingOp {
                opcode: 0x20,
                name: "BodySetTransform".to_owned(),
                return_tag: "RET_NONE".to_owned(),
                arguments: Vec::new(),
            }];
            let fixture = Self {
                root,
                inventory,
                rust_indexes,
                binding_routes,
                binding_indexes,
                contract,
                operations,
                active_revision,
            };
            fixture.validate().expect("valid contract fixture");
            fixture
        }

        fn create_default_constructor_refresh() -> Self {
            let mut fixture = Self::create();
            fs::write(
                fixture.root.join("boxdd/src/lib.rs"),
                r#"
                    pub struct Filter;
                    impl Default for Filter {
                        fn default() -> Self {
                            unsafe { boxdd_sys::ffi::b2DefaultFilter(); }
                            Self
                        }
                    }
                    impl Filter {
                        pub fn new() -> Self {
                            Self::default()
                        }
                        pub fn builder() -> Self {
                            Self::default()
                        }
                    }
                "#,
            )
            .expect("default constructor fixture source");
            let index = index_boxdd(&fixture.root).expect("default constructor Rust index");
            fixture.rust_indexes =
                AbiRustIndexes::from([(("single".to_owned(), "source".to_owned()), index)]);

            let signature = {
                let declaration = fixture
                    .inventory
                    .functions
                    .first_mut()
                    .expect("fixture declaration");
                declaration.name = "b2DefaultFilter".to_owned();
                declaration.signature = "void b2DefaultFilter ( void )".to_owned();
                declaration.physical_symbols = BTreeMap::from([
                    ("single".to_owned(), "b2DefaultFilter".to_owned()),
                    ("double".to_owned(), "b2DefaultFilter".to_owned()),
                ]);
                declaration.signature.clone()
            };
            fixture.contract.function_inventory_digests =
                compute_function_inventory_digests(&fixture.inventory)
                    .expect("default constructor inventory digest");
            fixture.contract.evidence.clear();
            let function = fixture
                .contract
                .functions
                .first_mut()
                .expect("fixture function");
            function.logical_name = "b2DefaultFilter".to_owned();
            function.signature = signature;
            function.link_symbols =
                BTreeMap::from([("single".to_owned(), "b2DefaultFilter".to_owned())]);
            function.area = "Foundation".to_owned();
            function.rust_paths = vec!["boxdd::Filter::new".to_owned()];
            function.rationale =
                "Filter::new constructs the reviewed Box2D collision-filter defaults.".to_owned();
            function.evidence.clear();
            function.recording = None;
            fixture.operations.clear();
            fixture
        }

        fn paths(&self) -> WorkspacePaths {
            WorkspacePaths::new(&self.root)
        }

        fn validate(&self) -> Result<()> {
            validate_contract(
                &self.paths(),
                &self.contract,
                &self.inventory,
                None,
                &self.rust_indexes,
                &self.binding_routes,
                &self.binding_indexes,
                &self.active_revision,
                &self.operations,
            )
        }

        fn refresh_evidence_fingerprint(&mut self, id: &str) {
            let position = self
                .contract
                .evidence
                .iter()
                .position(|evidence| evidence.id == id)
                .expect("fixture evidence row");
            let evidence = self.contract.evidence[position].clone();
            let indexed_routes = index_evidence_across_routes(
                &self.paths(),
                &evidence,
                &self.rust_indexes,
                &self.binding_routes,
            )
            .expect("fixture evidence index");
            self.contract.evidence[position].fingerprint =
                aggregate_evidence_fingerprint(&indexed_routes);
        }

        fn enable_abi_capabilities(&mut self) {
            self.inventory.structs.push(StructDecl {
                name: "b2Example".to_owned(),
                fingerprint: "fnv1a64:struct".to_owned(),
                fields: vec![FieldDecl {
                    name: "count".to_owned(),
                    signature: "int count".to_owned(),
                    overlays: Vec::new(),
                }],
                header: "box2d.h".to_owned(),
                line: 3,
            });
            self.inventory.callbacks.push(CallbackDecl {
                name: "b2ExampleCallback".to_owned(),
                signature: "void b2ExampleCallback ( void )".to_owned(),
                fingerprint: "fnv1a64:callback".to_owned(),
                header: "box2d.h".to_owned(),
                line: 4,
            });
            self.contract.abi =
                map_inventory(&self.inventory, &self.binding_routes, &self.binding_indexes)
                    .expect("ABI declarations should map to generated bindings");
            self.install_abi_evidence();
            self.validate()
                .expect("mapped ABI capability fixture should validate");
        }

        fn install_abi_evidence(&mut self) {
            if self
                .contract
                .evidence
                .iter()
                .any(|evidence| evidence.id == ABI_HEADER_EVIDENCE_ID)
            {
                return;
            }
            self.write_xtask_evidence_sources();
            let evidence = [
                (
                    ABI_HEADER_EVIDENCE_ID,
                    "xtask/src/c_api.rs",
                    "vendored_headers_build_precision_abi_inventories",
                    TestEvidenceRole::AbiHeaderInventory,
                ),
                (
                    ABI_BINDING_EVIDENCE_ID,
                    "xtask/src/sys_abi_index.rs",
                    "indexes_the_checked_in_pregenerated_bindings",
                    TestEvidenceRole::AbiBindingAst,
                ),
                (
                    ABI_VALIDATOR_EVIDENCE_ID,
                    "xtask/src/commands/api_coverage.rs",
                    "abi_capability_mapping_rejects_deleted_forged_and_unknown_references",
                    TestEvidenceRole::AbiContractValidator,
                ),
            ];
            let modes = self
                .binding_routes
                .keys()
                .map(|(mode, _)| mode.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let providers = self
                .binding_routes
                .keys()
                .map(|(_, provider)| provider.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            for (id, file, item, role) in evidence {
                let mut row = TestEvidence {
                    id: id.to_owned(),
                    file: file.to_owned(),
                    item: item.to_owned(),
                    package: "xtask".to_owned(),
                    gate: "nextest".to_owned(),
                    role,
                    fingerprint: String::new(),
                    modes: modes.clone(),
                    providers: providers.clone(),
                    call_witnesses: Vec::new(),
                    classification_witnesses: Vec::new(),
                };
                let indexed_routes = index_evidence_across_routes(
                    &self.paths(),
                    &row,
                    &self.rust_indexes,
                    &self.binding_routes,
                )
                .expect("ABI evidence index");
                row.fingerprint = aggregate_evidence_fingerprint(&indexed_routes);
                self.contract.evidence.push(row);
            }
        }

        fn install_classification_evidence(&mut self) {
            self.write_xtask_evidence_sources();
            let file = "xtask/src/commands/api_coverage.rs";
            let item = "typed_function_classification_evidence_rejects_unrelated_subjects";
            let mut row = TestEvidence {
                id: API_CLASSIFICATION_EVIDENCE_ID.to_owned(),
                file: file.to_owned(),
                item: item.to_owned(),
                package: "xtask".to_owned(),
                gate: "nextest".to_owned(),
                role: TestEvidenceRole::FunctionClassificationValidator,
                fingerprint: String::new(),
                modes: vec!["single".to_owned()],
                providers: vec!["source".to_owned()],
                call_witnesses: Vec::new(),
                classification_witnesses: vec![FunctionClassificationWitness {
                    function: "b2Body_SetTransform".to_owned(),
                    classification: Classification::Raw,
                }],
            };
            let indexed_routes = index_evidence_across_routes(
                &self.paths(),
                &row,
                &self.rust_indexes,
                &self.binding_routes,
            )
            .expect("classification evidence index");
            row.fingerprint = aggregate_evidence_fingerprint(&indexed_routes);
            self.contract.evidence.push(row);
        }

        fn write_xtask_evidence_sources(&self) {
            fs::create_dir_all(self.root.join("xtask/src/commands"))
                .expect("xtask evidence directories");
            fs::write(
                self.root.join("xtask/Cargo.toml"),
                "[package]\nname = \"xtask\"\nversion = \"0.0.0\"\n",
            )
            .expect("xtask fixture manifest");
            fs::write(
                self.root.join("xtask/src/lib.rs"),
                "mod c_api;\nmod sys_abi_index;\nmod commands;\n",
            )
            .expect("xtask fixture root");
            fs::write(
                self.root.join("xtask/src/commands/mod.rs"),
                "mod api_coverage;\n",
            )
            .expect("xtask commands module");
            fs::write(
                self.root.join("xtask/src/c_api.rs"),
                "fn parse_headers_for_precision() {}\n#[test]\nfn vendored_headers_build_precision_abi_inventories() { parse_headers_for_precision(); }\n",
            )
            .expect("header evidence source");
            fs::write(
                self.root.join("xtask/src/sys_abi_index.rs"),
                "fn index_bindings() {}\n#[test]\nfn indexes_the_checked_in_pregenerated_bindings() { index_bindings(); }\n",
            )
            .expect("binding evidence source");
            fs::write(
                self.root.join("xtask/src/commands/api_coverage.rs"),
                concat!(
                    "fn validate_contract() {}\n",
                    "#[test]\nfn abi_capability_mapping_rejects_deleted_forged_and_unknown_references() { validate_contract(); }\n",
                    "#[test]\nfn typed_function_classification_evidence_rejects_unrelated_subjects() { validate_contract(); }\n",
                ),
            )
            .expect("contract evidence source");
        }
    }

    impl Drop for ContractFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn migration_baseline_requires_explicit_classification_changes() {
        let mut contract = ApiContract {
            schema_version: API_CONTRACT_SCHEMA,
            evidence_policy: SAFE_CALL_EVIDENCE_POLICY.to_owned(),
            upstream_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            function_inventory_digests: FunctionInventoryDigests::default(),
            migration_baseline: CoverageCounts {
                total: 1,
                safe: 1,
                ..CoverageCounts::default()
            },
            classification_changes: vec![ClassificationChange {
                logical_name: "b2SetLengthUnitsPerMeter".to_owned(),
                from: Classification::Safe,
                to: Classification::Raw,
                unit: "U1".to_owned(),
                rationale:
                    "The global setter is unsafe while process activity remains uncoordinated."
                        .to_owned(),
            }],
            evidence: Vec::new(),
            functions: vec![FunctionContract {
                logical_name: "b2SetLengthUnitsPerMeter".to_owned(),
                signature: String::new(),
                fingerprint: String::new(),
                abi_fingerprints: BTreeMap::new(),
                link_symbols: BTreeMap::new(),
                classification: Classification::Raw,
                exposure: FunctionExposureKind::Callable,
                area: "Math".to_owned(),
                rust_paths: Vec::new(),
                rationale: String::new(),
                modes: Vec::new(),
                providers: Vec::new(),
                availability: Vec::new(),
                evidence: Vec::new(),
                provider_overrides: Vec::new(),
                recording: None,
            }],
            abi: AbiContract::default(),
        };
        let mut errors = Vec::new();
        validate_migration(&contract, &mut errors);
        assert!(errors.is_empty());
        contract.classification_changes.clear();
        validate_migration(&contract, &mut errors);
        assert!(!errors.is_empty());
    }

    #[test]
    fn active_refresh_identity_never_claims_the_next_revision() {
        let mut fixture = ContractFixture::create();
        let next_revision = "fedcba9876543210fedcba9876543210fedcba98";
        assert_ne!(fixture.active_revision, next_revision);
        fixture.contract.upstream_sha = next_revision.to_owned();

        set_active_refresh_identity(&mut fixture.contract, &fixture.active_revision);

        assert_eq!(fixture.contract.upstream_sha, fixture.active_revision);
        assert_ne!(fixture.contract.upstream_sha, next_revision);
    }

    fn reviewed_added_function_override(fixture: &ContractFixture) -> ReviewedMigrationOverrides {
        ReviewedMigrationOverrides {
            schema_version: REVIEWED_MIGRATION_SCHEMA,
            reviewed_revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            active_revision: fixture.active_revision.clone(),
            expected_counts: CoverageCounts {
                total: 1,
                safe: 1,
                ..CoverageCounts::default()
            },
            functions: vec![ReviewedFunctionOverride {
                logical_name: "b2Body_SetTransform".to_owned(),
                classification: Classification::Safe,
                exposure: FunctionExposureKind::Callable,
                area: "Body transform".to_owned(),
                rust_paths: vec!["boxdd::set_transform".to_owned()],
                rationale: "The Safe Rust wrapper validates ownership before the native mutation."
                    .to_owned(),
                previous_classification: None,
                transition_unit: None,
                revalidated: false,
            }],
            canonical_refreshes: Vec::new(),
        }
    }

    #[test]
    fn reviewed_migration_requires_exact_override_coverage() {
        let fixture = ContractFixture::create();
        let mut reviewed = fixture.contract.clone();
        reviewed.functions.clear();
        let mut missing = reviewed_added_function_override(&fixture);
        missing.functions.clear();

        let error = apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &reviewed,
            &fixture.contract,
            &missing,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("every added function needs an explicit reviewed override");
        assert!(
            error
                .to_string()
                .contains("missing=[\"b2Body_SetTransform\"]")
        );

        let mut unexpected = reviewed_added_function_override(&fixture);
        unexpected.functions.push(ReviewedFunctionOverride {
            logical_name: "b2Unexpected".to_owned(),
            classification: Classification::Raw,
            exposure: FunctionExposureKind::Callable,
            area: "Unexpected".to_owned(),
            rust_paths: Vec::new(),
            rationale: "This row must be rejected before it can alter the reviewed contract."
                .to_owned(),
            previous_classification: None,
            transition_unit: None,
            revalidated: false,
        });
        let error = apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &reviewed,
            &fixture.contract,
            &unexpected,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("unrelated overrides must fail closed");
        assert!(error.to_string().contains("unexpected=[\"b2Unexpected\"]"));
    }

    #[test]
    fn reviewed_migration_authenticates_exact_classification_transitions() {
        let fixture = ContractFixture::create();
        let mut reviewed = fixture.contract.clone();
        let historical = reviewed.functions.first_mut().expect("historical function");
        historical.classification = Classification::Raw;
        historical.rust_paths = vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()];
        historical.provider_overrides.clear();

        let mut transition = reviewed_added_function_override(&fixture);
        transition.functions[0] = ReviewedFunctionOverride {
            logical_name: "b2Body_SetTransform".to_owned(),
            classification: Classification::Safe,
            exposure: FunctionExposureKind::Callable,
            area: "Body transform".to_owned(),
            rust_paths: vec!["boxdd::RecordingSession::try_set_transform".to_owned()],
            rationale:
                "RecordingSession validates ownership before applying and recording the transform."
                    .to_owned(),
            previous_classification: Some(Classification::Raw),
            transition_unit: Some("U6".to_owned()),
            revalidated: false,
        };

        let mut missing = transition.clone();
        missing.functions.clear();
        let error = apply_reviewed_migration_overrides(
            &mut reviewed.clone(),
            &reviewed,
            &fixture.contract,
            &missing,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("an active classification transition requires an explicit override");
        assert!(
            error
                .to_string()
                .contains("missing=[\"b2Body_SetTransform\"]")
        );

        let mut unauthenticated_transition = transition.clone();
        unauthenticated_transition.functions[0].previous_classification = None;
        let error = apply_reviewed_migration_overrides(
            &mut reviewed.clone(),
            &reviewed,
            &fixture.contract,
            &unauthenticated_transition,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("an active transition must authenticate its historical classification");
        assert!(
            error
                .to_string()
                .contains("requires previous_classification=raw and classification=safe")
        );

        let mut wrong_target = transition.clone();
        wrong_target.functions[0].classification = Classification::Raw;
        let error = apply_reviewed_migration_overrides(
            &mut reviewed.clone(),
            &reviewed,
            &fixture.contract,
            &wrong_target,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("an active transition override must reproduce its reviewed target");
        assert!(
            error
                .to_string()
                .contains("requires previous_classification=raw and classification=safe")
        );

        let mut unauthenticated_active = fixture.contract.clone();
        unauthenticated_active.functions[0].classification = Classification::Raw;
        let error = apply_reviewed_migration_overrides(
            &mut reviewed.clone(),
            &reviewed,
            &unauthenticated_active,
            &transition,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("the active reviewed contract must authenticate the transition target");
        assert!(
            error
                .to_string()
                .contains("unexpected=[\"b2Body_SetTransform\"]")
        );

        let mut binding_routes = fixture.binding_routes.clone();
        let wasm_route = AbiBindingRoute {
            mode: "single".to_owned(),
            provider: "wasm-runtime".to_owned(),
            artifact: "bindings-single".to_owned(),
            rust_target: RustTarget::Wasm32UnknownUnknown,
            rust_features: Vec::new(),
        };
        binding_routes.insert(
            (wasm_route.mode.clone(), wasm_route.provider.clone()),
            wasm_route,
        );
        let active_change = ClassificationChange {
            logical_name: "b2Body_SetTransform".to_owned(),
            from: Classification::Raw,
            to: Classification::Safe,
            unit: "stale-unit".to_owned(),
            rationale: "This active provenance is superseded by the reviewed migration row."
                .to_owned(),
        };
        let mut authenticated_active = fixture.contract.clone();
        authenticated_active.classification_changes = vec![active_change.clone()];
        let mut migrated = reviewed.clone();
        apply_reviewed_migration_overrides(
            &mut migrated,
            &reviewed,
            &authenticated_active,
            &transition,
            &[],
            &fixture.inventory,
            &binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect("both immutable and active contracts authenticate the exact transition");

        let function = &migrated.functions[0];
        assert_eq!(function.classification, Classification::Safe);
        assert_eq!(
            function.provider_overrides[0].providers,
            ["wasm-runtime".to_owned()]
        );
        assert_eq!(
            migrated.classification_changes,
            [ClassificationChange {
                logical_name: "b2Body_SetTransform".to_owned(),
                from: Classification::Raw,
                to: Classification::Safe,
                unit: "U6".to_owned(),
                rationale: transition.functions[0].rationale.clone(),
            }]
        );
    }

    #[test]
    fn reviewed_migration_does_not_require_overrides_for_removed_transitions() {
        let fixture = ContractFixture::create();
        let mut reviewed = fixture.contract.clone();
        reviewed.functions[0].classification = Classification::Raw;
        reviewed.functions[0].rust_paths = vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()];
        let mut migrated = reviewed.clone();
        migrated.functions.clear();
        let mut overrides = reviewed_added_function_override(&fixture);
        overrides.functions.clear();

        apply_reviewed_migration_overrides(
            &mut migrated,
            &reviewed,
            &fixture.contract,
            &overrides,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect("a removed function cannot require an active classification override");

        assert!(migrated.functions.is_empty());
        assert!(migrated.classification_changes.is_empty());
    }

    #[test]
    fn reviewed_inheritance_clears_provider_overrides_after_safe_to_raw_reversion() {
        let fixture = ContractFixture::create();
        let mut binding_routes = fixture.binding_routes.clone();
        let wasm_route = AbiBindingRoute {
            mode: "single".to_owned(),
            provider: "wasm-runtime".to_owned(),
            artifact: "bindings-single".to_owned(),
            rust_target: RustTarget::Wasm32UnknownUnknown,
            rust_features: Vec::new(),
        };
        binding_routes.insert(
            (wasm_route.mode.clone(), wasm_route.provider.clone()),
            wasm_route,
        );

        let mut current = fixture.contract.clone();
        synchronize_wasm_function_overrides(
            &mut current.functions[0],
            &fixture.inventory.functions[0],
            &binding_routes,
            &fixture.rust_indexes,
        );
        assert_eq!(
            current.functions[0].provider_overrides[0].providers,
            ["wasm-runtime".to_owned()]
        );

        let mut reviewed = current.clone();
        reviewed.functions[0].classification = Classification::Raw;
        reviewed.functions[0].rust_paths = vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()];
        reviewed.functions[0].provider_overrides.clear();

        let gaps = inherit_reviewed_function_semantics(
            &mut current,
            &reviewed,
            &fixture.inventory,
            None,
            &binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        );

        assert!(gaps.is_empty());
        assert_eq!(current.functions[0].classification, Classification::Raw);
        assert!(current.functions[0].provider_overrides.is_empty());
    }

    #[test]
    fn reviewed_migration_rejects_placeholder_review_text() {
        let fixture = ContractFixture::create();
        let mut reviewed = fixture.contract.clone();
        reviewed.functions.clear();
        let mut overrides = reviewed_added_function_override(&fixture);
        overrides.functions[0].area = "Unreviewed upstream box2d.h".to_owned();

        let error = apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &reviewed,
            &fixture.contract,
            &overrides,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("placeholder review text must never enter the reviewed contract");
        assert!(error.to_string().contains("placeholder review text"));
    }

    #[test]
    fn reviewed_migration_logged_mutation_refresh_requires_recording_session_paths() {
        let fixture = ContractFixture::create();
        let mut overrides = reviewed_added_function_override(&fixture);
        overrides.functions.clear();
        overrides.canonical_refreshes = vec![ReviewedCanonicalRefresh {
            logical_name: "b2Body_SetTransform".to_owned(),
            rust_paths: vec!["boxdd::RecordingSession::try_set_transform".to_owned()],
            rationale: "RecordingSession validates the body identity before recording the transform mutation."
                .to_owned(),
        }];
        apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &fixture.contract,
            &fixture.contract,
            &overrides,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect("a reviewed logged mutation may move to its RecordingSession path");

        overrides.canonical_refreshes[0].rust_paths = vec!["boxdd::set_transform".to_owned()];
        let error = apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &fixture.contract,
            &fixture.contract,
            &overrides,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("a canonical refresh cannot bless an arbitrary alternate path");
        assert!(error.to_string().contains("RecordingSession paths"));
    }

    fn reviewed_default_constructor_fixture()
    -> (ContractFixture, ApiContract, ReviewedMigrationOverrides) {
        let fixture = ContractFixture::create_default_constructor_refresh();
        let mut reviewed = fixture.contract.clone();
        reviewed.functions[0].rust_paths = vec!["boxdd::Filter::default".to_owned()];
        let mut overrides = reviewed_added_function_override(&fixture);
        overrides.functions.clear();
        overrides.canonical_refreshes = vec![ReviewedCanonicalRefresh {
            logical_name: "b2DefaultFilter".to_owned(),
            rust_paths: vec!["boxdd::Filter::new".to_owned()],
            rationale: "Filter::new preserves the reviewed native defaults through an inherent constructor."
                .to_owned(),
        }];
        (fixture, reviewed, overrides)
    }

    #[test]
    fn reviewed_migration_accepts_same_owner_default_constructors() {
        let (fixture, reviewed, overrides) = reviewed_default_constructor_fixture();

        for constructor in ["new", "builder"] {
            let mut contract = fixture.contract.clone();
            let mut candidate = overrides.clone();
            candidate.canonical_refreshes[0].rust_paths =
                vec![format!("boxdd::Filter::{constructor}")];
            apply_reviewed_migration_overrides(
                &mut contract,
                &reviewed,
                &fixture.contract,
                &candidate,
                &[],
                &fixture.inventory,
                &fixture.binding_routes,
                &fixture.rust_indexes,
                &fixture.operations,
            )
            .expect("a reviewed default may move to a same-owner inherent constructor");
            assert_eq!(
                contract.functions[0].rust_paths,
                candidate.canonical_refreshes[0].rust_paths
            );
        }
    }

    #[test]
    fn reviewed_migration_default_refreshes_fail_closed() {
        let (fixture, reviewed, overrides) = reviewed_default_constructor_fixture();

        let mut missing = overrides.clone();
        missing.canonical_refreshes.clear();
        let error = apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &reviewed,
            &fixture.contract,
            &missing,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("every reviewed default constructor refresh is required");
        assert!(error.to_string().contains("missing=[\"b2DefaultFilter\"]"));

        let mut unexpected = overrides.clone();
        unexpected
            .canonical_refreshes
            .push(ReviewedCanonicalRefresh {
                logical_name: "b2DefaultOther".to_owned(),
                rust_paths: vec!["boxdd::Other::new".to_owned()],
                rationale: "An unrelated default must not enter the reviewed refresh set."
                    .to_owned(),
            });
        let error = apply_reviewed_migration_overrides(
            &mut fixture.contract.clone(),
            &reviewed,
            &fixture.contract,
            &unexpected,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect_err("unrelated default refreshes must fail closed");
        assert!(
            error
                .to_string()
                .contains("unexpected=[\"b2DefaultOther\"]")
        );

        for path in ["boxdd::Other::new", "boxdd::Filter::from_raw"] {
            let mut candidate = overrides.clone();
            candidate.canonical_refreshes[0].rust_paths = vec![path.to_owned()];
            let error = apply_reviewed_migration_overrides(
                &mut fixture.contract.clone(),
                &reviewed,
                &fixture.contract,
                &candidate,
                &[],
                &fixture.inventory,
                &fixture.binding_routes,
                &fixture.rust_indexes,
                &fixture.operations,
            )
            .expect_err("default refreshes require a same-owner new or builder path");
            assert!(error.to_string().contains("same-owner `new` or `builder`"));
        }
    }

    #[test]
    fn reviewed_migration_preserves_applicable_historical_classification_changes() {
        let fixture = ContractFixture::create();
        let mut reviewed = fixture.contract.clone();
        reviewed.classification_changes = vec![ClassificationChange {
            logical_name: "b2Body_SetTransform".to_owned(),
            from: Classification::Raw,
            to: Classification::Safe,
            unit: "historical-review".to_owned(),
            rationale: "The historical review promoted the validated wrapper to Safe Rust."
                .to_owned(),
        }];
        let mut contract = fixture.contract.clone();
        let mut overrides = reviewed_added_function_override(&fixture);
        overrides.functions.clear();
        overrides.canonical_refreshes = vec![ReviewedCanonicalRefresh {
            logical_name: "b2Body_SetTransform".to_owned(),
            rust_paths: vec!["boxdd::RecordingSession::try_set_transform".to_owned()],
            rationale: "RecordingSession validates the body identity before recording the transform mutation."
                .to_owned(),
        }];

        apply_reviewed_migration_overrides(
            &mut contract,
            &reviewed,
            &fixture.contract,
            &overrides,
            &[],
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        )
        .expect("unchanged historical Safe review should migrate");

        assert_eq!(
            contract.classification_changes,
            reviewed.classification_changes
        );
    }

    #[test]
    fn function_reconcile_preserves_exact_rows_and_fails_new_or_changed_rows_to_raw() {
        let exact = ContractFixture::create();
        let mut exact_contract = exact.contract.clone();
        reconcile_functions(
            &mut exact_contract,
            &exact.inventory,
            None,
            &exact.binding_routes,
            &exact.rust_indexes,
            &exact.operations,
        );
        assert_eq!(
            exact_contract.functions[0].classification,
            Classification::Safe
        );
        assert_eq!(
            exact_contract.functions[0].rust_paths,
            ["boxdd::set_transform"]
        );

        let mut added_inventory = exact.inventory.clone();
        added_inventory.functions.push(crate::c_api::FunctionDecl {
            name: "b2Added".to_owned(),
            signature: "void b2Added ( void )".to_owned(),
            fingerprint: "fnv1a64:added".to_owned(),
            parameters: Vec::new(),
            physical_symbols: BTreeMap::from([
                ("double".to_owned(), "b2Added".to_owned()),
                ("single".to_owned(), "b2Added".to_owned()),
            ]),
            availability: vec!["always".to_owned()],
            header: "box2d.h".to_owned(),
            line: 2,
        });
        let mut added_contract = exact.contract.clone();
        reconcile_functions(
            &mut added_contract,
            &added_inventory,
            None,
            &exact.binding_routes,
            &exact.rust_indexes,
            &exact.operations,
        );
        let added = added_contract
            .functions
            .iter()
            .find(|function| function.logical_name == "b2Added")
            .expect("added function row");
        assert_eq!(added.classification, Classification::Raw);
        assert_eq!(added.rust_paths, ["boxdd_sys::ffi::b2Added"]);
        assert!(
            added
                .rationale
                .contains("conservatively exposed only through raw FFI")
        );

        let mut changed_inventory = exact.inventory.clone();
        changed_inventory.functions[0].signature =
            "void b2Body_SetTransform ( int changed )".to_owned();
        changed_inventory.functions[0].fingerprint = "fnv1a64:changed".to_owned();
        changed_inventory.functions[0].parameters = vec!["int changed".to_owned()];
        let mut changed_contract = exact.contract.clone();
        reconcile_functions(
            &mut changed_contract,
            &changed_inventory,
            None,
            &exact.binding_routes,
            &exact.rust_indexes,
            &exact.operations,
        );
        assert_eq!(
            changed_contract.functions[0].classification,
            Classification::Raw
        );
        assert!(
            changed_contract
                .classification_changes
                .iter()
                .any(|change| {
                    change.logical_name == "b2Body_SetTransform"
                        && change.from == Classification::Safe
                        && change.to == Classification::Raw
                })
        );

        let mut relinked_inventory = exact.inventory.clone();
        relinked_inventory.functions[0]
            .physical_symbols
            .insert("single".to_owned(), "b2Body_SetTransformV2".to_owned());
        let mut relinked_contract = exact.contract.clone();
        reconcile_functions(
            &mut relinked_contract,
            &relinked_inventory,
            None,
            &exact.binding_routes,
            &exact.rust_indexes,
            &exact.operations,
        );
        assert_eq!(
            relinked_contract.functions[0].classification,
            Classification::Raw,
            "a same-signature physical symbol transition invalidates the previous Safe review"
        );
        assert_eq!(
            relinked_contract.functions[0].rust_paths,
            ["boxdd_sys::ffi::b2Body_SetTransformV2"]
        );

        let mut deleted_contract = exact.contract.clone();
        reconcile_functions(
            &mut deleted_contract,
            &CApiInventory::default(),
            None,
            &exact.binding_routes,
            &exact.rust_indexes,
            &exact.operations,
        );
        assert!(deleted_contract.functions.is_empty());
        assert_eq!(
            deleted_contract.migration_baseline,
            CoverageCounts::default()
        );
    }

    fn precision_function_inventory(
        name: &str,
        parameter: AbiPrimitive,
    ) -> (PrecisionCApiInventory, String) {
        let shape = AbiTypeShape::Function {
            result: Box::new(AbiTypeShape::Primitive {
                primitive: AbiPrimitive::Void,
            }),
            parameters: vec![AbiTypeShape::Primitive {
                primitive: parameter,
            }],
            variadic: false,
        };
        let fingerprint = shape.fingerprint();
        (
            PrecisionCApiInventory {
                precision: CAbiPrecision::Single,
                functions: vec![AbiCallableShape {
                    name: name.to_owned(),
                    shape,
                    fingerprint: fingerprint.clone(),
                    signature: format!("void {name} ( float value )"),
                    header: "box2d.h".to_owned(),
                    line: 1,
                }],
                ..PrecisionCApiInventory::default()
            },
            fingerprint,
        )
    }

    #[test]
    fn function_reconcile_requires_recursive_abi_fingerprints_for_safe_review() {
        let fixture = ContractFixture::create();
        let (inventory, fingerprint) =
            precision_function_inventory("b2Body_SetTransform", AbiPrimitive::F32);
        let precision_inventories =
            AbiPrecisionInventories::from([("single".to_owned(), inventory)]);

        let mut preserved = fixture.contract.clone();
        preserved.functions[0].abi_fingerprints =
            BTreeMap::from([("single".to_owned(), fingerprint)]);
        reconcile_functions(
            &mut preserved,
            &fixture.inventory,
            Some(&precision_inventories),
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        );
        assert_eq!(preserved.functions[0].classification, Classification::Safe);

        let (changed_inventory, _) =
            precision_function_inventory("b2Body_SetTransform", AbiPrimitive::F64);
        let changed_precision =
            AbiPrecisionInventories::from([("single".to_owned(), changed_inventory)]);
        let mut changed = preserved;
        reconcile_functions(
            &mut changed,
            &fixture.inventory,
            Some(&changed_precision),
            &fixture.binding_routes,
            &fixture.rust_indexes,
            &fixture.operations,
        );
        assert_eq!(
            changed.functions[0].classification,
            Classification::Raw,
            "a f32/f64 recursive ABI drift must invalidate a Safe review"
        );
    }

    #[test]
    fn precision_validator_rejects_f64_c_function_against_f32_binding() {
        let mut fixture = ContractFixture::create();
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        let source = fs::read_to_string(&bindings).expect("binding fixture");
        fs::write(
            &bindings,
            source.replace(
                "pub fn b2Body_SetTransform();",
                "pub fn b2Body_SetTransform(value: f32);",
            ),
        )
        .expect("f32 binding fixture");
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("f32 binding index");

        let (inventory, fingerprint) =
            precision_function_inventory("b2Body_SetTransform", AbiPrimitive::F64);
        let precision_inventories =
            AbiPrecisionInventories::from([("single".to_owned(), inventory)]);
        fixture.contract.functions[0].abi_fingerprints =
            BTreeMap::from([("single".to_owned(), fingerprint)]);

        let error = validate_contract(
            &fixture.paths(),
            &fixture.contract,
            &fixture.inventory,
            Some(&precision_inventories),
            &fixture.rust_indexes,
            &fixture.binding_routes,
            &fixture.binding_indexes,
            &fixture.active_revision,
            &fixture.operations,
        )
        .expect_err("a f64 C function must not validate against a f32 binding");
        assert!(error.to_string().contains("Rust ABI fingerprint"));
        assert!(error.to_string().contains("b2Body_SetTransform"));
    }

    #[test]
    fn registry_validation_rejects_unknown_and_duplicate_values() {
        let mut errors = Vec::new();
        validate_registry_values(
            "b2Example",
            "provider",
            &[
                "source".to_owned(),
                "source".to_owned(),
                "dynamic".to_owned(),
            ],
            &["source"].into_iter().collect(),
            &mut errors,
        );
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn contract_rejects_nonexistent_path_missing_rationale_and_unknown_registry_values() {
        let mut fixture = ContractFixture::create();
        fixture.contract.functions[0].rust_paths = vec!["boxdd::missing".to_owned()];
        fixture.contract.functions[0].rationale = "todo".to_owned();
        fixture.contract.functions[0].modes.push("quad".to_owned());
        fixture.contract.functions[0]
            .providers
            .push("ambient-dynamic".to_owned());

        let error = fixture.validate().expect_err("invalid metadata must fail");

        assert!(
            error
                .to_string()
                .contains("nonexistent public safe callable `boxdd::missing`")
        );
        assert!(error.to_string().contains("specific rationale"));
        assert!(error.to_string().contains("unknown mode `quad`"));
        assert!(
            error
                .to_string()
                .contains("unknown provider `ambient-dynamic`")
        );
    }

    #[test]
    fn contract_rejects_missing_or_comment_only_test_evidence() {
        let mut missing = ContractFixture::create();
        missing.contract.evidence[0].file = "boxdd/tests/missing.rs".to_owned();
        let error = missing
            .validate()
            .expect_err("missing evidence file must fail");
        assert!(error.to_string().contains("missing.rs"));

        let mut comment_only = ContractFixture::create();
        comment_only.contract.evidence[0].file = "boxdd/tests/comment_only.rs".to_owned();
        let error = comment_only
            .validate()
            .expect_err("comment-only evidence must fail");
        assert!(
            error
                .to_string()
                .contains("expected exactly one function `covers_body_set_transform`, found 0")
        );
    }

    #[test]
    fn evidence_fingerprints_and_safe_call_witnesses_fail_closed() {
        let mut drifted = ContractFixture::create();
        drifted.contract.evidence[0].fingerprint =
            "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
        let error = drifted
            .validate()
            .expect_err("an edited evidence test must require review");
        assert!(error.to_string().contains("fingerprint drifted"));

        let mut missing = ContractFixture::create();
        missing.contract.evidence[0].call_witnesses.clear();
        let error = missing
            .validate()
            .expect_err("a missing Safe-call witness must fail");
        assert!(
            error
                .to_string()
                .contains("has no exact executable call witness")
        );

        let mut extra = ContractFixture::create();
        extra.contract.evidence[0].call_witnesses = vec![SafeCallWitness {
            function: "b2Other".to_owned(),
            rust_path: "boxdd::set_transform".to_owned(),
        }];
        let error = extra
            .validate()
            .expect_err("an unknown Safe-call witness must fail");
        assert!(
            error
                .to_string()
                .contains("witnesses unknown function `b2Other`")
        );

        let mut duplicate = ContractFixture::create();
        duplicate.contract.evidence[0]
            .call_witnesses
            .push(SafeCallWitness {
                function: "b2Body_SetTransform".to_owned(),
                rust_path: "boxdd::set_transform".to_owned(),
            });
        let error = duplicate
            .validate()
            .expect_err("a repeated Safe-call witness must fail");
        assert!(
            error
                .to_string()
                .contains("Safe-call witnesses must be unique")
        );

        let mut orphan = ContractFixture::create();
        let mut orphan_evidence = orphan.contract.evidence[0].clone();
        orphan_evidence.id = "orphan-runtime".to_owned();
        orphan.contract.evidence.push(orphan_evidence);
        let error = orphan
            .validate()
            .expect_err("an unreferenced evidence row must fail");
        assert!(
            error
                .to_string()
                .contains("evidence `orphan-runtime` is orphaned")
        );
    }

    #[test]
    fn typed_function_classification_evidence_rejects_unrelated_subjects() {
        let mut fixture = ContractFixture::create();
        fixture.install_classification_evidence();
        fixture
            .contract
            .evidence
            .retain(|evidence| evidence.id == API_CLASSIFICATION_EVIDENCE_ID);
        let function = &mut fixture.contract.functions[0];
        function.classification = Classification::Raw;
        function.rust_paths = Vec::from(["boxdd_sys::ffi::b2Body_SetTransform".to_owned()]);
        function.evidence = Vec::from([API_CLASSIFICATION_EVIDENCE_ID.to_owned()]);
        function.recording = None;
        fixture.contract.classification_changes = Vec::from([ClassificationChange {
            logical_name: "b2Body_SetTransform".to_owned(),
            from: Classification::Safe,
            to: Classification::Raw,
            unit: "typed-evidence-test".to_owned(),
            rationale:
                "The fixture deliberately exercises the conservative raw classification gate."
                    .to_owned(),
        }]);

        validate_contract(
            &fixture.paths(),
            &fixture.contract,
            &fixture.inventory,
            None,
            &fixture.rust_indexes,
            &fixture.binding_routes,
            &fixture.binding_indexes,
            &fixture.active_revision,
            &fixture.operations,
        )
        .expect("the exact classification witness must pass the production validator");

        fixture.contract.evidence[0].classification_witnesses[0].function =
            "b2Unrelated".to_owned();
        let error = fixture
            .validate()
            .expect_err("an unrelated classification subject must fail");
        assert!(
            error
                .to_string()
                .contains("classifies unknown function `b2Unrelated`")
        );

        fixture.contract.evidence[0].classification_witnesses[0].function =
            "b2Body_SetTransform".to_owned();
        fixture.contract.evidence[0].classification_witnesses[0].classification =
            Classification::Omitted;
        let error = fixture
            .validate()
            .expect_err("a wrong expected classification must fail");
        assert!(
            error
                .to_string()
                .contains("records `b2Body_SetTransform` as omitted")
        );
    }

    #[test]
    fn classification_evidence_invokes_production_validator_on_a_straight_line_path() {
        let paths = WorkspacePaths::discover().expect("workspace paths");
        let index = crate::rust_index::index_test_evidence_for_gate(
            paths.root(),
            "xtask/src/commands/api_coverage.rs",
            "typed_function_classification_evidence_rejects_unrelated_subjects",
            "xtask",
            "nextest",
            &RustIndex::default(),
        )
        .expect("classification evidence index");

        assert!(
            index.called_local_paths.iter().any(|path| {
                path == "validate_contract" || path.ends_with("::validate_contract")
            }),
            "classification evidence must invoke validate_contract before any opaque control flow; unresolved calls: {:?}",
            index.unresolved_calls
        );
    }

    #[test]
    fn contract_rejects_duplicate_rows_and_header_function_drift() {
        let mut duplicate = ContractFixture::create();
        duplicate
            .contract
            .functions
            .push(duplicate.contract.functions[0].clone());
        let error = duplicate
            .validate()
            .expect_err("duplicate contract row must fail");
        assert!(
            error
                .to_string()
                .contains("duplicate API contract function")
        );

        let mut added = ContractFixture::create();
        added.inventory.functions.push(crate::c_api::FunctionDecl {
            name: "b2NewFunction".to_owned(),
            signature: "void b2NewFunction ( void )".to_owned(),
            fingerprint: "fnv1a64:new".to_owned(),
            parameters: Vec::new(),
            physical_symbols: BTreeMap::from([
                ("single".to_owned(), "b2NewFunction".to_owned()),
                ("double".to_owned(), "b2NewFunction".to_owned()),
            ]),
            availability: vec!["always".to_owned()],
            header: "box2d.h".to_owned(),
            line: 2,
        });
        let error = added
            .validate()
            .expect_err("unclassified header function must fail");
        assert!(error.to_string().contains("has no contract row"));

        let mut deleted = ContractFixture::create();
        deleted.inventory.functions.clear();
        let error = deleted
            .validate()
            .expect_err("deleted header function must fail");
        assert!(error.to_string().contains("absent from active headers"));
    }

    #[test]
    fn contract_rejects_legal_but_wrong_physical_symbols_and_availability() {
        let mut wrong_symbol = ContractFixture::create();
        let bindings = wrong_symbol
            .root
            .join("boxdd-sys/src/bindings_pregenerated.rs");
        let mut source = fs::read_to_string(&bindings).expect("binding fixture");
        source.push_str("\nunsafe extern \"C\" { pub fn b2Other(); }\n");
        fs::write(&bindings, source).expect("binding fixture with a second legal symbol");
        wrong_symbol
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index");
        wrong_symbol.contract.functions[0]
            .link_symbols
            .insert("single".to_owned(), "b2Other".to_owned());
        let error = wrong_symbol
            .validate()
            .expect_err("an existing but unrelated physical symbol must fail");
        assert!(
            error
                .to_string()
                .contains("does not match header-derived physical symbol")
        );

        let mut wrong_availability = ContractFixture::create();
        wrong_availability.contract.functions[0].availability = vec!["debug-profile".to_owned()];
        let error = wrong_availability
            .validate()
            .expect_err("a legal but header-inaccurate availability must fail");
        assert!(error.to_string().contains("availability drift"));
    }

    #[test]
    fn contract_rejects_binding_function_absent_from_headers() {
        let mut fixture = ContractFixture::create();
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        let mut source = fs::read_to_string(&bindings).expect("binding fixture");
        source.push_str("\nunsafe extern \"C\" { pub fn b2MissingFromParser(); }\n");
        fs::write(&bindings, source).expect("binding fixture with extra public function");
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index");

        let error = fixture
            .validate()
            .expect_err("a generated function absent from headers must fail closed");
        assert!(
            error
                .to_string()
                .contains("extra [\"b2MissingFromParser\"]"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn binding_surface_rejects_cfg_and_link_name_on_public_functions() {
        for (replacement, expected) in [
            (
                "#[cfg(any())] pub fn b2Body_SetTransform();",
                "uses unsupported `#[cfg]`",
            ),
            (
                "#[link_name = \"b2WrongSymbol\"] pub fn b2Body_SetTransform();",
                "uses unsupported `#[link_name]`",
            ),
        ] {
            let mut fixture = ContractFixture::create();
            let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
            let source = fs::read_to_string(&bindings).expect("binding fixture");
            fs::write(
                &bindings,
                source.replace("pub fn b2Body_SetTransform();", replacement),
            )
            .expect("binding fixture with unsupported function attribute");

            let error = fixture
                .binding_indexes
                .get_mut("bindings-single")
                .expect("binding artifact")
                .refresh_from_path(&bindings)
                .expect_err("conditional or renamed ABI functions must fail closed");
            assert!(
                error.to_string().contains(expected),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn pinned_box2d_target_requires_exact_export_count() {
        let mut fixture = ContractFixture::create();
        fixture.active_revision = BOX2D_3_2_TARGET_REVISION.to_owned();
        fixture.contract.upstream_sha = BOX2D_3_2_TARGET_REVISION.to_owned();

        let error = fixture
            .validate()
            .expect_err("the pinned target count cannot silently shrink");
        assert!(
            error
                .to_string()
                .contains("exposes 1 header functions, expected exactly 478"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn abi_capability_mapping_rejects_deleted_forged_and_unknown_references() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        validate_contract(
            &fixture.paths(),
            &fixture.contract,
            &fixture.inventory,
            None,
            &fixture.rust_indexes,
            &fixture.binding_routes,
            &fixture.binding_indexes,
            &fixture.active_revision,
            &fixture.operations,
        )
        .expect("the reviewed ABI mapping must pass the production validator");
        fixture.contract.abi.structs[0].fields.clear();
        fixture.contract.abi.callbacks.clear();
        let error = fixture
            .validate()
            .expect_err("deleted ABI mappings must fail");
        assert!(error.to_string().contains("no explicit capability mapping"));
        assert!(error.to_string().contains("callback `b2ExampleCallback`"));

        let mut forged = ContractFixture::create();
        forged.enable_abi_capabilities();
        forged.contract.abi.structs[0].fields[0].raw_mappings[0].steps[0].field =
            "other".to_owned();
        forged.contract.abi.callbacks[0].raw_mappings[0].path =
            "boxdd_sys::ffi::b2Example".to_owned();
        let error = forged
            .validate()
            .expect_err("existing but unrelated Rust paths must fail");
        assert!(error.to_string().contains("forged or stale"));
        assert!(
            error
                .to_string()
                .contains("expected canonical generated callback path")
        );

        let mut unknown = ContractFixture::create();
        unknown.enable_abi_capabilities();
        unknown.contract.abi.structs[0].fields[0].policy = "missing-policy".to_owned();
        unknown.contract.abi.policies[0].evidence = vec!["missing-evidence".to_owned()];
        let error = unknown
            .validate()
            .expect_err("unknown policy and evidence references must fail");
        assert!(error.to_string().contains("unknown ABI policy"));
        assert!(error.to_string().contains("unknown evidence"));

        let mut unsupported = ContractFixture::create();
        unsupported.enable_abi_capabilities();
        unsupported.contract.abi.policies[0]
            .modes
            .push("double".to_owned());
        unsupported.contract.abi.policies[0]
            .providers
            .push("system-static".to_owned());
        let error = unsupported
            .validate()
            .expect_err("unimplemented ABI coordinates must fail");
        assert!(error.to_string().contains("unsupported mode `double`"));
        assert!(
            error
                .to_string()
                .contains("unsupported provider `system-static`")
        );

        let mut drifted = ContractFixture::create();
        drifted.enable_abi_capabilities();
        drifted.contract.abi.structs[0].fingerprint = "fnv1a64:forged".to_owned();
        drifted.contract.abi.structs[0].fields[0].signature = "long count".to_owned();
        drifted.contract.abi.callbacks[0].signature = "void forged ( void )".to_owned();
        let error = drifted
            .validate()
            .expect_err("ABI declaration drift must fail");
        assert!(error.to_string().contains("declaration drifted"));
    }

    #[test]
    fn abi_evidence_roles_cannot_bless_empty_named_tests() {
        for (id, required_entry) in [
            (ABI_HEADER_EVIDENCE_ID, "parse_headers_for_precision"),
            (ABI_BINDING_EVIDENCE_ID, "index_bindings"),
            (ABI_VALIDATOR_EVIDENCE_ID, "validate_contract"),
        ] {
            let mut fixture = ContractFixture::create();
            fixture.enable_abi_capabilities();
            let evidence_position = fixture
                .contract
                .evidence
                .iter()
                .position(|evidence| evidence.id == id)
                .expect("ABI evidence row");
            let evidence = fixture.contract.evidence[evidence_position].clone();
            fs::write(
                fixture.root.join(&evidence.file),
                format!("#[test]\nfn {}() {{ assert!(true); }}\n", evidence.item),
            )
            .expect("empty named evidence test");
            let indexed_routes = index_evidence_across_routes(
                &fixture.paths(),
                &evidence,
                &fixture.rust_indexes,
                &fixture.binding_routes,
            )
            .expect("refreshed empty-test evidence index");
            fixture.contract.evidence[evidence_position].fingerprint =
                aggregate_evidence_fingerprint(&indexed_routes);

            let error = fixture
                .validate()
                .expect_err("a refreshed fingerprint cannot replace a production validator call");
            assert!(error.to_string().contains(&format!(
                "does not invoke required production entry `{required_entry}`"
            )));
            assert!(!error.to_string().contains("fingerprint drifted"));
        }
    }

    #[test]
    fn abi_evidence_scopes_follow_every_route_without_widening_safe_call_evidence() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        let safe_call_scope = fixture
            .contract
            .evidence
            .iter()
            .find(|evidence| evidence.role == TestEvidenceRole::SafeCall)
            .map(|evidence| (evidence.modes.clone(), evidence.providers.clone()))
            .expect("Safe-call evidence row");
        let routes = pinned_box2d_3_2_binding_routes();
        let route_coordinates = routes.keys().cloned().collect::<BTreeSet<_>>();
        let route_modes = routes
            .keys()
            .map(|(mode, _)| mode.as_str())
            .collect::<BTreeSet<_>>();
        let route_providers = routes
            .keys()
            .map(|(_, provider)| provider.as_str())
            .collect::<BTreeSet<_>>();

        synchronize_abi_evidence_scopes(&mut fixture.contract, &routes);

        for evidence in fixture.contract.evidence.iter().filter(|evidence| {
            matches!(
                evidence.role,
                TestEvidenceRole::AbiHeaderInventory
                    | TestEvidenceRole::AbiBindingAst
                    | TestEvidenceRole::AbiContractValidator
            )
        }) {
            assert_eq!(evidence.modes, ["double", "single"]);
            assert_eq!(
                evidence.providers,
                [
                    "prebuilt-static",
                    "source",
                    "system-static",
                    "wasm-compile-only",
                    "wasm-runtime",
                ]
            );
            let mut errors = Vec::new();
            validate_evidence_scope(
                evidence,
                &route_coordinates,
                &route_modes,
                &route_providers,
                &mut errors,
            );
            assert!(errors.is_empty(), "full ABI evidence scope: {errors:?}");
        }
        let safe_call = fixture
            .contract
            .evidence
            .iter()
            .find(|evidence| evidence.role == TestEvidenceRole::SafeCall)
            .expect("Safe-call evidence row");
        assert_eq!(
            (safe_call.modes.clone(), safe_call.providers.clone()),
            safe_call_scope,
            "ABI provider qualification must not widen Safe-call evidence"
        );
        let mut errors = Vec::new();
        validate_evidence_scope(
            safe_call,
            &route_coordinates,
            &route_modes,
            &route_providers,
            &mut errors,
        );
        assert!(
            errors.is_empty(),
            "Safe-call evidence may cover an exact route subset: {errors:?}"
        );

        let mut narrowed_abi = fixture
            .contract
            .evidence
            .iter()
            .find(|evidence| evidence.role == TestEvidenceRole::AbiBindingAst)
            .cloned()
            .expect("ABI binding evidence row");
        narrowed_abi.modes = vec!["single".to_owned()];
        narrowed_abi.providers = vec!["source".to_owned()];
        errors.clear();
        validate_evidence_scope(
            &narrowed_abi,
            &route_coordinates,
            &route_modes,
            &route_providers,
            &mut errors,
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must exactly cover every binding route")),
            "narrowed ABI evidence must fail closed: {errors:?}"
        );
    }

    #[test]
    fn fingerprint_drift_does_not_hide_required_production_entry() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        let evidence = fixture
            .contract
            .evidence
            .iter_mut()
            .find(|evidence| evidence.id == ABI_VALIDATOR_EVIDENCE_ID)
            .expect("ABI validator evidence row");
        evidence.fingerprint = "blake3-routes-v1:stale".to_owned();

        let error = fixture
            .validate()
            .expect_err("the stale fingerprint must fail closed");
        let message = error.to_string();
        assert!(message.contains("fingerprint drifted"));
        assert!(!message.contains("does not invoke required production entry"));
    }

    #[test]
    fn abi_capability_mapping_resolves_aliases_and_anonymous_union_access_chains() {
        let mut fixture = ContractFixture::create();
        fixture.inventory.structs = vec![
            StructDecl {
                name: "b2Pos".to_owned(),
                fingerprint: "fnv1a64:alias".to_owned(),
                fields: vec![FieldDecl {
                    name: "x".to_owned(),
                    signature: "float x".to_owned(),
                    overlays: Vec::new(),
                }],
                header: "math_functions.h".to_owned(),
                line: 1,
            },
            StructDecl {
                name: "b2TreeNode".to_owned(),
                fingerprint: "fnv1a64:tree-node".to_owned(),
                fields: vec![
                    FieldDecl {
                        name: "children.child1".to_owned(),
                        signature: "int child1".to_owned(),
                        overlays: vec![OverlayDecl {
                            group: "b2TreeNode/union@0".to_owned(),
                            alternative: "children".to_owned(),
                            relative_path: vec!["child1".to_owned()],
                        }],
                    },
                    FieldDecl {
                        name: "userData".to_owned(),
                        signature: "uint64_t userData".to_owned(),
                        overlays: vec![OverlayDecl {
                            group: "b2TreeNode/union@0".to_owned(),
                            alternative: "userData".to_owned(),
                            relative_path: Vec::new(),
                        }],
                    },
                ],
                header: "collision.h".to_owned(),
                line: 2,
            },
        ];
        fixture.contract.abi = map_inventory(
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("aliases and anonymous wrappers must map structurally");
        fixture.install_abi_evidence();
        fixture
            .validate()
            .expect("structured alias and union mappings must validate");

        let alias = &fixture.contract.abi.structs[0];
        assert_eq!(
            alias.raw_mappings[0].resolved_path,
            "boxdd_sys::ffi::b2Vec2"
        );
        assert_eq!(
            alias.fields[0].raw_mappings[0].steps[0].owner_type,
            "boxdd_sys::ffi::b2Vec2"
        );
        let child_steps = &fixture.contract.abi.structs[1].fields[0].raw_mappings[0].steps;
        assert_eq!(
            child_steps
                .iter()
                .map(|step| step.field.as_str())
                .collect::<Vec<_>>(),
            ["__bindgen_anon_1", "children", "child1"]
        );

        fixture.contract.abi.structs[1].fields[0].raw_mappings[0].steps[1].owner_type =
            "boxdd_sys::ffi::b2TreeNode".to_owned();
        let error = fixture
            .validate()
            .expect_err("a forged owner edge in a nested chain must fail");
        assert!(error.to_string().contains("forged or stale"));
        assert!(
            error
                .to_string()
                .contains("absent from the Rust binding AST")
        );
    }

    #[test]
    fn abi_exposure_classification_keeps_raw_proof_and_fails_closed() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();

        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-wrapper".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "A crate-owned public API exposes this capability without leaking native invariants."
                .to_owned();
        let mut omitted_policy = fixture.contract.abi.policies[0].clone();
        omitted_policy.id = "omitted-callback".to_owned();
        omitted_policy.classification = Classification::Omitted;
        omitted_policy.rationale =
            "The callback contract is intentionally unavailable because no sound adapter exists."
                .to_owned();
        fixture.contract.abi.policies.push(safe_policy);
        fixture.contract.abi.policies.push(omitted_policy);
        fixture.contract.abi.structs[0].fields[0].policy = "safe-wrapper".to_owned();
        fixture.contract.abi.structs[0].fields[0].safe_paths =
            vec!["boxdd::Example::from_raw".to_owned()];
        fixture.contract.abi.structs[0].fields[0].safe_witnesses =
            vec![crate::abi_contract::AbiSafeWitness {
                path: "boxdd::Example::from_raw".to_owned(),
                producer_path: None,
                kind: crate::abi_contract::AbiSafeWitnessKind::Accessor,
                raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
                raw_field: Some("count".to_owned()),
                native_symbols: Vec::new(),
            }];
        fixture.contract.abi.callbacks[0].policy = "omitted-callback".to_owned();
        fixture
            .validate()
            .expect("safe and omitted exposure policies should validate with raw ABI proof intact");

        fixture.contract.abi.structs[0].fields[0]
            .raw_mappings
            .clear();
        fixture.contract.abi.structs[0].fields[0].safe_paths = vec!["boxdd::missing".to_owned()];
        fixture.contract.abi.callbacks[0].safe_paths = vec!["boxdd::set_transform".to_owned()];
        let error = fixture
            .validate()
            .expect_err("safe exposure cannot replace raw proof or forge paths");
        assert!(
            error
                .to_string()
                .contains("nonexistent canonical Safe Rust path")
        );
        assert!(error.to_string().contains("cannot claim Safe Rust paths"));
        assert!(
            error
                .to_string()
                .contains("must map every mode/provider coordinate")
        );
    }

    #[test]
    fn abi_safe_witnesses_reject_unrelated_paths_and_wrong_path_kinds() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();

        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-wrapper".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "A crate-owned conversion exposes this exact field without leaking native invariants."
                .to_owned();
        fixture.contract.abi.policies.push(safe_policy);
        let field = &mut fixture.contract.abi.structs[0].fields[0];
        field.policy = "safe-wrapper".to_owned();
        field.safe_paths = vec!["boxdd::Example::from_raw".to_owned()];
        field.safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::Example::from_raw".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::Accessor,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: Some("count".to_owned()),
            native_symbols: Vec::new(),
        }];
        fixture
            .validate()
            .expect("the exact accessor witness should validate");

        let field = &mut fixture.contract.abi.structs[0].fields[0];
        field.safe_paths = vec!["boxdd::set_transform".to_owned()];
        field.safe_witnesses[0].path = "boxdd::set_transform".to_owned();
        let error = fixture
            .validate()
            .expect_err("an existing but unrelated callable must fail closed");
        assert!(
            error
                .to_string()
                .contains("has no exact witness for raw field")
        );

        let field = &mut fixture.contract.abi.structs[0].fields[0];
        field.safe_paths = vec!["boxdd::Example::count".to_owned()];
        field.safe_witnesses[0].path = "boxdd::Example::count".to_owned();
        field.safe_witnesses[0].kind = crate::abi_contract::AbiSafeWitnessKind::Accessor;
        let error = fixture
            .validate()
            .expect_err("a public field cannot masquerade as an accessor");
        assert!(
            error
                .to_string()
                .contains("is not an exact public callable path for Accessor")
        );
    }

    #[test]
    fn public_type_witness_rejects_unsafe_only_raw_conversion() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                pub fn set_transform() {
                    unsafe { boxdd_sys::ffi::b2Body_SetTransform(); }
                }
                pub struct Example { pub count: i32 }
                impl Example {
                    pub unsafe fn from_raw(raw: boxdd_sys::ffi::b2Example) -> Self {
                        Self { count: raw.count }
                    }
                }
            "#,
        )
        .expect("unsafe-only conversion fixture");
        fixture.rust_indexes.insert(
            ("single".to_owned(), "source".to_owned()),
            index_boxdd(&fixture.root).expect("unsafe-only Rust index"),
        );

        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-public-type".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "A Safe public type must have structural storage or a genuinely safe conversion root."
                .to_owned();
        fixture.contract.abi.policies.push(safe_policy);
        let structure = &mut fixture.contract.abi.structs[0];
        structure.policy = "safe-public-type".to_owned();
        structure.safe_paths = vec!["boxdd::Example".to_owned()];
        structure.safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::Example".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::PublicType,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: None,
            native_symbols: Vec::new(),
        }];

        let error = fixture
            .validate()
            .expect_err("an unsafe-only from_raw conversion cannot prove Safe type exposure");
        assert!(
            error
                .to_string()
                .contains("has no exact witness for raw type")
        );
    }

    #[test]
    fn callback_adapter_must_touch_the_exact_callback_field_before_installation() {
        let mut fixture = ContractFixture::create();
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                pub struct World;
                impl World {
                    pub fn new(def: boxdd_sys::ffi::b2WorldDef) -> Self {
                        unsafe { boxdd_sys::ffi::b2Body_SetTransform(&def); }
                        Self
                    }
                }
            "#,
        )
        .expect("Safe Rust fixture");
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        fs::write(
            &bindings,
            r#"
                pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
                pub struct b2WorldDef { pub enqueueTask: b2ExampleCallback }
                unsafe extern "C" {
                    pub fn b2Body_SetTransform(def: *const b2WorldDef);
                }
            "#,
        )
        .expect("generated binding fixture");
        fixture.rust_indexes.insert(
            ("single".to_owned(), "source".to_owned()),
            index_boxdd(&fixture.root).expect("Rust index"),
        );
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index");

        let signature = "void b2Body_SetTransform ( b2WorldDef const * def )".to_owned();
        fixture.inventory.functions[0]
            .signature
            .clone_from(&signature);
        fixture.contract.functions[0].signature = signature;
        fixture.contract.functions[0].rust_paths = vec!["boxdd::World::new".to_owned()];
        fixture.inventory.structs.push(StructDecl {
            name: "b2WorldDef".to_owned(),
            fingerprint: "fnv1a64:world-def".to_owned(),
            fields: vec![FieldDecl {
                name: "enqueueTask".to_owned(),
                signature: "b2ExampleCallback enqueueTask".to_owned(),
                overlays: Vec::new(),
            }],
            header: "types.h".to_owned(),
            line: 1,
        });
        fixture.inventory.callbacks.push(CallbackDecl {
            name: "b2ExampleCallback".to_owned(),
            signature: "void b2ExampleCallback ( void )".to_owned(),
            fingerprint: "fnv1a64:callback".to_owned(),
            header: "types.h".to_owned(),
            line: 2,
        });
        fixture.contract.abi = map_inventory(
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("ABI declarations should map to generated bindings");
        fixture.install_abi_evidence();
        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-callback-adapter".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "A reviewed adapter must install the exact callback field before calling native code."
                .to_owned();
        fixture.contract.abi.policies.push(safe_policy);
        let callback = &mut fixture.contract.abi.callbacks[0];
        callback.policy = "safe-callback-adapter".to_owned();
        callback.safe_paths = vec!["boxdd::World::new".to_owned()];
        callback.safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::World::new".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::CallbackAdapter,
            raw_type: "boxdd_sys::ffi::b2ExampleCallback".to_owned(),
            raw_field: None,
            native_symbols: vec!["b2Body_SetTransform".to_owned()],
        }];

        let error = fixture
            .validate()
            .expect_err("consuming the owner struct cannot prove callback installation");
        assert!(
            error
                .to_string()
                .contains("names unrelated native symbol `b2Body_SetTransform`")
        );
    }

    #[test]
    fn direct_callback_adapter_requires_a_callable_at_the_exact_c_argument() {
        let mut fixture = ContractFixture::create();
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                unsafe extern "C" fn callback() {}
                pub fn overlap() {
                    unsafe { boxdd_sys::ffi::b2Body_SetTransform(Some(callback)); }
                }
            "#,
        )
        .expect("Safe Rust callback fixture");
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        fs::write(
            &bindings,
            r#"
                pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
                unsafe extern "C" {
                    pub fn b2Body_SetTransform(callback: b2ExampleCallback);
                }
            "#,
        )
        .expect("generated binding fixture");
        fixture.rust_indexes.insert(
            ("single".to_owned(), "source".to_owned()),
            index_boxdd(&fixture.root).expect("Rust index"),
        );
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index");

        let signature = "void b2Body_SetTransform ( b2ExampleCallback callback )".to_owned();
        fixture.inventory.functions[0]
            .signature
            .clone_from(&signature);
        fixture.inventory.functions[0].parameters = vec!["b2ExampleCallback callback".to_owned()];
        fixture.contract.functions[0].signature = signature;
        fixture.contract.functions[0].rust_paths = vec!["boxdd::overlap".to_owned()];
        fs::write(
            fixture.root.join("boxdd/tests/evidence.rs"),
            "#[test]\nfn covers_body_set_transform() { boxdd::overlap(); }\n",
        )
        .expect("callback runtime evidence");
        fixture.contract.evidence[0].call_witnesses[0].rust_path = "boxdd::overlap".to_owned();
        fixture.refresh_evidence_fingerprint("body-runtime");
        fixture.inventory.callbacks.push(CallbackDecl {
            name: "b2ExampleCallback".to_owned(),
            signature: "void b2ExampleCallback ( void )".to_owned(),
            fingerprint: "fnv1a64:callback".to_owned(),
            header: "types.h".to_owned(),
            line: 1,
        });
        fixture.contract.abi = map_inventory(
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("ABI callback should map to generated bindings");
        fixture.install_abi_evidence();
        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-callback-adapter".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "A reviewed adapter passes a real Rust callable at the exact native callback slot."
                .to_owned();
        fixture.contract.abi.policies = vec![safe_policy];
        let callback = &mut fixture.contract.abi.callbacks[0];
        callback.policy = "safe-callback-adapter".to_owned();
        callback.safe_paths = vec!["boxdd::overlap".to_owned()];
        callback.safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::overlap".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::CallbackAdapter,
            raw_type: "boxdd_sys::ffi::b2ExampleCallback".to_owned(),
            raw_field: None,
            native_symbols: vec!["b2Body_SetTransform".to_owned()],
        }];
        fixture
            .validate()
            .expect("an exact callable callback argument should validate");

        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                pub fn overlap() {
                    unsafe { boxdd_sys::ffi::b2Body_SetTransform(None); }
                }
            "#,
        )
        .expect("None callback fixture");
        fixture.rust_indexes.insert(
            ("single".to_owned(), "source".to_owned()),
            index_boxdd(&fixture.root).expect("None callback Rust index"),
        );
        let error = fixture
            .validate()
            .expect_err("None at the callback argument cannot prove a Safe adapter");
        assert!(
            error
                .to_string()
                .contains("names unrelated native symbol `b2Body_SetTransform`")
        );
    }

    #[test]
    fn abi_refresh_drops_unchanged_callback_review_when_installation_slot_drifts() {
        let mut fixture = ContractFixture::create();
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                unsafe extern "C" fn callback() {}
                pub fn overlap() {
                    unsafe { boxdd_sys::ffi::b2Body_SetTransform(Some(callback)); }
                }
            "#,
        )
        .expect("Safe Rust callback fixture");
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        fs::write(
            &bindings,
            r#"
                pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
                unsafe extern "C" {
                    pub fn b2Body_SetTransform(callback: b2ExampleCallback);
                }
            "#,
        )
        .expect("generated binding fixture");
        fixture.rust_indexes.insert(
            ("single".to_owned(), "source".to_owned()),
            index_boxdd(&fixture.root).expect("callback Rust index"),
        );
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index");
        fixture.inventory.functions[0].signature =
            "void b2Body_SetTransform ( b2ExampleCallback callback )".to_owned();
        fixture.inventory.functions[0].parameters = vec!["b2ExampleCallback callback".to_owned()];
        fixture.inventory.callbacks.push(CallbackDecl {
            name: "b2ExampleCallback".to_owned(),
            signature: "void b2ExampleCallback ( void )".to_owned(),
            fingerprint: "fnv1a64:callback".to_owned(),
            header: "types.h".to_owned(),
            line: 1,
        });

        let mut previous = map_inventory(
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("callback ABI should map");
        let mut safe_policy = previous.policies[0].clone();
        safe_policy.id = "safe-callback-adapter".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "The exact callback slot receives a panic-contained Rust adapter.".to_owned();
        previous.policies.push(safe_policy);
        let callback = &mut previous.callbacks[0];
        callback.policy = "safe-callback-adapter".to_owned();
        callback.safe_paths = vec!["boxdd::overlap".to_owned()];
        callback.safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::overlap".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::CallbackAdapter,
            raw_type: "boxdd_sys::ffi::b2ExampleCallback".to_owned(),
            raw_field: None,
            native_symbols: vec!["b2Body_SetTransform".to_owned()],
        }];

        let mut exact = map_inventory(
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("exact callback ABI should map");
        preserve_reviewed_exposure(&previous, &mut exact);
        discard_unproven_reviewed_exposure(
            &mut exact,
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
        );
        assert_eq!(exact.callbacks[0].policy, "safe-callback-adapter");

        let mut drifted_inventory = fixture.inventory.clone();
        drifted_inventory.functions[0].signature =
            "void b2Body_SetTransform ( int origin , b2ExampleCallback callback )".to_owned();
        drifted_inventory.functions[0].parameters = vec![
            "int origin".to_owned(),
            "b2ExampleCallback callback".to_owned(),
        ];
        let mut drifted = map_inventory(
            &drifted_inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("drifted callback ABI should still map raw types");
        preserve_reviewed_exposure(&previous, &mut drifted);
        discard_unproven_reviewed_exposure(
            &mut drifted,
            &drifted_inventory,
            &fixture.binding_routes,
            &fixture.rust_indexes,
        );

        assert_eq!(
            drifted.callbacks[0].policy,
            crate::abi_contract::ABI_POLICY_ID
        );
        assert!(drifted.callbacks[0].safe_paths.is_empty());
        assert!(drifted.callbacks[0].safe_witnesses.is_empty());
        assert!(drifted.callbacks[0].rationale.contains("call graph"));
    }

    #[test]
    fn abi_refresh_preserves_exact_reviews_and_maps_added_or_removed_capabilities() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-wrapper".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "A reviewed crate-owned conversion exposes this exact native capability safely."
                .to_owned();
        fixture.contract.abi.policies.push(safe_policy);
        let previous = &mut fixture.contract.abi;
        previous.structs[0].fields[0].policy = "safe-wrapper".to_owned();
        previous.structs[0].fields[0].safe_paths = vec!["boxdd::Example::from_raw".to_owned()];
        previous.structs[0].fields[0].safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::Example::from_raw".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::Accessor,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: Some("count".to_owned()),
            native_symbols: Vec::new(),
        }];
        let previous = fixture.contract.abi.clone();
        let mut refreshed = map_inventory(
            &fixture.inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("raw proof should regenerate");
        preserve_reviewed_exposure(&previous, &mut refreshed);
        assert_eq!(refreshed.structs[0].fields[0].policy, "safe-wrapper");
        assert_eq!(
            refreshed.structs[0].fields[0].safe_paths,
            ["boxdd::Example::from_raw"]
        );
        assert_eq!(
            refreshed.structs[0].fields[0].raw_mappings[0].steps[0].field,
            "count"
        );

        let mut added_inventory = fixture.inventory.clone();
        added_inventory.structs[0].fields.push(FieldDecl {
            name: "other".to_owned(),
            signature: "int other".to_owned(),
            overlays: Vec::new(),
        });
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        let source = fs::read_to_string(&bindings).expect("binding fixture");
        fs::write(
            &bindings,
            source.replace(
                "pub struct b2Example { pub count: i32 }",
                "pub struct b2Example { pub count: i32, pub other: i32 }",
            ),
        )
        .expect("binding fixture with added field");
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index with added field");
        let mut added = map_inventory(
            &added_inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("new raw field should map");
        let mut previous_without_generated_raw_policy = previous.clone();
        previous_without_generated_raw_policy
            .policies
            .retain(|policy| policy.id != "raw-ffi-abi");
        preserve_reviewed_exposure(&previous_without_generated_raw_policy, &mut added);
        assert_eq!(added.structs[0].fields[1].policy, "raw-ffi-abi");
        assert!(
            added
                .policies
                .iter()
                .any(|policy| policy.id == "raw-ffi-abi")
        );
        fixture.inventory = added_inventory;
        fixture.contract.abi = added;
        fixture
            .validate()
            .expect("a newly discovered capability must fail closed to reviewed raw FFI");

        let mut removed_fixture = ContractFixture::create();
        removed_fixture.enable_abi_capabilities();
        let previous = removed_fixture.contract.abi.clone();
        removed_fixture.inventory.structs[0].fields.clear();
        let bindings = removed_fixture
            .root
            .join("boxdd-sys/src/bindings_pregenerated.rs");
        let source = fs::read_to_string(&bindings).expect("binding fixture");
        fs::write(
            &bindings,
            source.replace(
                "pub struct b2Example { pub count: i32 }",
                "pub struct b2Example {}",
            ),
        )
        .expect("binding fixture with removed field");
        removed_fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index with removed field");
        let mut removed = map_inventory(
            &removed_fixture.inventory,
            &removed_fixture.binding_routes,
            &removed_fixture.binding_indexes,
        )
        .expect("remaining raw declarations should map");
        preserve_reviewed_exposure(&previous, &mut removed);
        assert!(removed.structs[0].fields.is_empty());
        removed_fixture.contract.abi = removed;
        removed_fixture
            .validate()
            .expect("a removed capability must not leave a stale reviewed row");
    }

    #[test]
    fn abi_refresh_does_not_inherit_reviews_across_same_name_declaration_drift() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        let mut safe_policy = fixture.contract.abi.policies[0].clone();
        safe_policy.id = "safe-reviewed".to_owned();
        safe_policy.classification = Classification::Safe;
        safe_policy.rationale =
            "The fixture marks exact declarations Safe only so drift inheritance can be tested."
                .to_owned();
        fixture.contract.abi.policies.push(safe_policy);
        let previous = &mut fixture.contract.abi;
        previous.structs[0].policy = "safe-reviewed".to_owned();
        previous.structs[0].safe_paths = vec!["boxdd::Example".to_owned()];
        previous.structs[0].safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::Example".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::PublicType,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: None,
            native_symbols: Vec::new(),
        }];
        previous.structs[0].fields[0].policy = "safe-reviewed".to_owned();
        previous.structs[0].fields[0].safe_paths = vec!["boxdd::Example::from_raw".to_owned()];
        previous.structs[0].fields[0].safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::Example::from_raw".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::Accessor,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: Some("count".to_owned()),
            native_symbols: Vec::new(),
        }];
        previous.callbacks[0].policy = "safe-reviewed".to_owned();
        previous.callbacks[0].safe_paths = vec!["boxdd::set_transform".to_owned()];
        previous.callbacks[0].safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::set_transform".to_owned(),
            producer_path: None,
            kind: crate::abi_contract::AbiSafeWitnessKind::CallbackAdapter,
            raw_type: "boxdd_sys::ffi::b2ExampleCallback".to_owned(),
            raw_field: None,
            native_symbols: vec!["b2Body_SetTransform".to_owned()],
        }];

        let previous = previous.clone();
        let mut changed_inventory = fixture.inventory.clone();
        changed_inventory.structs[0].fingerprint = "fnv1a64:changed-struct".to_owned();
        changed_inventory.structs[0].header = "moved.h".to_owned();
        changed_inventory.structs[0].fields[0].signature = "int * count".to_owned();
        changed_inventory.structs[0].fields[0].overlays = vec![OverlayDecl {
            group: "b2Example/union@0".to_owned(),
            alternative: "count".to_owned(),
            relative_path: Vec::new(),
        }];
        changed_inventory.callbacks[0].signature =
            "bool b2ExampleCallback ( int changed )".to_owned();
        changed_inventory.callbacks[0].fingerprint = "fnv1a64:changed-callback".to_owned();
        changed_inventory.callbacks[0].header = "moved.h".to_owned();
        let bindings = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        let source = fs::read_to_string(&bindings).expect("binding fixture");
        fs::write(
            &bindings,
            source.replace(
                "pub struct b2Example { pub count: i32 }",
                r#"
                    pub struct b2Example {
                        pub __bindgen_anon_1: b2Example__bindgen_ty_1,
                    }
                    pub union b2Example__bindgen_ty_1 {
                        pub count: *mut i32,
                    }
                "#,
            ),
        )
        .expect("binding fixture with changed overlay layout");
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .refresh_from_path(&bindings)
            .expect("binding index with changed overlay layout");
        let mut refreshed = map_inventory(
            &changed_inventory,
            &fixture.binding_routes,
            &fixture.binding_indexes,
        )
        .expect("changed declarations still have raw binding mappings");
        preserve_reviewed_exposure(&previous, &mut refreshed);

        assert_eq!(refreshed.structs[0].policy, "raw-ffi-abi");
        assert!(refreshed.structs[0].safe_witnesses.is_empty());
        assert!(
            refreshed.structs[0]
                .rationale
                .contains("declaration identity or precision-specific raw ABI proof")
        );
        assert_eq!(refreshed.structs[0].fields[0].policy, "raw-ffi-abi");
        assert!(refreshed.structs[0].fields[0].safe_witnesses.is_empty());
        assert!(
            refreshed.structs[0].fields[0]
                .rationale
                .contains("overlay contract")
        );
        assert_eq!(refreshed.callbacks[0].policy, "raw-ffi-abi");
        assert!(refreshed.callbacks[0].safe_witnesses.is_empty());
        assert!(
            refreshed.callbacks[0]
                .rationale
                .contains("declaration identity or precision-specific raw ABI proof")
        );
    }

    #[test]
    fn abi_binding_routes_allow_many_to_one_native_artifacts_and_reject_wrong_coordinates() {
        let mut fixture = ContractFixture::create();
        let source_index = fixture
            .rust_indexes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source Rust index")
            .clone();
        let system_route = AbiBindingRoute {
            mode: "single".to_owned(),
            provider: "system-static".to_owned(),
            artifact: "bindings-single".to_owned(),
            rust_target: RustTarget::X86_64UnknownLinuxGnu,
            rust_features: Vec::new(),
        };
        fixture.binding_routes.insert(
            (system_route.mode.clone(), system_route.provider.clone()),
            system_route,
        );
        fixture.rust_indexes.insert(
            ("single".to_owned(), "system-static".to_owned()),
            source_index,
        );
        fixture.contract.functions[0]
            .providers
            .push("system-static".to_owned());
        fixture.contract.evidence[0]
            .providers
            .push("system-static".to_owned());
        fixture.refresh_evidence_fingerprint("body-runtime");
        fixture.enable_abi_capabilities();
        assert_eq!(fixture.contract.abi.structs[0].raw_mappings.len(), 2);
        assert_eq!(
            fixture.contract.abi.structs[0].fields[0].raw_mappings.len(),
            2
        );

        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .precision = Precision::Double;
        let error = fixture
            .validate()
            .expect_err("a single route cannot use a double binding artifact");
        assert!(error.to_string().contains("is incompatible with artifact"));

        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .precision = Precision::Single;
        fixture
            .binding_indexes
            .get_mut("bindings-single")
            .expect("binding artifact")
            .target = ArtifactTarget::Wasm32UnknownUnknown;
        let error = fixture
            .validate()
            .expect_err("a native route cannot use a WASM binding artifact");
        assert!(error.to_string().contains("is incompatible with artifact"));
    }

    #[test]
    fn route_target_and_expanded_feature_closure_drive_the_production_rust_index() {
        let mut fixture = ContractFixture::create();
        fs::write(
            fixture.root.join("boxdd/Cargo.toml"),
            r#"
                [package]
                name = "boxdd"
                version = "0.0.0"

                [dependencies]
                boxdd-sys = "0"

                [features]
                default = []
                serde = []
                serialize = ["serde"]
            "#,
        )
        .expect("crate manifest with feature alias");
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                #[cfg(target_os = "linux")]
                pub fn linux_only() {}

                #[cfg(target_os = "macos")]
                pub fn macos_only() {}

                #[cfg(feature = "serde")]
                pub fn enabled_through_alias() {}
            "#,
        )
        .expect("target and feature-gated Safe Rust fixture");
        fixture
            .binding_routes
            .get_mut(&("single".to_owned(), "source".to_owned()))
            .expect("source route")
            .rust_features = vec!["serialize".to_owned()];

        let indexes = load_rust_indexes(
            &fixture.paths(),
            &fixture.binding_routes,
            &fixture.binding_indexes,
            None,
        )
        .expect("manifest route should drive the Rust index");
        let index = indexes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source route index");
        assert!(index.contains_public_path("boxdd::linux_only"));
        assert!(!index.contains_public_path("boxdd::macos_only"));
        assert!(index.contains_public_path("boxdd::enabled_through_alias"));
    }

    #[test]
    fn route_binding_return_types_drive_exact_safe_producer_provenance() {
        let mut fixture = ContractFixture::create();
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            r#"
                pub struct Example {
                    pub count: i32,
                }

                pub fn make_example() -> Example {
                    let raw = unsafe { boxdd_sys::ffi::b2MakeExample() };
                    Example { count: raw.count }
                }
            "#,
        )
        .expect("Safe producer fixture");
        let binding_path = fixture.root.join("boxdd-sys/src/bindings_pregenerated.rs");
        fs::write(
            &binding_path,
            r#"
                #[repr(C)]
                pub struct b2Example { pub count: i32 }
                unsafe extern "C" {
                    pub fn b2MakeExample() -> b2Example;
                }
            "#,
        )
        .expect("route binding fixture");
        fixture.inventory.functions = vec![crate::c_api::FunctionDecl {
            name: "b2MakeExample".to_owned(),
            signature: "b2Example b2MakeExample ( void )".to_owned(),
            fingerprint: "fnv1a64:fixture-make-example".to_owned(),
            parameters: Vec::new(),
            physical_symbols: BTreeMap::from([
                ("single".to_owned(), "b2MakeExample".to_owned()),
                ("double".to_owned(), "b2MakeExample_double".to_owned()),
            ]),
            availability: vec!["always".to_owned()],
            header: "box2d.h".to_owned(),
            line: 1,
        }];
        let binding = AbiBindingIndex::from_path(
            "bindings-single",
            Precision::Single,
            ArtifactTarget::Universal,
            ArtifactProvider::Universal,
            &binding_path,
        )
        .expect("route binding index");
        fixture
            .binding_indexes
            .insert(binding.artifact.clone(), binding);

        let indexes = load_rust_indexes(
            &fixture.paths(),
            &fixture.binding_routes,
            &fixture.binding_indexes,
            Some(&fixture.inventory),
        )
        .expect("route-specific return type should drive the Rust index");
        let index = indexes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source route index");
        assert!(index.path_has_safe_ffi_type_witness_from(
            "boxdd::make_example",
            "boxdd::Example",
            "boxdd_sys::ffi::b2Example",
        ));
        assert!(index.path_has_safe_ffi_field_witness_from(
            "boxdd::make_example",
            "boxdd::Example::count",
            "boxdd_sys::ffi::b2Example",
            "count",
        ));
    }

    #[test]
    fn every_routed_binding_must_cover_each_field_and_physical_function() {
        let mut missing_function = ContractFixture::create();
        missing_function.enable_abi_capabilities();
        let double_path = missing_function
            .root
            .join("boxdd-sys/src/bindings_double.rs");
        fs::write(
            &double_path,
            r#"
                pub struct b2Example { pub count: i32 }
                pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
            "#,
        )
        .expect("double binding fixture");
        let double_binding = AbiBindingIndex::from_path(
            "bindings-double",
            Precision::Double,
            ArtifactTarget::Universal,
            ArtifactProvider::Universal,
            &double_path,
        )
        .expect("double binding index");
        missing_function
            .binding_indexes
            .insert(double_binding.artifact.clone(), double_binding);
        let double_route = AbiBindingRoute {
            mode: "double".to_owned(),
            provider: "source".to_owned(),
            artifact: "bindings-double".to_owned(),
            rust_target: RustTarget::X86_64UnknownLinuxGnu,
            rust_features: vec!["double-precision".to_owned()],
        };
        missing_function.binding_routes.insert(
            (double_route.mode.clone(), double_route.provider.clone()),
            double_route,
        );
        let double_rust = missing_function
            .rust_indexes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("single Rust index")
            .clone();
        missing_function
            .rust_indexes
            .insert(("double".to_owned(), "source".to_owned()), double_rust);
        missing_function.contract.functions[0]
            .modes
            .push("double".to_owned());
        missing_function.contract.functions[0]
            .link_symbols
            .insert("double".to_owned(), "b2Body_SetTransform".to_owned());
        let previous = missing_function.contract.abi.clone();
        let mut regenerated = map_inventory(
            &missing_function.inventory,
            &missing_function.binding_routes,
            &missing_function.binding_indexes,
        )
        .expect("all ABI types and fields exist in the second binding");
        preserve_reviewed_exposure(&previous, &mut regenerated);
        missing_function.contract.abi = regenerated;
        let error = missing_function
            .validate()
            .expect_err("the second binding is missing the physical function");
        assert!(
            error
                .to_string()
                .contains("binding artifact `bindings-double` is missing active C function")
        );

        let mut missing_field = ContractFixture::create();
        missing_field.enable_abi_capabilities();
        let double_path = missing_field
            .root
            .join("boxdd-sys/src/bindings_double_missing_field.rs");
        fs::write(
            &double_path,
            r#"
                pub struct b2Example { pub other: i32 }
                pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
                unsafe extern "C" { pub fn b2Body_SetTransform(); }
            "#,
        )
        .expect("missing-field binding fixture");
        let double_binding = AbiBindingIndex::from_path(
            "bindings-double-missing-field",
            Precision::Double,
            ArtifactTarget::Universal,
            ArtifactProvider::Universal,
            &double_path,
        )
        .expect("missing-field binding index");
        missing_field
            .binding_indexes
            .insert(double_binding.artifact.clone(), double_binding);
        let double_route = AbiBindingRoute {
            mode: "double".to_owned(),
            provider: "source".to_owned(),
            artifact: "bindings-double-missing-field".to_owned(),
            rust_target: RustTarget::X86_64UnknownLinuxGnu,
            rust_features: vec!["double-precision".to_owned()],
        };
        missing_field.binding_routes.insert(
            (double_route.mode.clone(), double_route.provider.clone()),
            double_route,
        );
        let error = map_inventory(
            &missing_field.inventory,
            &missing_field.binding_routes,
            &missing_field.binding_indexes,
        )
        .expect_err("a field missing from any routed binding must fail mapping");
        assert!(
            error
                .to_string()
                .contains("has no unique generated Rust access chain")
        );
    }

    #[test]
    fn comment_only_c_symbol_mention_does_not_satisfy_safe_path_reachability() {
        let mut fixture = ContractFixture::create();
        fs::write(
            fixture.root.join("boxdd/src/lib.rs"),
            "pub fn set_transform() { /* boxdd_sys::ffi::b2Body_SetTransform() */ }\n",
        )
        .expect("comment-only source");
        fixture.rust_indexes.insert(
            ("single".to_owned(), "source".to_owned()),
            index_boxdd(&fixture.root).expect("comment-only index"),
        );

        let error = fixture
            .validate()
            .expect_err("comment-only FFI mention must fail");

        assert!(
            error
                .to_string()
                .contains("does not reach physical symbol `b2Body_SetTransform`")
        );
    }

    #[test]
    fn function_provider_override_rejects_native_provider() {
        let mut fixture = ContractFixture::create();
        let source_route = fixture
            .binding_routes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source route")
            .clone();
        let mut system_route = source_route;
        system_route.provider = "system-static".to_owned();
        fixture.binding_routes.insert(
            (system_route.mode.clone(), system_route.provider.clone()),
            system_route,
        );
        let source_index = fixture
            .rust_indexes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source Rust index")
            .clone();
        fixture.rust_indexes.insert(
            ("single".to_owned(), "system-static".to_owned()),
            source_index,
        );
        let function = fixture
            .contract
            .functions
            .first_mut()
            .expect("fixture function");
        function.providers.push("system-static".to_owned());
        function.provider_overrides = vec![FunctionProviderOverride {
            providers: vec!["system-static".to_owned()],
            classification: Classification::Raw,
            rust_paths: vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()],
            rationale: "The system provider is deliberately forged as a narrower raw route."
                .to_owned(),
            evidence: Vec::new(),
        }];

        let error = fixture
            .validate()
            .expect_err("native provider overrides must fail closed");

        assert!(
            error
                .to_string()
                .contains("may only use conservative provider overrides for WASM providers")
        );
    }

    #[test]
    fn function_provider_override_cannot_hide_proven_wasm_safe_path() {
        let mut fixture = ContractFixture::create();
        let source_route = fixture
            .binding_routes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source route")
            .clone();
        let mut wasm_route = source_route;
        wasm_route.provider = "wasm-runtime".to_owned();
        wasm_route.rust_target = RustTarget::Wasm32UnknownUnknown;
        fixture.binding_routes.insert(
            (wasm_route.mode.clone(), wasm_route.provider.clone()),
            wasm_route,
        );
        let source_index = fixture
            .rust_indexes
            .get(&("single".to_owned(), "source".to_owned()))
            .expect("source Rust index")
            .clone();
        fixture.rust_indexes.insert(
            ("single".to_owned(), "wasm-runtime".to_owned()),
            source_index,
        );
        let function = fixture
            .contract
            .functions
            .first_mut()
            .expect("fixture function");
        function.providers.push("wasm-runtime".to_owned());
        function.provider_overrides = vec![FunctionProviderOverride {
            providers: vec!["wasm-runtime".to_owned()],
            classification: Classification::Raw,
            rust_paths: vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()],
            rationale: "The browser route is deliberately forged as raw despite its Safe path."
                .to_owned(),
            evidence: Vec::new(),
        }];

        let error = fixture
            .validate()
            .expect_err("a proven WASM Safe path cannot be hidden by an override");

        assert!(error.to_string().contains(
            "provider override unnecessarily hides a proven Safe path at route `single/wasm-runtime`"
        ));
    }

    #[test]
    fn coverage_report_rendering_is_deterministic_and_manual_edits_drift() {
        let fixture = ContractFixture::create();
        let first = render_report(&fixture.contract);
        let second = render_report(&fixture.contract);
        assert_eq!(first, second);
        let edited = first.replacen("| `safe` | 1 |", "| `safe` | 99 |", 1);
        assert_ne!(normalize_newlines(&edited), normalize_newlines(&second));
    }

    #[test]
    fn coverage_report_renders_effective_function_and_abi_provider_counts() {
        let mut fixture = ContractFixture::create();
        let function = fixture
            .contract
            .functions
            .first_mut()
            .expect("fixture function");
        function.providers.push("wasm-runtime".to_owned());
        function.provider_overrides = vec![FunctionProviderOverride {
            providers: vec!["wasm-runtime".to_owned()],
            classification: Classification::Raw,
            rust_paths: vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()],
            rationale: "The Safe adapter is cfg-disabled on the browser WASM provider route."
                .to_owned(),
            evidence: vec!["api-classification-wasm".to_owned()],
        }];
        fixture.contract.abi = AbiContract {
            policies: vec![
                crate::abi_contract::AbiCapabilityPolicy {
                    id: "safe-abi-adapter".to_owned(),
                    classification: Classification::Safe,
                    rationale:
                        "The reviewed adapter safely represents this aggregate on native routes."
                            .to_owned(),
                    modes: vec!["single".to_owned()],
                    providers: vec!["source".to_owned(), "wasm-runtime".to_owned()],
                    availability: vec!["always".to_owned()],
                    evidence: Vec::new(),
                },
                crate::abi_contract::AbiCapabilityPolicy {
                    id: crate::abi_contract::ABI_POLICY_ID.to_owned(),
                    classification: Classification::Raw,
                    rationale: "The generated binding retains the exact raw ABI representation."
                        .to_owned(),
                    modes: vec!["single".to_owned()],
                    providers: vec!["source".to_owned(), "wasm-runtime".to_owned()],
                    availability: vec!["always".to_owned()],
                    evidence: Vec::new(),
                },
            ],
            structs: vec![crate::abi_contract::AbiStructContract {
                name: "b2Example".to_owned(),
                fingerprint: "fnv1a64:fixture".to_owned(),
                header: "box2d.h".to_owned(),
                rationale: "The reviewed aggregate has a canonical crate-owned representation."
                    .to_owned(),
                policy: "safe-abi-adapter".to_owned(),
                safe_paths: vec!["boxdd::Example".to_owned()],
                safe_witnesses: Vec::new(),
                provider_overrides: vec![crate::abi_contract::AbiProviderOverride {
                    providers: vec!["wasm-runtime".to_owned()],
                    rationale: "The crate-owned representation is cfg-disabled on browser WASM."
                        .to_owned(),
                    policy: crate::abi_contract::ABI_POLICY_ID.to_owned(),
                    safe_paths: Vec::new(),
                    safe_witnesses: Vec::new(),
                }],
                raw_mappings: Vec::new(),
                fields: Vec::new(),
            }],
            callbacks: Vec::new(),
        };

        let report = render_report(&fixture.contract);
        assert!(report.contains("## Safe-call Witness Policy"));
        assert!(report.contains("| `single` | `source` | 1 | 0 | 0 | 0 | 1 |"));
        assert!(report.contains("| `single` | `wasm-runtime` | 0 | 1 | 0 | 0 | 1 |"));
        assert!(report.contains("| `single` | `source` | Structs | 1 | 0 | 0 | 0 | 1 |"));
        assert!(report.contains("| `single` | `wasm-runtime` | Structs | 0 | 1 | 0 | 0 | 1 |"));
        assert!(report.contains("provider identity and execution or compilation support"));
    }
}
