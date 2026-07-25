use std::{ffi::OsString, path::Path};

pub(crate) const COSIGN_VERSION: &str = "v3.0.6";
#[allow(dead_code)]
pub(crate) const SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH: &str = "security/sigstore/trusted_root.json";
#[allow(dead_code)]
pub(crate) const SIGSTORE_TRUSTED_ROOT_SHA256: &str =
    "6494e21ea73fa7ee769f85f57d5a3e6a08725eae1e38c755fc3517c9e6bc0b66";
pub(crate) const PUBLISHER_REPOSITORY: &str = "Latias94/boxdd";
pub(crate) const PUBLISHER_WORKFLOW: &str = ".github/workflows/prebuilt-binaries.yml";
pub(crate) const PUBLISHER_WORKFLOW_NAME: &str = "Build Prebuilt Binaries (boxdd-sys)";
pub(crate) const GITHUB_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";

pub(crate) struct PrebuiltProvenance<'a> {
    pub(crate) crate_version: &'a str,
    pub(crate) source_commit: &'a str,
    pub(crate) release_tag: &'a str,
    pub(crate) payload: &'a Path,
    pub(crate) bundle: &'a Path,
    pub(crate) trusted_root: &'a Path,
}

pub(crate) fn cosign_verify_blob_args(
    provenance: PrebuiltProvenance<'_>,
) -> Result<Vec<OsString>, String> {
    if provenance.source_commit.len() != 40
        || !provenance
            .source_commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("prebuilt source commit must be a lowercase Git SHA".to_owned());
    }
    let short_tag = format!("v{}", provenance.crate_version);
    let crate_tag = format!("boxdd-sys-v{}", provenance.crate_version);
    if provenance.release_tag != short_tag && provenance.release_tag != crate_tag {
        return Err(format!(
            "prebuilt release tag `{}` does not match crate version {}",
            provenance.release_tag, provenance.crate_version
        ));
    }
    let git_ref = format!("refs/tags/{}", provenance.release_tag);
    let certificate_identity =
        format!("https://github.com/{PUBLISHER_REPOSITORY}/{PUBLISHER_WORKFLOW}@{git_ref}");
    Ok(vec![
        "verify-blob".into(),
        "--bundle".into(),
        provenance.bundle.as_os_str().to_owned(),
        "--trusted-root".into(),
        provenance.trusted_root.as_os_str().to_owned(),
        "--certificate-identity".into(),
        certificate_identity.into(),
        "--certificate-oidc-issuer".into(),
        GITHUB_OIDC_ISSUER.into(),
        "--certificate-github-workflow-repository".into(),
        PUBLISHER_REPOSITORY.into(),
        "--certificate-github-workflow-ref".into(),
        git_ref.into(),
        "--certificate-github-workflow-sha".into(),
        provenance.source_commit.into(),
        "--certificate-github-workflow-trigger".into(),
        "push".into(),
        "--certificate-github-workflow-name".into(),
        PUBLISHER_WORKFLOW_NAME.into(),
        provenance.payload.as_os_str().to_owned(),
    ])
}

pub(crate) fn cosign_version_is_qualified(output: &str) -> bool {
    output
        .split(|character: char| {
            character.is_ascii_whitespace() || matches!(character, ':' | ',' | '"' | '\'')
        })
        .any(|token| token == COSIGN_VERSION)
}
