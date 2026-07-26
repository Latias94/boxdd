//! Fail-closed qualification for the Emscripten SDK used by the WASM provider.

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

pub(crate) const SDK_CONTRACT_RELATIVE_PATH: &str = "emscripten-sdk.toml";

pub(crate) const PROVIDER_ABI: &str = "box2d-sys-v1";
pub(crate) const EMSCRIPTEN_VERSION: &str = "6.0.3";
const EMSDK_REPOSITORY: &str = "https://github.com/emscripten-core/emsdk.git";
const EMSDK_REVISION: &str = "db04e88298d9916fc51fcd3743045ca3eb695127";
const EMSCRIPTEN_RELEASE: &str = "9074aa513b501925adb1361e208932ad32a29a5f";
const EMSCRIPTEN_REVISION: &str = "283e2d130132859fde6a4e4c87fd254b38127651";
const NODE_VERSION: &str = "22.16.0";
const PYTHON_VERSION: &str = "3.13.3";
pub(crate) const WASM_BINDGEN_VERSION: &str = "0.2.126";

const SDK_CONTRACT_FIELDS: &[&str] = &[
    "schema_version",
    "provider_abi",
    "emscripten_version",
    "emsdk_repository",
    "emsdk_revision",
    "emscripten_release",
    "emscripten_revision",
    "node_version",
    "python_version",
    "wasm_bindgen_version",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SdkContract {
    pub(crate) provider_abi: String,
    pub(crate) emscripten_version: String,
    pub(crate) emsdk_repository: String,
    pub(crate) emsdk_revision: String,
    emscripten_release: String,
    emscripten_revision: String,
    pub(crate) node_version: String,
    python_version: String,
    pub(crate) wasm_bindgen_version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostPlatform {
    Linux,
    MacOs,
    Windows,
}

#[derive(Debug)]
struct SdkEvidence {
    emsdk_repository: String,
    emsdk_revision: String,
    emsdk_head_reference: String,
    emsdk_status: String,
    emscripten_release: String,
    emscripten_version: String,
    emscripten_revision: String,
    activated_config: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QualifiedEmscriptenSdk {
    pub(crate) compiler: PathBuf,
    pub(crate) em_config: PathBuf,
    pub(crate) contract_sha256: String,
    pub(crate) watched_paths: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EmscriptenSdkInputs<'a> {
    pub(crate) root: Option<&'a OsStr>,
    pub(crate) compiler_override: Option<&'a OsStr>,
    pub(crate) em_config_override: Option<&'a OsStr>,
    pub(crate) self_attested_revision: Option<&'a OsStr>,
}

pub(crate) fn qualify_emscripten_sdk(
    manifest_dir: &Path,
    inputs: EmscriptenSdkInputs<'_>,
) -> Result<QualifiedEmscriptenSdk, String> {
    if inputs.self_attested_revision.is_some() {
        return Err(
            "BOXDD_EMSDK_REVISION is a self-attestation and is not accepted by the wasm-provider build; use EMSDK pointing at the pinned checkout"
                .to_owned(),
        );
    }

    let root = PathBuf::from(inputs.root.ok_or_else(|| {
        "BOXDD_SYS_PROVIDER=wasm-provider requires EMSDK to name the pinned SDK checkout; PATH-only compiler discovery is not qualified"
            .to_owned()
    })?);

    let contract_path = manifest_dir.join(SDK_CONTRACT_RELATIVE_PATH);
    let contract_bytes = read_regular_file(&contract_path, "Emscripten SDK contract")?;
    let contract_source = std::str::from_utf8(&contract_bytes)
        .map_err(|error| format!("Emscripten SDK contract is not UTF-8: {error}"))?;
    let contract = SdkContract::parse(contract_source)?;
    let contract_sha256 = sha256_bytes(&contract_bytes);

    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("failed to resolve EMSDK {}: {error}", root.display()))?;
    if !canonical_root.is_dir() {
        return Err(format!(
            "EMSDK does not name a directory: {}",
            canonical_root.display()
        ));
    }

    let emscripten_root = canonical_directory_within(
        &canonical_root,
        &canonical_root.join("upstream").join("emscripten"),
        "installed Emscripten source root",
    )?;
    let release_path = canonical_regular_file_within(
        &canonical_root,
        &canonical_root.join("upstream").join(".emsdk_version"),
        "installed Emscripten release",
    )?;
    let version_path = canonical_regular_file_within(
        &canonical_root,
        &emscripten_root.join("emscripten-version.txt"),
        "installed Emscripten version",
    )?;
    let revision_path = canonical_regular_file_within(
        &canonical_root,
        &emscripten_root.join("emscripten-revision.txt"),
        "installed Emscripten revision",
    )?;
    let em_config = canonical_regular_file_within(
        &canonical_root,
        &canonical_root.join(".emscripten"),
        "activated Emscripten configuration",
    )?;
    if let Some(config_override) = inputs.em_config_override {
        let override_path =
            canonical_regular_file(Path::new(config_override), "EM_CONFIG override")?;
        if override_path != em_config {
            return Err(format!(
                "EM_CONFIG must resolve to the activated configuration inside EMSDK: expected {}, found {}",
                em_config.display(),
                override_path.display()
            ));
        }
    }

    let platform = HostPlatform::current()?;
    let evidence = SdkEvidence {
        emsdk_repository: run_git(&canonical_root, &["remote", "get-url", "origin"])?,
        emsdk_revision: run_git(&canonical_root, &["rev-parse", "--verify", "HEAD^{commit}"])?,
        emsdk_head_reference: run_git(&canonical_root, &["rev-parse", "--abbrev-ref", "HEAD"])?,
        emsdk_status: run_git(
            &canonical_root,
            &["status", "--porcelain=v1", "--untracked-files=no"],
        )?,
        emscripten_release: read_identity_file(&release_path, "installed Emscripten release")?,
        emscripten_version: read_identity_file(&version_path, "installed Emscripten version")?,
        emscripten_revision: read_identity_file(&revision_path, "installed Emscripten revision")?,
        activated_config: read_utf8_file(&em_config, "activated Emscripten configuration")?,
    };
    validate_sdk_evidence(&contract, platform, &evidence)?;

    let compiler = resolve_compiler(
        &canonical_root,
        &emscripten_root,
        inputs.compiler_override.map(Path::new),
        platform,
    )?;
    verify_compiler_version(&compiler, &em_config, &contract.emscripten_version)?;

    Ok(QualifiedEmscriptenSdk {
        watched_paths: vec![
            contract_path,
            canonical_root,
            em_config.clone(),
            release_path,
            version_path,
            revision_path,
            compiler.clone(),
        ],
        compiler,
        em_config,
        contract_sha256,
    })
}

impl SdkContract {
    pub(crate) fn parse(source: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(source)
            .map_err(|error| format!("invalid Emscripten SDK contract TOML: {error}"))?;
        let table = value
            .as_table()
            .ok_or_else(|| "Emscripten SDK contract root must be a TOML table".to_owned())?;
        let fields = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected = SDK_CONTRACT_FIELDS.iter().copied().collect::<BTreeSet<_>>();
        if fields != expected {
            return Err(format!(
                "Emscripten SDK contract fields do not match the closed schema: expected {expected:?}, found {fields:?}"
            ));
        }
        if table
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            != Some(1)
        {
            return Err("unsupported Emscripten SDK contract schema_version".to_owned());
        }

        let contract = Self {
            provider_abi: required_string(table, "provider_abi")?,
            emscripten_version: required_string(table, "emscripten_version")?,
            emsdk_repository: required_string(table, "emsdk_repository")?,
            emsdk_revision: required_string(table, "emsdk_revision")?,
            emscripten_release: required_string(table, "emscripten_release")?,
            emscripten_revision: required_string(table, "emscripten_revision")?,
            node_version: required_string(table, "node_version")?,
            python_version: required_string(table, "python_version")?,
            wasm_bindgen_version: required_string(table, "wasm_bindgen_version")?,
        };
        let expected = Self::expected();
        if contract != expected {
            return Err(format!(
                "Emscripten SDK contract does not match the provider's pinned identity: expected {expected:?}, found {contract:?}"
            ));
        }
        Ok(contract)
    }

    fn expected() -> Self {
        Self {
            provider_abi: PROVIDER_ABI.to_owned(),
            emscripten_version: EMSCRIPTEN_VERSION.to_owned(),
            emsdk_repository: EMSDK_REPOSITORY.to_owned(),
            emsdk_revision: EMSDK_REVISION.to_owned(),
            emscripten_release: EMSCRIPTEN_RELEASE.to_owned(),
            emscripten_revision: EMSCRIPTEN_REVISION.to_owned(),
            node_version: NODE_VERSION.to_owned(),
            python_version: PYTHON_VERSION.to_owned(),
            wasm_bindgen_version: WASM_BINDGEN_VERSION.to_owned(),
        }
    }
}

// Included by xtask as well as the build script; only xtask owns the workspace lockfile.
#[allow(dead_code)]
pub(crate) fn validate_wasm_bindgen_lock(
    contract: &SdkContract,
    lockfile_source: &str,
) -> Result<(), String> {
    const CRATES_IO_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";

    let lockfile: toml::Value = toml::from_str(lockfile_source)
        .map_err(|error| format!("invalid Cargo.lock TOML: {error}"))?;
    let packages = lockfile
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "Cargo.lock must contain a package array".to_owned())?;
    let packages = packages
        .iter()
        .filter_map(|package| {
            package.as_table().filter(|package| {
                package.get("name").and_then(toml::Value::as_str) == Some("wasm-bindgen")
            })
        })
        .collect::<Vec<_>>();
    let [package] = packages.as_slice() else {
        return Err(format!(
            "Cargo.lock must contain exactly one wasm-bindgen package; found {}",
            packages.len()
        ));
    };
    let version = required_string(package, "version")?;
    let source = required_string(package, "source")?;
    require_identity(
        "Cargo.lock wasm-bindgen version",
        &version,
        &contract.wasm_bindgen_version,
    )?;
    require_identity("Cargo.lock wasm-bindgen source", &source, CRATES_IO_SOURCE)
}

impl HostPlatform {
    fn current() -> Result<Self, String> {
        if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else if cfg!(target_os = "macos") {
            Ok(Self::MacOs)
        } else if cfg!(target_os = "windows") {
            Ok(Self::Windows)
        } else {
            Err(format!(
                "the pinned Emscripten SDK configuration is not qualified on build host {}",
                env::consts::OS
            ))
        }
    }

    const fn compiler_name(self) -> &'static str {
        match self {
            Self::Linux | Self::MacOs => "emcc",
            Self::Windows => "emcc.bat",
        }
    }
}

fn validate_sdk_evidence(
    contract: &SdkContract,
    platform: HostPlatform,
    evidence: &SdkEvidence,
) -> Result<(), String> {
    require_identity(
        "emsdk origin",
        &evidence.emsdk_repository,
        &contract.emsdk_repository,
    )?;
    require_identity(
        "emsdk revision",
        &evidence.emsdk_revision,
        &contract.emsdk_revision,
    )?;
    require_identity(
        "emsdk HEAD reference",
        &evidence.emsdk_head_reference,
        "HEAD",
    )?;
    if !evidence.emsdk_status.is_empty() {
        return Err(format!(
            "the pinned emsdk checkout has modified tracked files:\n{}",
            evidence.emsdk_status
        ));
    }
    require_identity(
        "installed Emscripten release",
        &evidence.emscripten_release,
        &format!("releases-{}-64bit", contract.emscripten_release),
    )?;
    require_identity(
        "installed Emscripten version",
        &evidence.emscripten_version,
        &format!("\"{}\"", contract.emscripten_version),
    )?;
    require_identity(
        "installed Emscripten revision",
        &evidence.emscripten_revision,
        &contract.emscripten_revision,
    )?;

    let actual_config = normalize_newlines(&evidence.activated_config)?;
    let expected_config = expected_activated_config(contract, platform);
    if actual_config != expected_config {
        return Err(
            "the activated .emscripten configuration does not match the pinned SDK tool paths"
                .to_owned(),
        );
    }
    Ok(())
}

fn expected_activated_config(contract: &SdkContract, platform: HostPlatform) -> String {
    let executable_suffix = if platform == HostPlatform::Windows {
        ".exe"
    } else {
        ""
    };
    let mut config = format!(
        "import os\nemsdk_path = os.path.dirname(os.getenv('EM_CONFIG')).replace('\\\\', '/')\nNODE_JS = emsdk_path + '/node/{}_64bit/bin/node{}'\n",
        contract.node_version, executable_suffix
    );
    if matches!(platform, HostPlatform::MacOs | HostPlatform::Windows) {
        let python_executable = if platform == HostPlatform::Windows {
            "python.exe"
        } else {
            "bin/python3"
        };
        config.push_str(&format!(
            "PYTHON = emsdk_path + '/python/{}_64bit/{python_executable}'\n",
            contract.python_version
        ));
    }
    config.push_str(
        "LLVM_ROOT = emsdk_path + '/upstream/bin'\nBINARYEN_ROOT = emsdk_path + '/upstream'\nEMSCRIPTEN_ROOT = emsdk_path + '/upstream/emscripten'\n",
    );
    config
}

fn resolve_compiler(
    sdk_root: &Path,
    emscripten_root: &Path,
    compiler_override: Option<&Path>,
    platform: HostPlatform,
) -> Result<PathBuf, String> {
    let expected = canonical_regular_file_within(
        sdk_root,
        &emscripten_root.join(platform.compiler_name()),
        "pinned Emscripten compiler",
    )?;
    if let Some(override_path) = compiler_override {
        let override_path = canonical_regular_file(override_path, "BOXDD_SYS_EMCC override")?;
        if override_path != expected {
            return Err(format!(
                "BOXDD_SYS_EMCC may only assert the canonical compiler inside the qualified EMSDK: expected {}, found {}",
                expected.display(),
                override_path.display()
            ));
        }
    }
    Ok(expected)
}

fn verify_compiler_version(
    compiler: &Path,
    em_config: &Path,
    expected_version: &str,
) -> Result<(), String> {
    let output = Command::new(compiler)
        .arg("--em-config")
        .arg(em_config)
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "failed to execute qualified Emscripten compiler {}: {error}",
                compiler.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "qualified Emscripten compiler failed --version with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if !version_has_exact_token(&version, expected_version) {
        return Err(format!(
            "wasm-provider requires Emscripten {expected_version}; found {}",
            version.lines().next().unwrap_or("unknown version")
        ));
    }
    Ok(())
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    for (key, _) in env::vars_os() {
        if is_git_environment_key(&key) {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_OPTIONAL_LOCKS", "0");
    let output = command
        .arg("--no-replace-objects")
        .args(["-c", "core.fsmonitor=false"])
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| {
            format!(
                "failed to execute git for EMSDK {}: {error}",
                root.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "failed to inspect EMSDK {} with git {:?}: {}",
            root.display(),
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("git printed non-UTF-8 EMSDK identity: {error}"))
}

fn is_git_environment_key(key: &OsStr) -> bool {
    key.to_string_lossy()
        .to_ascii_uppercase()
        .starts_with("GIT_")
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

fn read_identity_file(path: &Path, label: &str) -> Result<String, String> {
    let source = read_utf8_file(path, label)?;
    let normalized = normalize_newlines(&source)?;
    let identity = normalized.trim_end_matches('\n');
    if identity.is_empty() || identity.contains('\n') {
        return Err(format!("{label} must contain exactly one non-empty line"));
    }
    Ok(identity.to_owned())
}

fn read_utf8_file(path: &Path, label: &str) -> Result<String, String> {
    let bytes = read_regular_file(path, label)?;
    String::from_utf8(bytes).map_err(|error| format!("{label} is not UTF-8: {error}"))
}

fn read_regular_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    fs::read(path).map_err(|error| format!("failed to read {label} {}: {error}", path.display()))
}

fn canonical_regular_file(path: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))
}

fn canonical_regular_file_within(root: &Path, path: &Path, label: &str) -> Result<PathBuf, String> {
    let canonical = canonical_regular_file(path, label)?;
    require_path_within(root, &canonical, label)?;
    Ok(canonical)
}

fn canonical_directory_within(root: &Path, path: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must be a real directory: {}",
            path.display()
        ));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))?;
    require_path_within(root, &canonical, label)?;
    Ok(canonical)
}

fn require_path_within(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(format!(
            "{label} escapes the qualified EMSDK root {}: {}",
            root.display(),
            path.display()
        ))
    }
}

fn normalize_newlines(source: &str) -> Result<String, String> {
    let normalized = source.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err(
            "Emscripten SDK identity contains unsupported bare carriage returns".to_owned(),
        );
    }
    Ok(normalized)
}

fn require_identity(label: &str, actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} does not match the pinned wasm-provider identity: expected {expected:?}, found {actual:?}"
        ))
    }
}

fn required_string(table: &toml::Table, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Emscripten SDK contract field `{key}` must be a non-empty string"))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn version_has_exact_token(output: &str, expected: &str) -> bool {
    output
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '.' | '-' | '+')
        })
        .any(|token| token == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_evidence(contract: &SdkContract, platform: HostPlatform) -> SdkEvidence {
        SdkEvidence {
            emsdk_repository: contract.emsdk_repository.clone(),
            emsdk_revision: contract.emsdk_revision.clone(),
            emsdk_head_reference: "HEAD".to_owned(),
            emsdk_status: String::new(),
            emscripten_release: format!("releases-{}-64bit", contract.emscripten_release),
            emscripten_version: format!("\"{}\"", contract.emscripten_version),
            emscripten_revision: contract.emscripten_revision.clone(),
            activated_config: expected_activated_config(contract, platform),
        }
    }

    #[test]
    fn sdk_contract_is_exact_and_closed() {
        let source = include_str!("../emscripten-sdk.toml");
        assert_eq!(SdkContract::parse(source).unwrap(), SdkContract::expected());
        assert!(SdkContract::parse(&source.replace("6.0.3", "6.0.30")).is_err());
        assert!(SdkContract::parse(&format!("{source}\nunknown = true\n")).is_err());
    }

    #[test]
    fn sdk_evidence_binds_checkout_release_and_activated_config() {
        let contract = SdkContract::expected();
        for platform in [
            HostPlatform::Linux,
            HostPlatform::MacOs,
            HostPlatform::Windows,
        ] {
            let evidence = valid_evidence(&contract, platform);
            validate_sdk_evidence(&contract, platform, &evidence).unwrap();

            let mut stale = valid_evidence(&contract, platform);
            stale.emscripten_revision.replace_range(..1, "0");
            assert!(validate_sdk_evidence(&contract, platform, &stale).is_err());

            let mut attached = valid_evidence(&contract, platform);
            attached.emsdk_head_reference = "main".to_owned();
            assert!(validate_sdk_evidence(&contract, platform, &attached).is_err());

            let mut configured_elsewhere = valid_evidence(&contract, platform);
            configured_elsewhere
                .activated_config
                .push_str("LLVM_ROOT = '/tmp/llvm'\n");
            assert!(validate_sdk_evidence(&contract, platform, &configured_elsewhere).is_err());
        }
    }

    #[test]
    fn compiler_override_cannot_escape_the_qualified_sdk() {
        let temp = tempfile::tempdir().unwrap();
        let emscripten = temp.path().join("upstream").join("emscripten");
        fs::create_dir_all(&emscripten).unwrap();
        let compiler = emscripten.join(HostPlatform::current().unwrap().compiler_name());
        fs::write(&compiler, b"compiler").unwrap();
        let arbitrary = temp.path().join("arbitrary-emcc");
        fs::write(&arbitrary, b"compiler").unwrap();
        let sdk_root = fs::canonicalize(temp.path()).unwrap();

        assert_eq!(
            resolve_compiler(
                &sdk_root,
                &emscripten,
                Some(&compiler),
                HostPlatform::current().unwrap()
            )
            .unwrap(),
            fs::canonicalize(&compiler).unwrap()
        );
        assert!(
            resolve_compiler(
                &sdk_root,
                &emscripten,
                Some(&arbitrary),
                HostPlatform::current().unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn compiler_version_requires_an_exact_token() {
        assert!(version_has_exact_token("emcc 6.0.3", EMSCRIPTEN_VERSION));
        assert!(!version_has_exact_token("emcc 16.0.30", EMSCRIPTEN_VERSION));
        assert!(!version_has_exact_token(
            "emcc 6.0.3-beta",
            EMSCRIPTEN_VERSION
        ));
        assert!(!version_has_exact_token(
            "emcc 6.0.3+local",
            EMSCRIPTEN_VERSION
        ));
    }

    #[test]
    fn ambient_git_environment_is_removed_case_insensitively() {
        assert!(is_git_environment_key(OsStr::new("GIT_DIR")));
        assert!(is_git_environment_key(OsStr::new("git_object_directory")));
        assert!(!is_git_environment_key(OsStr::new("LEGIT_SETTING")));
    }

    #[cfg(unix)]
    #[test]
    fn installed_tree_cannot_escape_the_qualified_sdk_through_a_parent_symlink() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let sdk = temp.path().join("sdk");
        let external = temp.path().join("external");
        fs::create_dir_all(&sdk).unwrap();
        fs::create_dir_all(external.join("emscripten")).unwrap();
        symlink(&external, sdk.join("upstream")).unwrap();

        let error = canonical_directory_within(
            &fs::canonicalize(&sdk).unwrap(),
            &sdk.join("upstream/emscripten"),
            "installed Emscripten source root",
        )
        .unwrap_err();
        assert!(error.contains("escapes the qualified EMSDK root"));
    }

    #[test]
    fn path_and_self_attested_revisions_are_not_qualification_evidence() {
        let revision = std::ffi::OsString::from(EMSDK_REVISION);
        let error = qualify_emscripten_sdk(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            EmscriptenSdkInputs {
                root: None,
                compiler_override: None,
                em_config_override: None,
                self_attested_revision: Some(&revision),
            },
        )
        .unwrap_err();
        assert!(error.contains("self-attestation"));

        let error = qualify_emscripten_sdk(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            EmscriptenSdkInputs {
                root: None,
                compiler_override: None,
                em_config_override: None,
                self_attested_revision: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("PATH-only compiler discovery is not qualified"));
    }
}
