use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use object::{Object, ObjectSection, ObjectSymbol};

#[allow(dead_code)]
#[path = "src/build_support/mod.rs"]
mod build_support;

// The producer only renders this shared protocol; repository tooling owns parsing.
#[allow(dead_code)]
#[path = "src/build_identity.rs"]
mod build_identity;

#[path = "src/adapter_contract.rs"]
mod adapter_contract;

#[path = "src/bindgen_contract.rs"]
mod bindgen_contract;

#[allow(dead_code)]
#[path = "src/provider_manifest.rs"]
mod provider_manifest;

#[path = "src/provenance_policy.rs"]
mod provenance_policy;

#[allow(dead_code)]
#[path = "src/provider_catalog.rs"]
mod provider_catalog;

#[allow(dead_code)]
#[path = "src/prebuilt_provenance.rs"]
mod prebuilt_provenance;

#[path = "src/provider_archive.rs"]
mod provider_archive;

#[allow(dead_code)]
#[path = "src/source_overlay.rs"]
mod source_overlay;

#[path = "src/wasm_provider_contract.rs"]
mod wasm_provider_contract;
#[path = "src/wasm_provider_memory.rs"]
mod wasm_provider_memory;

#[allow(dead_code)]
#[path = "src/precision.rs"]
mod precision;

use bindgen_contract::{
    ValidatedFreestandingHeaders, ValidatedWasiSysroot, WASI_LIBC_VERSION,
    resolve_unknown_unknown_headers, resolve_wasi_sysroot, validate_ambient_header_environment,
    validate_bindgen_target_override,
};
use build_identity::BuildIdentity;
use build_support::atomic_publish::publish_verified_file;
use build_support::provider_selection::{
    ProviderInputs, parse_optional_bool, parse_optional_unicode, select_provider,
    validate_force_bindgen_policy, validate_skip_cc_policy,
};
use build_support::target::{BindingTargetFamily, classify_binding_target, simd_identity};
use build_support::verified_snapshot::VerifiedFileSnapshot;
use build_support::{
    COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW, PrebuiltProvenance,
    SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH, SIGSTORE_TRUSTED_ROOT_SHA256, cosign_verify_blob_args,
    cosign_version_is_qualified,
};
use prebuilt_provenance::PrebuiltProvenanceStatement;
use precision::Precision;
use provider_archive::{
    ArchiveExpectation, private_abi_hash, private_abi_hash_hex, snapshot_layout_hash,
    verify_provider_archive,
};
use provider_catalog::ProviderCapability as ProviderAdapter;
use provider_manifest::{
    ArtifactExpectation, ArtifactIdentityExpectation, MAX_PROVIDER_ARCHIVE_BYTES,
    MAX_PROVIDER_BINDINGS_BYTES, RECORDING_CONTRACT_BLAKE3, REQUIRED_ADAPTER_SYMBOLS,
    VENDORED_SOURCE_IDENTITY_SHA256, VerifiedArtifact, verify_artifact,
};
use source_overlay::{MaterializedBuildInputs, materialize_build_inputs};
use wasm_provider_contract::{
    COMPILER_TARGET, ENDIANNESS, POINTER_WIDTH, PROVIDER_ABI, SIMD_MODE, WasmProviderExpectation,
    WasmProviderIdentity, contract_relative_path,
};

#[derive(Debug)]
struct BuildConfig {
    manifest_dir: PathBuf,
    #[cfg_attr(not(feature = "bindgen"), allow(dead_code))]
    out_dir: PathBuf,
    target_arch: String,
    target_env: String,
    target_os: String,
    target_features: String,
    target: String,
    binding_target: BindingTargetFamily,
    profile: String,
    is_docsrs: bool,
    skip_cc: bool,
    force_bindgen: bool,
    wasm_provider_final_link_opt_in: Option<OsString>,
    wasi_bindgen_sysroot: Option<ValidatedWasiSysroot>,
    freestanding_bindgen_headers: Option<ValidatedFreestandingHeaders>,
    provider: ProviderAdapter,
    precision: Precision,
}

fn run_output(command: &mut Command, label: &str) -> Result<std::process::Output, String> {
    command
        .output()
        .map_err(|error| format!("failed to run {label}: {error}"))
}

#[derive(Debug)]
struct PreparedExternal {
    artifact: VerifiedArtifact,
    native_abi_identity: NativeAbiIdentity,
    provenance_sha256: Option<String>,
    trusted_root_sha256: Option<String>,
}

#[derive(Debug)]
struct PreparedRustBindings {
    sha256: String,
}

#[derive(Debug)]
struct AuthenticatedPrebuiltProvenance {
    statement: PrebuiltProvenanceStatement,
    statement_sha256: String,
    trusted_root_sha256: String,
}

#[derive(Clone, Copy, Debug)]
struct NativeAbiIdentity {
    private_abi_hash: [u8; 32],
    snapshot_layout_hash: u32,
    definition_cookie: i32,
}

#[derive(Clone, Debug)]
struct CompiledIdentityProbe {
    identity: NativeAbiIdentity,
    object: PathBuf,
}

impl BuildConfig {
    fn from_env() -> Self {
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        let target = env::var("TARGET").expect("Cargo must provide TARGET");
        validate_bindgen_target_override(
            &target,
            env::var_os("BOXDD_SYS_BINDGEN_TARGET").as_deref(),
        )
        .unwrap_or_else(|error| panic!("invalid bindgen target assertion: {error}"));
        let binding_target = classify_binding_target(
            &target,
            &target_family,
            &target_arch,
            &target_os,
            &target_env,
        )
        .unwrap_or_else(|error| panic!("invalid checked-in binding target: {error}"));
        let docs_rs = parse_optional_bool("DOCS_RS", env::var_os("DOCS_RS").as_deref())
            .unwrap_or_else(|error| panic!("invalid optional build setting: {error}"));
        let cargo_cfg_docsrs = parse_optional_bool(
            "CARGO_CFG_DOCSRS",
            env::var_os("CARGO_CFG_DOCSRS").as_deref(),
        )
        .unwrap_or_else(|error| panic!("invalid optional build setting: {error}"));
        let is_docsrs = docs_rs || cargo_cfg_docsrs;
        let skip_cc = parse_optional_bool(
            "BOXDD_SYS_SKIP_CC",
            env::var_os("BOXDD_SYS_SKIP_CC").as_deref(),
        )
        .unwrap_or_else(|error| panic!("invalid optional build setting: {error}"));
        let force_bindgen = parse_optional_bool(
            "BOXDD_SYS_FORCE_BINDGEN",
            env::var_os("BOXDD_SYS_FORCE_BINDGEN").as_deref(),
        )
        .unwrap_or_else(|error| panic!("invalid optional build setting: {error}"));
        let wasm_provider_final_link_opt_in =
            env::var_os(wasm_provider_memory::FINAL_LINK_OPT_IN_ENV);
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let binding_generation_required = force_bindgen;
        validate_wasm_bindgen_header_environment(&target, binding_generation_required);
        let configured_wasi_sysroot = env::var_os("BOXDD_SYS_WASI_SYSROOT").map(PathBuf::from);
        let wasi_bindgen_sysroot = resolve_wasi_sysroot(
            &target,
            binding_generation_required,
            configured_wasi_sysroot.as_deref(),
        )
        .unwrap_or_else(|error| panic!("invalid WASI bindgen sysroot: {error}"));
        let freestanding_bindgen_headers =
            resolve_unknown_unknown_headers(&manifest_dir, &target, binding_generation_required)
                .unwrap_or_else(|error| panic!("invalid freestanding bindgen headers: {error}"));
        let explicit_provider_value = env::var_os("BOXDD_SYS_PROVIDER");
        let explicit_provider =
            parse_optional_unicode("BOXDD_SYS_PROVIDER", explicit_provider_value.as_deref())
                .unwrap_or_else(|error| panic!("invalid Box2D provider configuration: {error}"));
        let link_kind_value = env::var_os("BOXDD_SYS_LINK_KIND");
        let link_kind = parse_optional_unicode("BOXDD_SYS_LINK_KIND", link_kind_value.as_deref())
            .unwrap_or_else(|error| panic!("invalid Box2D provider configuration: {error}"));
        let provider = select_provider(ProviderInputs {
            target_arch: &target_arch,
            target_os: &target_os,
            explicit_provider,
            has_system_dir: env::var_os("BOX2D_LIB_DIR").is_some(),
            has_system_manifest: env::var_os("BOXDD_SYS_SYSTEM_MANIFEST").is_some(),
            has_prebuilt_manifest: env::var_os("BOXDD_SYS_PREBUILT_MANIFEST").is_some(),
            has_prebuilt_provenance: env::var_os("BOXDD_SYS_PREBUILT_PROVENANCE").is_some(),
            has_prebuilt_bundle: env::var_os("BOXDD_SYS_PREBUILT_BUNDLE").is_some(),
            has_prebuilt_trusted_root: env::var_os("BOXDD_SYS_PREBUILT_TRUSTED_ROOT").is_some(),
            // docs.rs still needs the vendored adapter for binding/type checking, but its
            // dedicated branch below skips the native compiler invocation.
            build_from_source_enabled: cfg!(feature = "build-from-source") || is_docsrs,
            link_kind,
        })
        .unwrap_or_else(|error| panic!("invalid Box2D provider configuration: {error}"));

        Self {
            manifest_dir,
            out_dir: PathBuf::from(env::var("OUT_DIR").unwrap()),
            target_arch,
            target_env,
            target_os,
            target_features,
            target,
            binding_target,
            profile: env::var("PROFILE").unwrap_or_else(|_| "release".into()),
            is_docsrs,
            skip_cc,
            force_bindgen,
            wasm_provider_final_link_opt_in,
            wasi_bindgen_sysroot,
            freestanding_bindgen_headers,
            provider,
            precision: Precision::ACTIVE,
        }
    }

    fn is_debug(&self) -> bool {
        self.profile == "debug"
    }

    fn pregenerated_bindings(&self) -> PathBuf {
        self.manifest_dir.join("src").join(
            self.binding_target
                .pregenerated_bindings_file(self.precision.is_double()),
        )
    }
}

fn validate_wasm_bindgen_header_environment(target: &str, binding_generation_required: bool) {
    let normalized_target = target.replace('-', "_");
    let mut variables = vec![
        "BINDGEN_EXTRA_CLANG_ARGS".to_owned(),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{target}"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{normalized_target}"),
        "CPATH".to_owned(),
        "C_INCLUDE_PATH".to_owned(),
        "CPLUS_INCLUDE_PATH".to_owned(),
        "OBJC_INCLUDE_PATH".to_owned(),
        "SDKROOT".to_owned(),
    ];
    variables.sort();
    variables.dedup();
    for variable in variables {
        println!("cargo:rerun-if-env-changed={variable}");
        validate_ambient_header_environment(
            target,
            binding_generation_required,
            &variable,
            env::var_os(&variable).as_deref(),
        )
        .unwrap_or_else(|error| panic!("invalid bindgen header environment: {error}"));
    }
}

fn validate_adapter_abi_header(manifest_dir: &Path) {
    const MACRO_NAME: &str = "BOXDD_ADAPTER_ABI_VERSION";

    let path = manifest_dir.join("native/boxdd_adapter.h");
    let header = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let definitions = header
        .lines()
        .filter(|line| line.split_whitespace().nth(1) == Some(MACRO_NAME))
        .collect::<Vec<_>>();
    let expected = format!(
        "#define {MACRO_NAME} {}u",
        adapter_contract::ADAPTER_ABI_VERSION
    );
    assert_eq!(
        definitions,
        [expected.as_str()],
        "{} must contain exactly one canonical {MACRO_NAME} definition derived from the Rust adapter contract",
        path.display()
    );
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(boxdd_sys_wasm_provider)");
    println!("cargo:rerun-if-changed=effective-source.toml");
    println!("cargo:rerun-if-changed=patches");
    println!("cargo:rerun-if-changed=upstream.toml");
    println!("cargo:rerun-if-changed=abi/wasm32-unknown-unknown-single.toml");
    println!("cargo:rerun-if-changed=abi/wasm32-unknown-unknown-double.toml");
    println!("cargo:rerun-if-changed=src/bindings_pregenerated.rs");
    println!("cargo:rerun-if-changed=src/bindings_double.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_unknown_unknown.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_unknown_unknown_double.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_wasip1.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_wasip1_double.rs");
    println!("cargo:rerun-if-changed=src/bindgen_headers/wasm32_unknown_unknown");
    println!("cargo:rerun-if-changed=src/bindgen_headers/wasm32_unknown_unknown/math.h");
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-changed={SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH}");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SKIP_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PROVIDER");
    println!(
        "cargo:rerun-if-env-changed={}",
        wasm_provider_memory::FINAL_LINK_OPT_IN_ENV
    );
    println!("cargo:rerun-if-env-changed=BOX2D_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SYSTEM_MANIFEST");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_MANIFEST");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_PROVENANCE");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_BUNDLE");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_TRUSTED_ROOT");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_COSIGN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_LINK_KIND");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_FORCE_BINDGEN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_BINDGEN_TARGET");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASI_SYSROOT");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_DOCSRS");

    let config = BuildConfig::from_env();
    validate_adapter_abi_header(&config.manifest_dir);
    validate_build_config(&config);
    if let Some(wasi_sysroot) = &config.wasi_bindgen_sysroot {
        println!(
            "cargo:rerun-if-changed={}",
            wasi_sysroot.headers_root.display()
        );
        println!(
            "cargo:wasi_sysroot_identity_sha256={}",
            wasi_sysroot.identity_sha256()
        );
        println!("cargo:wasi_libc_version={WASI_LIBC_VERSION}");
    }
    if let Some(headers) = &config.freestanding_bindgen_headers {
        println!(
            "cargo:freestanding_math_header_identity_sha256={}",
            headers.identity_sha256()
        );
    }
    reject_precision_macro_overrides(&config.target);
    let build_inputs = materialize_build_inputs(&config.manifest_dir, &config.out_dir)
        .unwrap_or_else(|error| panic!("failed to capture Box2D build inputs: {error}"));
    assert_eq!(
        build_inputs.vendored_source_sha256, VENDORED_SOURCE_IDENTITY_SHA256,
        "vendored Box2D source content/inventory does not match the reviewed identity"
    );
    let effective_source = &build_inputs.effective.identity;
    let adapter_source_sha256 = &build_inputs.adapter.adapter_source_sha256;
    let external = prepare_external_provider(&config, &build_inputs);
    let rust_bindings = prepare_rust_bindings(&config, &build_inputs, external.as_ref());

    if config.provider == ProviderAdapter::WasmProvider {
        println!("cargo:rustc-cfg=boxdd_sys_wasm_provider");
    }

    let unavailable_identity = NativeAbiIdentity {
        private_abi_hash: [0; 32],
        snapshot_layout_hash: 0,
        definition_cookie: 0,
    };
    let native_abi_identity = if config.is_docsrs {
        println!("cargo:warning=DOCS_RS detected: skipping native Box2D C build");
        unavailable_identity
    } else if config.skip_cc {
        println!("cargo:warning=Skipping native Box2D C build due to BOXDD_SYS_SKIP_CC");
        unavailable_identity
    } else {
        match config.provider {
            ProviderAdapter::WasmCompileOnly => {
                println!(
                    "cargo:warning=boxdd-sys is using compile-only WASM mode; no Box2D runtime is linked"
                );
                unavailable_identity
            }
            ProviderAdapter::WasmProvider => {
                println!(
                    "cargo:warning=boxdd-sys WASM provider mode is active; runtime identity must be verified before instantiation"
                );
                load_wasm_provider_identity_contract(
                    &config,
                    &effective_source.upstream_sha,
                    &effective_source.source_tree,
                    &effective_source.effective_source_sha256,
                    adapter_source_sha256,
                    &rust_bindings.sha256,
                )
            }
            ProviderAdapter::Vendored => build_box2d_from_source(&config, &build_inputs),
            ProviderAdapter::System | ProviderAdapter::Prebuilt => {
                let prepared = external.as_ref().expect("external provider was prepared");
                link_verified_artifact(&config, &prepared.artifact);
                prepared.native_abi_identity
            }
        }
    };
    write_expected_adapter_identity(&config.out_dir, native_abi_identity);

    let wasm_import_module = config.precision.wasm_import_module();
    emit_build_identity(
        &config,
        &build_inputs,
        &rust_bindings.sha256,
        native_abi_identity,
        wasm_import_module,
        external.as_ref(),
    );

    build_inputs
        .revalidate()
        .unwrap_or_else(|error| panic!("Box2D build inputs changed while in use: {error}"));
}

fn validate_build_config(config: &BuildConfig) {
    if config.provider.is_wasm() != (config.target_arch == "wasm32") {
        panic!(
            "provider `{}` does not match target architecture `{}`",
            config.provider.as_str(),
            config.target_arch
        );
    }
    validate_skip_cc_policy(
        config.is_docsrs,
        config.skip_cc,
        config.force_bindgen,
        config.provider,
    )
    .unwrap_or_else(|error| panic!("invalid BOXDD_SYS_SKIP_CC configuration: {error}"));
    validate_force_bindgen_policy(config.force_bindgen, config.provider)
        .unwrap_or_else(|error| panic!("invalid BOXDD_SYS_FORCE_BINDGEN configuration: {error}"));
    if config.provider == ProviderAdapter::WasmProvider && cfg!(feature = "validate") {
        panic!(
            "BOXDD_SYS_PROVIDER=wasm-provider does not have a checked validation-enabled ABI route"
        );
    }
    wasm_provider_memory::validate_final_link_opt_in(
        config.provider == ProviderAdapter::WasmProvider,
        config.wasm_provider_final_link_opt_in.as_deref(),
    )
    .unwrap_or_else(|error| panic!("invalid WASM provider final-link configuration: {error}"));
}

fn prepare_rust_bindings(
    config: &BuildConfig,
    build_inputs: &MaterializedBuildInputs,
    external: Option<&PreparedExternal>,
) -> PreparedRustBindings {
    if config.force_bindgen {
        #[cfg(feature = "bindgen")]
        {
            let bytes = generate_bindings(
                &build_inputs.effective.public_include,
                &config.target,
                config.precision,
                config
                    .wasi_bindgen_sysroot
                    .as_ref()
                    .map(|sysroot| sysroot.canonical_path.as_path()),
                config
                    .freestanding_bindgen_headers
                    .as_ref()
                    .map(|headers| headers.canonical_path.as_path()),
            );
            let sha256 = provider_manifest::sha256_bytes(&bytes);
            return publish_rust_bindings(&config.out_dir, &bytes, &sha256);
        }
        #[cfg(not(feature = "bindgen"))]
        {
            let _ = build_inputs;
            panic!("BOXDD_SYS_FORCE_BINDGEN=1 requires the `bindgen` feature");
        }
    }

    if let Some(prepared) = external {
        let snapshot = &prepared.artifact.bindings_snapshot;
        return publish_rust_bindings(&config.out_dir, snapshot.bytes(), snapshot.sha256());
    }

    let snapshot = VerifiedFileSnapshot::read(
        &config.pregenerated_bindings(),
        MAX_PROVIDER_BINDINGS_BYTES,
        "checked Rust FFI bindings",
    )
    .unwrap_or_else(|error| {
        panic!("checked Rust FFI bindings are required unless BOXDD_SYS_FORCE_BINDGEN=1: {error}")
    });
    publish_rust_bindings(&config.out_dir, snapshot.bytes(), snapshot.sha256())
}

fn publish_rust_bindings(out_dir: &Path, bytes: &[u8], sha256: &str) -> PreparedRustBindings {
    let byte_count = u64::try_from(bytes.len())
        .unwrap_or_else(|_| panic!("Rust FFI bindings length does not fit in u64"));
    assert!(
        byte_count <= MAX_PROVIDER_BINDINGS_BYTES,
        "Rust FFI bindings exceed the {MAX_PROVIDER_BINDINGS_BYTES} byte limit"
    );
    let path = out_dir.join(format!("boxdd-bindings-{sha256}.rs"));
    publish_verified_file(&path, sha256, bytes, "Rust FFI bindings")
        .unwrap_or_else(|error| panic!("failed to publish Rust FFI bindings snapshot: {error}"));
    println!("cargo:rustc-env=BOXDD_SYS_BINDINGS_FILE={}", path.display());
    println!("cargo:bindings_sha256={sha256}");
    PreparedRustBindings {
        sha256: sha256.to_owned(),
    }
}

fn emit_build_identity(
    config: &BuildConfig,
    build_inputs: &MaterializedBuildInputs,
    bindings_sha256: &str,
    native_abi_identity: NativeAbiIdentity,
    wasm_import_module: &str,
    external: Option<&PreparedExternal>,
) {
    let effective_source = &build_inputs.effective.identity;
    let upstream_sha = &effective_source.upstream_sha;
    let effective_source_sha256 = &effective_source.effective_source_sha256;
    let adapter_source_sha256 = &build_inputs.adapter.adapter_source_sha256;
    let vendored_archive = if config.provider == ProviderAdapter::Vendored
        && native_abi_identity.private_abi_hash != [0; 32]
        && native_abi_identity.snapshot_layout_hash != 0
    {
        let archive = config.out_dir.join(if config.target_env == "msvc" {
            "box2d.lib"
        } else {
            "libbox2d.a"
        });
        Some(
            VerifiedFileSnapshot::read(
                &archive,
                MAX_PROVIDER_ARCHIVE_BYTES,
                "vendored build archive identity",
            )
            .unwrap_or_else(|error| {
                panic!(
                    "failed to capture vendored build archive {}: {error}",
                    archive.display()
                )
            }),
        )
    } else {
        None
    };
    let archive_sha256 = external
        .map(|prepared| prepared.artifact.archive_snapshot.sha256())
        .or_else(|| vendored_archive.as_ref().map(VerifiedFileSnapshot::sha256))
        .unwrap_or("");
    println!("cargo:rustc-env=BOXDD_SYS_UPSTREAM_SHA={upstream_sha}");
    println!("cargo:rustc-env=BOXDD_SYS_EFFECTIVE_SOURCE_SHA256={effective_source_sha256}");
    println!("cargo:rustc-env=BOXDD_SYS_ADAPTER_SOURCE_SHA256={adapter_source_sha256}");
    println!("cargo:rustc-env=BOXDD_SYS_RECORDING_CONTRACT_BLAKE3={RECORDING_CONTRACT_BLAKE3}");
    println!("cargo:rustc-env=BOXDD_SYS_TARGET_ABI={}", config.target);
    println!("cargo:rustc-env=BOXDD_SYS_WASM_IMPORT_MODULE={wasm_import_module}");
    println!(
        "cargo:rustc-env=BOXDD_SYS_PROVIDER_ADAPTER={}",
        config.provider.as_str()
    );
    println!(
        "cargo:rustc-env=BOXDD_SYS_PROVIDER_MANIFEST_SHA256={}",
        external
            .map(|prepared| prepared.artifact.manifest_snapshot.sha256())
            .unwrap_or("")
    );
    println!(
        "cargo:rustc-env=BOXDD_SYS_PROVIDER_ARCHIVE_SHA256={}",
        external
            .map(|prepared| prepared.artifact.archive_snapshot.sha256())
            .unwrap_or("")
    );
    println!(
        "cargo:rustc-env=BOXDD_SYS_PROVIDER_PROVENANCE_SHA256={}",
        external
            .and_then(|prepared| prepared.provenance_sha256.as_deref())
            .unwrap_or("")
    );
    println!(
        "cargo:rustc-env=BOXDD_SYS_PROVIDER_TRUSTED_ROOT_SHA256={}",
        external
            .and_then(|prepared| prepared.trusted_root_sha256.as_deref())
            .unwrap_or("")
    );
    println!("cargo:precision={}", config.precision.as_str());
    println!("cargo:upstream_sha={upstream_sha}");
    println!("cargo:effective_source_sha256={effective_source_sha256}");
    println!("cargo:provider={}", config.provider.as_str());
    println!("cargo:wasm_import_module={wasm_import_module}");

    // Repository tooling consumes this local marker instead of inferring target ABI state from its
    // own compilation. It contains no secrets and is intentionally deterministic.
    let private_abi_hash = private_abi_hash_hex(native_abi_identity.private_abi_hash);
    let identity = BuildIdentity {
        provider: config.provider,
        crate_version: env!("CARGO_PKG_VERSION").to_owned(),
        upstream_sha: upstream_sha.to_owned(),
        effective_source_sha256: effective_source_sha256.to_owned(),
        precision: config.precision.as_str().to_owned(),
        target: config.target.clone(),
        crt: expected_crt_identity(config).to_owned(),
        simd: expected_simd_identity(config).to_owned(),
        validate: cfg!(feature = "validate"),
        adapter_source_sha256: adapter_source_sha256.to_owned(),
        private_abi_hash,
        snapshot_layout_hash: native_abi_identity.snapshot_layout_hash,
        bindings_sha256: bindings_sha256.to_owned(),
        manifest_sha256: external
            .map(|prepared| prepared.artifact.manifest_snapshot.sha256().to_owned())
            .unwrap_or_default(),
        archive_sha256: archive_sha256.to_owned(),
        provenance_sha256: external
            .and_then(|prepared| prepared.provenance_sha256.clone())
            .unwrap_or_default(),
        trusted_root_sha256: external
            .and_then(|prepared| prepared.trusted_root_sha256.clone())
            .unwrap_or_default(),
    }
    .render()
    .unwrap_or_else(|error| panic!("failed to render boxdd build identity marker: {error}"));
    fs::write(
        config.out_dir.join(build_identity::BUILD_IDENTITY_FILE),
        identity,
    )
    .expect("failed to write boxdd build identity marker");
    if let Some(archive) = vendored_archive {
        archive
            .revalidate("vendored build archive identity cohort")
            .unwrap_or_else(|error| panic!("vendored build archive changed: {error}"));
    }
}

fn prepare_external_provider(
    config: &BuildConfig,
    build_inputs: &MaterializedBuildInputs,
) -> Option<PreparedExternal> {
    let (provider, manifest_key) = match config.provider {
        ProviderAdapter::System => (
            ProviderAdapter::System.as_str(),
            "BOXDD_SYS_SYSTEM_MANIFEST",
        ),
        ProviderAdapter::Prebuilt => (
            ProviderAdapter::Prebuilt.as_str(),
            "BOXDD_SYS_PREBUILT_MANIFEST",
        ),
        _ => return None,
    };
    let effective_sources = &build_inputs.effective;
    let effective_source = &effective_sources.identity;
    let adapter_source_sha256 = &build_inputs.adapter.adapter_source_sha256;
    let manifest_path = PathBuf::from(
        env::var(manifest_key)
            .unwrap_or_else(|_| panic!("{manifest_key} is required for the {provider} provider")),
    );
    let header_path = effective_sources.public_include.join("box2d/box2d.h");
    let bindings_path = config.pregenerated_bindings();
    let native_abi_identity = compile_adapter_identity_probe(config, build_inputs).identity;
    let private_abi_hash = private_abi_hash_hex(native_abi_identity.private_abi_hash);
    let expectation = ArtifactExpectation {
        identity: ArtifactIdentityExpectation {
            provider,
            crate_version: env!("CARGO_PKG_VERSION"),
            upstream_sha: &effective_source.upstream_sha,
            effective_source_sha256: &effective_source.effective_source_sha256,
            precision: config.precision.as_str(),
            target: &config.target,
            crt: expected_crt_identity(config),
            simd: expected_simd_identity(config),
            validate: cfg!(feature = "validate"),
            adapter_source_sha256,
            private_abi_hash: &private_abi_hash,
            snapshot_layout_hash: native_abi_identity.snapshot_layout_hash,
        },
        header_path: &header_path,
        bindings_path: &bindings_path,
    };
    let authenticated_prebuilt = if config.provider == ProviderAdapter::Prebuilt {
        Some(authenticate_prebuilt_provenance(config))
    } else {
        None
    };
    let verified = verify_artifact(&manifest_path, &expectation).unwrap_or_else(|error| {
        panic!(
            "failed to verify {provider} provider manifest {}: {error}",
            manifest_path.display()
        )
    });
    let verified_archive = verify_provider_archive(
        &verified.archive_snapshot,
        &ArchiveExpectation {
            target: &config.target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256: &effective_source.effective_source_sha256,
            private_abi_hash: &private_abi_hash,
            snapshot_layout_hash: native_abi_identity.snapshot_layout_hash,
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "failed to prove {provider} provider archive {}: {error}",
            verified.archive_snapshot.path().display()
        )
    });
    assert_eq!(
        verified_archive.archive_sha256, verified.manifest.archive_sha256,
        "provider archive changed between manifest and structural verification"
    );

    if config.provider == ProviderAdapter::System {
        let lib_dir = PathBuf::from(
            env::var("BOX2D_LIB_DIR").expect("BOX2D_LIB_DIR is required for the system provider"),
        );
        let expected_dir = fs::canonicalize(&lib_dir).unwrap_or_else(|error| {
            panic!(
                "failed to resolve BOX2D_LIB_DIR {}: {error}",
                lib_dir.display()
            )
        });
        let archive_dir = fs::canonicalize(
            verified
                .archive_snapshot
                .path()
                .parent()
                .expect("verified archive must have a parent"),
        )
        .expect("failed to resolve verified archive directory");
        assert_eq!(
            archive_dir, expected_dir,
            "system manifest archive must be the exact archive inside BOX2D_LIB_DIR"
        );
    }

    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        verified.archive_snapshot.path().display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        verified.header_snapshot.path().display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        verified.bindings_snapshot.path().display()
    );
    let (provenance_sha256, trusted_root_sha256) =
        if let Some(authenticated) = authenticated_prebuilt {
            validate_authenticated_prebuilt_provenance(&authenticated, &verified);
            (
                Some(authenticated.statement_sha256),
                Some(authenticated.trusted_root_sha256),
            )
        } else {
            (None, None)
        };
    Some(PreparedExternal {
        artifact: verified,
        native_abi_identity,
        provenance_sha256,
        trusted_root_sha256,
    })
}

fn authenticate_prebuilt_provenance(config: &BuildConfig) -> AuthenticatedPrebuiltProvenance {
    let statement_snapshot = snapshot_prebuilt_input(
        config,
        "BOXDD_SYS_PREBUILT_PROVENANCE",
        "statement",
        "toml",
        4 * 1024 * 1024,
    );
    let statement = PrebuiltProvenanceStatement::parse_canonical(statement_snapshot.bytes())
        .unwrap_or_else(|error| panic!("invalid prebuilt provenance statement: {error}"));
    statement
        .validate_publisher(PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW)
        .unwrap_or_else(|error| panic!("untrusted prebuilt provenance statement: {error}"));
    let bundle_snapshot = snapshot_prebuilt_input(
        config,
        "BOXDD_SYS_PREBUILT_BUNDLE",
        "bundle",
        "json",
        16 * 1024 * 1024,
    );
    let (trusted_root_snapshot, trusted_root_sha256) = trusted_root_file(config);
    let cosign = env::var_os("BOXDD_SYS_COSIGN").unwrap_or_else(|| "cosign".into());
    let mut version_command = Command::new(&cosign);
    version_command.arg("version");
    let version_output = run_output(&mut version_command, "Cosign version qualification")
    .unwrap_or_else(|error| {
        panic!(
            "prebuilt provider requires Cosign {COSIGN_VERSION}; failed to execute {:?}: {error}",
            cosign
        )
    });
    assert!(
        version_output.status.success(),
        "prebuilt provider requires Cosign {COSIGN_VERSION}; version command failed: {}",
        String::from_utf8_lossy(&version_output.stderr).trim()
    );
    let version_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&version_output.stdout),
        String::from_utf8_lossy(&version_output.stderr)
    );
    assert!(
        cosign_version_is_qualified(&version_text),
        "prebuilt provider requires exact Cosign {COSIGN_VERSION}; found {}",
        version_text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("unknown version")
    );

    let args = cosign_verify_blob_args(PrebuiltProvenance {
        crate_version: env!("CARGO_PKG_VERSION"),
        source_commit: &statement.source_commit,
        release_tag: &statement.release_tag,
        payload: statement_snapshot.path(),
        bundle: bundle_snapshot.path(),
        trusted_root: trusted_root_snapshot.path(),
    })
    .expect("validated prebuilt manifest must produce a provenance policy");
    let mut verification = Command::new(&cosign);
    verification.args(args);
    let output =
        run_output(&mut verification, "Cosign provenance verification").unwrap_or_else(|error| {
            panic!("failed to execute Cosign provenance verification: {error}")
        });
    assert!(
        output.status.success(),
        "prebuilt provider provenance verification failed before linking: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    statement_snapshot
        .revalidate("authenticated prebuilt provenance statement")
        .unwrap_or_else(|error| panic!("Cosign input changed during verification: {error}"));
    bundle_snapshot
        .revalidate("authenticated prebuilt Sigstore bundle")
        .unwrap_or_else(|error| panic!("Cosign input changed during verification: {error}"));
    trusted_root_snapshot
        .revalidate("authenticated prebuilt Sigstore trusted root")
        .unwrap_or_else(|error| panic!("Cosign input changed during verification: {error}"));
    AuthenticatedPrebuiltProvenance {
        statement,
        statement_sha256: statement_snapshot.sha256().to_owned(),
        trusted_root_sha256,
    }
}

fn validate_authenticated_prebuilt_provenance(
    authenticated: &AuthenticatedPrebuiltProvenance,
    artifact: &VerifiedArtifact,
) {
    let statement_manifest = authenticated
        .statement
        .validate_provider_manifest(artifact.manifest_snapshot.bytes())
        .unwrap_or_else(|error| panic!("prebuilt provenance manifest mismatch: {error}"));
    assert_eq!(
        statement_manifest, artifact.manifest,
        "prebuilt provenance did not authenticate the verified provider manifest"
    );
    let provider_root = artifact
        .manifest_snapshot
        .path()
        .parent()
        .expect("verified prebuilt manifest must have a parent directory");
    authenticated
        .statement
        .verify_extracted_root(provider_root)
        .unwrap_or_else(|error| panic!("prebuilt provenance inventory mismatch: {error}"));
}

fn snapshot_prebuilt_input(
    config: &BuildConfig,
    key: &str,
    label: &str,
    extension: &str,
    maximum_bytes: u64,
) -> VerifiedFileSnapshot {
    let source = PathBuf::from(
        env::var_os(key).unwrap_or_else(|| panic!("{key} is required for the prebuilt provider")),
    );
    snapshot_prebuilt_file(config, &source, key, label, extension, maximum_bytes)
}

fn snapshot_prebuilt_file(
    config: &BuildConfig,
    source: &Path,
    source_description: &str,
    label: &str,
    extension: &str,
    maximum_bytes: u64,
) -> VerifiedFileSnapshot {
    let snapshot = VerifiedFileSnapshot::read(source, maximum_bytes, source_description)
        .unwrap_or_else(|error| panic!("failed to snapshot {source_description}: {error}"));
    assert!(
        !snapshot.is_empty(),
        "{source_description} must not be empty: {}",
        source.display()
    );
    let digest = snapshot.sha256().to_owned();
    let destination = config
        .out_dir
        .join(format!("boxdd-prebuilt-{label}-{}.{}", digest, extension));
    publish_verified_file(
        &destination,
        &digest,
        snapshot.bytes(),
        &format!("prebuilt {label} snapshot"),
    )
    .unwrap_or_else(|error| panic!("failed to publish prebuilt {label} snapshot: {error}"));
    let published = VerifiedFileSnapshot::read(
        &destination,
        u64::try_from(snapshot.len()).expect("prebuilt snapshot length must fit u64"),
        &format!("published prebuilt {label} snapshot"),
    )
    .unwrap_or_else(|error| panic!("failed to retain prebuilt {label} snapshot: {error}"));
    published
        .verify_exact(
            snapshot.bytes(),
            snapshot.sha256(),
            &format!("published prebuilt {label} snapshot"),
        )
        .unwrap_or_else(|error| panic!("prebuilt {label} snapshot changed: {error}"));
    println!("cargo:rerun-if-changed={}", source.display());
    published
}

fn trusted_root_file(config: &BuildConfig) -> (VerifiedFileSnapshot, String) {
    let (description, path) = match env::var_os("BOXDD_SYS_PREBUILT_TRUSTED_ROOT") {
        Some(path) => ("BOXDD_SYS_PREBUILT_TRUSTED_ROOT", PathBuf::from(path)),
        None => (
            "crate-owned Sigstore trusted root",
            config
                .manifest_dir
                .join(SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH),
        ),
    };
    let snapshot = snapshot_prebuilt_file(
        config,
        &path,
        description,
        "trusted-root",
        "json",
        4 * 1024 * 1024,
    );
    let digest = snapshot.sha256().to_owned();
    assert_eq!(
        digest, SIGSTORE_TRUSTED_ROOT_SHA256,
        "{description} does not match the crate-owned authenticated Sigstore trusted root"
    );
    (snapshot, digest)
}

fn link_verified_artifact(config: &BuildConfig, artifact: &VerifiedArtifact) {
    let archive_sha256 = artifact.archive_snapshot.sha256();
    let link_name = format!("box2d_{archive_sha256}");
    let file_name = if config.target_env == "msvc" {
        format!("{link_name}.lib")
    } else {
        format!("lib{link_name}.a")
    };
    let linked_archive = config.out_dir.join(file_name);
    publish_verified_file(
        &linked_archive,
        archive_sha256,
        artifact.archive_snapshot.bytes(),
        "prebuilt link archive",
    )
    .unwrap_or_else(|error| panic!("failed to publish verified link archive: {error}"));
    println!(
        "cargo:rustc-link-search=native={}",
        config.out_dir.display()
    );
    println!("cargo:rustc-link-lib=static={link_name}");
}

fn expected_simd_identity(config: &BuildConfig) -> &'static str {
    simd_identity(
        &config.target_arch,
        cfg!(feature = "disable-simd"),
        cfg!(feature = "simd-avx2"),
    )
}

fn expected_crt_identity(config: &BuildConfig) -> &'static str {
    if config.target_os == "windows" && config.target_env == "msvc" {
        if config
            .target_features
            .split(',')
            .any(|feature| feature == "crt-static")
        {
            "mt"
        } else {
            "md"
        }
    } else {
        "none"
    }
}

fn reject_precision_macro_overrides(target: &str) {
    let normalized_target = target.replace('-', "_");
    let target_keys = [
        format!("CFLAGS_{target}"),
        format!("CFLAGS_{normalized_target}"),
        format!("{target}_CFLAGS"),
        format!("{normalized_target}_CFLAGS"),
        "HOST_CFLAGS".to_owned(),
        "TARGET_CFLAGS".to_owned(),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{target}"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{normalized_target}"),
    ];
    for key in [
        "CFLAGS",
        "CPPFLAGS",
        "CL",
        "CPATH",
        "C_INCLUDE_PATH",
        "CPLUS_INCLUDE_PATH",
        "OBJC_INCLUDE_PATH",
        "SDKROOT",
        "INCLUDE",
        "BINDGEN_EXTRA_CLANG_ARGS",
        "CRATE_CC_NO_DEFAULTS",
        "CC_SHELL_ESCAPED_FLAGS",
        "CC_FORCE_DISABLE",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(target_keys)
    {
        println!("cargo:rerun-if-env-changed={key}");
        let Some(value) = env::var_os(&key) else {
            continue;
        };
        if value.to_string_lossy().contains("BOX2D_DOUBLE_PRECISION") {
            panic!(
                "{key} must not override BOX2D_DOUBLE_PRECISION; use the `double-precision` Cargo feature so C and Rust select one ABI"
            );
        }
    }
}

#[cfg(feature = "bindgen")]
#[derive(Debug)]
struct BoxddBindgenCallbacks;

#[cfg(feature = "bindgen")]
impl bindgen::callbacks::ParseCallbacks for BoxddBindgenCallbacks {
    fn header_file(&self, filename: &str) {
        println!("cargo:rerun-if-changed={filename}");
    }

    fn include_file(&self, filename: &str) {
        println!("cargo:rerun-if-changed={filename}");
    }

    fn read_env_var(&self, key: &str) {
        println!("cargo:rerun-if-env-changed={key}");
    }

    fn process_comment(&self, comment: &str) -> Option<String> {
        Some(
            comment
                .replace("[0,tMax]", r"\[0,tMax\]")
                .replace("[0,1]", r"\[0,1\]")
                .replace("https://semver.org/", "<https://semver.org/>")
                .replace(
                    "https://en.wikipedia.org/wiki/Coefficient_of_restitution",
                    "<https://en.wikipedia.org/wiki/Coefficient_of_restitution>",
                )
                .replace(
                    "https://en.wikipedia.org/wiki/Polygonal_chain",
                    "<https://en.wikipedia.org/wiki/Polygonal_chain>",
                )
                .replace(
                    "https://www.rapidtables.com/web/color/index.html",
                    "<https://www.rapidtables.com/web/color/index.html>",
                )
                .replace(
                    "https://johndecember.com/html/spec/colorsvg.html",
                    "<https://johndecember.com/html/spec/colorsvg.html>",
                )
                .replace(
                    "https://upload.wikimedia.org/wikipedia/commons/2/2b/SVG_Recognized_color_keyword_names.svg",
                    "<https://upload.wikimedia.org/wikipedia/commons/2/2b/SVG_Recognized_color_keyword_names.svg>",
                ),
        )
    }
}

#[cfg(feature = "bindgen")]
fn generate_bindings(
    effective_public_include: &Path,
    target: &str,
    precision: Precision,
    wasi_sysroot: Option<&Path>,
    freestanding_headers: Option<&Path>,
) -> Vec<u8> {
    let header = effective_public_include.join("box2d").join("box2d.h");
    let builder = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .parse_callbacks(Box::new(BoxddBindgenCallbacks))
        .clang_args(["-x", "c", "-std=c17"])
        .clang_arg(format!("--target={target}"))
        .clang_args(
            freestanding_headers
                .map(|headers| vec![format!("-I{}", headers.display())])
                .unwrap_or_default(),
        )
        .clang_arg(format!("-I{}", effective_public_include.display()))
        .wasm_import_module_name(precision.wasm_import_module())
        .allowlist_function("b2.*")
        .allowlist_type("b2.*")
        .allowlist_var("B2_.*")
        .blocklist_var("^B2_DEFAULT_MASK_BITS$")
        .blocklist_var("^B2_ENABLE_VALIDATION$")
        .layout_tests(false);
    let builder = if matches!(target, "wasm32-unknown-unknown" | "wasm32-wasip1") {
        builder.clang_arg("-Dbox2d_EXPORTS=1")
    } else {
        builder
    };
    let builder = match wasi_sysroot {
        Some(sysroot) => builder.clang_arg(format!("--sysroot={}", sysroot.display())),
        None => builder,
    };
    let builder = match precision {
        Precision::Single => builder.clang_arg("-UBOX2D_DOUBLE_PRECISION"),
        Precision::Double => builder.clang_arg("-DBOX2D_DOUBLE_PRECISION=1"),
    };
    let bindings = configure_bindgen_host_headers(builder, target)
        .generate()
        .expect("failed to generate Box2D bindings");
    bindings.to_string().into_bytes()
}

#[cfg(feature = "bindgen")]
fn configure_bindgen_host_headers(builder: bindgen::Builder, target: &str) -> bindgen::Builder {
    if !cfg!(target_os = "macos") || !target.contains("-linux-") {
        return builder;
    }

    // Apple Clang has no Linux libc sysroot. The Box2D public API only needs ISO C headers here;
    // Xcode supplies those headers while `--target` above remains the manifest's Linux ABI.
    let mut command = std::process::Command::new("/usr/bin/xcrun");
    command.args(["--sdk", "macosx", "--show-sdk-path"]);
    let output = run_output(&mut command, "xcrun macOS SDK lookup")
        .expect("xcrun is required to locate ISO C headers for Linux-target bindgen on macOS");
    assert!(
        output.status.success(),
        "xcrun could not locate the macOS SDK for Linux-target bindgen: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let sdk = String::from_utf8(output.stdout)
        .expect("xcrun SDK path must be UTF-8")
        .trim()
        .to_owned();
    assert!(!sdk.is_empty(), "xcrun returned an empty macOS SDK path");
    builder.clang_arg(format!("--sysroot={sdk}"))
}

#[cfg(not(feature = "bindgen"))]
#[allow(dead_code)]
fn generate_bindings(
    _effective_public_include: &Path,
    _target: &str,
    _precision: Precision,
    _wasi_sysroot: Option<&Path>,
    _freestanding_headers: Option<&Path>,
) -> Vec<u8> {
    unreachable!("generate_bindings is only available with the `bindgen` feature enabled");
}

fn configure_msvc_language(build: &mut cc::Build) {
    // C17 is a hard Box2D requirement. Let the real compilation report an unsupported compiler;
    // `is_flag_supported` cannot distinguish a rejected flag from an unrelated probe failure.
    build.flag("/std:c17");
    if build.get_compiler().is_like_clang_cl() {
        build.flag("/clang:-ffp-contract=off");
    }
}

fn c_string_define(value: &str, name: &str) -> String {
    assert!(
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')),
        "{name} contains characters that cannot be embedded in a C identity literal"
    );
    format!("\"{value}\"")
}

fn build_box2d_from_source(
    config: &BuildConfig,
    build_inputs: &MaterializedBuildInputs,
) -> NativeAbiIdentity {
    let effective_sources = &build_inputs.effective;
    let adapter_sources = &build_inputs.adapter;
    let effective_source = &effective_sources.identity;
    let adapter_source_sha256 = &adapter_sources.adapter_source_sha256;
    let mut build = cc::Build::new();
    build.include(&effective_sources.public_include);
    build.include(&effective_sources.private_include);
    build.include(&adapter_sources.native_include);
    for source in &effective_sources.c_sources {
        assert!(
            source.is_file(),
            "Box2D source declared by upstream.toml is missing: {}",
            source.display()
        );
        build.file(source);
    }
    for source in &adapter_sources.c_sources {
        assert!(
            source.is_file(),
            "captured adapter source is missing: {}",
            source.display()
        );
        build.file(source);
    }

    let identity_probe = compile_adapter_identity_probe(config, build_inputs);
    build.object(&identity_probe.object);

    let upstream_define = c_string_define(&effective_source.upstream_sha, "upstream SHA");
    let target_define = c_string_define(&config.target, "target ABI");
    let adapter_digest_define = c_string_define(adapter_source_sha256, "adapter source SHA-256");
    let recording_digest_define =
        c_string_define(RECORDING_CONTRACT_BLAKE3, "recording contract BLAKE3");
    build.define("BOXDD_UPSTREAM_SHA", Some(upstream_define.as_str()));
    build.define("BOXDD_TARGET_ABI", Some(target_define.as_str()));
    build.define(
        "BOXDD_ADAPTER_SOURCE_SHA256",
        Some(adapter_digest_define.as_str()),
    );
    define_effective_source_identity(&mut build, &effective_source.effective_source_sha256);
    build.define(
        "BOXDD_RECORDING_CONTRACT_BLAKE3",
        Some(recording_digest_define.as_str()),
    );

    if config.precision == Precision::Double {
        build.define("BOX2D_DOUBLE_PRECISION", None);
    }

    if config.target_env == "msvc" {
        let use_static_crt = env::var("CARGO_CFG_TARGET_FEATURE")
            .unwrap_or_default()
            .split(',')
            .any(|feature| feature == "crt-static");
        build.static_crt(use_static_crt);
        build.debug(config.is_debug());
        build.opt_level(if config.is_debug() { 0 } else { 2 });
        configure_msvc_language(&mut build);
        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && config.target_arch == "x86_64" {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("/arch:AVX2");
        }
    } else {
        build.flag("-std=c17");
        build.flag("-ffp-contract=off");
        build.debug(config.is_debug());
        build.opt_level(if config.is_debug() { 0 } else { 2 });

        if config.target_os == "linux" || config.target_os == "macos" || config.target_env == "gnu"
        {
            if config.target_os == "linux" {
                build.define("_POSIX_C_SOURCE", Some("199309L"));
                println!("cargo:rustc-link-lib=m");
                println!("cargo:rustc-link-lib=pthread");
            }
            build.flag_if_supported("-pthread");
        }

        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && config.target_arch == "x86_64" {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("-mavx2");
        }
    }

    if cfg!(feature = "validate") {
        build.define("BOX2D_VALIDATE", None);
    }

    build.compile("box2d");
    identity_probe.identity
}

fn compile_adapter_identity_probe(
    config: &BuildConfig,
    build_inputs: &MaterializedBuildInputs,
) -> CompiledIdentityProbe {
    let effective_sources = &build_inputs.effective;
    let adapter_sources = &build_inputs.adapter;
    let mut build = cc::Build::new();
    build.include(&effective_sources.public_include);
    build.include(&effective_sources.private_include);
    build.include(&adapter_sources.native_include);
    build.file(&adapter_sources.identity_probe_source);
    define_effective_source_identity(
        &mut build,
        &effective_sources.identity.effective_source_sha256,
    );

    if config.precision == Precision::Double {
        build.define("BOX2D_DOUBLE_PRECISION", None);
    }
    if cfg!(feature = "validate") {
        build.define("BOX2D_VALIDATE", None);
    }
    if config.target_env == "msvc" {
        configure_msvc_language(&mut build);
    } else {
        build.flag("-std=c17");
        if cfg!(feature = "disable-simd") {
            build.define("BOX2D_DISABLE_SIMD", None);
        } else if cfg!(feature = "simd-avx2") && config.target_arch == "x86_64" {
            build.define("BOX2D_AVX2", None);
            build.flag_if_supported("-mavx2");
        }
    }
    let objects = build.try_compile_intermediates().unwrap_or_else(|error| {
        panic!("failed to compile the native provider identity probe: {error}")
    });
    exact_identity_probe(objects, "native provider")
}

fn load_wasm_provider_identity_contract(
    config: &BuildConfig,
    upstream_sha: &str,
    source_tree: &str,
    effective_source_sha256: &str,
    adapter_source_sha256: &str,
    bindings_sha256: &str,
) -> NativeAbiIdentity {
    if cfg!(feature = "validate") {
        panic!(
            "BOXDD_SYS_PROVIDER=wasm-provider does not have a checked validation-enabled ABI route"
        );
    }
    let relative = contract_relative_path(config.precision.as_str())
        .unwrap_or_else(|error| panic!("invalid WASM provider identity route: {error}"));
    let path = config.manifest_dir.join(relative);
    let identity = WasmProviderIdentity::load(
        &config.manifest_dir,
        Path::new(relative),
        &WasmProviderExpectation {
            provider_abi: PROVIDER_ABI,
            target: &config.target,
            compiler_target: COMPILER_TARGET,
            precision: config.precision.as_str(),
            upstream_sha,
            source_tree,
            effective_source_sha256,
            adapter_abi_version: provider_manifest::ADAPTER_ABI_VERSION,
            adapter_source_sha256,
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3,
            validation_enabled: false,
            simd: SIMD_MODE,
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS,
            bindings_sha256,
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "failed to verify WASM provider identity contract {}: {error}",
            path.display()
        )
    });
    NativeAbiIdentity {
        private_abi_hash: identity.private_abi_hash,
        snapshot_layout_hash: identity.snapshot_layout_hash,
        definition_cookie: identity.definition_cookie,
    }
}

fn exact_identity_probe(objects: Vec<PathBuf>, label: &str) -> CompiledIdentityProbe {
    let [object]: [PathBuf; 1] = objects.try_into().unwrap_or_else(|objects: Vec<PathBuf>| {
        panic!(
            "{label} identity probe must produce exactly one object, found {}",
            objects.len()
        )
    });
    CompiledIdentityProbe {
        identity: read_compiled_adapter_identity_file(&object),
        object,
    }
}

fn define_effective_source_identity(build: &mut cc::Build, effective_source_sha256: &str) {
    let value = c_string_define(effective_source_sha256, "effective source SHA-256");
    build.define("BOXDD_EFFECTIVE_SOURCE_SHA256", Some(value.as_str()));
}

fn read_compiled_adapter_identity_file(object_path: &Path) -> NativeAbiIdentity {
    let snapshot = VerifiedFileSnapshot::read(
        object_path,
        64 * 1024 * 1024,
        "compiled adapter identity object",
    )
    .unwrap_or_else(|error| panic!("failed to read adapter identity object: {error}"));
    let file = object::File::parse(snapshot.bytes())
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", object_path.display()));
    let little_endian = file.is_little_endian();
    let private_count = read_identity_scalar(&file, "boxddPrivateAbiValueCount", little_endian);
    let layout_count = read_identity_scalar(&file, "boxddSnapshotLayoutValueCount", little_endian);
    let private_values =
        read_identity_values(&file, "boxddPrivateAbiValues", private_count, little_endian);
    let layout_values = read_identity_values(
        &file,
        "boxddSnapshotLayoutValues",
        layout_count,
        little_endian,
    );
    let definition_cookie = i32::try_from(read_identity_scalar(
        &file,
        "boxddDefinitionCookie",
        little_endian,
    ))
    .expect("native definition cookie exceeds i32");
    NativeAbiIdentity {
        private_abi_hash: private_abi_hash(&private_values, little_endian),
        snapshot_layout_hash: snapshot_layout_hash(&layout_values),
        definition_cookie,
    }
}

fn read_identity_scalar<'data>(
    file: &object::File<'data>,
    name: &str,
    little_endian: bool,
) -> usize {
    let value_bytes = identity_symbol_bytes(file, name, 8);
    let value = if little_endian {
        u64::from_le_bytes(value_bytes.try_into().expect("identity scalar width"))
    } else {
        u64::from_be_bytes(value_bytes.try_into().expect("identity scalar width"))
    };
    usize::try_from(value).unwrap_or_else(|_| panic!("adapter identity count {value} is too large"))
}

fn read_identity_values<'data>(
    file: &object::File<'data>,
    name: &str,
    count: usize,
    little_endian: bool,
) -> Vec<u64> {
    let byte_count = count
        .checked_mul(8)
        .unwrap_or_else(|| panic!("adapter identity array {name} is too large"));
    identity_symbol_bytes(file, name, byte_count)
        .chunks_exact(8)
        .map(|bytes| {
            let bytes: [u8; 8] = bytes.try_into().expect("identity value width");
            if little_endian {
                u64::from_le_bytes(bytes)
            } else {
                u64::from_be_bytes(bytes)
            }
        })
        .collect()
}

fn identity_symbol_bytes<'data>(
    file: &'data object::File<'data>,
    expected_name: &str,
    width: usize,
) -> &'data [u8] {
    let symbol = file
        .symbols()
        .find(|symbol| {
            symbol
                .name()
                .ok()
                .is_some_and(|name| name.trim_start_matches('_') == expected_name)
        })
        .unwrap_or_else(|| panic!("target object is missing identity symbol {expected_name}"));
    let section_index = symbol
        .section_index()
        .unwrap_or_else(|| panic!("identity symbol {expected_name} has no section"));
    let section = file
        .section_by_index(section_index)
        .unwrap_or_else(|error| {
            panic!("identity symbol {expected_name} has an invalid section: {error}")
        });
    let offset = symbol
        .address()
        .checked_sub(section.address())
        .and_then(|offset| usize::try_from(offset).ok())
        .unwrap_or_else(|| panic!("identity symbol {expected_name} has an invalid address"));
    let data = section.data().unwrap_or_else(|error| {
        panic!("identity section for {expected_name} is unreadable: {error}")
    });
    data.get(offset..offset.checked_add(width).expect("identity symbol width"))
        .unwrap_or_else(|| panic!("identity symbol {expected_name} is truncated"))
}

fn write_expected_adapter_identity(out_dir: &Path, identity: NativeAbiIdentity) {
    let hash = identity
        .private_abi_hash
        .iter()
        .map(|byte| format!("0x{byte:02X}, "))
        .collect::<String>();
    let source = format!(
        "pub const PRIVATE_ABI_HASH: [u8; 32] = [{hash}];\n\
         pub const SNAPSHOT_LAYOUT_HASH: u32 = 0x{:08X};\n",
        identity.snapshot_layout_hash
    );
    fs::write(out_dir.join("adapter_identity.rs"), source)
        .expect("failed to write target adapter identity constants");
    fs::write(
        out_dir.join("definition_cookie.rs"),
        format!(
            "pub const DEFINITION_COOKIE: i32 = {};\n",
            identity.definition_cookie
        ),
    )
    .expect("failed to write target definition cookie");
}
