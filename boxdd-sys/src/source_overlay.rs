//! Deterministic, fail-closed overlays for reviewed upstream C sources.
//!
//! The Box2D submodule stays byte-for-byte at its reviewed Git revision. This module records the
//! small reviewed overlay separately, verifies its exact preimages, and derives one identity from
//! every source file that participates in the native build. Callers can therefore distinguish an
//! official upstream checkout from the actual source bytes sent to the compiler.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[allow(dead_code)]
pub(crate) const BUILD_POLICY_SOURCE_SHA256: &str =
    "6ed071421f1d32483c693f40c2f28f45f50bbb4bea448a524589422e07e1fb6b";

#[cfg(unix)]
use std::{
    ffi::CString,
    os::{
        raw::{c_char, c_int, c_uint},
        unix::ffi::OsStrExt,
    },
};

pub const EFFECTIVE_SOURCE_MANIFEST: &str = "effective-source.toml";
pub const EFFECTIVE_SOURCE_SCHEMA: &str = "boxdd-effective-source-v1";
pub const EFFECTIVE_SOURCE_SCHEMA_VERSION: u64 = 1;
pub const WORLD_SNAPSHOT_SOURCE: &str = "src/world_snapshot.c";

const EFFECTIVE_SOURCE_DOMAIN: &[u8] = b"boxdd.effective-source.v1\0";
const ADAPTER_SOURCE_DOMAIN: &[u8] = b"boxdd.adapter.sources.v1\0";
const BUILD_POLICY_SOURCE_DOMAIN: &[u8] = b"boxdd.build-policy-source.v1\0";
const BUILD_POLICY_DIGEST_BYTES: usize = 64;
const MATERIALIZED_SOURCE_DIRECTORY: &str = "boxdd-effective-source";
const MATERIALIZED_ADAPTER_SOURCE_DIRECTORY: &str = "boxdd-adapter-source";
const STAGING_ATTEMPTS: u64 = 128;
const SOURCE_ROOT: &str = "third-party/box2d";
const UPSTREAM_MANIFEST: &str = "upstream.toml";
const UPSTREAM_BACKPORT_REPOSITORY: &str = "https://github.com/erincatto/box2d.git";
const UPSTREAM_BACKPORT_COMMIT: &str = "c7a044a08d8e25511b7bce8d554cf5392a783497";

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub const ADAPTER_SOURCE_PATHS: &[&str] = &[
    "effective-source.toml",
    "native/boxdd_adapter.h",
    "native/boxdd_adapter.c",
    "native/boxdd_identity_values.c",
    "native/boxdd_private_abi.inl",
    "native/boxdd_recording_adapter.c",
    "native/boxdd_snapshot_layout.inl",
    "native/boxdd_snapshot_validate.c",
    "native/boxdd_wasm_runtime.js",
    "src/source_overlay.rs",
];

/// Build-script sources and manifests whose exact bytes define the running capture policy.
///
/// The build script embeds these files with `include_bytes!` and supplies them to
/// [`materialize_build_inputs`]. Keeping the closed inventory here lets the capture reject a
/// partially updated or stale build-script executable before it derives any artifact identity.
pub const BUILD_POLICY_SOURCE_PATHS: &[&str] = &[
    "Cargo.toml",
    "build.rs",
    "effective-source.toml",
    "src/bindgen_contract.rs",
    "src/build_support.rs",
    "src/prebuilt_provenance.rs",
    "src/precision.rs",
    "src/provenance_policy.rs",
    "src/provider_archive.rs",
    "src/provider_manifest.rs",
    "src/source_overlay.rs",
    "src/wasm_provider_contract.rs",
    "upstream.toml",
];

const ADAPTER_C_SOURCE_PATHS: &[&str] = &[
    "native/boxdd_adapter.c",
    "native/boxdd_recording_adapter.c",
    "native/boxdd_snapshot_validate.c",
];

const ADAPTER_IDENTITY_PROBE_SOURCE: &str = "native/boxdd_identity_values.c";

/// The full identity of the reviewed upstream tree after applying the declared overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveSourceIdentity {
    pub upstream_sha: String,
    pub source_tree: String,
    pub effective_source_sha256: String,
}

/// Compiler inputs and their complete effective-source identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedEffectiveSources {
    pub identity: EffectiveSourceIdentity,
    pub root: PathBuf,
    pub public_include: PathBuf,
    pub private_include: PathBuf,
    pub c_sources: Vec<PathBuf>,
}

/// Immutable repository-adapter bytes and the identity derived from those exact bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedAdapterSources {
    pub adapter_source_sha256: String,
    pub root: PathBuf,
    pub native_include: PathBuf,
    pub identity_probe_source: PathBuf,
    pub c_sources: Vec<PathBuf>,
}

/// One source file or manifest embedded in the currently executing build script.
#[derive(Clone, Copy, Debug)]
pub struct CompiledBuildPolicySource<'a> {
    relative_path: &'a str,
    bytes: &'a [u8],
    kind: CompiledBuildPolicySourceKind<'a>,
}

impl<'a> CompiledBuildPolicySource<'a> {
    /// Bind a Rust policy source to both its embedded bytes and compiled AST constant.
    pub const fn rust(
        relative_path: &'a str,
        bytes: &'a [u8],
        compiled_source_sha256: &'a str,
    ) -> Self {
        Self {
            relative_path,
            bytes,
            kind: CompiledBuildPolicySourceKind::Rust {
                compiled_source_sha256,
            },
        }
    }

    /// Bind a non-Rust manifest to its embedded bytes.
    pub const fn data(relative_path: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            relative_path,
            bytes,
            kind: CompiledBuildPolicySourceKind::Data,
        }
    }

    pub const fn relative_path(&self) -> &str {
        self.relative_path
    }
}

#[derive(Clone, Copy, Debug)]
enum CompiledBuildPolicySourceKind<'a> {
    Rust { compiled_source_sha256: &'a str },
    Data,
}

/// Declared and recomputed identity of one canonical Rust build-policy source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildPolicySourceIdentity {
    pub declared_sha256: String,
    pub normalized_sha256: String,
}

impl BuildPolicySourceIdentity {
    pub fn is_current(&self) -> bool {
        self.declared_sha256 == self.normalized_sha256
    }
}

/// Effective Box2D and repository-adapter compiler inputs captured as one source generation.
#[derive(Debug)]
pub struct MaterializedBuildInputs {
    pub effective: MaterializedEffectiveSources,
    pub adapter: MaterializedAdapterSources,
    pub vendored_source_sha256: String,
    manifest_root: PathBuf,
    live_files: Vec<CapturedLiveFile>,
    effective_files: Vec<PreparedSourceFile>,
    adapter_files: Vec<PreparedSourceFile>,
}

impl MaterializedBuildInputs {
    /// Revalidate both the live cohort and every byte exposed to a compiler.
    pub fn revalidate(&self) -> Result<(), String> {
        self.revalidate_live()?;
        self.revalidate_materialized()
    }

    /// Reject repository drift relative to the exact cohort captured by this build.
    pub fn revalidate_live(&self) -> Result<(), String> {
        revalidate_live_files(&self.manifest_root, &self.live_files)
    }

    /// Reject missing, extra, symlinked, or changed bytes in either materialized tree.
    pub fn revalidate_materialized(&self) -> Result<(), String> {
        validate_materialized_tree(&self.effective.root, &self.effective_files)?;
        validate_materialized_tree(&self.adapter.root, &self.adapter_files)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceRole {
    CSource,
    PrivateHeader,
    InlineFile,
    PublicHeader,
}

impl SourceRole {
    const ALL: [Self; 4] = [
        Self::CSource,
        Self::PrivateHeader,
        Self::InlineFile,
        Self::PublicHeader,
    ];

    const fn manifest_key(self) -> &'static str {
        match self {
            Self::CSource => "c_sources",
            Self::PrivateHeader => "private_headers",
            Self::InlineFile => "inline_files",
            Self::PublicHeader => "public_headers",
        }
    }

    const fn digest_name(self) -> &'static str {
        match self {
            Self::CSource => "c-source",
            Self::PrivateHeader => "private-header",
            Self::InlineFile => "inline-file",
            Self::PublicHeader => "public-header",
        }
    }

    const fn required_prefix(self) -> &'static str {
        match self {
            Self::CSource | Self::PrivateHeader | Self::InlineFile => "src/",
            Self::PublicHeader => "include/box2d/",
        }
    }

    const fn required_extension(self) -> &'static str {
        match self {
            Self::CSource => "c",
            Self::PrivateHeader | Self::PublicHeader => "h",
            Self::InlineFile => "inl",
        }
    }
}

#[derive(Clone, Copy)]
struct SourcePatch {
    id: &'static str,
    path: &'static str,
    classification: &'static str,
    preimage: &'static str,
    replacement: &'static str,
    origin: Option<UpstreamOrigin>,
}

#[derive(Clone, Copy)]
struct UpstreamOrigin {
    repository: &'static str,
    commit: &'static str,
    path: &'static str,
}

const WORLD_SNAPSHOT_PATCHES: &[SourcePatch] = &[
    SourcePatch {
        id: "world-snapshot-chain-shapes-zero-memset",
        path: WORLD_SNAPSHOT_SOURCE,
        classification: "local-soundness",
        preimage: "\t\t\tb2Array_Resize( world->chainShapes, chainCount );\n\t\t\t// Zero the whole array so free slots have NULL pointers\n\t\t\tmemset( world->chainShapes.data, 0, chainCount * sizeof( b2ChainShape ) );\n",
        replacement: "\t\t\tb2Array_Resize( world->chainShapes, chainCount );\n\t\t\t// Zero the whole array so free slots have NULL pointers\n\t\t\tif ( chainCount > 0 )\n\t\t\t{\n\t\t\t\tmemset( world->chainShapes.data, 0, chainCount * sizeof( b2ChainShape ) );\n\t\t\t}\n",
        origin: None,
    },
    SourcePatch {
        id: "world-snapshot-sensors-zero-memset",
        path: WORLD_SNAPSHOT_SOURCE,
        classification: "local-soundness",
        preimage: "\t\t\tb2Array_Resize( world->sensors, sensorCount );\n\t\t\t// Zero so inner array headers start clean\n\t\t\tmemset( world->sensors.data, 0, sensorCount * sizeof( b2Sensor ) );\n",
        replacement: "\t\t\tb2Array_Resize( world->sensors, sensorCount );\n\t\t\t// Zero so inner array headers start clean\n\t\t\tif ( sensorCount > 0 )\n\t\t\t{\n\t\t\t\tmemset( world->sensors.data, 0, sensorCount * sizeof( b2Sensor ) );\n\t\t\t}\n",
        origin: None,
    },
    SourcePatch {
        id: "world-snapshot-islands-zero-memset",
        path: WORLD_SNAPSHOT_SOURCE,
        classification: "local-soundness",
        preimage: "\t\t\tb2Array_Resize( world->islands, islandCount );\n\t\t\tmemset( world->islands.data, 0, islandCount * sizeof( b2Island ) );\n",
        replacement: "\t\t\tb2Array_Resize( world->islands, islandCount );\n\t\t\tif ( islandCount > 0 )\n\t\t\t{\n\t\t\t\tmemset( world->islands.data, 0, islandCount * sizeof( b2Island ) );\n\t\t\t}\n",
        origin: None,
    },
    SourcePatch {
        id: "world-snapshot-clear-transient-events",
        path: WORLD_SNAPSHOT_SOURCE,
        classification: "upstream-backport",
        preimage: "\t// Step 9: constraint graph\n\t{\n\t\tb2ConstraintGraph* graph = &world->constraintGraph;\n\t\tfor ( int c = 0; c < B2_GRAPH_COLOR_COUNT; ++c )\n\t\t{\n\t\t\tb2DesGraphColor( r, &graph->colors[c], c == B2_OVERFLOW_INDEX );\n\t\t}\n\t}\n\n\treturn r->ok;\n",
        replacement: "\t// Step 9: constraint graph\n\t{\n\t\tb2ConstraintGraph* graph = &world->constraintGraph;\n\t\tfor ( int c = 0; c < B2_GRAPH_COLOR_COUNT; ++c )\n\t\t{\n\t\t\tb2DesGraphColor( r, &graph->colors[c], c == B2_OVERFLOW_INDEX );\n\t\t}\n\t}\n\n\t// Event buffers are transient and never serialized. An in-place restore reuses the live\n\t// world's arrays, so end events queued by a between-step mutator (b2Body_Disable and friends)\n\t// would survive the restore and surface after the first resimmed step. Reset to match a fresh\n\t// world from snapshot. The double-buffer parity is restored, but the contents must start empty.\n\tb2Array_Clear( world->bodyMoveEvents );\n\tb2Array_Clear( world->sensorBeginEvents );\n\tb2Array_Clear( world->sensorEndEvents[0] );\n\tb2Array_Clear( world->sensorEndEvents[1] );\n\tb2Array_Clear( world->contactBeginEvents );\n\tb2Array_Clear( world->contactEndEvents[0] );\n\tb2Array_Clear( world->contactEndEvents[1] );\n\tb2Array_Clear( world->contactHitEvents );\n\tb2Array_Clear( world->jointEvents );\n\n\treturn r->ok;\n",
        origin: Some(UpstreamOrigin {
            repository: UPSTREAM_BACKPORT_REPOSITORY,
            commit: UPSTREAM_BACKPORT_COMMIT,
            path: WORLD_SNAPSHOT_SOURCE,
        }),
    },
];

#[derive(Debug)]
struct EffectiveSourceManifest {
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    transforms: Vec<DeclaredTransform>,
}

#[derive(Debug)]
struct DeclaredTransform {
    id: String,
    path: String,
    classification: String,
    preimage_sha256: String,
    replacement_sha256: String,
    origin_repository: Option<String>,
    origin_commit: Option<String>,
    origin_path: Option<String>,
}

#[derive(Debug)]
struct UpstreamInventory {
    active_revision: String,
    source_tree: String,
    entries: Vec<InventoryEntry>,
}

#[derive(Clone, Debug)]
struct InventoryEntry {
    role: SourceRole,
    normalized_path: String,
    relative_path: PathBuf,
}

#[derive(Clone, Debug)]
struct PreparedEffectiveSources {
    identity: EffectiveSourceIdentity,
    files: Vec<PreparedSourceFile>,
}

#[derive(Clone, Debug)]
struct PreparedSourceFile {
    role: SourceRole,
    normalized_path: String,
    relative_path: PathBuf,
    source_path: PathBuf,
    source_bytes: Vec<u8>,
    effective_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct CapturedLiveFile {
    normalized_path: String,
    relative_path: PathBuf,
    source_path: PathBuf,
    bytes: Vec<u8>,
    compiled_source_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct BuildPolicySourceField {
    value_start: usize,
    value_end: usize,
}

/// Return whether an entry in the closed build-policy inventory is Rust source.
pub fn is_rust_build_policy_source(relative_path: &str) -> bool {
    relative_path.ends_with(".rs") && BUILD_POLICY_SOURCE_PATHS.contains(&relative_path)
}

/// Parse and recompute the normalized identity of one Rust build-policy source.
pub fn build_policy_source_identity(
    relative_path: &str,
    source: &[u8],
) -> Result<BuildPolicySourceIdentity, String> {
    if !is_rust_build_policy_source(relative_path) {
        return Err(format!(
            "build policy source {relative_path:?} is not a Rust entry in the closed inventory"
        ));
    }
    if source.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(format!(
            "Rust build policy source {relative_path:?} must not contain a UTF-8 BOM"
        ));
    }
    if source.contains(&b'\r') {
        return Err(format!(
            "Rust build policy source {relative_path:?} must use canonical LF line endings"
        ));
    }

    let field = parse_build_policy_source_field(relative_path, source)?;
    let declared = std::str::from_utf8(&source[field.value_start..field.value_end])
        .expect("validated build policy digest is ASCII")
        .to_owned();
    let mut digest = Sha256::new();
    digest.update(BUILD_POLICY_SOURCE_DOMAIN);
    update_length_prefixed(&mut digest, relative_path.as_bytes());
    digest.update((source.len() as u64).to_le_bytes());
    digest.update(&source[..field.value_start]);
    digest.update([b'0'; BUILD_POLICY_DIGEST_BYTES]);
    digest.update(&source[field.value_end..]);

    Ok(BuildPolicySourceIdentity {
        declared_sha256: declared,
        normalized_sha256: hex_digest(digest.finalize()),
    })
}

/// Return canonical bytes with the unique self-hash field refreshed.
pub fn canonicalize_build_policy_source(
    relative_path: &str,
    source: &[u8],
) -> Result<Vec<u8>, String> {
    let field = parse_build_policy_source_field(relative_path, source)?;
    let identity = build_policy_source_identity(relative_path, source)?;
    let mut canonical = source.to_vec();
    canonical[field.value_start..field.value_end]
        .copy_from_slice(identity.normalized_sha256.as_bytes());
    Ok(canonical)
}

/// Validate the complete embedded/live policy cohort before any build-script side effect.
pub fn validate_compiled_build_policy_sources(
    manifest_dir: &Path,
    compiled_policy_sources: &[CompiledBuildPolicySource<'_>],
) -> Result<(), String> {
    let manifest_root = canonical_real_root(manifest_dir, "build policy root")?;
    let policy_files = capture_compiled_policy_sources(&manifest_root, compiled_policy_sources)?;
    let files = policy_files.into_values().collect::<Vec<_>>();
    revalidate_live_files(&manifest_root, &files)
}

fn parse_build_policy_source_field(
    relative_path: &str,
    source: &[u8],
) -> Result<BuildPolicySourceField, String> {
    if !is_rust_build_policy_source(relative_path) {
        return Err(format!(
            "build policy source {relative_path:?} is not a Rust entry in the closed inventory"
        ));
    }

    // Keep these fragments split so this parser does not create a second field in its own source.
    let mut anchor = b"const BUILD_POLICY_".to_vec();
    anchor.extend_from_slice(b"SOURCE_SHA256");
    let occurrences = source
        .windows(anchor.len())
        .enumerate()
        .filter_map(|(index, window)| (window == anchor).then_some(index))
        .collect::<Vec<_>>();
    if occurrences.len() != 1 {
        return Err(format!(
            "Rust build policy source {relative_path:?} must contain exactly one self-hash field; found {}",
            occurrences.len()
        ));
    }

    let mut prefix = b"pub(crate) const BUILD_POLICY_".to_vec();
    prefix.extend_from_slice(b"SOURCE_SHA256: &str =\n    \"");
    let anchor_start = occurrences[0];
    let declaration_start = anchor_start
        .checked_sub(b"pub(crate) ".len())
        .ok_or_else(|| {
            format!(
                "Rust build policy source {relative_path:?} self-hash field has a non-canonical prefix"
            )
        })?;
    if source.get(declaration_start..declaration_start + prefix.len()) != Some(prefix.as_slice())
        || (declaration_start != 0 && source[declaration_start - 1] != b'\n')
    {
        return Err(format!(
            "Rust build policy source {relative_path:?} self-hash field has a non-canonical prefix"
        ));
    }

    let value_start = declaration_start + prefix.len();
    let value_end = value_start
        .checked_add(BUILD_POLICY_DIGEST_BYTES)
        .ok_or_else(|| format!("Rust build policy source {relative_path:?} field overflow"))?;
    let value = source.get(value_start..value_end).ok_or_else(|| {
        format!(
            "Rust build policy source {relative_path:?} self-hash field is shorter than 64 bytes"
        )
    })?;
    if !value
        .iter()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!(
            "Rust build policy source {relative_path:?} self-hash field must be 64 lowercase hexadecimal bytes"
        ));
    }
    if source.get(value_end..value_end + 3) != Some(b"\";\n") {
        return Err(format!(
            "Rust build policy source {relative_path:?} self-hash field must end with the canonical `\";` suffix and LF"
        ));
    }

    Ok(BuildPolicySourceField {
        value_start,
        value_end,
    })
}

/// Validate the complete effective-source contract without writing an overlay.
#[allow(dead_code)] // Reused by build and provider adapters as their integration lands.
pub fn effective_source_identity(manifest_dir: &Path) -> Result<EffectiveSourceIdentity, String> {
    Ok(prepare_effective_sources(manifest_dir)?.identity)
}

/// Validate and materialize the complete reviewed source tree for native consumers.
#[allow(dead_code)] // Reused by build and provider adapters as their integration lands.
pub fn materialize_effective_box2d_sources(
    manifest_dir: &Path,
    output_dir: &Path,
) -> Result<MaterializedEffectiveSources, String> {
    let prepared = prepare_effective_sources(manifest_dir)?;
    materialize_prepared_sources(prepared, output_dir)
}

/// Identify every repository-owned adapter input from one captured byte set.
pub fn adapter_source_sha256(manifest_dir: &Path) -> Result<String, String> {
    let files = capture_adapter_sources(manifest_dir)?;
    Ok(captured_adapter_source_sha256(&files))
}

/// Capture and materialize every repository-owned adapter input before compilation.
pub fn materialize_adapter_sources(
    manifest_dir: &Path,
    output_dir: &Path,
) -> Result<MaterializedAdapterSources, String> {
    let files = capture_adapter_sources(manifest_dir)?;
    let adapter_source_sha256 = captured_adapter_source_sha256(&files);
    materialize_prepared_adapter_sources(files, adapter_source_sha256, output_dir)
}

/// Capture effective and adapter inputs as one generation and materialize only those bytes.
pub fn materialize_build_inputs(
    manifest_dir: &Path,
    output_dir: &Path,
    compiled_policy_sources: &[CompiledBuildPolicySource<'_>],
) -> Result<MaterializedBuildInputs, String> {
    let manifest_root = canonical_real_root(manifest_dir, "build input root")?;
    let policy_files = capture_compiled_policy_sources(&manifest_root, compiled_policy_sources)?;
    let upstream_manifest = captured_policy_file(&policy_files, UPSTREAM_MANIFEST)?;
    let effective_manifest = captured_policy_file(&policy_files, EFFECTIVE_SOURCE_MANIFEST)?;
    let prepared_effective = prepare_effective_sources_from_manifests(
        &manifest_root,
        &upstream_manifest.bytes,
        &effective_manifest.bytes,
    )?;
    let prepared_adapter = capture_adapter_sources_from_policy(&manifest_root, &policy_files)?;
    let live_files = captured_build_generation_files(
        &policy_files,
        &prepared_effective.files,
        &prepared_adapter,
    )?;

    // A second read closes the capture window before any compiler can consume the snapshots.
    revalidate_live_files(&manifest_root, &live_files)?;

    let effective_files = prepared_effective.files.clone();
    let adapter_files = prepared_adapter.clone();
    let vendored_source_sha256 = captured_vendored_source_sha256(&prepared_effective);
    let adapter_source_sha256 = captured_adapter_source_sha256(&prepared_adapter);
    let effective = materialize_prepared_sources(prepared_effective, output_dir)?;
    let adapter =
        materialize_prepared_adapter_sources(prepared_adapter, adapter_source_sha256, output_dir)?;
    let inputs = MaterializedBuildInputs {
        effective,
        adapter,
        vendored_source_sha256,
        manifest_root,
        live_files,
        effective_files,
        adapter_files,
    };
    inputs.revalidate()?;
    Ok(inputs)
}

fn prepare_effective_sources(manifest_dir: &Path) -> Result<PreparedEffectiveSources, String> {
    let upstream_path = manifest_dir.join(UPSTREAM_MANIFEST);
    let upstream_bytes = fs::read(&upstream_path).map_err(|error| {
        format!(
            "failed to read upstream manifest {}: {error}",
            upstream_path.display()
        )
    })?;
    let effective_path = manifest_dir.join(EFFECTIVE_SOURCE_MANIFEST);
    let effective_bytes = fs::read(&effective_path).map_err(|error| {
        format!(
            "failed to read effective source manifest {}: {error}",
            effective_path.display()
        )
    })?;
    prepare_effective_sources_from_manifests(manifest_dir, &upstream_bytes, &effective_bytes)
}

fn prepare_effective_sources_from_manifests(
    manifest_dir: &Path,
    upstream_manifest: &[u8],
    effective_manifest: &[u8],
) -> Result<PreparedEffectiveSources, String> {
    let upstream =
        parse_upstream_inventory(&manifest_dir.join(UPSTREAM_MANIFEST), upstream_manifest)?;
    let effective = parse_effective_source_manifest(
        &manifest_dir.join(EFFECTIVE_SOURCE_MANIFEST),
        effective_manifest,
    )?;
    validate_effective_source_manifest(&effective, &upstream)?;

    let source_root = manifest_dir.join(SOURCE_ROOT);
    let canonical_root = canonical_source_root(&source_root)?;
    let mut files = upstream
        .entries
        .iter()
        .map(|entry| prepare_source_file(&canonical_root, entry))
        .collect::<Result<Vec<_>, _>>()?;

    let target_indices = files
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            (file.role == SourceRole::CSource && file.normalized_path == WORLD_SNAPSHOT_SOURCE)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if target_indices.len() != 1 {
        return Err(format!(
            "{UPSTREAM_MANIFEST} source inventory must contain {WORLD_SNAPSHOT_SOURCE:?} exactly once; found {}",
            target_indices.len()
        ));
    }

    let target = &mut files[target_indices[0]];
    let upstream_source = String::from_utf8(target.source_bytes.clone()).map_err(|error| {
        format!(
            "reviewed upstream source {} is not UTF-8: {error}",
            target.source_path.display()
        )
    })?;
    target.effective_bytes = patch_world_snapshot_source(upstream_source)?.into_bytes();

    let identity = EffectiveSourceIdentity {
        upstream_sha: effective.upstream_sha,
        source_tree: effective.source_tree,
        effective_source_sha256: effective_source_sha256(
            &files,
            &upstream.active_revision,
            &upstream.source_tree,
        ),
    };
    if identity.effective_source_sha256 != effective.effective_source_sha256 {
        return Err(format!(
            "effective source SHA-256 {} does not match {EFFECTIVE_SOURCE_MANIFEST} value {}; update only through a reviewed overlay transaction",
            identity.effective_source_sha256, effective.effective_source_sha256
        ));
    }

    Ok(PreparedEffectiveSources { identity, files })
}

fn materialize_prepared_sources(
    prepared: PreparedEffectiveSources,
    output_dir: &Path,
) -> Result<MaterializedEffectiveSources, String> {
    let parent = prepare_materialized_parent(output_dir)?;
    let root = parent.join(&prepared.identity.effective_source_sha256);
    match fs::symlink_metadata(&root) {
        Ok(_) => {
            validate_materialized_tree(&root, &prepared.files)?;
            return materialized_sources(&prepared, root);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "failed to inspect effective source tree {}: {error}",
                root.display()
            ));
        }
    }

    let mut staging =
        create_staging_directory(&parent, &prepared.identity.effective_source_sha256)?;
    write_materialized_tree(staging.path(), &prepared.files)?;
    validate_materialized_tree(staging.path(), &prepared.files)?;

    match publish_staging_directory(staging.path(), &root)? {
        PublishOutcome::Published => staging.disarm(),
        PublishOutcome::Existing => {
            validate_materialized_tree(&root, &prepared.files)?;
        }
    }
    validate_materialized_tree(&root, &prepared.files)?;
    materialized_sources(&prepared, root)
}

fn capture_adapter_sources(manifest_dir: &Path) -> Result<Vec<PreparedSourceFile>, String> {
    capture_adapter_sources_from_policy(manifest_dir, &BTreeMap::new())
}

fn capture_adapter_sources_from_policy(
    manifest_dir: &Path,
    policy_files: &BTreeMap<String, CapturedLiveFile>,
) -> Result<Vec<PreparedSourceFile>, String> {
    let canonical_root = canonical_real_root(manifest_dir, "adapter source root")?;
    ADAPTER_SOURCE_PATHS
        .iter()
        .map(|relative_path| {
            let normalized_path =
                validate_normalized_path(relative_path, "adapter source", None, None)?;
            let relative_path = PathBuf::from(&normalized_path);
            let (source_path, source_bytes) =
                if let Some(compiled) = policy_files.get(&normalized_path) {
                    (compiled.source_path.clone(), compiled.bytes.clone())
                } else {
                    let source_path = resolve_regular_file(
                        &canonical_root,
                        &relative_path,
                        &normalized_path,
                        "adapter source",
                    )?;
                    let source_bytes = fs::read(&source_path).map_err(|error| {
                        format!(
                            "failed to read adapter source {normalized_path:?} at {}: {error}",
                            source_path.display()
                        )
                    })?;
                    (source_path, source_bytes)
                };
            let role = if ADAPTER_C_SOURCE_PATHS.contains(&normalized_path.as_str()) {
                SourceRole::CSource
            } else {
                SourceRole::InlineFile
            };
            Ok(PreparedSourceFile {
                role,
                normalized_path,
                relative_path,
                source_path,
                effective_bytes: source_bytes.clone(),
                source_bytes,
            })
        })
        .collect()
}

fn capture_compiled_policy_sources(
    manifest_root: &Path,
    compiled_sources: &[CompiledBuildPolicySource<'_>],
) -> Result<BTreeMap<String, CapturedLiveFile>, String> {
    if compiled_sources.len() != BUILD_POLICY_SOURCE_PATHS.len() {
        return Err(format!(
            "compiled build policy inventory has {} entries; expected {}",
            compiled_sources.len(),
            BUILD_POLICY_SOURCE_PATHS.len()
        ));
    }

    let mut captured = BTreeMap::new();
    for (index, (expected, compiled)) in BUILD_POLICY_SOURCE_PATHS
        .iter()
        .zip(compiled_sources)
        .enumerate()
    {
        if compiled.relative_path != *expected {
            return Err(format!(
                "compiled build policy entry #{index} is {:?}; expected {:?}",
                compiled.relative_path, expected
            ));
        }
        let normalized =
            validate_normalized_path(compiled.relative_path, "compiled build policy", None, None)?;
        let compiled_source_sha256 = match (is_rust_build_policy_source(&normalized), compiled.kind)
        {
            (
                true,
                CompiledBuildPolicySourceKind::Rust {
                    compiled_source_sha256,
                },
            ) => Some(compiled_source_sha256),
            (false, CompiledBuildPolicySourceKind::Data) => None,
            (true, CompiledBuildPolicySourceKind::Data) => {
                return Err(format!(
                    "Rust build policy source {normalized:?} has no AST-compiled self-hash"
                ));
            }
            (false, CompiledBuildPolicySourceKind::Rust { .. }) => {
                return Err(format!(
                    "non-Rust build policy input {normalized:?} must use byte identity only"
                ));
            }
        };
        if let Some(compiled_source_sha256) = compiled_source_sha256 {
            validate_build_policy_source_generation(
                &normalized,
                compiled.bytes,
                compiled_source_sha256,
                "embedded",
            )?;
        }
        let relative_path = PathBuf::from(&normalized);
        let source_path = resolve_regular_file(
            manifest_root,
            &relative_path,
            &normalized,
            "compiled build policy",
        )?;
        let live_bytes = fs::read(&source_path).map_err(|error| {
            format!(
                "failed to read compiled build policy {normalized:?} at {}: {error}",
                source_path.display()
            )
        })?;
        if let Some(compiled_source_sha256) = compiled_source_sha256 {
            validate_build_policy_source_generation(
                &normalized,
                &live_bytes,
                compiled_source_sha256,
                "live",
            )?;
        }
        if live_bytes != compiled.bytes {
            return Err(format!(
                "build policy {normalized:?} differs from the bytes compiled into the running build script; rebuild before producing an artifact"
            ));
        }
        let previous = captured.insert(
            normalized.clone(),
            CapturedLiveFile {
                normalized_path: normalized,
                relative_path,
                source_path,
                bytes: compiled.bytes.to_vec(),
                compiled_source_sha256: compiled_source_sha256.map(str::to_owned),
            },
        );
        if previous.is_some() {
            return Err("compiled build policy inventory contains a duplicate path".to_owned());
        }
    }
    Ok(captured)
}

fn captured_policy_file<'a>(
    policy_files: &'a BTreeMap<String, CapturedLiveFile>,
    relative_path: &str,
) -> Result<&'a CapturedLiveFile, String> {
    policy_files.get(relative_path).ok_or_else(|| {
        format!("compiled build policy inventory is missing required file {relative_path:?}")
    })
}

fn captured_build_generation_files(
    policy_files: &BTreeMap<String, CapturedLiveFile>,
    effective_files: &[PreparedSourceFile],
    adapter_files: &[PreparedSourceFile],
) -> Result<Vec<CapturedLiveFile>, String> {
    let mut generation = policy_files.clone();
    for file in effective_files {
        let normalized_path = format!("{SOURCE_ROOT}/{}", file.normalized_path);
        insert_generation_file(
            &mut generation,
            CapturedLiveFile {
                relative_path: PathBuf::from(&normalized_path),
                normalized_path,
                source_path: file.source_path.clone(),
                bytes: file.source_bytes.clone(),
                compiled_source_sha256: None,
            },
        )?;
    }
    for file in adapter_files {
        insert_generation_file(
            &mut generation,
            CapturedLiveFile {
                normalized_path: file.normalized_path.clone(),
                relative_path: file.relative_path.clone(),
                source_path: file.source_path.clone(),
                bytes: file.source_bytes.clone(),
                compiled_source_sha256: None,
            },
        )?;
    }
    Ok(generation.into_values().collect())
}

fn insert_generation_file(
    generation: &mut BTreeMap<String, CapturedLiveFile>,
    candidate: CapturedLiveFile,
) -> Result<(), String> {
    if let Some(existing) = generation.get(&candidate.normalized_path) {
        if existing.relative_path != candidate.relative_path
            || existing.source_path != candidate.source_path
            || existing.bytes != candidate.bytes
        {
            return Err(format!(
                "overlapping build input {:?} was captured from different bytes or paths",
                candidate.normalized_path
            ));
        }
        return Ok(());
    }
    generation.insert(candidate.normalized_path.clone(), candidate);
    Ok(())
}

fn revalidate_live_files(manifest_root: &Path, files: &[CapturedLiveFile]) -> Result<(), String> {
    for file in files {
        if let Some(compiled_source_sha256) = &file.compiled_source_sha256 {
            validate_build_policy_source_generation(
                &file.normalized_path,
                &file.bytes,
                compiled_source_sha256,
                "embedded",
            )?;
        }
        let current_path = resolve_regular_file(
            manifest_root,
            &file.relative_path,
            &file.normalized_path,
            "captured build input",
        )?;
        if current_path != file.source_path {
            return Err(format!(
                "captured build input {:?} resolved to {} instead of {}",
                file.normalized_path,
                current_path.display(),
                file.source_path.display()
            ));
        }
        let current = fs::read(&current_path).map_err(|error| {
            format!(
                "failed to re-read captured build input {:?} at {}: {error}",
                file.normalized_path,
                current_path.display()
            )
        })?;
        if let Some(compiled_source_sha256) = &file.compiled_source_sha256 {
            validate_build_policy_source_generation(
                &file.normalized_path,
                &current,
                compiled_source_sha256,
                "live",
            )?;
        }
        if current != file.bytes {
            return Err(format!(
                "captured build input {:?} changed after its build generation was captured",
                file.normalized_path
            ));
        }
    }
    Ok(())
}

fn validate_build_policy_source_generation(
    relative_path: &str,
    source: &[u8],
    compiled_source_sha256: &str,
    generation: &str,
) -> Result<(), String> {
    validate_sha256(
        &format!("AST-compiled build policy self-hash for {relative_path:?}"),
        compiled_source_sha256,
    )?;
    let identity = build_policy_source_identity(relative_path, source)?;
    if !identity.is_current() {
        return Err(format!(
            "{generation} Rust build policy source {relative_path:?} declares self-hash {} but its normalized bytes require {}",
            identity.declared_sha256, identity.normalized_sha256
        ));
    }
    if identity.normalized_sha256 != compiled_source_sha256 {
        return Err(format!(
            "{generation} Rust build policy source {relative_path:?} has normalized self-hash {} but the executing AST compiled {compiled_source_sha256}",
            identity.normalized_sha256
        ));
    }
    Ok(())
}

fn captured_adapter_source_sha256(files: &[PreparedSourceFile]) -> String {
    debug_assert_eq!(files.len(), ADAPTER_SOURCE_PATHS.len());
    let mut digest = Sha256::new();
    digest.update(ADAPTER_SOURCE_DOMAIN);
    for file in files {
        update_length_prefixed(&mut digest, file.normalized_path.as_bytes());
        update_length_prefixed(&mut digest, &file.effective_bytes);
    }
    hex_digest(digest.finalize())
}

fn captured_vendored_source_sha256(prepared: &PreparedEffectiveSources) -> String {
    let mut files = prepared.files.iter().collect::<Vec<_>>();
    files.sort_by(|left, right| left.normalized_path.cmp(&right.normalized_path));
    let mut digest = Sha256::new();
    digest.update(b"boxdd.vendored-source-identity.v1\0");
    update_length_prefixed(&mut digest, prepared.identity.upstream_sha.as_bytes());
    update_length_prefixed(&mut digest, prepared.identity.source_tree.as_bytes());
    digest.update((files.len() as u64).to_le_bytes());
    for file in files {
        update_length_prefixed(&mut digest, file.normalized_path.as_bytes());
        update_length_prefixed(&mut digest, &file.source_bytes);
    }
    hex_digest(digest.finalize())
}

fn materialize_prepared_adapter_sources(
    files: Vec<PreparedSourceFile>,
    adapter_source_sha256: String,
    output_dir: &Path,
) -> Result<MaterializedAdapterSources, String> {
    let parent = prepare_materialized_parent_named(
        output_dir,
        MATERIALIZED_ADAPTER_SOURCE_DIRECTORY,
        "adapter source",
    )?;
    let root = parent.join(&adapter_source_sha256);
    match fs::symlink_metadata(&root) {
        Ok(_) => {
            validate_materialized_tree(&root, &files)?;
            return materialized_adapter_sources(files, adapter_source_sha256, root);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "failed to inspect adapter source tree {}: {error}",
                root.display()
            ));
        }
    }

    let mut staging = create_staging_directory(&parent, &adapter_source_sha256)?;
    write_materialized_tree(staging.path(), &files)?;
    validate_materialized_tree(staging.path(), &files)?;

    match publish_staging_directory(staging.path(), &root)? {
        PublishOutcome::Published => staging.disarm(),
        PublishOutcome::Existing => validate_materialized_tree(&root, &files)?,
    }
    validate_materialized_tree(&root, &files)?;
    materialized_adapter_sources(files, adapter_source_sha256, root)
}

fn materialized_adapter_sources(
    files: Vec<PreparedSourceFile>,
    adapter_source_sha256: String,
    root: PathBuf,
) -> Result<MaterializedAdapterSources, String> {
    validate_materialized_tree(&root, &files)?;
    let native_include = root.join("native");
    ensure_real_directory(&native_include, "adapter source include directory")?;
    let identity_probe_source = root.join(ADAPTER_IDENTITY_PROBE_SOURCE);
    let c_sources = ADAPTER_C_SOURCE_PATHS
        .iter()
        .map(|relative_path| root.join(relative_path))
        .collect::<Vec<_>>();
    if !identity_probe_source.is_file() || c_sources.iter().any(|source| !source.is_file()) {
        return Err(format!(
            "adapter source tree {} is missing a compiler input",
            root.display()
        ));
    }
    Ok(MaterializedAdapterSources {
        adapter_source_sha256,
        root,
        native_include,
        identity_probe_source,
        c_sources,
    })
}

fn materialized_sources(
    prepared: &PreparedEffectiveSources,
    root: PathBuf,
) -> Result<MaterializedEffectiveSources, String> {
    validate_materialized_tree(&root, &prepared.files)?;
    let public_include = root.join("include");
    let private_include = root.join("src");
    ensure_real_directory(&public_include, "effective public include directory")?;
    ensure_real_directory(&private_include, "effective private include directory")?;
    let c_sources = prepared
        .files
        .iter()
        .filter(|file| file.role == SourceRole::CSource)
        .map(|file| root.join(&file.relative_path))
        .collect();
    Ok(MaterializedEffectiveSources {
        identity: prepared.identity.clone(),
        root,
        public_include,
        private_include,
        c_sources,
    })
}

fn prepare_materialized_parent(output_dir: &Path) -> Result<PathBuf, String> {
    prepare_materialized_parent_named(
        output_dir,
        MATERIALIZED_SOURCE_DIRECTORY,
        "effective source",
    )
}

fn prepare_materialized_parent_named(
    output_dir: &Path,
    directory_name: &str,
    label: &str,
) -> Result<PathBuf, String> {
    ensure_real_directory(output_dir, &format!("{label} output directory"))?;
    let parent = output_dir.join(directory_name);
    match fs::create_dir(&parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(format!(
                "failed to create {label} parent {}: {error}",
                parent.display()
            ));
        }
    }
    ensure_real_directory(&parent, &format!("{label} parent"))?;
    Ok(parent)
}

fn ensure_real_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!(
            "{label} {} must be a real non-symlink directory",
            path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if metadata.mode() & 0o022 != 0 {
            return Err(format!(
                "{label} {} must not be group- or world-writable",
                path.display()
            ));
        }
    }
    Ok(())
}

struct StagingDirectory {
    path: PathBuf,
    armed: bool,
}

impl StagingDirectory {
    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = remove_owned_staging_directory(&self.path);
        }
    }
}

fn create_staging_directory(parent: &Path, digest: &str) -> Result<StagingDirectory, String> {
    for _ in 0..STAGING_ATTEMPTS {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".{digest}.staging-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                let staging = StagingDirectory { path, armed: true };
                ensure_real_directory(staging.path(), "effective source staging directory")?;
                return Ok(staging);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create effective source staging directory {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "could not allocate a unique effective source staging directory below {}",
        parent.display()
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublishOutcome {
    Published,
    Existing,
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn publish_staging_directory(staging: &Path, root: &Path) -> Result<PublishOutcome, String> {
    publish_staging_directory_linux(staging, root)
}

#[cfg(target_os = "macos")]
fn publish_staging_directory(staging: &Path, root: &Path) -> Result<PublishOutcome, String> {
    publish_staging_directory_macos(staging, root)
}

#[cfg(windows)]
fn publish_staging_directory(staging: &Path, root: &Path) -> Result<PublishOutcome, String> {
    match fs::rename(staging, root) {
        Ok(()) => Ok(PublishOutcome::Published),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Ok(PublishOutcome::Existing)
        }
        Err(error) => Err(format!(
            "failed to publish effective source tree {} without replacement: {error}",
            root.display()
        )),
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    windows
)))]
fn publish_staging_directory(staging: &Path, root: &Path) -> Result<PublishOutcome, String> {
    let _ = staging;
    let _ = root;
    Err("effective source materialization requires a no-replace directory publish primitive on this platform".to_owned())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn publish_staging_directory_linux(staging: &Path, root: &Path) -> Result<PublishOutcome, String> {
    unsafe extern "C" {
        fn renameat2(
            olddirfd: c_int,
            oldpath: *const c_char,
            newdirfd: c_int,
            newpath: *const c_char,
            flags: c_uint,
        ) -> c_int;
    }

    const AT_FDCWD: c_int = -100;
    const RENAME_NOREPLACE: c_uint = 1;
    let staging = c_path(staging, "effective source staging directory")?;
    let root = c_path(root, "effective source destination directory")?;
    // RENAME_NOREPLACE provides the publication guarantee that std::fs::rename lacks on Unix.
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            staging.as_ptr(),
            AT_FDCWD,
            root.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    publish_result(result, root.as_c_str().to_string_lossy().as_ref())
}

#[cfg(target_os = "macos")]
fn publish_staging_directory_macos(staging: &Path, root: &Path) -> Result<PublishOutcome, String> {
    unsafe extern "C" {
        fn renamex_np(from: *const c_char, to: *const c_char, flags: c_uint) -> c_int;
    }

    const RENAME_EXCL: c_uint = 0x0000_0004;
    let staging = c_path(staging, "effective source staging directory")?;
    let root = c_path(root, "effective source destination directory")?;
    // RENAME_EXCL publishes only when the destination name does not already exist.
    let result = unsafe { renamex_np(staging.as_ptr(), root.as_ptr(), RENAME_EXCL) };
    publish_result(result, root.as_c_str().to_string_lossy().as_ref())
}

#[cfg(unix)]
fn c_path(path: &Path, label: &str) -> Result<CString, String> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        format!(
            "{label} {} contains an unsupported NUL byte",
            path.display()
        )
    })
}

#[cfg(unix)]
fn publish_result(result: c_int, destination: &str) -> Result<PublishOutcome, String> {
    if result == 0 {
        return Ok(PublishOutcome::Published);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        return Ok(PublishOutcome::Existing);
    }
    Err(format!(
        "failed to publish effective source tree {destination} without replacement: {error}"
    ))
}

fn remove_owned_staging_directory(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "failed to inspect owned effective source staging directory {}: {error}",
                path.display()
            ));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        fs::remove_file(path).map_err(|error| {
            format!(
                "failed to remove owned effective source staging path {}: {error}",
                path.display()
            )
        })
    } else {
        fs::remove_dir_all(path).map_err(|error| {
            format!(
                "failed to remove owned effective source staging directory {}: {error}",
                path.display()
            )
        })
    }
}

fn write_materialized_tree(root: &Path, files: &[PreparedSourceFile]) -> Result<(), String> {
    for file in files {
        create_materialized_parent_directories(root, &file.relative_path)?;
        let path = root.join(&file.relative_path);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                format!(
                    "failed to create effective source file {}: {error}",
                    path.display()
                )
            })?;
        output.write_all(&file.effective_bytes).map_err(|error| {
            format!(
                "failed to write effective source file {}: {error}",
                path.display()
            )
        })?;
        output.sync_all().map_err(|error| {
            format!(
                "failed to sync effective source file {}: {error}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn create_materialized_parent_directories(root: &Path, relative_path: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    let Some(parent) = relative_path.parent() else {
        return Ok(());
    };
    for component in parent.components() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "effective source path {} is not normalized",
                relative_path.display()
            ));
        };
        current.push(component);
        match fs::create_dir(&current) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "failed to create effective source directory {}: {error}",
                    current.display()
                ));
            }
        }
        ensure_real_directory(&current, "effective source directory")?;
    }
    Ok(())
}

fn validate_materialized_tree(root: &Path, files: &[PreparedSourceFile]) -> Result<(), String> {
    ensure_real_directory(root, "effective source tree")?;
    let expected_files = files
        .iter()
        .map(|file| (file.relative_path.clone(), file.effective_bytes.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut expected_directories = BTreeSet::new();
    for path in expected_files.keys() {
        let mut current = PathBuf::new();
        if let Some(parent) = path.parent() {
            for component in parent.components() {
                let Component::Normal(component) = component else {
                    return Err(format!(
                        "effective source inventory path {} is not normalized",
                        path.display()
                    ));
                };
                current.push(component);
                expected_directories.insert(current.clone());
            }
        }
    }
    let mut seen = BTreeSet::new();
    validate_materialized_directory(
        root,
        Path::new(""),
        &expected_files,
        &expected_directories,
        &mut seen,
    )?;
    if let Some(missing) = expected_files.keys().find(|path| !seen.contains(*path)) {
        return Err(format!(
            "effective source tree {} is missing expected file {}",
            root.display(),
            missing.display()
        ));
    }
    Ok(())
}

fn validate_materialized_directory(
    root: &Path,
    relative: &Path,
    expected_files: &BTreeMap<PathBuf, &[u8]>,
    expected_directories: &BTreeSet<PathBuf>,
    seen: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let directory = root.join(relative);
    for entry in fs::read_dir(&directory).map_err(|error| {
        format!(
            "failed to read effective source directory {}: {error}",
            directory.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in effective source directory {}: {error}",
                directory.display()
            )
        })?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            format!(
                "effective source tree {} contains a non-UTF-8 path",
                root.display()
            )
        })?;
        let child_relative = relative.join(name);
        let child = root.join(&child_relative);
        let metadata = fs::symlink_metadata(&child).map_err(|error| {
            format!(
                "failed to inspect effective source path {}: {error}",
                child.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "effective source tree {} contains a symlink at {}",
                root.display(),
                child_relative.display()
            ));
        }
        if metadata.file_type().is_dir() {
            if !expected_directories.contains(&child_relative) {
                return Err(format!(
                    "effective source tree {} contains unexpected directory {}",
                    root.display(),
                    child_relative.display()
                ));
            }
            validate_materialized_directory(
                root,
                &child_relative,
                expected_files,
                expected_directories,
                seen,
            )?;
            continue;
        }
        if !metadata.file_type().is_file() {
            return Err(format!(
                "effective source tree {} contains a non-regular path at {}",
                root.display(),
                child_relative.display()
            ));
        }
        let expected = expected_files.get(&child_relative).ok_or_else(|| {
            format!(
                "effective source tree {} contains unexpected file {}",
                root.display(),
                child_relative.display()
            )
        })?;
        let actual = fs::read(&child).map_err(|error| {
            format!(
                "failed to read effective source file {}: {error}",
                child.display()
            )
        })?;
        if actual.as_slice() != *expected {
            return Err(format!(
                "effective source tree {} has byte drift at {}",
                root.display(),
                child_relative.display()
            ));
        }
        if !seen.insert(child_relative.clone()) {
            return Err(format!(
                "effective source tree {} contains duplicate path {}",
                root.display(),
                child_relative.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn load_effective_source_manifest(manifest_dir: &Path) -> Result<EffectiveSourceManifest, String> {
    let path = manifest_dir.join(EFFECTIVE_SOURCE_MANIFEST);
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read effective source manifest {}: {error}",
            path.display()
        )
    })?;
    parse_effective_source_manifest(&path, &bytes)
}

fn parse_effective_source_manifest(
    path: &Path,
    bytes: &[u8],
) -> Result<EffectiveSourceManifest, String> {
    let value = parse_toml_bytes(path, bytes, "effective source manifest")?;
    let table = value
        .as_table()
        .ok_or_else(|| format!("{} root must be a TOML table", path.display()))?;
    reject_unknown_fields(
        table,
        "effective source manifest",
        &[
            "schema_version",
            "schema",
            "upstream_sha",
            "source_tree",
            "effective_source_sha256",
            "transforms",
        ],
    )?;
    let transforms = required_array(table, "transforms", "effective source manifest")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let table = value.as_table().ok_or_else(|| {
                format!("effective source transform #{index} must be a TOML table")
            })?;
            parse_declared_transform(table, index)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let schema_version = required_integer(table, "schema_version", "effective source manifest")?;
    let schema = required_string(table, "schema", "effective source manifest")?;
    if schema_version != EFFECTIVE_SOURCE_SCHEMA_VERSION || schema != EFFECTIVE_SOURCE_SCHEMA {
        return Err(format!(
            "unsupported effective source manifest schema: version={schema_version} name={schema:?}"
        ));
    }

    Ok(EffectiveSourceManifest {
        upstream_sha: required_string(table, "upstream_sha", "effective source manifest")?,
        source_tree: required_string(table, "source_tree", "effective source manifest")?,
        effective_source_sha256: required_string(
            table,
            "effective_source_sha256",
            "effective source manifest",
        )?,
        transforms,
    })
}

fn parse_declared_transform(
    table: &toml::map::Map<String, toml::Value>,
    index: usize,
) -> Result<DeclaredTransform, String> {
    const FIELDS: &[&str] = &[
        "id",
        "path",
        "classification",
        "preimage_sha256",
        "replacement_sha256",
        "origin_repository",
        "origin_commit",
        "origin_path",
    ];
    let label = format!("effective source transform #{index}");
    reject_unknown_fields(table, &label, FIELDS)?;
    let classification = required_string(table, "classification", &label)?;
    let has_origin = ["origin_repository", "origin_commit", "origin_path"]
        .iter()
        .any(|field| table.contains_key(*field));
    let (origin_repository, origin_commit, origin_path) = match classification.as_str() {
        "local-soundness" => {
            if has_origin {
                return Err(format!(
                    "{label} local-soundness transform must not declare upstream origin fields"
                ));
            }
            (None, None, None)
        }
        "upstream-backport" => (
            Some(required_string(table, "origin_repository", &label)?),
            Some(required_string(table, "origin_commit", &label)?),
            Some(required_string(table, "origin_path", &label)?),
        ),
        value => {
            return Err(format!(
                "{label} has unsupported classification {value:?}; expected local-soundness or upstream-backport"
            ));
        }
    };
    Ok(DeclaredTransform {
        id: required_string(table, "id", &label)?,
        path: required_string(table, "path", &label)?,
        classification,
        preimage_sha256: required_string(table, "preimage_sha256", &label)?,
        replacement_sha256: required_string(table, "replacement_sha256", &label)?,
        origin_repository,
        origin_commit,
        origin_path,
    })
}

fn validate_effective_source_manifest(
    effective: &EffectiveSourceManifest,
    upstream: &UpstreamInventory,
) -> Result<(), String> {
    validate_git_sha("effective source upstream_sha", &effective.upstream_sha)?;
    validate_git_sha("effective source source_tree", &effective.source_tree)?;
    validate_sha256(
        "effective source effective_source_sha256",
        &effective.effective_source_sha256,
    )?;
    if effective.upstream_sha != upstream.active_revision {
        return Err(format!(
            "effective source manifest upstream SHA {} does not match {UPSTREAM_MANIFEST} active_revision {}",
            effective.upstream_sha, upstream.active_revision
        ));
    }
    if effective.source_tree != upstream.source_tree {
        return Err(format!(
            "effective source manifest source tree {} does not match {UPSTREAM_MANIFEST} source_inventory.tree {}",
            effective.source_tree, upstream.source_tree
        ));
    }
    if effective.transforms.len() != WORLD_SNAPSHOT_PATCHES.len() {
        return Err(format!(
            "effective source manifest must declare exactly {} ordered transforms; found {}",
            WORLD_SNAPSHOT_PATCHES.len(),
            effective.transforms.len()
        ));
    }
    let mut transform_ids = BTreeSet::new();
    for transform in &effective.transforms {
        if !transform_ids.insert(transform.id.as_str()) {
            return Err(format!(
                "effective source manifest contains duplicate transform id {:?}",
                transform.id
            ));
        }
    }

    for (index, (declared, expected)) in effective
        .transforms
        .iter()
        .zip(WORLD_SNAPSHOT_PATCHES)
        .enumerate()
    {
        if declared.id != expected.id {
            return Err(format!(
                "effective source transform #{index} has id {:?}, expected {:?}; transforms must remain in reviewed order",
                declared.id, expected.id
            ));
        }
        validate_normalized_path(
            &declared.path,
            "effective source transform path",
            None,
            None,
        )?;
        if declared.path != expected.path {
            return Err(format!(
                "effective source transform {} targets {:?}, expected {:?}",
                declared.id, declared.path, expected.path
            ));
        }
        if declared.classification != expected.classification {
            return Err(format!(
                "effective source transform {} classification {:?} does not match {:?}",
                declared.id, declared.classification, expected.classification
            ));
        }
        validate_sha256(
            &format!("effective source transform {} preimage_sha256", declared.id),
            &declared.preimage_sha256,
        )?;
        validate_sha256(
            &format!(
                "effective source transform {} replacement_sha256",
                declared.id
            ),
            &declared.replacement_sha256,
        )?;
        let expected_preimage = sha256_bytes(expected.preimage.as_bytes());
        let expected_replacement = sha256_bytes(expected.replacement.as_bytes());
        if declared.preimage_sha256 != expected_preimage {
            return Err(format!(
                "effective source transform {} preimage_sha256 {} does not match reviewed preimage {}",
                declared.id, declared.preimage_sha256, expected_preimage
            ));
        }
        if declared.replacement_sha256 != expected_replacement {
            return Err(format!(
                "effective source transform {} replacement_sha256 {} does not match reviewed replacement {}",
                declared.id, declared.replacement_sha256, expected_replacement
            ));
        }
        match (
            expected.origin,
            &declared.origin_repository,
            &declared.origin_commit,
            &declared.origin_path,
        ) {
            (None, None, None, None) => {}
            (Some(origin), Some(repository), Some(commit), Some(path)) => {
                validate_normalized_path(
                    path,
                    "effective source upstream origin path",
                    None,
                    None,
                )?;
                if repository != origin.repository || commit != origin.commit || path != origin.path
                {
                    return Err(format!(
                        "effective source transform {} upstream origin does not match the reviewed backport",
                        declared.id
                    ));
                }
            }
            (None, _, _, _) => {
                return Err(format!(
                    "effective source transform {} local-soundness origin fields are inconsistent",
                    declared.id
                ));
            }
            (Some(_), _, _, _) => {
                return Err(format!(
                    "effective source transform {} upstream-backport must declare all origin fields",
                    declared.id
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
fn load_upstream_inventory(manifest_dir: &Path) -> Result<UpstreamInventory, String> {
    let path = manifest_dir.join(UPSTREAM_MANIFEST);
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "failed to read upstream manifest {}: {error}",
            path.display()
        )
    })?;
    parse_upstream_inventory(&path, &bytes)
}

fn parse_upstream_inventory(path: &Path, bytes: &[u8]) -> Result<UpstreamInventory, String> {
    let value = parse_toml_bytes(path, bytes, "upstream manifest")?;
    let root = value
        .as_table()
        .ok_or_else(|| format!("{} root must be a TOML table", path.display()))?;
    let active_revision = required_string(root, "active_revision", "upstream manifest")?;
    validate_git_sha("upstream active_revision", &active_revision)?;
    let inventory = root
        .get("source_inventory")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{} source_inventory must be a TOML table", path.display()))?;
    reject_unknown_fields(
        inventory,
        "upstream source_inventory",
        &[
            "tree",
            "c_sources",
            "private_headers",
            "inline_files",
            "public_headers",
        ],
    )?;
    let source_tree = required_string(inventory, "tree", "upstream source_inventory")?;
    validate_git_sha("upstream source_inventory.tree", &source_tree)?;

    let mut entries = Vec::new();
    let mut all_paths = BTreeSet::new();
    for role in SourceRole::ALL {
        let key = role.manifest_key();
        let values = required_string_array(inventory, key, "upstream source_inventory")?;
        if values.is_empty() {
            return Err(format!("upstream source_inventory.{key} must not be empty"));
        }
        let mut previous = None::<String>;
        for value in values {
            let normalized = validate_inventory_path(role, &value)?;
            if !all_paths.insert(normalized.clone()) {
                return Err(format!(
                    "upstream source_inventory contains duplicate path {normalized:?}"
                ));
            }
            if let Some(previous) = &previous
                && previous >= &normalized
            {
                return Err(format!(
                    "upstream source_inventory.{key} must be strictly sorted; {previous:?} precedes {normalized:?}"
                ));
            }
            previous = Some(normalized.clone());
            entries.push(InventoryEntry {
                role,
                relative_path: PathBuf::from(&normalized),
                normalized_path: normalized,
            });
        }
    }
    Ok(UpstreamInventory {
        active_revision,
        source_tree,
        entries,
    })
}

fn prepare_source_file(
    canonical_root: &Path,
    entry: &InventoryEntry,
) -> Result<PreparedSourceFile, String> {
    let source_path =
        resolve_inventory_file(canonical_root, &entry.relative_path, &entry.normalized_path)?;
    let source_bytes = fs::read(&source_path).map_err(|error| {
        format!(
            "failed to read vendored source {:?} at {}: {error}",
            entry.normalized_path,
            source_path.display()
        )
    })?;
    Ok(PreparedSourceFile {
        role: entry.role,
        normalized_path: entry.normalized_path.clone(),
        relative_path: entry.relative_path.clone(),
        source_path,
        effective_bytes: source_bytes.clone(),
        source_bytes,
    })
}

fn canonical_source_root(source_root: &Path) -> Result<PathBuf, String> {
    canonical_real_root(source_root, "vendored source root")
}

fn canonical_real_root(root: &Path, label: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", root.display()))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} {} must be a real non-symlink directory",
            root.display()
        ));
    }
    fs::canonicalize(root)
        .map_err(|error| format!("failed to resolve {label} {}: {error}", root.display()))
}

fn resolve_inventory_file(
    canonical_root: &Path,
    relative_path: &Path,
    normalized_path: &str,
) -> Result<PathBuf, String> {
    resolve_regular_file(
        canonical_root,
        relative_path,
        normalized_path,
        "vendored source",
    )
}

fn resolve_regular_file(
    canonical_root: &Path,
    relative_path: &Path,
    normalized_path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let mut current = canonical_root.to_path_buf();
    for component in relative_path.components() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "{label} {normalized_path:?} is not a normalized relative path"
            ));
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            format!(
                "failed to inspect {label} {normalized_path:?} at {}: {error}",
                current.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "{label} {normalized_path:?} must not traverse a symlink"
            ));
        }
    }
    let metadata = fs::symlink_metadata(&current).map_err(|error| {
        format!(
            "failed to inspect {label} {normalized_path:?} at {}: {error}",
            current.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{label} {normalized_path:?} must be a regular non-symlink file"
        ));
    }
    let canonical = fs::canonicalize(&current).map_err(|error| {
        format!(
            "failed to resolve {label} {normalized_path:?} at {}: {error}",
            current.display()
        )
    })?;
    if !canonical.starts_with(canonical_root) {
        return Err(format!(
            "{label} {normalized_path:?} escapes {}",
            canonical_root.display()
        ));
    }
    Ok(canonical)
}

fn patch_world_snapshot_source(mut source: String) -> Result<String, String> {
    for patch in WORLD_SNAPSHOT_PATCHES {
        let replacement_occurrences = source.matches(patch.replacement).count();
        if replacement_occurrences != 0 {
            return Err(format!(
                "reviewed world snapshot patch {} found {replacement_occurrences} existing replacement blocks",
                patch.id
            ));
        }
        let occurrences = source.matches(patch.preimage).count();
        if occurrences != 1 {
            return Err(format!(
                "reviewed world snapshot patch {} expected one exact preimage; found {occurrences}",
                patch.id
            ));
        }
        source = source.replacen(patch.preimage, patch.replacement, 1);
    }
    Ok(source)
}

fn effective_source_sha256(
    files: &[PreparedSourceFile],
    upstream_sha: &str,
    source_tree: &str,
) -> String {
    let mut ordered = files.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        (left.role.digest_name(), left.normalized_path.as_str())
            .cmp(&(right.role.digest_name(), right.normalized_path.as_str()))
    });
    let mut digest = Sha256::new();
    digest.update(EFFECTIVE_SOURCE_DOMAIN);
    update_length_prefixed(&mut digest, upstream_sha.as_bytes());
    update_length_prefixed(&mut digest, source_tree.as_bytes());
    digest.update((ordered.len() as u64).to_le_bytes());
    for file in ordered {
        update_length_prefixed(&mut digest, file.role.digest_name().as_bytes());
        update_length_prefixed(&mut digest, file.normalized_path.as_bytes());
        update_length_prefixed(&mut digest, &file.effective_bytes);
    }
    hex_digest(digest.finalize())
}

fn validate_inventory_path(role: SourceRole, value: &str) -> Result<String, String> {
    validate_normalized_path(
        value,
        &format!("upstream source_inventory.{}", role.manifest_key()),
        Some(role.required_prefix()),
        Some(role.required_extension()),
    )
}

fn validate_normalized_path(
    value: &str,
    label: &str,
    required_prefix: Option<&str>,
    required_extension: Option<&str>,
) -> Result<String, String> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.contains("//")
        || value.starts_with("./")
        || value.ends_with('/')
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{label} path {value:?} is not normalized and relative"
        ));
    }
    if let Some(prefix) = required_prefix
        && !value.starts_with(prefix)
    {
        return Err(format!("{label} path {value:?} must begin with {prefix:?}"));
    }
    if let Some(extension) = required_extension
        && path.extension().and_then(|value| value.to_str()) != Some(extension)
    {
        return Err(format!(
            "{label} path {value:?} must have .{extension} extension"
        ));
    }
    Ok(value.to_owned())
}

fn parse_toml_bytes(path: &Path, bytes: &[u8], label: &str) -> Result<toml::Value, String> {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| format!("{label} {} is not UTF-8: {error}", path.display()))?;
    toml::from_str(source)
        .map_err(|error| format!("failed to parse {label} {}: {error}", path.display()))
}

fn required_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    label: &str,
) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{label} field `{key}` must be a string"))
}

fn required_integer(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    label: &str,
) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| format!("{label} field `{key}` must be a non-negative integer"))
}

fn required_array<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
    label: &str,
) -> Result<&'a Vec<toml::Value>, String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("{label} field `{key}` must be an array"))
}

fn required_string_array(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    label: &str,
) -> Result<Vec<String>, String> {
    required_array(table, key, label)?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{label} field `{key}` entry #{index} must be a string"))
        })
        .collect()
}

fn reject_unknown_fields(
    table: &toml::map::Map<String, toml::Value>,
    label: &str,
    allowed: &[&str],
) -> Result<(), String> {
    if let Some(field) = table
        .keys()
        .filter(|field| !allowed.contains(&field.as_str()))
        .min()
    {
        return Err(format!("{label} has unknown field `{field}`"));
    }
    Ok(())
}

fn validate_git_sha(label: &str, value: &str) -> Result<(), String> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} must be a lowercase 40-character Git SHA"));
    }
    Ok(())
}

fn validate_sha256(label: &str, value: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} must be a lowercase 64-character SHA-256"));
    }
    Ok(())
}

fn update_length_prefixed(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn repository_manifest_dir() -> PathBuf {
        let host_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if is_repository_manifest_dir(&host_manifest_dir) {
            return host_manifest_dir;
        }

        for ancestor in host_manifest_dir.ancestors().skip(1) {
            let candidate = ancestor.join("boxdd-sys");
            if is_repository_manifest_dir(&candidate) {
                return candidate;
            }
        }

        panic!(
            "could not locate boxdd-sys manifests from host crate {}",
            host_manifest_dir.display()
        );
    }

    fn is_repository_manifest_dir(path: &Path) -> bool {
        path.join(UPSTREAM_MANIFEST).is_file() && path.join(EFFECTIVE_SOURCE_MANIFEST).is_file()
    }

    fn fixture() -> TempDir {
        let directory = tempfile::tempdir().unwrap();
        let repository = repository_manifest_dir();
        fs::copy(
            repository.join(UPSTREAM_MANIFEST),
            directory.path().join(UPSTREAM_MANIFEST),
        )
        .unwrap();
        fs::copy(
            repository.join(EFFECTIVE_SOURCE_MANIFEST),
            directory.path().join(EFFECTIVE_SOURCE_MANIFEST),
        )
        .unwrap();

        let inventory = load_upstream_inventory(&repository).unwrap();
        for entry in inventory.entries {
            let source = repository.join(SOURCE_ROOT).join(&entry.relative_path);
            let destination = directory
                .path()
                .join(SOURCE_ROOT)
                .join(&entry.relative_path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(source, destination).unwrap();
        }
        directory
    }

    fn adapter_fixture(root: &Path) {
        let repository = repository_manifest_dir();
        for relative in ADAPTER_SOURCE_PATHS {
            let source = repository.join(relative);
            let destination = root.join(relative);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(source, destination).unwrap();
        }
    }

    fn build_input_fixture() -> TempDir {
        let directory = fixture();
        let repository = repository_manifest_dir();
        for relative in BUILD_POLICY_SOURCE_PATHS.iter().chain(ADAPTER_SOURCE_PATHS) {
            let source = repository.join(relative);
            let destination = directory.path().join(relative);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(source, destination).unwrap();
        }
        directory
    }

    struct CompiledPolicyFixture {
        bytes: Vec<Vec<u8>>,
        source_sha256: Vec<Option<String>>,
    }

    impl CompiledPolicyFixture {
        fn capture(root: &Path) -> Self {
            let bytes = BUILD_POLICY_SOURCE_PATHS
                .iter()
                .map(|relative| fs::read(root.join(relative)).unwrap())
                .collect::<Vec<_>>();
            let source_sha256 = BUILD_POLICY_SOURCE_PATHS
                .iter()
                .zip(&bytes)
                .map(|(relative_path, source)| {
                    is_rust_build_policy_source(relative_path).then(|| {
                        build_policy_source_identity(relative_path, source)
                            .unwrap()
                            .declared_sha256
                    })
                })
                .collect();
            Self {
                bytes,
                source_sha256,
            }
        }

        fn sources(&self) -> Vec<CompiledBuildPolicySource<'_>> {
            BUILD_POLICY_SOURCE_PATHS
                .iter()
                .zip(&self.bytes)
                .zip(&self.source_sha256)
                .map(
                    |((relative_path, bytes), source_sha256)| match source_sha256 {
                        Some(source_sha256) => {
                            CompiledBuildPolicySource::rust(relative_path, bytes, source_sha256)
                        }
                        None => CompiledBuildPolicySource::data(relative_path, bytes),
                    },
                )
                .collect()
        }
    }

    fn policy_source_with_hash(hash: &str, suffix: &[u8], body: &[u8]) -> Vec<u8> {
        let mut source = b"pub(crate) const BUILD_POLICY_".to_vec();
        source.extend_from_slice(b"SOURCE_SHA256: &str =\n    \"");
        source.extend_from_slice(hash.as_bytes());
        source.extend_from_slice(suffix);
        source.extend_from_slice(body);
        source
    }

    fn declared_external_policy_modules(source: &str, label: &str) -> BTreeSet<String> {
        let mut paths = BTreeSet::new();
        let mut pending_path = None::<String>;
        for line in source.lines() {
            let line = line.trim();
            if let Some(path) = line
                .strip_prefix("#[path = \"")
                .and_then(|path| path.strip_suffix("\"]"))
            {
                assert!(pending_path.is_none(), "nested path attribute in {label}");
                pending_path = Some(if path.starts_with("src/") {
                    path.to_owned()
                } else {
                    format!("src/{path}")
                });
                continue;
            }
            let external_mod = line
                .strip_prefix("mod ")
                .or_else(|| line.strip_prefix("pub(crate) mod "))
                .and_then(|module| module.strip_suffix(';'));
            if let Some(module) = external_mod {
                let path = pending_path
                    .take()
                    .unwrap_or_else(|| panic!("bare external module {module:?} in {label}"));
                assert!(
                    paths.insert(path),
                    "duplicate external module path in {label}"
                );
            } else if pending_path.is_some() && !line.is_empty() && !line.starts_with("#[") {
                panic!("unpaired path attribute in {label}");
            }
        }
        assert!(pending_path.is_none(), "trailing path attribute in {label}");
        paths
    }

    fn read_effective_manifest(directory: &TempDir) -> String {
        fs::read_to_string(directory.path().join(EFFECTIVE_SOURCE_MANIFEST)).unwrap()
    }

    fn write_effective_manifest(directory: &TempDir, value: String) {
        fs::write(directory.path().join(EFFECTIVE_SOURCE_MANIFEST), value).unwrap();
    }

    fn write_upstream_manifest(directory: &TempDir, value: String) {
        fs::write(directory.path().join(UPSTREAM_MANIFEST), value).unwrap();
    }

    fn replace_once(value: String, from: &str, to: &str) -> String {
        assert!(
            value.contains(from),
            "missing expected fixture text {from:?}"
        );
        value.replacen(from, to, 1)
    }

    fn mutate_hash_field(value: String, field: &str) -> String {
        let prefix = format!("{field} = \"");
        let start = value.find(&prefix).unwrap() + prefix.len();
        let end = start + 64;
        let original = &value[start..end];
        assert!(original.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let leading = if &original[..1] == "0" { "1" } else { "0" };
        let replacement = format!("{leading}{}", &original[1..]);
        format!("{}{}{}", &value[..start], replacement, &value[end..])
    }

    fn remove_last_transform(value: String) -> String {
        let index = value.rfind("\n[[transforms]]").unwrap();
        value[..index].to_owned()
    }

    #[test]
    fn pinned_effective_source_identity_and_materialized_tree_match_the_reviewed_contract() {
        let repository = repository_manifest_dir();
        let identity = effective_source_identity(&repository).unwrap();
        let manifest = load_effective_source_manifest(&repository).unwrap();
        let prepared = prepare_effective_sources(&repository).unwrap();
        assert_eq!(
            identity.upstream_sha,
            "56edae79f2949d86142b03450d5d60f63bcf5a6f"
        );
        assert_eq!(
            identity.source_tree,
            "63a1ab02e3d2bf7c4d86b257b78976842b8c5ddb"
        );
        assert_eq!(
            identity.effective_source_sha256,
            "9948291f4ea6e14b01304d19473e4539f47313133b4c2e7c6f3ae312d4f2c112"
        );
        assert_eq!(
            identity.effective_source_sha256,
            manifest.effective_source_sha256
        );

        let output = tempfile::tempdir().unwrap();
        let materialized = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        assert_eq!(materialized.identity, identity);
        assert_eq!(materialized.identity, prepared.identity);
        assert_eq!(
            materialized.public_include,
            materialized.root.join("include")
        );
        assert_eq!(materialized.private_include, materialized.root.join("src"));
        assert_eq!(
            materialized.c_sources.len(),
            prepared
                .files
                .iter()
                .filter(|file| file.role == SourceRole::CSource)
                .count()
        );
        for file in &prepared.files {
            let path = materialized.root.join(&file.relative_path);
            assert!(path.is_file(), "missing materialized {}", path.display());
            assert_eq!(fs::read(&path).unwrap(), file.effective_bytes);
        }
        assert!(
            materialized
                .c_sources
                .iter()
                .all(|source| source.starts_with(&materialized.root))
        );
        let world_snapshot = materialized
            .c_sources
            .iter()
            .find(|path| path.ends_with("world_snapshot.c"))
            .unwrap();
        let patched = fs::read_to_string(world_snapshot).unwrap();
        for patch in WORLD_SNAPSHOT_PATCHES {
            assert_eq!(patched.matches(patch.preimage).count(), 0, "{}", patch.id);
            assert_eq!(
                patched.matches(patch.replacement).count(),
                1,
                "{}",
                patch.id
            );
        }
        let backport = WORLD_SNAPSHOT_PATCHES.last().unwrap();
        assert!(patched.contains(backport.replacement));
        assert!(!patched.contains("bool restored = b2DeserializeIntoShell"));
        assert!(!patched.contains("b2World_GetStateHash( b2WorldId worldId )"));
    }

    #[test]
    fn materialized_tree_is_reused_without_rewrite() {
        let repository = repository_manifest_dir();
        let output = tempfile::tempdir().unwrap();
        let first = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let tracked = first.root.join("src/aabb.c");
        let modified = fs::metadata(&tracked).unwrap().modified().unwrap();

        let second = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        assert_eq!(second, first);
        assert_eq!(
            fs::metadata(&tracked).unwrap().modified().unwrap(),
            modified
        );
    }

    #[test]
    fn concurrent_materializers_publish_one_complete_tree() {
        use std::{
            sync::{Arc, Barrier},
            thread,
        };

        let repository = repository_manifest_dir();
        let output = tempfile::tempdir().unwrap();
        let barrier = Arc::new(Barrier::new(4));
        let workers = (0..4)
            .map(|_| {
                let repository = repository.clone();
                let output = output.path().to_path_buf();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    materialize_effective_box2d_sources(&repository, &output)
                })
            })
            .collect::<Vec<_>>();
        let materialized = workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap())
            .collect::<Vec<_>>();
        let first = materialized.first().unwrap();
        for sources in &materialized {
            assert_eq!(sources, first);
        }
        validate_materialized_tree(
            &first.root,
            &prepare_effective_sources(&repository).unwrap().files,
        )
        .unwrap();
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        windows
    ))]
    #[test]
    fn publication_never_replaces_an_existing_empty_directory() {
        let output = tempfile::tempdir().unwrap();
        let parent = prepare_materialized_parent(output.path()).unwrap();
        let digest = "a".repeat(64);
        let staging = create_staging_directory(&parent, &digest).unwrap();
        let existing = parent.join("existing");
        fs::create_dir(&existing).unwrap();

        assert_eq!(
            publish_staging_directory(staging.path(), &existing).unwrap(),
            PublishOutcome::Existing
        );
        assert!(staging.path().is_dir());
        assert!(existing.is_dir());
        assert!(fs::read_dir(&existing).unwrap().next().is_none());
    }

    #[test]
    fn preexisting_empty_or_non_directory_targets_are_rejected_without_repair() {
        let repository = repository_manifest_dir();
        let identity = effective_source_identity(&repository).unwrap();

        let output = tempfile::tempdir().unwrap();
        let parent = output.path().join(MATERIALIZED_SOURCE_DIRECTORY);
        fs::create_dir(&parent).unwrap();
        let empty = parent.join(&identity.effective_source_sha256);
        fs::create_dir(&empty).unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("missing expected file"));
        assert!(empty.is_dir());
        assert!(fs::read_dir(&empty).unwrap().next().is_none());

        let output = tempfile::tempdir().unwrap();
        let parent = output.path().join(MATERIALIZED_SOURCE_DIRECTORY);
        fs::create_dir(&parent).unwrap();
        let file = parent.join(&identity.effective_source_sha256);
        fs::write(&file, "not a source tree\n").unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("must be a real non-symlink directory"));
        assert_eq!(fs::read_to_string(&file).unwrap(), "not a source tree\n");
    }

    #[cfg(unix)]
    #[test]
    fn materialization_rejects_symlinked_or_unsafe_output_directories() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let repository = repository_manifest_dir();
        let output = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let parent = output.path().join(MATERIALIZED_SOURCE_DIRECTORY);
        symlink(external.path(), &parent).unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("must be a real non-symlink directory"));
        assert!(
            fs::symlink_metadata(&parent)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        let output = tempfile::tempdir().unwrap();
        let mut permissions = fs::metadata(output.path()).unwrap().permissions();
        permissions.set_mode(0o770);
        fs::set_permissions(output.path(), permissions).unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("must not be group- or world-writable"));
        assert!(!output.path().join(MATERIALIZED_SOURCE_DIRECTORY).exists());
    }

    #[test]
    fn materialized_tree_remains_usable_after_the_source_tree_is_unavailable() {
        let directory = fixture();
        let output = tempfile::tempdir().unwrap();
        let materialized =
            materialize_effective_box2d_sources(directory.path(), output.path()).unwrap();
        let source_root = directory.path().join(SOURCE_ROOT);
        let unavailable = directory.path().join("source-tree-unavailable");
        fs::rename(&source_root, &unavailable).unwrap();
        assert!(!source_root.exists());

        for source in &materialized.c_sources {
            assert!(
                source.is_file(),
                "missing materialized C source {}",
                source.display()
            );
            assert!(!fs::read(source).unwrap().is_empty());
        }
        assert!(materialized.public_include.join("box2d/box2d.h").is_file());
        assert!(
            materialized
                .private_include
                .join("world_snapshot.c")
                .is_file()
        );
    }

    #[test]
    fn adapter_snapshot_survives_live_source_drift() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("output");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&output).unwrap();
        adapter_fixture(&source);

        let first = materialize_adapter_sources(&source, &output).unwrap();
        let first_digest = first.adapter_source_sha256.clone();
        let first_adapter = fs::read(first.root.join("native/boxdd_adapter.c")).unwrap();
        let first_probe = fs::read(&first.identity_probe_source).unwrap();
        let first_private_abi = fs::read(first.root.join("native/boxdd_private_abi.inl")).unwrap();

        fs::write(
            source.join("native/boxdd_adapter.c"),
            b"mutated adapter source\n",
        )
        .unwrap();
        fs::write(
            source.join("native/boxdd_identity_values.c"),
            b"mutated identity probe\n",
        )
        .unwrap();
        fs::write(
            source.join("native/boxdd_private_abi.inl"),
            b"mutated private ABI\n",
        )
        .unwrap();

        assert_eq!(adapter_source_sha256(&first.root).unwrap(), first_digest);
        assert_eq!(
            fs::read(first.root.join("native/boxdd_adapter.c")).unwrap(),
            first_adapter
        );
        assert_eq!(fs::read(&first.identity_probe_source).unwrap(), first_probe);
        assert_eq!(
            fs::read(first.root.join("native/boxdd_private_abi.inl")).unwrap(),
            first_private_abi
        );
        assert!(
            first
                .c_sources
                .iter()
                .all(|path| path.starts_with(&first.root))
        );

        let second = materialize_adapter_sources(&source, &output).unwrap();
        assert_ne!(second.adapter_source_sha256, first_digest);
        assert_ne!(second.root, first.root);

        let unavailable = directory.path().join("source-unavailable");
        fs::rename(&source, &unavailable).unwrap();
        assert_eq!(
            fs::read(first.root.join("native/boxdd_adapter.c")).unwrap(),
            first_adapter
        );
        assert_eq!(fs::read(&first.identity_probe_source).unwrap(), first_probe);
    }

    #[test]
    fn build_policy_self_hash_matches_known_answer() {
        let source = policy_source_with_hash(
            &"0".repeat(64),
            b"\";\n",
            b"pub fn answer() -> u32 {\n    42\n}\n",
        );
        let identity = build_policy_source_identity("src/precision.rs", &source).unwrap();
        assert_eq!(
            identity.normalized_sha256,
            "341466a04f22d9a8c3ea752e5d85e19669ac947104c25906a357467c5ef8926d"
        );
    }

    #[test]
    fn checked_in_rust_build_policy_sources_are_canonical() {
        let repository = repository_manifest_dir();
        let rust_paths = BUILD_POLICY_SOURCE_PATHS
            .iter()
            .copied()
            .filter(|path| is_rust_build_policy_source(path))
            .collect::<Vec<_>>();
        assert_eq!(rust_paths.len(), 10);
        assert!(rust_paths.windows(2).all(|pair| pair[0] < pair[1]));

        for relative_path in rust_paths {
            let source = fs::read(repository.join(relative_path)).unwrap();
            let identity = build_policy_source_identity(relative_path, &source).unwrap();
            assert!(identity.is_current(), "stale self-hash in {relative_path}");
            assert_eq!(
                canonicalize_build_policy_source(relative_path, &source).unwrap(),
                source,
                "non-canonical self-hash in {relative_path}"
            );
        }
    }

    #[test]
    fn build_policy_inventory_covers_executed_external_modules() {
        let repository = repository_manifest_dir();
        let expected = BUILD_POLICY_SOURCE_PATHS
            .iter()
            .copied()
            .filter(|path| is_rust_build_policy_source(path))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let mut discovered = BTreeSet::from(["build.rs".to_owned()]);
        let mut pending = vec!["build.rs".to_owned()];
        while let Some(relative_path) = pending.pop() {
            let source = fs::read_to_string(repository.join(&relative_path)).unwrap();
            for dependency in declared_external_policy_modules(&source, &relative_path) {
                assert!(
                    expected.contains(&dependency),
                    "{relative_path} executes untracked policy module {dependency}"
                );
                if discovered.insert(dependency.clone()) {
                    pending.push(dependency);
                }
            }
        }
        assert_eq!(discovered, expected);
    }

    #[test]
    fn build_policy_parser_rejects_duplicate_and_illegal_fields() {
        let valid = "0".repeat(64);
        let mut duplicate = policy_source_with_hash(&valid, b"\";\n", b"");
        duplicate.extend_from_slice(&policy_source_with_hash(&valid, b"\";\n", b""));
        let error = build_policy_source_identity("src/precision.rs", &duplicate).unwrap_err();
        assert!(
            error.contains("exactly one self-hash field; found 2"),
            "{error}"
        );

        let uppercase = policy_source_with_hash(&"A".repeat(64), b"\";\n", b"");
        let error = build_policy_source_identity("src/precision.rs", &uppercase).unwrap_err();
        assert!(error.contains("64 lowercase hexadecimal bytes"), "{error}");

        let bad_suffix = policy_source_with_hash(&valid, b"\"; \n", b"");
        let error = build_policy_source_identity("src/precision.rs", &bad_suffix).unwrap_err();
        assert!(error.contains("canonical `\";` suffix"), "{error}");
    }

    #[test]
    fn build_policy_parser_rejects_bom_and_crlf() {
        let valid = "0".repeat(64);
        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend_from_slice(&policy_source_with_hash(&valid, b"\";\n", b""));
        let error = build_policy_source_identity("src/precision.rs", &bom).unwrap_err();
        assert!(error.contains("UTF-8 BOM"), "{error}");

        let crlf = policy_source_with_hash(&valid, b"\";\n", b"fn value() {}\r\n");
        let error = build_policy_source_identity("src/precision.rs", &crlf).unwrap_err();
        assert!(error.contains("canonical LF"), "{error}");
    }

    #[test]
    fn build_input_capture_rejects_stale_ast_digest_with_matching_live_bytes() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let mut compiled = CompiledPolicyFixture::capture(directory.path());
        let index = BUILD_POLICY_SOURCE_PATHS
            .iter()
            .position(|path| *path == "src/source_overlay.rs")
            .unwrap();
        let stale = {
            let current = compiled.source_sha256[index].as_ref().unwrap();
            let replacement = if current.starts_with('0') { '1' } else { '0' };
            format!("{replacement}{}", &current[1..])
        };
        compiled.source_sha256[index] = Some(stale);
        let compiled_sources = compiled.sources();

        let error = materialize_build_inputs(directory.path(), output.path(), &compiled_sources)
            .unwrap_err();
        assert!(error.contains("src/source_overlay.rs"), "{error}");
        assert!(error.contains("executing AST compiled"), "{error}");
    }

    #[test]
    fn build_policy_inventory_rejects_rust_data_kind_mismatch() {
        let directory = build_input_fixture();
        let compiled = CompiledPolicyFixture::capture(directory.path());
        let mut sources = compiled.sources();
        let rust_index = BUILD_POLICY_SOURCE_PATHS
            .iter()
            .position(|path| *path == "build.rs")
            .unwrap();
        assert_eq!(sources[rust_index].relative_path(), "build.rs");
        sources[rust_index] =
            CompiledBuildPolicySource::data("build.rs", &compiled.bytes[rust_index]);
        let error = validate_compiled_build_policy_sources(directory.path(), &sources).unwrap_err();
        assert!(error.contains("has no AST-compiled self-hash"), "{error}");

        let mut sources = compiled.sources();
        let data_index = BUILD_POLICY_SOURCE_PATHS
            .iter()
            .position(|path| *path == "Cargo.toml")
            .unwrap();
        let invalid_data_digest = "0".repeat(64);
        sources[data_index] = CompiledBuildPolicySource::rust(
            "Cargo.toml",
            &compiled.bytes[data_index],
            &invalid_data_digest,
        );
        let error = validate_compiled_build_policy_sources(directory.path(), &sources).unwrap_err();
        assert!(error.contains("must use byte identity only"), "{error}");
    }

    #[test]
    fn build_input_capture_rejects_compiled_policy_drift() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let compiled = CompiledPolicyFixture::capture(directory.path());
        fs::write(
            directory.path().join("Cargo.toml"),
            "[package]\nname = \"newer-policy-generation\"\n",
        )
        .unwrap();
        let compiled_sources = compiled.sources();

        let error = materialize_build_inputs(directory.path(), output.path(), &compiled_sources)
            .unwrap_err();
        assert!(error.contains("differs from the bytes compiled"), "{error}");
        assert!(error.contains("Cargo.toml"), "{error}");
    }

    #[test]
    fn build_input_capture_reuses_one_effective_manifest_byte_set() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let compiled = CompiledPolicyFixture::capture(directory.path());
        let compiled_sources = compiled.sources();
        let inputs =
            materialize_build_inputs(directory.path(), output.path(), &compiled_sources).unwrap();
        let expected = &compiled.bytes[BUILD_POLICY_SOURCE_PATHS
            .iter()
            .position(|path| *path == EFFECTIVE_SOURCE_MANIFEST)
            .unwrap()];

        let materialized_manifest =
            fs::read(inputs.adapter.root.join(EFFECTIVE_SOURCE_MANIFEST)).unwrap();
        assert_eq!(materialized_manifest.as_slice(), expected.as_slice());
        assert_eq!(
            inputs
                .live_files
                .iter()
                .filter(|file| file.normalized_path == EFFECTIVE_SOURCE_MANIFEST)
                .count(),
            1
        );
        assert_eq!(
            inputs
                .live_files
                .iter()
                .find(|file| file.normalized_path == EFFECTIVE_SOURCE_MANIFEST)
                .unwrap()
                .bytes
                .as_slice(),
            expected.as_slice()
        );
        assert_eq!(
            adapter_source_sha256(&inputs.adapter.root).unwrap(),
            inputs.adapter.adapter_source_sha256
        );
        assert_eq!(
            inputs.vendored_source_sha256,
            "0fac2ddee3fb443075db2a9adb8abef375fc0a27f050af93d7bf1771ec4b8de7"
        );
    }

    #[test]
    fn build_input_revalidation_rejects_live_generation_drift() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let compiled = CompiledPolicyFixture::capture(directory.path());
        let compiled_sources = compiled.sources();
        let inputs =
            materialize_build_inputs(directory.path(), output.path(), &compiled_sources).unwrap();

        for relative in [
            "native/boxdd_adapter.h",
            "third-party/box2d/src/aabb.c",
            "src/source_overlay.rs",
            EFFECTIVE_SOURCE_MANIFEST,
        ] {
            let path = directory.path().join(relative);
            let original = fs::read(&path).unwrap();
            fs::write(&path, format!("drift in {relative}\n")).unwrap();
            let error = inputs.revalidate_live().unwrap_err();
            assert!(error.contains(relative), "{error}");
            fs::write(&path, original).unwrap();
            inputs.revalidate_live().unwrap();
        }
    }

    #[test]
    fn build_input_revalidation_rejects_materialized_tree_tampering() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let compiled = CompiledPolicyFixture::capture(directory.path());
        let compiled_sources = compiled.sources();
        let inputs =
            materialize_build_inputs(directory.path(), output.path(), &compiled_sources).unwrap();

        let adapter = inputs.adapter.root.join("native/boxdd_adapter.c");
        let adapter_bytes = fs::read(&adapter).unwrap();
        fs::write(&adapter, "tampered adapter\n").unwrap();
        let error = inputs.revalidate_materialized().unwrap_err();
        assert!(error.contains("byte drift"), "{error}");
        fs::write(&adapter, adapter_bytes).unwrap();
        inputs.revalidate_materialized().unwrap();

        let effective = inputs.effective.root.join("src/aabb.c");
        fs::write(&effective, "tampered effective source\n").unwrap();
        let error = inputs.revalidate_materialized().unwrap_err();
        assert!(error.contains("byte drift"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn adapter_snapshot_rejects_symlinked_inputs() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("output");
        let external = directory.path().join("external-adapter.c");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(&external, "unreviewed adapter source\n").unwrap();
        adapter_fixture(&source);

        let adapter = source.join("native/boxdd_adapter.c");
        fs::remove_file(&adapter).unwrap();
        symlink(&external, &adapter).unwrap();

        let error = materialize_adapter_sources(&source, &output).unwrap_err();
        assert!(error.contains("must not traverse a symlink"), "{error}");
        assert!(
            fs::symlink_metadata(&adapter)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn materialized_tree_rejects_missing_extra_and_drift_without_repair() {
        let repository = repository_manifest_dir();

        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let missing = tree.root.join("src/aabb.c");
        fs::remove_file(&missing).unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("missing expected file"));
        assert!(!missing.exists());

        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let extra = tree.root.join("src/unreviewed.c");
        fs::write(&extra, "unreviewed\n").unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("unexpected file"));
        assert_eq!(fs::read_to_string(&extra).unwrap(), "unreviewed\n");

        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let drifted = tree.root.join("src/aabb.c");
        fs::write(&drifted, "byte drift\n").unwrap();
        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("byte drift"));
        assert_eq!(fs::read_to_string(&drifted).unwrap(), "byte drift\n");
    }

    #[cfg(unix)]
    #[test]
    fn materialized_tree_rejects_symlinks_without_repair() {
        use std::os::unix::fs::symlink;

        let repository = repository_manifest_dir();
        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let target = tree.root.join("src/aabb.c");
        let external = output.path().join("external.c");
        fs::write(&external, "external\n").unwrap();
        fs::remove_file(&target).unwrap();
        symlink(&external, &target).unwrap();

        let error = materialize_effective_box2d_sources(&repository, output.path()).unwrap_err();
        assert!(error.contains("contains a symlink"));
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn effective_source_manifest_rejects_unknown_missing_duplicate_and_out_of_order_transforms() {
        let directory = fixture();
        write_effective_manifest(
            &directory,
            format!("{}\nunknown = true\n", read_effective_manifest(&directory)),
        );
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("unknown field")
        );

        let directory = fixture();
        let missing_schema = read_effective_manifest(&directory)
            .lines()
            .filter(|line| !line.starts_with("schema ="))
            .collect::<Vec<_>>()
            .join("\n");
        write_effective_manifest(&directory, format!("{missing_schema}\n"));
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("field `schema`")
        );

        let directory = fixture();
        let duplicate = replace_once(
            read_effective_manifest(&directory),
            "id = \"world-snapshot-sensors-zero-memset\"",
            "id = \"world-snapshot-chain-shapes-zero-memset\"",
        );
        write_effective_manifest(&directory, duplicate);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("duplicate transform id")
        );

        let directory = fixture();
        let out_of_order = replace_once(
            replace_once(
                replace_once(
                    read_effective_manifest(&directory),
                    "id = \"world-snapshot-chain-shapes-zero-memset\"",
                    "id = \"temporary-transform-id\"",
                ),
                "id = \"world-snapshot-sensors-zero-memset\"",
                "id = \"world-snapshot-chain-shapes-zero-memset\"",
            ),
            "id = \"temporary-transform-id\"",
            "id = \"world-snapshot-sensors-zero-memset\"",
        );
        write_effective_manifest(&directory, out_of_order);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("must remain in reviewed order")
        );

        let directory = fixture();
        write_effective_manifest(
            &directory,
            remove_last_transform(read_effective_manifest(&directory)),
        );
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("exactly 4 ordered transforms")
        );
    }

    #[test]
    fn effective_source_manifest_rejects_escape_identity_and_transform_hash_drift_without_writing()
    {
        let directory = fixture();
        let output = directory.path().join("out");
        let escape = replace_once(
            read_effective_manifest(&directory),
            "path = \"src/world_snapshot.c\"",
            "path = \"../world_snapshot.c\"",
        );
        write_effective_manifest(&directory, escape);
        let error = materialize_effective_box2d_sources(directory.path(), &output).unwrap_err();
        assert!(error.contains("not normalized and relative"));
        assert!(!output.join(MATERIALIZED_SOURCE_DIRECTORY).exists());

        let directory = fixture();
        let wrong_upstream = replace_once(
            read_effective_manifest(&directory),
            "upstream_sha = \"56edae79f2949d86142b03450d5d60f63bcf5a6f\"",
            "upstream_sha = \"0000000000000000000000000000000000000000\"",
        );
        write_effective_manifest(&directory, wrong_upstream);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("does not match upstream.toml active_revision")
        );

        let directory = fixture();
        let wrong_tree = replace_once(
            read_effective_manifest(&directory),
            "source_tree = \"63a1ab02e3d2bf7c4d86b257b78976842b8c5ddb\"",
            "source_tree = \"0000000000000000000000000000000000000000\"",
        );
        write_effective_manifest(&directory, wrong_tree);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("does not match upstream.toml source_inventory.tree")
        );

        let directory = fixture();
        let wrong_origin = replace_once(
            read_effective_manifest(&directory),
            "origin_commit = \"c7a044a08d8e25511b7bce8d554cf5392a783497\"",
            "origin_commit = \"0000000000000000000000000000000000000000\"",
        );
        write_effective_manifest(&directory, wrong_origin);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("upstream origin does not match the reviewed backport")
        );

        let directory = fixture();
        let wrong_preimage =
            mutate_hash_field(read_effective_manifest(&directory), "preimage_sha256");
        write_effective_manifest(&directory, wrong_preimage);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("does not match reviewed preimage")
        );

        let directory = fixture();
        let wrong_replacement =
            mutate_hash_field(read_effective_manifest(&directory), "replacement_sha256");
        write_effective_manifest(&directory, wrong_replacement);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("does not match reviewed replacement")
        );

        let directory = fixture();
        let wrong_digest = mutate_hash_field(
            read_effective_manifest(&directory),
            "effective_source_sha256",
        );
        write_effective_manifest(&directory, wrong_digest);
        let output = directory.path().join("out");
        let error = materialize_effective_box2d_sources(directory.path(), &output).unwrap_err();
        assert!(error.contains("effective source SHA-256"));
        assert!(!output.join(MATERIALIZED_SOURCE_DIRECTORY).exists());
    }

    #[test]
    fn source_inventory_rejects_role_path_duplicate_order_byte_and_preimage_drift() {
        let directory = fixture();
        let invalid_path = replace_once(
            fs::read_to_string(directory.path().join(UPSTREAM_MANIFEST)).unwrap(),
            "\"src/aabb.c\"",
            "\"../aabb.c\"",
        );
        write_upstream_manifest(&directory, invalid_path);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("not normalized and relative")
        );

        let directory = fixture();
        let wrong_role = replace_once(
            fs::read_to_string(directory.path().join(UPSTREAM_MANIFEST)).unwrap(),
            "\"src/aabb.c\"",
            "\"src/aabb.h\"",
        );
        write_upstream_manifest(&directory, wrong_role);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("must have .c extension")
        );

        let directory = fixture();
        let duplicate = replace_once(
            fs::read_to_string(directory.path().join(UPSTREAM_MANIFEST)).unwrap(),
            "\"src/arena_allocator.c\"",
            "\"src/aabb.c\"",
        );
        write_upstream_manifest(&directory, duplicate);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("duplicate path")
        );

        let directory = fixture();
        let out_of_order = replace_once(
            replace_once(
                replace_once(
                    fs::read_to_string(directory.path().join(UPSTREAM_MANIFEST)).unwrap(),
                    "\"src/aabb.c\"",
                    "\"temporary-source-path\"",
                ),
                "\"src/arena_allocator.c\"",
                "\"src/aabb.c\"",
            ),
            "\"temporary-source-path\"",
            "\"src/arena_allocator.c\"",
        );
        write_upstream_manifest(&directory, out_of_order);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("must be strictly sorted")
        );

        let directory = fixture();
        fs::write(
            directory.path().join(SOURCE_ROOT).join("src/aabb.c"),
            "reviewed source drift\n",
        )
        .unwrap();
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("effective source SHA-256")
        );

        let directory = fixture();
        let target = directory
            .path()
            .join(SOURCE_ROOT)
            .join(WORLD_SNAPSHOT_SOURCE);
        let changed = fs::read_to_string(&target).unwrap().replacen(
            WORLD_SNAPSHOT_PATCHES[0].preimage,
            "/* upstream drift */\n",
            1,
        );
        fs::write(target, changed).unwrap();
        let output = directory.path().join("out");
        let error = materialize_effective_box2d_sources(directory.path(), &output).unwrap_err();
        assert!(error.contains(WORLD_SNAPSHOT_PATCHES[0].id));
        assert!(error.contains("found 0"));
        assert!(!output.join(MATERIALIZED_SOURCE_DIRECTORY).exists());

        let directory = fixture();
        let target = directory
            .path()
            .join(SOURCE_ROOT)
            .join(WORLD_SNAPSHOT_SOURCE);
        let changed = fs::read_to_string(&target).unwrap().replacen(
            "\t// Step 9: constraint graph\n",
            "\t// Step 9: reviewed context drift\n",
            1,
        );
        fs::write(target, changed).unwrap();
        let output = directory.path().join("out");
        let backport = WORLD_SNAPSHOT_PATCHES.last().unwrap();
        let error = materialize_effective_box2d_sources(directory.path(), &output).unwrap_err();
        assert!(error.contains(backport.id));
        assert!(error.contains("found 0"));
        assert!(!output.join(MATERIALIZED_SOURCE_DIRECTORY).exists());
    }

    #[cfg(unix)]
    #[test]
    fn source_inventory_rejects_symlinked_files() {
        use std::os::unix::fs::symlink;

        let directory = fixture();
        let target = directory.path().join(SOURCE_ROOT).join("src/aabb.c");
        let external = directory.path().join("outside.c");
        fs::write(&external, "not an inventory file\n").unwrap();
        fs::remove_file(&target).unwrap();
        symlink(&external, &target).unwrap();
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("must not traverse a symlink")
        );
    }
}
