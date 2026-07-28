use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
};

use flate2::{Compression, GzBuilder, read::GzDecoder};
use tar::{Archive, Builder, EntryType, Header};

use crate::{
    Error, Result,
    emscripten_sdk::{SDK_CONTRACT_RELATIVE_PATH, SdkContract},
    provenance_policy::{
        self, COSIGN_VERSION, PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW,
        SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH, SIGSTORE_TRUSTED_ROOT_SHA256,
    },
    provider_manifest::{self, ADAPTER_ABI_VERSION, RECORDING_CONTRACT_BLAKE3},
    qualified_git::{qualified_git_command, remove_process_injection_environment},
    source_overlay::{adapter_source_sha256, effective_source_identity},
    wasm_provider_contract::{
        COMPILER_TARGET, ENDIANNESS, POINTER_WIDTH, PROVIDER_ABI, SIMD_MODE,
        WasmProviderExpectation, WasmProviderIdentity, contract_relative_path,
    },
    wasm_release_provenance::{
        SCHEMA_NAME as PROVENANCE_SCHEMA, SCHEMA_VERSION as PROVENANCE_SCHEMA_VERSION,
        WasmReleaseContext, WasmReleaseProvenanceStatement, canonical_inner_checksums_bytes,
        members_from_files, sha256_bytes,
    },
};

use super::{
    provider::{self, ProviderPrecision},
    verification,
};

const MANIFEST_SCHEMA_VERSION: u64 = 1;
const MANIFEST_SCHEMA: &str = "boxdd-wasm-provider-runtime-v1";
const PACKAGE_TYPE: &str = "wasm-provider";
const TARGET: &str = "wasm32-unknown-unknown";
const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MEMBER_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOTAL_MEMBER_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MEMBERS: usize = 64;
const MAX_PROVENANCE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_BUNDLE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TRUSTED_ROOT_BYTES: u64 = 4 * 1024 * 1024;
const RELEASE_WORKFLOW_NAME: &str = "Build Prebuilt Binaries (boxdd-sys)";

const PROJECT_MIT: &str = "licenses/PROJECT-LICENSE-MIT";
const PROJECT_APACHE: &str = "licenses/PROJECT-LICENSE-APACHE";
const BOX2D_LICENSE: &str = "licenses/BOX2D-LICENSE";
const RUNTIME_MANIFEST: &str = "manifest.toml";
const INNER_CHECKSUMS: &str = "checksums.sha256";

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeAsset {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WasmRuntimeManifest {
    schema_version: u64,
    schema: String,
    crate_version: String,
    source_commit: String,
    release_tag: String,
    provider_abi: String,
    target: String,
    compiler_target: String,
    precision: String,
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    adapter_abi_version: u64,
    adapter_source_sha256: String,
    recording_contract_blake3: String,
    validation_enabled: bool,
    simd: String,
    pointer_width: u64,
    endianness: String,
    emscripten_sdk_contract_sha256: String,
    wasm_provider_contract_sha256: String,
    bindings_sha256: String,
    private_abi_hash: String,
    snapshot_layout_hash: u64,
    provider_js: RuntimeAsset,
    provider_wasm: RuntimeAsset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepositoryRuntimeIdentity {
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    adapter_source_sha256: String,
    emscripten_sdk_contract_sha256: String,
    wasm_provider_contract_sha256: String,
    bindings_sha256: String,
    private_abi_hash: String,
    snapshot_layout_hash: u64,
}

struct PackageInputs<'a> {
    precision: ProviderPrecision,
    version: &'a str,
    source_commit: &'a str,
    release_tag: &'a str,
    identity: &'a RepositoryRuntimeIdentity,
    provider_js: &'a Path,
    provider_wasm: &'a Path,
}

struct CanonicalPackageArchive {
    bytes: Vec<u8>,
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuildOptions {
    precision: ProviderPrecision,
    output: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QualifyOptions {
    precision: ProviderPrecision,
    artifacts: PathBuf,
    cosign: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UnsignedReleaseContext<'a> {
    pub(crate) repository: &'a str,
    pub(crate) workflow_ref: &'a str,
    pub(crate) source_commit: &'a str,
    pub(crate) release_tag: &'a str,
    pub(crate) run_id: &'a str,
    pub(crate) run_attempt: &'a str,
    pub(crate) crate_version: &'a str,
}

pub(crate) fn build(root: &Path, args: &[String]) -> Result<()> {
    let options = BuildOptions::parse(args)?;
    let version = workspace_version(root)?;
    let output = prepare_output_directory(&options.output)?;
    let destination = output.join(archive_name(&version, options.precision.as_str())?);
    if destination.exists() {
        return Err(Error::message(format!(
            "WASM provider package output already exists: {}",
            destination.display()
        )));
    }

    let source_commit = qualified_checkout_commit(root)?;
    let release_tag = release_tag(&version)?;
    validate_build_github_context(&source_commit, &release_tag)?;
    validate_tag_points_at_head(root, &release_tag, &source_commit)?;
    require_clean_checkout(root)?;

    // This path compiles the official bytes but deliberately never runs either module.
    let (smoke, sdk) = provider::build_provider_smoke_only(root, options.precision)?;
    let identity = repository_runtime_identity(root, options.precision)?;
    if sdk.contract_sha256() != identity.emscripten_sdk_contract_sha256 {
        return Err(Error::message(
            "qualified Emscripten SDK does not match the repository SDK contract",
        ));
    }
    let files = package_files(
        root,
        PackageInputs {
            precision: options.precision,
            version: &version,
            source_commit: &source_commit,
            release_tag: &release_tag,
            identity: &identity,
            provider_js: smoke.provider_js(),
            provider_wasm: smoke.provider_wasm(),
        },
    )?;
    sdk.revalidate().map_err(Error::message)?;
    if qualified_checkout_commit(root)? != source_commit {
        return Err(Error::message(
            "checkout HEAD changed while the WASM provider package was built",
        ));
    }
    require_clean_checkout(root)?;
    if repository_runtime_identity(root, options.precision)? != identity {
        return Err(Error::message(
            "repository runtime identity changed while the WASM provider package was built",
        ));
    }
    let bytes = render_archive(&files)?;
    write_new_file(&destination, &bytes, "WASM provider package")?;
    drop(smoke);
    println!("WASM provider package ready: {}", destination.display());
    Ok(())
}

pub(crate) fn qualify(root: &Path, args: &[String]) -> Result<()> {
    let options = QualifyOptions::parse(args)?;
    qualify_authenticated(root, &options)
}

pub(crate) fn archive_name(version: &str, precision: &str) -> Result<String> {
    ProviderPrecision::parse(precision)?;
    if !is_canonical_version(version) {
        return Err(Error::message(format!(
            "WASM provider package version is not canonical: {version:?}"
        )));
    }
    Ok(format!(
        "boxdd-wasm-provider-{version}-{TARGET}-{precision}.tar.gz"
    ))
}

impl BuildOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut precision = None;
        let mut output = None;
        parse_options(
            args,
            "build-wasm-provider-package",
            |flag, value| match flag {
                "--precision" => set_once(
                    &mut precision,
                    ProviderPrecision::parse(value)?,
                    "--precision",
                ),
                "--output" => set_once(&mut output, PathBuf::from(value), "--output"),
                _ => Err(Error::message(format!(
                    "unknown build-wasm-provider-package argument {flag:?}"
                ))),
            },
        )?;
        Ok(Self {
            precision: precision.ok_or_else(|| {
                Error::message("build-wasm-provider-package requires --precision")
            })?,
            output: output
                .ok_or_else(|| Error::message("build-wasm-provider-package requires --output"))?,
        })
    }
}

impl QualifyOptions {
    fn parse(args: &[String]) -> Result<Self> {
        let mut precision = None;
        let mut artifacts = None;
        let mut cosign = None;
        parse_options(args, "qualify-wasm-provider", |flag, value| match flag {
            "--precision" => set_once(
                &mut precision,
                ProviderPrecision::parse(value)?,
                "--precision",
            ),
            "--artifacts" => set_once(&mut artifacts, PathBuf::from(value), "--artifacts"),
            "--cosign" => set_once(&mut cosign, PathBuf::from(value), "--cosign"),
            _ => Err(Error::message(format!(
                "unknown qualify-wasm-provider argument {flag:?}"
            ))),
        })?;
        Ok(Self {
            precision: precision
                .ok_or_else(|| Error::message("qualify-wasm-provider requires --precision"))?,
            artifacts: artifacts
                .ok_or_else(|| Error::message("qualify-wasm-provider requires --artifacts"))?,
            cosign: cosign
                .ok_or_else(|| Error::message("qualify-wasm-provider requires --cosign"))?,
        })
    }
}

fn parse_options(
    args: &[String],
    command: &str,
    mut set: impl FnMut(&str, &str) -> Result<()>,
) -> Result<()> {
    if !args.len().is_multiple_of(2) {
        return Err(Error::message(format!(
            "{command} expects flag/value pairs"
        )));
    }
    for pair in args.chunks_exact(2) {
        if !pair[0].starts_with("--") || pair[1].starts_with("--") {
            return Err(Error::message(format!(
                "{command} requires a value for {:?}",
                pair[0]
            )));
        }
        set(&pair[0], &pair[1])?;
    }
    Ok(())
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        Err(Error::message(format!("{flag} may only be supplied once")))
    } else {
        Ok(())
    }
}

impl WasmRuntimeManifest {
    fn parse_canonical(bytes: &[u8]) -> Result<Self> {
        let source = std::str::from_utf8(bytes).map_err(|error| {
            Error::message(format!("WASM runtime manifest is not UTF-8: {error}"))
        })?;
        let value: toml::Value = toml::from_str(source).map_err(|error| {
            Error::message(format!("WASM runtime manifest is not valid TOML: {error}"))
        })?;
        let table = value
            .as_table()
            .ok_or_else(|| Error::message("WASM runtime manifest root must be a TOML table"))?;
        const FIELDS: &[&str] = &[
            "adapter_abi_version",
            "adapter_source_sha256",
            "bindings_sha256",
            "compiler_target",
            "crate_version",
            "effective_source_sha256",
            "emscripten_sdk_contract_sha256",
            "endianness",
            "precision",
            "private_abi_hash",
            "provider_abi",
            "provider_js",
            "provider_wasm",
            "recording_contract_blake3",
            "release_tag",
            "schema",
            "schema_version",
            "simd",
            "snapshot_layout_hash",
            "source_commit",
            "source_tree",
            "target",
            "upstream_sha",
            "validation_enabled",
            "wasm_provider_contract_sha256",
            "pointer_width",
        ];
        let actual = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected = FIELDS.iter().copied().collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(Error::message(format!(
                "WASM runtime manifest fields do not match the closed schema: expected {expected:?}, found {actual:?}"
            )));
        }
        let manifest = Self {
            schema_version: required_integer(table, "schema_version")?,
            schema: required_string(table, "schema")?,
            crate_version: required_string(table, "crate_version")?,
            source_commit: required_string(table, "source_commit")?,
            release_tag: required_string(table, "release_tag")?,
            provider_abi: required_string(table, "provider_abi")?,
            target: required_string(table, "target")?,
            compiler_target: required_string(table, "compiler_target")?,
            precision: required_string(table, "precision")?,
            upstream_sha: required_string(table, "upstream_sha")?,
            source_tree: required_string(table, "source_tree")?,
            effective_source_sha256: required_string(table, "effective_source_sha256")?,
            adapter_abi_version: required_integer(table, "adapter_abi_version")?,
            adapter_source_sha256: required_string(table, "adapter_source_sha256")?,
            recording_contract_blake3: required_string(table, "recording_contract_blake3")?,
            validation_enabled: required_bool(table, "validation_enabled")?,
            simd: required_string(table, "simd")?,
            pointer_width: required_integer(table, "pointer_width")?,
            endianness: required_string(table, "endianness")?,
            emscripten_sdk_contract_sha256: required_string(
                table,
                "emscripten_sdk_contract_sha256",
            )?,
            wasm_provider_contract_sha256: required_string(table, "wasm_provider_contract_sha256")?,
            bindings_sha256: required_string(table, "bindings_sha256")?,
            private_abi_hash: required_string(table, "private_abi_hash")?,
            snapshot_layout_hash: required_integer(table, "snapshot_layout_hash")?,
            provider_js: required_asset(table, "provider_js")?,
            provider_wasm: required_asset(table, "provider_wasm")?,
        };
        manifest.validate_intrinsic()?;
        if manifest.render().as_bytes() != bytes {
            return Err(Error::message(
                "WASM runtime manifest is not in canonical byte form",
            ));
        }
        Ok(manifest)
    }

    fn validate_intrinsic(&self) -> Result<()> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION || self.schema != MANIFEST_SCHEMA {
            return Err(Error::message(format!(
                "unsupported WASM runtime manifest schema: version={} name={:?}",
                self.schema_version, self.schema
            )));
        }
        if !is_canonical_version(&self.crate_version) {
            return Err(Error::message(
                "WASM runtime manifest has an invalid crate version",
            ));
        }
        validate_release_tag(&self.release_tag, &self.crate_version)?;
        validate_lower_hex("source_commit", &self.source_commit, 40)?;
        validate_lower_hex("upstream_sha", &self.upstream_sha, 40)?;
        validate_lower_hex("source_tree", &self.source_tree, 40)?;
        for (label, digest) in [
            (
                "effective_source_sha256",
                self.effective_source_sha256.as_str(),
            ),
            ("adapter_source_sha256", self.adapter_source_sha256.as_str()),
            (
                "emscripten_sdk_contract_sha256",
                self.emscripten_sdk_contract_sha256.as_str(),
            ),
            (
                "wasm_provider_contract_sha256",
                self.wasm_provider_contract_sha256.as_str(),
            ),
            ("bindings_sha256", self.bindings_sha256.as_str()),
            ("private_abi_hash", self.private_abi_hash.as_str()),
        ] {
            validate_lower_hex(label, digest, 64)?;
        }
        if self.provider_abi != PROVIDER_ABI
            || self.target != TARGET
            || self.compiler_target != COMPILER_TARGET
            || ProviderPrecision::parse(&self.precision).is_err()
            || self.adapter_abi_version != ADAPTER_ABI_VERSION
            || self.recording_contract_blake3 != RECORDING_CONTRACT_BLAKE3
            || self.validation_enabled
            || self.simd != SIMD_MODE
            || self.pointer_width != POINTER_WIDTH
            || self.endianness != ENDIANNESS
            || self.snapshot_layout_hash == 0
        {
            return Err(Error::message(
                "WASM runtime manifest contains unsupported ABI coordinates",
            ));
        }
        validate_asset(&self.provider_js)?;
        validate_asset(&self.provider_wasm)?;
        let precision = ProviderPrecision::parse(&self.precision)?;
        if self.provider_js.path != provider_member_path(precision, "js")
            || self.provider_wasm.path != provider_member_path(precision, "wasm")
        {
            return Err(Error::message(
                "WASM runtime manifest provider paths do not match its precision",
            ));
        }
        Ok(())
    }

    fn render(&self) -> String {
        format!(
            concat!(
                "schema_version = {}\n",
                "schema = {:?}\n",
                "crate_version = {:?}\n",
                "source_commit = {:?}\n",
                "release_tag = {:?}\n",
                "provider_abi = {:?}\n",
                "target = {:?}\n",
                "compiler_target = {:?}\n",
                "precision = {:?}\n",
                "upstream_sha = {:?}\n",
                "source_tree = {:?}\n",
                "effective_source_sha256 = {:?}\n",
                "adapter_abi_version = {}\n",
                "adapter_source_sha256 = {:?}\n",
                "recording_contract_blake3 = {:?}\n",
                "validation_enabled = {}\n",
                "simd = {:?}\n",
                "pointer_width = {}\n",
                "endianness = {:?}\n",
                "emscripten_sdk_contract_sha256 = {:?}\n",
                "wasm_provider_contract_sha256 = {:?}\n",
                "bindings_sha256 = {:?}\n",
                "private_abi_hash = {:?}\n",
                "snapshot_layout_hash = {}\n",
                "\n[provider_js]\n",
                "path = {:?}\n",
                "size = {}\n",
                "sha256 = {:?}\n",
                "\n[provider_wasm]\n",
                "path = {:?}\n",
                "size = {}\n",
                "sha256 = {:?}\n",
            ),
            self.schema_version,
            self.schema,
            self.crate_version,
            self.source_commit,
            self.release_tag,
            self.provider_abi,
            self.target,
            self.compiler_target,
            self.precision,
            self.upstream_sha,
            self.source_tree,
            self.effective_source_sha256,
            self.adapter_abi_version,
            self.adapter_source_sha256,
            self.recording_contract_blake3,
            self.validation_enabled,
            self.simd,
            self.pointer_width,
            self.endianness,
            self.emscripten_sdk_contract_sha256,
            self.wasm_provider_contract_sha256,
            self.bindings_sha256,
            self.private_abi_hash,
            self.snapshot_layout_hash,
            self.provider_js.path,
            self.provider_js.size,
            self.provider_js.sha256,
            self.provider_wasm.path,
            self.provider_wasm.size,
            self.provider_wasm.sha256,
        )
    }
}

fn required_asset(table: &toml::Table, key: &str) -> Result<RuntimeAsset> {
    let asset = table
        .get(key)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            Error::message(format!(
                "WASM runtime manifest field `{key}` must be a table"
            ))
        })?;
    let actual = asset.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = ["path", "sha256", "size"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(Error::message(format!(
            "WASM runtime manifest asset `{key}` has unsupported fields"
        )));
    }
    Ok(RuntimeAsset {
        path: required_string(asset, "path")?,
        size: required_integer(asset, "size")?,
        sha256: required_string(asset, "sha256")?,
    })
}

fn required_string(table: &toml::Table, key: &str) -> Result<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            Error::message(format!(
                "WASM runtime manifest field `{key}` must be a non-empty string"
            ))
        })
}

fn required_integer(table: &toml::Table, key: &str) -> Result<u64> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            Error::message(format!(
                "WASM runtime manifest field `{key}` must be a non-negative integer"
            ))
        })
}

fn required_bool(table: &toml::Table, key: &str) -> Result<bool> {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .ok_or_else(|| {
            Error::message(format!(
                "WASM runtime manifest field `{key}` must be a boolean"
            ))
        })
}

fn validate_asset(asset: &RuntimeAsset) -> Result<()> {
    validate_relative_path(&asset.path)?;
    if asset.size == 0 || asset.size > MAX_MEMBER_BYTES {
        return Err(Error::message(format!(
            "WASM runtime asset {:?} has invalid size {}",
            asset.path, asset.size
        )));
    }
    validate_lower_hex("runtime asset sha256", &asset.sha256, 64)
}

fn package_files(root: &Path, inputs: PackageInputs<'_>) -> Result<BTreeMap<String, Vec<u8>>> {
    let js =
        read_bounded_regular_file(inputs.provider_js, MAX_MEMBER_BYTES, "provider JavaScript")?;
    let wasm = read_bounded_regular_file(inputs.provider_wasm, MAX_MEMBER_BYTES, "provider WASM")?;
    let js_path = provider_member_path(inputs.precision, "js");
    let wasm_path = provider_member_path(inputs.precision, "wasm");
    let manifest = WasmRuntimeManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        schema: MANIFEST_SCHEMA.to_owned(),
        crate_version: inputs.version.to_owned(),
        source_commit: inputs.source_commit.to_owned(),
        release_tag: inputs.release_tag.to_owned(),
        provider_abi: PROVIDER_ABI.to_owned(),
        target: TARGET.to_owned(),
        compiler_target: COMPILER_TARGET.to_owned(),
        precision: inputs.precision.as_str().to_owned(),
        upstream_sha: inputs.identity.upstream_sha.clone(),
        source_tree: inputs.identity.source_tree.clone(),
        effective_source_sha256: inputs.identity.effective_source_sha256.clone(),
        adapter_abi_version: ADAPTER_ABI_VERSION,
        adapter_source_sha256: inputs.identity.adapter_source_sha256.clone(),
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
        validation_enabled: false,
        simd: SIMD_MODE.to_owned(),
        pointer_width: POINTER_WIDTH,
        endianness: ENDIANNESS.to_owned(),
        emscripten_sdk_contract_sha256: inputs.identity.emscripten_sdk_contract_sha256.clone(),
        wasm_provider_contract_sha256: inputs.identity.wasm_provider_contract_sha256.clone(),
        bindings_sha256: inputs.identity.bindings_sha256.clone(),
        private_abi_hash: inputs.identity.private_abi_hash.clone(),
        snapshot_layout_hash: inputs.identity.snapshot_layout_hash,
        provider_js: RuntimeAsset {
            path: js_path.clone(),
            size: js.len() as u64,
            sha256: provider_manifest::sha256_bytes(&js),
        },
        provider_wasm: RuntimeAsset {
            path: wasm_path.clone(),
            size: wasm.len() as u64,
            sha256: provider_manifest::sha256_bytes(&wasm),
        },
    };
    manifest.validate_intrinsic()?;

    let mut files = BTreeMap::from([
        (js_path, js),
        (wasm_path, wasm),
        (RUNTIME_MANIFEST.to_owned(), manifest.render().into_bytes()),
        (
            PROJECT_MIT.to_owned(),
            read_bounded_regular_file(&root.join("LICENSE-MIT"), MAX_MEMBER_BYTES, "MIT license")?,
        ),
        (
            PROJECT_APACHE.to_owned(),
            read_bounded_regular_file(
                &root.join("LICENSE-APACHE"),
                MAX_MEMBER_BYTES,
                "Apache license",
            )?,
        ),
        (
            BOX2D_LICENSE.to_owned(),
            read_bounded_regular_file(
                &root.join("boxdd-sys/third-party/box2d/LICENSE"),
                MAX_MEMBER_BYTES,
                "Box2D license",
            )?,
        ),
    ]);
    files.insert(
        INNER_CHECKSUMS.to_owned(),
        canonical_inner_checksums(&files)?.into_bytes(),
    );
    validate_package_files(
        root,
        &files,
        inputs.precision,
        inputs.version,
        inputs.source_commit,
        inputs.release_tag,
    )?;
    Ok(files)
}

fn provider_member_path(precision: ProviderPrecision, extension: &str) -> String {
    format!("provider/{}.{extension}", precision.module())
}

fn expected_package_paths(precision: ProviderPrecision) -> BTreeSet<String> {
    BTreeSet::from([
        INNER_CHECKSUMS.to_owned(),
        BOX2D_LICENSE.to_owned(),
        PROJECT_APACHE.to_owned(),
        PROJECT_MIT.to_owned(),
        RUNTIME_MANIFEST.to_owned(),
        provider_member_path(precision, "js"),
        provider_member_path(precision, "wasm"),
    ])
}

fn canonical_inner_checksums(files: &BTreeMap<String, Vec<u8>>) -> Result<String> {
    let members = members_from_files(files).map_err(Error::message)?;
    let bytes = canonical_inner_checksums_bytes(&members).map_err(Error::message)?;
    String::from_utf8(bytes)
        .map_err(|error| Error::message(format!("canonical WASM checksums are not UTF-8: {error}")))
}

fn workspace_version(root: &Path) -> Result<String> {
    let path = root.join("Cargo.toml");
    let bytes = read_bounded_regular_file(&path, MAX_MEMBER_BYTES, "workspace manifest")?;
    let source = std::str::from_utf8(&bytes)
        .map_err(|error| Error::message(format!("workspace manifest is not UTF-8: {error}")))?;
    let value: toml::Value = toml::from_str(source)
        .map_err(|error| Error::message(format!("workspace manifest is invalid TOML: {error}")))?;
    let version = value
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::message("workspace.package.version is missing"))?;
    if !is_canonical_version(version) {
        return Err(Error::message(format!(
            "workspace package version is not canonical: {version:?}"
        )));
    }
    Ok(version.to_owned())
}

fn is_canonical_version(version: &str) -> bool {
    let (without_build, build) = match version.split_once('+') {
        Some((left, right)) if !right.contains('+') => (left, Some(right)),
        Some(_) => return false,
        None => (version, None),
    };
    let (core, prerelease) = match without_build.split_once('-') {
        Some((left, right)) => (left, Some(right)),
        None => (without_build, None),
    };
    let core = core.split('.').collect::<Vec<_>>();
    if core.len() != 3
        || !core.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (*part == "0" || !part.starts_with('0'))
        })
    {
        return false;
    }
    let identifiers_are_canonical = |value: &str, reject_numeric_leading_zero: bool| {
        !value.is_empty()
            && value.split('.').all(|identifier| {
                !identifier.is_empty()
                    && identifier
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    && (!reject_numeric_leading_zero
                        || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                        || identifier == "0"
                        || !identifier.starts_with('0'))
            })
    };
    prerelease.is_none_or(|value| identifiers_are_canonical(value, true))
        && build.is_none_or(|value| identifiers_are_canonical(value, false))
}

fn validate_release_tag(tag: &str, version: &str) -> Result<()> {
    if tag == format!("v{version}") || tag == format!("boxdd-sys-v{version}") {
        Ok(())
    } else {
        Err(Error::message(format!(
            "release tag {tag:?} does not match crate version {version}"
        )))
    }
}

fn release_tag(version: &str) -> Result<String> {
    let tag = env::var("GITHUB_REF_NAME").unwrap_or_else(|_| format!("v{version}"));
    validate_release_tag(&tag, version)?;
    Ok(tag)
}

fn qualified_checkout_commit(root: &Path) -> Result<String> {
    let commit = git_output(root, &["rev-parse", "HEAD"], "read checkout HEAD")?;
    validate_lower_hex("checkout HEAD", &commit, 40)?;
    Ok(commit)
}

fn git_output(root: &Path, args: &[&str], label: &str) -> Result<String> {
    let output = qualified_git_command()
        .map_err(Error::message)?
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| Error::io(label, error))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "{label} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| Error::message(format!("{label} did not return UTF-8: {error}")))
}

fn require_clean_checkout(root: &Path) -> Result<()> {
    let status = git_output(
        root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        "inspect WASM release checkout",
    )?;
    if status.is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM release checkout must be clean; found:\n{status}"
        )))
    }
}

fn validate_tag_points_at_head(root: &Path, tag: &str, commit: &str) -> Result<()> {
    if env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        // The release checkout is intentionally by immutable SHA and may not fetch tag refs.
        return Ok(());
    }
    let revision = format!("refs/tags/{tag}^{{commit}}");
    let tagged = git_output(root, &["rev-parse", &revision], "resolve WASM release tag")?;
    if tagged == commit {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM release tag {tag:?} resolves to {tagged}, not checkout HEAD {commit}"
        )))
    }
}

fn validate_build_github_context(commit: &str, tag: &str) -> Result<()> {
    let Some(actions) = env::var_os("GITHUB_ACTIONS") else {
        return Ok(());
    };
    if actions != "true" {
        return Err(Error::message(
            "GITHUB_ACTIONS must be exactly `true` when GitHub context is present",
        ));
    }
    require_env_equal("GITHUB_SHA", commit)?;
    require_env_equal("GITHUB_REF_TYPE", "tag")?;
    require_env_equal("GITHUB_REF", &format!("refs/tags/{tag}"))?;
    require_env_equal("GITHUB_REF_NAME", tag)?;
    require_env_equal("GITHUB_REF_PROTECTED", "true")?;
    require_env_equal("GITHUB_EVENT_NAME", "push")?;
    require_env_equal("GITHUB_REPOSITORY", PUBLISHER_REPOSITORY)?;
    require_env_equal("GITHUB_WORKFLOW", RELEASE_WORKFLOW_NAME)?;
    require_env_equal(
        "GITHUB_WORKFLOW_REF",
        &format!("{PUBLISHER_REPOSITORY}/{PUBLISHER_WORKFLOW}@refs/tags/{tag}"),
    )?;
    validate_positive_decimal_env("GITHUB_RUN_ID")?;
    validate_positive_decimal_env("GITHUB_RUN_ATTEMPT")
}

fn require_env_equal(key: &str, expected: &str) -> Result<()> {
    let actual = env::var(key)
        .map_err(|_| Error::message(format!("WASM release requires {key}={expected:?}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM release {key}={actual:?} does not match {expected:?}"
        )))
    }
}

fn validate_positive_decimal_env(key: &str) -> Result<()> {
    let value =
        env::var(key).map_err(|_| Error::message(format!("WASM release requires {key}")))?;
    if is_positive_decimal(&value) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM release {key} must be a positive canonical decimal"
        )))
    }
}

fn is_positive_decimal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) && !value.starts_with('0')
}

fn prepare_output_directory(path: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
                return Err(Error::message(format!(
                    "WASM package output must be a real directory: {}",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(path).map_err(|error| Error::io(path, error))?;
        }
        Err(error) => return Err(Error::io(path, error)),
    }
    let canonical = fs::canonicalize(path).map_err(|error| Error::io(path, error))?;
    let metadata =
        fs::symlink_metadata(&canonical).map_err(|error| Error::io(&canonical, error))?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        Ok(canonical)
    } else {
        Err(Error::message(format!(
            "WASM package output must resolve to a real directory: {}",
            path.display()
        )))
    }
}

fn write_new_file(path: &Path, bytes: &[u8], label: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| Error::io(path, error))?;
    file.write_all(bytes)
        .map_err(|error| Error::io(path, error))?;
    file.flush().map_err(|error| Error::io(path, error))?;
    file.sync_all().map_err(|error| Error::io(path, error))?;
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() != bytes.len() as u64
    {
        return Err(Error::message(format!(
            "{label} was not written as the exact regular file: {}",
            path.display()
        )));
    }
    Ok(())
}

fn repository_runtime_identity(
    root: &Path,
    precision: ProviderPrecision,
) -> Result<RepositoryRuntimeIdentity> {
    let sys_root = root.join("boxdd-sys");
    let effective = effective_source_identity(&sys_root).map_err(Error::message)?;
    let adapter_source_sha256 = adapter_source_sha256(&sys_root).map_err(Error::message)?;
    let bindings = read_bounded_regular_file(
        &sys_root.join("src").join(precision.wasm_bindings_file()),
        MAX_MEMBER_BYTES,
        "WASM bindings",
    )?;
    let bindings_sha256 = sha256_bytes(&bindings);
    let expectation = WasmProviderExpectation {
        provider_abi: PROVIDER_ABI,
        target: TARGET,
        compiler_target: COMPILER_TARGET,
        precision: precision.as_str(),
        upstream_sha: &effective.upstream_sha,
        source_tree: &effective.source_tree,
        effective_source_sha256: &effective.effective_source_sha256,
        adapter_abi_version: ADAPTER_ABI_VERSION,
        adapter_source_sha256: &adapter_source_sha256,
        recording_contract_blake3: RECORDING_CONTRACT_BLAKE3,
        validation_enabled: false,
        simd: SIMD_MODE,
        pointer_width: POINTER_WIDTH,
        endianness: ENDIANNESS,
        bindings_sha256: &bindings_sha256,
    };
    let contract = contract_relative_path(precision.as_str()).map_err(Error::message)?;
    let (provider, provider_contract_bytes) =
        WasmProviderIdentity::load_with_source_bytes(&sys_root, Path::new(contract), &expectation)
            .map_err(Error::message)?;

    let sdk_path = root.join("xtask").join(SDK_CONTRACT_RELATIVE_PATH);
    let sdk_bytes =
        read_bounded_regular_file(&sdk_path, MAX_MEMBER_BYTES, "Emscripten SDK contract")?;
    let sdk_source = std::str::from_utf8(&sdk_bytes).map_err(|error| {
        Error::message(format!("Emscripten SDK contract is not UTF-8: {error}"))
    })?;
    SdkContract::parse(sdk_source).map_err(Error::message)?;

    Ok(RepositoryRuntimeIdentity {
        upstream_sha: effective.upstream_sha,
        source_tree: effective.source_tree,
        effective_source_sha256: effective.effective_source_sha256,
        adapter_source_sha256,
        emscripten_sdk_contract_sha256: sha256_bytes(&sdk_bytes),
        wasm_provider_contract_sha256: sha256_bytes(&provider_contract_bytes),
        bindings_sha256,
        private_abi_hash: hex_bytes(&provider.private_abi_hash),
        snapshot_layout_hash: u64::from(provider.snapshot_layout_hash),
    })
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut rendered = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        rendered.push(HEX[(byte >> 4) as usize] as char);
        rendered.push(HEX[(byte & 0x0f) as usize] as char);
    }
    rendered
}

fn render_archive(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    validate_archive_inventory(files)?;
    let encoder = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), Compression::best());
    let mut archive = Builder::new(encoder);
    for (path, bytes) in files {
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Regular);
        header.set_path(path).map_err(|error| {
            Error::message(format!(
                "failed to encode WASM package member {path:?}: {error}"
            ))
        })?;
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        archive
            .append(&header, Cursor::new(bytes))
            .map_err(|error| Error::message(format!("failed to append {path:?}: {error}")))?;
    }
    let encoder = archive
        .into_inner()
        .map_err(|error| Error::message(format!("failed to finish WASM package tar: {error}")))?;
    encoder
        .finish()
        .map_err(|error| Error::message(format!("failed to finish WASM package gzip: {error}")))
}

fn validate_archive_inventory(files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    if files.is_empty() || files.len() > MAX_MEMBERS {
        return Err(Error::message(format!(
            "WASM package member count must be in 1..={MAX_MEMBERS}"
        )));
    }
    let mut total = 0_u64;
    for (path, bytes) in files {
        validate_relative_path(path)?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_MEMBER_BYTES {
            return Err(Error::message(format!(
                "WASM package member {path:?} is outside the accepted size range"
            )));
        }
        total = total
            .checked_add(bytes.len() as u64)
            .filter(|value| *value <= MAX_TOTAL_MEMBER_BYTES)
            .ok_or_else(|| Error::message("WASM package members exceed the total size limit"))?;
    }
    Ok(())
}

fn read_package_archive(path: &Path) -> Result<CanonicalPackageArchive> {
    let bytes = read_bounded_regular_file(path, MAX_PACKAGE_BYTES, "WASM provider package")?;
    let files = read_archive_bytes(&bytes)?;
    Ok(CanonicalPackageArchive { bytes, files })
}

fn read_archive_bytes(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return Err(Error::message(
            "WASM package archive is outside the accepted size range",
        ));
    }
    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(decoder);
    let mut files = BTreeMap::new();
    let mut total = 0_u64;
    let entries = archive
        .entries()
        .map_err(|error| Error::message(format!("invalid WASM package tar stream: {error}")))?;
    for entry in entries {
        let mut entry = entry
            .map_err(|error| Error::message(format!("invalid WASM package entry: {error}")))?;
        if files.len() >= MAX_MEMBERS {
            return Err(Error::message(format!(
                "WASM package contains more than {MAX_MEMBERS} members"
            )));
        }
        if !entry.header().entry_type().is_file() {
            return Err(Error::message(
                "WASM package may contain only regular file members",
            ));
        }
        let path = std::str::from_utf8(entry.path_bytes().as_ref())
            .map_err(|error| Error::message(format!("WASM package path is not UTF-8: {error}")))?
            .to_owned();
        validate_relative_path(&path)?;
        let size = entry.header().size().map_err(|error| {
            Error::message(format!("invalid size for member {path:?}: {error}"))
        })?;
        if size == 0 || size > MAX_MEMBER_BYTES {
            return Err(Error::message(format!(
                "WASM package member {path:?} is outside the accepted size range"
            )));
        }
        total = total
            .checked_add(size)
            .filter(|value| *value <= MAX_TOTAL_MEMBER_BYTES)
            .ok_or_else(|| Error::message("WASM package members exceed the total size limit"))?;
        let mut contents = Vec::with_capacity(size as usize);
        Read::by_ref(&mut entry)
            .take(size + 1)
            .read_to_end(&mut contents)
            .map_err(|error| Error::message(format!("failed to read member {path:?}: {error}")))?;
        if contents.len() as u64 != size {
            return Err(Error::message(format!(
                "WASM package member {path:?} size changed while reading"
            )));
        }
        if files.insert(path.clone(), contents).is_some() {
            return Err(Error::message(format!(
                "WASM package contains duplicate member {path:?}"
            )));
        }
    }
    validate_archive_inventory(&files)?;
    let canonical = render_archive(&files)?;
    if canonical != bytes {
        return Err(Error::message(
            "WASM package archive is not in canonical deterministic tar.gz form",
        ));
    }
    Ok(files)
}

fn validate_package_files(
    root: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    precision: ProviderPrecision,
    version: &str,
    source_commit: &str,
    release_tag: &str,
) -> Result<WasmRuntimeManifest> {
    let actual = files.keys().cloned().collect::<BTreeSet<_>>();
    let expected = expected_package_paths(precision);
    if actual != expected {
        return Err(Error::message(format!(
            "WASM package members do not match the fixed inventory: expected {expected:?}, found {actual:?}"
        )));
    }
    let expected_checksums = canonical_inner_checksums(files)?;
    if files[INNER_CHECKSUMS] != expected_checksums.as_bytes() {
        return Err(Error::message(
            "WASM package checksums.sha256 is not canonical or does not match its members",
        ));
    }
    for (member, source, label) in [
        (PROJECT_MIT, root.join("LICENSE-MIT"), "project MIT license"),
        (
            PROJECT_APACHE,
            root.join("LICENSE-APACHE"),
            "project Apache license",
        ),
        (
            BOX2D_LICENSE,
            root.join("boxdd-sys/third-party/box2d/LICENSE"),
            "Box2D license",
        ),
    ] {
        let expected = read_bounded_regular_file(&source, MAX_MEMBER_BYTES, label)?;
        if files[member] != expected {
            return Err(Error::message(format!(
                "WASM package member {member:?} does not match the repository source"
            )));
        }
    }

    let manifest = WasmRuntimeManifest::parse_canonical(&files[RUNTIME_MANIFEST])?;
    let identity = repository_runtime_identity(root, precision)?;
    if manifest.crate_version != version
        || manifest.source_commit != source_commit
        || manifest.release_tag != release_tag
        || manifest.precision != precision.as_str()
        || manifest.upstream_sha != identity.upstream_sha
        || manifest.source_tree != identity.source_tree
        || manifest.effective_source_sha256 != identity.effective_source_sha256
        || manifest.adapter_source_sha256 != identity.adapter_source_sha256
        || manifest.emscripten_sdk_contract_sha256 != identity.emscripten_sdk_contract_sha256
        || manifest.wasm_provider_contract_sha256 != identity.wasm_provider_contract_sha256
        || manifest.bindings_sha256 != identity.bindings_sha256
        || manifest.private_abi_hash != identity.private_abi_hash
        || manifest.snapshot_layout_hash != identity.snapshot_layout_hash
    {
        return Err(Error::message(
            "WASM runtime manifest does not match the exact release and repository identity",
        ));
    }
    validate_release_tag(&manifest.release_tag, &manifest.crate_version)?;
    let js = &files[&manifest.provider_js.path];
    let wasm = &files[&manifest.provider_wasm.path];
    for (asset, bytes) in [(&manifest.provider_js, js), (&manifest.provider_wasm, wasm)] {
        if asset.size != bytes.len() as u64 || asset.sha256 != sha256_bytes(bytes) {
            return Err(Error::message(format!(
                "WASM runtime asset {:?} does not match its manifest identity",
                asset.path
            )));
        }
    }
    Ok(manifest)
}

pub(crate) fn validate_unsigned_package(
    repository_root: &Path,
    archive_path: &Path,
    precision: &str,
    context: UnsignedReleaseContext<'_>,
) -> Result<WasmReleaseProvenanceStatement> {
    let precision = ProviderPrecision::parse(precision)?;
    let expected_name = archive_name(context.crate_version, precision.as_str())?;
    if archive_path.file_name() != Some(OsStr::new(&expected_name)) {
        return Err(Error::message(format!(
            "WASM package filename must be exactly {expected_name:?}: {}",
            archive_path.display()
        )));
    }
    let CanonicalPackageArchive {
        bytes: package,
        files,
    } = read_package_archive(archive_path)?;
    let manifest = validate_package_files(
        repository_root,
        &files,
        precision,
        context.crate_version,
        context.source_commit,
        context.release_tag,
    )?;
    release_statement_from_verified_package(expected_name, &package, &files, &manifest, context)
}

fn release_statement_from_verified_package(
    package_name: String,
    package: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &WasmRuntimeManifest,
    context: UnsignedReleaseContext<'_>,
) -> Result<WasmReleaseProvenanceStatement> {
    let members = members_from_files(files).map_err(Error::message)?;
    let statement = WasmReleaseProvenanceStatement {
        schema_version: PROVENANCE_SCHEMA_VERSION,
        schema: PROVENANCE_SCHEMA.to_owned(),
        repository: context.repository.to_owned(),
        workflow: PUBLISHER_WORKFLOW.to_owned(),
        workflow_ref: context.workflow_ref.to_owned(),
        source_commit: context.source_commit.to_owned(),
        release_tag: context.release_tag.to_owned(),
        run_id: context.run_id.to_owned(),
        run_attempt: context.run_attempt.to_owned(),
        crate_version: context.crate_version.to_owned(),
        package_type: PACKAGE_TYPE.to_owned(),
        package_name,
        package_size: package.len() as u64,
        package_sha256: sha256_bytes(package),
        provider_abi: manifest.provider_abi.clone(),
        target: manifest.target.clone(),
        compiler_target: manifest.compiler_target.clone(),
        precision: manifest.precision.clone(),
        upstream_sha: manifest.upstream_sha.clone(),
        source_tree: manifest.source_tree.clone(),
        effective_source_sha256: manifest.effective_source_sha256.clone(),
        adapter_abi_version: manifest.adapter_abi_version,
        adapter_source_sha256: manifest.adapter_source_sha256.clone(),
        recording_contract_blake3: manifest.recording_contract_blake3.clone(),
        validation_enabled: manifest.validation_enabled,
        simd: manifest.simd.clone(),
        pointer_width: manifest.pointer_width,
        endianness: manifest.endianness.clone(),
        emscripten_sdk_contract_sha256: manifest.emscripten_sdk_contract_sha256.clone(),
        wasm_provider_contract_sha256: manifest.wasm_provider_contract_sha256.clone(),
        bindings_sha256: manifest.bindings_sha256.clone(),
        private_abi_hash: manifest.private_abi_hash.clone(),
        snapshot_layout_hash: manifest.snapshot_layout_hash,
        member_count: members.len() as u64,
        members,
    };
    statement.validate_intrinsic().map_err(Error::message)?;
    statement
        .validate_publisher(PUBLISHER_REPOSITORY, PUBLISHER_WORKFLOW)
        .map_err(Error::message)?;
    statement
        .validate_release_context(WasmReleaseContext {
            repository: context.repository,
            workflow: PUBLISHER_WORKFLOW,
            workflow_ref: context.workflow_ref,
            source_commit: context.source_commit,
            release_tag: context.release_tag,
            run_id: context.run_id,
            run_attempt: context.run_attempt,
            crate_version: context.crate_version,
            precision: &manifest.precision,
        })
        .map_err(Error::message)?;
    statement
        .verify_package_bytes(package)
        .map_err(Error::message)?;
    statement.verify_members(files).map_err(Error::message)?;
    Ok(statement)
}

fn validate_lower_hex(label: &str, value: &str, length: usize) -> Result<()> {
    if value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} must be exactly {length} lowercase hexadecimal characters"
        )))
    }
}

fn validate_relative_path(path: &str) -> Result<()> {
    let portable = path
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'));
    if path.is_empty()
        || !portable
        || path.contains("//")
        || path.starts_with("./")
        || path.ends_with('/')
        || Path::new(path).is_absolute()
        || !Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        Err(Error::message(format!(
            "WASM package path {path:?} is not a portable normalized relative path"
        )))
    } else {
        Ok(())
    }
}

fn read_bounded_regular_file(path: &Path, maximum_bytes: u64, label: &str) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        )));
    }
    if metadata.len() == 0 || metadata.len() > maximum_bytes {
        return Err(Error::message(format!(
            "{label} size {} is outside the accepted 1..={maximum_bytes} byte range",
            metadata.len()
        )));
    }
    let mut file = File::open(path).map_err(|error| Error::io(path, error))?;
    let opened = file.metadata().map_err(|error| Error::io(path, error))?;
    if !opened.is_file() || opened.len() != metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while it was being opened: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
            return Err(Error::message(format!(
                "{label} changed while it was being opened: {}",
                path.display()
            )));
        }
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io(path, error))?;
    if bytes.len() as u64 != metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while it was being read: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QualificationContext {
    repository: String,
    workflow_ref: String,
    source_commit: String,
    release_tag: String,
    run_id: String,
    run_attempt: String,
    crate_version: String,
}

impl QualificationContext {
    fn unsigned(&self) -> UnsignedReleaseContext<'_> {
        UnsignedReleaseContext {
            repository: &self.repository,
            workflow_ref: &self.workflow_ref,
            source_commit: &self.source_commit,
            release_tag: &self.release_tag,
            run_id: &self.run_id,
            run_attempt: &self.run_attempt,
            crate_version: &self.crate_version,
        }
    }

    fn signed(&self, precision: ProviderPrecision) -> WasmReleaseContext<'_> {
        WasmReleaseContext {
            repository: &self.repository,
            workflow: PUBLISHER_WORKFLOW,
            workflow_ref: &self.workflow_ref,
            source_commit: &self.source_commit,
            release_tag: &self.release_tag,
            run_id: &self.run_id,
            run_attempt: &self.run_attempt,
            crate_version: &self.crate_version,
            precision: precision.as_str(),
        }
    }
}

fn qualify_authenticated(root: &Path, options: &QualifyOptions) -> Result<()> {
    let version = workspace_version(root)?;
    let source_commit = qualified_checkout_commit(root)?;
    let release_tag = release_tag(&version)?;
    let context = qualification_context(&version, &source_commit, &release_tag)?;
    validate_tag_points_at_head(root, &release_tag, &source_commit)?;
    require_clean_checkout(root)?;

    let artifacts = canonical_real_directory(&options.artifacts, "WASM release artifact root")?;
    let package_name = archive_name(&version, options.precision.as_str())?;
    let archive_source = find_exact_regular_file(&artifacts, &package_name)?;
    let statement_source = adjacent_input(
        &archive_source,
        &package_name,
        ".provenance.toml",
        &artifacts,
        "WASM release provenance statement",
    )?;
    let bundle_source = adjacent_input(
        &archive_source,
        &package_name,
        ".provenance.sigstore.json",
        &artifacts,
        "WASM release Sigstore bundle",
    )?;
    let trusted_root_source = trusted_root_source(root)?;

    let scratch = tempfile::Builder::new()
        .prefix("boxdd-wasm-qualification-")
        .tempdir()
        .map_err(|error| {
            Error::message(format!("failed to create WASM qualification root: {error}"))
        })?;
    let scratch_root =
        fs::canonicalize(scratch.path()).map_err(|error| Error::io(scratch.path(), error))?;
    require_outside(&scratch_root, root, "WASM qualification root")?;
    let inputs = scratch_root.join("authenticated-inputs");
    create_private_directory(&inputs, "WASM qualification input directory")?;

    // Snapshot every mutable input before parsing or invoking the verifier.
    let archive = snapshot_bounded_regular_file(
        &archive_source,
        &inputs.join(&package_name),
        MAX_PACKAGE_BYTES,
        "WASM provider package",
    )?;
    let statement_name = format!("{package_name}.provenance.toml");
    let statement_path = snapshot_bounded_regular_file(
        &statement_source,
        &inputs.join(&statement_name),
        MAX_PROVENANCE_BYTES,
        "WASM release provenance statement",
    )?;
    let bundle_name = format!("{package_name}.provenance.sigstore.json");
    let bundle = snapshot_bounded_regular_file(
        &bundle_source,
        &inputs.join(&bundle_name),
        MAX_BUNDLE_BYTES,
        "WASM release Sigstore bundle",
    )?;
    let trusted_root = snapshot_bounded_regular_file(
        &trusted_root_source,
        &inputs.join("trusted_root.json"),
        MAX_TRUSTED_ROOT_BYTES,
        "repository Sigstore trusted root",
    )?;

    let statement_bytes = read_bounded_regular_file(
        &statement_path,
        MAX_PROVENANCE_BYTES,
        "snapshotted WASM release provenance statement",
    )?;
    let bundle_bytes = read_bounded_regular_file(
        &bundle,
        MAX_BUNDLE_BYTES,
        "snapshotted WASM release Sigstore bundle",
    )?;
    let trusted_root_bytes = read_bounded_regular_file(
        &trusted_root,
        MAX_TRUSTED_ROOT_BYTES,
        "snapshotted repository Sigstore trusted root",
    )?;
    validate_trusted_root_bytes(&trusted_root_bytes)?;

    let cosign = resolve_executable(&options.cosign, "Cosign")?;
    verify_cosign_version(&cosign)?;
    verify_signature(&cosign, &statement_path, &bundle, &trusted_root, &context)?;
    revalidate_snapshot(
        &statement_path,
        &statement_bytes,
        MAX_PROVENANCE_BYTES,
        "WASM release provenance statement",
    )?;
    revalidate_snapshot(
        &bundle,
        &bundle_bytes,
        MAX_BUNDLE_BYTES,
        "WASM release Sigstore bundle",
    )?;
    revalidate_snapshot(
        &trusted_root,
        &trusted_root_bytes,
        MAX_TRUSTED_ROOT_BYTES,
        "repository Sigstore trusted root",
    )?;

    // Authentication is the boundary after which package bytes may be interpreted.
    let package = read_bounded_regular_file(
        &archive,
        MAX_PACKAGE_BYTES,
        "snapshotted WASM provider package",
    )?;
    let statement = WasmReleaseProvenanceStatement::parse_canonical_for_package(
        &statement_bytes,
        context.signed(options.precision),
        &package,
    )
    .map_err(|error| Error::message(format!("invalid signed WASM release provenance: {error}")))?;
    let files = verify_authenticated_archive(&statement, &package)?;
    let manifest = validate_package_files(
        root,
        &files,
        options.precision,
        &version,
        &source_commit,
        &release_tag,
    )?;
    let expected = release_statement_from_verified_package(
        package_name.clone(),
        &package,
        &files,
        &manifest,
        context.unsigned(),
    )?;
    if statement != expected {
        return Err(Error::message(
            "signed WASM provenance does not match the exact package, manifest, and release context",
        ));
    }

    let provider_root = scratch_root.join("verified-provider");
    let (provider_js, provider_wasm) =
        materialize_verified_provider(&provider_root, &files, options.precision)?;
    let smoke = provider::prepare_existing_provider_smoke(
        root,
        options.precision,
        &provider_js,
        &provider_wasm,
    )?;

    let contract = provider::provider_toolchain_contract()?;
    let node = resolve_executable(Path::new("node"), "Node.js")?;
    verify_node_version(&node, &contract.node_version)?;
    let npm = resolve_executable(Path::new("npm"), "npm")?;
    provider::run_existing_provider_node_smoke(runtime_command(&node), &smoke)?;
    verification::run_existing_provider_browser_smoke(
        npm_command(&npm, &node)?,
        root,
        &smoke,
        options.precision,
    )?;

    if qualified_checkout_commit(root)? != source_commit {
        return Err(Error::message(
            "checkout HEAD changed during authenticated WASM qualification",
        ));
    }
    require_clean_checkout(root)?;
    let revalidated = validate_package_files(
        root,
        &files,
        options.precision,
        &version,
        &source_commit,
        &release_tag,
    )?;
    if revalidated != manifest {
        return Err(Error::message(
            "repository runtime identity changed during authenticated WASM qualification",
        ));
    }
    drop(smoke);
    println!(
        "authenticated WASM provider qualified: {} ({})",
        package_name,
        options.precision.as_str()
    );
    Ok(())
}

fn qualification_context(
    version: &str,
    source_commit: &str,
    release_tag: &str,
) -> Result<QualificationContext> {
    require_env_equal("GITHUB_ACTIONS", "true")?;
    validate_build_github_context(source_commit, release_tag)?;
    let repository = env::var("GITHUB_REPOSITORY")
        .map_err(|_| Error::message("WASM qualification requires GITHUB_REPOSITORY"))?;
    let workflow_ref = env::var("GITHUB_WORKFLOW_REF")
        .map_err(|_| Error::message("WASM qualification requires GITHUB_WORKFLOW_REF"))?;
    let run_id = env::var("GITHUB_RUN_ID")
        .map_err(|_| Error::message("WASM qualification requires GITHUB_RUN_ID"))?;
    let run_attempt = env::var("GITHUB_RUN_ATTEMPT")
        .map_err(|_| Error::message("WASM qualification requires GITHUB_RUN_ATTEMPT"))?;
    Ok(QualificationContext {
        repository,
        workflow_ref,
        source_commit: source_commit.to_owned(),
        release_tag: release_tag.to_owned(),
        run_id,
        run_attempt,
        crate_version: version.to_owned(),
    })
}

fn verify_authenticated_archive(
    statement: &WasmReleaseProvenanceStatement,
    package: &[u8],
) -> Result<BTreeMap<String, Vec<u8>>> {
    statement.verify_package_bytes(package).map_err(|error| {
        Error::message(format!("signed WASM package identity mismatch: {error}"))
    })?;
    let files = read_archive_bytes(package)?;
    statement.verify_members(&files).map_err(|error| {
        Error::message(format!("signed WASM package inventory mismatch: {error}"))
    })?;
    Ok(files)
}

fn trusted_root_source(root: &Path) -> Result<PathBuf> {
    let checkout = fs::canonicalize(root).map_err(|error| Error::io(root, error))?;
    let source = root
        .join("boxdd-sys")
        .join(SIGSTORE_TRUSTED_ROOT_RELATIVE_PATH);
    let metadata = fs::symlink_metadata(&source).map_err(|error| Error::io(&source, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "repository Sigstore trusted root must be a regular non-symlink file: {}",
            source.display()
        )));
    }
    let source = fs::canonicalize(&source).map_err(|error| Error::io(&source, error))?;
    if !source.starts_with(checkout) {
        return Err(Error::message(
            "repository Sigstore trusted root resolves outside the checkout",
        ));
    }
    Ok(source)
}

fn validate_trusted_root_bytes(bytes: &[u8]) -> Result<()> {
    let actual = sha256_bytes(bytes);
    if actual == SIGSTORE_TRUSTED_ROOT_SHA256 {
        Ok(())
    } else {
        Err(Error::message(format!(
            "repository Sigstore trusted root digest {actual} does not match {SIGSTORE_TRUSTED_ROOT_SHA256}"
        )))
    }
}

fn revalidate_snapshot(
    path: &Path,
    expected: &[u8],
    maximum_bytes: u64,
    label: &str,
) -> Result<()> {
    let current = read_bounded_regular_file(path, maximum_bytes, label)?;
    if current == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "private {label} snapshot changed while it was authenticated: {}",
            path.display()
        )))
    }
}

fn canonical_real_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be a real non-symlink directory: {}",
            path.display()
        )));
    }
    fs::canonicalize(path).map_err(|error| Error::io(path, error))
}

fn require_outside(path: &Path, forbidden: &Path, label: &str) -> Result<()> {
    let forbidden = fs::canonicalize(forbidden).map_err(|error| Error::io(forbidden, error))?;
    if path.starts_with(&forbidden) {
        Err(Error::message(format!(
            "{label} must remain outside the checkout: {}",
            path.display()
        )))
    } else {
        Ok(())
    }
}

fn find_exact_regular_file(root: &Path, name: &str) -> Result<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| Error::io(&directory, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| Error::io(&directory, error))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| Error::io(&path, error))?;
            if metadata.file_type().is_symlink() {
                return Err(Error::message(format!(
                    "WASM artifact tree must not contain symlinks: {}",
                    path.display()
                )));
            }
            if metadata.file_type().is_dir() {
                pending.push(path);
            } else if metadata.file_type().is_file() {
                if path.file_name() == Some(OsStr::new(name)) {
                    found.push(fs::canonicalize(&path).map_err(|error| Error::io(&path, error))?);
                }
            } else {
                return Err(Error::message(format!(
                    "WASM artifact tree must not contain special entries: {}",
                    path.display()
                )));
            }
        }
    }
    found.sort();
    if found.len() == 1 {
        Ok(found.remove(0))
    } else {
        Err(Error::message(format!(
            "expected exactly one WASM provider package named {name:?}; found {found:?}"
        )))
    }
}

fn adjacent_input(
    archive: &Path,
    archive_name: &str,
    suffix: &str,
    artifact_root: &Path,
    label: &str,
) -> Result<PathBuf> {
    let path = archive.with_file_name(format!("{archive_name}{suffix}"));
    let metadata = fs::symlink_metadata(&path).map_err(|error| Error::io(&path, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be an adjacent regular non-symlink file: {}",
            path.display()
        )));
    }
    let canonical = fs::canonicalize(&path).map_err(|error| Error::io(&path, error))?;
    if !canonical.starts_with(artifact_root) {
        return Err(Error::message(format!(
            "{label} resolves outside the artifact root: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn create_private_directory(path: &Path, label: &str) -> Result<()> {
    fs::create_dir(path).map_err(|error| Error::io(path, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| Error::io(path, error))?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| Error::io(path, error))?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "{label} is not a real directory: {}",
            path.display()
        )))
    }
}

fn snapshot_bounded_regular_file(
    source: &Path,
    destination: &Path,
    maximum_bytes: u64,
    label: &str,
) -> Result<PathBuf> {
    let source_metadata = fs::symlink_metadata(source).map_err(|error| Error::io(source, error))?;
    if !source_metadata.file_type().is_file() || source_metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be a regular non-symlink file: {}",
            source.display()
        )));
    }
    if source_metadata.len() == 0 || source_metadata.len() > maximum_bytes {
        return Err(Error::message(format!(
            "{label} size {} is outside the accepted 1..={maximum_bytes} byte range",
            source_metadata.len()
        )));
    }
    let mut input = File::open(source).map_err(|error| Error::io(source, error))?;
    let opened_metadata = input.metadata().map_err(|error| Error::io(source, error))?;
    if !opened_metadata.is_file() || opened_metadata.len() != source_metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while it was opened for snapshotting: {}",
            source.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if opened_metadata.dev() != source_metadata.dev()
            || opened_metadata.ino() != source_metadata.ino()
        {
            return Err(Error::message(format!(
                "{label} changed while it was opened for snapshotting: {}",
                source.display()
            )));
        }
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| Error::io(destination, error))?;
    let copied = io::copy(
        &mut Read::by_ref(&mut input).take(maximum_bytes + 1),
        &mut output,
    )
    .map_err(|error| Error::io(destination, error))?;
    if copied != source_metadata.len() {
        return Err(Error::message(format!(
            "{label} changed while its bytes were snapshotted: {}",
            source.display()
        )));
    }
    output
        .flush()
        .map_err(|error| Error::io(destination, error))?;
    output
        .sync_all()
        .map_err(|error| Error::io(destination, error))?;
    let destination =
        fs::canonicalize(destination).map_err(|error| Error::io(destination, error))?;
    let metadata =
        fs::symlink_metadata(&destination).map_err(|error| Error::io(&destination, error))?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() != copied
    {
        return Err(Error::message(format!(
            "private {label} snapshot is not the exact regular file written: {}",
            destination.display()
        )));
    }
    Ok(destination)
}

fn resolve_executable(requested: &Path, label: &str) -> Result<PathBuf> {
    let candidates = if requested.is_absolute() || requested.components().count() > 1 {
        vec![requested.to_path_buf()]
    } else {
        let path = env::var_os("PATH")
            .ok_or_else(|| Error::message(format!("PATH is required to resolve {label}")))?;
        env::split_paths(&path)
            .filter(|directory| !directory.as_os_str().is_empty())
            .flat_map(|directory| executable_candidates(&directory, requested))
            .collect::<Vec<_>>()
    };
    for candidate in candidates {
        let Ok(canonical) = fs::canonicalize(&candidate) else {
            continue;
        };
        let metadata =
            fs::symlink_metadata(&canonical).map_err(|error| Error::io(&canonical, error))?;
        if metadata.file_type().is_file()
            && !metadata.file_type().is_symlink()
            && is_executable(&canonical)?
        {
            return Ok(canonical);
        }
    }
    Err(Error::message(format!(
        "failed to resolve executable {label} path {requested:?}"
    )))
}

fn executable_candidates(directory: &Path, requested: &Path) -> Vec<PathBuf> {
    let direct = directory.join(requested);
    #[cfg(windows)]
    {
        let mut candidates = vec![direct.clone()];
        if requested.extension().is_none() {
            let extensions = env::var_os("PATHEXT").unwrap_or_else(|| ".EXE;.CMD;.BAT;.COM".into());
            candidates.extend(
                extensions
                    .to_string_lossy()
                    .split(';')
                    .filter(|extension| !extension.is_empty())
                    .map(|extension| {
                        let mut name = requested.as_os_str().to_os_string();
                        name.push(extension);
                        directory.join(name)
                    }),
            );
        }
        candidates
    }
    #[cfg(not(windows))]
    {
        vec![direct]
    }
}

fn is_executable(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let metadata = fs::metadata(path).map_err(|error| Error::io(path, error))?;
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(path.is_file())
    }
}

fn verify_cosign_version(cosign: &Path) -> Result<()> {
    let mut command = Command::new(cosign);
    remove_process_injection_environment(&mut command);
    let output = command
        .arg("version")
        .output()
        .map_err(|error| Error::io(cosign, error))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "WASM qualification requires Cosign {COSIGN_VERSION}; version command failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let version = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if provenance_policy::cosign_version_is_qualified(&version) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM qualification requires exact Cosign {COSIGN_VERSION}; found {}",
            version
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("unknown version")
        )))
    }
}

fn verify_signature(
    cosign: &Path,
    statement: &Path,
    bundle: &Path,
    trusted_root: &Path,
    context: &QualificationContext,
) -> Result<()> {
    let args = provenance_policy::cosign_verify_blob_args(provenance_policy::PrebuiltProvenance {
        crate_version: &context.crate_version,
        source_commit: &context.source_commit,
        release_tag: &context.release_tag,
        payload: statement,
        bundle,
        trusted_root,
    })
    .map_err(|error| Error::message(format!("invalid WASM Sigstore policy input: {error}")))?;
    let mut command = Command::new(cosign);
    remove_process_injection_environment(&mut command);
    let output = command
        .args(args)
        .output()
        .map_err(|error| Error::io("verify WASM provenance signature", error))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "WASM provenance signature verification failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn materialize_verified_provider(
    root: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    precision: ProviderPrecision,
) -> Result<(PathBuf, PathBuf)> {
    create_private_directory(root, "verified WASM provider directory")?;
    let provider_root = root.join("provider");
    create_private_directory(&provider_root, "verified WASM provider member directory")?;
    let js_member = provider_member_path(precision, "js");
    let wasm_member = provider_member_path(precision, "wasm");
    let js = root.join(&js_member);
    let wasm = root.join(&wasm_member);
    write_new_file(&js, &files[&js_member], "verified provider JavaScript")?;
    write_new_file(&wasm, &files[&wasm_member], "verified provider WASM")?;
    Ok((js, wasm))
}

fn verify_node_version(node: &Path, expected: &str) -> Result<()> {
    let output = runtime_command(node)
        .arg("--version")
        .output()
        .map_err(|error| Error::io(node, error))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "Node.js version command failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let actual = String::from_utf8(output.stdout)
        .map_err(|error| Error::message(format!("Node.js version is not UTF-8: {error}")))?;
    if actual.trim() == format!("v{expected}") {
        Ok(())
    } else {
        Err(Error::message(format!(
            "authenticated WASM qualification requires Node.js {expected}; found {:?}",
            actual.trim()
        )))
    }
}

fn runtime_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    remove_process_injection_environment(&mut command);
    command.env_remove("NODE_OPTIONS").env_remove("NODE_PATH");
    command
}

fn npm_command(npm: &Path, node: &Path) -> Result<Command> {
    let node_directory = node
        .parent()
        .ok_or_else(|| Error::message("resolved Node.js executable has no parent directory"))?;
    let mut paths = vec![node_directory.to_path_buf()];
    if let Some(current) = env::var_os("PATH") {
        paths.extend(env::split_paths(&current).filter(|path| path != node_directory));
    }
    let path = env::join_paths(paths)
        .map_err(|error| Error::message(format!("failed to construct Node.js PATH: {error}")))?;
    let mut command = runtime_command(npm);
    command.env("PATH", path);
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn manifest_fixture() -> WasmRuntimeManifest {
        WasmRuntimeManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            schema: MANIFEST_SCHEMA.to_owned(),
            crate_version: "1.2.3".to_owned(),
            source_commit: "a".repeat(40),
            release_tag: "v1.2.3".to_owned(),
            provider_abi: PROVIDER_ABI.to_owned(),
            target: TARGET.to_owned(),
            compiler_target: COMPILER_TARGET.to_owned(),
            precision: "single".to_owned(),
            upstream_sha: "b".repeat(40),
            source_tree: "c".repeat(40),
            effective_source_sha256: "d".repeat(64),
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: "e".repeat(64),
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            validation_enabled: false,
            simd: SIMD_MODE.to_owned(),
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS.to_owned(),
            emscripten_sdk_contract_sha256: "1".repeat(64),
            wasm_provider_contract_sha256: "2".repeat(64),
            bindings_sha256: "3".repeat(64),
            private_abi_hash: "4".repeat(64),
            snapshot_layout_hash: 1,
            provider_js: RuntimeAsset {
                path: provider_member_path(ProviderPrecision::Single, "js"),
                size: 1,
                sha256: "5".repeat(64),
            },
            provider_wasm: RuntimeAsset {
                path: provider_member_path(ProviderPrecision::Single, "wasm"),
                size: 1,
                sha256: "6".repeat(64),
            },
        }
    }

    fn package_fixture(precision: ProviderPrecision) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::from([
            (BOX2D_LICENSE.to_owned(), b"Box2D license\n".to_vec()),
            (PROJECT_APACHE.to_owned(), b"Apache license\n".to_vec()),
            (PROJECT_MIT.to_owned(), b"MIT license\n".to_vec()),
            (RUNTIME_MANIFEST.to_owned(), b"manifest\n".to_vec()),
            (
                provider_member_path(precision, "js"),
                b"provider JavaScript\n".to_vec(),
            ),
            (
                provider_member_path(precision, "wasm"),
                b"provider WASM\n".to_vec(),
            ),
        ]);
        files.insert(
            INNER_CHECKSUMS.to_owned(),
            canonical_inner_checksums(&files).unwrap().into_bytes(),
        );
        files
    }

    fn statement_fixture(
        package: &[u8],
        files: &BTreeMap<String, Vec<u8>>,
    ) -> WasmReleaseProvenanceStatement {
        let members = members_from_files(files).unwrap();
        let statement = WasmReleaseProvenanceStatement {
            schema_version: PROVENANCE_SCHEMA_VERSION,
            schema: PROVENANCE_SCHEMA.to_owned(),
            repository: PUBLISHER_REPOSITORY.to_owned(),
            workflow: PUBLISHER_WORKFLOW.to_owned(),
            workflow_ref: format!("{PUBLISHER_REPOSITORY}/{PUBLISHER_WORKFLOW}@refs/tags/v1.2.3"),
            source_commit: "a".repeat(40),
            release_tag: "v1.2.3".to_owned(),
            run_id: "1".to_owned(),
            run_attempt: "1".to_owned(),
            crate_version: "1.2.3".to_owned(),
            package_type: PACKAGE_TYPE.to_owned(),
            package_name: "boxdd-wasm-provider-1.2.3-wasm32-unknown-unknown-single.tar.gz"
                .to_owned(),
            package_size: package.len() as u64,
            package_sha256: sha256_bytes(package),
            provider_abi: PROVIDER_ABI.to_owned(),
            target: TARGET.to_owned(),
            compiler_target: COMPILER_TARGET.to_owned(),
            precision: "single".to_owned(),
            upstream_sha: "b".repeat(40),
            source_tree: "c".repeat(40),
            effective_source_sha256: "d".repeat(64),
            adapter_abi_version: ADAPTER_ABI_VERSION,
            adapter_source_sha256: "e".repeat(64),
            recording_contract_blake3: RECORDING_CONTRACT_BLAKE3.to_owned(),
            validation_enabled: false,
            simd: SIMD_MODE.to_owned(),
            pointer_width: POINTER_WIDTH,
            endianness: ENDIANNESS.to_owned(),
            emscripten_sdk_contract_sha256: "1".repeat(64),
            wasm_provider_contract_sha256: "2".repeat(64),
            bindings_sha256: "3".repeat(64),
            private_abi_hash: "4".repeat(64),
            snapshot_layout_hash: 1,
            member_count: members.len() as u64,
            members,
        };
        statement.validate_intrinsic().unwrap();
        statement
    }

    fn render_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let encoder = GzBuilder::new()
            .mtime(0)
            .operating_system(255)
            .write(Vec::new(), Compression::best());
        let mut archive = Builder::new(encoder);
        for (path, bytes) in entries {
            let mut header = Header::new_gnu();
            header.set_entry_type(EntryType::Regular);
            header.set_path(path).unwrap();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_cksum();
            archive.append(&header, Cursor::new(*bytes)).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn command_options_are_closed_and_exact() {
        let build = BuildOptions::parse(&arguments(&[
            "--precision",
            "double",
            "--output",
            "packages",
        ]))
        .unwrap();
        assert_eq!(build.precision, ProviderPrecision::Double);
        assert_eq!(build.output, PathBuf::from("packages"));
        assert!(BuildOptions::parse(&arguments(&["--precision", "single"])).is_err());
        assert!(
            BuildOptions::parse(&arguments(&[
                "--precision",
                "single",
                "--precision",
                "double",
                "--output",
                "packages",
            ]))
            .is_err()
        );
        assert!(
            BuildOptions::parse(&arguments(&[
                "--precision",
                "single",
                "--output",
                "packages",
                "--unknown",
                "value",
            ]))
            .is_err()
        );

        let qualify = QualifyOptions::parse(&arguments(&[
            "--precision",
            "single",
            "--artifacts",
            "artifacts",
            "--cosign",
            "cosign",
        ]))
        .unwrap();
        assert_eq!(qualify.precision, ProviderPrecision::Single);
        assert_eq!(qualify.artifacts, PathBuf::from("artifacts"));
        assert_eq!(qualify.cosign, PathBuf::from("cosign"));
        assert!(QualifyOptions::parse(&arguments(&["--precision", "single"])).is_err());
    }

    #[test]
    fn versions_and_archive_names_are_canonical() {
        assert_eq!(
            archive_name("1.2.3-rc.1+build.7", "double").unwrap(),
            "boxdd-wasm-provider-1.2.3-rc.1+build.7-wasm32-unknown-unknown-double.tar.gz"
        );
        for invalid in ["", "1", "01.2.3", "1.02.3", "1.2.03", "1.2.3-"] {
            assert!(archive_name(invalid, "single").is_err(), "{invalid}");
        }
        assert!(archive_name("1.2.3", "quad").is_err());
    }

    #[test]
    fn runtime_manifest_requires_canonical_closed_schema() {
        let manifest = manifest_fixture();
        let rendered = manifest.render();
        assert_eq!(
            WasmRuntimeManifest::parse_canonical(rendered.as_bytes()).unwrap(),
            manifest
        );

        let unknown = format!("{rendered}unknown = true\n");
        assert!(WasmRuntimeManifest::parse_canonical(unknown.as_bytes()).is_err());
        let noncanonical = format!("\n{rendered}");
        assert!(WasmRuntimeManifest::parse_canonical(noncanonical.as_bytes()).is_err());

        let mut wrong_precision = manifest_fixture();
        wrong_precision.precision = "double".to_owned();
        assert!(wrong_precision.validate_intrinsic().is_err());
        let mut wrong_digest = manifest_fixture();
        wrong_digest.provider_js.sha256 = "A".repeat(64);
        assert!(wrong_digest.validate_intrinsic().is_err());
    }

    #[test]
    fn deterministic_archive_round_trips_and_rejects_header_drift() {
        let files = BTreeMap::from([
            ("a.txt".to_owned(), b"alpha\n".to_vec()),
            ("nested/b.txt".to_owned(), b"beta\n".to_vec()),
        ]);
        let first = render_archive(&files).unwrap();
        let second = render_archive(&files).unwrap();
        assert_eq!(first, second);
        assert_eq!(read_archive_bytes(&first).unwrap(), files);

        let mut noncanonical = first;
        noncanonical[4] = 1;
        let error = read_archive_bytes(&noncanonical).unwrap_err().to_string();
        assert!(error.contains("canonical deterministic"), "{error}");
    }

    #[test]
    fn archive_rejects_duplicate_members_and_unsafe_paths() {
        let archive = render_entries(&[("same", b"one"), ("same", b"two")]);
        let error = read_archive_bytes(&archive).unwrap_err().to_string();
        assert!(error.contains("duplicate member"), "{error}");
        for path in ["../escape", "/absolute", "a//b", "./a", "a/../b", "a\\b"] {
            assert!(validate_relative_path(path).is_err(), "{path}");
        }
    }

    #[test]
    fn signed_outer_identity_is_checked_before_tar_parsing() {
        let files = package_fixture(ProviderPrecision::Single);
        let package = render_archive(&files).unwrap();
        let statement = statement_fixture(&package, &files);
        let mut tampered = package;
        tampered[0] ^= 0xff;
        let error = verify_authenticated_archive(&statement, &tampered)
            .unwrap_err()
            .to_string();
        assert!(error.contains("package identity mismatch"), "{error}");
        assert!(error.contains("digest mismatch"), "{error}");
    }

    #[test]
    fn qualification_authenticates_before_interpreting_or_executing_package_bytes() {
        let source = include_str!("wasm_release.rs");
        let start = source.find("fn qualify_authenticated(").unwrap();
        let end = source[start..].find("\nfn qualification_context(").unwrap() + start;
        let body = &source[start..end];
        let position = |needle: &str| {
            body.find(needle)
                .unwrap_or_else(|| panic!("qualification source is missing {needle:?}"))
        };

        let signature = position("verify_signature(&cosign");
        let snapshot_revalidation = position("revalidate_snapshot(");
        let statement = position("parse_canonical_for_package(");
        let archive = position("verify_authenticated_archive(");
        let materialization = position("materialize_verified_provider(");
        let rust_consumer = position("prepare_existing_provider_smoke(");
        let node = position("run_existing_provider_node_smoke(");
        let browser = position("run_existing_provider_browser_smoke(");
        let terminal_revalidation = position("let revalidated = validate_package_files(");
        let session_release = position("drop(smoke);");
        assert!(signature < snapshot_revalidation);
        assert!(snapshot_revalidation < statement);
        assert!(statement < archive);
        assert!(archive < materialization);
        assert!(materialization < rust_consumer);
        assert!(rust_consumer < node);
        assert!(node < browser);
        assert!(browser < terminal_revalidation);
        assert!(terminal_revalidation < session_release);
        for forbidden in [
            "build_provider_smoke_only",
            "qualified_provider_sdk",
            "EMSDK",
        ] {
            assert!(
                !body.contains(forbidden),
                "authenticated qualification may not use {forbidden}"
            );
        }
    }

    #[test]
    fn signed_member_inventory_rejects_byte_and_path_additions() {
        let mut files = package_fixture(ProviderPrecision::Single);
        let package = render_archive(&files).unwrap();
        let statement = statement_fixture(&package, &files);
        files
            .get_mut(&provider_member_path(ProviderPrecision::Single, "js"))
            .unwrap()
            .push(b'!');
        assert!(statement.verify_members(&files).is_err());
        files.insert("unexpected".to_owned(), b"extra".to_vec());
        assert!(statement.verify_members(&files).is_err());
    }

    #[test]
    fn snapshots_are_bounded_and_create_new() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        fs::write(&source, b"immutable input").unwrap();
        let destination = temporary.path().join("snapshot");
        let snapshot = snapshot_bounded_regular_file(&source, &destination, 32, "test").unwrap();
        let expected = fs::read(&snapshot).unwrap();
        assert_eq!(expected, b"immutable input");
        revalidate_snapshot(&snapshot, &expected, 32, "test").unwrap();
        fs::write(&snapshot, b"modified input!").unwrap();
        assert!(revalidate_snapshot(&snapshot, &expected, 32, "test").is_err());
        assert!(snapshot_bounded_regular_file(&source, &destination, 32, "test").is_err());
        assert!(
            snapshot_bounded_regular_file(&source, &temporary.path().join("too-small"), 2, "test",)
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn snapshots_reject_symbolic_links() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let link = temporary.path().join("link");
        fs::write(&source, b"input").unwrap();
        symlink(&source, &link).unwrap();
        assert!(
            snapshot_bounded_regular_file(&link, &temporary.path().join("snapshot"), 32, "test",)
                .is_err()
        );
    }

    #[test]
    fn materialization_writes_only_authenticated_provider_members() {
        let files = package_fixture(ProviderPrecision::Single);
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("verified");
        let (js, wasm) =
            materialize_verified_provider(&root, &files, ProviderPrecision::Single).unwrap();
        assert_eq!(fs::read(js).unwrap(), b"provider JavaScript\n");
        assert_eq!(fs::read(wasm).unwrap(), b"provider WASM\n");
        let entries = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec![std::ffi::OsString::from("provider")]);
    }
}
