//! Rust target classification and target-dependent build identities.

use std::collections::BTreeSet;

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
