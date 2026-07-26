use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Component, Path},
};

use crate::emscripten_sdk::{SdkContract, validate_wasm_bindgen_lock};

const RELEASE_VERSION: &str = "0.6.0";
const RUST_VERSION: &str = "1.95";
const MSRV: &str = "1.95.0";
const DEVELOPMENT: &str = "1.97.1";
const VERIFICATION_NIGHTLY: &str = "nightly-2026-05-27";

#[derive(Debug)]
pub(crate) struct Verification {
    workspace_version: String,
    msrv: String,
    development: String,
    verification_nightly: String,
    wasm_bindgen: String,
}

impl fmt::Display for Verification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "toolchain configuration ok: workspace {}, MSRV {}, development {}, verification {}, wasm-bindgen {}",
            self.workspace_version,
            self.msrv,
            self.development,
            self.verification_nightly,
            self.wasm_bindgen
        )
    }
}

#[derive(Debug)]
pub(crate) struct VerifyError(String);

impl VerifyError {
    fn message(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for VerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VerifyError {}

#[derive(Deserialize)]
struct RootManifest {
    workspace: Workspace,
}

#[derive(Deserialize)]
struct Workspace {
    members: Vec<String>,
    package: WorkspacePackage,
    dependencies: WorkspaceDependencies,
    metadata: WorkspaceMetadata,
}

#[derive(Deserialize)]
struct WorkspacePackage {
    version: String,
    #[serde(rename = "rust-version")]
    rust_version: String,
}

#[derive(Deserialize)]
struct WorkspaceDependencies {
    boxdd: WorkspaceDependency,
    #[serde(rename = "boxdd-sys")]
    boxdd_sys: WorkspaceDependency,
}

#[derive(Deserialize)]
struct WorkspaceDependency {
    version: String,
    path: String,
}

#[derive(Deserialize)]
struct WorkspaceMetadata {
    toolchains: ToolchainPolicy,
}

#[derive(Deserialize)]
struct ToolchainPolicy {
    msrv: String,
    development: String,
    #[serde(rename = "verification-nightly")]
    verification_nightly: String,
    #[serde(rename = "verification-components")]
    verification_components: BTreeSet<String>,
}

#[derive(Deserialize)]
struct MemberManifest {
    package: MemberPackage,
    #[serde(default)]
    dependencies: BTreeMap<String, MemberDependency>,
}

#[derive(Deserialize)]
struct MemberPackage {
    name: String,
    version: WorkspaceInheritance,
    #[serde(rename = "rust-version")]
    rust_version: WorkspaceInheritance,
}

#[derive(Deserialize)]
struct WorkspaceInheritance {
    workspace: bool,
}

#[derive(Deserialize)]
struct MemberDependency {
    #[serde(default)]
    workspace: bool,
}

#[derive(Deserialize)]
struct ToolchainFile {
    toolchain: DevelopmentToolchain,
}

#[derive(Deserialize)]
struct DevelopmentToolchain {
    channel: String,
    components: BTreeSet<String>,
    profile: String,
}

pub(crate) fn verify_configuration(root: &Path) -> Result<Verification, VerifyError> {
    let workspace_path = root.join("Cargo.toml");
    let manifest: RootManifest = read_toml(&workspace_path)?;
    ensure_exact(
        "workspace package version",
        &manifest.workspace.package.version,
        RELEASE_VERSION,
    )?;
    ensure_exact(
        "workspace rust-version",
        &manifest.workspace.package.rust_version,
        RUST_VERSION,
    )?;
    verify_dependency_versions(
        &manifest.workspace.dependencies,
        &manifest.workspace.package.version,
    )?;
    verify_members(root, &manifest.workspace.members)?;

    let policy = &manifest.workspace.metadata.toolchains;
    ensure_exact("MSRV toolchain", &policy.msrv, MSRV)?;
    ensure_exact("development toolchain", &policy.development, DEVELOPMENT)?;
    ensure_exact(
        "verification nightly",
        &policy.verification_nightly,
        VERIFICATION_NIGHTLY,
    )?;
    ensure_components(
        "verification components",
        &policy.verification_components,
        &["miri", "rust-src"],
    )?;

    let toolchain_path = root.join("rust-toolchain.toml");
    let toolchain: ToolchainFile = read_toml(&toolchain_path)?;
    ensure_exact(
        "rust-toolchain channel",
        &toolchain.toolchain.channel,
        &policy.development,
    )?;
    ensure_exact(
        "rust-toolchain profile",
        &toolchain.toolchain.profile,
        "minimal",
    )?;
    ensure_components(
        "development components",
        &toolchain.toolchain.components,
        &["clippy", "rustfmt"],
    )?;

    let sdk_path = root.join("boxdd-sys/emscripten-sdk.toml");
    let sdk_source = fs::read_to_string(&sdk_path).map_err(|error| {
        VerifyError::message(format!("failed to read {}: {error}", sdk_path.display()))
    })?;
    let sdk = SdkContract::parse(&sdk_source).map_err(VerifyError::message)?;
    let lock_path = root.join("Cargo.lock");
    let lock_source = fs::read_to_string(&lock_path).map_err(|error| {
        VerifyError::message(format!("failed to read {}: {error}", lock_path.display()))
    })?;
    validate_wasm_bindgen_lock(&sdk, &lock_source).map_err(VerifyError::message)?;

    Ok(Verification {
        workspace_version: manifest.workspace.package.version,
        msrv: policy.msrv.clone(),
        development: policy.development.clone(),
        verification_nightly: policy.verification_nightly.clone(),
        wasm_bindgen: sdk.wasm_bindgen_version,
    })
}

fn verify_dependency_versions(
    dependencies: &WorkspaceDependencies,
    workspace_version: &str,
) -> Result<(), VerifyError> {
    let internal = BTreeMap::from([
        (
            "boxdd",
            (
                dependencies.boxdd.version.as_str(),
                dependencies.boxdd.path.as_str(),
            ),
        ),
        (
            "boxdd-sys",
            (
                dependencies.boxdd_sys.version.as_str(),
                dependencies.boxdd_sys.path.as_str(),
            ),
        ),
    ]);
    for (name, (version, path)) in internal {
        ensure_exact(
            &format!("workspace dependency `{name}` version"),
            version,
            workspace_version,
        )?;
        ensure_exact(&format!("workspace dependency `{name}` path"), path, name)?;
    }
    Ok(())
}

fn verify_members(root: &Path, members: &[String]) -> Result<(), VerifyError> {
    if members.is_empty() {
        return Err(VerifyError::message("workspace must declare members"));
    }
    for member in members {
        let relative = Path::new(member);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(VerifyError::message(format!(
                "workspace member `{member}` must stay under the workspace root"
            )));
        }
        let path = root.join(relative).join("Cargo.toml");
        let manifest: MemberManifest = read_toml(&path)?;
        require_inherited(&path, "version", manifest.package.version.workspace)?;
        require_inherited(
            &path,
            "rust-version",
            manifest.package.rust_version.workspace,
        )?;
        for dependency in required_internal_dependencies(&manifest.package.name) {
            let inherited = manifest
                .dependencies
                .get(*dependency)
                .is_some_and(|dependency| dependency.workspace);
            if !inherited {
                return Err(VerifyError::message(format!(
                    "{} dependency `{dependency}` must inherit from the workspace",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn required_internal_dependencies(package: &str) -> &'static [&'static str] {
    match package {
        "boxdd" => &["boxdd-sys"],
        "bevy_boxdd" | "boxdd-provider-smoke" => &["boxdd"],
        _ => &[],
    }
}

fn require_inherited(path: &Path, field: &str, inherited: bool) -> Result<(), VerifyError> {
    if inherited {
        return Ok(());
    }
    Err(VerifyError::message(format!(
        "{} `package.{field}.workspace` must be true",
        path.display()
    )))
}

fn ensure_components(
    label: &str,
    actual: &BTreeSet<String>,
    expected: &[&str],
) -> Result<(), VerifyError> {
    let actual: BTreeSet<_> = actual.iter().map(String::as_str).collect();
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    if actual == expected {
        return Ok(());
    }
    Err(VerifyError::message(format!(
        "{label} must be {expected:?}, found {actual:?}"
    )))
}

fn ensure_exact(label: &str, actual: &str, expected: &str) -> Result<(), VerifyError> {
    if actual == expected {
        return Ok(());
    }
    Err(VerifyError::message(format!(
        "{label} must be `{expected}`, found `{actual}`"
    )))
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T, VerifyError> {
    let source = fs::read_to_string(path).map_err(|error| {
        VerifyError::message(format!("failed to read {}: {error}", path.display()))
    })?;
    toml::from_str(&source).map_err(|error| {
        VerifyError::message(format!("failed to parse {}: {error}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: std::path::PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "boxdd-toolchains-{name}-{}-{nonce}",
                std::process::id()
            ));
            for member in ["boxdd", "boxdd-sys", "xtask"] {
                fs::create_dir_all(root.join(member))
                    .expect("member fixture directory should exist");
                let dependencies = if member == "boxdd" {
                    "\n[dependencies]\nboxdd-sys.workspace = true\n"
                } else {
                    ""
                };
                fs::write(
                    root.join(member).join("Cargo.toml"),
                    format!(
                        r#"[package]
name = "{member}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
{dependencies}
"#
                    ),
                )
                .expect("member fixture should be written");
            }
            fs::write(
                root.join("Cargo.toml"),
                r#"[workspace]
members = ["boxdd", "boxdd-sys", "xtask"]

[workspace.package]
version = "0.6.0"
edition = "2024"
rust-version = "1.95"

[workspace.dependencies]
boxdd = { version = "0.6.0", path = "boxdd" }
boxdd-sys = { version = "0.6.0", path = "boxdd-sys" }

[workspace.metadata.toolchains]
msrv = "1.95.0"
development = "1.97.1"
verification-nightly = "nightly-2026-05-27"
verification-components = ["miri", "rust-src"]
"#,
            )
            .expect("workspace fixture should be written");
            fs::write(
                root.join("rust-toolchain.toml"),
                r#"[toolchain]
channel = "1.97.1"
components = ["clippy", "rustfmt"]
profile = "minimal"
"#,
            )
            .expect("toolchain fixture should be written");
            fs::write(
                root.join("boxdd-sys/emscripten-sdk.toml"),
                include_str!("../../boxdd-sys/emscripten-sdk.toml"),
            )
            .expect("SDK contract fixture should be written");
            fs::write(
                root.join("Cargo.lock"),
                r#"version = 4

[[package]]
name = "wasm-bindgen"
version = "0.2.126"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#,
            )
            .expect("lockfile fixture should be written");
            Self { root }
        }

        fn rewrite(&self, relative: &str, from: &str, to: &str) {
            let path = self.root.join(relative);
            let source = fs::read_to_string(&path).expect("fixture should be readable");
            let updated = source.replacen(from, to, 1);
            assert_ne!(source, updated, "fixture replacement must match");
            fs::write(path, updated).expect("fixture should be rewritten");
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn accepts_consistent_workspace() {
        let fixture = Fixture::new("consistent");

        verify_configuration(&fixture.root).expect("consistent fixture should pass");
    }

    #[test]
    fn rejects_member_without_rust_version() {
        let fixture = Fixture::new("member-msrv");
        fixture.rewrite("xtask/Cargo.toml", "rust-version.workspace = true\n", "");

        let error =
            verify_configuration(&fixture.root).expect_err("member without rust-version must fail");
        assert!(error.to_string().contains("rust-version"));
    }

    #[test]
    fn rejects_member_without_workspace_version() {
        let fixture = Fixture::new("member-version");
        fixture.rewrite("xtask/Cargo.toml", "version.workspace = true\n", "");

        let error =
            verify_configuration(&fixture.root).expect_err("member without version must fail");
        assert!(error.to_string().contains("version"));
    }

    #[test]
    fn rejects_workspace_release_version_drift() {
        let fixture = Fixture::new("release-version");
        fixture.rewrite("Cargo.toml", "version = \"0.6.0\"", "version = \"0.5.0\"");

        let error = verify_configuration(&fixture.root)
            .expect_err("workspace release version drift must fail");
        assert!(error.to_string().contains("workspace package version"));
    }

    #[test]
    fn rejects_internal_dependency_version_drift() {
        let fixture = Fixture::new("dependency-version");
        fixture.rewrite(
            "Cargo.toml",
            "boxdd = { version = \"0.6.0\"",
            "boxdd = { version = \"0.5.0\"",
        );

        let error =
            verify_configuration(&fixture.root).expect_err("internal version drift must fail");
        assert!(
            error
                .to_string()
                .contains("workspace dependency `boxdd` version")
        );
    }

    #[test]
    fn rejects_internal_dependency_path_drift() {
        let fixture = Fixture::new("dependency-path");
        fixture.rewrite(
            "Cargo.toml",
            "boxdd-sys = { version = \"0.6.0\", path = \"boxdd-sys\" }",
            "boxdd-sys = { version = \"0.6.0\", path = \"vendor/boxdd-sys\" }",
        );

        let error = verify_configuration(&fixture.root)
            .expect_err("internal dependency path drift must fail");
        assert!(
            error
                .to_string()
                .contains("workspace dependency `boxdd-sys` path")
        );
    }

    #[test]
    fn rejects_member_without_workspace_internal_dependency() {
        let fixture = Fixture::new("member-dependency");
        fixture.rewrite(
            "boxdd/Cargo.toml",
            "\n[dependencies]\nboxdd-sys.workspace = true\n",
            "",
        );

        let error = verify_configuration(&fixture.root)
            .expect_err("member internal dependency must inherit from workspace");
        assert!(error.to_string().contains("dependency `boxdd-sys`"));
    }

    #[test]
    fn rejects_development_channel_drift() {
        let fixture = Fixture::new("development-channel");
        fixture.rewrite(
            "rust-toolchain.toml",
            "channel = \"1.97.1\"",
            "channel = \"1.96.0\"",
        );

        let error =
            verify_configuration(&fixture.root).expect_err("development channel drift must fail");
        assert!(error.to_string().contains("rust-toolchain channel"));
    }

    #[test]
    fn rejects_missing_verification_component() {
        let fixture = Fixture::new("nightly-components");
        fixture.rewrite(
            "Cargo.toml",
            "verification-components = [\"miri\", \"rust-src\"]",
            "verification-components = [\"miri\"]",
        );

        let error =
            verify_configuration(&fixture.root).expect_err("missing nightly component must fail");
        assert!(error.to_string().contains("verification components"));
    }

    #[test]
    fn rejects_wasm_bindgen_lockfile_drift() {
        let fixture = Fixture::new("wasm-bindgen-lockfile");
        fixture.rewrite("Cargo.lock", "version = \"0.2.126\"", "version = \"0.2.0\"");

        let error =
            verify_configuration(&fixture.root).expect_err("wasm-bindgen lockfile drift must fail");
        assert!(error.to_string().contains("wasm-bindgen version"));
    }
}
