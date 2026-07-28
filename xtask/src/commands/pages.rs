use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs, io,
    path::{Component, Path, PathBuf},
};
use wasm_bindgen_cli_support::Bindgen;

use crate::{
    Error, Result,
    config::{write_atomic, write_atomic_bytes},
    emscripten_sdk::{QualifiedEmscriptenSdk, SDK_CONTRACT_RELATIVE_PATH, SdkContract},
    provenance_policy::PUBLISHER_REPOSITORY,
    provider_manifest::{self, ADAPTER_ABI_VERSION, RECORDING_CONTRACT_BLAKE3},
    qualified_git::qualified_git_command,
    source_overlay::{adapter_source_sha256, effective_source_identity},
    wasm_provider_contract::{
        COMPILER_TARGET, ENDIANNESS, POINTER_WIDTH, PROVIDER_ABI, SIMD_MODE,
        WasmProviderExpectation, WasmProviderIdentity, contract_relative_path,
    },
};

use super::{
    provider::{
        ProviderPrecision, build_box2d_provider, collect_provider_imports, provider_smoke_dir,
        provider_toolchain_contract, qualified_provider_sdk, write_exports_json,
    },
    support::{
        BuildProfile, QualifiedCargo, WASM_TARGET, add_wasm_app_link_args, copy_file, ensure_file,
        replace_dir_under, run_command,
    },
    upstream_sync::UpdateLock,
};

const PAGES_WASM_OPT_ENV: &str = "BOXDD_PAGES_WASM_OPT";
const PAGES_WASM_DIR: &str = "wasm/generated";
const BEVY_EXAMPLES_DIR: &str = "examples";
const BEVY_WEB_EXAMPLE: &str = "testbed_2d";
const BEVY_WEB_OUT_DIR: &str = "bevy-testbed/generated";
const BEVY_WEB_OUT_NAME: &str = "bevy_boxdd_testbed";
const BEVY_WEB_JS: &str = "bevy_boxdd_testbed.js";
const BEVY_WEB_WASM: &str = "bevy_boxdd_testbed_bg.wasm";
const BEVY_PROVIDER_SHIM: &str = "box2d-provider-shim.js";
const PAGES_RUNTIME_MANIFEST: &str = "wasm/generated/boxdd-pages-runtime-v2.json";
const PAGES_RUNTIME_SCHEMA: &str = "boxdd-pages-runtime-v2";
const PAGES_RUNTIME_SCHEMA_VERSION: u64 = 2;
const PAGES_PUBLISHER_WORKFLOW: &str = ".github/workflows/pages.yml";
const JAVASCRIPT_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PagesAssetSpec {
    role: &'static str,
    path: &'static str,
}

const PAGES_RUNTIME_ASSETS: [PagesAssetSpec; 5] = [
    PagesAssetSpec {
        role: "provider_js",
        path: "wasm/generated/box2d-sys-v1-single.js",
    },
    PagesAssetSpec {
        role: "provider_wasm",
        path: "wasm/generated/box2d-sys-v1-single.wasm",
    },
    PagesAssetSpec {
        role: "app_js",
        path: "bevy-testbed/generated/bevy_boxdd_testbed.js",
    },
    PagesAssetSpec {
        role: "app_wasm",
        path: "bevy-testbed/generated/bevy_boxdd_testbed_bg.wasm",
    },
    PagesAssetSpec {
        role: "provider_shim_js",
        path: "bevy-testbed/generated/box2d-provider-shim.js",
    },
];

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct PagesRuntimeManifest {
    schema_version: u64,
    schema: String,
    publisher_repository: String,
    publisher_workflow: String,
    provider: String,
    provider_abi: String,
    adapter_abi_version: u64,
    crate_version: String,
    source_commit: String,
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    adapter_source_sha256: String,
    emscripten_sdk_contract_sha256: String,
    wasm_provider_contract_sha256: String,
    recording_contract_blake3: String,
    precision: String,
    target: String,
    assets: Vec<PagesRuntimeAsset>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct PagesRuntimeAsset {
    role: String,
    path: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PagesRuntimeIdentity {
    source_commit: String,
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    adapter_source_sha256: String,
    emscripten_sdk_contract_sha256: String,
    wasm_provider_contract_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PagesLoaderTrust {
    manifest_sha256: String,
    schema_version: u64,
    schema: String,
    publisher_repository: String,
    publisher_workflow: String,
    provider: String,
    provider_abi: String,
    adapter_abi_version: u64,
    crate_version: String,
    source_commit: String,
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    adapter_source_sha256: String,
    emscripten_sdk_contract_sha256: String,
    wasm_provider_contract_sha256: String,
    recording_contract_blake3: String,
    precision: String,
    target: String,
}

struct RegistrySample {
    id: String,
    category: String,
    name: String,
    description: String,
    upstream: Vec<RegistryUpstreamSample>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct RegistryUpstreamSample {
    category: String,
    name: String,
    mode: String,
}

#[derive(Debug, Default)]
struct PageSampleBuilder {
    id: Option<String>,
    category: Option<String>,
    name: Option<String>,
    description: Option<String>,
    upstream: Vec<RegistryUpstreamSample>,
}

#[derive(Debug, Default)]
struct UpstreamSampleBuilder {
    category: Option<String>,
    name: Option<String>,
    mode: Option<String>,
}

struct BevyWebArtifacts {
    out_dir: PathBuf,
    imports: Vec<String>,
}

#[derive(Copy, Clone)]
enum ExampleIndexLocation {
    Root,
    ExamplesDirectory,
}

pub(crate) fn build_pages_wasm(root: &Path) -> Result<()> {
    let _lock = UpdateLock::acquire(root)?;
    ensure_pages_build_inputs_clean(root)?;
    let cargo = QualifiedCargo::qualify(root)?;
    let target_dir = cargo.target_dir().to_path_buf();
    let precision = ProviderPrecision::from_env()?;
    validate_pages_precision(precision)?;
    let sdk = qualified_provider_sdk()?;
    let identity = pages_runtime_identity(root, precision)?;
    if identity.emscripten_sdk_contract_sha256 != sdk.contract_sha256() {
        return Err(Error::Message(
            "qualified Emscripten SDK contract does not match the Pages source identity".to_owned(),
        ));
    }
    provider_toolchain_contract()?;
    generate_pages(root)?;
    let bevy_artifacts = build_bevy_web_app(root, &target_dir, &cargo, &sdk, precision)?;
    let out_dir = provider_smoke_dir(&target_dir);
    let exports = write_exports_json(&out_dir, &bevy_artifacts.imports)?;
    let provider = build_box2d_provider(root, &out_dir, &exports, &sdk, precision)?;
    let provider_wasm = provider.with_extension("wasm");
    ensure_file(&provider, "Box2D provider module")?;
    ensure_file(&provider_wasm, "Box2D provider wasm")?;
    optimize_wasm_if_available(&sdk, &provider_wasm, "Box2D provider wasm")?;

    let generated = pages_wasm_generated_dir(root);
    replace_dir_under(&generated, &root.join("docs/pages"))?;
    copy_file(
        &provider,
        &generated.join(format!("{}.js", precision.module())),
    )?;
    copy_file(
        &provider_wasm,
        &generated.join(format!("{}.wasm", precision.module())),
    )?;
    copy_bevy_web_artifacts(root, &bevy_artifacts)?;
    ensure_pages_source_state(root, precision, &identity)?;
    sdk.revalidate().map_err(Error::Message)?;
    let (manifest, manifest_sha256) = write_pages_runtime_manifest(root, precision, &identity)?;
    let trust = PagesLoaderTrust::from_manifest(&manifest, manifest_sha256);
    write_bevy_testbed_loader(root, Some(&trust))?;
    ensure_pages_source_state(root, precision, &identity)?;

    println!(
        "pages wasm assets ready: {} and {} ({} Bevy imports)",
        generated.display(),
        pages_bevy_generated_dir(root).display(),
        bevy_artifacts.imports.len()
    );
    Ok(())
}

impl PagesRuntimeManifest {
    fn parse(bytes: &[u8]) -> Result<Self> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|error| {
            Error::Message(format!(
                "Pages runtime manifest is not valid strict JSON: {error}"
            ))
        })?;
        if manifest.render()? != bytes {
            return Err(Error::Message(
                "Pages runtime manifest must use its canonical byte representation".to_owned(),
            ));
        }
        Ok(manifest)
    }

    fn render(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|error| {
            Error::Message(format!(
                "failed to serialize canonical Pages runtime manifest: {error}"
            ))
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

impl PagesLoaderTrust {
    fn from_manifest(manifest: &PagesRuntimeManifest, manifest_sha256: String) -> Self {
        Self {
            manifest_sha256,
            schema_version: manifest.schema_version,
            schema: manifest.schema.clone(),
            publisher_repository: manifest.publisher_repository.clone(),
            publisher_workflow: manifest.publisher_workflow.clone(),
            provider: manifest.provider.clone(),
            provider_abi: manifest.provider_abi.clone(),
            adapter_abi_version: manifest.adapter_abi_version,
            crate_version: manifest.crate_version.clone(),
            source_commit: manifest.source_commit.clone(),
            upstream_sha: manifest.upstream_sha.clone(),
            source_tree: manifest.source_tree.clone(),
            effective_source_sha256: manifest.effective_source_sha256.clone(),
            adapter_source_sha256: manifest.adapter_source_sha256.clone(),
            emscripten_sdk_contract_sha256: manifest.emscripten_sdk_contract_sha256.clone(),
            wasm_provider_contract_sha256: manifest.wasm_provider_contract_sha256.clone(),
            recording_contract_blake3: manifest.recording_contract_blake3.clone(),
            precision: manifest.precision.clone(),
            target: manifest.target.clone(),
        }
    }
}

fn pages_runtime_manifest_path(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join(PAGES_RUNTIME_MANIFEST)
}

fn pages_runtime_identity(
    root: &Path,
    precision: ProviderPrecision,
) -> Result<PagesRuntimeIdentity> {
    let effective = effective_source_identity(&root.join("boxdd-sys")).map_err(|error| {
        Error::Message(format!(
            "failed to identify Pages provider effective sources: {error}"
        ))
    })?;
    let adapter_source_sha256 =
        adapter_source_sha256(&root.join("boxdd-sys")).map_err(|error| {
            Error::Message(format!(
                "failed to identify Pages provider adapter: {error}"
            ))
        })?;
    let wasm_provider_contract_sha256 = pages_provider_contract_sha256(
        root,
        precision,
        &effective.upstream_sha,
        &effective.source_tree,
        &effective.effective_source_sha256,
        &adapter_source_sha256,
    )?;
    Ok(PagesRuntimeIdentity {
        source_commit: pages_source_commit(root)?,
        upstream_sha: effective.upstream_sha,
        source_tree: effective.source_tree,
        effective_source_sha256: effective.effective_source_sha256,
        adapter_source_sha256,
        emscripten_sdk_contract_sha256: pages_sdk_contract_sha256(root)?,
        wasm_provider_contract_sha256,
    })
}

fn pages_provider_contract_sha256(
    root: &Path,
    precision: ProviderPrecision,
    upstream_sha: &str,
    source_tree: &str,
    effective_source_sha256: &str,
    adapter_source_sha256: &str,
) -> Result<String> {
    let relative = contract_relative_path(precision.as_str()).map_err(Error::Message)?;
    let bindings = root
        .join("boxdd-sys/src")
        .join(precision.wasm_bindings_file());
    let bindings_sha256 = provider_manifest::sha256_file(&bindings).map_err(Error::Message)?;
    let (_, source) = WasmProviderIdentity::load_with_source_bytes(
        &root.join("boxdd-sys"),
        Path::new(relative),
        &WasmProviderExpectation {
            provider_abi: PROVIDER_ABI,
            target: WASM_TARGET,
            compiler_target: COMPILER_TARGET,
            precision: precision.as_str(),
            upstream_sha,
            source_tree,
            effective_source_sha256,
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256,
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3,
            validation_enabled: false,
            simd: SIMD_MODE,
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS,
            bindings_sha256: &bindings_sha256,
        },
    )
    .map_err(Error::Message)?;
    Ok(provider_manifest::sha256_bytes(&source))
}

fn pages_sdk_contract_sha256(root: &Path) -> Result<String> {
    let path = root.join("xtask").join(SDK_CONTRACT_RELATIVE_PATH);
    let metadata = fs::symlink_metadata(&path).map_err(|source| Error::io(&path, source))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::Message(format!(
            "Pages Emscripten SDK contract must be a regular non-symlink file: {}",
            path.display()
        )));
    }
    let bytes = fs::read(&path).map_err(|source| Error::io(&path, source))?;
    let source = std::str::from_utf8(&bytes).map_err(|error| {
        Error::Message(format!(
            "Pages Emscripten SDK contract is not UTF-8: {error}"
        ))
    })?;
    SdkContract::parse(source).map_err(|error| {
        Error::Message(format!("Pages Emscripten SDK contract is invalid: {error}"))
    })?;
    Ok(provider_manifest::sha256_bytes(&bytes))
}

fn pages_source_commit(root: &Path) -> Result<String> {
    let commit = git_stdout(
        root,
        &["rev-parse", "--verify", "HEAD^{commit}"],
        "resolve Pages source commit",
    )?;
    validate_lower_hex("Pages source commit", &commit, 40)?;
    if env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true") {
        let github_repository = required_environment("GITHUB_REPOSITORY")?;
        if github_repository != PUBLISHER_REPOSITORY {
            return Err(Error::Message(format!(
                "Pages publisher repository {github_repository:?} does not match {PUBLISHER_REPOSITORY:?}"
            )));
        }
        let workflow_ref = required_environment("GITHUB_WORKFLOW_REF")?;
        let expected_workflow_ref =
            format!("{PUBLISHER_REPOSITORY}/{PAGES_PUBLISHER_WORKFLOW}@refs/heads/main");
        if workflow_ref != expected_workflow_ref {
            return Err(Error::Message(format!(
                "Pages workflow identity {workflow_ref:?} does not match {expected_workflow_ref:?}"
            )));
        }
        let github_sha = required_environment("GITHUB_SHA")?;
        validate_lower_hex("GITHUB_SHA", &github_sha, 40)?;
        if github_sha != commit {
            return Err(Error::Message(format!(
                "Pages checkout commit {commit} does not match GITHUB_SHA {github_sha}"
            )));
        }
    }
    Ok(commit)
}

fn required_environment(name: &str) -> Result<String> {
    env::var(name)
        .map_err(|_| Error::Message(format!("GitHub Pages requires non-empty {name}")))
        .and_then(|value| {
            if value.is_empty() {
                Err(Error::Message(format!(
                    "GitHub Pages requires non-empty {name}"
                )))
            } else {
                Ok(value)
            }
        })
}

fn ensure_pages_build_inputs_clean(root: &Path) -> Result<()> {
    let status = git_stdout(
        root,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--",
            ".",
            ":(exclude)docs/pages/**",
        ],
        "inspect Pages build inputs",
    )?;
    if status.is_empty() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "Pages runtime assets require commit-bound clean inputs outside docs/pages; commit or remove these changes first:\n{status}"
        )))
    }
}

fn ensure_pages_source_state(
    root: &Path,
    precision: ProviderPrecision,
    expected: &PagesRuntimeIdentity,
) -> Result<()> {
    ensure_pages_build_inputs_clean(root)?;
    let actual = pages_runtime_identity(root, precision)?;
    if &actual == expected {
        Ok(())
    } else {
        Err(Error::Message(
            "Pages source identity changed while runtime assets were being built; discard the generated output and rebuild from one clean commit"
                .to_owned(),
        ))
    }
}

fn git_stdout(root: &Path, args: &[&str], label: &str) -> Result<String> {
    let mut command = qualified_git_command().map_err(Error::Message)?;
    let output = command
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| Error::Message(format!("failed to {label}: {error}")))?;
    if !output.status.success() {
        return Err(Error::Message(format!(
            "failed to {label}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| Error::Message(format!("{label} returned non-UTF-8 output: {error}")))
}

fn write_pages_runtime_manifest(
    root: &Path,
    precision: ProviderPrecision,
    identity: &PagesRuntimeIdentity,
) -> Result<(PagesRuntimeManifest, String)> {
    let pages_dir = root.join("docs/pages");
    let assets = PAGES_RUNTIME_ASSETS
        .iter()
        .map(|spec| pages_runtime_asset(&pages_dir, *spec))
        .collect::<Result<Vec<_>>>()?;
    let manifest = PagesRuntimeManifest {
        schema_version: PAGES_RUNTIME_SCHEMA_VERSION,
        schema: PAGES_RUNTIME_SCHEMA.to_owned(),
        publisher_repository: PUBLISHER_REPOSITORY.to_owned(),
        publisher_workflow: PAGES_PUBLISHER_WORKFLOW.to_owned(),
        provider: "wasm-runtime".to_owned(),
        provider_abi: PROVIDER_ABI.to_owned(),
        adapter_abi_version: ADAPTER_ABI_VERSION,
        crate_version: env!("CARGO_PKG_VERSION").to_owned(),
        source_commit: identity.source_commit.clone(),
        upstream_sha: identity.upstream_sha.clone(),
        source_tree: identity.source_tree.clone(),
        effective_source_sha256: identity.effective_source_sha256.clone(),
        adapter_source_sha256: identity.adapter_source_sha256.clone(),
        emscripten_sdk_contract_sha256: identity.emscripten_sdk_contract_sha256.clone(),
        wasm_provider_contract_sha256: identity.wasm_provider_contract_sha256.clone(),
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
        precision: precision.as_str().to_owned(),
        target: WASM_TARGET.to_owned(),
        assets,
    };
    validate_pages_runtime_manifest_identity(&manifest, identity)?;
    let bytes = manifest.render()?;
    let path = pages_runtime_manifest_path(root);
    ensure_pages_output_parent(root, &path)?;
    write_atomic_bytes(&path, &bytes)?;
    Ok((manifest, provider_manifest::sha256_bytes(&bytes)))
}

fn pages_runtime_asset(pages_dir: &Path, spec: PagesAssetSpec) -> Result<PagesRuntimeAsset> {
    let path = pages_dir.join(spec.path);
    let metadata = fs::symlink_metadata(&path).map_err(|source| Error::io(&path, source))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::Message(format!(
            "Pages runtime asset must be a regular non-symlink file: {}",
            path.display()
        )));
    }
    validate_pages_asset_byte_length(&path, metadata.len())?;
    Ok(PagesRuntimeAsset {
        role: spec.role.to_owned(),
        path: spec.path.to_owned(),
        byte_length: metadata.len(),
        sha256: provider_manifest::sha256_file(&path).map_err(Error::Message)?,
    })
}

fn validate_pages_asset_byte_length(path: &Path, byte_length: u64) -> Result<()> {
    if byte_length == 0 || byte_length > JAVASCRIPT_MAX_SAFE_INTEGER {
        return Err(Error::Message(format!(
            "Pages runtime asset byte length must be within 1..={JAVASCRIPT_MAX_SAFE_INTEGER}: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_pages_runtime_manifest_identity(
    manifest: &PagesRuntimeManifest,
    identity: &PagesRuntimeIdentity,
) -> Result<()> {
    if manifest.schema_version != PAGES_RUNTIME_SCHEMA_VERSION
        || manifest.schema != PAGES_RUNTIME_SCHEMA
    {
        return Err(Error::Message(format!(
            "unsupported Pages runtime manifest schema: version={} name={:?}",
            manifest.schema_version, manifest.schema
        )));
    }
    if manifest.provider != "wasm-runtime"
        || manifest.publisher_repository != PUBLISHER_REPOSITORY
        || manifest.publisher_workflow != PAGES_PUBLISHER_WORKFLOW
        || manifest.provider_abi != PROVIDER_ABI
        || manifest.adapter_abi_version != ADAPTER_ABI_VERSION
        || manifest.crate_version != env!("CARGO_PKG_VERSION")
        || manifest.precision != ProviderPrecision::Single.as_str()
        || manifest.target != WASM_TARGET
        || manifest.recording_contract_blake3 != RECORDING_CONTRACT_BLAKE3
    {
        return Err(Error::Message(
            "Pages runtime manifest ABI, crate, precision, target, or recording identity does not match this build"
                .to_owned(),
        ));
    }
    for (label, actual, expected, digits) in [
        (
            "source_commit",
            manifest.source_commit.as_str(),
            identity.source_commit.as_str(),
            40,
        ),
        (
            "upstream_sha",
            manifest.upstream_sha.as_str(),
            identity.upstream_sha.as_str(),
            40,
        ),
        (
            "source_tree",
            manifest.source_tree.as_str(),
            identity.source_tree.as_str(),
            40,
        ),
        (
            "effective_source_sha256",
            manifest.effective_source_sha256.as_str(),
            identity.effective_source_sha256.as_str(),
            64,
        ),
        (
            "adapter_source_sha256",
            manifest.adapter_source_sha256.as_str(),
            identity.adapter_source_sha256.as_str(),
            64,
        ),
        (
            "emscripten_sdk_contract_sha256",
            manifest.emscripten_sdk_contract_sha256.as_str(),
            identity.emscripten_sdk_contract_sha256.as_str(),
            64,
        ),
        (
            "wasm_provider_contract_sha256",
            manifest.wasm_provider_contract_sha256.as_str(),
            identity.wasm_provider_contract_sha256.as_str(),
            64,
        ),
    ] {
        validate_lower_hex(label, actual, digits)?;
        if actual != expected {
            return Err(Error::Message(format!(
                "Pages runtime manifest {label} {actual} does not match {expected}"
            )));
        }
    }
    Ok(())
}

fn validate_lower_hex(label: &str, value: &str, digits: usize) -> Result<()> {
    if value.len() == digits
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} must be exactly {digits} lowercase hexadecimal characters"
        )))
    }
}

fn validate_pages_precision(precision: ProviderPrecision) -> Result<()> {
    if precision == ProviderPrecision::Single {
        Ok(())
    } else {
        Err(Error::Message(
            "GitHub Pages currently qualifies only BOXDD_WASM_PRECISION=single; use provider-smoke to qualify the double-precision runtime"
                .to_owned(),
        ))
    }
}

fn pages_wasm_generated_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join(PAGES_WASM_DIR)
}

fn pages_bevy_generated_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join(BEVY_WEB_OUT_DIR)
}

fn pages_bevy_testbed_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pages").join("bevy-testbed")
}

fn build_bevy_web_app(
    root: &Path,
    target_dir: &Path,
    cargo: &QualifiedCargo,
    sdk: &QualifiedEmscriptenSdk,
    precision: ProviderPrecision,
) -> Result<BevyWebArtifacts> {
    let out_dir = target_dir.join("boxdd-bevy-testbed-web");
    replace_dir_under(&out_dir, target_dir)?;

    let profile = BuildProfile::for_pages()?;
    let mut command = cargo.command(root)?;
    command
        .arg("rustc")
        .arg("--locked")
        .arg("-p")
        .arg("bevy_boxdd")
        .arg("--example")
        .arg(BEVY_WEB_EXAMPLE)
        .arg("--target")
        .arg(WASM_TARGET)
        .args(profile.cargo_args())
        .current_dir(root)
        .env("BOXDD_SYS_PROVIDER", "wasm-provider");
    if let Some(feature) = precision.cargo_feature() {
        command
            .arg("--features")
            .arg(format!("bevy_boxdd/{feature}"));
    }
    add_wasm_app_link_args(&mut command, &[]);
    run_command(
        &mut command,
        &format!("build Bevy testbed wasm ({})", profile.label()),
    )?;

    let wasm = target_dir
        .join(WASM_TARGET)
        .join(profile.target_dir())
        .join("examples")
        .join(format!("{BEVY_WEB_EXAMPLE}.wasm"));
    ensure_file(&wasm, "Bevy testbed wasm")?;

    let mut bindgen = Bindgen::new();
    bindgen
        .input_path(&wasm)
        .out_name(BEVY_WEB_OUT_NAME)
        .typescript(true)
        .web(true)
        .map_err(|error| Error::Message(format!("configure wasm-bindgen: {error}")))?
        .generate(&out_dir)
        .map_err(|error| Error::Message(format!("run wasm-bindgen for Bevy testbed: {error}")))?;

    patch_bevy_bindgen_imports(&out_dir.join(BEVY_WEB_JS), precision.module())?;
    let bevy_wasm = out_dir.join(BEVY_WEB_WASM);
    optimize_wasm_if_available(sdk, &bevy_wasm, "Bevy testbed wasm")?;
    let imports = collect_provider_imports(&bevy_wasm, precision.module())?;
    write_browser_provider_shim(&out_dir, &imports)?;

    Ok(BevyWebArtifacts { out_dir, imports })
}

fn patch_bevy_bindgen_imports(js: &Path, provider_module: &str) -> Result<()> {
    let source = fs::read_to_string(js).map_err(|source| Error::io(js, source))?;
    let patched_imports = rewrite_bevy_bindgen_provider_imports(&source, provider_module)
        .map_err(|error| Error::Message(format!("{error}: {}", js.display())))?;
    let patched = patched_imports.replace(
        "    wasm = instance.exports;\n",
        "    wasm = instance.exports;\n    if (typeof import1.setBoxddAppExports === \"function\") {\n        import1.setBoxddAppExports(wasm);\n    }\n",
    );
    if patched == patched_imports {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not assign instance exports: {}",
            js.display()
        )));
    }
    let decode_patched = patched.replace(
        "cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len))",
        "cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len))",
    );
    if decode_patched == patched {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not decode strings from wasm memory: {}",
            js.display()
        )));
    }
    write_atomic(js, &decode_patched)
}

fn rewrite_bevy_bindgen_provider_imports(source: &str, provider_module: &str) -> Result<String> {
    let provider_import = format!("from \"{provider_module}\"");
    let provider_suffix = format!(" {provider_import}");
    let mut import_lines = Vec::new();
    let mut import_bindings = BTreeSet::new();

    for (line_number, source_line) in source.lines().enumerate() {
        if !source_line.contains(&provider_import) {
            continue;
        }

        let line = source_line.strip_suffix('\r').unwrap_or(source_line);
        let line = line.strip_suffix(';').unwrap_or(line);
        let Some(numeric_suffix) = line
            .strip_prefix("import * as import")
            .and_then(|line| line.strip_suffix(&provider_suffix))
        else {
            return Err(Error::Message(format!(
                "wasm-bindgen output has an unsupported provider import at line {}; expected `import * as importN {provider_import}`",
                line_number + 1
            )));
        };
        if numeric_suffix.is_empty() || !numeric_suffix.as_bytes().iter().all(u8::is_ascii_digit) {
            return Err(Error::Message(format!(
                "wasm-bindgen output has an unsupported provider namespace at line {}; expected `importN`",
                line_number + 1
            )));
        }

        let binding = format!("import{numeric_suffix}");
        if !import_bindings.insert(binding) {
            return Err(Error::Message(format!(
                "wasm-bindgen output repeats a provider namespace at line {}",
                line_number + 1
            )));
        }
        import_lines.push(line_number);
    }

    if import_lines.is_empty() {
        return Err(Error::Message(format!(
            "wasm-bindgen output does not import {provider_module}"
        )));
    }
    if import_lines
        .iter()
        .enumerate()
        .any(|(offset, line_number)| *line_number != import_lines[0] + offset)
    {
        return Err(Error::Message(format!(
            "wasm-bindgen provider imports for {provider_module} must form one contiguous namespace declaration block"
        )));
    }
    if !import_bindings.contains("import1") {
        return Err(Error::Message(format!(
            "wasm-bindgen provider imports for {provider_module} must include import1 for the application export handoff"
        )));
    }

    Ok(source.replace(
        &provider_import,
        &format!("from \"./{BEVY_PROVIDER_SHIM}\""),
    ))
}

fn write_browser_provider_shim(out_dir: &Path, imports: &[String]) -> Result<PathBuf> {
    let exports = imports
        .iter()
        .map(|name| {
            format!("export function {name}(...args) {{ return callProvider(\"{name}\", args); }}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let shim = format!(
        r#"let provider;
let providerCalls = 0;
let stepCalls = 0;

export function setBox2dProvider(nextProvider) {{
  const refreshMemoryViews = nextProvider?.boxddRefreshMemoryViews;
  if (typeof refreshMemoryViews !== "function") {{
    throw new Error("Box2D provider does not expose boxddRefreshMemoryViews");
  }}
  const heap = refreshMemoryViews();
  if (!(heap instanceof Uint8Array) || heap !== nextProvider.HEAPU8) {{
    throw new Error("Box2D provider does not expose its canonical Emscripten HEAPU8 view");
  }}
  provider = nextProvider;
}}

export function setBoxddAppExports(exports) {{
  if (!provider) {{
    throw new Error("Box2D provider is not initialized");
  }}
  provider.boxddAppExports = exports;
}}

function resolveProviderExport(name) {{
  if (!provider) {{
    throw new Error("Box2D provider is not initialized");
  }}
  const exported = provider[`_${{name}}`] || provider[name];
  if (typeof exported !== "function") {{
    throw new Error(`Box2D provider is missing export ${{name}}`);
  }}
  return exported;
}}

function callProvider(name, args) {{
  provider.boxddRefreshMemoryViews();
  providerCalls += 1;
  if (name === "b2World_Step") {{
    stepCalls += 1;
  }}
  return resolveProviderExport(name)(...args);
}}

export function boxddProviderRuntimeEvidence() {{
  return Object.freeze({{ providerCalls, stepCalls }});
}}

{exports}
"#
    );
    let path = out_dir.join(BEVY_PROVIDER_SHIM);
    write_atomic(&path, &shim)?;
    Ok(path)
}

fn copy_bevy_web_artifacts(root: &Path, artifacts: &BevyWebArtifacts) -> Result<()> {
    let generated = pages_bevy_generated_dir(root);
    replace_dir_under(&generated, &root.join("docs/pages"))?;

    for file in [BEVY_WEB_JS, BEVY_WEB_WASM, BEVY_PROVIDER_SHIM] {
        copy_file(&artifacts.out_dir.join(file), &generated.join(file))?;
    }

    Ok(())
}

fn optimize_wasm_if_available(
    sdk: &QualifiedEmscriptenSdk,
    wasm: &Path,
    label: &str,
) -> Result<()> {
    if !pages_wasm_opt_enabled() {
        println!("wasm-opt skipped for {label}: disabled by {PAGES_WASM_OPT_ENV}");
        return Ok(());
    }

    let before = file_size(wasm)?;
    let tmp = wasm.with_extension("wasm-opt.tmp");
    remove_optimized_wasm_temp(&tmp)?;
    let mut command = sdk.wasm_opt_command().map_err(Error::Message)?;
    command
        .arg("-Oz")
        .arg("--enable-bulk-memory")
        .arg("--enable-bulk-memory-opt")
        .arg("--enable-nontrapping-float-to-int")
        .arg("--strip-debug")
        .arg("--strip-producers")
        .arg(wasm)
        .arg("-o")
        .arg(&tmp);
    let optimization = (|| {
        run_command(&mut command, &format!("optimize {label} with wasm-opt"))?;
        sdk.revalidate().map_err(Error::Message)?;
        ensure_file(&tmp, "wasm-opt output")?;
        fs::copy(&tmp, wasm).map_err(|source| Error::io(wasm, source))?;
        Ok(())
    })();
    if let Err(error) = optimization {
        if let Err(cleanup) = remove_optimized_wasm_temp(&tmp) {
            return Err(Error::Message(format!(
                "{error}; failed to remove wasm-opt temporary output: {cleanup}"
            )));
        }
        return Err(error);
    }
    remove_optimized_wasm_temp(&tmp)?;

    let after = file_size(wasm)?;
    let saved = before.saturating_sub(after);
    let pct = if before == 0 {
        0.0
    } else {
        saved as f64 * 100.0 / before as f64
    };
    println!(
        "{label} optimized: {} -> {} ({saved} bytes saved, {pct:.1}%)",
        format_bytes(before),
        format_bytes(after)
    );
    Ok(())
}

fn remove_optimized_wasm_temp(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io(path, error)),
    }
}

fn pages_wasm_opt_enabled() -> bool {
    !matches!(
        env::var(PAGES_WASM_OPT_ENV).ok().as_deref(),
        Some("0" | "false" | "False" | "FALSE" | "off" | "OFF" | "no" | "NO")
    )
}

fn file_size(path: &Path) -> Result<u64> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|source| Error::io(path, source))
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= MIB {
        format!("{:.2} MiB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.2} KiB", bytes_f / KIB)
    } else {
        format!("{bytes} B")
    }
}
pub(crate) fn generate_pages(root: &Path) -> Result<()> {
    let samples = read_testbed_registry(root)?;
    let pages = expected_bevy_pages(root, &samples);
    let pages_dir = root.join("docs/pages");
    let examples_dir = pages_dir.join("examples");

    clear_pages_runtime_assets(root)?;
    reset_generated_examples_dir(&pages_dir, &examples_dir)?;
    for (path, html) in pages {
        ensure_pages_output_parent(root, &path)?;
        write_atomic(&path, &html)?;
    }
    write_bevy_testbed_loader(root, None)?;
    remove_file_if_exists(&pages_dir.join("wasm/index.html"))?;
    remove_file_if_exists(&pages_dir.join("wasm/loader.js"))?;

    println!(
        "generated pages: {} Bevy WASM examples under {}",
        samples.len(),
        pages_dir.display()
    );
    Ok(())
}

fn clear_pages_runtime_assets(root: &Path) -> Result<()> {
    let pages_dir = root.join("docs/pages");
    for generated in [
        pages_wasm_generated_dir(root),
        pages_bevy_generated_dir(root),
    ] {
        replace_dir_under(&generated, &pages_dir)?;
        fs::remove_dir(&generated).map_err(|source| Error::io(&generated, source))?;
    }
    Ok(())
}

fn expected_bevy_pages(root: &Path, samples: &[RegistrySample]) -> BTreeMap<PathBuf, String> {
    let pages_dir = root.join("docs/pages");
    let mut pages = BTreeMap::new();
    pages.insert(
        pages_dir.join("index.html"),
        bevy_example_index_page(samples, ExampleIndexLocation::Root),
    );
    pages.insert(
        pages_dir.join(BEVY_EXAMPLES_DIR).join("index.html"),
        bevy_example_index_page(samples, ExampleIndexLocation::ExamplesDirectory),
    );
    pages.insert(
        pages_bevy_testbed_dir(root).join("index.html"),
        bevy_testbed_page(),
    );
    for sample in samples {
        pages.insert(
            pages_dir
                .join(BEVY_EXAMPLES_DIR)
                .join(&sample.id)
                .join("index.html"),
            bevy_example_page(sample),
        );
    }
    pages
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io(path, source)),
    }
}

fn write_bevy_testbed_loader(root: &Path, trust: Option<&PagesLoaderTrust>) -> Result<()> {
    let path = pages_bevy_testbed_dir(root).join("loader.js");
    ensure_pages_output_parent(root, &path)?;
    write_atomic(&path, &bevy_testbed_loader_js(trust))
}

fn ensure_pages_output_parent(root: &Path, path: &Path) -> Result<()> {
    pages_output_parent(root, path, true)
}

fn require_pages_output_parent(root: &Path, path: &Path) -> Result<()> {
    pages_output_parent(root, path, false)
}

fn pages_output_parent(root: &Path, path: &Path, create_missing: bool) -> Result<()> {
    let pages_dir = root.join("docs/pages");
    let parent = path.parent().ok_or_else(|| {
        Error::Message(format!(
            "Pages output has no parent directory: {}",
            path.display()
        ))
    })?;
    let relative = parent.strip_prefix(&pages_dir).map_err(|_| {
        Error::Message(format!(
            "Pages output must remain under {}: {}",
            pages_dir.display(),
            path.display()
        ))
    })?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Message(format!(
            "Pages output path is not canonical: {}",
            path.display()
        )));
    }

    require_real_directory(&pages_dir, "Pages output root")?;
    let canonical_pages = pages_dir
        .canonicalize()
        .map_err(|source| Error::io(&pages_dir, source))?;
    let mut current = pages_dir;
    for component in relative.components() {
        let Component::Normal(component) = component else {
            unreachable!("relative Pages components were validated");
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            }
            Ok(_) => {
                return Err(Error::Message(format!(
                    "Pages output directory tree contains a symlink or non-directory: {}",
                    current.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound && create_missing => {
                fs::create_dir(&current).map_err(|source| Error::io(&current, source))?;
            }
            Err(source) => return Err(Error::io(&current, source)),
        }
        let canonical = current
            .canonicalize()
            .map_err(|source| Error::io(&current, source))?;
        if !canonical.starts_with(&canonical_pages) {
            return Err(Error::Message(format!(
                "Pages output directory escaped {}: {}",
                canonical_pages.display(),
                canonical.display()
            )));
        }
    }
    Ok(())
}

fn require_real_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{label} must be a real directory: {}",
            path.display()
        )))
    }
}

fn read_testbed_registry(root: &Path) -> Result<Vec<RegistrySample>> {
    let scenes = root
        .join("bevy_boxdd")
        .join("examples")
        .join("testbed_2d")
        .join("scenes.rs");
    let source = fs::read_to_string(&scenes).map_err(|source| Error::io(&scenes, source))?;
    let mut samples = Vec::new();
    let mut current: Option<PageSampleBuilder> = None;
    let mut current_upstream: Option<UpstreamSampleBuilder> = None;
    let mut in_registry = false;

    for line in source.lines() {
        if line.contains("pub const SCENE_REGISTRY") {
            in_registry = true;
            continue;
        }
        if !in_registry {
            continue;
        }

        let trimmed = line.trim();
        if let Some(upstream) = current_upstream.as_mut() {
            read_upstream_fields(upstream, trimmed);
            if trimmed == "}," || trimmed.ends_with("},") || trimmed.ends_with("}],") {
                let upstream = current_upstream
                    .take()
                    .expect("upstream builder should be present");
                current
                    .as_mut()
                    .ok_or_else(|| {
                        Error::Message(format!(
                            "upstream sample outside registry entry in {}",
                            scenes.display()
                        ))
                    })?
                    .upstream
                    .push(upstream.build()?);
            }
            continue;
        }
        if trimmed == "];" {
            break;
        }
        if trimmed.starts_with("TestbedSceneMetadata {") {
            current = Some(PageSampleBuilder::default());
            continue;
        }
        if trimmed == "}," {
            let builder = current.take().ok_or_else(|| {
                Error::Message(format!(
                    "unexpected registry entry terminator in {}",
                    scenes.display()
                ))
            })?;
            samples.push(builder.build()?);
            continue;
        }

        let Some(builder) = current.as_mut() else {
            continue;
        };
        if trimmed.contains("UpstreamSampleRef {") {
            let mut upstream = UpstreamSampleBuilder::default();
            read_upstream_fields(&mut upstream, trimmed);
            if trimmed.ends_with("},") || trimmed.ends_with("}],") {
                builder.upstream.push(upstream.build()?);
            } else {
                current_upstream = Some(upstream);
            }
        } else if let Some(value) = extract_string_field(trimmed, "id") {
            builder.id = Some(value);
        } else if let Some(value) = extract_string_field(trimmed, "category") {
            builder.category = Some(value);
        } else if let Some(value) = extract_string_field(trimmed, "name") {
            builder.name = Some(value);
        } else if let Some(value) = extract_string_field(trimmed, "description") {
            builder.description = Some(value);
        }
    }

    validate_registry_catalog(&samples)?;
    Ok(samples)
}

impl PageSampleBuilder {
    fn build(self) -> Result<RegistrySample> {
        Ok(RegistrySample {
            id: required_registry_field(self.id, "id")?,
            category: required_registry_field(self.category, "category")?,
            name: required_registry_field(self.name, "name")?,
            description: required_registry_field(self.description, "description")?,
            upstream: self.upstream,
        })
    }
}

impl UpstreamSampleBuilder {
    fn build(self) -> Result<RegistryUpstreamSample> {
        Ok(RegistryUpstreamSample {
            category: required_registry_field(self.category, "upstream.category")?,
            name: required_registry_field(self.name, "upstream.name")?,
            mode: required_registry_field(self.mode, "upstream.mode")?,
        })
    }
}

fn required_registry_field(value: Option<String>, field: &str) -> Result<String> {
    value.ok_or_else(|| Error::Message(format!("SCENE_REGISTRY entry is missing `{field}`")))
}

fn read_upstream_fields(builder: &mut UpstreamSampleBuilder, line: &str) {
    if let Some(value) = extract_string_field(line, "category") {
        builder.category = Some(value);
    }
    if let Some(value) = extract_string_field(line, "name") {
        builder.name = Some(value);
    }
    if let Some(value) = extract_parity_mode_field(line) {
        builder.mode = Some(value);
    }
}

fn extract_parity_mode_field(line: &str) -> Option<String> {
    let needle = "mode: ParityMode::";
    let start = line.find(needle)? + needle.len();
    let tail = &line[start..];
    let end = tail
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .unwrap_or(tail.len());
    Some(tail[..end].to_owned())
}

fn validate_registry_catalog(samples: &[RegistrySample]) -> Result<()> {
    if samples.is_empty() {
        return Err(Error::Message(
            "testbed registry must contain at least one entry".to_owned(),
        ));
    }

    let mut seen = BTreeSet::new();
    for sample in samples {
        validate_registry_field(sample, "id", &sample.id)?;
        validate_registry_field(sample, "category", &sample.category)?;
        validate_registry_field(sample, "name", &sample.name)?;
        validate_registry_field(sample, "description", &sample.description)?;
        if sample.upstream.is_empty() {
            return Err(Error::Message(format!(
                "testbed registry sample `{}` must include upstream sample references",
                sample.id
            )));
        }
        if !is_slug(&sample.id) {
            return Err(Error::Message(format!(
                "testbed registry id `{}` must be a lowercase ASCII slug",
                sample.id
            )));
        }
        if !seen.insert(sample.id.as_str()) {
            return Err(Error::Message(format!(
                "duplicate testbed registry id `{}`",
                sample.id
            )));
        }

        let mut upstream_seen = BTreeSet::new();
        for upstream in &sample.upstream {
            validate_registry_field(sample, "upstream.category", &upstream.category)?;
            validate_registry_field(sample, "upstream.name", &upstream.name)?;
            validate_registry_field(sample, "upstream.mode", &upstream.mode)?;
            if !matches!(
                upstream.mode.as_str(),
                "FaithfulPort" | "TeachingAdaptation"
            ) {
                return Err(Error::Message(format!(
                    "testbed registry sample `{}` uses unsupported upstream parity mode `{}`",
                    sample.id, upstream.mode
                )));
            }
            if !upstream_seen.insert((upstream.category.as_str(), upstream.name.as_str())) {
                return Err(Error::Message(format!(
                    "testbed registry sample `{}` duplicates upstream ref `{}` / `{}`",
                    sample.id, upstream.category, upstream.name
                )));
            }
        }
    }

    Ok(())
}

fn validate_registry_field(sample: &RegistrySample, field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::Message(format!(
            "testbed registry sample `{}` has an empty `{field}` field",
            sample.id
        )))
    } else {
        Ok(())
    }
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn reset_generated_examples_dir(pages_dir: &Path, examples_dir: &Path) -> Result<()> {
    if examples_dir.file_name().and_then(|name| name.to_str()) != Some("examples") {
        return Err(Error::Message(format!(
            "refusing to replace unexpected generated examples dir: {}",
            examples_dir.display()
        )));
    }
    replace_dir_under(examples_dir, pages_dir)
}

fn bevy_example_index_page(samples: &[RegistrySample], location: ExampleIndexLocation) -> String {
    let links = samples
        .iter()
        .map(|sample| {
            format!(
                "        <a class=\"card\" href=\"{href}\"><span>{category}</span><strong>{name}</strong><small>{description}</small><em>{upstream}</em></a>",
                href = location.example_href(&sample.id),
                category = escape_html(&sample.category),
                name = escape_html(&sample.name),
                description = escape_html(&sample.description),
                upstream = upstream_summary(&sample.upstream)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>boxdd Bevy Examples</title>
  <link rel="icon" href="data:,">
  <meta name="description" content="Direct Bevy Web examples for boxdd.">
  <style>{css}</style>
</head>
<body>
  <div class="directory">
    <header class="topbar">
      <a href="{home_href}">boxdd Examples</a>
      <nav>
        <a href="https://github.com/Latias94/boxdd">GitHub</a>
        <a href="https://docs.rs/boxdd">Docs.rs</a>
      </nav>
    </header>
    <main class="directory-main">
      <p class="eyebrow">Bevy Web examples</p>
      <h1>Run a Box2D scene</h1>
      <p class="lead">Each entry opens a dedicated Bevy + egui WASM page backed by the same Box2D provider runtime.</p>
      <section class="card-grid">
{links}
      </section>
    </main>
  </div>
</body>
</html>
"#,
        css = example_page_css(),
        home_href = location.home_href(),
        links = links
    )
}

fn bevy_testbed_page() -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>boxdd Bevy Testbed</title>
  <link rel="icon" href="data:,">
  <meta name="description" content="Bevy + egui WASM testbed for boxdd.">
  <style>{css}</style>
</head>
<body>
  <div class="shell">
    <header class="topbar">
      <div>
        <a href="../">boxdd Examples</a>
        <h1>Bevy Testbed</h1>
        <p><span>All scenes</span> Switch scenes from the egui panel.</p>
      </div>
      <nav>
        <a href="../examples/">All Bevy examples</a>
        <a href="https://github.com/Latias94/boxdd/tree/main/bevy_boxdd/examples/testbed_2d">Source</a>
      </nav>
    </header>
    <main id="bevy-app" data-scene-id="" data-scene-name="Bevy Testbed" data-scene-category="All scenes">
      <canvas id="bevy-canvas" tabindex="0"></canvas>
      <div id="bevy-status" role="status" aria-live="polite">
        <strong>Loading Bevy Testbed</strong>
        <span>Preparing the shared Box2D provider and the Rust Bevy wasm module.</span>
      </div>
    </main>
  </div>
  <script type="module" src="loader.js"></script>
</body>
</html>
"#,
        css = example_page_css()
    )
}

fn bevy_example_page(sample: &RegistrySample) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{name} - boxdd Bevy Example</title>
  <link rel="icon" href="data:,">
  <meta name="description" content="{description}">
  <style>{css}</style>
</head>
<body>
  <div class="shell">
    <header class="topbar">
      <div>
        <a href="../../">boxdd Examples</a>
        <h1>{name}</h1>
        <p><span>{category}</span>{description}</p>
        {upstream}
      </div>
      <nav>
        <a href="../">All Bevy examples</a>
        <a href="https://github.com/Latias94/boxdd/tree/main/bevy_boxdd/examples/testbed_2d">Source</a>
      </nav>
    </header>
    <main id="bevy-app" data-scene-id="{id}" data-scene-name="{name}" data-scene-category="{category}">
      <canvas id="bevy-canvas" tabindex="0"></canvas>
      <div id="bevy-status" role="status" aria-live="polite">
        <strong>Loading {name}</strong>
        <span>Preparing the shared Box2D provider and the Rust Bevy wasm module.</span>
      </div>
    </main>
  </div>
  <script type="module" src="../../bevy-testbed/loader.js"></script>
</body>
</html>
"#,
        id = escape_html(&sample.id),
        name = escape_html(&sample.name),
        category = escape_html(&sample.category),
        description = escape_html(&sample.description),
        upstream = source_list_html(sample),
        css = example_page_css()
    )
}

fn upstream_summary(upstream: &[RegistryUpstreamSample]) -> String {
    let mut labels = upstream
        .iter()
        .take(3)
        .map(|sample| format!("{} / {}", sample.category, sample.name))
        .collect::<Vec<_>>();
    if upstream.len() > labels.len() {
        labels.push(format!("+{} more", upstream.len() - labels.len()));
    }
    escape_html(&labels.join(", "))
}

fn source_list_html(sample: &RegistrySample) -> String {
    let mut items = String::new();
    for upstream in &sample.upstream {
        write!(
            items,
            "<span>{category} / {name} · {mode}</span>",
            category = escape_html(&upstream.category),
            name = escape_html(&upstream.name),
            mode = escape_html(&parity_mode_label(&upstream.mode))
        )
        .expect("writing to String cannot fail");
    }
    format!(r#"<div class="upstream-list">{items}</div>"#)
}

fn parity_mode_label(mode: &str) -> String {
    let mut label = String::new();
    for (index, ch) in mode.chars().enumerate() {
        if index > 0 && ch.is_ascii_uppercase() {
            label.push(' ');
        }
        label.push(ch.to_ascii_lowercase());
    }
    label
}

fn bevy_testbed_loader_js(trust: Option<&PagesLoaderTrust>) -> String {
    r##"const runtimeTrust = __BOXDD_RUNTIME_TRUST__;
const runtimeManifestUrl = new URL("../wasm/generated/boxdd-pages-runtime-v2.json", import.meta.url);
const expectedAssets = Object.freeze([
  Object.freeze({ role: "provider_js", path: "wasm/generated/box2d-sys-v1-single.js" }),
  Object.freeze({ role: "provider_wasm", path: "wasm/generated/box2d-sys-v1-single.wasm" }),
  Object.freeze({ role: "app_js", path: "bevy-testbed/generated/bevy_boxdd_testbed.js" }),
  Object.freeze({ role: "app_wasm", path: "bevy-testbed/generated/bevy_boxdd_testbed_bg.wasm" }),
  Object.freeze({ role: "provider_shim_js", path: "bevy-testbed/generated/box2d-provider-shim.js" }),
]);
const manifestKeys = Object.freeze([
  "adapter_abi_version",
  "adapter_source_sha256",
  "assets",
  "crate_version",
  "emscripten_sdk_contract_sha256",
  "effective_source_sha256",
  "precision",
  "provider",
  "provider_abi",
  "publisher_repository",
  "publisher_workflow",
  "recording_contract_blake3",
  "schema",
  "schema_version",
  "source_commit",
  "source_tree",
  "target",
  "upstream_sha",
  "wasm_provider_contract_sha256",
]);
const identityKeys = Object.freeze(manifestKeys.filter((key) => key !== "assets"));
const assetKeys = Object.freeze(["byte_length", "path", "role", "sha256"]);

const statusPanel = document.querySelector("#bevy-status");
const appRoot = document.querySelector("#bevy-app");
const sceneId = appRoot?.dataset.sceneId || "";
const sceneName = appRoot?.dataset.sceneName || "Bevy testbed";
const isExamplePage = Boolean(sceneId);

function setStatus(state, title, detail, progress) {
  statusPanel.dataset.state = state;
  statusPanel.replaceChildren();

  const titleNode = document.createElement("strong");
  titleNode.textContent = title;
  const detailNode = document.createElement("span");
  detailNode.textContent = detail;
  statusPanel.append(titleNode, detailNode);

  if (progress) {
    const progressNode = document.createElement("progress");
    progressNode.value = progress.loaded;
    if (progress.total) {
      progressNode.max = progress.total;
    } else {
      progressNode.removeAttribute("value");
    }

    const progressText = document.createElement("small");
    progressText.textContent = progressTextFor(progress.loaded, progress.total);
    statusPanel.append(progressNode, progressText);
  }
}

function pageAssetUrl(path) {
  return new URL(`../${path}`, import.meta.url);
}

function progressTextFor(loaded, total) {
  if (total) {
    const percent = Math.min(100, Math.round((loaded / total) * 100));
    return `${formatBytes(loaded)} / ${formatBytes(total)} (${percent}%)`;
  }
  return `${formatBytes(loaded)} downloaded`;
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${value} ${units[unit]}` : `${value.toFixed(2)} ${units[unit]}`;
}

async function fetchArrayBufferWithProgress(url, label) {
  setStatus("loading", `Downloading ${label}`, "Starting download.", { loaded: 0, total: 0 });
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${label} download failed with HTTP ${response.status}`);
  }

  const total = Number(response.headers.get("Content-Length")) || 0;
  if (!response.body) {
    const buffer = await response.arrayBuffer();
    setStatus("loading", `Downloading ${label}`, "Download complete.", {
      loaded: buffer.byteLength,
      total: total || buffer.byteLength,
    });
    return buffer;
  }

  const reader = response.body.getReader();
  const chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(value);
    loaded += value.byteLength;
    setStatus("loading", `Downloading ${label}`, "Downloading runtime asset.", { loaded, total });
  }

  const bytes = new Uint8Array(loaded);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  setStatus("loading", `Downloading ${label}`, "Download complete.", { loaded, total: total || loaded });
  return bytes.buffer;
}

function assertExactObjectKeys(value, expected, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  if (actual.length !== required.length || actual.some((key, index) => key !== required[index])) {
    throw new Error(`${label} fields do not match the canonical schema`);
  }
}

function decodeUtf8(bytes, label) {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (error) {
    throw new Error(`${label} is not valid UTF-8`, { cause: error });
  }
}

async function sha256Hex(bytes) {
  if (!globalThis.crypto?.subtle) {
    throw new Error("Web Crypto SHA-256 is unavailable; refusing unverified runtime assets");
  }
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function verifySha256(bytes, expected, label) {
  if (!/^[0-9a-f]{64}$/.test(expected)) {
    throw new Error(`${label} manifest SHA-256 is malformed`);
  }
  const actual = await sha256Hex(bytes);
  if (actual !== expected) {
    throw new Error(`${label} SHA-256 mismatch: expected ${expected}, got ${actual}`);
  }
}

async function loadRuntimeManifest() {
  if (!runtimeTrust) {
    throw new Error("Pages runtime trust anchor is absent; publish assets with build-pages-wasm");
  }
  const bytes = await fetchArrayBufferWithProgress(runtimeManifestUrl, "runtime manifest");
  await verifySha256(bytes, runtimeTrust.manifest_sha256, "runtime manifest");

  let manifest;
  try {
    manifest = JSON.parse(decodeUtf8(bytes, "runtime manifest"));
  } catch (error) {
    throw new Error("runtime manifest is not valid JSON", { cause: error });
  }
  assertExactObjectKeys(manifest, manifestKeys, "runtime manifest");
  for (const key of identityKeys) {
    if (manifest[key] !== runtimeTrust[key]) {
      throw new Error(`runtime manifest identity ${key} does not match the loader trust anchor`);
    }
  }
  if (!Array.isArray(manifest.assets) || manifest.assets.length !== expectedAssets.length) {
    throw new Error("runtime manifest must contain the exact qualified asset set");
  }
  manifest.assets.forEach((asset, index) => {
    assertExactObjectKeys(asset, assetKeys, `runtime asset ${index}`);
    const expected = expectedAssets[index];
    if (asset.role !== expected.role || asset.path !== expected.path) {
      throw new Error(`runtime asset ${index} does not match the canonical role and path`);
    }
    if (!Number.isSafeInteger(asset.byte_length) || asset.byte_length <= 0) {
      throw new Error(`runtime asset ${asset.role} has an invalid byte length`);
    }
    if (!/^[0-9a-f]{64}$/.test(asset.sha256)) {
      throw new Error(`runtime asset ${asset.role} has an invalid SHA-256`);
    }
  });
  return manifest;
}

async function loadVerifiedRuntimeAssets(manifest) {
  const verified = new Map();
  for (const asset of manifest.assets) {
    const bytes = await fetchArrayBufferWithProgress(pageAssetUrl(asset.path), asset.role);
    if (bytes.byteLength !== asset.byte_length) {
      throw new Error(
        `${asset.role} byte length mismatch: expected ${asset.byte_length}, got ${bytes.byteLength}`,
      );
    }
    await verifySha256(bytes, asset.sha256, asset.role);
    verified.set(asset.role, bytes);
  }
  return verified;
}

function replaceShimImport(appBytes, shimModuleUrl) {
  const source = decodeUtf8(appBytes, "Bevy app JavaScript");
  const shimModuleName = "box2d-provider-shim.js";
  const specifier = '"./box2d-provider-shim.js"';
  const shimImportPattern = /^import \* as (import[0-9]+) from "\.\/box2d-provider-shim\.js";?$/;
  const importLines = [];
  const importBindings = new Set();
  for (const [lineNumber, line] of source.split(/\r?\n/).entries()) {
    if (!line.includes(shimModuleName)) {
      continue;
    }
    const match = shimImportPattern.exec(line);
    if (!match || importBindings.has(match[1])) {
      throw new Error("Bevy app JavaScript contains an unsupported wasm-bindgen provider shim import");
    }
    importBindings.add(match[1]);
    importLines.push(lineNumber);
  }
  if (
    importLines.length === 0 ||
    importLines.some((lineNumber, offset) => lineNumber !== importLines[0] + offset) ||
    !importBindings.has("import1")
  ) {
    throw new Error("Bevy app JavaScript must contain one contiguous block of qualified wasm-bindgen provider shim imports");
  }
  return source.replaceAll(specifier, JSON.stringify(shimModuleUrl));
}

async function importVerifiedRuntimeModules(assets) {
  const shimUrl = URL.createObjectURL(
    new Blob([assets.get("provider_shim_js")], { type: "text/javascript" }),
  );
  const providerUrl = URL.createObjectURL(
    new Blob([assets.get("provider_js")], { type: "text/javascript" }),
  );
  const appSource = replaceShimImport(assets.get("app_js"), shimUrl);
  const appUrl = URL.createObjectURL(new Blob([appSource], { type: "text/javascript" }));

  try {
    return await Promise.all([import(providerUrl), import(appUrl), import(shimUrl)]);
  } finally {
    URL.revokeObjectURL(appUrl);
    URL.revokeObjectURL(providerUrl);
    URL.revokeObjectURL(shimUrl);
  }
}

async function waitForProviderStep(providerEvidence, previousSteps, label) {
  const deadline = performance.now() + 20_000;
  for (;;) {
    const evidence = providerEvidence();
    if (
      Number.isSafeInteger(evidence.providerCalls) &&
      Number.isSafeInteger(evidence.stepCalls) &&
      evidence.providerCalls >= evidence.stepCalls &&
      evidence.stepCalls > previousSteps
    ) {
      return evidence;
    }
    if (performance.now() >= deadline) {
      throw new Error(`${label} did not observe a Box2D physics step before the deadline`);
    }
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
}

async function main() {
  setStatus("loading", "Verifying runtime identity", `Checking the published assets for ${sceneName}.`);
  const manifest = await loadRuntimeManifest();
  const assets = await loadVerifiedRuntimeAssets(manifest);
  setStatus("loading", "Loading verified modules", `Preparing the browser runtime for ${sceneName}.`);
  const [
    { default: createProvider },
    { default: initBevyTestbed },
    { boxddProviderRuntimeEvidence, setBox2dProvider, setBoxddAppExports },
  ] = await importVerifiedRuntimeModules(assets);
  const memory = new WebAssembly.Memory({ initial: 4096, maximum: 8192 });

  setStatus("loading", "Starting Box2D provider", `Instantiating the shared Box2D C provider for ${sceneName}.`);
  const provider = await createProvider({
    wasmMemory: memory,
    wasmBinary: assets.get("provider_wasm"),
    locateFile: (path) => pageAssetUrl(`wasm/generated/${path}`).href,
    print: (text) => console.log(`[box2d-sys-v1-single] ${text}`),
    printErr: (text) => console.warn(`[box2d-sys-v1-single] ${text}`),
  });

  if (provider.wasmMemory && provider.wasmMemory !== memory) {
    throw new Error("Box2D provider did not use the shared WebAssembly.Memory");
  }
  const adapterAbiVersion = provider._boxddAdapter_AbiVersion || provider.boxddAdapter_AbiVersion;
  if (typeof adapterAbiVersion !== "function" || adapterAbiVersion() !== manifest.adapter_abi_version) {
    throw new Error("Box2D provider runtime adapter ABI does not match the verified manifest");
  }

  setBox2dProvider(provider);
  setStatus("loading", `Starting ${sceneName}`, "Instantiating the Rust Bevy + egui wasm module.");

  const bevyExports = await initBevyTestbed({
    module_or_path: assets.get("app_wasm"),
    memory,
  });
  setBoxddAppExports(bevyExports);

  const initialEvidence = await waitForProviderStep(
    boxddProviderRuntimeEvidence,
    0,
    "initial runtime proof",
  );
  const proofRequested = new URLSearchParams(window.location.search).get("boxdd-runtime-proof") === "1";
  const memoryProof = {
    requested: proofRequested,
    memoryGrew: false,
    staleBufferDetached: false,
    providerHeapViewRefreshed: false,
    providerHeapReadWrite: false,
    postGrowthPhysicsStep: false,
    byteLengthBeforeGrowth: memory.buffer.byteLength,
    byteLengthAfterGrowth: memory.buffer.byteLength,
    stepCallsBeforeGrowth: initialEvidence.stepCalls,
    stepCallsAfterGrowth: initialEvidence.stepCalls,
  };
  if (proofRequested) {
    const staleBuffer = memory.buffer;
    const staleProviderHeap = provider.boxddRefreshMemoryViews();
    if (staleProviderHeap !== provider.HEAPU8 || staleProviderHeap.buffer !== staleBuffer) {
      throw new Error("Box2D provider HEAPU8 does not bind the shared WebAssembly.Memory");
    }
    memory.grow(1);
    memoryProof.memoryGrew = memory.buffer !== staleBuffer;
    memoryProof.staleBufferDetached = staleBuffer.byteLength === 0;
    memoryProof.byteLengthAfterGrowth = memory.buffer.byteLength;
    if (
      !memoryProof.memoryGrew ||
      !memoryProof.staleBufferDetached ||
      staleProviderHeap.byteLength !== 0 ||
      memoryProof.byteLengthAfterGrowth <= memoryProof.byteLengthBeforeGrowth
    ) {
      throw new Error("shared WebAssembly.Memory did not detach and grow its buffer");
    }
    const refreshedProviderHeap = provider.boxddRefreshMemoryViews();
    memoryProof.providerHeapViewRefreshed =
      refreshedProviderHeap instanceof Uint8Array &&
      refreshedProviderHeap === provider.HEAPU8 &&
      refreshedProviderHeap.buffer === memory.buffer;
    if (!memoryProof.providerHeapViewRefreshed) {
      throw new Error("Emscripten HEAPU8 was not rebound after external memory.grow");
    }
    const probeOffset = memoryProof.byteLengthBeforeGrowth;
    const refreshedData = new DataView(memory.buffer);
    const original = refreshedData.getUint32(probeOffset, true);
    refreshedProviderHeap.set([0x12, 0x34, 0x56, 0x78], probeOffset);
    memoryProof.providerHeapReadWrite =
      refreshedData.getUint32(probeOffset, true) === 0x78563412;
    refreshedData.setUint32(probeOffset, original, true);
    if (!memoryProof.providerHeapReadWrite) {
      throw new Error("refreshed Emscripten HEAPU8 is not readable and writable");
    }
    const postGrowthEvidence = await waitForProviderStep(
      boxddProviderRuntimeEvidence,
      memoryProof.stepCallsBeforeGrowth,
      "post-growth runtime proof",
    );
    memoryProof.stepCallsAfterGrowth = postGrowthEvidence.stepCalls;
    memoryProof.postGrowthPhysicsStep = true;
  }

  window.BOXDD_BEVY_RUNTIME_EVIDENCE = () => {
    const evidence = boxddProviderRuntimeEvidence();
    return Object.freeze({
      providerCalls: evidence.providerCalls,
      stepCalls: evidence.stepCalls,
      memoryProof: Object.freeze({ ...memoryProof }),
    });
  };

  window.BOXDD_BEVY_TESTBED_READY = true;
  window.BOXDD_BEVY_EXAMPLE_READY = true;
  window.BOXDD_BEVY_SCENE_ID = sceneId;
  setStatus(
    "running",
    `${sceneName} running`,
    isExamplePage
      ? "This dedicated example page is running the selected Box2D scene in Bevy."
      : "The scene browser, egui controls, and Box2D simulation are running in this canvas.",
  );
}

main().catch((error) => {
  console.error(error);
  const message = error instanceof Error ? error.message : String(error);
  setStatus("error", `${sceneName} failed`, message);
});
"##
        .replace("__BOXDD_RUNTIME_TRUST__", &pages_loader_trust_js(trust))
}

fn pages_loader_trust_js(trust: Option<&PagesLoaderTrust>) -> String {
    let Some(trust) = trust else {
        return "null".to_owned();
    };
    let value = serde_json::json!({
        "manifest_sha256": trust.manifest_sha256,
        "schema_version": trust.schema_version,
        "schema": trust.schema,
        "publisher_repository": trust.publisher_repository,
        "publisher_workflow": trust.publisher_workflow,
        "provider": trust.provider,
        "provider_abi": trust.provider_abi,
        "adapter_abi_version": trust.adapter_abi_version,
        "crate_version": trust.crate_version,
        "source_commit": trust.source_commit,
        "upstream_sha": trust.upstream_sha,
        "source_tree": trust.source_tree,
        "effective_source_sha256": trust.effective_source_sha256,
        "adapter_source_sha256": trust.adapter_source_sha256,
        "emscripten_sdk_contract_sha256": trust.emscripten_sdk_contract_sha256,
        "wasm_provider_contract_sha256": trust.wasm_provider_contract_sha256,
        "recording_contract_blake3": trust.recording_contract_blake3,
        "precision": trust.precision,
        "target": trust.target,
    });
    format!("Object.freeze({value})")
}

impl ExampleIndexLocation {
    fn home_href(self) -> &'static str {
        match self {
            Self::Root => "./",
            Self::ExamplesDirectory => "../",
        }
    }

    fn example_href(self, id: &str) -> String {
        match self {
            Self::Root => format!("examples/{id}/"),
            Self::ExamplesDirectory => format!("{id}/"),
        }
    }
}

fn example_page_css() -> &'static str {
    r#"
:root {
  color-scheme: dark;
  --background: #09090b;
  --foreground: #fafafa;
  --card: #0f0f12;
  --muted: #a1a1aa;
  --border: #27272a;
  --accent: #2dd4bf;
  --danger: #f87171;
}
* { box-sizing: border-box; }
html, body { width: 100%; height: 100%; margin: 0; background: var(--background); color: var(--foreground); font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
a { color: var(--foreground); text-decoration: none; }
a:hover { text-decoration: underline; text-underline-offset: 4px; }
.shell { display: grid; grid-template-rows: auto minmax(0, 1fr); width: 100%; height: 100%; }
.topbar { display: flex; flex-wrap: wrap; gap: 14px; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border); background: rgba(9, 9, 11, 0.94); padding: 14px 18px; }
.topbar h1 { margin: 4px 0 0; font-size: 20px; line-height: 1.2; letter-spacing: 0; }
.topbar p { display: flex; flex-wrap: wrap; gap: 8px; margin: 5px 0 0; color: var(--muted); font-size: 13px; }
.topbar p span, .eyebrow { color: var(--accent); font-weight: 700; text-transform: uppercase; }
.topbar nav { display: flex; flex-wrap: wrap; gap: 12px; color: var(--muted); font-size: 14px; }
#bevy-app { position: relative; min-width: 0; min-height: 0; background: #020617; }
#bevy-canvas { display: block; width: 100%; height: 100%; outline: none; touch-action: none; }
#bevy-status { position: absolute; left: 18px; bottom: 18px; max-width: min(560px, calc(100% - 36px)); border: 1px solid var(--border); border-radius: 8px; background: rgba(15, 15, 18, 0.94); padding: 12px 14px; color: var(--muted); font-size: 14px; line-height: 1.45; }
#bevy-status strong { display: block; margin-bottom: 4px; color: var(--foreground); font-size: 15px; }
#bevy-status progress { display: block; width: min(360px, 100%); height: 8px; margin-top: 10px; accent-color: var(--accent); }
#bevy-status small { display: block; margin-top: 6px; color: #d4d4d8; font-size: 12px; }
#bevy-status[data-state="error"] strong { color: var(--danger); }
#bevy-status[data-state="running"] { opacity: 0; pointer-events: none; transition: opacity 180ms ease; }
.directory { min-height: 100%; }
.directory-main { width: min(1180px, calc(100% - 32px)); margin: 0 auto; padding: 54px 0; }
.directory-main h1 { margin: 0; font-size: clamp(34px, 6vw, 58px); line-height: 1; letter-spacing: 0; }
.lead { max-width: 720px; color: var(--muted); font-size: 17px; }
.card-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; margin-top: 28px; }
.card { display: grid; min-height: 150px; gap: 8px; border: 1px solid var(--border); border-radius: 8px; background: var(--card); padding: 16px; }
.card:hover { border-color: #52525b; text-decoration: none; }
.card span { color: var(--accent); font-size: 12px; font-weight: 700; text-transform: uppercase; }
.card strong { font-size: 18px; }
.card small { color: var(--muted); font-size: 13px; line-height: 1.5; }
.card em { color: #d4d4d8; font-size: 12px; font-style: normal; line-height: 1.45; }
.upstream-list { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
.upstream-list span { border: 1px solid var(--border); border-radius: 999px; background: rgba(39, 39, 42, 0.7); padding: 4px 7px; color: #d4d4d8; font-size: 12px; line-height: 1.2; text-transform: none; }
"#
}

fn extract_string_field(line: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}: ");
    let rest = line.strip_prefix(&prefix)?;
    extract_quoted_string(rest)
}

fn extract_quoted_string(value: &str) -> Option<String> {
    let start = value.find('"')?;
    let rest = &value[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn validate_pages_runtime(root: &Path) -> Result<Option<PagesLoaderTrust>> {
    let wasm_generated = pages_wasm_generated_dir(root);
    let bevy_generated = pages_bevy_generated_dir(root);
    let wasm_metadata = optional_symlink_metadata(&wasm_generated)?;
    let bevy_metadata = optional_symlink_metadata(&bevy_generated)?;
    if wasm_metadata.is_none() && bevy_metadata.is_none() {
        return Ok(None);
    }
    if !wasm_metadata
        .is_some_and(|metadata| metadata.file_type().is_dir() && !metadata.file_type().is_symlink())
        || !bevy_metadata.is_some_and(|metadata| {
            metadata.file_type().is_dir() && !metadata.file_type().is_symlink()
        })
    {
        return Err(Error::Message(
            "Pages runtime generation is partial or contains a symlink; both generated paths must be real directories"
                .to_owned(),
        ));
    }
    for spec in PAGES_RUNTIME_ASSETS {
        require_pages_output_parent(root, &root.join("docs/pages").join(spec.path))?;
    }
    ensure_pages_build_inputs_clean(root)?;
    let precision = ProviderPrecision::Single;
    let identity = pages_runtime_identity(root, precision)?;

    validate_pages_runtime_directory(
        &wasm_generated,
        &[
            "box2d-sys-v1-single.js",
            "box2d-sys-v1-single.wasm",
            "boxdd-pages-runtime-v2.json",
        ],
    )?;
    validate_pages_runtime_directory(
        &bevy_generated,
        &[BEVY_WEB_JS, BEVY_WEB_WASM, BEVY_PROVIDER_SHIM],
    )?;

    let manifest_path = pages_runtime_manifest_path(root);
    let metadata =
        fs::symlink_metadata(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::Message(format!(
            "Pages runtime manifest must be a regular non-symlink file: {}",
            manifest_path.display()
        )));
    }
    let bytes = fs::read(&manifest_path).map_err(|source| Error::io(&manifest_path, source))?;
    let manifest = PagesRuntimeManifest::parse(&bytes)?;
    validate_pages_runtime_manifest_identity(&manifest, &identity)?;
    validate_pages_runtime_asset_records(&root.join("docs/pages"), &manifest)?;
    ensure_pages_source_state(root, precision, &identity)?;

    Ok(Some(PagesLoaderTrust::from_manifest(
        &manifest,
        provider_manifest::sha256_bytes(&bytes),
    )))
}

fn optional_symlink_metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::io(path, source)),
    }
}

fn validate_pages_runtime_asset_records(
    pages_dir: &Path,
    manifest: &PagesRuntimeManifest,
) -> Result<()> {
    if manifest.assets.len() != PAGES_RUNTIME_ASSETS.len() {
        return Err(Error::Message(format!(
            "Pages runtime manifest contains {} assets, expected exactly {}",
            manifest.assets.len(),
            PAGES_RUNTIME_ASSETS.len()
        )));
    }

    for (recorded, spec) in manifest.assets.iter().zip(PAGES_RUNTIME_ASSETS) {
        if recorded.role != spec.role || recorded.path != spec.path {
            return Err(Error::Message(format!(
                "Pages runtime manifest asset `{}` at `{}` does not match canonical role `{}` at `{}`",
                recorded.role, recorded.path, spec.role, spec.path
            )));
        }
        validate_lower_hex(
            &format!("Pages runtime asset {} SHA-256", recorded.role),
            &recorded.sha256,
            64,
        )?;
        let actual = pages_runtime_asset(pages_dir, spec)?;
        if &actual != recorded {
            return Err(Error::Message(format!(
                "Pages runtime asset `{}` bytes do not match the canonical manifest",
                recorded.role
            )));
        }
    }
    Ok(())
}

fn validate_pages_runtime_directory(dir: &Path, expected_names: &[&str]) -> Result<()> {
    let expected = expected_names.iter().copied().collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|source| Error::io(&path, source))?;
        if !kind.is_file() || kind.is_symlink() {
            return Err(Error::Message(format!(
                "Pages runtime generated directory contains a non-regular entry: {}",
                path.display()
            )));
        }
        let name = entry.file_name().into_string().map_err(|_| {
            Error::Message(format!(
                "Pages runtime asset is not UTF-8: {}",
                path.display()
            ))
        })?;
        actual.insert(name);
    }
    let actual_refs = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if actual_refs != expected {
        return Err(Error::Message(format!(
            "Pages runtime generated directory {} contains {:?}, expected exactly {:?}",
            dir.display(),
            actual_refs,
            expected
        )));
    }
    Ok(())
}

pub(crate) fn validate_pages(root: &Path) -> Result<()> {
    let pages_dir = root.join("docs/pages");
    require_real_directory(&pages_dir, "Pages output root")?;
    let samples = read_testbed_registry(root)?;
    let expected_pages = expected_bevy_pages(root, &samples);
    let html_files = collect_html_files(&pages_dir)?;
    if html_files.is_empty() {
        return Err(Error::Message(format!(
            "no html pages found under {}",
            pages_dir.display()
        )));
    }

    let expected_paths: BTreeSet<PathBuf> = expected_pages.keys().cloned().collect();
    let actual_paths: BTreeSet<PathBuf> = html_files.iter().cloned().collect();
    let mut errors = Vec::new();
    for stale in actual_paths.difference(&expected_paths) {
        errors.push(format!(
            "{} is not generated by `cargo run -p xtask -- generate-pages`",
            stale.strip_prefix(root).unwrap_or(stale).display()
        ));
    }
    for (path, expected) in &expected_pages {
        if !path.exists() {
            errors.push(format!(
                "missing generated page {}",
                path.strip_prefix(root).unwrap_or(path).display()
            ));
            continue;
        }
        let actual = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
        if normalize_newlines(&actual) != normalize_newlines(expected) {
            errors.push(format!(
                "{} is stale; run `cargo run -p xtask -- generate-pages`",
                path.strip_prefix(root).unwrap_or(path).display()
            ));
        }
    }

    for file in &html_files {
        let content = fs::read_to_string(file).map_err(|source| Error::io(file, source))?;
        for link in extract_links(&content) {
            if should_skip_link(&link) {
                continue;
            }
            let without_fragment = link.split('#').next().unwrap_or_default();
            if without_fragment.is_empty() {
                continue;
            }
            let target = file.parent().unwrap_or(root).join(without_fragment);
            if !target.exists() {
                errors.push(format!(
                    "{} links to missing local target `{}`",
                    file.strip_prefix(root).unwrap_or(file).display(),
                    link
                ));
            }
        }
    }

    let runtime_trust = match validate_pages_runtime(root) {
        Ok(trust) => trust,
        Err(error) => {
            errors.push(error.to_string());
            None
        }
    };

    let loader = pages_bevy_testbed_dir(root).join("loader.js");
    if !loader.exists() {
        errors.push(
            "missing generated Bevy testbed loader docs/pages/bevy-testbed/loader.js".to_owned(),
        );
    } else {
        require_pages_output_parent(root, &loader)?;
        let metadata =
            fs::symlink_metadata(&loader).map_err(|source| Error::io(&loader, source))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            errors.push(format!(
                "{} must be a regular non-symlink generated loader",
                loader.strip_prefix(root).unwrap_or(&loader).display()
            ));
        } else {
            let actual =
                fs::read_to_string(&loader).map_err(|source| Error::io(&loader, source))?;
            if normalize_newlines(&actual)
                != normalize_newlines(&bevy_testbed_loader_js(runtime_trust.as_ref()))
            {
                errors.push(
                    "docs/pages/bevy-testbed/loader.js is stale or does not bind the validated runtime manifest; run `cargo run -p xtask -- build-pages-wasm` for runtime assets or `generate-pages` without them".to_owned(),
                );
            }
            for required in [
                "runtimeTrust",
                "crypto.subtle.digest",
                "verifySha256",
                "box2d-provider-shim.js",
                "setBox2dProvider",
                "setBoxddAppExports",
                "bevyExports",
            ] {
                if !actual.contains(required) {
                    errors.push(format!(
                        "{} is missing required Bevy provider glue `{required}`",
                        loader.strip_prefix(root).unwrap_or(&loader).display()
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        println!(
            "pages ok: {} html files checked, {} Bevy WASM examples",
            html_files.len(),
            samples.len()
        );
        Ok(())
    } else {
        Err(Error::Message(errors.join("\n")))
    }
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn collect_html_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_html_files_into(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_html_files_into(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| Error::io(&path, source))?;
        if file_type.is_symlink() {
            return Err(Error::Message(format!(
                "Pages HTML tree contains a symbolic link: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            collect_html_files_into(&path, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "html") {
            out.push(path);
        }
    }
    Ok(())
}

fn extract_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    for attr in ["href=\"", "src=\""] {
        let mut rest = content;
        while let Some(index) = rest.find(attr) {
            rest = &rest[index + attr.len()..];
            let Some(end) = rest.find('"') else {
                break;
            };
            links.push(rest[..end].to_owned());
            rest = &rest[end + 1..];
        }
    }
    links
}

fn should_skip_link(link: &str) -> bool {
    link.starts_with('#')
        || link.starts_with("http://")
        || link.starts_with("https://")
        || link.starts_with("mailto:")
        || link.starts_with("data:")
        || link.starts_with('/')
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        ADAPTER_ABI_VERSION, JAVASCRIPT_MAX_SAFE_INTEGER, PAGES_PUBLISHER_WORKFLOW,
        PAGES_RUNTIME_ASSETS, PAGES_RUNTIME_SCHEMA, PAGES_RUNTIME_SCHEMA_VERSION, PROVIDER_ABI,
        PUBLISHER_REPOSITORY, PagesLoaderTrust, PagesRuntimeAsset, PagesRuntimeIdentity,
        PagesRuntimeManifest, ProviderPrecision, RECORDING_CONTRACT_BLAKE3, WASM_TARGET,
        bevy_testbed_loader_js, clear_pages_runtime_assets, collect_html_files,
        ensure_pages_output_parent, format_bytes, pages_bevy_generated_dir, pages_runtime_asset,
        pages_wasm_generated_dir, rewrite_bevy_bindgen_provider_imports,
        validate_pages_asset_byte_length, validate_pages_precision,
        validate_pages_runtime_asset_records, validate_pages_runtime_manifest_identity,
    };
    use crate::qualified_git::qualified_git_command;

    fn manifest() -> PagesRuntimeManifest {
        PagesRuntimeManifest {
            schema_version: PAGES_RUNTIME_SCHEMA_VERSION,
            schema: PAGES_RUNTIME_SCHEMA.to_owned(),
            publisher_repository: PUBLISHER_REPOSITORY.to_owned(),
            publisher_workflow: PAGES_PUBLISHER_WORKFLOW.to_owned(),
            provider: "wasm-runtime".to_owned(),
            provider_abi: PROVIDER_ABI.to_owned(),
            adapter_abi_version: ADAPTER_ABI_VERSION,
            crate_version: env!("CARGO_PKG_VERSION").to_owned(),
            source_commit: "1".repeat(40),
            upstream_sha: "2".repeat(40),
            source_tree: "3".repeat(40),
            effective_source_sha256: "4".repeat(64),
            adapter_source_sha256: "5".repeat(64),
            emscripten_sdk_contract_sha256: "6".repeat(64),
            wasm_provider_contract_sha256: "8".repeat(64),
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            precision: ProviderPrecision::Single.as_str().to_owned(),
            target: WASM_TARGET.to_owned(),
            assets: PAGES_RUNTIME_ASSETS
                .iter()
                .enumerate()
                .map(|(index, spec)| PagesRuntimeAsset {
                    role: spec.role.to_owned(),
                    path: spec.path.to_owned(),
                    byte_length: (index + 1) as u64,
                    sha256: format!("{index:064x}"),
                })
                .collect(),
        }
    }

    fn runtime_identity(manifest: &PagesRuntimeManifest) -> PagesRuntimeIdentity {
        PagesRuntimeIdentity {
            source_commit: manifest.source_commit.clone(),
            upstream_sha: manifest.upstream_sha.clone(),
            source_tree: manifest.source_tree.clone(),
            effective_source_sha256: manifest.effective_source_sha256.clone(),
            adapter_source_sha256: manifest.adapter_source_sha256.clone(),
            emscripten_sdk_contract_sha256: manifest.emscripten_sdk_contract_sha256.clone(),
            wasm_provider_contract_sha256: manifest.wasm_provider_contract_sha256.clone(),
        }
    }

    #[test]
    fn format_bytes_uses_binary_units() {
        assert_eq!(format_bytes(31), "31 B");
        assert_eq!(format_bytes(1536), "1.50 KiB");
        assert_eq!(format_bytes(2 * 1024 * 1024), "2.00 MiB");
    }

    #[test]
    fn pages_rejects_unimplemented_double_precision_loader() {
        assert!(validate_pages_precision(ProviderPrecision::Single).is_ok());
        assert!(validate_pages_precision(ProviderPrecision::Double).is_err());
    }

    #[test]
    fn pages_assets_must_fit_the_browser_integer_contract() {
        let path = std::path::Path::new("asset.wasm");
        assert!(validate_pages_asset_byte_length(path, 1).is_ok());
        assert!(validate_pages_asset_byte_length(path, JAVASCRIPT_MAX_SAFE_INTEGER).is_ok());
        assert!(validate_pages_asset_byte_length(path, 0).is_err());
        assert!(validate_pages_asset_byte_length(path, JAVASCRIPT_MAX_SAFE_INTEGER + 1).is_err());
    }

    #[test]
    fn pages_git_commands_use_a_qualified_absolute_program() {
        let command = qualified_git_command().unwrap();
        assert!(std::path::Path::new(command.get_program()).is_absolute());
    }

    #[cfg(unix)]
    #[test]
    fn pages_output_rejects_a_symlinked_parent() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::tempdir().unwrap();
        let pages = fixture.path().join("docs/pages");
        let outside = fixture.path().join("outside");
        fs::create_dir_all(&pages).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, pages.join("generated")).unwrap();

        let output = pages.join("generated/runtime.js");
        assert!(ensure_pages_output_parent(fixture.path(), &output).is_err());
        assert!(!outside.join("runtime.js").exists());
    }

    #[cfg(unix)]
    #[test]
    fn pages_html_scan_rejects_nested_symbolic_links() {
        use std::os::unix::fs::symlink;

        let fixture = tempfile::tempdir().unwrap();
        let pages = fixture.path().join("pages");
        let outside = fixture.path().join("outside");
        fs::create_dir_all(&pages).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("index.html"), "outside").unwrap();
        symlink(&outside, pages.join("nested")).unwrap();

        assert!(collect_html_files(&pages).is_err());
    }

    #[test]
    fn null_trust_generation_removes_runtime_asset_directories() {
        let fixture = tempfile::tempdir().unwrap();
        let pages = fixture.path().join("docs/pages");
        let wasm = pages_wasm_generated_dir(fixture.path());
        let bevy = pages_bevy_generated_dir(fixture.path());
        fs::create_dir_all(&wasm).unwrap();
        fs::create_dir_all(&bevy).unwrap();
        fs::write(wasm.join("stale.json"), b"stale").unwrap();
        fs::write(bevy.join("stale.wasm"), b"stale").unwrap();

        clear_pages_runtime_assets(fixture.path()).unwrap();

        assert!(pages.is_dir());
        assert!(!wasm.exists());
        assert!(!bevy.exists());
    }

    #[test]
    fn runtime_manifest_has_one_canonical_strict_representation() {
        let manifest = manifest();
        let bytes = manifest.render().unwrap();
        assert_eq!(PagesRuntimeManifest::parse(&bytes).unwrap(), manifest);

        let with_unknown_field =
            String::from_utf8(bytes)
                .unwrap()
                .replacen("{\n", "{\n  \"unknown\": true,\n", 1);
        assert!(PagesRuntimeManifest::parse(with_unknown_field.as_bytes()).is_err());

        let noncanonical = manifest
            .render()
            .unwrap()
            .into_iter()
            .filter(|byte| *byte != b' ' && *byte != b'\n')
            .collect::<Vec<_>>();
        assert!(PagesRuntimeManifest::parse(&noncanonical).is_err());
    }

    #[test]
    fn runtime_manifest_rejects_mismatched_wasm_provider_contract_digest() {
        let mut manifest = manifest();
        let identity = runtime_identity(&manifest);
        validate_pages_runtime_manifest_identity(&manifest, &identity).unwrap();

        manifest.wasm_provider_contract_sha256 = "9".repeat(64);
        let error = validate_pages_runtime_manifest_identity(&manifest, &identity).unwrap_err();

        assert!(
            error.to_string().contains("wasm_provider_contract_sha256"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn loader_verifies_all_assets_before_importing_or_instantiating() {
        let manifest = manifest();
        let trust = PagesLoaderTrust::from_manifest(&manifest, "7".repeat(64));
        let loader = bevy_testbed_loader_js(Some(&trust));
        let verify = loader
            .find("const assets = await loadVerifiedRuntimeAssets(manifest);")
            .unwrap();
        let import = loader
            .find("] = await importVerifiedRuntimeModules(assets);")
            .unwrap();
        let instantiate = loader.find("await createProvider({").unwrap();

        assert!(loader.contains("crypto.subtle.digest(\"SHA-256\", bytes)"));
        assert!(loader.contains("manifest_sha256"));
        assert!(loader.contains("emscripten_sdk_contract_sha256"));
        assert!(loader.contains("wasm_provider_contract_sha256"));
        assert!(loader.contains("boxddProviderRuntimeEvidence"));
        assert!(loader.contains("memory.grow(1)"));
        assert!(loader.contains("staleBufferDetached"));
        assert!(loader.contains("boxddRefreshMemoryViews"));
        assert!(loader.contains("providerHeapViewRefreshed"));
        assert!(loader.contains("postGrowthPhysicsStep"));
        assert!(loader.contains("BOXDD_BEVY_RUNTIME_EVIDENCE"));
        assert!(loader.contains("const shimImportPattern = /^import \\* as (import[0-9]+)"));
        assert!(loader.contains("importLines.some"));
        assert!(loader.contains("source.replaceAll(specifier, JSON.stringify(shimModuleUrl))"));
        assert!(verify < import);
        assert!(import < instantiate);
        assert!(bevy_testbed_loader_js(None).contains("const runtimeTrust = null;"));
    }

    #[test]
    fn bindgen_provider_import_rewrite_requires_one_contiguous_namespace_block() {
        let source = concat!(
            "/* wasm-bindgen output */\n",
            "import * as import1 from \"box2d-sys-v1-single\"\n",
            "import * as import2 from \"box2d-sys-v1-single\";\n",
            "export const ready = true;\n",
        );
        let patched = rewrite_bevy_bindgen_provider_imports(source, "box2d-sys-v1-single").unwrap();
        assert_eq!(
            patched.matches("from \"./box2d-provider-shim.js\"").count(),
            2
        );

        let interleaved = concat!(
            "import * as import1 from \"box2d-sys-v1-single\"\n",
            "const unrelated = true;\n",
            "import * as import2 from \"box2d-sys-v1-single\"\n",
        );
        assert!(rewrite_bevy_bindgen_provider_imports(interleaved, "box2d-sys-v1-single").is_err());
    }

    #[test]
    fn bindgen_provider_import_rewrite_preserves_metadata_and_rejects_non_import_replacements() {
        let metadata = concat!(
            "import * as import1 from \"box2d-sys-v1-single\"\n",
            "const imports = { \"box2d-sys-v1-single\": import1 };\n",
        );
        let patched =
            rewrite_bevy_bindgen_provider_imports(metadata, "box2d-sys-v1-single").unwrap();
        assert!(patched.contains("\"box2d-sys-v1-single\": import1"));

        let unsupported = concat!(
            "import * as import1 from \"box2d-sys-v1-single\"\n",
            "const unexpected = 'from \"box2d-sys-v1-single\"';\n",
        );
        assert!(rewrite_bevy_bindgen_provider_imports(unsupported, "box2d-sys-v1-single").is_err());

        let missing_handoff = "import * as import2 from \"box2d-sys-v1-single\"\n";
        assert!(
            rewrite_bevy_bindgen_provider_imports(missing_handoff, "box2d-sys-v1-single").is_err()
        );

        let duplicate_namespace = concat!(
            "import * as import1 from \"box2d-sys-v1-single\"\n",
            "import * as import1 from \"box2d-sys-v1-single\"\n",
        );
        assert!(
            rewrite_bevy_bindgen_provider_imports(duplicate_namespace, "box2d-sys-v1-single")
                .is_err()
        );

        let indented_namespace = " import * as import1 from \"box2d-sys-v1-single\"\n";
        assert!(
            rewrite_bevy_bindgen_provider_imports(indented_namespace, "box2d-sys-v1-single")
                .is_err()
        );
    }

    #[test]
    fn runtime_asset_validation_rejects_bytes_changed_after_manifest_generation() {
        let temp = tempfile::tempdir().unwrap();
        for (index, spec) in PAGES_RUNTIME_ASSETS.iter().enumerate() {
            let path = temp.path().join(spec.path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, [index as u8 + 1]).unwrap();
        }
        let mut manifest = manifest();
        manifest.assets = PAGES_RUNTIME_ASSETS
            .iter()
            .map(|spec| pages_runtime_asset(temp.path(), *spec).unwrap())
            .collect();
        validate_pages_runtime_asset_records(temp.path(), &manifest).unwrap();

        fs::write(temp.path().join(PAGES_RUNTIME_ASSETS[1].path), b"changed").unwrap();
        assert!(validate_pages_runtime_asset_records(temp.path(), &manifest).is_err());
    }
}
