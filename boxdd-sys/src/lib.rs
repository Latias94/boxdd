#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod ffi {
    #![allow(clippy::approx_constant)]
    // Prefer wasm-specific pregenerated bindings when targeting wasm32
    #[cfg(all(target_arch = "wasm32", has_wasm_pregenerated))]
    include!("wasm_bindings_pregenerated.rs");
    // Otherwise use general pregenerated if available
    #[cfg(all(
        has_pregenerated,
        not(all(target_arch = "wasm32", has_wasm_pregenerated))
    ))]
    include!("bindings_pregenerated.rs");
    // Fallback to generated bindings in OUT_DIR
    #[cfg(not(any(has_pregenerated, all(target_arch = "wasm32", has_wasm_pregenerated))))]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
