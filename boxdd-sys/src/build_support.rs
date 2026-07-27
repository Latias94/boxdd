use std::collections::BTreeSet;
use std::fmt;
use std::path::{Component, PathBuf};

#[cfg(all(test, feature = "package-bin"))]
#[path = "bindgen_contract.rs"]
mod bindgen_contract;

#[path = "provenance_policy.rs"]
mod provenance_policy;
#[allow(unused_imports)]
pub(crate) use provenance_policy::{
    COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW, PrebuiltProvenance,
    SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH, SIGSTORE_TRUSTED_ROOT_SHA256, cosign_verify_blob_args,
    cosign_version_is_qualified,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderAdapter {
    Vendored,
    System,
    Prebuilt,
    WasmCompileOnly,
    WasmProvider,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindingTargetFamily {
    Native,
    WasmUnknownUnknown,
    WasmWasiP1,
}

impl BindingTargetFamily {
    pub(crate) const fn pregenerated_bindings_file(self, double_precision: bool) -> &'static str {
        match (self, double_precision) {
            (Self::Native, false) => "bindings_pregenerated.rs",
            (Self::Native, true) => "bindings_double.rs",
            (Self::WasmUnknownUnknown, false) => "bindings_wasm32_unknown_unknown.rs",
            (Self::WasmUnknownUnknown, true) => "bindings_wasm32_unknown_unknown_double.rs",
            (Self::WasmWasiP1, false) => "bindings_wasm32_wasip1.rs",
            (Self::WasmWasiP1, true) => "bindings_wasm32_wasip1_double.rs",
        }
    }
}

pub(crate) fn classify_binding_target(
    target: &str,
    target_family: &str,
    target_arch: &str,
    target_os: &str,
    target_env: &str,
) -> Result<BindingTargetFamily, String> {
    let target_families = target_family
        .split(',')
        .filter(|family| !family.is_empty())
        .collect::<BTreeSet<_>>();
    let is_wasm = target_families.contains("wasm")
        || target_arch.starts_with("wasm")
        || target.starts_with("wasm");
    if !is_wasm {
        return Ok(BindingTargetFamily::Native);
    }

    if target_families != BTreeSet::from(["wasm"]) {
        return Err(format!(
            "unsupported WASM Rust target family {target_family:?} for target {target:?}; expected the exact `wasm` family"
        ));
    }

    match (target, target_arch, target_os, target_env) {
        ("wasm32-unknown-unknown", "wasm32", "unknown", "") => {
            Ok(BindingTargetFamily::WasmUnknownUnknown)
        }
        ("wasm32-wasip1", "wasm32", "wasi", "p1") => Ok(BindingTargetFamily::WasmWasiP1),
        _ => Err(format!(
            "unsupported WASM Rust target {target:?} (target_family={target_family:?}, target_arch={target_arch:?}, target_os={target_os:?}, target_env={target_env:?}); checked-in bindings exist only for wasm32-unknown-unknown and wasm32-wasip1"
        )),
    }
}

impl ProviderAdapter {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Vendored => "vendored",
            Self::System => "system",
            Self::Prebuilt => "prebuilt",
            Self::WasmCompileOnly => "wasm-compile-only",
            Self::WasmProvider => "wasm-provider",
        }
    }

    pub(crate) const fn is_wasm(self) -> bool {
        matches!(self, Self::WasmCompileOnly | Self::WasmProvider)
    }
}

pub(crate) fn validate_skip_cc_policy(
    is_docsrs: bool,
    skip_cc: bool,
    force_bindgen: bool,
    provider: ProviderAdapter,
) -> Result<(), String> {
    if !skip_cc || is_docsrs {
        return Ok(());
    }
    if force_bindgen
        && matches!(
            provider,
            ProviderAdapter::Vendored | ProviderAdapter::WasmCompileOnly
        )
    {
        return Ok(());
    }
    Err(
        "BOXDD_SYS_SKIP_CC=1 is reserved for explicit vendored or wasm-compile-only bindgen generation"
            .to_owned(),
    )
}

pub(crate) fn simd_identity(
    target_arch: &str,
    disable_simd: bool,
    avx2_feature: bool,
) -> &'static str {
    if target_arch == "wasm32" || disable_simd {
        "disabled"
    } else if avx2_feature && target_arch == "x86_64" {
        "avx2"
    } else {
        "default"
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProviderInputs<'a> {
    pub(crate) target_arch: &'a str,
    pub(crate) target_os: &'a str,
    pub(crate) explicit_provider: Option<&'a str>,
    pub(crate) has_system_dir: bool,
    pub(crate) has_system_manifest: bool,
    pub(crate) has_prebuilt_manifest: bool,
    pub(crate) has_prebuilt_provenance: bool,
    pub(crate) has_prebuilt_bundle: bool,
    pub(crate) has_prebuilt_trusted_root: bool,
    pub(crate) build_from_source_enabled: bool,
    pub(crate) link_kind: Option<&'a str>,
}

pub(crate) fn select_provider(inputs: ProviderInputs<'_>) -> Result<ProviderAdapter, String> {
    if inputs.target_arch == "wasm32" {
        return select_wasm_provider(inputs);
    }

    let has_system_signal = inputs.has_system_dir || inputs.has_system_manifest;
    let has_prebuilt_signal = inputs.has_prebuilt_manifest
        || inputs.has_prebuilt_provenance
        || inputs.has_prebuilt_bundle
        || inputs.has_prebuilt_trusted_root;
    if has_system_signal && has_prebuilt_signal {
        return Err(
            "system and prebuilt provider inputs are both present; select exactly one adapter"
                .to_owned(),
        );
    }

    let adapter = match inputs.explicit_provider {
        Some(value) => parse_provider(value)?,
        None if has_system_signal => ProviderAdapter::System,
        None if has_prebuilt_signal => ProviderAdapter::Prebuilt,
        None => ProviderAdapter::Vendored,
    };

    if adapter.is_wasm() {
        return Err(format!(
            "provider `{}` is only valid for wasm32 targets",
            adapter.as_str()
        ));
    }
    match adapter {
        ProviderAdapter::Vendored => {
            if has_system_signal || has_prebuilt_signal {
                return Err(
                    "vendored provider cannot be combined with system or prebuilt inputs"
                        .to_owned(),
                );
            }
            if !inputs.build_from_source_enabled {
                return Err(
                    "vendored provider requires the `build-from-source` Cargo feature".to_owned(),
                );
            }
            if inputs.link_kind.is_some() {
                return Err(
                    "BOXDD_SYS_LINK_KIND is only valid for system or prebuilt providers".to_owned(),
                );
            }
        }
        ProviderAdapter::System => {
            if has_prebuilt_signal {
                return Err("system provider cannot use prebuilt provider inputs".to_owned());
            }
            if !inputs.has_system_dir || !inputs.has_system_manifest {
                return Err(
                    "system provider requires BOX2D_LIB_DIR and BOXDD_SYS_SYSTEM_MANIFEST"
                        .to_owned(),
                );
            }
            require_static_link(inputs.link_kind)?;
        }
        ProviderAdapter::Prebuilt => {
            if has_system_signal {
                return Err("prebuilt provider cannot use system provider inputs".to_owned());
            }
            if !inputs.has_prebuilt_manifest
                || !inputs.has_prebuilt_provenance
                || !inputs.has_prebuilt_bundle
            {
                return Err(
                    "prebuilt provider requires BOXDD_SYS_PREBUILT_MANIFEST, BOXDD_SYS_PREBUILT_PROVENANCE, and BOXDD_SYS_PREBUILT_BUNDLE"
                        .to_owned(),
                );
            }
            require_static_link(inputs.link_kind)?;
        }
        ProviderAdapter::WasmCompileOnly | ProviderAdapter::WasmProvider => {
            unreachable!("native target rejected WASM adapter")
        }
    }
    Ok(adapter)
}

fn select_wasm_provider(inputs: ProviderInputs<'_>) -> Result<ProviderAdapter, String> {
    if inputs.has_system_dir
        || inputs.has_system_manifest
        || inputs.has_prebuilt_manifest
        || inputs.has_prebuilt_provenance
        || inputs.has_prebuilt_bundle
        || inputs.has_prebuilt_trusted_root
        || inputs.link_kind.is_some()
    {
        return Err("native provider inputs cannot be used for a wasm32 target".to_owned());
    }
    let adapter = match inputs.explicit_provider {
        Some(value) => parse_provider(value)?,
        None => ProviderAdapter::WasmCompileOnly,
    };
    if !adapter.is_wasm() {
        return Err(format!(
            "provider `{}` is not valid for a wasm32 target",
            adapter.as_str()
        ));
    }
    if adapter == ProviderAdapter::WasmProvider && inputs.target_os != "unknown" {
        return Err(format!(
            "wasm-provider is runtime-qualified only for wasm32-unknown-unknown (target_os={:?}); use wasm-compile-only for this target",
            inputs.target_os
        ));
    }
    Ok(adapter)
}

fn parse_provider(value: &str) -> Result<ProviderAdapter, String> {
    match value {
        "vendored" => Ok(ProviderAdapter::Vendored),
        "system" => Ok(ProviderAdapter::System),
        "prebuilt" => Ok(ProviderAdapter::Prebuilt),
        "wasm-compile-only" => Ok(ProviderAdapter::WasmCompileOnly),
        "wasm-provider" => Ok(ProviderAdapter::WasmProvider),
        _ => Err(format!(
            "unsupported BOXDD_SYS_PROVIDER={value:?}; expected vendored, system, prebuilt, wasm-compile-only, or wasm-provider"
        )),
    }
}

fn require_static_link(link_kind: Option<&str>) -> Result<(), String> {
    match link_kind {
        None | Some("static") => Ok(()),
        Some(value) => Err(format!(
            "BOXDD_SYS_LINK_KIND={value:?} is not qualified; system and prebuilt providers are static-only"
        )),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SourceInventoryError {
    Empty,
    Duplicate(PathBuf),
    InvalidPath { path: String, reason: &'static str },
}

impl fmt::Display for SourceInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("C source inventory must not be empty"),
            Self::Duplicate(path) => {
                write!(
                    formatter,
                    "C source inventory contains duplicate path {path:?}"
                )
            }
            Self::InvalidPath { path, reason } => {
                write!(
                    formatter,
                    "invalid C source inventory path {path:?}: {reason}"
                )
            }
        }
    }
}

pub(crate) fn validate_c_source_paths<'a>(
    sources: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<PathBuf>, SourceInventoryError> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();

    for source in sources {
        let source_path = validate_c_source_path(source)?;
        if !seen.insert(source_path.clone()) {
            return Err(SourceInventoryError::Duplicate(source_path));
        }
        validated.push(source_path);
    }

    if validated.is_empty() {
        return Err(SourceInventoryError::Empty);
    }
    Ok(validated)
}

fn validate_c_source_path(source: &str) -> Result<PathBuf, SourceInventoryError> {
    if source.contains('\\') {
        return Err(invalid_path(source, "paths must use forward slashes"));
    }

    let segments = source.split('/').collect::<Vec<_>>();
    if segments.len() < 2
        || segments[0] != "src"
        || segments
            .iter()
            .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err(invalid_path(
            source,
            "paths must be normalized, relative, and below src/",
        ));
    }

    let source_path = PathBuf::from(source);
    if !source_path
        .extension()
        .is_some_and(|extension| extension == "c")
        || !source_path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(invalid_path(source, "paths must name a .c file"));
    }
    Ok(source_path)
}

fn invalid_path(path: &str, reason: &'static str) -> SourceInventoryError {
    SourceInventoryError::InvalidPath {
        path: path.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BindingTargetFamily, COSIGN_VERSION, PrebuiltProvenance, ProviderAdapter, ProviderInputs,
        SourceInventoryError, classify_binding_target, cosign_verify_blob_args,
        cosign_version_is_qualified, select_provider, simd_identity, validate_c_source_paths,
        validate_skip_cc_policy,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn accepts_normalized_nested_c_sources_in_manifest_order() {
        let sources = ["src/core.c", "src/solver/contact.c"];

        assert_eq!(
            validate_c_source_paths(sources).unwrap(),
            sources.map(PathBuf::from)
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_inventories() {
        assert_eq!(
            validate_c_source_paths(std::iter::empty()).unwrap_err(),
            SourceInventoryError::Empty
        );
        assert_eq!(
            validate_c_source_paths(["src/core.c", "src/core.c"]).unwrap_err(),
            SourceInventoryError::Duplicate(PathBuf::from("src/core.c"))
        );
    }

    #[test]
    fn rejects_paths_outside_the_reviewed_source_tree() {
        for source in [
            "/src/core.c",
            "core.c",
            "test/core.c",
            "src/../test/core.c",
            "src/./core.c",
            "src//core.c",
            "src\\core.c",
            "src/core.cpp",
        ] {
            assert!(
                matches!(
                    validate_c_source_paths([source]),
                    Err(SourceInventoryError::InvalidPath { .. })
                ),
                "unexpectedly accepted {source:?}"
            );
        }
    }

    fn native_inputs() -> ProviderInputs<'static> {
        ProviderInputs {
            target_arch: "x86_64",
            target_os: "linux",
            explicit_provider: None,
            has_system_dir: false,
            has_system_manifest: false,
            has_prebuilt_manifest: false,
            has_prebuilt_provenance: false,
            has_prebuilt_bundle: false,
            has_prebuilt_trusted_root: false,
            build_from_source_enabled: true,
            link_kind: None,
        }
    }

    #[test]
    fn vendored_is_the_only_implicit_native_default() {
        assert_eq!(
            select_provider(native_inputs()).unwrap(),
            ProviderAdapter::Vendored
        );
    }

    #[test]
    fn external_native_adapters_require_complete_static_inputs() {
        let mut system = native_inputs();
        system.explicit_provider = Some("system");
        assert!(select_provider(system).is_err());
        system.has_system_dir = true;
        system.has_system_manifest = true;
        system.link_kind = Some("static");
        assert_eq!(select_provider(system).unwrap(), ProviderAdapter::System);
        system.link_kind = Some("dylib");
        assert!(select_provider(system).is_err());

        let mut prebuilt = native_inputs();
        prebuilt.explicit_provider = Some("prebuilt");
        prebuilt.has_prebuilt_manifest = true;
        assert!(select_provider(prebuilt).is_err());
        prebuilt.has_prebuilt_provenance = true;
        assert!(select_provider(prebuilt).is_err());
        prebuilt.has_prebuilt_bundle = true;
        assert_eq!(
            select_provider(prebuilt).unwrap(),
            ProviderAdapter::Prebuilt
        );

        let mut root_override_only = native_inputs();
        root_override_only.has_prebuilt_trusted_root = true;
        assert!(select_provider(root_override_only).is_err());
    }

    #[test]
    fn multiple_provider_signals_fail_closed() {
        let mut inputs = native_inputs();
        inputs.has_system_dir = true;
        inputs.has_system_manifest = true;
        inputs.has_prebuilt_manifest = true;
        inputs.has_prebuilt_provenance = true;
        inputs.has_prebuilt_bundle = true;
        inputs.has_prebuilt_trusted_root = true;
        assert!(select_provider(inputs).is_err());
    }

    #[test]
    fn wasm_selection_is_explicit_and_native_inputs_are_rejected() {
        let mut inputs = native_inputs();
        inputs.target_arch = "wasm32";
        inputs.target_os = "unknown";
        assert_eq!(
            select_provider(inputs).unwrap(),
            ProviderAdapter::WasmCompileOnly
        );

        inputs.explicit_provider = Some("wasm-provider");
        assert_eq!(
            select_provider(inputs).unwrap(),
            ProviderAdapter::WasmProvider
        );
        for target_os in ["wasi", "emscripten"] {
            inputs.target_os = target_os;
            assert!(select_provider(inputs).is_err());
        }
        inputs.explicit_provider = None;
        assert_eq!(
            select_provider(inputs).unwrap(),
            ProviderAdapter::WasmCompileOnly
        );
        inputs.explicit_provider = Some("wasm-source");
        assert!(select_provider(inputs).is_err());
        inputs.explicit_provider = None;
        inputs.has_system_dir = true;
        assert!(select_provider(inputs).is_err());
    }

    #[test]
    fn checked_in_bindings_are_selected_by_exact_target_family_and_precision() {
        let native =
            classify_binding_target("x86_64-unknown-linux-gnu", "unix", "x86_64", "linux", "gnu")
                .unwrap();
        assert_eq!(native, BindingTargetFamily::Native);
        assert_eq!(
            native.pregenerated_bindings_file(false),
            "bindings_pregenerated.rs"
        );
        assert_eq!(
            native.pregenerated_bindings_file(true),
            "bindings_double.rs"
        );

        let unknown =
            classify_binding_target("wasm32-unknown-unknown", "wasm", "wasm32", "unknown", "")
                .unwrap();
        assert_eq!(unknown, BindingTargetFamily::WasmUnknownUnknown);
        assert_eq!(
            unknown.pregenerated_bindings_file(false),
            "bindings_wasm32_unknown_unknown.rs"
        );
        assert_eq!(
            unknown.pregenerated_bindings_file(true),
            "bindings_wasm32_unknown_unknown_double.rs"
        );

        let wasip1 =
            classify_binding_target("wasm32-wasip1", "wasm", "wasm32", "wasi", "p1").unwrap();
        assert_eq!(wasip1, BindingTargetFamily::WasmWasiP1);
        assert_eq!(
            wasip1.pregenerated_bindings_file(false),
            "bindings_wasm32_wasip1.rs"
        );
        assert_eq!(
            wasip1.pregenerated_bindings_file(true),
            "bindings_wasm32_wasip1_double.rs"
        );
    }

    #[test]
    fn unsupported_or_inconsistent_wasm_targets_fail_closed() {
        for (target, target_family, target_arch, target_os, target_env) in [
            ("wasm32-wasip2", "wasm", "wasm32", "wasi", "p2"),
            (
                "wasm32-unknown-emscripten",
                "wasm",
                "wasm32",
                "emscripten",
                "",
            ),
            ("custom-wasm32", "wasm", "wasm32", "unknown", ""),
            ("wasm32-wasip1", "wasm", "wasm32", "wasi", "p2"),
            ("wasm32-unknown-unknown", "wasm", "wasm32", "wasi", "p1"),
            ("wasm64-unknown-unknown", "wasm", "wasm64", "unknown", ""),
            ("wasm32-wasip1", "unix,wasm", "wasm32", "wasi", "p1"),
            ("wasm32-wasip1", "", "wasm32", "wasi", "p1"),
        ] {
            assert!(
                classify_binding_target(target, target_family, target_arch, target_os, target_env)
                    .is_err(),
                "unexpectedly accepted target={target:?}, target_family={target_family:?}, target_arch={target_arch:?}, target_os={target_os:?}, target_env={target_env:?}"
            );
        }
    }

    #[test]
    fn prebuilt_provenance_binds_repository_workflow_tag_and_commit() {
        assert_eq!(COSIGN_VERSION, "v3.0.6");
        assert!(cosign_version_is_qualified("GitVersion: v3.0.6"));
        assert!(!cosign_version_is_qualified("GitVersion: v3.0.60"));
        let commit = "1234567890abcdef1234567890abcdef12345678";
        let args = cosign_verify_blob_args(PrebuiltProvenance {
            crate_version: "0.6.0",
            source_commit: commit,
            release_tag: "v0.6.0",
            payload: Path::new("artifact.toml"),
            bundle: Path::new("artifact.sigstore.json"),
            trusted_root: Path::new("trusted-root.json"),
        })
        .unwrap();
        let args = args
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>();
        assert!(args.iter().any(|arg| {
            arg.as_ref()
                == "https://github.com/Latias94/boxdd/.github/workflows/prebuilt-binaries.yml@refs/tags/v0.6.0"
        }));
        assert!(args.iter().any(|arg| arg.as_ref() == commit));
        assert!(args.windows(2).any(|pair| {
            pair[0].as_ref() == "--certificate-github-workflow-trigger"
                && pair[1].as_ref() == "push"
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0].as_ref() == "--certificate-github-workflow-name"
                && pair[1].as_ref() == "Build Prebuilt Binaries (boxdd-sys)"
        }));
        assert!(
            cosign_verify_blob_args(PrebuiltProvenance {
                crate_version: "0.6.0",
                source_commit: commit,
                release_tag: "main",
                payload: Path::new("artifact.toml"),
                bundle: Path::new("artifact.sigstore.json"),
                trusted_root: Path::new("trusted-root.json"),
            })
            .is_err()
        );
    }

    #[test]
    fn skip_cc_is_fail_closed_to_bindgen_fixture_only() {
        assert!(validate_skip_cc_policy(false, false, false, ProviderAdapter::Vendored).is_ok());
        assert!(validate_skip_cc_policy(true, true, false, ProviderAdapter::Vendored).is_ok());
        assert!(validate_skip_cc_policy(false, true, true, ProviderAdapter::Vendored).is_ok());
        assert!(
            validate_skip_cc_policy(false, true, true, ProviderAdapter::WasmCompileOnly).is_ok()
        );
        for provider in [
            ProviderAdapter::System,
            ProviderAdapter::Prebuilt,
            ProviderAdapter::WasmProvider,
        ] {
            assert!(validate_skip_cc_policy(false, true, true, provider).is_err());
        }
        assert!(validate_skip_cc_policy(false, true, false, ProviderAdapter::Vendored).is_err());
        assert!(
            validate_skip_cc_policy(false, true, false, ProviderAdapter::WasmCompileOnly).is_err()
        );
    }

    #[test]
    fn simd_identity_matches_the_actual_target_compiler_policy() {
        assert_eq!(simd_identity("wasm32", false, true), "disabled");
        assert_eq!(simd_identity("x86_64", false, true), "avx2");
        assert_eq!(simd_identity("aarch64", false, true), "default");
        assert_eq!(simd_identity("x86_64", true, true), "disabled");
    }
}
