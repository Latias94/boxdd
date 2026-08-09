//! Fail-closed parsing and selection of native and WASM provider routes.

use std::ffi::OsStr;

use crate::provider_catalog::ProviderCapability as ProviderAdapter;

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

pub(crate) fn validate_force_bindgen_policy(
    force_bindgen: bool,
    provider: ProviderAdapter,
) -> Result<(), String> {
    if !force_bindgen
        || matches!(
            provider,
            ProviderAdapter::Vendored | ProviderAdapter::WasmCompileOnly
        )
    {
        return Ok(());
    }
    Err(format!(
        "BOXDD_SYS_FORCE_BINDGEN=1 is incompatible with the authenticated {} provider bindings identity",
        provider.as_str()
    ))
}

pub(crate) fn parse_optional_bool(key: &str, value: Option<&OsStr>) -> Result<bool, String> {
    let Some(value) = value else {
        return Ok(false);
    };
    let value = value
        .to_str()
        .ok_or_else(|| format!("{key} must be valid Unicode and a documented boolean value"))?;
    match value {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "{key}={value:?} is not a supported boolean; expected 1, true, yes, on, 0, false, no, or off"
        )),
    }
}

/// Read an optional environment setting without treating a non-Unicode value as absent.
///
/// Cargo environment variables are OS strings. Provider identity affects which native bytes are
/// accepted, so a value that cannot be represented as UTF-8 must fail before selection rather
/// than silently taking the default route.
pub(crate) fn parse_optional_unicode<'a>(
    key: &str,
    value: Option<&'a OsStr>,
) -> Result<Option<&'a str>, String> {
    value
        .map(|value| {
            value
                .to_str()
                .ok_or_else(|| format!("{key} must be valid Unicode when set"))
        })
        .transpose()
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
    if inputs.explicit_provider.is_none() && (has_system_signal || has_prebuilt_signal) {
        return Err(
            "external native provider inputs require an explicit BOXDD_SYS_PROVIDER=system or BOXDD_SYS_PROVIDER=prebuilt selector"
                .to_owned(),
        );
    }

    let adapter = match inputs.explicit_provider {
        Some(value) => parse_provider(value)?,
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
    ProviderAdapter::parse_build_name(value)
        .map_err(|error| format!("invalid BOXDD_SYS_PROVIDER: {error}"))
}

fn require_static_link(link_kind: Option<&str>) -> Result<(), String> {
    match link_kind {
        None | Some("static") => Ok(()),
        Some(value) => Err(format!(
            "BOXDD_SYS_LINK_KIND={value:?} is not qualified; system and prebuilt providers are static-only"
        )),
    }
}
