#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod precision;

mod expected_adapter_identity {
    include!(concat!(env!("OUT_DIR"), "/adapter_identity.rs"));
}

pub mod adapter;

#[cfg(test)]
mod build_support;

#[cfg(test)]
mod wasm_provider_contract;

pub mod ffi;

pub use precision::{
    ABI_PRECISION, ActivePrecision, DoublePrecision, IS_DOUBLE_PRECISION, Precision,
    SinglePrecision,
};

/// Exact private C ABI identity computed independently from target-compiler layout constants.
pub const PRIVATE_ABI_HASH: [u8; 32] = expected_adapter_identity::PRIVATE_ABI_HASH;

/// Exact private snapshot layout identity computed by the target compiler.
pub const SNAPSHOT_LAYOUT_HASH: u32 = expected_adapter_identity::SNAPSHOT_LAYOUT_HASH;

/// Exact upstream Box2D revision described by the build manifest.
pub const UPSTREAM_SHA: &str = env!("BOXDD_SYS_UPSTREAM_SHA");

/// SHA-256 of the repository-owned native adapter source contract.
pub const ADAPTER_SOURCE_SHA256: &str = env!("BOXDD_SYS_ADAPTER_SOURCE_SHA256");

/// SHA-256 of the complete reviewed Box2D source bytes compiled by this crate.
pub const EFFECTIVE_SOURCE_SHA256: &str = env!("BOXDD_SYS_EFFECTIVE_SOURCE_SHA256");

/// BLAKE3 digest of the reviewed recording contract bound into the native adapter.
pub const RECORDING_CONTRACT_BLAKE3: &str = env!("BOXDD_SYS_RECORDING_CONTRACT_BLAKE3");

/// Rust target ABI that the linked native adapter must report at runtime.
pub const TARGET_ABI: &str = env!("BOXDD_SYS_TARGET_ABI");

/// WASM import module selected for this build's precision and provider route.
pub const WASM_IMPORT_MODULE: &str = env!("BOXDD_SYS_WASM_IMPORT_MODULE");

/// Explicit build adapter selected for this crate instance.
pub const PROVIDER_ADAPTER: &str = env!("BOXDD_SYS_PROVIDER_ADAPTER");

/// SHA-256 of the verified external-provider manifest, or an empty string for vendored builds.
pub const PROVIDER_MANIFEST_SHA256: &str = env!("BOXDD_SYS_PROVIDER_MANIFEST_SHA256");

/// SHA-256 of the exact static archive linked by an external provider.
pub const PROVIDER_ARCHIVE_SHA256: &str = env!("BOXDD_SYS_PROVIDER_ARCHIVE_SHA256");

/// SHA-256 of the Sigstore bundle verified for an official prebuilt provider.
pub const PROVIDER_PROVENANCE_SHA256: &str = env!("BOXDD_SYS_PROVIDER_PROVENANCE_SHA256");

/// SHA-256 of the caller-supplied Sigstore trusted root used for verification.
pub const PROVIDER_TRUSTED_ROOT_SHA256: &str = env!("BOXDD_SYS_PROVIDER_TRUSTED_ROOT_SHA256");
