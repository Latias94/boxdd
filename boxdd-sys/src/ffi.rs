#![allow(clippy::approx_constant)]
#![allow(rustdoc::bare_urls)]
#![allow(rustdoc::broken_intra_doc_links)]

#[cfg(boxdd_sys_wasm_provider)]
include!(concat!(env!("OUT_DIR"), "/wasm_provider_bindings.rs"));

#[cfg(all(not(boxdd_sys_wasm_provider), has_pregenerated, not(force_bindgen)))]
include!("bindings_pregenerated.rs");

#[cfg(all(
    not(boxdd_sys_wasm_provider),
    any(force_bindgen, not(has_pregenerated))
))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
