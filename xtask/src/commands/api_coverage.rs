use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    abi_contract::{
        ABI_BINDING_EVIDENCE_ID, ABI_HEADER_EVIDENCE_ID, ABI_VALIDATOR_EVIDENCE_ID,
        AbiBindingIndex, AbiBindingIndexes, AbiBindingRoute, AbiBindingRoutes, AbiContract,
        AbiFunctionSymbols, AbiRustIndexes, AbiValidationContext, map_inventory,
        preserve_reviewed_exposure, validate as validate_abi,
    },
    c_api::{CApiInventory, parse_headers},
    commands::upstream_sync::{
        ArtifactKind, ArtifactTarget, ManagedArtifactWrite, RustTarget, UpdateLock,
        UpstreamManifest, expanded_binding_route_features, install_managed_artifact_writes_locked,
        reviewed_recording_operations_source, validate_repository,
    },
    commands::{UpdateMode, parse_update_mode},
    config::{API_CONTRACT_SCHEMA, read_toml, render_toml},
    paths::WorkspacePaths,
    recording_ops::parse as parse_recording_ops,
    recording_wire::{
        RecordingWireContract, generate_wire_contract, reviewed_sources_aggregate_blake3,
        validate_wire_contract,
    },
    rust_index::{
        RustIndex, RustIndexCoordinate, TestEvidenceIndex, discover_test_evidence_items,
        index_boxdd_routes, index_test_evidence_for_gate_at_coordinate,
    },
    sys_abi_index::index_bindings,
};

const AVAILABILITY: &[&str] = &[
    "always",
    "debug-profile",
    "assertions-enabled",
    "validation-enabled",
];
const API_CLASSIFICATION_EVIDENCE_ID: &str = "api-classification-validator";

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
    #[serde(default)]
    pub runtime_witnesses: Vec<RuntimeCallWitness>,
    #[serde(default)]
    pub classification_witnesses: Vec<FunctionClassificationWitness>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestEvidenceRole {
    #[default]
    Runtime,
    FunctionClassificationValidator,
    AbiHeaderInventory,
    AbiBindingAst,
    AbiContractValidator,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCallWitness {
    pub function: String,
    pub rust_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionClassificationWitness {
    pub function: String,
    pub classification: Classification,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingClass {
    PureWorldless,
    ReadOnly,
    LoggedMutation,
    LoggedQuery,
    RecordingLifecycle,
    SnapshotLifecycle,
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
    pub recording: Option<RecordingCoverage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApiContract {
    pub schema_version: u32,
    pub upstream_sha: String,
    pub migration_baseline: CoverageCounts,
    pub classification_changes: Vec<ClassificationChange>,
    pub evidence: Vec<TestEvidence>,
    pub functions: Vec<FunctionContract>,
    pub abi: AbiContract,
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
    if matches!(args, [argument] if argument == "--refresh-abi") {
        return refresh_abi_contract(paths);
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
  cargo run -p xtask -- api-coverage --audit-evidence
  cargo run -p xtask -- api-coverage --audit-canonical-paths
"
    );
}

pub fn check(paths: &WorkspacePaths) -> Result<()> {
    let validated = load_validated_coverage(paths)?;
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
    validate_repository(paths, &manifest, false)?;
    let outputs = render_generated_outputs(paths)?;
    let writes = [
        ManagedArtifactWrite::active("recording-wire", outputs.recording_wire),
        ManagedArtifactWrite::active("api-coverage-report", outputs.report),
    ];
    install_managed_artifact_writes_locked(paths, &writes, Some(&manifest_baseline), || {
        validate_managed_repository_and_api(paths)
    })?;
    println!("wrote generated recording contract and API coverage report");
    Ok(())
}

pub(crate) struct GeneratedApiCoverageOutputs {
    pub(crate) recording_wire: Vec<u8>,
    pub(crate) report: Vec<u8>,
}

pub(crate) fn render_generated_outputs(
    paths: &WorkspacePaths,
) -> Result<GeneratedApiCoverageOutputs> {
    let validated = load_validated_coverage(paths)?;
    let wire = generate_wire_contract(
        &validated.manifest.recording_revision,
        &validated.recording_operations,
        &validated.recording_source_git_blobs,
        &validated.recording_sources_aggregate,
    )?;
    Ok(GeneratedApiCoverageOutputs {
        recording_wire: render_toml(&wire)?.into_bytes(),
        report: validated.report.into_bytes(),
    })
}

struct ValidatedCoverage {
    manifest: UpstreamManifest,
    contract: ApiContract,
    recording_operations: Vec<crate::recording_ops::RecordingOp>,
    recording_source_git_blobs: BTreeMap<String, String>,
    recording_sources_aggregate: String,
    report: String,
}

fn load_validated_coverage(paths: &WorkspacePaths) -> Result<ValidatedCoverage> {
    let manifest = UpstreamManifest::load(paths)?;
    let api_contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let rust_indexes = load_rust_indexes(paths, &binding_routes, &binding_indexes)?;
    let contract: ApiContract = read_toml(&api_contract_path)?;
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

#[allow(
    clippy::too_many_arguments,
    reason = "the top-level validator composes independently indexed contract domains"
)]
pub fn validate_contract(
    paths: &WorkspacePaths,
    contract: &ApiContract,
    inventory: &CApiInventory,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
    expected_active_revision: &str,
    recording_operations: &[crate::recording_ops::RecordingOp],
) -> Result<()> {
    let mut errors = Vec::new();
    if contract.schema_version != API_CONTRACT_SCHEMA {
        errors.push(format!(
            "API contract schema {} does not match supported schema {API_CONTRACT_SCHEMA}",
            contract.schema_version
        ));
    }
    if contract.upstream_sha != expected_active_revision {
        errors.push(format!(
            "API contract upstream {} does not match active revision {expected_active_revision}",
            contract.upstream_sha,
        ));
    }

    let inventory_by_name = inventory
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let mut evidence_by_id = BTreeMap::new();
    let mut evidence_indexes = BTreeMap::new();
    for evidence in &contract.evidence {
        if evidence_by_id
            .insert(evidence.id.as_str(), evidence)
            .is_some()
        {
            errors.push(format!("duplicate evidence id `{}`", evidence.id));
        }
        match index_evidence_across_routes(paths, evidence, rust_indexes, binding_routes) {
            Ok(actual) if aggregate_evidence_fingerprint(&actual) != evidence.fingerprint => {
                let fingerprint = aggregate_evidence_fingerprint(&actual);
                errors.push(format!(
                    "evidence `{}` fingerprint drifted: reviewed `{}`, normalized AST `{fingerprint}`",
                    evidence.id, evidence.fingerprint
                ));
            }
            Ok(actual) => {
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
    let route_coordinates = binding_routes.keys().cloned().collect::<BTreeSet<_>>();
    let route_modes = binding_routes
        .keys()
        .map(|(mode, _)| mode.as_str())
        .collect::<BTreeSet<_>>();
    let route_providers = binding_routes
        .keys()
        .map(|(_, provider)| provider.as_str())
        .collect::<BTreeSet<_>>();
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
        if function.area.trim().is_empty() {
            errors.push(format!("`{}` has no explicit area", function.logical_name));
        }
        if !has_rationale(&function.rationale) {
            errors.push(format!(
                "`{}` needs a specific rationale",
                function.logical_name
            ));
        }
        if function.evidence.is_empty() {
            errors.push(format!("`{}` has no test evidence", function.logical_name));
        }
        for evidence in &function.evidence {
            let expected_role = match function.classification {
                Classification::Safe => TestEvidenceRole::Runtime,
                Classification::Raw | Classification::Omitted | Classification::Deferred => {
                    TestEvidenceRole::FunctionClassificationValidator
                }
            };
            match evidence_by_id.get(evidence.as_str()) {
                None => errors.push(format!(
                    "`{}` references unknown evidence `{evidence}`",
                    function.logical_name
                )),
                Some(row) if row.role != expected_role => errors.push(format!(
                    "{} function `{}` references evidence `{evidence}` with role {:?}, expected {:?}",
                    function.classification.as_str(),
                    function.logical_name,
                    row.role,
                    expected_role
                )),
                Some(_) => {}
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
        if function.classification != Classification::Safe
            && function.exposure != FunctionExposureKind::Callable
        {
            errors.push(format!(
                "{} function `{}` cannot claim the {:?} Safe Rust exposure kind",
                function.classification.as_str(),
                function.logical_name,
                function.exposure
            ));
        }
        match function.classification {
            Classification::Safe => {
                if function.rust_paths.is_empty() {
                    errors.push(format!(
                        "safe function `{}` has no canonical Rust path",
                        function.logical_name
                    ));
                }
                for coordinate in &route_coordinates {
                    let Some(index) = rust_indexes.get(coordinate) else {
                        errors.push(format!(
                            "safe function `{}` has no Rust index for route `{}/{}`",
                            function.logical_name, coordinate.0, coordinate.1
                        ));
                        continue;
                    };
                    let Some(symbol) = declaration.physical_symbols.get(&coordinate.0) else {
                        continue;
                    };
                    for path in &function.rust_paths {
                        if !safe_exposure_path_exists(index, function.exposure, path) {
                            errors.push(format!(
                                "safe function `{}` references nonexistent {} `{path}` at route `{}/{}`",
                                function.logical_name,
                                function.exposure.path_kind(),
                                coordinate.0,
                                coordinate.1
                            ));
                        } else if !index.path_reaches_symbol(path, symbol) {
                            errors.push(format!(
                                "{} `{path}` does not reach physical symbol `{symbol}` through the Rust AST call graph at route `{}/{}`",
                                function.exposure.path_kind(),
                                coordinate.0, coordinate.1
                            ));
                        }
                    }
                }
            }
            Classification::Raw => {
                let expected_paths = route_modes
                    .iter()
                    .filter_map(|mode| declaration.physical_symbols.get(*mode))
                    .map(|symbol| format!("boxdd_sys::ffi::{symbol}"))
                    .collect::<BTreeSet<_>>();
                let actual_paths = function.rust_paths.iter().cloned().collect::<BTreeSet<_>>();
                if actual_paths != expected_paths || function.rust_paths.is_empty() {
                    errors.push(format!(
                        "raw function `{}` must name exactly its header-derived boxdd_sys::ffi physical paths",
                        function.logical_name,
                    ));
                }
            }
            Classification::Omitted | Classification::Deferred => {
                if !function.rust_paths.is_empty() {
                    errors.push(format!(
                        "{} function `{}` cannot claim a Rust path",
                        function.classification.as_str(),
                        function.logical_name
                    ));
                }
            }
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

    validate_typed_evidence(contract, &evidence_by_id, &evidence_indexes, &mut errors);

    validate_migration(contract, &mut errors);
    let evidence_ids = evidence_by_id
        .keys()
        .map(|id| (*id).to_owned())
        .collect::<BTreeSet<_>>();
    let function_symbols = abi_function_symbols(inventory, binding_routes);
    let abi_context = AbiValidationContext::new(
        inventory,
        binding_routes,
        binding_indexes,
        &function_symbols,
        rust_indexes,
        &evidence_ids,
    );
    validate_abi(&contract.abi, &abi_context, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn index_evidence_across_routes(
    paths: &WorkspacePaths,
    evidence: &TestEvidence,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<BTreeMap<(String, String), TestEvidenceIndex>> {
    let mut indexed_routes = BTreeMap::new();
    for (coordinate, rust_index) in rust_indexes {
        let route = binding_routes.get(coordinate).ok_or_else(|| {
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
        indexed_routes.insert(coordinate.clone(), indexed);
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

fn aggregate_evidence_fingerprint(
    indexed_routes: &BTreeMap<(String, String), TestEvidenceIndex>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"boxdd-test-evidence-route-fingerprint-v1\0");
    for ((mode, provider), index) in indexed_routes {
        for component in [mode.as_str(), provider.as_str(), index.fingerprint.as_str()] {
            hasher.update(&(component.len() as u64).to_le_bytes());
            hasher.update(component.as_bytes());
        }
    }
    format!("blake3-routes-v1:{}", hasher.finalize().to_hex())
}

fn validate_evidence_role(
    evidence: &TestEvidence,
    route_indexes: Option<&BTreeMap<(String, String), TestEvidenceIndex>>,
    errors: &mut Vec<String>,
) {
    let (expected, required_local_path) = match evidence.role {
        TestEvidenceRole::Runtime => {
            if evidence.package != "boxdd" || !evidence.file.starts_with("boxdd/tests/") {
                errors.push(format!(
                    "runtime evidence `{}` must be an executable boxdd integration test",
                    evidence.id
                ));
            }
            if !evidence.classification_witnesses.is_empty() {
                errors.push(format!(
                    "runtime evidence `{}` cannot contain classification witnesses",
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
                "parser_indexes_struct_fields_and_callbacks",
            ),
            "parse_header",
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
    if (
        evidence.id.as_str(),
        evidence.file.as_str(),
        evidence.item.as_str(),
        evidence.package.as_str(),
        evidence.gate.as_str(),
    ) != (expected.0, expected.1, expected.2, "xtask", "nextest")
    {
        errors.push(format!(
            "ABI evidence role {:?} must point to the reviewed production validator `{}` in `{}`",
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
    if !evidence.runtime_witnesses.is_empty() {
        errors.push(format!(
            "ABI evidence `{}` cannot contain runtime call witnesses",
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
            .runtime_witnesses
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
        if evidence.role == TestEvidenceRole::Runtime && evidence.runtime_witnesses.is_empty() {
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
        for witness in &evidence.runtime_witnesses {
            let Some(function) = functions.get(witness.function.as_str()) else {
                errors.push(format!(
                    "evidence `{}` witnesses unknown function `{}`",
                    evidence.id, witness.function
                ));
                continue;
            };
            if evidence.role != TestEvidenceRole::Runtime {
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
    let rust_indexes = load_rust_indexes(paths, &binding_routes, &binding_indexes)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let mut contract: ApiContract = read_toml(&contract_path)?;
    reconcile_functions(
        &mut contract,
        &inventory,
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
        "runtime evidence audit: {} proven, {} gaps, {} Safe functions",
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
        .filter(|evidence| evidence.role == TestEvidenceRole::Runtime)
    {
        for witness in &evidence.runtime_witnesses {
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
    let binding_indexes = load_binding_indexes(paths, &manifest)?;
    let binding_routes = load_binding_routes(&manifest)?;
    let rust_indexes = load_rust_indexes(paths, &binding_routes, &binding_indexes)?;
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

fn refresh_abi_contract(paths: &WorkspacePaths) -> Result<()> {
    let _lock = UpdateLock::acquire(paths.root())?;
    let manifest_baseline = fs::read(paths.upstream_manifest())
        .map_err(|source| Error::io(paths.upstream_manifest(), source))?;
    let manifest = UpstreamManifest::load(paths)?;
    if manifest.artifact_digests_initialized {
        validate_repository(paths, &manifest, false)?;
    }
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let contract = build_refreshed_contract(paths, &manifest, &recording_operations)?;
    let recording_source_git_blobs = manifest.recording_source_git_blobs();
    let recording_sources_aggregate =
        reviewed_sources_aggregate_blake3(&recording_source_git_blobs)?;
    let wire = generate_wire_contract(
        &manifest.recording_revision,
        &recording_operations,
        &recording_source_git_blobs,
        &recording_sources_aggregate,
    )?;
    let writes = [
        ManagedArtifactWrite::reviewed_active("api-contract", render_toml(&contract)?.into_bytes()),
        ManagedArtifactWrite::active("recording-wire", render_toml(&wire)?.into_bytes()),
        ManagedArtifactWrite::active("api-coverage-report", render_report(&contract).into_bytes()),
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

fn validate_managed_repository_and_api(paths: &WorkspacePaths) -> Result<()> {
    let manifest = UpstreamManifest::load(paths)?;
    validate_repository(paths, &manifest, false)?;
    check(paths)
}

pub(crate) fn render_refreshed_contract_candidate(paths: &WorkspacePaths) -> Result<Vec<u8>> {
    let manifest = UpstreamManifest::load(paths)?;
    let recording_operations =
        parse_recording_ops(&reviewed_recording_operations_source(paths, &manifest)?)?;
    let contract = build_refreshed_contract(paths, &manifest, &recording_operations)?;
    Ok(render_toml(&contract)?.into_bytes())
}

fn build_refreshed_contract(
    paths: &WorkspacePaths,
    manifest: &UpstreamManifest,
    recording_operations: &[crate::recording_ops::RecordingOp],
) -> Result<ApiContract> {
    let contract_path = manifest.artifact_path(paths.root(), ArtifactKind::ApiContract)?;
    let inventory = parse_headers(&paths.box2d_headers())?;
    let binding_indexes = load_binding_indexes(paths, manifest)?;
    let binding_routes = load_binding_routes(manifest)?;
    let rust_indexes = load_rust_indexes(paths, &binding_routes, &binding_indexes)?;
    let mut contract: ApiContract = read_toml(&contract_path)?;

    contract.schema_version = API_CONTRACT_SCHEMA;
    set_active_refresh_identity(&mut contract, &manifest.active_revision);
    reconcile_functions(
        &mut contract,
        &inventory,
        &binding_routes,
        &rust_indexes,
        recording_operations,
    );
    synchronize_classification_evidence(&mut contract);
    let _runtime_gaps =
        synchronize_runtime_evidence(paths, &mut contract, &rust_indexes, &binding_routes)?;
    let previous_abi = contract.abi.clone();
    let mut generated_abi = map_inventory(&inventory, &binding_routes, &binding_indexes)?;
    preserve_reviewed_exposure(&previous_abi, &mut generated_abi);
    contract.abi = generated_abi;
    refresh_evidence_metadata(paths, &mut contract, &rust_indexes, &binding_routes)?;
    validate_contract(
        paths,
        &contract,
        &inventory,
        &rust_indexes,
        &binding_routes,
        &binding_indexes,
        &manifest.active_revision,
        recording_operations,
    )?;
    Ok(contract)
}

fn reconcile_functions(
    contract: &mut ApiContract,
    inventory: &CApiInventory,
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
                && safe_function_review_matches_routes(
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
                recording: None,
            }
        };

        function.signature.clone_from(&declaration.signature);
        function.fingerprint.clone_from(&declaration.fingerprint);
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
    !function.rust_paths.is_empty()
        && binding_routes.keys().all(|coordinate| {
            let Some(index) = rust_indexes.get(coordinate) else {
                return false;
            };
            let Some(symbol) = declaration.physical_symbols.get(&coordinate.0) else {
                return false;
            };
            function.rust_paths.iter().all(|path| {
                safe_exposure_path_exists(index, function.exposure, path)
                    && index.path_reaches_symbol(path, symbol)
            })
        })
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
    let mut witnesses = contract
        .functions
        .iter_mut()
        .filter(|function| function.classification != Classification::Safe)
        .map(|function| {
            function.evidence = vec![API_CLASSIFICATION_EVIDENCE_ID.to_owned()];
            FunctionClassificationWitness {
                function: function.logical_name.clone(),
                classification: function.classification,
            }
        })
        .collect::<Vec<_>>();
    witnesses.sort();
    if witnesses.is_empty() {
        contract
            .evidence
            .retain(|evidence| evidence.id != API_CLASSIFICATION_EVIDENCE_ID);
        return;
    }
    let row = TestEvidence {
        id: API_CLASSIFICATION_EVIDENCE_ID.to_owned(),
        file: "xtask/src/commands/api_coverage.rs".to_owned(),
        item: "typed_function_classification_evidence_rejects_unrelated_subjects".to_owned(),
        package: "xtask".to_owned(),
        gate: "nextest".to_owned(),
        role: TestEvidenceRole::FunctionClassificationValidator,
        fingerprint: String::new(),
        runtime_witnesses: Vec::new(),
        classification_witnesses: witnesses,
    };
    if let Some(existing) = contract
        .evidence
        .iter_mut()
        .find(|evidence| evidence.id == API_CLASSIFICATION_EVIDENCE_ID)
    {
        let fingerprint = std::mem::take(&mut existing.fingerprint);
        *existing = TestEvidence { fingerprint, ..row };
    } else {
        contract.evidence.push(row);
    }
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
    let safe_functions = contract
        .functions
        .iter()
        .filter(|function| function.classification == Classification::Safe)
        .cloned()
        .collect::<Vec<_>>();
    let reviewed_ids = contract
        .evidence
        .iter()
        .filter(|evidence| evidence.role == TestEvidenceRole::Runtime)
        .map(|evidence| {
            (
                (evidence.file.clone(), evidence.item.clone()),
                evidence.id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates = Vec::<TestEvidence>::new();
    let mut matches = BTreeMap::<String, Vec<(usize, RuntimeCallWitness)>>::new();

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
            role: TestEvidenceRole::Runtime,
            fingerprint: String::new(),
            runtime_witnesses: Vec::new(),
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
                    RuntimeCallWitness {
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
    let mut selected = BTreeMap::<String, (usize, RuntimeCallWitness)>::new();
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

    let mut witnesses_by_candidate = BTreeMap::<usize, Vec<RuntimeCallWitness>>::new();
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
            evidence.runtime_witnesses = witnesses;
            evidence
        })
        .collect::<Vec<_>>();
    runtime_rows.sort_by(|left, right| left.id.cmp(&right.id));
    contract
        .evidence
        .retain(|evidence| evidence.role != TestEvidenceRole::Runtime);
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

fn refresh_evidence_metadata(
    paths: &WorkspacePaths,
    contract: &mut ApiContract,
    rust_indexes: &AbiRustIndexes,
    binding_routes: &AbiBindingRoutes,
) -> Result<()> {
    let mut expected = contract
        .functions
        .iter()
        .flat_map(|function| function.evidence.iter().cloned())
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
        let index = index_bindings(&paths.root().join(&artifact.path))?;
        let binding = AbiBindingIndex::new(
            artifact.name.clone(),
            precision,
            artifact.target,
            artifact.provider,
            index,
        );
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

fn load_rust_indexes(
    paths: &WorkspacePaths,
    routes: &AbiBindingRoutes,
    binding_indexes: &AbiBindingIndexes,
) -> Result<AbiRustIndexes> {
    let mut coordinates = BTreeMap::new();
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
    }
    index_boxdd_routes(paths.root(), &coordinates)
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
    output.push_str("## Summary\n\n| Status | Count |\n|---|---:|\n");
    writeln!(output, "| `safe` | {} |", counts.safe).expect("write to string");
    writeln!(output, "| `raw` | {} |", counts.raw).expect("write to string");
    writeln!(output, "| `omitted` | {} |", counts.omitted).expect("write to string");
    writeln!(output, "| `deferred` | {} |", counts.deferred).expect("write to string");
    writeln!(output, "| Total | {} |\n", counts.total).expect("write to string");
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
        for field in &structure.fields {
            render_non_safe_abi_row(
                &mut output,
                &format!("{}::{}", structure.name, field.name),
                &field.policy,
                &field.rationale,
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
        c_api::{CallbackDecl, FieldDecl, OverlayDecl, StructDecl},
        commands::upstream_sync::{ArtifactProvider, Precision},
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
        assert!(aggregate.starts_with("blake3-routes-v1:"));
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
                    pub struct b2Example { pub count: i32, pub other: i32 }
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
            let sys_abi_index =
                index_bindings(&root.join("boxdd-sys/src/bindings_pregenerated.rs"))
                    .expect("sys ABI index");
            let binding = AbiBindingIndex::new(
                "bindings-single",
                Precision::Single,
                ArtifactTarget::Universal,
                ArtifactProvider::Universal,
                sys_abi_index,
            );
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
            let contract = ApiContract {
                schema_version: API_CONTRACT_SCHEMA,
                upstream_sha: active_revision.clone(),
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
                    role: TestEvidenceRole::Runtime,
                    fingerprint: evidence_fingerprint,
                    runtime_witnesses: vec![RuntimeCallWitness {
                        function: "b2Body_SetTransform".to_owned(),
                        rust_path: "boxdd::set_transform".to_owned(),
                    }],
                    classification_witnesses: Vec::new(),
                }],
                functions: vec![FunctionContract {
                    logical_name: "b2Body_SetTransform".to_owned(),
                    signature,
                    fingerprint,
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

        fn paths(&self) -> WorkspacePaths {
            WorkspacePaths::new(&self.root)
        }

        fn validate(&self) -> Result<()> {
            validate_contract(
                &self.paths(),
                &self.contract,
                &self.inventory,
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
                    "parser_indexes_struct_fields_and_callbacks",
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
            for (id, file, item, role) in evidence {
                let mut row = TestEvidence {
                    id: id.to_owned(),
                    file: file.to_owned(),
                    item: item.to_owned(),
                    package: "xtask".to_owned(),
                    gate: "nextest".to_owned(),
                    role,
                    fingerprint: String::new(),
                    runtime_witnesses: Vec::new(),
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
                runtime_witnesses: Vec::new(),
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
                "fn parse_header() {}\n#[test]\nfn parser_indexes_struct_fields_and_callbacks() { parse_header(); }\n",
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
            upstream_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
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

    #[test]
    fn function_reconcile_preserves_exact_rows_and_fails_new_or_changed_rows_to_raw() {
        let exact = ContractFixture::create();
        let mut exact_contract = exact.contract.clone();
        reconcile_functions(
            &mut exact_contract,
            &exact.inventory,
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
    fn evidence_fingerprints_and_runtime_witnesses_fail_closed() {
        let mut drifted = ContractFixture::create();
        drifted.contract.evidence[0].fingerprint =
            "blake3:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
        let error = drifted
            .validate()
            .expect_err("an edited evidence test must require review");
        assert!(error.to_string().contains("fingerprint drifted"));

        let mut missing = ContractFixture::create();
        missing.contract.evidence[0].runtime_witnesses.clear();
        let error = missing
            .validate()
            .expect_err("a missing runtime witness must fail");
        assert!(
            error
                .to_string()
                .contains("has no exact executable runtime witness")
        );

        let mut extra = ContractFixture::create();
        extra.contract.evidence[0].runtime_witnesses = vec![RuntimeCallWitness {
            function: "b2Other".to_owned(),
            rust_path: "boxdd::set_transform".to_owned(),
        }];
        let error = extra
            .validate()
            .expect_err("an unknown runtime witness must fail");
        assert!(
            error
                .to_string()
                .contains("witnesses unknown function `b2Other`")
        );

        let mut duplicate = ContractFixture::create();
        duplicate.contract.evidence[0]
            .runtime_witnesses
            .push(RuntimeCallWitness {
                function: "b2Body_SetTransform".to_owned(),
                rust_path: "boxdd::set_transform".to_owned(),
            });
        let error = duplicate
            .validate()
            .expect_err("a repeated runtime witness must fail");
        assert!(
            error
                .to_string()
                .contains("runtime witnesses must be unique")
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
        function.rust_paths = vec!["boxdd_sys::ffi::b2Body_SetTransform".to_owned()];
        function.evidence = vec![API_CLASSIFICATION_EVIDENCE_ID.to_owned()];
        function.recording = None;
        fixture.contract.classification_changes = vec![ClassificationChange {
            logical_name: "b2Body_SetTransform".to_owned(),
            from: Classification::Safe,
            to: Classification::Raw,
            unit: "typed-evidence-test".to_owned(),
            rationale:
                "The fixture deliberately exercises the conservative raw classification gate."
                    .to_owned(),
        }];

        validate_contract(
            &fixture.paths(),
            &fixture.contract,
            &fixture.inventory,
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
            .index = index_bindings(&bindings).expect("binding index");
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
    fn abi_capability_mapping_rejects_deleted_forged_and_unknown_references() {
        let mut fixture = ContractFixture::create();
        fixture.enable_abi_capabilities();
        validate_contract(
            &fixture.paths(),
            &fixture.contract,
            &fixture.inventory,
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
                .contains("expected canonical generated path")
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
            (ABI_HEADER_EVIDENCE_ID, "parse_header"),
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
    fn abi_capability_mapping_resolves_aliases_and_anonymous_union_access_chains() {
        let mut fixture = ContractFixture::create();
        fixture.inventory.structs = vec![
            StructDecl {
                name: "b2Pos".to_owned(),
                fingerprint: "fnv1a64:alias".to_owned(),
                fields: vec![FieldDecl {
                    name: "x".to_owned(),
                    signature: "double x".to_owned(),
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
            .index = index_bindings(&bindings).expect("binding index");

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
            .index = index_bindings(&bindings).expect("binding index");

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
        fixture.contract.evidence[0].runtime_witnesses[0].rust_path = "boxdd::overlap".to_owned();
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
            kind: crate::abi_contract::AbiSafeWitnessKind::Accessor,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: Some("count".to_owned()),
            native_symbols: Vec::new(),
        }];
        previous.structs[0].fields[0].raw_mappings[0].steps[0].field = "other".to_owned();

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
            kind: crate::abi_contract::AbiSafeWitnessKind::PublicType,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: None,
            native_symbols: Vec::new(),
        }];
        previous.structs[0].fields[0].policy = "safe-reviewed".to_owned();
        previous.structs[0].fields[0].safe_paths = vec!["boxdd::Example::from_raw".to_owned()];
        previous.structs[0].fields[0].safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::Example::from_raw".to_owned(),
            kind: crate::abi_contract::AbiSafeWitnessKind::Accessor,
            raw_type: "boxdd_sys::ffi::b2Example".to_owned(),
            raw_field: Some("count".to_owned()),
            native_symbols: Vec::new(),
        }];
        previous.callbacks[0].policy = "safe-reviewed".to_owned();
        previous.callbacks[0].safe_paths = vec!["boxdd::set_transform".to_owned()];
        previous.callbacks[0].safe_witnesses = vec![crate::abi_contract::AbiSafeWitness {
            path: "boxdd::set_transform".to_owned(),
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
                .contains("fingerprint or header")
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
                .contains("signature, or header")
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
    fn every_routed_binding_must_cover_each_field_and_physical_function() {
        let mut missing_function = ContractFixture::create();
        missing_function.enable_abi_capabilities();
        let double_path = missing_function
            .root
            .join("boxdd-sys/src/bindings_double.rs");
        fs::write(
            &double_path,
            r#"
                pub struct b2Example { pub count: i32, pub other: i32 }
                pub type b2ExampleCallback = Option<unsafe extern "C" fn()>;
            "#,
        )
        .expect("double binding fixture");
        let double_binding = AbiBindingIndex::new(
            "bindings-double",
            Precision::Double,
            ArtifactTarget::Universal,
            ArtifactProvider::Universal,
            index_bindings(&double_path).expect("double binding index"),
        );
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
        let double_binding = AbiBindingIndex::new(
            "bindings-double-missing-field",
            Precision::Double,
            ArtifactTarget::Universal,
            ArtifactProvider::Universal,
            index_bindings(&double_path).expect("missing-field binding index"),
        );
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
    fn coverage_report_rendering_is_deterministic_and_manual_edits_drift() {
        let fixture = ContractFixture::create();
        let first = render_report(&fixture.contract);
        let second = render_report(&fixture.contract);
        assert_eq!(first, second);
        let edited = first.replacen("| `safe` | 1 |", "| `safe` | 99 |", 1);
        assert_ne!(normalize_newlines(&edited), normalize_newlines(&second));
    }
}
