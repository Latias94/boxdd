#![allow(clippy::approx_constant)]
#![allow(rustdoc::bare_urls)]
#![allow(rustdoc::broken_intra_doc_links)]

#[cfg(all(
    not(target_family = "wasm"),
    not(feature = "double-precision"),
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_pregenerated.rs");

#[cfg(all(
    not(target_family = "wasm"),
    feature = "double-precision",
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_double.rs");

#[cfg(all(
    target_arch = "wasm32",
    target_family = "wasm",
    target_os = "unknown",
    target_env = "",
    not(feature = "double-precision"),
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_wasm32_unknown_unknown.rs");

#[cfg(all(
    target_arch = "wasm32",
    target_family = "wasm",
    target_os = "unknown",
    target_env = "",
    feature = "double-precision",
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_wasm32_unknown_unknown_double.rs");

#[cfg(all(
    target_arch = "wasm32",
    target_family = "wasm",
    target_os = "wasi",
    target_env = "p1",
    not(feature = "double-precision"),
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_wasm32_wasip1.rs");

#[cfg(all(
    target_arch = "wasm32",
    target_family = "wasm",
    target_os = "wasi",
    target_env = "p1",
    feature = "double-precision",
    has_pregenerated,
    not(force_bindgen)
))]
include!("bindings_wasm32_wasip1_double.rs");

#[cfg(all(
    target_family = "wasm",
    not(any(
        all(target_arch = "wasm32", target_os = "unknown", target_env = ""),
        all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
    ))
))]
compile_error!("boxdd-sys supports WASM targets only for wasm32-unknown-unknown and wasm32-wasip1");

#[cfg(any(force_bindgen, not(has_pregenerated)))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(feature = "double-precision")]
pub use b2CreateWorldDoublePrecision as b2CreateWorld;
