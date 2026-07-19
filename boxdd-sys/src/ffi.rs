#![allow(clippy::approx_constant)]
#![allow(rustdoc::bare_urls)]
#![allow(rustdoc::broken_intra_doc_links)]

#[cfg(boxdd_sys_legacy_wasm_provider_bindings)]
include!(concat!(env!("OUT_DIR"), "/wasm_provider_bindings.rs"));

#[cfg(all(
    not(boxdd_sys_legacy_wasm_provider_bindings),
    not(feature = "double-precision"),
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_pregenerated.rs");

#[cfg(all(
    not(boxdd_sys_legacy_wasm_provider_bindings),
    feature = "double-precision",
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_double.rs");

#[cfg(all(
    not(boxdd_sys_legacy_wasm_provider_bindings),
    any(force_bindgen, not(has_pregenerated))
))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(feature = "double-precision")]
pub use b2CreateWorldDoublePrecision as b2CreateWorld;
