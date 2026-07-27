//! Fail-closed qualification for the Emscripten SDK used by repository WASM tooling.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
#[cfg(unix)]
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::qualified_git::{
    configured_git_command, is_process_injection_environment_key, qualified_git_executable,
    remove_process_injection_environment,
};

pub(crate) use crate::wasm_provider_contract::PROVIDER_ABI;

pub(crate) const SDK_CONTRACT_RELATIVE_PATH: &str = "toolchains/emscripten-sdk.toml";
const SDK_CONTRACT_SHA256: &str =
    "73e628846dcb3732005b3cf8598e9eee4909a1f5ddb333c31ff946855c51e9e8";

pub(crate) const EMSCRIPTEN_VERSION: &str = "6.0.3";
pub(crate) const WASM_BINDGEN_VERSION: &str = "0.2.126";
const WASM_BINDGEN_CHECKSUM: &str =
    "4b067c0c11094aef6b7a801c1e34a26affafdf3d051dba08456b868789aaf9a4";
const WASM_BINDGEN_CLI_SUPPORT_CHECKSUM: &str =
    "80c3e3bac5bcdc2a15ba22862e2cb7c0d1beddf9d46dd320bec1f6ae82f2da53";

const TREE_DIGEST_DOMAIN: &[u8] = b"boxdd-emsdk-tree-v1\0";
const EMCC_BOOTSTRAP: &str = "import runpy,sys; root=sys.argv.pop(1); driver=sys.argv.pop(1); sys.path.insert(0,root); sys.argv[0]=driver; runpy.run_path(driver,run_name='__main__')";
const PROVISION_COMMAND_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const PROVISION_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);
const TRUSTED_PROVISION_PATH: &str = "/usr/bin:/bin";

#[cfg(unix)]
unsafe extern "C" {
    #[link_name = "kill"]
    fn send_unix_signal(process: i32, signal: i32) -> i32;
}

const SDK_CONTRACT_FIELDS: &[&str] = &[
    "schema_version",
    "provider_abi",
    "emscripten_version",
    "emsdk_repository",
    "emsdk_revision",
    "emscripten_release",
    "emscripten_revision",
    "node_version",
    "wasm_bindgen_version",
    "hosts",
];

const HOST_CONTRACT_FIELDS: &[&str] = &[
    "id",
    "os",
    "arch",
    "release_url",
    "release_archive_sha256",
    "release_tree_sha256",
    "node_url",
    "node_archive_sha256",
    "node_tree_sha256",
    "host_probe",
    "python",
];

const OS_ARCH_HOST_PROBE_FIELDS: &[&str] = &["kind"];
const LINUX_OS_RELEASE_HOST_PROBE_FIELDS: &[&str] = &["kind", "id", "version_id"];
const SYSTEM_PYTHON_FIELDS: &[&str] = &["kind", "executable", "runtime_root", "minimum_version"];
const ARCHIVE_PYTHON_FIELDS: &[&str] = &[
    "kind",
    "version",
    "url",
    "archive_sha256",
    "tree_sha256",
    "executable",
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
    pub(crate) wasm_bindgen_version: String,
    hosts: Vec<HostToolchainContract>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HostToolchainContract {
    id: String,
    os: String,
    arch: String,
    release_url: String,
    release_archive_sha256: String,
    release_tree_sha256: String,
    node_url: String,
    node_archive_sha256: String,
    node_tree_sha256: String,
    host_probe: HostProbeContract,
    python: PythonRuntimeContract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum HostProbeContract {
    OsArch,
    LinuxOsRelease { id: String, version_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PythonRuntimeContract {
    System {
        executable: String,
        runtime_root: String,
        minimum_version: String,
    },
    Archive {
        version: String,
        url: String,
        archive_sha256: String,
        tree_sha256: String,
        executable: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PythonVersionRequirement<'a> {
    Exact(&'a str),
    Minimum(&'a str),
}

impl PythonRuntimeContract {
    fn version_requirement(&self) -> PythonVersionRequirement<'_> {
        match self {
            Self::System {
                minimum_version, ..
            } => PythonVersionRequirement::Minimum(minimum_version),
            Self::Archive { version, .. } => PythonVersionRequirement::Exact(version),
        }
    }

    fn executable_path(&self, sdk_root: &Path) -> PathBuf {
        match self {
            Self::System { executable, .. } => PathBuf::from(executable),
            Self::Archive {
                version,
                executable,
                ..
            } => archived_python_root(sdk_root, version).join(executable),
        }
    }
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

#[derive(Debug)]
pub(crate) struct QualifiedEmscriptenSdk {
    root: PathBuf,
    compiler: PathBuf,
    compiler_driver: PathBuf,
    wasm_opt: PathBuf,
    emscripten_root: PathBuf,
    em_config: PathBuf,
    python: PathBuf,
    python_runtime_root: PathBuf,
    node: PathBuf,
    git: PathBuf,
    contract: SdkContract,
    contract_path: PathBuf,
    contract_sha256: String,
    platform: HostPlatform,
    release_path: PathBuf,
    version_path: PathBuf,
    revision_path: PathBuf,
    watched_files: Vec<QualifiedFile>,
    qualified_trees: Vec<QualifiedTree>,
    scratch: tempfile::TempDir,
    cache: PathBuf,
    ports: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QualifiedFile {
    path: PathBuf,
    sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QualifiedTree {
    label: &'static str,
    root: PathBuf,
    sha256: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EmscriptenSdkInputs<'a> {
    pub(crate) root: Option<&'a OsStr>,
    pub(crate) em_config_override: Option<&'a OsStr>,
    pub(crate) python_override: Option<&'a OsStr>,
    pub(crate) node_override: Option<&'a OsStr>,
    pub(crate) self_attested_revision: Option<&'a OsStr>,
}

#[derive(Clone, Copy, Debug)]
struct QualifiedToolEnvironment<'a> {
    root: &'a Path,
    em_config: &'a Path,
    python: &'a Path,
    node: &'a Path,
    cache: &'a Path,
    ports: &'a Path,
    scratch: &'a Path,
}

impl QualifiedToolEnvironment<'_> {
    fn configure<I, K>(&self, command: &mut Command, keys: I)
    where
        I: IntoIterator<Item = K>,
        K: AsRef<OsStr>,
    {
        for key in keys {
            let key = key.as_ref();
            if is_qualified_tool_environment_key(key) {
                command.env_remove(key);
            }
        }
        command
            .env("EMSDK", self.root)
            .env("EM_CONFIG", self.em_config)
            .env("EMSDK_PYTHON", self.python)
            .env("EMSDK_NODE", self.node)
            .env("EM_CACHE", self.cache)
            .env("EM_PORTS", self.ports)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("TMPDIR", self.scratch)
            .env("TMP", self.scratch)
            .env("TEMP", self.scratch);
    }
}

impl QualifiedEmscriptenSdk {
    pub(crate) fn emcc_command(&self) -> Result<Command, String> {
        self.revalidate()?;
        Ok(self.unchecked_emcc_command())
    }

    fn unchecked_emcc_command(&self) -> Command {
        configured_emcc_command(
            &self.emscripten_root,
            &self.compiler_driver,
            env::vars_os().map(|(key, _)| key),
            self.tool_environment(),
        )
    }

    fn tool_environment(&self) -> QualifiedToolEnvironment<'_> {
        QualifiedToolEnvironment {
            root: &self.root,
            em_config: &self.em_config,
            python: &self.python,
            node: &self.node,
            cache: &self.cache,
            ports: &self.ports,
            scratch: self.scratch.path(),
        }
    }

    pub(crate) fn contract_sha256(&self) -> &str {
        &self.contract_sha256
    }

    pub(crate) fn node_command(&self) -> Result<Command, String> {
        self.revalidate()?;
        let mut command = Command::new(&self.node);
        remove_process_injection_environment(&mut command);
        remove_matching_environment(&mut command, is_node_environment_key);
        command.env("TMPDIR", self.scratch.path());
        command.env("TMP", self.scratch.path());
        command.env("TEMP", self.scratch.path());
        Ok(command)
    }

    pub(crate) fn wasm_opt_command(&self) -> Result<Command, String> {
        self.revalidate()?;
        let mut command = Command::new(&self.wasm_opt);
        remove_process_injection_environment(&mut command);
        remove_matching_environment(&mut command, is_qualified_tool_environment_key);
        command.env("TMPDIR", self.scratch.path());
        command.env("TMP", self.scratch.path());
        command.env("TEMP", self.scratch.path());
        Ok(command)
    }

    pub(crate) fn revalidate(&self) -> Result<(), String> {
        validate_ambient_emscripten_environment()?;
        require_private_emsdk_root(&self.root, "qualified EMSDK root")?;
        validate_qualified_files(&self.watched_files)?;
        validate_qualified_trees(&self.qualified_trees)?;
        validate_scratch_directory(&self.scratch, &self.cache, &self.ports)?;
        let contract_bytes = read_regular_file(&self.contract_path, "Emscripten SDK contract")?;
        require_identity(
            "Emscripten SDK contract SHA-256",
            &sha256_bytes(&contract_bytes),
            &self.contract_sha256,
        )?;
        let contract_source = std::str::from_utf8(&contract_bytes)
            .map_err(|error| format!("Emscripten SDK contract is not UTF-8: {error}"))?;
        let contract = SdkContract::parse(contract_source)?;
        if contract != self.contract {
            return Err("Emscripten SDK contract changed after qualification".to_owned());
        }
        let evidence = collect_sdk_evidence(
            &self.git,
            &self.root,
            &self.release_path,
            &self.version_path,
            &self.revision_path,
            &self.em_config,
        )?;
        let host = self.contract.current_host()?;
        validate_sdk_evidence(&self.contract, host, self.platform, &evidence)?;
        let resolved_python = resolve_python(&self.root, Some(&self.python), &host.python)?;
        if resolved_python.runtime_root != self.python_runtime_root
            || resolved_python.executable != self.python
        {
            return Err("qualified Python runtime changed after SDK qualification".to_owned());
        }
        verify_python_runtime(
            &self.python,
            &self.python_runtime_root,
            host.python.version_requirement(),
        )?;
        verify_node_version(&self.node, &self.contract.node_version)?;
        verify_compiler_version_unchecked(self, &self.contract.emscripten_version)
    }
}

fn configured_emcc_command<I, K>(
    emscripten_root: &Path,
    compiler_driver: &Path,
    keys: I,
    environment: QualifiedToolEnvironment<'_>,
) -> Command
where
    I: IntoIterator<Item = K>,
    K: AsRef<OsStr>,
{
    let mut command = Command::new(environment.python);
    environment.configure(&mut command, keys);
    command
        .arg("-I")
        .arg("-B")
        .arg("-X")
        .arg("utf8")
        .arg("-c")
        .arg(EMCC_BOOTSTRAP)
        .arg(emscripten_root)
        .arg(compiler_driver)
        .arg("--em-config")
        .arg(environment.em_config);
    command
}

fn remove_matching_environment(command: &mut Command, predicate: fn(&OsStr) -> bool) {
    for (key, _) in env::vars_os() {
        if predicate(&key) {
            command.env_remove(key);
        }
    }
}

pub(crate) fn qualify_emscripten_sdk(
    xtask_manifest_dir: &Path,
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

    let contract_path = canonical_regular_file(
        &xtask_manifest_dir.join(SDK_CONTRACT_RELATIVE_PATH),
        "Emscripten SDK contract",
    )?;
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
    require_private_emsdk_root(&canonical_root, "EMSDK root")?;
    validate_ambient_emscripten_environment()?;
    let platform = HostPlatform::current()?;
    let host = contract.current_host()?.clone();

    let upstream_root = canonical_directory_within(
        &canonical_root,
        &canonical_root.join("upstream"),
        "installed Emscripten release tree",
    )?;
    let emscripten_root = canonical_directory_within(
        &canonical_root,
        &upstream_root.join("emscripten"),
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

    let resolved_python = resolve_python(
        &canonical_root,
        inputs.python_override.map(Path::new),
        &host.python,
    )?;
    let python_runtime_root = resolved_python.runtime_root;
    let python = resolved_python.executable;

    let node_root = canonical_directory_within(
        &canonical_root,
        &canonical_root
            .join("node")
            .join(format!("{}_64bit", contract.node_version)),
        "installed EMSDK Node.js runtime",
    )?;
    let node_version_path = canonical_regular_file_within(
        &node_root,
        &node_root.join(".emsdk_version"),
        "installed EMSDK Node.js identity",
    )?;
    require_identity(
        "installed EMSDK Node.js identity",
        &read_identity_file(&node_version_path, "installed EMSDK Node.js identity")?,
        &format!("node-{}-64bit", contract.node_version),
    )?;
    let node = resolve_node(&node_root, inputs.node_override.map(Path::new), platform)?;
    let wasm_opt = resolve_wasm_opt(&canonical_root, &upstream_root, platform)?;

    let mut qualified_trees = vec![
        qualify_tree(
            "installed Emscripten release tree",
            upstream_root,
            &host.release_tree_sha256,
        )?,
        qualify_tree(
            "installed EMSDK Node.js tree",
            node_root,
            &host.node_tree_sha256,
        )?,
    ];
    if let Some((python_root, tree_sha256)) = resolved_python.archive_tree {
        qualified_trees.push(qualify_tree(
            "installed EMSDK Python tree",
            python_root,
            &tree_sha256,
        )?);
    }

    verify_python_runtime(
        &python,
        &python_runtime_root,
        host.python.version_requirement(),
    )?;
    verify_node_version(&node, &contract.node_version)?;

    let git = qualified_git_executable()?;
    let evidence = collect_sdk_evidence(
        &git,
        &canonical_root,
        &release_path,
        &version_path,
        &revision_path,
        &em_config,
    )?;
    validate_sdk_evidence(&contract, &host, platform, &evidence)?;

    let compiler = resolve_compiler(&canonical_root, &emscripten_root, platform)?;
    let compiler_driver = resolve_compiler_driver(&canonical_root, &emscripten_root)?;
    let watched_paths = vec![
        contract_path.clone(),
        em_config.clone(),
        git.clone(),
        python.clone(),
    ];
    let watched_files = qualify_files(watched_paths)?;
    let scratch = tempfile::Builder::new()
        .prefix("boxdd-emscripten-")
        .tempdir()
        .map_err(|error| {
            format!("failed to create private Emscripten scratch directory: {error}")
        })?;
    secure_private_directory(scratch.path(), "private Emscripten scratch directory")?;
    let cache = scratch.path().join("cache");
    let ports = scratch.path().join("ports");
    fs::create_dir(&cache)
        .map_err(|error| format!("failed to create private Emscripten cache: {error}"))?;
    fs::create_dir(&ports)
        .map_err(|error| format!("failed to create private Emscripten ports cache: {error}"))?;
    secure_private_directory(&cache, "private Emscripten cache")?;
    secure_private_directory(&ports, "private Emscripten ports cache")?;
    let sdk = QualifiedEmscriptenSdk {
        root: canonical_root,
        compiler,
        compiler_driver,
        wasm_opt,
        emscripten_root,
        em_config,
        python,
        python_runtime_root,
        node,
        git,
        contract,
        contract_path,
        contract_sha256,
        platform,
        release_path,
        version_path,
        revision_path,
        watched_files,
        qualified_trees,
        scratch,
        cache,
        ports,
    };
    sdk.revalidate()?;
    Ok(sdk)
}

pub(crate) fn provision_emscripten_sdk(
    xtask_manifest_dir: &Path,
    destination: &Path,
    emit_github_actions_environment: bool,
) -> Result<(), String> {
    let contract_path = canonical_regular_file(
        &xtask_manifest_dir.join(SDK_CONTRACT_RELATIVE_PATH),
        "Emscripten SDK contract",
    )?;
    let contract_bytes = read_regular_file(&contract_path, "Emscripten SDK contract")?;
    let contract_source = std::str::from_utf8(&contract_bytes)
        .map_err(|error| format!("Emscripten SDK contract is not UTF-8: {error}"))?;
    let contract = SdkContract::parse(contract_source)?;
    let platform = HostPlatform::current()?;
    let host = contract.current_host()?.clone();
    let (destination, destination_exists) = canonical_provision_destination(destination)?;
    if destination_exists {
        reuse_qualified_emscripten_sdk(
            xtask_manifest_dir,
            &destination,
            &contract,
            &host,
            emit_github_actions_environment,
        )?;
        println!(
            "existing qualified Emscripten SDK reused at {}",
            destination.display()
        );
        return Ok(());
    }
    let parent = destination.parent().ok_or_else(|| {
        format!(
            "Emscripten provisioning destination has no parent directory: {}",
            destination.display()
        )
    })?;
    let staging = tempfile::Builder::new()
        .prefix(".boxdd-emsdk-")
        .tempdir_in(parent)
        .map_err(|error| {
            format!(
                "failed to create Emscripten provisioning directory under {}: {error}",
                parent.display()
            )
        })?;
    secure_private_directory(staging.path(), "Emscripten provisioning directory")?;

    initialize_pinned_emsdk_checkout(staging.path(), &contract)?;
    let tools = qualified_provision_tools(platform)?;
    let downloads = staging.path().join(".boxdd-downloads");
    fs::create_dir(&downloads).map_err(|error| {
        format!(
            "failed to create Emscripten provisioning downloads directory {}: {error}",
            downloads.display()
        )
    })?;
    secure_private_directory(&downloads, "Emscripten provisioning downloads directory")?;

    let release_root = staging.path().join("upstream");
    let node_root = staging
        .path()
        .join("node")
        .join(format!("{}_64bit", contract.node_version));
    provision_component_tree(
        &tools,
        &downloads.join("emscripten-release.archive"),
        &host.release_url,
        &host.release_archive_sha256,
        &release_root,
        "Emscripten release",
    )?;
    provision_component_tree(
        &tools,
        &downloads.join("node.archive"),
        &host.node_url,
        &host.node_archive_sha256,
        &node_root,
        "EMSDK Node.js",
    )?;
    if let PythonRuntimeContract::Archive {
        version,
        url,
        archive_sha256,
        ..
    } = &host.python
    {
        let python_root = archived_python_root(staging.path(), version);
        provision_component_tree(
            &tools,
            &downloads.join("python.archive"),
            url,
            archive_sha256,
            &python_root,
            "EMSDK Python",
        )?;
        write_provisioned_identity(
            &python_root.join(".emsdk_version"),
            &format!("python-{version}-64bit"),
            "installed EMSDK Python identity",
        )?;
    }
    write_provisioned_component_identities(&contract, &release_root, &node_root)?;
    fs::write(
        staging.path().join(".emscripten"),
        expected_activated_config(&contract, &host, platform),
    )
    .map_err(|error| {
        format!(
            "failed to write activated Emscripten configuration under {}: {error}",
            staging.path().display()
        )
    })?;
    fs::remove_dir_all(&downloads).map_err(|error| {
        format!(
            "failed to remove Emscripten provisioning downloads directory {}: {error}",
            downloads.display()
        )
    })?;

    let qualified = qualify_emscripten_sdk(
        xtask_manifest_dir,
        EmscriptenSdkInputs {
            root: Some(staging.path().as_os_str()),
            em_config_override: None,
            python_override: None,
            node_override: None,
            self_attested_revision: None,
        },
    )?;
    drop(qualified);

    if let Err(publish_error) = fs::rename(staging.path(), &destination) {
        let reuse_result = canonical_provision_destination(&destination).and_then(
            |(concurrent_destination, exists)| {
                if !exists {
                    return Err("the destination remains absent".to_owned());
                }
                reuse_qualified_emscripten_sdk(
                    xtask_manifest_dir,
                    &concurrent_destination,
                    &contract,
                    &host,
                    emit_github_actions_environment,
                )
            },
        );
        match reuse_result {
            Ok(()) => {
                println!(
                    "concurrently provisioned qualified Emscripten SDK reused at {}",
                    destination.display()
                );
                return Ok(());
            }
            Err(reuse_error) => {
                return Err(format!(
                    "failed to publish qualified Emscripten SDK at {}: {publish_error}; no qualified concurrent provision could be reused: {reuse_error}",
                    destination.display()
                ));
            }
        }
    }
    let _ = staging.keep();
    let destination = fs::canonicalize(&destination).map_err(|error| {
        format!(
            "failed to resolve published Emscripten SDK {}: {error}",
            destination.display()
        )
    })?;
    if emit_github_actions_environment {
        write_github_actions_emsdk_environment(&destination, &contract, &host)?;
    }
    println!(
        "qualified Emscripten SDK provisioned at {}",
        destination.display()
    );
    Ok(())
}

fn reuse_qualified_emscripten_sdk(
    xtask_manifest_dir: &Path,
    destination: &Path,
    contract: &SdkContract,
    host: &HostToolchainContract,
    emit_github_actions_environment: bool,
) -> Result<(), String> {
    let qualified = qualify_emscripten_sdk(
        xtask_manifest_dir,
        EmscriptenSdkInputs {
            root: Some(destination.as_os_str()),
            em_config_override: None,
            python_override: None,
            node_override: None,
            self_attested_revision: None,
        },
    )?;
    drop(qualified);
    if emit_github_actions_environment {
        write_github_actions_emsdk_environment(destination, contract, host)?;
    }
    Ok(())
}

fn canonical_provision_destination(destination: &Path) -> Result<(PathBuf, bool), String> {
    if !destination.is_absolute() {
        return Err(format!(
            "Emscripten provisioning destination must be absolute: {}",
            destination.display()
        ));
    }
    let parent = destination.parent().ok_or_else(|| {
        format!(
            "Emscripten provisioning destination has no parent directory: {}",
            destination.display()
        )
    })?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        format!(
            "failed to resolve Emscripten provisioning parent {}: {error}",
            parent.display()
        )
    })?;
    require_real_canonical_directory(&parent, "Emscripten provisioning parent")?;
    #[cfg(unix)]
    require_trusted_directory_ancestry(&parent, "Emscripten provisioning parent")?;
    let name = destination.file_name().ok_or_else(|| {
        format!(
            "Emscripten provisioning destination must have one final path component: {}",
            destination.display()
        )
    })?;
    if Path::new(name)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "Emscripten provisioning destination must have a normalized final path component: {}",
            destination.display()
        ));
    }
    let destination = parent.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            let canonical = fs::canonicalize(&destination).map_err(|error| {
                format!(
                    "failed to resolve existing Emscripten provisioning destination {}: {error}",
                    destination.display()
                )
            })?;
            if canonical != destination {
                return Err(format!(
                    "existing Emscripten provisioning destination is not canonical: expected {}, found {}",
                    destination.display(),
                    canonical.display()
                ));
            }
            Ok((destination, true))
        }
        Ok(_) => Err(format!(
            "Emscripten provisioning destination must be absent or a real directory: {}",
            destination.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((destination, false)),
        Err(error) => Err(format!(
            "failed to inspect Emscripten provisioning destination {}: {error}",
            destination.display()
        )),
    }
}

fn initialize_pinned_emsdk_checkout(root: &Path, contract: &SdkContract) -> Result<(), String> {
    let git = qualified_git_executable()?;
    require_root_owned_system_file(&git, "qualified system Git for Emscripten provisioning")?;
    let mut initialize = configured_provision_git_command(&git);
    initialize.arg("init").arg(root);
    run_provision_command(&mut initialize, "initialize pinned EMSDK checkout")?;
    run_git(
        &git,
        root,
        &["remote", "add", "origin", &contract.emsdk_repository],
    )?;
    run_git(
        &git,
        root,
        &["fetch", "--depth", "1", "origin", &contract.emsdk_revision],
    )?;
    run_git(&git, root, &["checkout", "--detach", "FETCH_HEAD"])?;
    require_identity(
        "provisioned emsdk revision",
        &run_git(&git, root, &["rev-parse", "--verify", "HEAD^{commit}"])?,
        &contract.emsdk_revision,
    )
}

struct QualifiedProvisionTools {
    curl: PathBuf,
    tar: PathBuf,
    xz: Option<PathBuf>,
}

fn qualified_provision_tools(platform: HostPlatform) -> Result<QualifiedProvisionTools, String> {
    Ok(QualifiedProvisionTools {
        curl: qualified_provision_curl()?,
        tar: qualified_provision_tar(platform)?,
        xz: qualified_provision_xz(platform)?,
    })
}

fn qualified_provision_curl() -> Result<PathBuf, String> {
    let curl = canonical_regular_file(
        Path::new("/usr/bin/curl"),
        "qualified system curl for Emscripten provisioning",
    )?;
    require_root_owned_system_file(&curl, "qualified system curl for Emscripten provisioning")?;
    Ok(curl)
}

fn qualified_provision_tar(platform: HostPlatform) -> Result<PathBuf, String> {
    let path = match platform {
        HostPlatform::Linux => "/usr/bin/tar",
        HostPlatform::MacOs => "/usr/bin/bsdtar",
        HostPlatform::Windows => {
            return Err("Emscripten provisioning is not qualified on Windows".to_owned());
        }
    };
    let tar = canonical_regular_file(
        Path::new(path),
        "qualified system tar for Emscripten provisioning",
    )?;
    require_root_owned_system_file(&tar, "qualified system tar for Emscripten provisioning")?;
    Ok(tar)
}

fn qualified_provision_xz(platform: HostPlatform) -> Result<Option<PathBuf>, String> {
    match platform {
        HostPlatform::Linux => {
            let xz = canonical_regular_file(
                Path::new("/usr/bin/xz"),
                "qualified system xz for Emscripten provisioning",
            )?;
            require_root_owned_system_file(&xz, "qualified system xz for Emscripten provisioning")?;
            Ok(Some(xz))
        }
        HostPlatform::MacOs => Ok(None),
        HostPlatform::Windows => {
            Err("Emscripten provisioning is not qualified on Windows".to_owned())
        }
    }
}

fn provision_component_tree(
    tools: &QualifiedProvisionTools,
    archive: &Path,
    url: &str,
    expected_sha256: &str,
    destination: &Path,
    label: &str,
) -> Result<(), String> {
    download_verified_archive(&tools.curl, archive, url, expected_sha256, label)?;
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "failed to create {label} extraction directory {}: {error}",
            destination.display()
        )
    })?;
    let mut extract =
        configured_archive_extract_command(&tools.tar, tools.xz.as_deref(), archive, destination);
    run_provision_command(&mut extract, &format!("extract verified {label} archive"))?;
    require_real_canonical_directory(destination, &format!("extracted {label} tree"))
}

fn download_verified_archive(
    curl: &Path,
    archive: &Path,
    url: &str,
    expected_sha256: &str,
    label: &str,
) -> Result<(), String> {
    require_lower_sha256(&format!("{label} archive SHA-256"), expected_sha256)?;
    let mut download = configured_archive_download_command(curl, archive, url);
    run_provision_command(&mut download, &format!("download pinned {label} archive"))?;
    require_identity(
        &format!("{label} archive SHA-256"),
        &sha256_file(archive)?,
        expected_sha256,
    )
}

fn configured_archive_download_command(curl: &Path, archive: &Path, url: &str) -> Command {
    let mut download = Command::new(curl);
    remove_process_injection_environment(&mut download);
    remove_matching_environment(&mut download, is_curl_environment_key);
    download
        .args([
            "--disable",
            "--fail",
            "--location",
            "--connect-timeout",
            "30",
            "--max-time",
            "900",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--retry",
            "4",
            "--retry-all-errors",
            "--retry-delay",
            "5",
            "--retry-max-time",
            "1200",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(archive)
        .arg(url);
    download
}

fn configured_archive_extract_command(
    tar: &Path,
    xz: Option<&Path>,
    archive: &Path,
    destination: &Path,
) -> Command {
    let mut extract = Command::new(tar);
    remove_process_injection_environment(&mut extract);
    remove_matching_environment(&mut extract, is_tar_environment_key);
    extract.env("PATH", TRUSTED_PROVISION_PATH);
    if let Some(xz) = xz {
        extract.arg("--use-compress-program").arg(xz);
    }
    extract
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(destination)
        .arg("--strip-components=1");
    extract
}

fn run_provision_command(command: &mut Command, label: &str) -> Result<(), String> {
    let output = command_output_with_timeout(command, label, PROVISION_COMMAND_TIMEOUT)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to {label} with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn command_output_with_timeout(
    command: &mut Command,
    label: &str,
    timeout: Duration,
) -> Result<Output, String> {
    let mut stdout = tempfile::tempfile().map_err(|error| {
        format!("failed to create stdout capture while trying to {label}: {error}")
    })?;
    let mut stderr = tempfile::tempfile().map_err(|error| {
        format!("failed to create stderr capture while trying to {label}: {error}")
    })?;
    let stdout_sink = stdout.try_clone().map_err(|error| {
        format!("failed to clone stdout capture while trying to {label}: {error}")
    })?;
    let stderr_sink = stderr.try_clone().map_err(|error| {
        format!("failed to clone stderr capture while trying to {label}: {error}")
    })?;
    command
        .stdout(Stdio::from(stdout_sink))
        .stderr(Stdio::from(stderr_sink));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;

        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to {label}: {error}"))?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to wait while trying to {label}: {error}"))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            let kill_error = terminate_provision_process_tree(&mut child).err();
            let wait_error = child.wait().err();
            let cleanup = match (kill_error, wait_error) {
                (None, None) => String::new(),
                (kill_error, wait_error) => format!(
                    "; cleanup errors: kill={:?}, wait={:?}",
                    kill_error.map(|error| error.to_string()),
                    wait_error.map(|error| error.to_string())
                ),
            };
            return Err(format!(
                "timed out after {timeout:?} while trying to {label}{cleanup}"
            ));
        }
        thread::sleep(PROVISION_COMMAND_POLL_INTERVAL);
    };
    stdout
        .seek(SeekFrom::Start(0))
        .and_then(|_| {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        })
        .and_then(|stdout| {
            stderr.seek(SeekFrom::Start(0))?;
            let mut stderr_bytes = Vec::new();
            stderr.read_to_end(&mut stderr_bytes)?;
            Ok(Output {
                status,
                stdout,
                stderr: stderr_bytes,
            })
        })
        .map_err(|error| format!("failed to read command output after trying to {label}: {error}"))
}

#[cfg(unix)]
fn terminate_provision_process_tree(child: &mut std::process::Child) -> io::Result<()> {
    const SIGKILL: i32 = 9;

    let process_group = i32::try_from(child.id())
        .map_err(|_| io::Error::other("provisioning process id exceeds the Unix pid range"))?;
    // SAFETY: the command is placed in a new process group whose id equals the child's pid before
    // it is spawned. A negative pid therefore targets only that dedicated provisioning group.
    if unsafe { send_unix_signal(-process_group, SIGKILL) } == 0 {
        Ok(())
    } else {
        let group_error = io::Error::last_os_error();
        child.kill().map_err(|child_error| {
            io::Error::other(format!(
                "failed to terminate provisioning process group: {group_error}; failed to terminate its leader: {child_error}"
            ))
        })
    }
}

#[cfg(not(unix))]
fn terminate_provision_process_tree(child: &mut std::process::Child) -> io::Result<()> {
    child.kill()
}

fn is_curl_environment_key(key: &OsStr) -> bool {
    matches!(
        key.to_string_lossy().to_ascii_uppercase().as_str(),
        "CURL_HOME"
    )
}

fn is_tar_environment_key(key: &OsStr) -> bool {
    matches!(
        key.to_string_lossy().to_ascii_uppercase().as_str(),
        "TAR_OPTIONS" | "TAPE" | "XZ_DEFAULTS" | "XZ_OPT"
    )
}

fn write_provisioned_component_identities(
    contract: &SdkContract,
    release_root: &Path,
    node_root: &Path,
) -> Result<(), String> {
    write_provisioned_identity(
        &release_root.join(".emsdk_version"),
        &format!("releases-{}-64bit", contract.emscripten_release),
        "installed Emscripten release",
    )?;
    write_provisioned_identity(
        &release_root
            .join("emscripten")
            .join("emscripten-version.txt"),
        &format!("\"{}\"", contract.emscripten_version),
        "installed Emscripten version",
    )?;
    write_provisioned_identity(
        &node_root.join(".emsdk_version"),
        &format!("node-{}-64bit", contract.node_version),
        "installed EMSDK Node.js identity",
    )
}

fn write_provisioned_identity(path: &Path, identity: &str, label: &str) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "{label} path has no parent directory during Emscripten provisioning: {}",
            path.display()
        )
    })?;
    require_real_canonical_directory(parent, &format!("{label} parent"))?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "{label} path must not be a symlink during Emscripten provisioning: {}",
                path.display()
            ));
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(format!(
                "{label} path must be a regular file during Emscripten provisioning: {}",
                path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "failed to inspect {label} path during Emscripten provisioning {}: {error}",
                path.display()
            ));
        }
    }
    fs::write(path, format!("{identity}\n"))
        .map_err(|error| format!("failed to write {label} {}: {error}", path.display()))
}

fn write_github_actions_emsdk_environment(
    root: &Path,
    contract: &SdkContract,
    host: &HostToolchainContract,
) -> Result<(), String> {
    let environment_file = env::var_os("GITHUB_ENV")
        .map(PathBuf::from)
        .ok_or_else(|| "provision-emsdk --github-actions requires GITHUB_ENV".to_owned())?;
    write_github_actions_emsdk_environment_file(
        &environment_file,
        root,
        contract,
        host,
        HostPlatform::current()?,
    )
}

fn write_github_actions_emsdk_environment_file(
    environment_file: &Path,
    root: &Path,
    contract: &SdkContract,
    host: &HostToolchainContract,
    platform: HostPlatform,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(environment_file).map_err(|error| {
        format!(
            "failed to inspect GitHub Actions environment file {}: {error}",
            environment_file.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "GitHub Actions environment file must be a regular non-symlink file: {}",
            environment_file.display()
        ));
    }
    let values = [
        ("EMSDK", root.to_path_buf()),
        ("EM_CONFIG", root.join(".emscripten")),
        ("EMSDK_PYTHON", host.python.executable_path(root)),
        (
            "EMSDK_NODE",
            root.join("node")
                .join(format!("{}_64bit", contract.node_version))
                .join("bin")
                .join(platform.node_name()),
        ),
    ];
    let mut payload = String::new();
    for (key, value) in &values {
        let value = value.to_str().ok_or_else(|| {
            format!(
                "GitHub Actions Emscripten environment value is not UTF-8 for {key}: {}",
                value.display()
            )
        })?;
        if value.contains(['\n', '\r']) {
            return Err(format!(
                "GitHub Actions Emscripten environment value contains a line break for {key}"
            ));
        }
        payload.push_str(key);
        payload.push('=');
        payload.push_str(value);
        payload.push('\n');
    }
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .open(environment_file)
        .map_err(|error| {
            format!(
                "failed to append GitHub Actions environment file {}: {error}",
                environment_file.display()
            )
        })?;
    let opened_metadata = file.metadata().map_err(|error| {
        format!(
            "failed to revalidate GitHub Actions environment file {}: {error}",
            environment_file.display()
        )
    })?;
    if !opened_metadata.file_type().is_file() {
        return Err(format!(
            "GitHub Actions environment file changed while opening: {}",
            environment_file.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if metadata.dev() != opened_metadata.dev() || metadata.ino() != opened_metadata.ino() {
            return Err(format!(
                "GitHub Actions environment file changed while opening: {}",
                environment_file.display()
            ));
        }
    }
    file.write_all(payload.as_bytes()).map_err(|error| {
        format!("failed to write GitHub Actions Emscripten environment values: {error}")
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "failed to flush GitHub Actions environment file {}: {error}",
            environment_file.display()
        )
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
            != Some(3)
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
            wasm_bindgen_version: required_string(table, "wasm_bindgen_version")?,
            hosts: parse_host_contracts(table)?,
        };
        require_identity(
            "Emscripten SDK contract SHA-256",
            &sha256_bytes(source.as_bytes()),
            SDK_CONTRACT_SHA256,
        )?;
        require_identity(
            "Emscripten provider ABI",
            &contract.provider_abi,
            PROVIDER_ABI,
        )?;
        require_identity(
            "Emscripten version",
            &contract.emscripten_version,
            EMSCRIPTEN_VERSION,
        )?;
        require_identity(
            "wasm-bindgen version",
            &contract.wasm_bindgen_version,
            WASM_BINDGEN_VERSION,
        )?;
        Ok(contract)
    }

    #[cfg(test)]
    fn canonical() -> Self {
        Self::parse(include_str!("../toolchains/emscripten-sdk.toml"))
            .expect("the embedded Emscripten SDK contract must remain canonical")
    }

    fn current_host(&self) -> Result<&HostToolchainContract, String> {
        let matches = self
            .hosts
            .iter()
            .filter(|host| host.os == env::consts::OS && host.arch == env::consts::ARCH)
            .collect::<Vec<_>>();
        let [host] = matches.as_slice() else {
            return Err(format!(
                "the pinned Emscripten SDK contract does not qualify host {}-{}",
                env::consts::OS,
                env::consts::ARCH
            ));
        };
        host.host_probe.validate_current(&host.os)?;
        Ok(host)
    }
}

impl HostProbeContract {
    fn validate_current(&self, host_os: &str) -> Result<(), String> {
        match self {
            Self::OsArch => Ok(()),
            Self::LinuxOsRelease { id, version_id } => {
                if host_os != "linux" || !cfg!(target_os = "linux") {
                    return Err(
                        "a Linux os-release host probe may only qualify a Linux host".to_owned(),
                    );
                }
                let os_release_path = canonical_regular_file_target(
                    Path::new("/etc/os-release"),
                    "Linux os-release identity",
                )?;
                require_root_owned_system_file(&os_release_path, "Linux os-release identity")?;
                let source = read_utf8_file(&os_release_path, "Linux os-release identity")?;
                validate_linux_os_release_identity(&source, id, version_id)
            }
        }
    }
}

fn parse_host_contracts(
    table: &toml::map::Map<String, toml::Value>,
) -> Result<Vec<HostToolchainContract>, String> {
    let hosts = table
        .get("hosts")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "Emscripten SDK contract hosts must be an array of tables".to_owned())?;
    if hosts.is_empty() {
        return Err("Emscripten SDK contract hosts must not be empty".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut coordinates = BTreeSet::new();
    hosts
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let table = value.as_table().ok_or_else(|| {
                format!("Emscripten SDK contract hosts[{index}] must be a table")
            })?;
            let fields = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
            let expected = HOST_CONTRACT_FIELDS
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            if fields != expected {
                return Err(format!(
                    "Emscripten SDK host contract fields do not match the closed schema: expected {expected:?}, found {fields:?}"
                ));
            }
            let host = HostToolchainContract {
                id: required_string(table, "id")?,
                os: required_string(table, "os")?,
                arch: required_string(table, "arch")?,
                release_url: required_string(table, "release_url")?,
                release_archive_sha256: required_string(table, "release_archive_sha256")?,
                release_tree_sha256: required_string(table, "release_tree_sha256")?,
                node_url: required_string(table, "node_url")?,
                node_archive_sha256: required_string(table, "node_archive_sha256")?,
                node_tree_sha256: required_string(table, "node_tree_sha256")?,
                host_probe: parse_host_probe(required_table(table, "host_probe")?)?,
                python: parse_python_runtime_contract(required_table(table, "python")?)?,
            };
            validate_host_contract(&host)?;
            if !ids.insert(host.id.clone()) {
                return Err(format!("duplicate Emscripten SDK host id: {}", host.id));
            }
            if !coordinates.insert((host.os.clone(), host.arch.clone())) {
                return Err(format!(
                    "duplicate Emscripten SDK host coordinate: {}-{}",
                    host.os, host.arch
                ));
            }
            Ok(host)
        })
        .collect()
}

fn validate_host_contract(host: &HostToolchainContract) -> Result<(), String> {
    for (label, digest) in [
        ("release_archive_sha256", &host.release_archive_sha256),
        ("release_tree_sha256", &host.release_tree_sha256),
        ("node_archive_sha256", &host.node_archive_sha256),
        ("node_tree_sha256", &host.node_tree_sha256),
    ] {
        require_lower_sha256(&format!("host {} {label}", host.id), digest)?;
    }
    for (label, url) in [
        ("release_url", &host.release_url),
        ("node_url", &host.node_url),
    ] {
        if !url.starts_with("https://") || url.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(format!("host {} {label} must be an HTTPS URL", host.id));
        }
    }
    match (&host.host_probe, host.os.as_str()) {
        (HostProbeContract::LinuxOsRelease { .. }, "linux")
        | (HostProbeContract::OsArch, "macos") => {}
        (HostProbeContract::OsArch, "linux") => {
            return Err(format!(
                "host {} must bind its Linux distribution through os-release",
                host.id
            ));
        }
        _ => {
            return Err(format!(
                "host {} has an unsupported host probe for {}",
                host.id, host.os
            ));
        }
    }
    validate_python_runtime_contract(&host.python, &host.id)
}

fn parse_host_probe(table: &toml::Table) -> Result<HostProbeContract, String> {
    let kind = required_string(table, "kind")?;
    match kind.as_str() {
        "os-arch" => {
            require_closed_table_fields(table, OS_ARCH_HOST_PROBE_FIELDS, "host probe")?;
            Ok(HostProbeContract::OsArch)
        }
        "linux-os-release" => {
            require_closed_table_fields(
                table,
                LINUX_OS_RELEASE_HOST_PROBE_FIELDS,
                "Linux os-release host probe",
            )?;
            Ok(HostProbeContract::LinuxOsRelease {
                id: required_string(table, "id")?,
                version_id: required_string(table, "version_id")?,
            })
        }
        _ => Err(format!(
            "unsupported Emscripten SDK host probe kind: {kind}"
        )),
    }
}

fn parse_python_runtime_contract(table: &toml::Table) -> Result<PythonRuntimeContract, String> {
    let kind = required_string(table, "kind")?;
    match kind.as_str() {
        "system" => {
            require_closed_table_fields(table, SYSTEM_PYTHON_FIELDS, "system Python contract")?;
            Ok(PythonRuntimeContract::System {
                executable: required_string(table, "executable")?,
                runtime_root: required_string(table, "runtime_root")?,
                minimum_version: required_string(table, "minimum_version")?,
            })
        }
        "archive" => {
            require_closed_table_fields(table, ARCHIVE_PYTHON_FIELDS, "archive Python contract")?;
            Ok(PythonRuntimeContract::Archive {
                version: required_string(table, "version")?,
                url: required_string(table, "url")?,
                archive_sha256: required_string(table, "archive_sha256")?,
                tree_sha256: required_string(table, "tree_sha256")?,
                executable: required_string(table, "executable")?,
            })
        }
        _ => Err(format!(
            "unsupported Emscripten SDK Python contract kind: {kind}"
        )),
    }
}

fn validate_python_runtime_contract(
    python: &PythonRuntimeContract,
    host_id: &str,
) -> Result<(), String> {
    match python {
        PythonRuntimeContract::System {
            executable,
            runtime_root,
            minimum_version,
        } => {
            validate_normal_absolute_utf8_path(executable, "system Python executable")?;
            validate_normal_absolute_utf8_path(runtime_root, "system Python runtime root")?;
            if runtime_root == "/" || !Path::new(executable).starts_with(runtime_root) {
                return Err(format!(
                    "host {host_id} system Python executable must be contained by its non-root runtime root"
                ));
            }
            parse_numeric_version(minimum_version, 2, "minimum system Python version")?;
        }
        PythonRuntimeContract::Archive {
            version,
            url,
            archive_sha256,
            tree_sha256,
            executable,
        } => {
            parse_numeric_version(version, 3, "archive Python version")?;
            if !url.starts_with("https://") || url.bytes().any(|byte| byte.is_ascii_whitespace()) {
                return Err(format!("host {host_id} Python URL must be an HTTPS URL"));
            }
            require_lower_sha256(
                &format!("host {host_id} Python archive SHA-256"),
                archive_sha256,
            )?;
            require_lower_sha256(&format!("host {host_id} Python tree SHA-256"), tree_sha256)?;
            validate_normal_relative_utf8_path(executable, "archive Python executable")?;
        }
    }
    Ok(())
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
    for (name, checksum) in [
        ("wasm-bindgen", WASM_BINDGEN_CHECKSUM),
        (
            "wasm-bindgen-cli-support",
            WASM_BINDGEN_CLI_SUPPORT_CHECKSUM,
        ),
    ] {
        let matches = packages
            .iter()
            .filter_map(toml::Value::as_table)
            .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some(name))
            .collect::<Vec<_>>();
        let [package] = matches.as_slice() else {
            return Err(format!(
                "Cargo.lock must contain exactly one {name} package; found {}",
                matches.len()
            ));
        };
        require_identity(
            &format!("Cargo.lock {name} version"),
            &required_string(package, "version")?,
            &contract.wasm_bindgen_version,
        )?;
        require_identity(
            &format!("Cargo.lock {name} source"),
            &required_string(package, "source")?,
            CRATES_IO_SOURCE,
        )?;
        require_identity(
            &format!("Cargo.lock {name} checksum"),
            &required_string(package, "checksum")?,
            checksum,
        )?;
    }
    Ok(())
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

    const fn node_name(self) -> &'static str {
        match self {
            Self::Linux | Self::MacOs => "node",
            Self::Windows => "node.exe",
        }
    }
}

fn collect_sdk_evidence(
    git: &Path,
    root: &Path,
    release_path: &Path,
    version_path: &Path,
    revision_path: &Path,
    em_config: &Path,
) -> Result<SdkEvidence, String> {
    Ok(SdkEvidence {
        emsdk_repository: run_git(git, root, &["remote", "get-url", "origin"])?,
        emsdk_revision: run_git(git, root, &["rev-parse", "--verify", "HEAD^{commit}"])?,
        emsdk_head_reference: run_git(git, root, &["rev-parse", "--abbrev-ref", "HEAD"])?,
        emsdk_status: run_git(
            git,
            root,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?,
        emscripten_release: read_identity_file(release_path, "installed Emscripten release")?,
        emscripten_version: read_identity_file(version_path, "installed Emscripten version")?,
        emscripten_revision: read_identity_file(revision_path, "installed Emscripten revision")?,
        activated_config: read_utf8_file(em_config, "activated Emscripten configuration")?,
    })
}

fn validate_sdk_evidence(
    contract: &SdkContract,
    host: &HostToolchainContract,
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
    let expected_config = expected_activated_config(contract, host, platform);
    if actual_config != expected_config {
        return Err(
            "the activated .emscripten configuration does not match the pinned SDK tool paths"
                .to_owned(),
        );
    }
    Ok(())
}

fn expected_activated_config(
    contract: &SdkContract,
    host: &HostToolchainContract,
    platform: HostPlatform,
) -> String {
    let executable_suffix = if platform == HostPlatform::Windows {
        ".exe"
    } else {
        ""
    };
    let mut config = format!(
        "import os\nemsdk_path = os.path.dirname(os.getenv('EM_CONFIG')).replace('\\\\', '/')\nNODE_JS = emsdk_path + '/node/{}_64bit/bin/node{}'\n",
        contract.node_version, executable_suffix
    );
    match &host.python {
        PythonRuntimeContract::System { executable, .. } => {
            config.push_str(&format!("PYTHON = '{executable}'\n"));
        }
        PythonRuntimeContract::Archive {
            version,
            executable,
            ..
        } => {
            config.push_str(&format!(
                "PYTHON = emsdk_path + '/python/{version}_64bit/{executable}'\n"
            ));
        }
    }
    config.push_str(
        "LLVM_ROOT = emsdk_path + '/upstream/bin'\nBINARYEN_ROOT = emsdk_path + '/upstream'\nEMSCRIPTEN_ROOT = emsdk_path + '/upstream/emscripten'\n",
    );
    config
}

fn archived_python_root(sdk_root: &Path, version: &str) -> PathBuf {
    sdk_root.join("python").join(format!("{version}_64bit"))
}

struct ResolvedPython {
    runtime_root: PathBuf,
    executable: PathBuf,
    archive_tree: Option<(PathBuf, String)>,
}

fn resolve_python(
    sdk_root: &Path,
    python_override: Option<&Path>,
    contract: &PythonRuntimeContract,
) -> Result<ResolvedPython, String> {
    let (python_root, expected, tree) = match contract {
        PythonRuntimeContract::System {
            executable,
            runtime_root,
            ..
        } => {
            let python_root = fs::canonicalize(runtime_root).map_err(|error| {
                format!("failed to resolve system Python runtime root {runtime_root}: {error}")
            })?;
            require_real_canonical_directory(&python_root, "system Python runtime root")?;
            require_root_owned_system_directory(&python_root, "system Python runtime root")?;
            let expected = canonical_regular_file_target(
                Path::new(executable),
                "qualified system Python interpreter",
            )?;
            require_path_within(
                &python_root,
                &expected,
                "qualified system Python interpreter",
            )?;
            require_root_owned_system_file(&expected, "qualified system Python interpreter")?;
            (python_root, expected, None)
        }
        PythonRuntimeContract::Archive {
            version,
            tree_sha256,
            executable,
            ..
        } => {
            let python_root = canonical_directory_within(
                sdk_root,
                &archived_python_root(sdk_root, version),
                "installed EMSDK Python runtime",
            )?;
            let version_path = canonical_regular_file_within(
                &python_root,
                &python_root.join(".emsdk_version"),
                "installed EMSDK Python identity",
            )?;
            require_identity(
                "installed EMSDK Python identity",
                &read_identity_file(&version_path, "installed EMSDK Python identity")?,
                &format!("python-{version}-64bit"),
            )?;
            let expected = canonical_regular_file_within(
                &python_root,
                &python_root.join(executable),
                "pinned EMSDK Python interpreter",
            )?;
            (
                python_root.clone(),
                expected,
                Some((python_root, tree_sha256.clone())),
            )
        }
    };
    if let Some(override_path) = python_override {
        let override_path = canonical_regular_file_target(override_path, "EMSDK_PYTHON override")?;
        if override_path != expected {
            return Err(format!(
                "EMSDK_PYTHON must resolve to the interpreter selected by the qualified host contract: expected {}, found {}",
                expected.display(),
                override_path.display()
            ));
        }
    }
    Ok(ResolvedPython {
        runtime_root: python_root,
        executable: expected,
        archive_tree: tree,
    })
}

fn resolve_node(
    node_root: &Path,
    node_override: Option<&Path>,
    platform: HostPlatform,
) -> Result<PathBuf, String> {
    let expected = canonical_regular_file_within(
        node_root,
        &node_root.join("bin").join(platform.node_name()),
        "pinned EMSDK Node.js runtime",
    )?;
    if let Some(override_path) = node_override {
        let override_path = canonical_regular_file(override_path, "EMSDK_NODE override")?;
        if override_path != expected {
            return Err(format!(
                "EMSDK_NODE may only assert the runtime inside the qualified EMSDK: expected {}, found {}",
                expected.display(),
                override_path.display()
            ));
        }
    }
    Ok(expected)
}

fn resolve_compiler(
    sdk_root: &Path,
    emscripten_root: &Path,
    platform: HostPlatform,
) -> Result<PathBuf, String> {
    canonical_regular_file_within(
        sdk_root,
        &emscripten_root.join(platform.compiler_name()),
        "pinned Emscripten compiler",
    )
}

fn resolve_compiler_driver(sdk_root: &Path, emscripten_root: &Path) -> Result<PathBuf, String> {
    canonical_regular_file_within(
        sdk_root,
        &emscripten_root.join("emcc.py"),
        "pinned Emscripten compiler driver",
    )
}

fn resolve_wasm_opt(
    sdk_root: &Path,
    upstream_root: &Path,
    platform: HostPlatform,
) -> Result<PathBuf, String> {
    let name = if platform == HostPlatform::Windows {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    };
    canonical_regular_file_within(
        sdk_root,
        &upstream_root.join("bin").join(name),
        "pinned Binaryen wasm-opt",
    )
}

fn verify_python_version(
    python: &Path,
    requirement: PythonVersionRequirement<'_>,
) -> Result<(), String> {
    let mut command = Command::new(python);
    remove_process_injection_environment(&mut command);
    for (key, _) in env::vars_os() {
        if is_python_environment_key(&key) {
            command.env_remove(key);
        }
    }
    let output = command
        .arg("-I")
        .arg("-B")
        .arg("-X")
        .arg("utf8")
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "failed to execute qualified EMSDK Python {}: {error}",
                python.display()
            )
        })?;
    if !output.status.success() {
        let version = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(format!(
            "qualified EMSDK Python failed --version with status {}: {}",
            output.status,
            version.trim()
        ));
    }
    validate_python_version_output(&output.stdout, &output.stderr, requirement)
}

fn verify_python_runtime(
    python: &Path,
    python_root: &Path,
    requirement: PythonVersionRequirement<'_>,
) -> Result<(), String> {
    verify_python_version(python, requirement)?;
    let mut command = Command::new(python);
    remove_process_injection_environment(&mut command);
    for (key, _) in env::vars_os() {
        if is_python_environment_key(&key) {
            command.env_remove(key);
        }
    }
    let output = command
        .args([
            "-I",
            "-B",
            "-X",
            "utf8",
            "-c",
            "import os,sys,sysconfig,_sha2; print('\\n'.join(os.path.realpath(path) for path in (sys.prefix,sys.base_prefix,sysconfig.get_path('stdlib'),_sha2.__file__)))",
        ])
        .output()
        .map_err(|error| {
            format!(
                "failed to inspect qualified EMSDK Python runtime {}: {error}",
                python.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "qualified EMSDK Python runtime inspection failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let output = String::from_utf8(output.stdout)
        .map_err(|error| format!("qualified EMSDK Python runtime printed non-UTF-8: {error}"))?;
    let paths = output.lines().collect::<Vec<_>>();
    if paths.len() != 4 || paths.iter().any(|path| path.is_empty()) {
        return Err(
            "qualified EMSDK Python runtime did not report the expected interpreter paths"
                .to_owned(),
        );
    }
    for path in paths {
        let path = fs::canonicalize(path).map_err(|error| {
            format!("qualified EMSDK Python runtime reported an invalid path {path:?}: {error}")
        })?;
        require_path_within(python_root, &path, "qualified EMSDK Python runtime")?;
    }
    Ok(())
}

fn validate_python_version_output(
    stdout: &[u8],
    stderr: &[u8],
    requirement: PythonVersionRequirement<'_>,
) -> Result<(), String> {
    let output = match (stdout.is_empty(), stderr.is_empty()) {
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => {
            return Err("wasm-provider Python --version printed no output".to_owned());
        }
        (false, false) => {
            return Err(
                "wasm-provider Python --version wrote to both stdout and stderr".to_owned(),
            );
        }
    };
    let line = output
        .strip_suffix(b"\r\n")
        .or_else(|| output.strip_suffix(b"\n"))
        .unwrap_or(output);
    let line = std::str::from_utf8(line)
        .map_err(|error| format!("wasm-provider Python --version was not UTF-8: {error}"))?;
    let actual = line.strip_prefix("Python ").ok_or_else(|| {
        format!(
            "wasm-provider Python --version has an invalid form: {:?}",
            String::from_utf8_lossy(output)
        )
    })?;
    match requirement {
        PythonVersionRequirement::Exact(expected) if actual == expected => Ok(()),
        PythonVersionRequirement::Exact(expected) => Err(format!(
            "wasm-provider requires exact Python {expected}; found {actual:?}"
        )),
        PythonVersionRequirement::Minimum(minimum) => {
            let actual_parts = parse_numeric_version(actual, 3, "system Python version")?;
            let minimum_parts = parse_numeric_version(minimum, 2, "minimum system Python version")?;
            if numeric_version_at_least(&actual_parts, &minimum_parts) {
                Ok(())
            } else {
                Err(format!(
                    "wasm-provider requires Python >= {minimum}; found {actual}"
                ))
            }
        }
    }
}

fn verify_node_version(node: &Path, expected_version: &str) -> Result<(), String> {
    let mut command = Command::new(node);
    remove_process_injection_environment(&mut command);
    for (key, _) in env::vars_os() {
        if is_node_environment_key(&key) {
            command.env_remove(key);
        }
    }
    let output = command.arg("--version").output().map_err(|error| {
        format!(
            "failed to execute qualified EMSDK Node.js {}: {error}",
            node.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "qualified EMSDK Node.js failed --version with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let version = String::from_utf8(output.stdout)
        .map_err(|error| format!("qualified EMSDK Node.js printed non-UTF-8: {error}"))?;
    require_identity(
        "qualified EMSDK Node.js version",
        version.trim(),
        &format!("v{expected_version}"),
    )
}

fn verify_compiler_version_unchecked(
    sdk: &QualifiedEmscriptenSdk,
    expected_version: &str,
) -> Result<(), String> {
    let output = sdk
        .unchecked_emcc_command()
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "failed to execute qualified Emscripten compiler {}: {error}",
                sdk.compiler.display()
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

fn validate_ambient_emscripten_environment() -> Result<(), String> {
    let mut disallowed = env::vars_os()
        .filter(|(_, value)| !value.is_empty())
        .filter_map(|(key, _)| {
            is_disallowed_ambient_emscripten_environment_key(&key)
                .then(|| key.to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    disallowed.sort();
    disallowed.dedup();
    if disallowed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "ambient Emscripten environment overrides are not qualified; unset: {}",
            disallowed.join(", ")
        ))
    }
}

fn is_disallowed_ambient_emscripten_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    if matches!(
        key.as_str(),
        "EMSDK"
            | "EM_CONFIG"
            | "EMSDK_PYTHON"
            | "EMSDK_NODE"
            | "EMSDK_VERSION"
            | "EMSDK_REVISION"
            | "EMSDK_QUIET"
            | "EMSDK_POWERSHELL"
            | "EMSDK_CSH"
            | "EMSDK_CMD"
            | "EMSDK_BASH"
            | "EMSDK_FISH"
            | "EMSDK_NUM_CORES"
            | "EMSDK_NOTTY"
            | "EMSDK_KEEP_DOWNLOADS"
    ) {
        return false;
    }
    if matches!(
        key.as_str(),
        "NODE_OPTIONS" | "NODE_PATH" | "PYTHONHOME" | "PYTHONPATH"
    ) {
        return true;
    }
    is_emscripten_environment_key(OsStr::new(&key))
}

fn is_qualified_tool_environment_key(key: &OsStr) -> bool {
    is_emscripten_environment_key(key)
        || is_node_environment_key(key)
        || is_python_environment_key(key)
        || is_external_compiler_environment_key(key)
        || is_process_injection_environment_key(key)
}

fn is_external_compiler_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("CFLAGS_")
        || key.ends_with("_CFLAGS")
        || key.starts_with("BINDGEN_EXTRA_CLANG_ARGS_")
        || matches!(
            key.as_str(),
            "CFLAGS"
                | "CPPFLAGS"
                | "CL"
                | "CPATH"
                | "C_INCLUDE_PATH"
                | "CPLUS_INCLUDE_PATH"
                | "OBJC_INCLUDE_PATH"
                | "SDKROOT"
                | "INCLUDE"
                | "BINDGEN_EXTRA_CLANG_ARGS"
                | "CRATE_CC_NO_DEFAULTS"
                | "CC_SHELL_ESCAPED_FLAGS"
                | "CC_FORCE_DISABLE"
        )
}

fn is_emscripten_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("EM_")
        || key.starts_with("EMCC_")
        || key.starts_with("_EMCC_")
        || key.starts_with("EMSDK_")
        || key.starts_with("EMMAKEN_")
        || key.starts_with("EMBUILDER_")
        || key.starts_with("EMSCRIPTEN_")
        || matches!(
            key.as_str(),
            "EMSDK"
                | "EMSCRIPTEN"
                | "EMPROFILE"
                | "LLVM"
                | "BINARYEN"
                | "NODE"
                | "LLVM_ADD_VERSION"
                | "CLANG_ADD_VERSION"
        )
}

fn is_node_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key == "NODE" || key.starts_with("NODE_")
}

fn is_python_environment_key(key: &OsStr) -> bool {
    let key = key.to_string_lossy().to_ascii_uppercase();
    key.starts_with("PYTHON") || key == "_PYTHON_SYSCONFIGDATA_NAME"
}

fn configured_provision_git_command(git: &Path) -> Command {
    let mut command = configured_git_command(git);
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .args(["-c", "credential.interactive=never"])
        .args(["-c", "http.lowSpeedLimit=1"])
        .args(["-c", "http.lowSpeedTime=60"]);
    command
}

fn run_git(git: &Path, root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = configured_provision_git_command(git);
    command.arg("-C").arg(root).args(args);
    let output = command_output_with_timeout(
        &mut command,
        &format!("execute git for EMSDK {}", root.display()),
        PROVISION_COMMAND_TIMEOUT,
    )?;
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

pub(crate) fn canonical_regular_file(path: &Path, label: &str) -> Result<PathBuf, String> {
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

fn canonical_regular_file_target(path: &Path, label: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))?;
    let metadata = fs::symlink_metadata(&canonical)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", canonical.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must resolve to a regular file: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
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

fn require_real_canonical_directory(path: &Path, label: &str) -> Result<(), String> {
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
    if canonical != path {
        return Err(format!(
            "{label} changed after qualification: expected {}, found {}",
            path.display(),
            canonical.display()
        ));
    }
    Ok(())
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

fn required_table<'a>(table: &'a toml::Table, key: &str) -> Result<&'a toml::Table, String> {
    table
        .get(key)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("Emscripten SDK contract field `{key}` must be a table"))
}

fn require_closed_table_fields(
    table: &toml::Table,
    expected_fields: &[&str],
    label: &str,
) -> Result<(), String> {
    let fields = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected_fields.iter().copied().collect::<BTreeSet<_>>();
    if fields == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} fields do not match the closed schema: expected {expected:?}, found {fields:?}"
        ))
    }
}

fn parse_numeric_version(
    version: &str,
    component_count: usize,
    label: &str,
) -> Result<Vec<u64>, String> {
    let components = version.split('.').collect::<Vec<_>>();
    if components.len() != component_count {
        return Err(format!(
            "{label} must contain exactly {component_count} numeric components: {version:?}"
        ));
    }
    components
        .into_iter()
        .map(|component| {
            if component.is_empty()
                || !component.bytes().all(|byte| byte.is_ascii_digit())
                || (component.len() > 1 && component.starts_with('0'))
            {
                return Err(format!(
                    "{label} contains a non-canonical numeric component: {version:?}"
                ));
            }
            component
                .parse::<u64>()
                .map_err(|error| format!("{label} contains an invalid component: {error}"))
        })
        .collect()
}

fn numeric_version_at_least(actual: &[u64], minimum: &[u64]) -> bool {
    let width = actual.len().max(minimum.len());
    (0..width)
        .map(|index| actual.get(index).copied().unwrap_or(0))
        .cmp((0..width).map(|index| minimum.get(index).copied().unwrap_or(0)))
        .is_ge()
}

fn validate_linux_os_release_identity(
    source: &str,
    expected_id: &str,
    expected_version_id: &str,
) -> Result<(), String> {
    let source = normalize_newlines(source)?;
    let mut values = BTreeMap::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if !matches!(key, "ID" | "VERSION_ID") {
            continue;
        }
        let value = parse_os_release_identity_value(raw_value, key)?;
        if values.insert(key, value).is_some() {
            return Err(format!("Linux os-release contains duplicate {key} fields"));
        }
    }
    let id = values
        .get("ID")
        .ok_or_else(|| "Linux os-release does not contain ID".to_owned())?;
    let version_id = values
        .get("VERSION_ID")
        .ok_or_else(|| "Linux os-release does not contain VERSION_ID".to_owned())?;
    if id != expected_id || version_id != expected_version_id {
        return Err(format!(
            "the pinned Emscripten SDK contract requires Linux {expected_id} {expected_version_id}; found {id} {version_id}"
        ));
    }
    Ok(())
}

fn parse_os_release_identity_value(value: &str, key: &str) -> Result<String, String> {
    let value = if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    };
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "Linux os-release {key} is not a canonical identity value: {value:?}"
        ));
    }
    Ok(value.to_owned())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    read_regular_file(path, "qualified SDK input").map(|bytes| sha256_bytes(&bytes))
}

fn qualify_tree(
    label: &'static str,
    root: PathBuf,
    expected_sha256: &str,
) -> Result<QualifiedTree, String> {
    require_lower_sha256(&format!("{label} SHA-256"), expected_sha256)?;
    require_real_canonical_directory(&root, label)?;
    let actual_sha256 = qualified_tree_sha256(&root, label)?;
    require_identity(&format!("{label} SHA-256"), &actual_sha256, expected_sha256)?;
    Ok(QualifiedTree {
        label,
        root,
        sha256: actual_sha256,
    })
}

fn validate_qualified_trees(trees: &[QualifiedTree]) -> Result<(), String> {
    for tree in trees {
        require_real_canonical_directory(&tree.root, tree.label)?;
        let actual_sha256 = qualified_tree_sha256(&tree.root, tree.label)?;
        require_identity(
            &format!("{} SHA-256", tree.label),
            &actual_sha256,
            &tree.sha256,
        )?;
    }
    Ok(())
}

#[derive(Debug)]
enum TreeEntry {
    Directory,
    File {
        executable: bool,
        size: u64,
        sha256: [u8; 32],
    },
    Symlink {
        target: String,
    },
}

fn qualified_tree_sha256(root: &Path, label: &str) -> Result<String, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", root.display()))?;
    require_real_canonical_directory(&root, label)?;
    let mut entries = BTreeMap::new();
    collect_tree_entries(&root, &root, &mut entries, label)?;
    let mut seen_case_folded_paths = BTreeSet::new();
    let mut hasher = Sha256::new();
    hasher.update(TREE_DIGEST_DOMAIN);
    for (relative, entry) in entries {
        let case_folded = relative.to_ascii_lowercase();
        if !seen_case_folded_paths.insert(case_folded) {
            return Err(format!(
                "{label} contains paths that collide under ASCII case folding: {relative}"
            ));
        }
        match entry {
            TreeEntry::Directory => {
                hasher.update(b"D");
                hash_length_prefixed(&mut hasher, relative.as_bytes())?;
            }
            TreeEntry::File {
                executable,
                size,
                sha256,
            } => {
                hasher.update(b"F");
                hash_length_prefixed(&mut hasher, relative.as_bytes())?;
                hasher.update([u8::from(executable)]);
                hasher.update(size.to_be_bytes());
                hasher.update(sha256);
            }
            TreeEntry::Symlink { target } => {
                hasher.update(b"L");
                hash_length_prefixed(&mut hasher, relative.as_bytes())?;
                hash_length_prefixed(&mut hasher, target.as_bytes())?;
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_tree_entries(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<String, TreeEntry>,
    label: &str,
) -> Result<(), String> {
    let mut children = fs::read_dir(directory)
        .map_err(|error| {
            format!(
                "failed to enumerate {label} {}: {error}",
                directory.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "failed to enumerate {label} {}: {error}",
                directory.display()
            )
        })?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let path = child.path();
        let relative = canonical_tree_relative_path(root, &path, label)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "failed to inspect {label} entry {}: {error}",
                path.display()
            )
        })?;
        let entry = if metadata.file_type().is_dir() {
            TreeEntry::Directory
        } else if metadata.file_type().is_file() {
            require_single_tree_link(&metadata, &path, label)?;
            let (size, sha256) = tree_file_sha256(&path, &metadata, label)?;
            TreeEntry::File {
                executable: tree_file_is_executable(&metadata),
                size,
                sha256,
            }
        } else if metadata.file_type().is_symlink() {
            require_single_tree_link(&metadata, &path, label)?;
            TreeEntry::Symlink {
                target: validated_tree_symlink_target(root, &path, label)?,
            }
        } else {
            return Err(format!(
                "{label} contains an unsupported filesystem object: {}",
                path.display()
            ));
        };
        if entries.insert(relative, entry).is_some() {
            return Err(format!(
                "{label} contains duplicate tree entries at {}",
                path.display()
            ));
        }
        if metadata.file_type().is_dir() {
            collect_tree_entries(root, &path, entries, label)?;
        }
    }
    Ok(())
}

fn canonical_tree_relative_path(root: &Path, path: &Path, label: &str) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        format!(
            "{label} entry escapes its component root {}: {}",
            root.display(),
            path.display()
        )
    })?;
    let relative = relative
        .to_str()
        .ok_or_else(|| format!("{label} contains a non-UTF-8 path: {}", relative.display()))?;
    validate_normal_relative_utf8_path(relative, label)?;
    Ok(relative.replace(std::path::MAIN_SEPARATOR, "/"))
}

fn validate_normal_relative_utf8_path(path: &str, label: &str) -> Result<(), String> {
    if path.is_empty()
        || path.contains('\\')
        || path.bytes().any(|byte| byte == 0)
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(format!(
            "{label} must be a normalized relative path: {path:?}"
        ))
    } else {
        Ok(())
    }
}

fn validate_normal_absolute_utf8_path(path: &str, label: &str) -> Result<(), String> {
    let mut components = Path::new(path).components();
    let has_root = matches!(components.next(), Some(Component::RootDir));
    if !has_root
        || path.contains(['\\', '\'', '"', '\n', '\r'])
        || path.bytes().any(|byte| byte == 0)
        || components.any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(format!(
            "{label} must be a normalized absolute Unix path: {path:?}"
        ))
    } else {
        Ok(())
    }
}

fn tree_file_sha256(
    path: &Path,
    observed: &fs::Metadata,
    label: &str,
) -> Result<(u64, [u8; 32]), String> {
    let mut file = File::open(path)
        .map_err(|error| format!("failed to open {label} file {}: {error}", path.display()))?;
    let before = file
        .metadata()
        .map_err(|error| format!("failed to inspect {label} file {}: {error}", path.display()))?;
    if !same_tree_file_metadata(observed, &before) {
        return Err(format!(
            "{label} entry changed while hashing: {}",
            path.display()
        ));
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {label} file {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let after = file.metadata().map_err(|error| {
        format!(
            "failed to revalidate {label} file {}: {error}",
            path.display()
        )
    })?;
    if !same_tree_file_metadata(&before, &after) {
        return Err(format!(
            "{label} file changed while hashing: {}",
            path.display()
        ));
    }
    let rebound = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "failed to rebind {label} file path {}: {error}",
            path.display()
        )
    })?;
    if !same_tree_file_metadata(&after, &rebound) {
        return Err(format!(
            "{label} file path changed while hashing: {}",
            path.display()
        ));
    }
    Ok((after.len(), hasher.finalize().into()))
}

#[cfg(unix)]
fn require_single_tree_link(
    metadata: &fs::Metadata,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;

    if metadata.nlink() == 1 {
        Ok(())
    } else {
        Err(format!(
            "{label} files and symlinks must not have hard links: {}",
            path.display()
        ))
    }
}

#[cfg(not(unix))]
fn require_single_tree_link(
    _metadata: &fs::Metadata,
    _path: &Path,
    _label: &str,
) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn same_tree_file_metadata(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.file_type().is_file()
        && right.file_type().is_file()
        && left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mode() == right.mode()
        && left.nlink() == right.nlink()
        && left.uid() == right.uid()
        && left.gid() == right.gid()
        && left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(not(unix))]
fn same_tree_file_metadata(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.file_type().is_file()
        && right.file_type().is_file()
        && left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
}

#[cfg(unix)]
fn tree_file_is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn tree_file_is_executable(_metadata: &fs::Metadata) -> bool {
    false
}

fn validated_tree_symlink_target(root: &Path, path: &Path, label: &str) -> Result<String, String> {
    let target = fs::read_link(path)
        .map_err(|error| format!("failed to read {label} symlink {}: {error}", path.display()))?;
    if target.is_absolute() {
        return Err(format!(
            "{label} symlink target must be relative: {} -> {}",
            path.display(),
            target.display()
        ));
    }
    let target_text = target.to_str().ok_or_else(|| {
        format!(
            "{label} symlink target is not UTF-8: {} -> {}",
            path.display(),
            target.display()
        )
    })?;
    if target_text.contains('\\') || target_text.bytes().any(|byte| byte == 0) {
        return Err(format!(
            "{label} symlink target must be a normalized UTF-8 path: {} -> {}",
            path.display(),
            target.display()
        ));
    }
    let resolved = fs::canonicalize(path).map_err(|error| {
        format!(
            "failed to resolve {label} symlink {}: {error}",
            path.display()
        )
    })?;
    require_path_within(root, &resolved, label)?;
    Ok(target_text.replace(std::path::MAIN_SEPARATOR, "/"))
}

fn hash_length_prefixed(hasher: &mut Sha256, value: &[u8]) -> Result<(), String> {
    let length = u32::try_from(value.len())
        .map_err(|_| "qualified Emscripten tree entry exceeds u32 length".to_owned())?;
    hasher.update(length.to_be_bytes());
    hasher.update(value);
    Ok(())
}

fn require_lower_sha256(label: &str, digest: &str) -> Result<(), String> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "{label} must be exactly 64 lowercase hexadecimal characters"
        ))
    }
}

fn secure_private_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} must be a real directory: {}",
            path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!(
                "failed to restrict {label} {} to mode 0700: {error}",
                path.display()
            )
        })?;
        let mode = fs::symlink_metadata(path)
            .map_err(|error| format!("failed to revalidate {label} {}: {error}", path.display()))?
            .permissions()
            .mode()
            & 0o777;
        if mode != 0o700 {
            return Err(format!(
                "{label} must retain mode 0700; found {mode:04o} at {}",
                path.display()
            ));
        }
        if fs::symlink_metadata(path)
            .map_err(|error| {
                format!(
                    "failed to inspect private {label} owner {}: {error}",
                    path.display()
                )
            })?
            .uid()
            != current_process_uid()?
        {
            return Err(format!(
                "{label} must be owned by the current user: {}",
                path.display()
            ));
        }
        let canonical = fs::canonicalize(path).map_err(|error| {
            format!(
                "failed to resolve private {label} {}: {error}",
                path.display()
            )
        })?;
        require_trusted_directory_ancestry(&canonical, label)?;
    }
    Ok(())
}

fn require_private_emsdk_root(path: &Path, label: &str) -> Result<(), String> {
    require_real_canonical_directory(path, label)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
        if metadata.uid() != current_process_uid()? {
            return Err(format!(
                "{label} must be owned by the current user: {}",
                path.display()
            ));
        }
        let mode = metadata.permissions().mode() & 0o7777;
        if mode != 0o700 {
            return Err(format!(
                "{label} must have mode 0700; found {mode:04o} at {}",
                path.display()
            ));
        }
        require_trusted_directory_ancestry(path, label)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(format!("{label} is not qualified on this host"))
    }
}

fn require_root_owned_system_directory(path: &Path, label: &str) -> Result<(), String> {
    require_real_canonical_directory(path, label)?;
    #[cfg(unix)]
    {
        require_root_owned_directory_ancestry(path, label)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(format!("{label} is not qualified on this host"))
    }
}

fn require_root_owned_system_file(path: &Path, label: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(format!(
                "{label} must be a regular non-symlink system file: {}",
                path.display()
            ));
        }
        if metadata.uid() != 0 || metadata.permissions().mode() & 0o022 != 0 {
            return Err(format!(
                "{label} must be root-owned and not group/other writable: {}",
                path.display()
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| format!("{label} has no parent directory: {}", path.display()))?;
        require_root_owned_directory_ancestry(parent, label)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(format!("{label} is not qualified on this host"))
    }
}

#[cfg(unix)]
fn current_process_uid() -> Result<u32, String> {
    use std::os::unix::fs::MetadataExt;

    static UID: OnceLock<Result<u32, String>> = OnceLock::new();
    UID.get_or_init(|| {
        let temporary = tempfile::tempdir()
            .map_err(|error| format!("failed to determine the current user identity: {error}"))?;
        fs::metadata(temporary.path())
            .map(|metadata| metadata.uid())
            .map_err(|error| format!("failed to inspect the current user identity: {error}"))
    })
    .clone()
}

#[cfg(unix)]
fn require_trusted_directory_ancestry(path: &Path, label: &str) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let current_uid = current_process_uid()?;
    let mut current = path;
    loop {
        let metadata = fs::symlink_metadata(current).map_err(|error| {
            format!(
                "failed to inspect {label} ancestry {}: {error}",
                current.display()
            )
        })?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "{label} ancestry must contain only real directories: {}",
                current.display()
            ));
        }
        if metadata.uid() != current_uid && metadata.uid() != 0 {
            return Err(format!(
                "{label} ancestry has an untrusted owner at {}",
                current.display()
            ));
        }
        let mode = metadata.permissions().mode() & 0o7777;
        if mode & 0o022 != 0 && mode & 0o1000 == 0 {
            return Err(format!(
                "{label} ancestry is writable without sticky protection at {}",
                current.display()
            ));
        }
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent;
    }
    Ok(())
}

#[cfg(unix)]
fn require_root_owned_directory_ancestry(path: &Path, label: &str) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let mut current = path;
    loop {
        let metadata = fs::symlink_metadata(current).map_err(|error| {
            format!(
                "failed to inspect {label} system ancestry {}: {error}",
                current.display()
            )
        })?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "{label} system ancestry must contain only real directories: {}",
                current.display()
            ));
        }
        if metadata.uid() != 0 || metadata.permissions().mode() & 0o022 != 0 {
            return Err(format!(
                "{label} system ancestry must be root-owned and not group/other writable: {}",
                current.display()
            ));
        }
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent;
    }
    Ok(())
}

fn validate_scratch_directory(
    scratch: &tempfile::TempDir,
    cache: &Path,
    ports: &Path,
) -> Result<(), String> {
    let root = fs::canonicalize(scratch.path()).map_err(|error| {
        format!(
            "failed to resolve private Emscripten scratch directory {}: {error}",
            scratch.path().display()
        )
    })?;
    secure_private_directory(&root, "private Emscripten scratch directory")?;
    for (label, path) in [
        ("private Emscripten cache", cache),
        ("private Emscripten ports cache", ports),
    ] {
        secure_private_directory(path, label)?;
        let canonical = fs::canonicalize(path)
            .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))?;
        require_path_within(&root, &canonical, label)?;
    }
    Ok(())
}

fn qualify_files(paths: Vec<PathBuf>) -> Result<Vec<QualifiedFile>, String> {
    let mut seen = BTreeSet::new();
    paths
        .into_iter()
        .map(|path| {
            let canonical = canonical_regular_file(&path, "qualified SDK input")?;
            if !seen.insert(canonical.clone()) {
                return Err(format!(
                    "qualified SDK input is listed more than once: {}",
                    canonical.display()
                ));
            }
            let sha256 = sha256_file(&canonical)?;
            Ok(QualifiedFile {
                path: canonical,
                sha256,
            })
        })
        .collect()
}

fn validate_qualified_files(files: &[QualifiedFile]) -> Result<(), String> {
    for file in files {
        let path = canonical_regular_file(&file.path, "qualified SDK input")?;
        if path != file.path {
            return Err(format!(
                "qualified SDK input path changed: expected {}, found {}",
                file.path.display(),
                path.display()
            ));
        }
        let sha256 = sha256_file(&path)?;
        if sha256 != file.sha256 {
            return Err(format!(
                "qualified SDK input changed after qualification: {}",
                path.display()
            ));
        }
    }
    Ok(())
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

    const TEST_BUNDLED_PYTHON_VERSION: &str = "3.13.3";
    const TEST_MINIMUM_SYSTEM_PYTHON_VERSION: &str = "3.10";

    fn command_environment<'a>(command: &'a Command, name: &str) -> Option<Option<&'a OsStr>> {
        command
            .get_envs()
            .find_map(|(key, value)| (key == OsStr::new(name)).then_some(value))
    }

    fn command_arguments(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    fn valid_evidence(
        contract: &SdkContract,
        host: &HostToolchainContract,
        platform: HostPlatform,
    ) -> SdkEvidence {
        SdkEvidence {
            emsdk_repository: contract.emsdk_repository.clone(),
            emsdk_revision: contract.emsdk_revision.clone(),
            emsdk_head_reference: "HEAD".to_owned(),
            emsdk_status: String::new(),
            emscripten_release: format!("releases-{}-64bit", contract.emscripten_release),
            emscripten_version: format!("\"{}\"", contract.emscripten_version),
            emscripten_revision: contract.emscripten_revision.clone(),
            activated_config: expected_activated_config(contract, host, platform),
        }
    }

    #[test]
    fn archive_extraction_uses_only_the_explicit_linux_xz() {
        let command = configured_archive_extract_command(
            Path::new("/usr/bin/tar"),
            Some(Path::new("/usr/bin/xz")),
            Path::new("/private/archive.tar.xz"),
            Path::new("/private/destination"),
        );
        assert_eq!(
            command_arguments(&command),
            [
                "--use-compress-program",
                "/usr/bin/xz",
                "-xf",
                "/private/archive.tar.xz",
                "-C",
                "/private/destination",
                "--strip-components=1",
            ]
        );
        assert_eq!(
            command_environment(&command, "PATH"),
            Some(Some(OsStr::new(TRUSTED_PROVISION_PATH)))
        );
        for key in ["TAR_OPTIONS", "TAPE", "XZ_DEFAULTS", "XZ_OPT"] {
            assert!(is_tar_environment_key(OsStr::new(key)), "{key}");
        }

        let macos_command = configured_archive_extract_command(
            Path::new("/usr/bin/bsdtar"),
            None,
            Path::new("/private/archive.tar.xz"),
            Path::new("/private/destination"),
        );
        assert!(
            !command_arguments(&macos_command)
                .iter()
                .any(|argument| argument == "--use-compress-program")
        );
    }

    #[test]
    fn archive_download_has_finite_retries_and_deadlines() {
        let command = configured_archive_download_command(
            Path::new("/usr/bin/curl"),
            Path::new("/private/archive"),
            "https://example.invalid/archive",
        );
        let arguments = command_arguments(&command);
        for required in [
            ["--connect-timeout", "30"],
            ["--max-time", "900"],
            ["--retry", "4"],
            ["--retry-delay", "5"],
            ["--retry-max-time", "1200"],
        ] {
            assert!(
                arguments
                    .windows(2)
                    .any(|window| window == required.as_slice()),
                "missing curl argument pair {required:?}"
            );
        }
        assert!(
            arguments
                .iter()
                .any(|argument| argument == "--retry-all-errors")
        );
    }

    #[test]
    fn provisioning_git_is_noninteractive_and_has_a_stall_deadline() {
        let command = configured_provision_git_command(Path::new("/usr/bin/git"));
        assert_eq!(
            command_environment(&command, "GIT_TERMINAL_PROMPT"),
            Some(Some(OsStr::new("0")))
        );
        assert_eq!(
            command_environment(&command, "GCM_INTERACTIVE"),
            Some(Some(OsStr::new("never")))
        );
        let arguments = command_arguments(&command);
        for required in [
            ["-c", "credential.interactive=never"],
            ["-c", "http.lowSpeedLimit=1"],
            ["-c", "http.lowSpeedTime=60"],
        ] {
            assert!(
                arguments
                    .windows(2)
                    .any(|window| window == required.as_slice()),
                "missing Git argument pair {required:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn provisioning_command_timeout_terminates_and_reaps_the_child() {
        let sleep = if cfg!(target_os = "macos") {
            "/bin/sleep"
        } else {
            "/usr/bin/sleep"
        };
        let mut command = Command::new(sleep);
        command.arg("30");
        let error = command_output_with_timeout(
            &mut command,
            "exercise the provisioning timeout",
            Duration::from_millis(10),
        )
        .unwrap_err();
        assert!(error.contains("timed out after 10ms"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess process-group helper"]
    fn provisioning_process_tree_helper() {
        let Some(mode) = std::env::var_os("BOXDD_PROVISION_PROCESS_TREE_MODE") else {
            return;
        };
        let marker = PathBuf::from(
            std::env::var_os("BOXDD_PROVISION_PROCESS_TREE_MARKER")
                .expect("process-tree helper marker"),
        );
        if mode == "parent" {
            let mut descendant = Command::new(std::env::current_exe().expect("test executable"))
                .args([
                    "--ignored",
                    "--exact",
                    "emscripten_sdk::tests::provisioning_process_tree_helper",
                ])
                .env("BOXDD_PROVISION_PROCESS_TREE_MODE", "descendant")
                .env("BOXDD_PROVISION_PROCESS_TREE_MARKER", &marker)
                .spawn()
                .expect("spawn process-tree descendant");
            fs::write(marker.with_extension("spawned"), b"spawned")
                .expect("write descendant-spawned marker");
            descendant.wait().expect("wait for process-tree descendant");
        } else if mode == "descendant" {
            thread::sleep(Duration::from_millis(750));
            fs::write(marker, b"survived").expect("write descendant survival marker");
        }
    }

    #[cfg(unix)]
    #[test]
    fn provisioning_timeout_terminates_the_entire_process_group() {
        let directory = tempfile::tempdir().expect("process-tree fixture");
        let marker = directory.path().join("descendant-survived");
        let mut command = Command::new(std::env::current_exe().expect("test executable"));
        command
            .args([
                "--ignored",
                "--exact",
                "emscripten_sdk::tests::provisioning_process_tree_helper",
            ])
            .env("BOXDD_PROVISION_PROCESS_TREE_MODE", "parent")
            .env("BOXDD_PROVISION_PROCESS_TREE_MARKER", &marker);

        let error = command_output_with_timeout(
            &mut command,
            "exercise process-tree termination",
            Duration::from_millis(500),
        )
        .expect_err("the process tree must exceed its deadline");
        assert!(error.contains("timed out after 500ms"), "{error}");
        assert!(
            marker.with_extension("spawned").exists(),
            "the helper did not spawn its descendant before the timeout"
        );
        thread::sleep(Duration::from_millis(900));
        assert!(
            !marker.exists(),
            "a provisioning descendant survived process-group termination"
        );
    }

    #[test]
    fn qualified_tool_environment_removes_injections_and_overwrites_sdk_keys() {
        let root = Path::new("qualified-sdk");
        let em_config = Path::new("qualified-sdk/.emscripten");
        let python = Path::new("qualified-python");
        let node = Path::new("qualified-sdk/node");
        let cache = Path::new("qualified-scratch/cache");
        let ports = Path::new("qualified-scratch/ports");
        let scratch = Path::new("qualified-scratch");
        let mut command = Command::new("emcc");
        let environment = QualifiedToolEnvironment {
            root,
            em_config,
            python,
            node,
            cache,
            ports,
            scratch,
        };
        environment.configure(
            &mut command,
            [
                OsStr::new("LD_PRELOAD"),
                OsStr::new("NODE_OPTIONS"),
                OsStr::new("PYTHONPATH"),
                OsStr::new("CFLAGS"),
                OsStr::new("EMSDK"),
                OsStr::new("EM_CONFIG"),
                OsStr::new("EMSDK_PYTHON"),
                OsStr::new("EMSDK_NODE"),
            ],
        );

        for key in ["LD_PRELOAD", "NODE_OPTIONS", "PYTHONPATH", "CFLAGS"] {
            assert_eq!(command_environment(&command, key), Some(None), "{key}");
        }
        for (key, expected) in [
            ("EMSDK", root.as_os_str()),
            ("EM_CONFIG", em_config.as_os_str()),
            ("EMSDK_PYTHON", python.as_os_str()),
            ("EMSDK_NODE", node.as_os_str()),
            ("EM_CACHE", cache.as_os_str()),
            ("EM_PORTS", ports.as_os_str()),
            ("TMPDIR", scratch.as_os_str()),
            ("TMP", scratch.as_os_str()),
            ("TEMP", scratch.as_os_str()),
            ("PYTHONDONTWRITEBYTECODE", OsStr::new("1")),
        ] {
            assert_eq!(
                command_environment(&command, key),
                Some(Some(expected)),
                "{key}"
            );
        }
    }

    #[test]
    fn qualified_emcc_command_uses_python_isolation_and_the_canonical_driver() {
        let root = Path::new("qualified-sdk");
        let em_config = Path::new("qualified-sdk/.emscripten");
        let python = Path::new("qualified-sdk/python");
        let node = Path::new("qualified-sdk/node");
        let emscripten_root = Path::new("qualified-sdk/upstream/emscripten");
        let driver = Path::new("qualified-sdk/upstream/emscripten/emcc.py");
        let cache = Path::new("qualified-scratch/cache");
        let ports = Path::new("qualified-scratch/ports");
        let scratch = Path::new("qualified-scratch");
        let environment = QualifiedToolEnvironment {
            root,
            em_config,
            python,
            node,
            cache,
            ports,
            scratch,
        };

        let command = configured_emcc_command(
            emscripten_root,
            driver,
            [
                OsStr::new("PYTHONPATH"),
                OsStr::new("_PYTHON_SYSCONFIGDATA_NAME"),
                OsStr::new("EMSDK_PYTHON"),
            ],
            environment,
        );

        assert_eq!(command.get_program(), python.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![
                OsStr::new("-I"),
                OsStr::new("-B"),
                OsStr::new("-X"),
                OsStr::new("utf8"),
                OsStr::new("-c"),
                OsStr::new(EMCC_BOOTSTRAP),
                emscripten_root.as_os_str(),
                driver.as_os_str(),
                OsStr::new("--em-config"),
                em_config.as_os_str(),
            ]
        );
        assert_eq!(command_environment(&command, "PYTHONPATH"), Some(None));
        assert_eq!(
            command_environment(&command, "_PYTHON_SYSCONFIGDATA_NAME"),
            Some(None)
        );
        assert_eq!(
            command_environment(&command, "EMSDK_PYTHON"),
            Some(Some(python.as_os_str()))
        );
        assert_eq!(
            command_environment(&command, "EM_CACHE"),
            Some(Some(cache.as_os_str()))
        );
        assert_eq!(
            command_environment(&command, "PYTHONDONTWRITEBYTECODE"),
            Some(Some(OsStr::new("1")))
        );
    }

    #[test]
    fn sdk_contract_is_exact_and_closed() {
        let source = include_str!("../toolchains/emscripten-sdk.toml");
        assert_eq!(
            SdkContract::parse(source).unwrap(),
            SdkContract::canonical()
        );
        assert!(SdkContract::parse(&source.replace("6.0.3", "6.0.30")).is_err());
        assert!(SdkContract::parse(&format!("{source}\nunknown = true\n")).is_err());
    }

    #[test]
    fn github_actions_environment_contains_only_the_qualified_sdk_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let environment_file = temp.path().join("github-env");
        fs::write(&environment_file, "EXISTING=1\n").unwrap();
        let contract = SdkContract::canonical();
        let host = contract.current_host().unwrap();
        let platform = HostPlatform::current().unwrap();

        write_github_actions_emsdk_environment_file(
            &environment_file,
            &root,
            &contract,
            host,
            platform,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(&environment_file).unwrap(),
            format!(
                "EXISTING=1\nEMSDK={}\nEM_CONFIG={}\nEMSDK_PYTHON={}\nEMSDK_NODE={}\n",
                root.display(),
                root.join(".emscripten").display(),
                host.python.executable_path(&root).display(),
                root.join("node")
                    .join(format!("{}_64bit", contract.node_version))
                    .join("bin")
                    .join(platform.node_name())
                    .display(),
            )
        );
    }

    #[test]
    fn provisioning_destination_can_reuse_only_an_existing_real_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("emsdk");
        let (resolved, exists) = canonical_provision_destination(&destination).unwrap();
        assert_eq!(
            resolved,
            fs::canonicalize(temp.path()).unwrap().join("emsdk")
        );
        assert!(!exists);

        fs::create_dir(&destination).unwrap();
        let (resolved, exists) = canonical_provision_destination(&destination).unwrap();
        assert_eq!(resolved, fs::canonicalize(&destination).unwrap());
        assert!(exists);

        let file = temp.path().join("not-a-directory");
        fs::write(&file, b"occupied").unwrap();
        assert!(canonical_provision_destination(&file).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn private_emsdk_root_rejects_permissive_roots_and_ancestors() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sdk");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let root = fs::canonicalize(&root).unwrap();
        require_private_emsdk_root(&root, "test SDK root").unwrap();

        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let error = require_private_emsdk_root(&root, "test SDK root").unwrap_err();
        assert!(error.contains("mode 0700"));

        let shared = temp.path().join("shared");
        let nested_root = shared.join("sdk");
        fs::create_dir(&shared).unwrap();
        fs::create_dir(&nested_root).unwrap();
        fs::set_permissions(&nested_root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
        let nested_root = fs::canonicalize(&nested_root).unwrap();
        let error = require_private_emsdk_root(&nested_root, "test SDK root").unwrap_err();
        assert!(error.contains("writable without sticky protection"));
    }

    #[cfg(unix)]
    #[test]
    fn private_scratch_directory_checks_canonical_ancestors() {
        let temp = tempfile::tempdir().unwrap();
        secure_private_directory(temp.path(), "test scratch directory").unwrap();
    }

    #[test]
    fn sdk_evidence_binds_checkout_release_and_activated_config() {
        let contract = SdkContract::canonical();
        for (platform, host) in [
            (HostPlatform::Linux, &contract.hosts[0]),
            (HostPlatform::MacOs, &contract.hosts[1]),
        ] {
            let evidence = valid_evidence(&contract, host, platform);
            validate_sdk_evidence(&contract, host, platform, &evidence).unwrap();

            let mut stale = valid_evidence(&contract, host, platform);
            stale.emscripten_revision.replace_range(..1, "0");
            assert!(validate_sdk_evidence(&contract, host, platform, &stale).is_err());

            let mut attached = valid_evidence(&contract, host, platform);
            attached.emsdk_head_reference = "main".to_owned();
            assert!(validate_sdk_evidence(&contract, host, platform, &attached).is_err());

            let mut configured_elsewhere = valid_evidence(&contract, host, platform);
            configured_elsewhere
                .activated_config
                .push_str("LLVM_ROOT = '/tmp/llvm'\n");
            assert!(
                validate_sdk_evidence(&contract, host, platform, &configured_elsewhere).is_err()
            );
        }
    }

    #[test]
    fn compiler_resolves_only_from_the_qualified_sdk() {
        let temp = tempfile::tempdir().unwrap();
        let emscripten = temp.path().join("upstream").join("emscripten");
        let platform = HostPlatform::current().unwrap();
        fs::create_dir_all(&emscripten).unwrap();
        let compiler = emscripten.join(platform.compiler_name());
        let compiler_driver = emscripten.join("emcc.py");
        let wasm_opt =
            temp.path()
                .join("upstream")
                .join("bin")
                .join(if platform == HostPlatform::Windows {
                    "wasm-opt.exe"
                } else {
                    "wasm-opt"
                });
        fs::create_dir_all(wasm_opt.parent().unwrap()).unwrap();
        fs::write(&compiler, b"compiler").unwrap();
        fs::write(&compiler_driver, b"compiler driver").unwrap();
        fs::write(&wasm_opt, b"wasm-opt").unwrap();
        let sdk_root = fs::canonicalize(temp.path()).unwrap();

        assert_eq!(
            resolve_compiler(&sdk_root, &emscripten, platform).unwrap(),
            fs::canonicalize(&compiler).unwrap()
        );
        assert_eq!(
            resolve_compiler_driver(&sdk_root, &emscripten).unwrap(),
            fs::canonicalize(&compiler_driver).unwrap()
        );
        assert_eq!(
            resolve_wasm_opt(&sdk_root, &temp.path().join("upstream"), platform).unwrap(),
            fs::canonicalize(&wasm_opt).unwrap()
        );
    }

    #[test]
    fn runtime_tool_overrides_follow_the_host_sdk_policy() {
        let temp = tempfile::tempdir().unwrap();
        let sdk_root = temp.path().join("sdk");
        fs::create_dir_all(&sdk_root).unwrap();

        let python_root = sdk_root
            .join("python")
            .join(format!("{TEST_BUNDLED_PYTHON_VERSION}_64bit"));
        fs::create_dir_all(python_root.join("bin")).unwrap();
        fs::write(
            python_root.join(".emsdk_version"),
            format!("python-{TEST_BUNDLED_PYTHON_VERSION}-64bit\n"),
        )
        .unwrap();
        let python = python_root.join("bin").join("python3.13");
        fs::write(&python, b"python").unwrap();
        let external_python = temp.path().join("external-python");
        fs::write(&external_python, b"python").unwrap();

        let node_root = temp.path().join("node-runtime");
        fs::create_dir_all(node_root.join("bin")).unwrap();
        let unix_node = node_root.join("bin").join("node");
        let windows_node = node_root.join("bin").join("node.exe");
        fs::write(&unix_node, b"node").unwrap();
        fs::write(&windows_node, b"node").unwrap();

        let arbitrary = temp.path().join("arbitrary-runtime");
        fs::write(&arbitrary, b"runtime").unwrap();
        let sdk_root = fs::canonicalize(sdk_root).unwrap();
        let node_root = fs::canonicalize(node_root).unwrap();
        let python_contract = SdkContract::canonical().hosts[1].python.clone();

        let resolved = resolve_python(&sdk_root, Some(&python), &python_contract).unwrap();
        assert_eq!(
            resolved.runtime_root,
            fs::canonicalize(&python_root).unwrap()
        );
        assert_eq!(resolved.executable, fs::canonicalize(&python).unwrap());
        assert!(resolved.archive_tree.is_some());
        assert!(resolve_python(&sdk_root, Some(&external_python), &python_contract).is_err());
        assert!(resolve_python(&sdk_root, Some(&arbitrary), &python_contract).is_err());
        assert!(resolve_python(&sdk_root, None, &python_contract).is_ok());

        for (platform, node) in [
            (HostPlatform::Linux, &unix_node),
            (HostPlatform::MacOs, &unix_node),
            (HostPlatform::Windows, &windows_node),
        ] {
            assert_eq!(
                resolve_node(&node_root, Some(node), platform).unwrap(),
                fs::canonicalize(node).unwrap()
            );
            assert!(resolve_node(&node_root, Some(&arbitrary), platform).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn python_override_accepts_only_the_official_activation_symlink() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let sdk_root = temp.path().join("sdk");
        let python_root = sdk_root
            .join("python")
            .join(format!("{TEST_BUNDLED_PYTHON_VERSION}_64bit"));
        let bin = python_root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            python_root.join(".emsdk_version"),
            format!("python-{TEST_BUNDLED_PYTHON_VERSION}-64bit\n"),
        )
        .unwrap();
        let expected = bin.join("python3.13");
        let other = bin.join("python3.12");
        let activation = bin.join("python3");
        let external = temp.path().join("external-python");
        fs::write(&expected, b"python").unwrap();
        fs::write(&other, b"other python").unwrap();
        fs::write(&external, b"external python").unwrap();
        symlink("python3.13", &activation).unwrap();
        let sdk_root = fs::canonicalize(sdk_root).unwrap();
        let python_contract = SdkContract::canonical().hosts[1].python.clone();

        let resolved = resolve_python(&sdk_root, Some(&activation), &python_contract).unwrap();
        assert_eq!(
            resolved.runtime_root,
            fs::canonicalize(&python_root).unwrap()
        );
        assert_eq!(resolved.executable, fs::canonicalize(&expected).unwrap());
        assert!(resolved.archive_tree.is_some());

        fs::remove_file(&activation).unwrap();
        symlink(&external, &activation).unwrap();
        assert!(resolve_python(&sdk_root, Some(&activation), &python_contract).is_err());

        fs::remove_file(&activation).unwrap();
        symlink("python3.12", &activation).unwrap();
        assert!(resolve_python(&sdk_root, Some(&activation), &python_contract).is_err());
    }

    #[test]
    fn python_version_output_requires_one_exact_canonical_line() {
        for (stdout, stderr) in [
            (b"Python 3.13.3".as_slice(), b"".as_slice()),
            (b"Python 3.13.3\n".as_slice(), b"".as_slice()),
            (b"Python 3.13.3\r\n".as_slice(), b"".as_slice()),
            (b"".as_slice(), b"Python 3.13.3".as_slice()),
            (b"".as_slice(), b"Python 3.13.3\n".as_slice()),
            (b"".as_slice(), b"Python 3.13.3\r\n".as_slice()),
        ] {
            validate_python_version_output(
                stdout,
                stderr,
                PythonVersionRequirement::Exact(TEST_BUNDLED_PYTHON_VERSION),
            )
            .unwrap();
        }

        for (stdout, stderr) in [
            (b"".as_slice(), b"".as_slice()),
            (b"Python 3.13.3".as_slice(), b"Python 3.13.3".as_slice()),
            (b"Python 3.13.30".as_slice(), b"".as_slice()),
            (b"Python 3.13.3 extra".as_slice(), b"".as_slice()),
            (b"Python 3.13.3\nextra".as_slice(), b"".as_slice()),
            (b"Python 3.13.3\nPython 3.12.0\n".as_slice(), b"".as_slice()),
            (b"Python 3.13.3\n\n".as_slice(), b"".as_slice()),
            (b"Python 3.13.3\r".as_slice(), b"".as_slice()),
            (b"Python 3.13.3".as_slice(), b"Python 3.12.0".as_slice()),
        ] {
            assert!(
                validate_python_version_output(
                    stdout,
                    stderr,
                    PythonVersionRequirement::Exact(TEST_BUNDLED_PYTHON_VERSION),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn system_python_version_enforces_the_minimum_without_pinning_a_patch() {
        for version in ["3.10.0", "3.12.3", "3.13.3", "4.0.0"] {
            validate_python_version_output(
                format!("Python {version}\n").as_bytes(),
                b"",
                PythonVersionRequirement::Minimum(TEST_MINIMUM_SYSTEM_PYTHON_VERSION),
            )
            .unwrap();
        }
        for version in ["3.9.99", "3.010.0", "3.12", "3.12.3+local"] {
            assert!(
                validate_python_version_output(
                    format!("Python {version}\n").as_bytes(),
                    b"",
                    PythonVersionRequirement::Minimum(TEST_MINIMUM_SYSTEM_PYTHON_VERSION),
                )
                .is_err(),
                "{version}"
            );
        }
    }

    #[test]
    fn linux_host_probe_requires_exact_os_release_identity() {
        validate_linux_os_release_identity(
            "NAME=Ubuntu\nID=ubuntu\nVERSION_ID=\"24.04\"\n",
            "ubuntu",
            "24.04",
        )
        .unwrap();
        assert!(
            validate_linux_os_release_identity(
                "ID=ubuntu\nVERSION_ID=\"22.04\"\n",
                "ubuntu",
                "24.04",
            )
            .is_err()
        );
        assert!(
            validate_linux_os_release_identity(
                "ID=ubuntu\nID=debian\nVERSION_ID=\"24.04\"\n",
                "ubuntu",
                "24.04",
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
    fn qualified_files_reject_post_qualification_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tool");
        fs::write(&path, b"qualified bytes").unwrap();
        let files = qualify_files(vec![path.clone()]).unwrap();
        validate_qualified_files(&files).unwrap();

        fs::write(&path, b"mutated bytes").unwrap();
        assert!(validate_qualified_files(&files).is_err());
    }

    #[test]
    fn qualified_tree_digest_is_deterministic_and_binds_every_entry() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        fs::create_dir_all(first.path().join("empty")).unwrap();
        fs::create_dir_all(first.path().join("nested")).unwrap();
        fs::write(first.path().join("nested/tool"), b"tool bytes").unwrap();
        fs::write(first.path().join("identity"), b"identity\n").unwrap();

        fs::write(second.path().join("identity"), b"identity\n").unwrap();
        fs::create_dir_all(second.path().join("nested")).unwrap();
        fs::write(second.path().join("nested/tool"), b"tool bytes").unwrap();
        fs::create_dir_all(second.path().join("empty")).unwrap();

        let expected = qualified_tree_sha256(first.path(), "test tree").unwrap();
        assert_eq!(
            expected,
            qualified_tree_sha256(second.path(), "test tree").unwrap()
        );

        fs::write(first.path().join("nested/tool"), b"changed bytes").unwrap();
        assert_ne!(
            expected,
            qualified_tree_sha256(first.path(), "test tree").unwrap()
        );
        fs::write(first.path().join("nested/tool"), b"tool bytes").unwrap();
        fs::create_dir(first.path().join("additional-empty")).unwrap();
        assert_ne!(
            expected,
            qualified_tree_sha256(first.path(), "test tree").unwrap()
        );
    }

    #[test]
    fn qualified_tree_revalidation_rejects_post_qualification_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("tool");
        fs::write(&file, b"qualified bytes").unwrap();
        let digest = qualified_tree_sha256(temp.path(), "test tree").unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let tree = qualify_tree("test tree", root, &digest).unwrap();
        validate_qualified_trees(std::slice::from_ref(&tree)).unwrap();

        fs::write(&file, b"mutated bytes").unwrap();
        assert!(validate_qualified_trees(&[tree]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn qualified_tree_digest_binds_executable_bits_and_symlink_targets() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let temp = tempfile::tempdir().unwrap();
        let tool = temp.path().join("tool");
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let selected = temp.path().join("selected");
        fs::write(&tool, b"tool").unwrap();
        fs::write(&first, b"same").unwrap();
        fs::write(&second, b"same").unwrap();
        symlink("first", &selected).unwrap();
        let original = qualified_tree_sha256(temp.path(), "test tree").unwrap();

        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let executable = qualified_tree_sha256(temp.path(), "test tree").unwrap();
        assert_ne!(original, executable);

        fs::remove_file(&selected).unwrap();
        symlink("second", &selected).unwrap();
        assert_ne!(
            executable,
            qualified_tree_sha256(temp.path(), "test tree").unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn qualified_tree_digest_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let tree = temp.path().join("tree");
        fs::create_dir(&tree).unwrap();
        fs::write(temp.path().join("outside"), b"outside").unwrap();
        symlink("../outside", tree.join("escape")).unwrap();

        let error = qualified_tree_sha256(&tree, "test tree").unwrap_err();
        assert!(error.contains("escapes"));
    }

    #[cfg(unix)]
    #[test]
    fn qualified_tree_digest_rejects_hard_links() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        fs::write(&first, b"shared inode").unwrap();
        fs::hard_link(&first, temp.path().join("second")).unwrap();

        let error = qualified_tree_sha256(temp.path(), "test tree").unwrap_err();
        assert!(error.contains("must not have hard links"));
    }

    #[test]
    fn ambient_git_environment_is_removed_case_insensitively() {
        assert!(crate::qualified_git::is_git_environment_key(OsStr::new(
            "GIT_DIR"
        )));
        assert!(crate::qualified_git::is_git_environment_key(OsStr::new(
            "git_object_directory"
        )));
        assert!(!crate::qualified_git::is_git_environment_key(OsStr::new(
            "LEGIT_SETTING"
        )));
    }

    #[test]
    fn ambient_emscripten_overrides_are_classified_case_insensitively() {
        for key in [
            "EMCC_CCACHE",
            "_EMCC_CCACHE",
            "emcc_cflags",
            "EM_CACHE",
            "EM_COMPILER_WRAPPER",
            "EM_LLVM_ROOT",
            "EMMAKEN_JUST_CONFIGURE",
            "LLVM",
            "BINARYEN",
            "NODE",
            "NODE_OPTIONS",
            "NODE_PATH",
            "PYTHONHOME",
            "PYTHONPATH",
        ] {
            assert!(
                is_disallowed_ambient_emscripten_environment_key(OsStr::new(key)),
                "{key} must be rejected"
            );
        }
        for key in [
            "EMSDK",
            "EM_CONFIG",
            "EMSDK_PYTHON",
            "EMSDK_NODE",
            "EMSDK_VERSION",
            "EMSDK_REVISION",
            "EMSDK_QUIET",
            "EMAIL",
        ] {
            assert!(
                !is_disallowed_ambient_emscripten_environment_key(OsStr::new(key)),
                "{key} must remain permitted"
            );
        }
        assert!(is_qualified_tool_environment_key(OsStr::new(
            "NODE_OPTIONS"
        )));
        assert!(is_qualified_tool_environment_key(OsStr::new("PYTHONPATH")));
        for key in ["LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "BASH_ENV"] {
            assert!(!is_disallowed_ambient_emscripten_environment_key(
                OsStr::new(key)
            ));
            assert!(is_qualified_tool_environment_key(OsStr::new(key)));
        }
        for key in [
            "CPPFLAGS",
            "CFLAGS_wasm32-unknown-unknown",
            "TARGET_CFLAGS",
            "CPATH",
            "CRATE_CC_NO_DEFAULTS",
        ] {
            assert!(is_qualified_tool_environment_key(OsStr::new(key)));
        }
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
        let revision = std::ffi::OsString::from(SdkContract::canonical().emsdk_revision);
        let error = qualify_emscripten_sdk(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            EmscriptenSdkInputs {
                root: None,
                em_config_override: None,
                python_override: None,
                node_override: None,
                self_attested_revision: Some(&revision),
            },
        )
        .unwrap_err();
        assert!(error.contains("self-attestation"));

        let error = qualify_emscripten_sdk(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            EmscriptenSdkInputs {
                root: None,
                em_config_override: None,
                python_override: None,
                node_override: None,
                self_attested_revision: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("PATH-only compiler discovery is not qualified"));
    }
}
