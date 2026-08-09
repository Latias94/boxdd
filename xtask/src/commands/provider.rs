use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use wasmparser::{Parser, Payload, TypeRef};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

use crate::{
    Error, Result,
    build_support::VerifiedFileSnapshot,
    config::{render_toml, write_atomic_bytes},
    emscripten_sdk::EmscriptenTools,
    paths::WorkspacePaths,
    provider_archive::{private_abi_hash, snapshot_layout_hash},
    provider_catalog::ProviderCapability,
    provider_manifest::{
        ADAPTER_ABI_VERSION, MAX_PROVIDER_BINDINGS_BYTES, RECORDING_CONTRACT_BLAKE3,
        REQUIRED_RUNTIME_IDENTITY_IMPORTS, REQUIRED_WASM_PROVIDER_ADAPTER_EXPORTS,
    },
    source_overlay::{
        ADAPTER_SOURCE_PATHS, EffectiveSourceIdentity, MaterializedEffectiveSources,
        adapter_source_sha256, effective_source_identity, materialize_effective_box2d_sources,
    },
    wasm_identity,
    wasm_provider_contract::{
        COMPILER_TARGET, ENDIANNESS, POINTER_WIDTH, PROVIDER_ABI, SIMD_MODE,
        WasmProviderExpectation, WasmProviderIdentity, contract_relative_path,
    },
    wasm_provider_gate,
    wasm_provider_memory::{
        INITIAL_MEMORY_BYTES, INITIAL_MEMORY_PAGES, MAXIMUM_MEMORY_BYTES, MAXIMUM_MEMORY_PAGES,
        PROVIDER_HEAP_LIMIT_BYTES, PROVIDER_STATIC_BASE_BYTES,
    },
};

use super::support::{
    BuildProfile, QualifiedCargo, WASM_TARGET, add_wasm_app_link_args, copy_file, ensure_file,
    replace_dir_under, run_command,
};
use super::upstream_sync::{
    ArtifactKind, Precision as ManifestPrecision, UpdateLock, UpstreamManifest,
    require_provider_identity_topology, validate_repository,
};

pub(super) const PROVIDER_MODULE: &str = "box2d-sys-v2-single";
const PROVIDER_MODULE_DOUBLE: &str = "box2d-sys-v2-double";
pub(super) const PROVIDER_IDENTITY_SINGLE_ARTIFACT: &str = "wasm-provider-identity-single";
pub(super) const PROVIDER_IDENTITY_DOUBLE_ARTIFACT: &str = "wasm-provider-identity-double";
const PROVIDER_SMOKE_PACKAGE: &str = "boxdd-provider-smoke";
const PROVIDER_SMOKE_WASM: &str = "boxdd_provider_smoke.wasm";
const PROVIDER_RUNTIME_CONTRACT_FILE: &str = "provider-runtime-contract.mjs";
const MAX_IDENTITY_VALUES: usize = 4_096;
const MAX_CAPTURED_PROVIDER_INPUT_BYTES: u64 = MAX_PROVIDER_BINDINGS_BYTES;
const CAPTURED_PROVIDER_INPUTS_DIRECTORY: &str = "wasm-provider-captured-inputs";
const PROVIDER_RUNTIME_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../examples-wasm/provider-smoke/provider-runtime-contract.mjs"
));
const PROVIDER_HEAP_BOUNDARY_PROBE: &str = "providerHeapBoundaryProbe";
const PROVIDER_SMOKE_EXPORTS: &[&str] = &[
    "boxdd_provider_smoke",
    "boxdd_provider_drop_millimeters",
    "boxdd_provider_ray_hit_millimeters",
    "boxdd_provider_shape_cast_permyriad",
    "boxdd_provider_joint_error_millimeters",
    "boxdd_provider_box2d_byte_count",
    "boxdd_allocator_probe_push",
    "boxdd_allocator_probe_validate",
    "boxdd_allocator_aligned_probe_push",
    "boxdd_allocator_aligned_probe_validate",
    "boxdd_allocator_probe_reset",
];
const RUNTIME_EXPORTS: &[&str] = &[
    "boxdd_runtime_init",
    "boxdd_runtime_step",
    "boxdd_runtime_reset",
    "boxdd_runtime_body_count",
    "boxdd_runtime_body_shape",
    "boxdd_runtime_body_x_millimeters",
    "boxdd_runtime_body_y_millimeters",
    "boxdd_runtime_body_angle_milliradians",
    "boxdd_runtime_body_half_width_millimeters",
    "boxdd_runtime_body_half_height_millimeters",
    "boxdd_runtime_body_radius_millimeters",
];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProviderPrecision {
    Single,
    Double,
}

impl ProviderPrecision {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value {
            "single" => Ok(Self::Single),
            "double" => Ok(Self::Double),
            _ => Err(Error::Message(format!(
                "invalid WASM precision `{value}`; expected single or double"
            ))),
        }
    }

    pub(super) fn from_env() -> Result<Self> {
        Self::parse(&env::var("BOXDD_WASM_PRECISION").unwrap_or_else(|_| "single".to_owned()))
    }

    pub(crate) const fn module(self) -> &'static str {
        match self {
            Self::Single => PROVIDER_MODULE,
            Self::Double => PROVIDER_MODULE_DOUBLE,
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }

    pub(super) const fn cargo_feature(self) -> Option<&'static str> {
        match self {
            Self::Single => None,
            Self::Double => Some("double-precision"),
        }
    }

    pub(super) const fn c_define(self) -> Option<&'static str> {
        match self {
            Self::Single => None,
            Self::Double => Some("-DBOX2D_DOUBLE_PRECISION=1"),
        }
    }

    pub(crate) const fn wasm_bindings_file(self) -> &'static str {
        match self {
            Self::Single => "bindings_wasm32_unknown_unknown.rs",
            Self::Double => "bindings_wasm32_unknown_unknown_double.rs",
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProviderSmokeSession {
    target_dir: PathBuf,
    out_dir: PathBuf,
    provider_js: PathBuf,
    provider_wasm: PathBuf,
    _update_lock: UpdateLock,
}

impl ProviderSmokeSession {
    pub(crate) fn target_dir(&self) -> &Path {
        &self.target_dir
    }

    pub(crate) fn provider_js(&self) -> &Path {
        &self.provider_js
    }

    pub(crate) fn provider_wasm(&self) -> &Path {
        &self.provider_wasm
    }
}

pub(crate) fn provider_smoke_app(root: &Path) -> Result<()> {
    let _lock = UpdateLock::acquire(root)?;
    let cargo = QualifiedCargo::qualify(root)?;
    let target_dir = cargo.target_dir().to_path_buf();
    let precision = ProviderPrecision::from_env()?;
    let app = build_provider_smoke_app(root, &target_dir, &cargo, precision)?;
    let imports = collect_provider_imports(&app, precision.module())?;
    write_exports_json(root, &provider_smoke_dir(&target_dir), &imports, precision)?;
    println!(
        "provider smoke app ready: {} ({} provider imports)",
        app.display(),
        imports.len()
    );
    Ok(())
}

pub(crate) fn provider_smoke(root: &Path) -> Result<()> {
    let precision = ProviderPrecision::from_env()?;
    provider_smoke_for_precision(root, precision).map(|_| ())
}

pub(super) fn provider_smoke_for_precision(
    root: &Path,
    precision: ProviderPrecision,
) -> Result<(ProviderSmokeSession, EmscriptenTools)> {
    let (session, sdk) = build_provider_smoke_only(root, precision)?;
    let command = sdk.node_command().map_err(Error::Message)?;
    run_existing_provider_node_smoke(command, &session)?;
    Ok((session, sdk))
}

pub(crate) fn build_provider_smoke_only(
    root: &Path,
    precision: ProviderPrecision,
) -> Result<(ProviderSmokeSession, EmscriptenTools)> {
    let update_lock = UpdateLock::acquire(root)?;
    let cargo = QualifiedCargo::qualify(root)?;
    let target_dir = cargo.target_dir().to_path_buf();
    let sdk = qualified_provider_sdk()?;
    let app_wasm = build_provider_smoke_app(root, &target_dir, &cargo, precision)?;
    let imports = collect_provider_imports(&app_wasm, precision.module())?;
    let out_dir = provider_smoke_dir(&target_dir);
    let exports = write_exports_json(root, &out_dir, &imports, precision)?;
    let provider = build_box2d_provider(root, &out_dir, &exports, &sdk, precision)?;
    let provider_wasm = provider.with_extension("wasm");
    ensure_file(&provider_wasm, "Box2D provider wasm")?;
    let consumer_bytes = fs::read(&app_wasm).map_err(|source| Error::io(&app_wasm, source))?;
    let provider_bytes =
        fs::read(&provider_wasm).map_err(|source| Error::io(&provider_wasm, source))?;
    wasm_provider_gate::validate_consumer_provider_signatures(
        &consumer_bytes,
        &provider_bytes,
        precision.module(),
    )
    .map_err(|error| {
        Error::Message(format!(
            "provider-smoke consumer/provider function contract failed: {error}"
        ))
    })?;
    let app_copy = out_dir.join(PROVIDER_SMOKE_WASM);
    write_node_runner(&out_dir, &provider, &app_copy, &imports, precision.module())?;
    Ok((
        ProviderSmokeSession {
            target_dir,
            out_dir,
            provider_js: provider,
            provider_wasm,
            _update_lock: update_lock,
        },
        sdk,
    ))
}

pub(crate) fn prepare_existing_provider_smoke(
    root: &Path,
    precision: ProviderPrecision,
    provider_js: &Path,
    provider_wasm: &Path,
) -> Result<ProviderSmokeSession> {
    let update_lock = UpdateLock::acquire(root)?;
    let cargo = QualifiedCargo::qualify(root)?;
    let target_dir = cargo.target_dir().to_path_buf();
    let app_wasm = build_provider_smoke_app(root, &target_dir, &cargo, precision)?;
    let imports = collect_provider_imports(&app_wasm, precision.module())?;
    let expected_exports = provider_export_contract(root, precision, &imports)?;
    let provider_bytes =
        fs::read(provider_wasm).map_err(|source| Error::io(provider_wasm, source))?;
    wasm_provider_gate::validate_provider(&provider_bytes, &expected_exports).map_err(|error| {
        Error::Message(format!(
            "authenticated provider Wasm export contract failed for {}: {error}",
            provider_wasm.display()
        ))
    })?;
    let consumer_bytes = fs::read(&app_wasm).map_err(|source| Error::io(&app_wasm, source))?;
    wasm_provider_gate::validate_consumer_provider_signatures(
        &consumer_bytes,
        &provider_bytes,
        precision.module(),
    )
    .map_err(|error| {
        Error::Message(format!(
            "authenticated consumer/provider function contract failed for {}: {error}",
            provider_wasm.display()
        ))
    })?;
    let out_dir = provider_smoke_dir(&target_dir);
    let installed_js = out_dir.join(format!("{}.js", precision.module()));
    let installed_wasm = out_dir.join(format!("{}.wasm", precision.module()));
    copy_file(provider_js, &installed_js)?;
    copy_file(provider_wasm, &installed_wasm)?;
    let app_copy = out_dir.join(PROVIDER_SMOKE_WASM);
    write_node_runner(
        &out_dir,
        &installed_js,
        &app_copy,
        &imports,
        precision.module(),
    )?;
    Ok(ProviderSmokeSession {
        target_dir,
        out_dir,
        provider_js: installed_js,
        provider_wasm: installed_wasm,
        _update_lock: update_lock,
    })
}

pub(crate) fn run_existing_provider_node_smoke(
    mut command: Command,
    session: &ProviderSmokeSession,
) -> Result<()> {
    command.arg(session.out_dir.join("run-provider-smoke.mjs"));
    run_command(&mut command, "run provider shared-memory smoke")
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum WasmProviderContractMode {
    Check,
    Write,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WasmProviderContractInputs {
    effective_source: EffectiveSourceIdentity,
    adapter_source_sha256: String,
    single_bindings_sha256: String,
    double_bindings_sha256: String,
}

#[derive(Debug)]
struct CapturedProviderFile {
    live_path: PathBuf,
    captured_path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct CapturedWasmProviderSources {
    _staging_guard: tempfile::TempDir,
    staging_root: PathBuf,
    effective_sources: MaterializedEffectiveSources,
    adapter_crate_root: PathBuf,
    adapter_source_sha256: String,
    single_bindings: CapturedProviderFile,
    double_bindings: CapturedProviderFile,
}

impl CapturedWasmProviderSources {
    fn bindings(&self, precision: ProviderPrecision) -> &CapturedProviderFile {
        match precision {
            ProviderPrecision::Single => &self.single_bindings,
            ProviderPrecision::Double => &self.double_bindings,
        }
    }

    fn adapter_dir(&self) -> PathBuf {
        self.adapter_crate_root.join("native")
    }

    fn adapter_c_sources(&self) -> impl Iterator<Item = PathBuf> + '_ {
        ADAPTER_SOURCE_PATHS
            .iter()
            .filter(|relative| relative.ends_with(".c"))
            .map(|relative| self.adapter_crate_root.join(relative))
    }

    fn wasm_runtime_source(&self) -> PathBuf {
        self.adapter_dir().join("boxdd_wasm_runtime.js")
    }

    fn revalidate(&self, root: &Path, sdk: &EmscriptenTools) -> Result<()> {
        require_private_capture_directory(&self.staging_root)?;
        let crate_root = root.join("boxdd-sys");
        ensure_adapter_source_sha256(
            &self.adapter_crate_root,
            &self.adapter_source_sha256,
            "captured provider adapter",
        )?;
        ensure_adapter_source_sha256(
            &crate_root,
            &self.adapter_source_sha256,
            "live provider adapter",
        )?;
        self.single_bindings.revalidate()?;
        self.double_bindings.revalidate()?;

        let current = materialize_effective_box2d_sources(&crate_root, &self.staging_root)
            .map_err(|error| {
                Error::Message(format!(
                    "failed to revalidate captured provider effective sources: {error}"
                ))
            })?;
        ensure_same_materialized_effective_sources(&self.effective_sources, &current)?;
        sdk.revalidate().map_err(Error::Message)
    }
}

impl CapturedProviderFile {
    fn revalidate(&self) -> Result<()> {
        ensure_regular_file(&self.captured_path, "captured provider input")?;
        ensure_regular_file(&self.live_path, "live provider input")?;
        ensure_file_sha256(&self.captured_path, &self.sha256, "captured provider input")?;
        ensure_file_sha256(&self.live_path, &self.sha256, "live provider input")
    }
}

#[derive(Debug)]
struct GeneratedWasmProviderContract {
    precision: ProviderPrecision,
    identity: WasmProviderIdentity,
    source: String,
}

#[derive(Debug, Eq, PartialEq)]
struct WasmProviderContractBaseline {
    path: PathBuf,
    state: WasmProviderContractBaselineState,
}

#[derive(Debug, Eq, PartialEq)]
enum WasmProviderContractBaselineState {
    Missing,
    Existing(Vec<u8>),
}

pub(crate) fn wasm_provider_contract(root: &Path, args: &[String]) -> Result<()> {
    let mode = match args {
        [flag] if flag == "--check" => WasmProviderContractMode::Check,
        [flag] if flag == "--write" => WasmProviderContractMode::Write,
        _ => {
            return Err(Error::Message(
                "usage: cargo run -p xtask -- wasm-provider-contract --check|--write".to_owned(),
            ));
        }
    };

    let _lock = UpdateLock::acquire(root)?;
    let paths = WorkspacePaths::new(root);
    let manifest_path = paths.upstream_manifest();
    let manifest_metadata =
        fs::symlink_metadata(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    if !manifest_metadata.file_type().is_file() || manifest_metadata.file_type().is_symlink() {
        return Err(Error::Message(format!(
            "upstream manifest must be a regular non-symlink file: {}",
            manifest_path.display()
        )));
    }
    let manifest_baseline =
        fs::read(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    let mut manifest = UpstreamManifest::load(&paths)?;
    require_provider_identity_topology(&manifest)?;
    let baselines = capture_wasm_provider_contract_baselines(root, mode)?;
    let inputs = capture_wasm_provider_contract_inputs(root)?;
    let sdk = qualified_provider_sdk()?;
    let target_dir = root.join("target");
    let out_dir = target_dir.join("boxdd-wasm-provider-contract");
    replace_dir_under(&out_dir, &target_dir)?;
    let generated = generate_wasm_provider_contracts(root, &out_dir, &sdk, &inputs)?;
    revalidate_wasm_provider_contract_inputs(root, &sdk, &inputs)?;

    match mode {
        WasmProviderContractMode::Check => {
            validate_wasm_provider_contract_baselines(&baselines)?;
            validate_generated_wasm_provider_contracts(root, &generated)?;
            revalidate_wasm_provider_contract_inputs(root, &sdk, &inputs)?;
            validate_wasm_provider_contract_baselines(&baselines)?;
            validate_repository(&paths, &manifest)?;
            println!("WASM provider contracts are canonical and current for single and double");
        }
        WasmProviderContractMode::Write => {
            update_wasm_provider_artifact_digests(&mut manifest, &generated)?;
            let manifest_content = render_toml(&manifest)?.into_bytes();
            validate_wasm_provider_contract_baselines(&baselines)?;
            write_atomic_bytes(&baselines[0].path, generated[0].source.as_bytes())?;
            write_atomic_bytes(&baselines[1].path, generated[1].source.as_bytes())?;
            let current_manifest =
                fs::read(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
            if current_manifest != manifest_baseline {
                return Err(Error::Message(
                    "upstream manifest changed while WASM provider contracts were generated"
                        .to_owned(),
                ));
            }
            write_atomic_bytes(&manifest_path, &manifest_content)?;
            validate_generated_wasm_provider_contracts(root, &generated)?;
            revalidate_wasm_provider_contract_inputs(root, &sdk, &inputs)?;
            let installed_manifest = UpstreamManifest::load(&paths)?;
            validate_repository(&paths, &installed_manifest)?;
            println!(
                "refreshed canonical single and double WASM provider contracts and upstream manifest digests"
            );
        }
    }
    Ok(())
}

fn update_wasm_provider_artifact_digests(
    manifest: &mut UpstreamManifest,
    generated: &[GeneratedWasmProviderContract; 2],
) -> Result<()> {
    for contract in generated {
        let precision = match contract.precision {
            ProviderPrecision::Single => ManifestPrecision::Single,
            ProviderPrecision::Double => ManifestPrecision::Double,
        };
        let expected_name = match contract.precision {
            ProviderPrecision::Single => PROVIDER_IDENTITY_SINGLE_ARTIFACT,
            ProviderPrecision::Double => PROVIDER_IDENTITY_DOUBLE_ARTIFACT,
        };
        let mut artifacts = manifest.artifacts.iter_mut().filter(|artifact| {
            artifact.kind == ArtifactKind::ProviderIdentity && artifact.precision == Some(precision)
        });
        let artifact = artifacts.next().ok_or_else(|| {
            Error::Message(format!(
                "upstream manifest has no {} provider identity artifact",
                contract.precision.as_str()
            ))
        })?;
        if artifacts.next().is_some() || artifact.name != expected_name {
            return Err(Error::Message(format!(
                "upstream manifest does not contain the unique canonical {} provider identity artifact",
                contract.precision.as_str()
            )));
        }
        artifact.content_blake3 = blake3::hash(contract.source.as_bytes())
            .to_hex()
            .to_string();
    }
    Ok(())
}

/// Regenerates both provider contracts without acquiring `UpdateLock`.
///
/// The caller owns the ordered repository update and must include both individually atomic outputs
/// in the final manifest digest. Git remains the recovery mechanism for interruption between files.
pub(super) fn refresh_wasm_provider_contracts_unlocked(
    root: &Path,
    out_dir: &Path,
    sdk: &EmscriptenTools,
) -> Result<()> {
    let baselines =
        capture_wasm_provider_contract_baselines(root, WasmProviderContractMode::Write)?;
    let inputs = capture_wasm_provider_contract_inputs(root)?;
    let parent = out_dir.parent().ok_or_else(|| {
        Error::Message(format!(
            "WASM provider contract output has no parent: {}",
            out_dir.display()
        ))
    })?;
    replace_dir_under(out_dir, parent)?;
    let generated = generate_wasm_provider_contracts(root, out_dir, sdk, &inputs)?;
    revalidate_wasm_provider_contract_inputs(root, sdk, &inputs)?;
    validate_wasm_provider_contract_baselines(&baselines)?;
    write_atomic_bytes(&baselines[0].path, generated[0].source.as_bytes())?;
    write_atomic_bytes(&baselines[1].path, generated[1].source.as_bytes())?;
    validate_generated_wasm_provider_contracts(root, &generated)?;
    revalidate_wasm_provider_contract_inputs(root, sdk, &inputs)
}

fn revalidate_wasm_provider_contract_inputs(
    root: &Path,
    sdk: &EmscriptenTools,
    expected: &WasmProviderContractInputs,
) -> Result<()> {
    sdk.revalidate().map_err(Error::Message)?;
    let current = capture_wasm_provider_contract_inputs(root)?;
    if current == *expected {
        Ok(())
    } else {
        Err(Error::Message(
            "WASM provider contract inputs changed while candidates were generated".to_owned(),
        ))
    }
}

fn capture_wasm_provider_contract_inputs(root: &Path) -> Result<WasmProviderContractInputs> {
    let crate_root = root.join("boxdd-sys");
    let binding_sha256 = |precision: ProviderPrecision| {
        provider_input_sha256(
            &crate_root.join("src").join(precision.wasm_bindings_file()),
            "WASM provider bindings",
        )
    };
    Ok(WasmProviderContractInputs {
        effective_source: effective_source_identity(&crate_root).map_err(Error::Message)?,
        adapter_source_sha256: adapter_source_sha256(&crate_root).map_err(Error::Message)?,
        single_bindings_sha256: binding_sha256(ProviderPrecision::Single)?,
        double_bindings_sha256: binding_sha256(ProviderPrecision::Double)?,
    })
}

fn capture_wasm_provider_contract_baselines(
    root: &Path,
    mode: WasmProviderContractMode,
) -> Result<[WasmProviderContractBaseline; 2]> {
    let crate_root = root.join("boxdd-sys");
    [ProviderPrecision::Single, ProviderPrecision::Double]
        .map(|precision| {
            let relative = contract_relative_path(precision.as_str()).map_err(Error::Message)?;
            let path = crate_root.join(relative);
            let state = match fs::symlink_metadata(&path) {
                Ok(metadata)
                    if metadata.file_type().is_file() && !metadata.file_type().is_symlink() =>
                {
                    WasmProviderContractBaselineState::Existing(
                        fs::read(&path).map_err(|source| Error::io(&path, source))?,
                    )
                }
                Ok(_) => {
                    return Err(Error::Message(format!(
                        "WASM provider contract must be a regular non-symlink file: {}",
                        path.display()
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if mode == WasmProviderContractMode::Check {
                        return Err(Error::Message(format!(
                            "WASM provider contract is missing: {}",
                            path.display()
                        )));
                    }
                    WasmProviderContractBaselineState::Missing
                }
                Err(error) => return Err(Error::io(&path, error)),
            };
            Ok(WasmProviderContractBaseline { path, state })
        })
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .map_err(|_| Error::Message("expected exactly two WASM provider contracts".to_owned()))
}

fn validate_wasm_provider_contract_baselines(
    baselines: &[WasmProviderContractBaseline; 2],
) -> Result<()> {
    for baseline in baselines {
        let unchanged = match (&baseline.state, fs::symlink_metadata(&baseline.path)) {
            (WasmProviderContractBaselineState::Missing, Err(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                true
            }
            (WasmProviderContractBaselineState::Existing(expected), Ok(metadata))
                if metadata.file_type().is_file() && !metadata.file_type().is_symlink() =>
            {
                fs::read(&baseline.path)
                    .map(|current| current == *expected)
                    .map_err(|source| Error::io(&baseline.path, source))?
            }
            (_, Err(error)) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(Error::io(&baseline.path, error));
            }
            _ => false,
        };
        if !unchanged {
            return Err(Error::Message(format!(
                "WASM provider contract changed while candidates were generated: {}",
                baseline.path.display()
            )));
        }
    }
    Ok(())
}

fn capture_wasm_provider_sources(
    root: &Path,
    out_dir: &Path,
) -> Result<CapturedWasmProviderSources> {
    let crate_root = root.join("boxdd-sys");
    fs::create_dir_all(out_dir).map_err(|source| Error::io(out_dir, source))?;
    let staging_guard = tempfile::Builder::new()
        .prefix(CAPTURED_PROVIDER_INPUTS_DIRECTORY)
        .tempdir_in(out_dir)
        .map_err(|source| Error::io(out_dir, source))?;
    let staging_root = staging_guard.path().to_path_buf();
    make_capture_directory_private(&staging_root)?;

    let expected_effective_source =
        effective_source_identity(&crate_root).map_err(Error::Message)?;
    let expected_adapter_source_sha256 =
        checked_adapter_source_sha256(&crate_root, "live provider adapter")?;
    let effective_sources = materialize_effective_box2d_sources(&crate_root, &staging_root)
        .map_err(|error| {
            Error::Message(format!(
                "failed to materialize captured provider effective sources: {error}"
            ))
        })?;
    if effective_sources.identity != expected_effective_source {
        return Err(Error::Message(
            "provider effective source identity changed while its private capture was created"
                .to_owned(),
        ));
    }

    let adapter_crate_root = staging_root.join("adapter-crate");
    fs::create_dir(&adapter_crate_root).map_err(|source| Error::io(&adapter_crate_root, source))?;
    for relative in ADAPTER_SOURCE_PATHS {
        let source = crate_root.join(relative);
        let destination = adapter_crate_root.join(relative);
        copy_regular_provider_input(&source, &destination, "provider adapter input")?;
    }
    ensure_adapter_source_sha256(
        &adapter_crate_root,
        &expected_adapter_source_sha256,
        "captured provider adapter",
    )?;
    ensure_adapter_source_sha256(
        &crate_root,
        &expected_adapter_source_sha256,
        "live provider adapter",
    )?;

    let bindings_root = staging_root.join("bindings");
    fs::create_dir(&bindings_root).map_err(|source| Error::io(&bindings_root, source))?;
    let capture_bindings = |precision: ProviderPrecision| {
        let name = precision.wasm_bindings_file();
        capture_provider_file(
            &crate_root.join("src").join(name),
            &bindings_root.join(name),
            "WASM provider bindings",
        )
    };
    let captured = CapturedWasmProviderSources {
        _staging_guard: staging_guard,
        staging_root,
        effective_sources,
        adapter_crate_root,
        adapter_source_sha256: expected_adapter_source_sha256,
        single_bindings: capture_bindings(ProviderPrecision::Single)?,
        double_bindings: capture_bindings(ProviderPrecision::Double)?,
    };
    captured.single_bindings.revalidate()?;
    captured.double_bindings.revalidate()?;
    require_private_capture_directory(&captured.staging_root)?;
    Ok(captured)
}

fn capture_provider_file(
    live_path: &Path,
    captured_path: &Path,
    label: &str,
) -> Result<CapturedProviderFile> {
    ensure_regular_file(live_path, label)?;
    let expected = provider_input_sha256(live_path, label)?;
    copy_regular_provider_input(live_path, captured_path, label)?;
    ensure_file_sha256(captured_path, &expected, "captured provider input")?;
    ensure_file_sha256(live_path, &expected, "live provider input")?;
    Ok(CapturedProviderFile {
        live_path: live_path.to_path_buf(),
        captured_path: captured_path.to_path_buf(),
        sha256: expected,
    })
}

fn copy_regular_provider_input(source: &Path, destination: &Path, label: &str) -> Result<()> {
    ensure_regular_file(source, label)?;
    copy_file(source, destination)?;
    ensure_regular_file(source, label)?;
    ensure_regular_file(destination, "captured provider input")?;
    Ok(())
}

fn checked_adapter_source_sha256(crate_root: &Path, label: &str) -> Result<String> {
    for relative in ADAPTER_SOURCE_PATHS {
        ensure_regular_file(&crate_root.join(relative), label)?;
    }
    let digest = adapter_source_sha256(crate_root)
        .map_err(|error| Error::Message(format!("failed to identify {label}: {error}")))?;
    for relative in ADAPTER_SOURCE_PATHS {
        ensure_regular_file(&crate_root.join(relative), label)?;
    }
    Ok(digest)
}

fn ensure_adapter_source_sha256(crate_root: &Path, expected: &str, label: &str) -> Result<()> {
    let current = checked_adapter_source_sha256(crate_root, label)?;
    if current == expected {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} SHA-256 changed: expected {expected}, observed {current}"
        )))
    }
}

fn ensure_file_sha256(path: &Path, expected: &str, label: &str) -> Result<()> {
    let current = provider_input_sha256(path, label)?;
    if current == expected {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} SHA-256 changed for {}: expected {expected}, observed {current}",
            path.display()
        )))
    }
}

fn provider_input_sha256(path: &Path, label: &str) -> Result<String> {
    Ok(
        VerifiedFileSnapshot::read(path, MAX_CAPTURED_PROVIDER_INPUT_BYTES, label)
            .map_err(Error::Message)?
            .sha256()
            .to_owned(),
    )
}

fn ensure_regular_file(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if metadata.file_type().is_file() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        )))
    }
}

fn make_capture_directory_private(path: &Path) -> Result<()> {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|source| Error::io(path, source))?;
    require_private_capture_directory(path)
}

fn require_private_capture_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::Message(format!(
            "captured provider input root must be a real directory: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error::Message(format!(
            "captured provider input root must not grant group or other access: {}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_same_materialized_effective_sources(
    expected: &MaterializedEffectiveSources,
    current: &MaterializedEffectiveSources,
) -> Result<()> {
    expected.revalidate().map_err(|error| {
        Error::Message(format!(
            "captured provider effective source tree changed while the provider was compiled: {error}"
        ))
    })?;
    current.revalidate().map_err(|error| {
        Error::Message(format!(
            "revalidated provider effective source tree is invalid: {error}"
        ))
    })?;
    if expected.identity == current.identity
        && materialized_source_layout(expected) == materialized_source_layout(current)
    {
        Ok(())
    } else {
        Err(Error::Message(
            "captured provider effective sources changed while the provider was compiled"
                .to_owned(),
        ))
    }
}

fn materialized_source_layout(
    sources: &MaterializedEffectiveSources,
) -> Option<(PathBuf, PathBuf, Vec<PathBuf>)> {
    Some((
        sources
            .public_include
            .strip_prefix(&sources.root)
            .ok()?
            .to_path_buf(),
        sources
            .private_include
            .strip_prefix(&sources.root)
            .ok()?
            .to_path_buf(),
        sources
            .c_sources
            .iter()
            .map(|source| {
                source
                    .strip_prefix(&sources.root)
                    .map(Path::to_path_buf)
                    .ok()
            })
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn run_provider_compilation_bound_to_inputs<T>(
    compile: impl FnOnce() -> Result<T>,
    revalidate: impl FnOnce() -> Result<()>,
) -> Result<T> {
    let output = compile()?;
    revalidate()?;
    Ok(output)
}

fn generate_wasm_provider_contracts(
    root: &Path,
    out_dir: &Path,
    sdk: &EmscriptenTools,
    inputs: &WasmProviderContractInputs,
) -> Result<[GeneratedWasmProviderContract; 2]> {
    let captured = capture_wasm_provider_sources(root, out_dir)?;
    if captured.effective_sources.identity != inputs.effective_source {
        return Err(Error::Message(
            "materialized effective source identity changed after refresh input capture".to_owned(),
        ));
    }
    if captured.adapter_source_sha256 != inputs.adapter_source_sha256 {
        return Err(Error::Message(
            "captured provider adapter identity changed after refresh input capture".to_owned(),
        ));
    }
    let mut generated = Vec::with_capacity(2);
    for precision in [ProviderPrecision::Single, ProviderPrecision::Double] {
        let identity = compile_wasm_provider_identity(sdk, precision, &captured)?;
        let expected_bindings = match precision {
            ProviderPrecision::Single => &inputs.single_bindings_sha256,
            ProviderPrecision::Double => &inputs.double_bindings_sha256,
        };
        if identity.bindings_sha256 != *expected_bindings {
            return Err(Error::Message(format!(
                "{} WASM provider bindings changed while its identity was compiled",
                precision.as_str()
            )));
        }
        let source = identity.render();
        generated.push(GeneratedWasmProviderContract {
            precision,
            identity,
            source,
        });
    }
    captured.revalidate(root, sdk)?;
    generated
        .try_into()
        .map_err(|_| Error::Message("expected exactly two generated WASM contracts".to_owned()))
}

fn validate_generated_wasm_provider_contracts(
    root: &Path,
    generated: &[GeneratedWasmProviderContract; 2],
) -> Result<()> {
    for contract in generated {
        verify_checked_wasm_provider_identity(root, contract.precision, &contract.identity)?;
    }
    Ok(())
}

pub(super) fn provider_smoke_dir(target_dir: &Path) -> PathBuf {
    target_dir.join("boxdd-provider-smoke")
}

fn build_provider_smoke_app(
    root: &Path,
    target_dir: &Path,
    cargo: &QualifiedCargo,
    precision: ProviderPrecision,
) -> Result<PathBuf> {
    let profile = BuildProfile::for_provider_smoke();
    let mut command = cargo.wasm_command(root)?;
    command
        .arg("rustc")
        .arg("--locked")
        .arg("-p")
        .arg(PROVIDER_SMOKE_PACKAGE)
        .arg("--lib")
        .arg("--target")
        .arg(WASM_TARGET)
        .args(profile.cargo_args())
        .current_dir(root)
        .env(
            "BOXDD_SYS_PROVIDER",
            ProviderCapability::WasmProvider.as_str(),
        );
    if let Some(feature) = precision.cargo_feature() {
        command.arg("--features").arg(feature);
    }
    add_wasm_app_link_args(&mut command, &[PROVIDER_SMOKE_EXPORTS, RUNTIME_EXPORTS]);
    run_command(&mut command, "build provider-smoke Rust wasm")?;

    let wasm = target_dir
        .join(WASM_TARGET)
        .join(profile.target_dir())
        .join(PROVIDER_SMOKE_WASM);
    let bytes = fs::read(&wasm).map_err(|source| Error::io(&wasm, source))?;
    wasm_provider_gate::validate_consumer(&bytes, precision.module(), "env").map_err(|error| {
        Error::Message(format!(
            "provider-smoke Wasm memory contract failed for {}: {error}",
            wasm.display()
        ))
    })?;

    let out_dir = provider_smoke_dir(target_dir);
    replace_dir_under(&out_dir, target_dir)?;
    copy_file(&wasm, &out_dir.join(PROVIDER_SMOKE_WASM))?;
    Ok(wasm)
}

pub(super) fn collect_provider_imports(wasm: &Path, provider_module: &str) -> Result<Vec<String>> {
    let bytes = fs::read(wasm).map_err(|source| Error::io(wasm, source))?;
    let mut imports = BTreeSet::new();
    for payload in Parser::new(0).parse_all(&bytes) {
        let payload = payload.map_err(|error| {
            Error::Message(format!(
                "failed to parse WASM imports from {}: {error}",
                wasm.display()
            ))
        })?;
        let Payload::ImportSection(section) = payload else {
            continue;
        };
        for import in section.into_imports() {
            let import = import.map_err(|error| {
                Error::Message(format!(
                    "failed to parse WASM import from {}: {error}",
                    wasm.display()
                ))
            })?;
            if import.module == provider_module
                && matches!(import.ty, TypeRef::Func(_) | TypeRef::FuncExact(_))
            {
                imports.insert(import.name.to_owned());
            }
        }
    }
    let imports = imports.into_iter().collect::<Vec<_>>();
    if imports.is_empty() {
        return Err(Error::Message(format!(
            "{} does not import any functions from {provider_module}",
            wasm.display()
        )));
    }
    if let Some(missing) = REQUIRED_RUNTIME_IDENTITY_IMPORTS
        .iter()
        .find(|required| !imports.iter().any(|import| import == *required))
    {
        return Err(Error::Message(format!(
            "{} does not import required provider runtime identity function `{missing}` from {provider_module}",
            wasm.display()
        )));
    }
    Ok(imports)
}

fn provider_export_contract(
    root: &Path,
    precision: ProviderPrecision,
    imports: &[String],
) -> Result<BTreeSet<String>> {
    let paths = WorkspacePaths::new(root);
    let mut exports = super::api_inventory::validated_c_api_function_names(&paths)?;
    if precision == ProviderPrecision::Double {
        if !exports.remove("b2CreateWorld") {
            return Err(Error::Message(
                "reviewed C API inventory does not contain b2CreateWorld".to_owned(),
            ));
        }
        exports.insert("b2CreateWorldDoublePrecision".to_owned());
    }
    exports.extend(
        REQUIRED_WASM_PROVIDER_ADAPTER_EXPORTS
            .iter()
            .map(|name| (*name).to_owned()),
    );
    exports.insert(PROVIDER_HEAP_BOUNDARY_PROBE.to_owned());
    let unsupported = imports
        .iter()
        .filter(|name| !exports.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !unsupported.is_empty() {
        return Err(Error::Message(format!(
            "consumer imports functions outside the {} WASM provider ABI: {unsupported:?}",
            precision.as_str()
        )));
    }
    Ok(exports)
}

pub(super) fn write_exports_json(
    root: &Path,
    out_dir: &Path,
    imports: &[String],
    precision: ProviderPrecision,
) -> Result<PathBuf> {
    fs::create_dir_all(out_dir).map_err(|source| Error::io(out_dir, source))?;
    let exported = provider_export_contract(root, precision, imports)?;
    let path = out_dir.join("box2d-provider-exports.json");
    let exports = exported
        .iter()
        .map(|name| format!("_{name}"))
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&exports)
        .map_err(|error| Error::Message(format!("serialize provider exports: {error}")))?;
    fs::write(&path, bytes).map_err(|source| Error::io(&path, source))?;
    Ok(path)
}

pub(super) fn read_exports_json(path: &Path) -> Result<BTreeSet<String>> {
    let bytes = fs::read(path).map_err(|source| Error::io(path, source))?;
    let encoded: Vec<String> = serde_json::from_slice(&bytes).map_err(|error| {
        Error::Message(format!(
            "provider exports manifest {} is invalid JSON: {error}",
            path.display()
        ))
    })?;
    let mut exports = BTreeSet::new();
    for encoded_name in encoded {
        let name = encoded_name.strip_prefix('_').ok_or_else(|| {
            Error::Message(format!(
                "provider export `{encoded_name}` in {} must use Emscripten's leading underscore",
                path.display()
            ))
        })?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || !exports.insert(name.to_owned())
        {
            return Err(Error::Message(format!(
                "provider export `{encoded_name}` in {} is invalid or duplicated",
                path.display()
            )));
        }
    }
    if exports.is_empty() {
        return Err(Error::Message(format!(
            "provider exports manifest {} is empty",
            path.display()
        )));
    }
    Ok(exports)
}

pub(super) fn build_box2d_provider(
    root: &Path,
    out_dir: &Path,
    exports_json: &Path,
    sdk: &EmscriptenTools,
    precision: ProviderPrecision,
) -> Result<PathBuf> {
    let provider = out_dir.join(format!("{}.js", precision.module()));
    let captured = capture_wasm_provider_sources(root, out_dir)?;
    let captured_exports = capture_provider_file(
        exports_json,
        &captured.staging_root.join("box2d-provider-exports.json"),
        "provider exports manifest",
    )?;
    let expected_exports = read_exports_json(&captured_exports.captured_path)?;
    for source in &captured.effective_sources.c_sources {
        ensure_file(source, "Box2D provider source inventory entry")?;
    }
    let adapter_sources = captured.adapter_c_sources().collect::<Vec<_>>();
    for source in &adapter_sources {
        ensure_file(source, "captured BoxDD provider adapter source")?;
    }
    let wasm_runtime_source = captured.wasm_runtime_source();
    ensure_file(
        &wasm_runtime_source,
        "captured BoxDD provider WASM runtime source",
    )?;
    let compiled_identity =
        verify_wasm_provider_identity_contract(root, sdk, precision, &captured)?;

    let mut command = sdk.emcc_command().map_err(Error::Message)?;
    let adapter_dir = captured.adapter_dir();
    configure_provider_c_abi(
        &mut command,
        precision,
        &captured.effective_sources.identity.effective_source_sha256,
        &captured.effective_sources.public_include,
        &captured.effective_sources.private_include,
        &adapter_dir,
    );
    command
        .arg("-s")
        .arg("MODULARIZE=1")
        .arg("-s")
        .arg("EXPORT_ES6=1")
        .arg("-s")
        .arg("ENVIRONMENT=node,web")
        .arg("-s")
        .arg("INCOMING_MODULE_JS_API=['wasmMemory','wasmBinary','locateFile','print','printErr']")
        .arg("-s")
        .arg("EXPORTED_RUNTIME_METHODS=['HEAPU8']")
        .arg("--post-js")
        .arg(&wasm_runtime_source)
        .arg("-s")
        .arg(format!("GLOBAL_BASE={PROVIDER_STATIC_BASE_BYTES}"))
        .arg("-s")
        .arg("IMPORTED_MEMORY=1")
        .arg("-s")
        .arg("ALLOW_MEMORY_GROWTH=1")
        .arg("-s")
        .arg(format!("INITIAL_MEMORY={INITIAL_MEMORY_BYTES}"))
        .arg("-s")
        .arg(format!("MAXIMUM_MEMORY={MAXIMUM_MEMORY_BYTES}"))
        .arg("-s")
        .arg("FILESYSTEM=0")
        .arg("-s")
        .arg("NO_EXIT_RUNTIME=1")
        .arg("-s")
        .arg("MALLOC=emmalloc")
        .arg("-s")
        .arg("ASSERTIONS=1")
        .arg("-s")
        .arg("STACK_SIZE=1048576")
        .arg("-s")
        .arg("ERROR_ON_UNDEFINED_SYMBOLS=1")
        .arg("-Wl,--export=__data_end,--export=__stack_low,--export=__stack_high,--export=__heap_base")
        .arg("-s")
        .arg(format!(
            "EXPORTED_FUNCTIONS=@{}",
            captured_exports
                .captured_path
                .to_string_lossy()
                .replace('\\', "/")
        ))
        .arg(c_string_define(
            "BOXDD_UPSTREAM_SHA",
            &captured.effective_sources.identity.upstream_sha,
        ))
        .arg(c_string_define("BOXDD_TARGET_ABI", WASM_TARGET))
        .arg(c_string_define(
            "BOXDD_ADAPTER_SOURCE_SHA256",
            &captured.adapter_source_sha256,
        ))
        .arg(c_string_define(
            "BOXDD_RECORDING_CONTRACT_BLAKE3",
            RECORDING_CONTRACT_BLAKE3,
        ))
        .arg(format!(
            "-DBOXDD_WASM_PROVIDER_HEAP_LIMIT={PROVIDER_HEAP_LIMIT_BYTES}"
        ));
    for file in &captured.effective_sources.c_sources {
        command.arg(file);
    }
    for file in adapter_sources {
        command.arg(file);
    }
    command.arg("-o").arg(&provider);
    run_provider_compilation_bound_to_inputs(
        || {
            run_command(&mut command, "build Box2D provider wasm")?;
            let provider_wasm = provider.with_extension("wasm");
            let bytes =
                fs::read(&provider_wasm).map_err(|source| Error::io(&provider_wasm, source))?;
            wasm_provider_gate::validate_provider(&bytes, &expected_exports).map_err(|error| {
                Error::Message(format!(
                    "provider Wasm memory/export contract failed for {}: {error}",
                    provider_wasm.display()
                ))
            })
        },
        || {
            verify_checked_wasm_provider_identity(root, precision, &compiled_identity)?;
            captured_exports.revalidate()?;
            captured.revalidate(root, sdk)
        },
    )?;
    Ok(provider)
}

fn verify_wasm_provider_identity_contract(
    root: &Path,
    sdk: &EmscriptenTools,
    precision: ProviderPrecision,
    captured: &CapturedWasmProviderSources,
) -> Result<WasmProviderIdentity> {
    let compiled = compile_wasm_provider_identity(sdk, precision, captured)?;
    verify_checked_wasm_provider_identity(root, precision, &compiled)?;
    Ok(compiled)
}

fn compile_wasm_provider_identity(
    sdk: &EmscriptenTools,
    precision: ProviderPrecision,
    captured: &CapturedWasmProviderSources,
) -> Result<WasmProviderIdentity> {
    let probe_dir = captured.staging_root.join("wasm-provider-identity-probe");
    replace_dir_under(&probe_dir, &captured.staging_root)?;
    let object = probe_dir.join("boxdd_identity_values.o");
    let source = captured.adapter_dir().join("boxdd_identity_values.c");
    let mut command = sdk.emcc_command().map_err(Error::Message)?;
    configure_provider_c_abi(
        &mut command,
        precision,
        &captured.effective_sources.identity.effective_source_sha256,
        &captured.effective_sources.public_include,
        &captured.effective_sources.private_include,
        &captured.adapter_dir(),
    );
    command.arg("-c").arg(&source).arg("-o").arg(&object);
    run_command(&mut command, "compile WASM provider identity probe")?;
    ensure_file(&object, "WASM provider identity probe")?;

    let (private_abi_hash, snapshot_layout_hash, definition_cookie) =
        read_wasm_provider_identity(&object)?;
    let bindings_sha256 = &captured.bindings(precision).sha256;
    WasmProviderIdentity::from_compiled(
        &wasm_provider_expectation(
            &captured.effective_sources.identity,
            &captured.adapter_source_sha256,
            bindings_sha256,
            precision,
        ),
        private_abi_hash,
        snapshot_layout_hash,
        definition_cookie,
    )
    .map_err(Error::Message)
}

fn wasm_provider_expectation<'a>(
    effective_source: &'a EffectiveSourceIdentity,
    adapter_source_sha256: &'a str,
    bindings_sha256: &'a str,
    precision: ProviderPrecision,
) -> WasmProviderExpectation<'a> {
    WasmProviderExpectation {
        provider_abi: PROVIDER_ABI,
        target: WASM_TARGET,
        compiler_target: COMPILER_TARGET,
        precision: precision.as_str(),
        upstream_sha: &effective_source.upstream_sha,
        source_tree: &effective_source.source_tree,
        effective_source_sha256: &effective_source.effective_source_sha256,
        adapter_abi_version: ADAPTER_ABI_VERSION,
        adapter_source_sha256,
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3,
        validation_enabled: false,
        simd: SIMD_MODE,
        pointer_width: POINTER_WIDTH,
        endianness: ENDIANNESS,
        bindings_sha256,
    }
}

fn verify_checked_wasm_provider_identity(
    root: &Path,
    precision: ProviderPrecision,
    compiled: &WasmProviderIdentity,
) -> Result<()> {
    let relative = contract_relative_path(precision.as_str()).map_err(Error::Message)?;
    let contract_path = root.join("boxdd-sys").join(relative);
    let checked = WasmProviderIdentity::load(
        &root.join("boxdd-sys"),
        Path::new(relative),
        &compiled.expectation(),
    )
    .map_err(Error::Message)?;
    if checked != *compiled {
        return Err(Error::Message(format!(
            "compiled {} WASM provider identity does not match {}",
            precision.as_str(),
            contract_path.display()
        )));
    }
    Ok(())
}

fn configure_provider_c_abi(
    command: &mut Command,
    precision: ProviderPrecision,
    effective_source_sha256: &str,
    public_include: &Path,
    private_include: &Path,
    adapter_include: &Path,
) {
    command
        .arg(format!("--target={COMPILER_TARGET}"))
        .arg("-std=c17")
        .arg("-O2")
        .arg("-D_POSIX_C_SOURCE=199309L")
        .arg("-DBOX2D_DISABLE_SIMD")
        .arg(c_string_define(
            "BOXDD_EFFECTIVE_SOURCE_SHA256",
            effective_source_sha256,
        ))
        .arg("-I")
        .arg(public_include)
        .arg("-I")
        .arg(private_include)
        .arg("-I")
        .arg(adapter_include);
    if let Some(define) = precision.c_define() {
        command.arg(define);
    }
}

fn read_wasm_provider_identity(object: &Path) -> Result<([u8; 32], u32, i32)> {
    let bytes = fs::read(object).map_err(|source| Error::io(object, source))?;
    let private_count = read_wasm_identity_count(&bytes, "boxddPrivateAbiValueCount")?;
    let layout_count = read_wasm_identity_count(&bytes, "boxddSnapshotLayoutValueCount")?;
    let private_values = read_wasm_identity_values(&bytes, "boxddPrivateAbiValues", private_count)?;
    let layout_values =
        read_wasm_identity_values(&bytes, "boxddSnapshotLayoutValues", layout_count)?;
    let definition_cookie =
        i32::try_from(read_wasm_identity_scalar(&bytes, "boxddDefinitionCookie")?)
            .map_err(|_| Error::Message("WASM definition cookie exceeds i32".to_owned()))?;
    Ok((
        private_abi_hash(&private_values, true),
        snapshot_layout_hash(&layout_values),
        definition_cookie,
    ))
}

fn read_wasm_identity_count(bytes: &[u8], name: &str) -> Result<usize> {
    let value = read_wasm_identity_scalar(bytes, name)?;
    let count = usize::try_from(value)
        .map_err(|_| Error::Message(format!("WASM identity count {value} is too large")))?;
    if count == 0 || count > MAX_IDENTITY_VALUES {
        return Err(Error::Message(format!(
            "WASM identity count {count} for {name} is outside 1..={MAX_IDENTITY_VALUES}"
        )));
    }
    Ok(count)
}

fn read_wasm_identity_scalar(bytes: &[u8], name: &str) -> Result<u64> {
    Ok(u64::from_le_bytes(
        wasm_identity::symbol_bytes(bytes, name, 8)
            .map_err(|error| Error::Message(error.to_string()))?
            .try_into()
            .expect("identity scalar has a fixed width"),
    ))
}

fn read_wasm_identity_values(bytes: &[u8], name: &str, count: usize) -> Result<Vec<u64>> {
    let width = count
        .checked_mul(8)
        .ok_or_else(|| Error::Message(format!("WASM identity array {name} is too large")))?;
    Ok(wasm_identity::symbol_bytes(bytes, name, width)
        .map_err(|error| Error::Message(error.to_string()))?
        .chunks_exact(8)
        .map(|chunk| {
            u64::from_le_bytes(chunk.try_into().expect("identity value has a fixed width"))
        })
        .collect())
}

fn c_string_define(name: &str, value: &str) -> String {
    format!("-D{name}=\"{value}\"")
}

fn write_node_runner(
    out_dir: &Path,
    provider: &Path,
    app_wasm: &Path,
    imports: &[String],
    provider_module: &str,
) -> Result<()> {
    let provider_name = provider
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Message("invalid provider file name".to_owned()))?;
    let app_name = app_wasm
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Message("invalid app wasm file name".to_owned()))?;
    let imports_array = imports
        .iter()
        .map(|name| format!("  \"{name}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let runner = format!(
        r#"import fs from 'node:fs';
import {{ dirname, join }} from 'node:path';
import {{ fileURLToPath }} from 'node:url';
import createProvider from './{provider_name}';
import {{
  inspectProviderContract,
  resolveProviderFunctions,
  runProviderPhysicsScenario,
}} from './provider-runtime-contract.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const memory = new WebAssembly.Memory({{ initial: {INITIAL_MEMORY_PAGES}, maximum: {MAXIMUM_MEMORY_PAGES} }});
const provider = await createProvider({{
  wasmMemory: memory,
  locateFile: (path) => join(here, path),
  print: (text) => console.log(`[{provider_module}] ${{text}}`),
  printErr: (text) => console.warn(`[{provider_module}] ${{text}}`),
}});

if (provider.wasmMemory && provider.wasmMemory !== memory) {{
  throw new Error('provider did not use the shared WebAssembly.Memory');
}}
const providerImports = [
{imports_array}
];
const appBytes = fs.readFileSync(join(here, '{app_name}'));
const appModule = await WebAssembly.compile(appBytes);
const providerContract = inspectProviderContract(appModule, '{provider_module}');
if (JSON.stringify(providerContract.names) !== JSON.stringify(providerImports)) {{
  throw new Error('generated provider import inventory differs from the runtime module');
}}
const providerFunctions = resolveProviderFunctions(provider, providerContract.names);
const result = await runProviderPhysicsScenario({{
  appModule,
  memory,
  provider,
  providerHeapLimitBytes: {PROVIDER_HEAP_LIMIT_BYTES},
  contract: providerContract,
  functions: providerFunctions,
}});

console.log(
  `boxdd provider smoke passed: drop_mm=${{result.metrics.dropMillimeters}}, ` +
    `ray_hit_mm=${{result.metrics.rayHitMillimeters}}, ` +
    `shape_cast_permyriad=${{result.metrics.shapeCastPermyriad}}, ` +
    `joint_error_mm=${{result.metrics.jointErrorMillimeters}}, ` +
    `stale_views_rejected=${{result.memoryProof.staleTypedArrayRejected && result.memoryProof.staleDataViewRejected}}, ` +
    `provider_glue_calls_after_growth=${{result.memoryProof.providerGlueCallsAfterGrowth}}, ` +
    `provider_heap_boundary=${{JSON.stringify(result.providerHeapBoundary)}}, ` +
    `allocator_proof=${{JSON.stringify(result.allocatorProof)}}, ` +
    `link_failures=${{JSON.stringify(result.linkFailures)}}, ` +
    `runtime_bodies=${{result.runtimeBodies}}, ` +
    `runtime_state=${{JSON.stringify(result.runtimeState)}}`
);
"#
    );
    let package_json = out_dir.join("package.json");
    fs::write(&package_json, r#"{"type":"module"}"#)
        .map_err(|source| Error::io(&package_json, source))?;
    let runtime_contract = out_dir.join(PROVIDER_RUNTIME_CONTRACT_FILE);
    fs::write(&runtime_contract, PROVIDER_RUNTIME_CONTRACT)
        .map_err(|source| Error::io(&runtime_contract, source))?;
    let path = out_dir.join("run-provider-smoke.mjs");
    fs::write(&path, runner).map_err(|source| Error::io(&path, source))
}

pub(super) fn qualified_provider_sdk() -> Result<EmscriptenTools> {
    EmscriptenTools::discover().map_err(Error::Message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolated_git::isolated_git_command;

    fn run_test_git(root: &Path, args: &[&str]) {
        let output = isolated_git_command()
            .expect("qualified Git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("run test Git command");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn provider_smoke_session_holds_the_repository_update_lock() {
        let repository = tempfile::tempdir().unwrap();
        let root = repository.path();
        run_test_git(root, &["init", "--quiet"]);

        let session = ProviderSmokeSession {
            target_dir: root.join("target"),
            out_dir: root.join("target/boxdd-provider-smoke"),
            provider_js: root.join("target/boxdd-provider-smoke/provider.js"),
            provider_wasm: root.join("target/boxdd-provider-smoke/provider.wasm"),
            _update_lock: UpdateLock::acquire(root).unwrap(),
        };
        assert!(UpdateLock::acquire(root).is_err());

        drop(session);
        UpdateLock::acquire(root).expect("lock after provider smoke session release");
    }

    fn write_adapter_fixture(root: &Path, marker: &str) {
        for relative in ADAPTER_SOURCE_PATHS {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("adapter fixture parent"))
                .expect("create adapter fixture parent");
            fs::write(&path, format!("{relative}:{marker}\n")).expect("write adapter fixture");
        }
    }

    #[test]
    fn provider_compile_capture_rejects_live_adapter_drift_after_compile_hook() {
        let directory = tempfile::tempdir().unwrap();
        let live = directory.path().join("live");
        let captured = directory.path().join("captured");
        fs::create_dir_all(&live).unwrap();
        fs::create_dir_all(&captured).unwrap();
        write_adapter_fixture(&live, "reviewed");
        let expected = checked_adapter_source_sha256(&live, "live test adapter").unwrap();
        for relative in ADAPTER_SOURCE_PATHS {
            copy_regular_provider_input(
                &live.join(relative),
                &captured.join(relative),
                "test adapter input",
            )
            .unwrap();
        }
        ensure_adapter_source_sha256(&captured, &expected, "captured test adapter").unwrap();

        let live_source = live.join("native/boxdd_adapter.c");
        let captured_source = captured.join("native/boxdd_adapter.c");
        let captured_before = fs::read(&captured_source).unwrap();
        let error = run_provider_compilation_bound_to_inputs(
            || {
                fs::write(&live_source, "changed during provider compilation\n")
                    .map_err(|source| Error::io(&live_source, source))?;
                assert_eq!(fs::read(&captured_source).unwrap(), captured_before);
                Ok(())
            },
            || ensure_adapter_source_sha256(&live, &expected, "live test adapter"),
        )
        .expect_err("compile-period live adapter drift must fail closed");
        assert!(error.to_string().contains("SHA-256 changed"));
    }

    #[test]
    fn provider_effective_source_revalidation_compares_content_not_generation_paths() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../boxdd-sys");
        let captured_output = tempfile::tempdir().unwrap();
        let current_output = tempfile::tempdir().unwrap();
        let captured =
            materialize_effective_box2d_sources(&crate_root, captured_output.path()).unwrap();
        let current =
            materialize_effective_box2d_sources(&crate_root, current_output.path()).unwrap();

        assert_ne!(captured.root, current.root);
        ensure_same_materialized_effective_sources(&captured, &current).unwrap();

        fs::write(captured.root.join("src/aabb.c"), "changed after capture\n").unwrap();
        let error = ensure_same_materialized_effective_sources(&captured, &current)
            .expect_err("captured effective-source drift must fail closed");
        assert!(
            error
                .to_string()
                .contains("captured provider effective source tree changed"),
            "{error}"
        );
    }

    #[test]
    fn provider_capture_directories_are_unique_and_lifetime_scoped() {
        let output = tempfile::tempdir().unwrap();
        let first = tempfile::Builder::new()
            .prefix(CAPTURED_PROVIDER_INPUTS_DIRECTORY)
            .tempdir_in(output.path())
            .unwrap();
        let first_path = first.path().to_path_buf();
        let second = tempfile::Builder::new()
            .prefix(CAPTURED_PROVIDER_INPUTS_DIRECTORY)
            .tempdir_in(output.path())
            .unwrap();
        let second_path = second.path().to_path_buf();

        assert_ne!(first_path, second_path);
        assert!(first_path.is_dir());
        assert!(second_path.is_dir());
        drop(first);
        assert!(!first_path.exists());
        assert!(second_path.is_dir());
    }

    #[test]
    fn write_baselines_allow_missing_contracts_but_check_baselines_do_not() {
        let root = tempfile::tempdir().unwrap();
        let crate_root = root.path().join("boxdd-sys");
        fs::create_dir_all(crate_root.join("abi")).unwrap();
        let single_relative = contract_relative_path("single").unwrap();
        let single_path = crate_root.join(single_relative);
        fs::write(&single_path, b"single baseline").unwrap();

        let baselines =
            capture_wasm_provider_contract_baselines(root.path(), WasmProviderContractMode::Write)
                .unwrap();
        assert_eq!(
            baselines[0].state,
            WasmProviderContractBaselineState::Existing(b"single baseline".to_vec())
        );
        assert_eq!(
            baselines[1].state,
            WasmProviderContractBaselineState::Missing
        );
        assert!(
            capture_wasm_provider_contract_baselines(root.path(), WasmProviderContractMode::Check)
                .is_err()
        );

        fs::remove_file(&single_path).unwrap();
        let baselines =
            capture_wasm_provider_contract_baselines(root.path(), WasmProviderContractMode::Write)
                .unwrap();
        assert!(
            baselines
                .iter()
                .all(|baseline| baseline.state == WasmProviderContractBaselineState::Missing)
        );
    }

    #[test]
    fn provider_precision_owns_distinct_module_and_build_identity() {
        assert_eq!(ProviderPrecision::Single.module(), "box2d-sys-v2-single");
        assert_eq!(ProviderPrecision::Single.cargo_feature(), None);
        assert_eq!(ProviderPrecision::Double.module(), "box2d-sys-v2-double");
        assert_eq!(
            ProviderPrecision::Double.cargo_feature(),
            Some("double-precision")
        );
        assert_eq!(
            ProviderPrecision::Double.c_define(),
            Some("-DBOX2D_DOUBLE_PRECISION=1")
        );
    }

    #[test]
    fn provider_exports_cover_reviewed_api_not_only_smoke_imports() {
        let output = tempfile::tempdir().unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let path = write_exports_json(
            root,
            output.path(),
            &["b2CreateWorld".to_owned()],
            ProviderPrecision::Single,
        )
        .unwrap();
        let exports: Vec<String> =
            serde_json::from_slice(&fs::read(path).unwrap()).expect("provider exports JSON");

        assert!(exports.iter().any(|name| name == "_b2CreateWorld"));
        assert!(exports.iter().any(|name| name == "_b2World_SetGravity"));
        assert!(
            exports
                .iter()
                .any(|name| name == "_providerHeapBoundaryProbe")
        );
    }

    #[test]
    fn provider_and_identity_probe_share_exact_c_abi_arguments() {
        let digest = "a".repeat(64);
        let mut single = Command::new("emcc");
        configure_provider_c_abi(
            &mut single,
            ProviderPrecision::Single,
            &digest,
            Path::new("public"),
            Path::new("private"),
            Path::new("adapter"),
        );
        let single = single
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            single,
            [
                "--target=wasm32-unknown-emscripten".to_owned(),
                "-std=c17".to_owned(),
                "-O2".to_owned(),
                "-D_POSIX_C_SOURCE=199309L".to_owned(),
                "-DBOX2D_DISABLE_SIMD".to_owned(),
                format!("-DBOXDD_EFFECTIVE_SOURCE_SHA256=\"{digest}\""),
                "-I".to_owned(),
                "public".to_owned(),
                "-I".to_owned(),
                "private".to_owned(),
                "-I".to_owned(),
                "adapter".to_owned(),
            ]
        );

        let mut double = Command::new("emcc");
        configure_provider_c_abi(
            &mut double,
            ProviderPrecision::Double,
            &digest,
            Path::new("public"),
            Path::new("private"),
            Path::new("adapter"),
        );
        let double = double
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(&double[..single.len()], single);
        assert_eq!(double.last().unwrap(), "-DBOX2D_DOUBLE_PRECISION=1");
    }

    #[test]
    fn provider_contract_sources_update_both_manifest_digests() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let paths = WorkspacePaths::new(root);
        let mut manifest = UpstreamManifest::load(&paths).expect("repository manifest");
        let load = |precision: ProviderPrecision| {
            let relative = contract_relative_path(precision.as_str()).unwrap();
            let source = fs::read_to_string(root.join("boxdd-sys").join(relative)).unwrap();
            GeneratedWasmProviderContract {
                precision,
                identity: WasmProviderIdentity::parse(&source).unwrap(),
                source,
            }
        };
        let generated = [
            load(ProviderPrecision::Single),
            load(ProviderPrecision::Double),
        ];

        update_wasm_provider_artifact_digests(&mut manifest, &generated).unwrap();

        for contract in &generated {
            let precision = match contract.precision {
                ProviderPrecision::Single => ManifestPrecision::Single,
                ProviderPrecision::Double => ManifestPrecision::Double,
            };
            let artifact = manifest
                .artifacts
                .iter()
                .find(|artifact| {
                    artifact.kind == ArtifactKind::ProviderIdentity
                        && artifact.precision == Some(precision)
                })
                .expect("provider identity artifact");
            assert_eq!(
                artifact.content_blake3,
                blake3::hash(contract.source.as_bytes())
                    .to_hex()
                    .to_string()
            );
        }
    }

    #[test]
    fn node_runner_materializes_shared_runtime_contract() {
        let output = tempfile::tempdir().unwrap();
        write_node_runner(
            output.path(),
            Path::new("box2d-sys-v2-single.js"),
            Path::new(PROVIDER_SMOKE_WASM),
            &["boxddAdapter_AbiVersion".to_owned()],
            PROVIDER_MODULE,
        )
        .unwrap();

        let runner = fs::read_to_string(output.path().join("run-provider-smoke.mjs")).unwrap();
        assert!(runner.contains("runProviderPhysicsScenario"));
        assert!(runner.contains("initial: 2048"));
        assert!(runner.contains("providerHeapLimitBytes: 67108864"));
        assert!(!runner.contains("verifyProviderAllocatorIdentity"));
        let app_compile = runner.find("WebAssembly.compile(appBytes)").unwrap();
        let app_run = runner.find("await runProviderPhysicsScenario({").unwrap();
        assert!(app_compile < app_run);
        assert!(!runner.contains("boxdd_runtime_step"));
        assert!(!runner.contains("RefreshableMemoryViews"));
        assert_eq!(
            fs::read_to_string(output.path().join(PROVIDER_RUNTIME_CONTRACT_FILE)).unwrap(),
            PROVIDER_RUNTIME_CONTRACT
        );
    }
}
