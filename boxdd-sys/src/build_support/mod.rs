//! Build-policy primitives shared by the `boxdd-sys` library tests and build script.

pub(crate) mod atomic_publish;
pub(crate) mod provider_selection;
pub(crate) mod target;
pub(crate) mod verified_snapshot;

#[allow(unused_imports)]
pub(crate) use crate::provenance_policy::{
    COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW, PrebuiltProvenance,
    SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH, SIGSTORE_TRUSTED_ROOT_SHA256, cosign_verify_blob_args,
    cosign_version_is_qualified, release_tag_matches_version, workspace_release_tag,
};
pub use verified_snapshot::VerifiedFileSnapshot;

#[cfg(test)]
mod tests;
