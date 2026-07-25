use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use object::{Object, ObjectSection, ObjectSymbol};

#[path = "src/build_support.rs"]
mod build_support;

#[path = "src/bindgen_contract.rs"]
mod bindgen_contract;

#[path = "src/provider_manifest.rs"]
mod provider_manifest;

#[path = "src/provider_archive.rs"]
mod provider_archive;

#[allow(dead_code)]
#[path = "src/source_overlay.rs"]
mod source_overlay;

#[path = "src/wasm_identity.rs"]
mod wasm_identity;

#[allow(dead_code)]
#[path = "src/precision.rs"]
mod precision;

use bindgen_contract::{
    ValidatedFreestandingHeaders, ValidatedWasiSysroot, WASI_LIBC_VERSION,
    resolve_unknown_unknown_headers, resolve_wasi_sysroot, validate_ambient_header_environment,
    validate_bindgen_target_override,
};
use build_support::{
    BindingTargetFamily, COSIGN_VERSION, PrebuiltProvenance, ProviderAdapter, ProviderInputs,
    SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH, SIGSTORE_TRUSTED_ROOT_SHA256, classify_binding_target,
    cosign_verify_blob_args, cosign_version_is_qualified, select_provider, simd_identity,
    validate_c_source_paths, validate_skip_cc_policy,
};
use precision::Precision;
use provider_archive::{
    ArchiveExpectation, private_abi_hash, private_abi_hash_hex, snapshot_layout_hash,
    verify_provider_archive,
};
use provider_manifest::{
    ArtifactExpectation, ArtifactIdentityExpectation, RECORDING_CONTRACT_BLAKE3,
    REQUIRED_ADAPTER_SYMBOLS, VENDORED_SOURCE_IDENTITY_SHA256, VerifiedArtifact,
    adapter_source_sha256, sha256_bytes, sha256_file, vendored_source_identity_sha256,
    verify_artifact,
};
use source_overlay::{
    EffectiveSourceIdentity, MaterializedEffectiveSources, effective_source_identity,
    materialize_effective_box2d_sources,
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
    wasi_bindgen_sysroot: Option<ValidatedWasiSysroot>,
    freestanding_bindgen_headers: Option<ValidatedFreestandingHeaders>,
    provider: ProviderAdapter,
    precision: Precision,
}

#[derive(Debug)]
struct UpstreamBuildManifest {
    active_revision: String,
    source_tree: String,
    source_paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct PreparedExternal {
    artifact: VerifiedArtifact,
    archive_bytes: Vec<u8>,
    native_abi_identity: NativeAbiIdentity,
    provenance_sha256: Option<String>,
    trusted_root_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct NativeAbiIdentity {
    private_abi_hash: [u8; 32],
    snapshot_layout_hash: u32,
}

#[derive(Clone, Debug)]
struct CompiledIdentityProbe {
    identity: NativeAbiIdentity,
    object: PathBuf,
}

const ADAPTER_C_SOURCE_PATHS: &[&str] = &[
    "native/boxdd_adapter.c",
    "native/boxdd_recording_adapter.c",
    "native/boxdd_snapshot_validate.c",
];
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
        let is_docsrs = env::var("DOCS_RS").is_ok() || env::var("CARGO_CFG_DOCSRS").is_ok();
        let skip_cc = parse_bool_env("BOXDD_SYS_SKIP_CC");
        let force_bindgen = parse_bool_env("BOXDD_SYS_FORCE_BINDGEN");
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let checked_bindings = manifest_dir
            .join("src")
            .join(binding_target.pregenerated_bindings_file(Precision::ACTIVE.is_double()));
        let binding_generation_required = force_bindgen || !checked_bindings.is_file();
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
        let explicit_provider = env::var("BOXDD_SYS_PROVIDER").ok();
        let link_kind = env::var("BOXDD_SYS_LINK_KIND").ok();
        let provider = select_provider(ProviderInputs {
            target_arch: &target_arch,
            target_os: &target_os,
            explicit_provider: explicit_provider.as_deref(),
            has_system_dir: env::var_os("BOX2D_LIB_DIR").is_some(),
            has_system_manifest: env::var_os("BOXDD_SYS_SYSTEM_MANIFEST").is_some(),
            has_prebuilt_manifest: env::var_os("BOXDD_SYS_PREBUILT_MANIFEST").is_some(),
            has_prebuilt_bundle: env::var_os("BOXDD_SYS_PREBUILT_BUNDLE").is_some(),
            has_prebuilt_trusted_root: env::var_os("BOXDD_SYS_PREBUILT_TRUSTED_ROOT").is_some(),
            // docs.rs still needs the vendored adapter for binding/type checking, but its
            // dedicated branch below skips the native compiler invocation.
            build_from_source_enabled: cfg!(feature = "build-from-source") || is_docsrs,
            link_kind: link_kind.as_deref(),
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

fn parse_bool_env(key: &str) -> bool {
    match env::var(key) {
        Ok(v) => matches!(
            v.as_str(),
            "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
        ),
        Err(_) => false,
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

fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_pregenerated)");
    println!("cargo:rustc-check-cfg=cfg(force_bindgen)");
    println!("cargo:rustc-check-cfg=cfg(boxdd_sys_wasm_provider)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/bindgen_contract.rs");
    println!("cargo:rerun-if-changed=src/precision.rs");
    println!("cargo:rerun-if-changed=src/source_overlay.rs");
    println!("cargo:rerun-if-changed=src/provider_archive.rs");
    println!("cargo:rerun-if-changed=src/bindings_pregenerated.rs");
    println!("cargo:rerun-if-changed=src/bindings_double.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_unknown_unknown.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_unknown_unknown_double.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_wasip1.rs");
    println!("cargo:rerun-if-changed=src/bindings_wasm32_wasip1_double.rs");
    println!("cargo:rerun-if-changed=src/bindgen_headers/wasm32_unknown_unknown");
    println!("cargo:rerun-if-changed=src/bindgen_headers/wasm32_unknown_unknown/math.h");
    println!("cargo:rerun-if-changed=upstream.toml");
    println!("cargo:rerun-if-changed=effective-source.toml");
    println!("cargo:rerun-if-changed=third-party/box2d/include/box2d/box2d.h");
    println!("cargo:rerun-if-changed=third-party/box2d");
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-changed={SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH}");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SKIP_CC");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PROVIDER");
    println!("cargo:rerun-if-env-changed=BOX2D_LIB_DIR");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_SYSTEM_MANIFEST");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_MANIFEST");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_BUNDLE");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_PREBUILT_TRUSTED_ROOT");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_COSIGN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_LINK_KIND");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_FORCE_BINDGEN");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_BINDGEN_TARGET");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_WASI_SYSROOT");
    println!("cargo:rerun-if-env-changed=BOXDD_SYS_EMCC");
    println!("cargo:rerun-if-env-changed=EMSDK");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_DOCSRS");

    let config = BuildConfig::from_env();
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
    reject_external_precision_overrides(&config.target);
    let upstream = load_upstream_manifest(&config.manifest_dir);
    validate_vendored_source(&config.manifest_dir, &upstream);
    let effective_source = effective_source_identity(&config.manifest_dir)
        .unwrap_or_else(|error| panic!("failed to validate effective Box2D sources: {error}"));
    assert_eq!(
        effective_source.upstream_sha, upstream.active_revision,
        "effective-source identity does not match the validated upstream revision"
    );
    assert_eq!(
        effective_source.source_tree, upstream.source_tree,
        "effective-source identity does not match the validated upstream tree"
    );
    let materialized_sources =
        materialize_effective_box2d_sources(&config.manifest_dir, &config.out_dir).unwrap_or_else(
            |error| panic!("failed to materialize reviewed Box2D source tree: {error}"),
        );
    assert_eq!(
        materialized_sources.identity, effective_source,
        "materialized compiler inputs do not match the prevalidated effective-source identity"
    );
    let pregenerated = config.pregenerated_bindings();
    let has_pregenerated = pregenerated.is_file();
    let adapter_source_sha256 = adapter_source_sha256(&config.manifest_dir)
        .unwrap_or_else(|error| panic!("failed to identify the repository adapter: {error}"));

    validate_build_config(&config);
    let external = prepare_external_provider(
        &config,
        &upstream.active_revision,
        &effective_source.effective_source_sha256,
        &adapter_source_sha256,
        &materialized_sources,
    );

    let wasm_import_module = config.precision.wasm_import_module();
    emit_build_identity(
        &config,
        &upstream.active_revision,
        &effective_source.effective_source_sha256,
        &adapter_source_sha256,
        wasm_import_module,
        external.as_ref(),
        &materialized_sources.public_include,
    );
    write_expected_adapter_identity(
        &config.out_dir,
        NativeAbiIdentity {
            private_abi_hash: [0; 32],
            snapshot_layout_hash: 0,
        },
    );

    if config.force_bindgen {
        println!("cargo:rustc-cfg=force_bindgen");
    } else if has_pregenerated {
        println!("cargo:rustc-cfg=has_pregenerated");
    }

    if config.provider == ProviderAdapter::WasmProvider {
        println!("cargo:rustc-cfg=boxdd_sys_wasm_provider");
        if !has_pregenerated && !config.force_bindgen {
            panic!("BOXDD_SYS_PROVIDER=wasm-provider requires checked-in pregenerated bindings");
        }
    }

    if config.force_bindgen || (!has_pregenerated && !config.is_docsrs) {
        #[cfg(feature = "bindgen")]
        generate_bindings(
            &materialized_sources.public_include,
            &config.out_dir,
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
        #[cfg(not(feature = "bindgen"))]
        {
            if config.force_bindgen {
                panic!("BOXDD_SYS_FORCE_BINDGEN=1 requires the `bindgen` feature");
            }
            panic!(
                "pregenerated Box2D bindings are missing; enable `bindgen` or refresh checked-in bindings"
            );
        }
    }

    if config.is_docsrs {
        println!("cargo:warning=DOCS_RS detected: skipping native Box2D C build");
        return;
    }

    if config.skip_cc {
        println!("cargo:warning=Skipping native Box2D C build due to BOXDD_SYS_SKIP_CC");
        return;
    }

    match config.provider {
        ProviderAdapter::WasmCompileOnly => println!(
            "cargo:warning=boxdd-sys is using compile-only WASM mode; no Box2D runtime is linked"
        ),
        ProviderAdapter::WasmProvider => {
            println!(
                "cargo:warning=boxdd-sys WASM provider mode is active; runtime identity must be verified before instantiation"
            );
            let identity = build_wasm_provider_identity_probe(
                &config,
                &effective_source.effective_source_sha256,
                &materialized_sources,
            );
            write_expected_adapter_identity(&config.out_dir, identity);
        }
        ProviderAdapter::Vendored => {
            let identity = build_box2d_from_source(
                &config,
                &upstream.active_revision,
                &effective_source,
                &adapter_source_sha256,
                &materialized_sources,
            );
            write_expected_adapter_identity(&config.out_dir, identity);
        }
        ProviderAdapter::System | ProviderAdapter::Prebuilt => {
            let prepared = external.expect("external provider was prepared");
            write_expected_adapter_identity(&config.out_dir, prepared.native_abi_identity);
            link_verified_artifact(&config, prepared.artifact, &prepared.archive_bytes);
        }
    }
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
}

fn load_upstream_manifest(manifest_dir: &Path) -> UpstreamBuildManifest {
    let path = manifest_dir.join("upstream.toml");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let manifest: toml::Value = toml::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
    let revision = manifest
        .get("active_revision")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{} has no string active_revision", path.display()));
    assert!(
        revision.len() == 40
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{} active_revision must be a lowercase 40-character Git SHA",
        path.display()
    );

    let raw_sources = manifest
        .get("source_inventory")
        .and_then(|inventory| inventory.get("c_sources"))
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{} has no source_inventory.c_sources array", path.display()));
    assert!(
        !raw_sources.is_empty(),
        "{} source_inventory.c_sources must not be empty",
        path.display()
    );

    let raw_sources = raw_sources
        .iter()
        .map(|source| {
            source.as_str().unwrap_or_else(|| {
                panic!(
                    "{} source_inventory.c_sources entries must be strings",
                    path.display()
                )
            })
        })
        .collect::<Vec<_>>();
    let c_sources = validate_c_source_paths(raw_sources).unwrap_or_else(|error| {
        panic!(
            "{} has invalid source_inventory.c_sources: {error}",
            path.display()
        )
    });

    let source_inventory = manifest
        .get("source_inventory")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{} has no source_inventory table", path.display()));
    let source_tree = source_inventory
        .get("tree")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{} source_inventory.tree must be a string", path.display()));
    assert_git_sha(
        source_tree,
        &format!("{} source_inventory.tree", path.display()),
    );
    let mut source_paths = c_sources.clone();
    for (key, prefix, extension) in [
        ("private_headers", "src/", "h"),
        ("inline_files", "src/", "inl"),
        ("public_headers", "include/box2d/", "h"),
    ] {
        let values = source_inventory
            .get(key)
            .and_then(toml::Value::as_array)
            .unwrap_or_else(|| {
                panic!("{} source_inventory.{key} must be an array", path.display())
            });
        for value in values {
            let rendered = value.as_str().unwrap_or_else(|| {
                panic!(
                    "{} source_inventory.{key} entries must be strings",
                    path.display()
                )
            });
            assert_inventory_path(rendered, prefix, extension, &path, key);
            source_paths.push(PathBuf::from(rendered));
        }
    }

    UpstreamBuildManifest {
        active_revision: revision.to_owned(),
        source_tree: source_tree.to_owned(),
        source_paths,
    }
}

fn assert_git_sha(value: &str, label: &str) {
    assert!(
        value.len() == 40
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{label} must be a lowercase 40-character Git SHA"
    );
}

fn assert_inventory_path(
    value: &str,
    prefix: &str,
    extension: &str,
    manifest_path: &Path,
    field: &str,
) {
    let path = Path::new(value);
    assert!(
        value.starts_with(prefix)
            && !value.contains('\\')
            && path.extension().and_then(|value| value.to_str()) == Some(extension)
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_))),
        "{} source_inventory.{field} contains invalid path {value:?}",
        manifest_path.display()
    );
}

fn validate_vendored_source(manifest_dir: &Path, upstream: &UpstreamBuildManifest) {
    let source_root = manifest_dir.join("third-party/box2d");
    let actual = vendored_source_identity_sha256(
        &upstream.active_revision,
        &upstream.source_tree,
        &source_root,
        &upstream.source_paths,
    )
    .unwrap_or_else(|error| panic!("failed to validate vendored Box2D source identity: {error}"));
    assert_eq!(
        actual, VENDORED_SOURCE_IDENTITY_SHA256,
        "vendored Box2D source content/inventory does not match the reviewed identity"
    );

    if !source_root.join(".git").exists() {
        return;
    }
    let git_value = |args: &[&str], label: &str| {
        let output = Command::new("git")
            .args(["-C", &source_root.to_string_lossy()])
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("failed to execute git {label}: {error}"));
        assert!(
            output.status.success(),
            "git {label} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    assert_eq!(
        git_value(&["rev-parse", "HEAD"], "rev-parse HEAD"),
        upstream.active_revision,
        "vendored Box2D submodule HEAD does not match upstream.toml"
    );
    assert_eq!(
        git_value(&["rev-parse", "HEAD^{tree}"], "rev-parse HEAD^{tree}"),
        upstream.source_tree,
        "vendored Box2D submodule tree does not match upstream.toml"
    );
    let status = git_value(
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignored",
        ],
        "status",
    );
    assert!(
        status.is_empty(),
        "vendored Box2D submodule is dirty or contains ignored files: {status}"
    );
}

fn emit_build_identity(
    config: &BuildConfig,
    upstream_sha: &str,
    effective_source_sha256: &str,
    adapter_source_sha256: &str,
    wasm_import_module: &str,
    external: Option<&PreparedExternal>,
    effective_public_include: &Path,
) {
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
            .map(|prepared| prepared.artifact.manifest_sha256.as_str())
            .unwrap_or("")
    );
    println!(
        "cargo:rustc-env=BOXDD_SYS_PROVIDER_ARCHIVE_SHA256={}",
        external
            .map(|prepared| prepared.artifact.archive_sha256.as_str())
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
    println!("cargo:include={}", effective_public_include.display());
    println!("cargo:wasm_import_module={wasm_import_module}");

    // The package helper consumes this local marker instead of guessing which Cargo fingerprint
    // belongs to the requested target.  It contains no secrets and is intentionally deterministic.
    let identity = format!(
        "schema_version = 2\nprovider = {:?}\ncrate_version = {:?}\nupstream_sha = {:?}\neffective_source_sha256 = {:?}\nprecision = {:?}\ntarget = {:?}\ncrt = {:?}\nsimd = {:?}\nvalidate = {}\nadapter_source_sha256 = {:?}\nrecording_contract_blake3 = {:?}\nmanifest_sha256 = {:?}\narchive_sha256 = {:?}\nprovenance_sha256 = {:?}\ntrusted_root_sha256 = {:?}\n",
        config.provider.as_str(),
        env!("CARGO_PKG_VERSION"),
        upstream_sha,
        effective_source_sha256,
        config.precision.as_str(),
        config.target,
        expected_crt_identity(config),
        expected_simd_identity(config),
        cfg!(feature = "validate"),
        adapter_source_sha256,
        RECORDING_CONTRACT_BLAKE3,
        external
            .map(|prepared| prepared.artifact.manifest_sha256.as_str())
            .unwrap_or(""),
        external
            .map(|prepared| prepared.artifact.archive_sha256.as_str())
            .unwrap_or(""),
        external
            .and_then(|prepared| prepared.provenance_sha256.as_deref())
            .unwrap_or(""),
        external
            .and_then(|prepared| prepared.trusted_root_sha256.as_deref())
            .unwrap_or(""),
    );
    fs::write(config.out_dir.join("boxdd-build-identity.toml"), identity)
        .expect("failed to write boxdd build identity marker");
}

fn prepare_external_provider(
    config: &BuildConfig,
    upstream_sha: &str,
    effective_source_sha256: &str,
    adapter_source_sha256: &str,
    effective_sources: &MaterializedEffectiveSources,
) -> Option<PreparedExternal> {
    let (provider, manifest_key) = match config.provider {
        ProviderAdapter::System => ("system", "BOXDD_SYS_SYSTEM_MANIFEST"),
        ProviderAdapter::Prebuilt => ("prebuilt", "BOXDD_SYS_PREBUILT_MANIFEST"),
        _ => return None,
    };
    let manifest_path = PathBuf::from(
        env::var(manifest_key)
            .unwrap_or_else(|_| panic!("{manifest_key} is required for the {provider} provider")),
    );
    assert_eq!(
        effective_sources.identity.effective_source_sha256, effective_source_sha256,
        "materialized external-provider inputs do not match the prevalidated effective-source identity"
    );
    let header_path = effective_sources.public_include.join("box2d/box2d.h");
    let bindings_path = config.pregenerated_bindings();
    let native_abi_identity = compile_adapter_identity_probe(
        config,
        &effective_sources.public_include,
        &effective_sources.private_include,
        effective_source_sha256,
    )
    .identity;
    let private_abi_hash = private_abi_hash_hex(native_abi_identity.private_abi_hash);
    let expectation = ArtifactExpectation {
        identity: ArtifactIdentityExpectation {
            provider,
            crate_version: env!("CARGO_PKG_VERSION"),
            upstream_sha,
            effective_source_sha256,
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
    let verified = verify_artifact(&manifest_path, &expectation).unwrap_or_else(|error| {
        panic!(
            "failed to verify {provider} provider manifest {}: {error}",
            manifest_path.display()
        )
    });
    let verified_archive = verify_provider_archive(
        &verified.archive_path,
        &ArchiveExpectation {
            target: &config.target,
            required_symbols: REQUIRED_ADAPTER_SYMBOLS,
            effective_source_sha256,
            private_abi_hash: &private_abi_hash,
            snapshot_layout_hash: native_abi_identity.snapshot_layout_hash,
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "failed to prove {provider} provider archive {}: {error}",
            verified.archive_path.display()
        )
    });
    assert_eq!(
        verified_archive.archive_sha256, verified.archive_sha256,
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
                .archive_path
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
    println!("cargo:rerun-if-changed={}", verified.archive_path.display());
    println!("cargo:rerun-if-changed={}", verified.header_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        verified.bindings_path.display()
    );
    let (provenance_sha256, trusted_root_sha256) = if config.provider == ProviderAdapter::Prebuilt {
        let (provenance, trusted_root) = verify_prebuilt_provenance(config, &verified);
        (Some(provenance), Some(trusted_root))
    } else {
        (None, None)
    };
    Some(PreparedExternal {
        artifact: verified,
        archive_bytes: verified_archive.archive_bytes,
        native_abi_identity,
        provenance_sha256,
        trusted_root_sha256,
    })
}

fn verify_prebuilt_provenance(
    config: &BuildConfig,
    artifact: &VerifiedArtifact,
) -> (String, String) {
    let bundle = canonical_env_file("BOXDD_SYS_PREBUILT_BUNDLE");
    let trusted_root = trusted_root_file(config);
    let payload = config
        .out_dir
        .join("boxdd-prebuilt-provenance-manifest.toml");
    let payload_bytes = artifact.manifest.render();
    assert_eq!(
        sha256_bytes(&payload_bytes),
        artifact.manifest_sha256,
        "verified prebuilt manifest is not the canonical signed payload"
    );
    fs::write(&payload, payload_bytes)
        .unwrap_or_else(|error| panic!("failed to write canonical provenance payload: {error}"));
    let cosign = env::var_os("BOXDD_SYS_COSIGN").unwrap_or_else(|| "cosign".into());
    let version_output = Command::new(&cosign)
        .arg("version")
        .output()
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
        source_commit: artifact
            .manifest
            .source_commit
            .as_deref()
            .expect("validated prebuilt manifest must contain source_commit"),
        release_tag: artifact
            .manifest
            .release_tag
            .as_deref()
            .expect("validated prebuilt manifest must contain release_tag"),
        payload: &payload,
        bundle: &bundle,
        trusted_root: &trusted_root,
    })
    .expect("validated prebuilt manifest must produce a provenance policy");
    let output = Command::new(&cosign)
        .args(args)
        .output()
        .unwrap_or_else(|error| {
            panic!("failed to execute Cosign provenance verification: {error}")
        });
    assert!(
        output.status.success(),
        "prebuilt provider provenance verification failed before linking: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    println!("cargo:rerun-if-changed={}", bundle.display());
    println!("cargo:rerun-if-changed={}", trusted_root.display());
    (
        sha256_file(&bundle).expect("failed to hash verified Sigstore bundle"),
        sha256_file(&trusted_root).expect("failed to hash the Sigstore trusted root"),
    )
}

fn trusted_root_file(config: &BuildConfig) -> PathBuf {
    let (description, path) = match env::var_os("BOXDD_SYS_PREBUILT_TRUSTED_ROOT") {
        Some(path) => ("BOXDD_SYS_PREBUILT_TRUSTED_ROOT", PathBuf::from(path)),
        None => (
            "crate-owned Sigstore trusted root",
            config
                .manifest_dir
                .join(SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH),
        ),
    };
    let canonical = fs::canonicalize(&path).unwrap_or_else(|error| {
        panic!(
            "failed to resolve {description} {}: {error}",
            path.display()
        )
    });
    assert!(
        canonical.is_file(),
        "{description} must identify a local regular file: {}",
        canonical.display()
    );
    let digest = sha256_file(&canonical)
        .unwrap_or_else(|error| panic!("failed to hash {description}: {error}"));
    assert_eq!(
        digest, SIGSTORE_TRUSTED_ROOT_SHA256,
        "{description} does not match the crate-owned authenticated Sigstore trusted root"
    );
    canonical
}

fn canonical_env_file(key: &str) -> PathBuf {
    let path = PathBuf::from(
        env::var_os(key).unwrap_or_else(|| panic!("{key} is required for the prebuilt provider")),
    );
    let canonical = fs::canonicalize(&path)
        .unwrap_or_else(|error| panic!("failed to resolve {key} {}: {error}", path.display()));
    assert!(
        canonical.is_file(),
        "{key} must identify a local regular file: {}",
        canonical.display()
    );
    canonical
}

fn link_verified_artifact(config: &BuildConfig, artifact: VerifiedArtifact, archive_bytes: &[u8]) {
    let digest_prefix = &artifact.archive_sha256[..16];
    let link_name = format!("box2d_{digest_prefix}");
    let file_name = if config.target_env == "msvc" {
        format!("{link_name}.lib")
    } else {
        format!("lib{link_name}.a")
    };
    let linked_archive = config.out_dir.join(file_name);
    fs::write(&linked_archive, archive_bytes).unwrap_or_else(|error| {
        panic!(
            "failed to write the verified archive snapshot to {}: {error}",
            linked_archive.display()
        )
    });
    let copied_sha = sha256_file(&linked_archive).expect("failed to hash copied link archive");
    assert_eq!(
        copied_sha, artifact.archive_sha256,
        "verified archive changed while preparing the linker input"
    );
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

fn reject_external_precision_overrides(target: &str) {
    let normalized_target = target.replace('-', "_");
    let target_keys = [
        format!("CFLAGS_{target}"),
        format!("CFLAGS_{normalized_target}"),
        format!("{target}_CFLAGS"),
        format!("{normalized_target}_CFLAGS"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{target}"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{normalized_target}"),
    ];
    for key in ["CFLAGS", "CPPFLAGS", "CL", "BINDGEN_EXTRA_CLANG_ARGS"]
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
fn generate_bindings(
    effective_public_include: &Path,
    out_dir: &Path,
    target: &str,
    precision: Precision,
    wasi_sysroot: Option<&Path>,
    freestanding_headers: Option<&Path>,
) {
    let header = effective_public_include.join("box2d").join("box2d.h");
    let builder = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
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

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write Box2D bindings");
}

#[cfg(feature = "bindgen")]
fn configure_bindgen_host_headers(builder: bindgen::Builder, target: &str) -> bindgen::Builder {
    if !cfg!(target_os = "macos") || !target.contains("-linux-") {
        return builder;
    }

    // Apple Clang has no Linux libc sysroot. The Box2D public API only needs ISO C headers here;
    // Xcode supplies those headers while `--target` above remains the manifest's Linux ABI.
    let output = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
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
    _out_dir: &Path,
    _target: &str,
    _precision: Precision,
    _wasi_sysroot: Option<&Path>,
    _freestanding_headers: Option<&Path>,
) {
    unreachable!("generate_bindings is only available with the `bindgen` feature enabled");
}

fn configure_msvc_language(build: &mut cc::Build) {
    match build.is_flag_supported("/std:c17") {
        Ok(true) => {
            build.flag("/std:c17");
        }
        Ok(false) => {
            panic!("the selected MSVC C compiler does not support the C17 mode required by Box2D");
        }
        Err(error) => {
            panic!("failed to verify MSVC C17 support required by Box2D: {error}");
        }
    }
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
    upstream_sha: &str,
    effective_source: &EffectiveSourceIdentity,
    adapter_source_sha256: &str,
    effective_sources: &MaterializedEffectiveSources,
) -> NativeAbiIdentity {
    let adapter_include = config.manifest_dir.join("native");
    assert_eq!(
        &effective_sources.identity, effective_source,
        "materialized compiler inputs do not match the prevalidated effective-source identity"
    );
    let mut build = cc::Build::new();
    build.include(&effective_sources.public_include);
    build.include(&effective_sources.private_include);
    build.include(&adapter_include);
    for source in &effective_sources.c_sources {
        assert!(
            source.is_file(),
            "Box2D source declared by upstream.toml is missing: {}",
            source.display()
        );
        build.file(source);
    }
    for relative_path in ADAPTER_C_SOURCE_PATHS {
        build.file(config.manifest_dir.join(relative_path));
    }

    let identity_probe = compile_adapter_identity_probe(
        config,
        &effective_sources.public_include,
        &effective_sources.private_include,
        &effective_source.effective_source_sha256,
    );
    build.object(&identity_probe.object);

    let upstream_define = c_string_define(upstream_sha, "upstream SHA");
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
    public_include: &Path,
    private_include: &Path,
    effective_source_sha256: &str,
) -> CompiledIdentityProbe {
    let mut build = cc::Build::new();
    build.include(public_include);
    build.include(private_include);
    build.include(config.manifest_dir.join("native"));
    build.file(config.manifest_dir.join("native/boxdd_identity_values.c"));
    define_effective_source_identity(&mut build, effective_source_sha256);

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

fn build_wasm_provider_identity_probe(
    config: &BuildConfig,
    effective_source_sha256: &str,
    effective_sources: &MaterializedEffectiveSources,
) -> NativeAbiIdentity {
    assert_eq!(
        effective_sources.identity.effective_source_sha256, effective_source_sha256,
        "materialized WASM identity probe inputs do not match the prevalidated effective-source identity"
    );
    let source = config
        .manifest_dir
        .join("native")
        .join("boxdd_identity_values.c");
    let compiler = find_emscripten_compiler();
    let mut build = cc::Build::new();
    build
        .target("wasm32-unknown-emscripten")
        .compiler(&compiler)
        .cargo_metadata(false)
        .include(&effective_sources.public_include)
        .include(&effective_sources.private_include)
        .include(config.manifest_dir.join("native"))
        .file(source);
    build.flag("-std=c17");
    build.flag("-O2");
    build.flag("--target=wasm32-unknown-emscripten");
    build.define("BOX2D_DISABLE_SIMD", None);
    define_effective_source_identity(&mut build, effective_source_sha256);
    if config.precision == Precision::Double {
        build.define("BOX2D_DOUBLE_PRECISION", None);
    }
    if cfg!(feature = "validate") {
        build.define("BOX2D_VALIDATE", None);
    }

    let objects = build.try_compile_intermediates().unwrap_or_else(|error| {
        panic!("failed to compile Emscripten WASM provider identity probe: {error}")
    });
    exact_identity_probe(objects, "Emscripten WASM provider").identity
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

fn find_emscripten_compiler() -> PathBuf {
    if let Some(path) = env::var_os("BOXDD_SYS_EMCC") {
        return PathBuf::from(path);
    }
    if let Some(root) = env::var_os("EMSDK") {
        let emscripten = PathBuf::from(root).join("upstream").join("emscripten");
        for name in ["emcc", "emcc.exe", "emcc.bat"] {
            let candidate = emscripten.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from(if cfg!(windows) { "emcc.bat" } else { "emcc" })
}

fn read_compiled_adapter_identity_file(object_path: &Path) -> NativeAbiIdentity {
    let bytes = fs::read(object_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", object_path.display()));
    let file = object::File::parse(bytes.as_slice())
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", object_path.display()));
    let little_endian = file.is_little_endian();
    let private_count = read_identity_scalar(
        &file,
        bytes.as_slice(),
        "boxddPrivateAbiValueCount",
        little_endian,
    );
    let layout_count = read_identity_scalar(
        &file,
        bytes.as_slice(),
        "boxddSnapshotLayoutValueCount",
        little_endian,
    );
    let private_values = read_identity_values(
        &file,
        bytes.as_slice(),
        "boxddPrivateAbiValues",
        private_count,
        little_endian,
    );
    let layout_values = read_identity_values(
        &file,
        bytes.as_slice(),
        "boxddSnapshotLayoutValues",
        layout_count,
        little_endian,
    );
    NativeAbiIdentity {
        private_abi_hash: private_abi_hash(&private_values, little_endian),
        snapshot_layout_hash: snapshot_layout_hash(&layout_values),
    }
}

fn read_identity_scalar<'data>(
    file: &object::File<'data>,
    object_bytes: &'data [u8],
    name: &str,
    little_endian: bool,
) -> usize {
    let value_bytes = identity_symbol_bytes(file, object_bytes, name, 8);
    let value = if little_endian {
        u64::from_le_bytes(value_bytes.try_into().expect("identity scalar width"))
    } else {
        u64::from_be_bytes(value_bytes.try_into().expect("identity scalar width"))
    };
    usize::try_from(value).unwrap_or_else(|_| panic!("adapter identity count {value} is too large"))
}

fn read_identity_values<'data>(
    file: &object::File<'data>,
    object_bytes: &'data [u8],
    name: &str,
    count: usize,
    little_endian: bool,
) -> Vec<u64> {
    let byte_count = count
        .checked_mul(8)
        .unwrap_or_else(|| panic!("adapter identity array {name} is too large"));
    identity_symbol_bytes(file, object_bytes, name, byte_count)
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
    object_bytes: &'data [u8],
    expected_name: &str,
    width: usize,
) -> &'data [u8] {
    if file.format() == object::BinaryFormat::Wasm {
        return wasm_identity::symbol_bytes(object_bytes, expected_name, width)
            .unwrap_or_else(|error| panic!("{error}"));
    }

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
}
