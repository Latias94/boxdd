//! Deterministic, fail-closed overlays for reviewed upstream C sources.
//!
//! The Box2D submodule stays byte-for-byte at its reviewed Git revision. This module records the
//! small reviewed overlay separately, applies standard patches to files with exact upstream
//! digests, and derives one identity from every source file that participates in the native build.
//! Callers can therefore distinguish an official upstream checkout from the actual source bytes
//! sent to the compiler.

use diffy::{Patch, apply};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
use tempfile::TempDir;

pub const EFFECTIVE_SOURCE_MANIFEST: &str = "effective-source.toml";
pub const EFFECTIVE_SOURCE_SCHEMA: &str = "boxdd-effective-source-v2";
pub const EFFECTIVE_SOURCE_SCHEMA_VERSION: u64 = 2;
pub const UPSTREAM_MANIFEST_SCHEMA: u32 = 6;
pub const UPSTREAM_REPOSITORY: &str = "https://github.com/erincatto/box2d.git";
pub const WORLD_SNAPSHOT_SOURCE: &str = "src/world_snapshot.c";
pub const RECORDING_HEADER: &str = "src/recording.h";
pub const RECORDING_SOURCE: &str = "src/recording.c";
pub const RECORDING_REPLAY_HEADER: &str = "src/recording_replay.h";
pub const RECORDING_REPLAY_SOURCE: &str = "src/recording_replay.c";
pub const CORE_SOURCE: &str = "src/core.c";
pub const TIMER_SOURCE: &str = "src/timer.c";
pub const BOX2D_PUBLIC_HEADER: &str = "include/box2d/box2d.h";

const EFFECTIVE_SOURCE_DOMAIN: &[u8] = b"boxdd.effective-source.v1\0";
const ADAPTER_SOURCE_DOMAIN: &[u8] = b"boxdd.adapter.sources.v1\0";
const MATERIALIZED_SOURCE_DIRECTORY: &str = "boxdd-effective-source";
const MATERIALIZED_ADAPTER_SOURCE_DIRECTORY: &str = "boxdd-adapter-source";
const SOURCE_ROOT: &str = "third-party/box2d";
const UPSTREAM_MANIFEST: &str = "upstream.toml";

#[derive(Clone, Copy)]
struct SourcePatchSpec {
    path: &'static str,
    patch: &'static str,
}

const SOURCE_PATCHES: &[SourcePatchSpec] = &[
    SourcePatchSpec {
        path: BOX2D_PUBLIC_HEADER,
        patch: "patches/box2d-h.patch",
    },
    SourcePatchSpec {
        path: CORE_SOURCE,
        patch: "patches/core-c.patch",
    },
    SourcePatchSpec {
        path: RECORDING_SOURCE,
        patch: "patches/recording-c.patch",
    },
    SourcePatchSpec {
        path: RECORDING_HEADER,
        patch: "patches/recording-h.patch",
    },
    SourcePatchSpec {
        path: RECORDING_REPLAY_SOURCE,
        patch: "patches/recording-replay-c.patch",
    },
    SourcePatchSpec {
        path: RECORDING_REPLAY_HEADER,
        patch: "patches/recording-replay-h.patch",
    },
    SourcePatchSpec {
        path: TIMER_SOURCE,
        patch: "patches/timer-c.patch",
    },
    SourcePatchSpec {
        path: WORLD_SNAPSHOT_SOURCE,
        patch: "patches/world-snapshot-c.patch",
    },
];

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
#[derive(Debug)]
pub struct MaterializedEffectiveSources {
    pub identity: EffectiveSourceIdentity,
    pub root: PathBuf,
    pub public_include: PathBuf,
    pub private_include: PathBuf,
    pub c_sources: Vec<PathBuf>,
    _generation: TempDir,
    files: Vec<PreparedSourceFile>,
}

impl MaterializedEffectiveSources {
    /// Revalidate every file and directory in this materialized source tree.
    pub fn revalidate(&self) -> Result<(), String> {
        validate_materialized_tree(&self.root, &self.files)
    }
}

/// Repository-adapter bytes and the identity derived from those exact bytes.
#[derive(Debug)]
pub struct MaterializedAdapterSources {
    pub adapter_source_sha256: String,
    pub root: PathBuf,
    pub native_include: PathBuf,
    pub identity_probe_source: PathBuf,
    pub c_sources: Vec<PathBuf>,
    _generation: TempDir,
    files: Vec<PreparedSourceFile>,
}

/// Effective Box2D and repository-adapter compiler inputs captured from one source snapshot.
#[derive(Debug)]
pub struct MaterializedBuildInputs {
    pub effective: MaterializedEffectiveSources,
    pub adapter: MaterializedAdapterSources,
    pub vendored_source_sha256: String,
}

impl MaterializedBuildInputs {
    /// Revalidate every byte exposed to a compiler.
    pub fn revalidate(&self) -> Result<(), String> {
        self.revalidate_materialized()
    }

    /// Reject missing, extra, symlinked, or changed bytes in either materialized tree.
    pub fn revalidate_materialized(&self) -> Result<(), String> {
        self.effective.revalidate()?;
        validate_materialized_tree(&self.adapter.root, &self.adapter.files)
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

#[derive(Debug)]
struct EffectiveSourceManifest {
    upstream_sha: String,
    source_tree: String,
    effective_source_sha256: String,
    patches: Vec<DeclaredPatch>,
}

#[derive(Debug)]
struct DeclaredPatch {
    path: String,
    patch: String,
    upstream_sha256: String,
    effective_sha256: String,
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
    patch_files: Vec<CapturedLiveFile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedSourceFile {
    role: SourceRole,
    normalized_path: String,
    relative_path: PathBuf,
    source_path: PathBuf,
    captured_bytes: Vec<u8>,
    source_bytes: Vec<u8>,
    effective_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct CapturedLiveFile {
    normalized_path: String,
    relative_path: PathBuf,
    source_path: PathBuf,
    captured_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

/// Validate the complete effective-source contract without writing an overlay.
#[allow(dead_code)] // Shared with repository provider and ABI tooling; unused by this library target.
pub fn effective_source_identity(manifest_dir: &Path) -> Result<EffectiveSourceIdentity, String> {
    Ok(prepare_effective_sources(manifest_dir)?.identity)
}

/// Return SHA-256 identities for an exact set of effective source files without materializing it.
#[allow(dead_code)] // Repository tooling consumes this through the shared source module.
pub fn effective_source_file_sha256s(
    manifest_dir: &Path,
    paths: &[&str],
) -> Result<BTreeMap<String, String>, String> {
    let prepared = prepare_effective_sources(manifest_dir)?;
    let requested = paths
        .iter()
        .map(|path| validate_normalized_path(path, "effective source digest", None, None))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if requested.len() != paths.len() {
        return Err("effective source digest paths must be unique".to_owned());
    }

    let identities = prepared
        .files
        .iter()
        .filter(|file| requested.contains(&file.normalized_path))
        .map(|file| {
            (
                file.normalized_path.clone(),
                sha256_bytes(&file.effective_bytes),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if identities.len() != requested.len() {
        let missing = requested
            .iter()
            .filter(|path| !identities.contains_key(path.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "effective source digest paths are absent from the reviewed inventory: {missing:?}"
        ));
    }
    Ok(identities)
}

/// Validate and materialize the complete reviewed source tree for native consumers.
#[allow(dead_code)] // Shared with repository provider and ABI tooling; unused by this library target.
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

/// Capture effective and adapter inputs from one snapshot and materialize only those bytes.
pub fn materialize_build_inputs(
    manifest_dir: &Path,
    output_dir: &Path,
) -> Result<MaterializedBuildInputs, String> {
    let manifest_root = canonical_real_root(manifest_dir, "build input root")?;
    let upstream_manifest = capture_live_file(&manifest_root, UPSTREAM_MANIFEST)?;
    let effective_manifest = capture_live_file(&manifest_root, EFFECTIVE_SOURCE_MANIFEST)?;
    let prepared_effective = prepare_effective_sources_from_manifests(
        &manifest_root,
        &upstream_manifest.bytes,
        &effective_manifest.bytes,
    )?;
    let prepared_adapter = capture_adapter_sources(&manifest_root)?;
    let mut repository_files = vec![upstream_manifest, effective_manifest];
    repository_files.extend(prepared_effective.patch_files.iter().cloned());
    let live_files = captured_build_inputs(
        repository_files,
        &prepared_effective.files,
        &prepared_adapter,
    )?;

    // A second read closes the capture window before any compiler can consume the snapshots.
    revalidate_live_files(&manifest_root, &live_files)?;

    let vendored_source_sha256 = captured_vendored_source_sha256(&prepared_effective);
    let adapter_source_sha256 = captured_adapter_source_sha256(&prepared_adapter);
    let effective = materialize_prepared_sources(prepared_effective, output_dir)?;
    let adapter =
        materialize_prepared_adapter_sources(prepared_adapter, adapter_source_sha256, output_dir)?;
    let inputs = MaterializedBuildInputs {
        effective,
        adapter,
        vendored_source_sha256,
    };
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
    let upstream_manifest =
        canonical_reviewed_text(upstream_manifest, &manifest_dir.join(UPSTREAM_MANIFEST))?;
    let effective_manifest = canonical_reviewed_text(
        effective_manifest,
        &manifest_dir.join(EFFECTIVE_SOURCE_MANIFEST),
    )?;
    let upstream =
        parse_upstream_inventory(&manifest_dir.join(UPSTREAM_MANIFEST), &upstream_manifest)?;
    let effective = parse_effective_source_manifest(
        &manifest_dir.join(EFFECTIVE_SOURCE_MANIFEST),
        &effective_manifest,
    )?;
    validate_effective_source_manifest(&effective, &upstream)?;

    let source_root = manifest_dir.join(SOURCE_ROOT);
    let canonical_root = canonical_source_root(&source_root)?;
    let mut files = upstream
        .entries
        .iter()
        .map(|entry| prepare_source_file(&canonical_root, entry))
        .collect::<Result<Vec<_>, _>>()?;

    let mut patch_files = Vec::with_capacity(effective.patches.len());
    for declared in &effective.patches {
        let path = declared.path.as_str();
        let target_indices = files
            .iter()
            .enumerate()
            .filter_map(|(index, file)| (file.normalized_path == path).then_some(index))
            .collect::<Vec<_>>();
        if target_indices.len() != 1 {
            return Err(format!(
                "{UPSTREAM_MANIFEST} source inventory must contain {path:?} exactly once; found {}",
                target_indices.len()
            ));
        }

        let target = &mut files[target_indices[0]];
        let (effective_bytes, patch_file) = apply_declared_patch(manifest_dir, target, declared)?;
        target.effective_bytes = effective_bytes;
        patch_files.push(patch_file);
    }

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

    Ok(PreparedEffectiveSources {
        identity,
        files,
        patch_files,
    })
}

fn materialize_prepared_sources(
    prepared: PreparedEffectiveSources,
    output_dir: &Path,
) -> Result<MaterializedEffectiveSources, String> {
    let generation = create_materialized_generation_directory(
        output_dir,
        MATERIALIZED_SOURCE_DIRECTORY,
        "effective source",
    )?;
    let root = generation.path().to_path_buf();
    write_materialized_tree(&root, &prepared.files)
        .and_then(|()| validate_materialized_tree(&root, &prepared.files))?;
    materialized_sources(prepared, generation)
}

fn capture_adapter_sources(manifest_dir: &Path) -> Result<Vec<PreparedSourceFile>, String> {
    let canonical_root = canonical_real_root(manifest_dir, "adapter source root")?;
    ADAPTER_SOURCE_PATHS
        .iter()
        .map(|relative_path| {
            let normalized_path =
                validate_normalized_path(relative_path, "adapter source", None, None)?;
            let relative_path = PathBuf::from(&normalized_path);
            let source_path = resolve_regular_file(
                &canonical_root,
                &relative_path,
                &normalized_path,
                "adapter source",
            )?;
            let captured_bytes = fs::read(&source_path).map_err(|error| {
                format!(
                    "failed to read adapter source {normalized_path:?} at {}: {error}",
                    source_path.display()
                )
            })?;
            let source_bytes = canonical_reviewed_text(&captured_bytes, &source_path)?;
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
                captured_bytes,
                effective_bytes: source_bytes.clone(),
                source_bytes,
            })
        })
        .collect()
}

fn capture_live_file(
    manifest_root: &Path,
    relative_path: &str,
) -> Result<CapturedLiveFile, String> {
    let normalized_path = validate_normalized_path(relative_path, "build input", None, None)?;
    let relative_path = PathBuf::from(&normalized_path);
    let source_path = resolve_regular_file(
        manifest_root,
        &relative_path,
        &normalized_path,
        "build input",
    )?;
    let captured_bytes = fs::read(&source_path).map_err(|error| {
        format!(
            "failed to read build input {normalized_path:?} at {}: {error}",
            source_path.display()
        )
    })?;
    let bytes = canonical_reviewed_text(&captured_bytes, &source_path)?;
    Ok(CapturedLiveFile {
        normalized_path,
        relative_path,
        source_path,
        captured_bytes,
        bytes,
    })
}

fn captured_build_inputs(
    manifest_files: impl IntoIterator<Item = CapturedLiveFile>,
    effective_files: &[PreparedSourceFile],
    adapter_files: &[PreparedSourceFile],
) -> Result<Vec<CapturedLiveFile>, String> {
    let mut inputs = BTreeMap::new();
    for file in manifest_files {
        insert_build_input(&mut inputs, file)?;
    }
    for file in effective_files {
        let normalized_path = format!("{SOURCE_ROOT}/{}", file.normalized_path);
        insert_build_input(
            &mut inputs,
            CapturedLiveFile {
                relative_path: PathBuf::from(&normalized_path),
                normalized_path,
                source_path: file.source_path.clone(),
                captured_bytes: file.captured_bytes.clone(),
                bytes: file.source_bytes.clone(),
            },
        )?;
    }
    for file in adapter_files {
        insert_build_input(
            &mut inputs,
            CapturedLiveFile {
                normalized_path: file.normalized_path.clone(),
                relative_path: file.relative_path.clone(),
                source_path: file.source_path.clone(),
                captured_bytes: file.captured_bytes.clone(),
                bytes: file.source_bytes.clone(),
            },
        )?;
    }
    Ok(inputs.into_values().collect())
}

fn insert_build_input(
    inputs: &mut BTreeMap<String, CapturedLiveFile>,
    candidate: CapturedLiveFile,
) -> Result<(), String> {
    if let Some(existing) = inputs.get(&candidate.normalized_path) {
        if existing.relative_path != candidate.relative_path
            || existing.source_path != candidate.source_path
            || existing.captured_bytes != candidate.captured_bytes
            || existing.bytes != candidate.bytes
        {
            return Err(format!(
                "overlapping build input {:?} was captured from different bytes or paths",
                candidate.normalized_path
            ));
        }
        return Ok(());
    }
    inputs.insert(candidate.normalized_path.clone(), candidate);
    Ok(())
}

fn revalidate_live_files(manifest_root: &Path, files: &[CapturedLiveFile]) -> Result<(), String> {
    for file in files {
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
        if current != file.captured_bytes {
            return Err(format!(
                "captured build input {:?} changed after its source snapshot was captured",
                file.normalized_path
            ));
        }
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
    let generation = create_materialized_generation_directory(
        output_dir,
        MATERIALIZED_ADAPTER_SOURCE_DIRECTORY,
        "adapter source",
    )?;
    let root = generation.path().to_path_buf();
    write_materialized_tree(&root, &files)
        .and_then(|()| validate_materialized_tree(&root, &files))?;
    materialized_adapter_sources(files, adapter_source_sha256, generation)
}

fn materialized_adapter_sources(
    files: Vec<PreparedSourceFile>,
    adapter_source_sha256: String,
    generation: TempDir,
) -> Result<MaterializedAdapterSources, String> {
    let root = generation.path().to_path_buf();
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
        _generation: generation,
        files,
    })
}

fn materialized_sources(
    prepared: PreparedEffectiveSources,
    generation: TempDir,
) -> Result<MaterializedEffectiveSources, String> {
    let root = generation.path().to_path_buf();
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
        _generation: generation,
        files: prepared.files,
    })
}

fn create_materialized_generation_directory(
    output_dir: &Path,
    directory_prefix: &str,
    label: &str,
) -> Result<TempDir, String> {
    ensure_real_directory(output_dir, &format!("{label} output directory"))?;
    let directory = tempfile::Builder::new()
        .prefix(&format!("{directory_prefix}-"))
        .tempdir_in(output_dir)
        .map_err(|error| {
            format!(
                "failed to create {label} generation below {}: {error}",
                output_dir.display()
            )
        })?;
    set_private_directory_permissions(directory.path(), label)?;
    Ok(directory)
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
    Ok(())
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path, label: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("failed to protect {label} {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path, _label: &str) -> Result<(), String> {
    Ok(())
}

fn write_materialized_tree(root: &Path, files: &[PreparedSourceFile]) -> Result<(), String> {
    for file in files {
        create_materialized_parent_directories(root, &file.relative_path)?;
        let path = root.join(&file.relative_path);
        fs::write(&path, &file.effective_bytes).map_err(|error| {
            format!(
                "failed to write effective source file {}: {error}",
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
            "patches",
        ],
    )?;

    let schema_version = required_integer(table, "schema_version", "effective source manifest")?;
    let schema = required_string(table, "schema", "effective source manifest")?;
    if schema_version != EFFECTIVE_SOURCE_SCHEMA_VERSION || schema != EFFECTIVE_SOURCE_SCHEMA {
        return Err(format!(
            "unsupported effective source manifest schema: version={schema_version} name={schema:?}"
        ));
    }

    let patches = required_array(table, "patches", "effective source manifest")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let table = value
                .as_table()
                .ok_or_else(|| format!("effective source patch #{index} must be a TOML table"))?;
            parse_declared_patch(table, index)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EffectiveSourceManifest {
        upstream_sha: required_string(table, "upstream_sha", "effective source manifest")?,
        source_tree: required_string(table, "source_tree", "effective source manifest")?,
        effective_source_sha256: required_string(
            table,
            "effective_source_sha256",
            "effective source manifest",
        )?,
        patches,
    })
}

fn parse_declared_patch(
    table: &toml::map::Map<String, toml::Value>,
    index: usize,
) -> Result<DeclaredPatch, String> {
    let label = format!("effective source patch #{index}");
    reject_unknown_fields(
        table,
        &label,
        &["path", "patch", "upstream_sha256", "effective_sha256"],
    )?;
    Ok(DeclaredPatch {
        path: required_string(table, "path", &label)?,
        patch: required_string(table, "patch", &label)?,
        upstream_sha256: required_string(table, "upstream_sha256", &label)?,
        effective_sha256: required_string(table, "effective_sha256", &label)?,
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
    if effective.patches.len() != SOURCE_PATCHES.len() {
        return Err(format!(
            "effective source manifest must declare exactly {} ordered patches; found {}",
            SOURCE_PATCHES.len(),
            effective.patches.len()
        ));
    }

    let mut paths = BTreeSet::new();
    let mut patch_files = BTreeSet::new();
    for (index, (declared, expected)) in effective.patches.iter().zip(SOURCE_PATCHES).enumerate() {
        let path =
            validate_normalized_path(&declared.path, "effective source patch target", None, None)?;
        let patch = validate_normalized_path(
            &declared.patch,
            "effective source patch file",
            Some("patches/"),
            Some("patch"),
        )?;
        if !paths.insert(path.clone()) {
            return Err(format!(
                "effective source manifest contains duplicate patch target {path:?}"
            ));
        }
        if !patch_files.insert(patch.clone()) {
            return Err(format!(
                "effective source manifest contains duplicate patch file {patch:?}"
            ));
        }
        if path != expected.path || patch != expected.patch {
            return Err(format!(
                "effective source patch #{index} must map {:?} to {:?}",
                expected.path, expected.patch
            ));
        }
        validate_sha256(
            &format!("effective source patch {path} upstream_sha256"),
            &declared.upstream_sha256,
        )?;
        validate_sha256(
            &format!("effective source patch {path} effective_sha256"),
            &declared.effective_sha256,
        )?;
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
    let schema_version = required_integer(root, "schema_version", "upstream manifest")?;
    if schema_version != u64::from(UPSTREAM_MANIFEST_SCHEMA) {
        return Err(format!(
            "unsupported upstream manifest schema {schema_version}; expected {UPSTREAM_MANIFEST_SCHEMA}"
        ));
    }
    let repository = required_string(root, "repository", "upstream manifest")?;
    if repository != UPSTREAM_REPOSITORY {
        return Err(format!(
            "upstream manifest repository must be the official Box2D repository {UPSTREAM_REPOSITORY:?}"
        ));
    }
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
    let captured_bytes = fs::read(&source_path).map_err(|error| {
        format!(
            "failed to read vendored source {:?} at {}: {error}",
            entry.normalized_path,
            source_path.display()
        )
    })?;
    let source_bytes = canonical_reviewed_text(&captured_bytes, &source_path)?;
    Ok(PreparedSourceFile {
        role: entry.role,
        normalized_path: entry.normalized_path.clone(),
        relative_path: entry.relative_path.clone(),
        source_path,
        captured_bytes,
        effective_bytes: source_bytes.clone(),
        source_bytes,
    })
}

fn canonical_reviewed_text(bytes: &[u8], path: &Path) -> Result<Vec<u8>, String> {
    let mut canonical = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\r' {
            canonical.push(bytes[index]);
            index += 1;
            continue;
        }
        if bytes.get(index + 1) != Some(&b'\n') {
            return Err(format!(
                "reviewed text input {} contains a lone carriage return",
                path.display()
            ));
        }
        canonical.push(b'\n');
        index += 2;
    }
    Ok(canonical)
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

fn apply_declared_patch(
    manifest_dir: &Path,
    source: &PreparedSourceFile,
    declared: &DeclaredPatch,
) -> Result<(Vec<u8>, CapturedLiveFile), String> {
    let upstream_sha256 = sha256_bytes(&source.source_bytes);
    if upstream_sha256 != declared.upstream_sha256 {
        return Err(format!(
            "reviewed source {} SHA-256 mismatch: expected {}, found {upstream_sha256}",
            declared.path, declared.upstream_sha256
        ));
    }

    let manifest_root = canonical_real_root(manifest_dir, "effective source root")?;
    let patch_file = capture_live_file(&manifest_root, &declared.patch)?;
    let patch_source = std::str::from_utf8(&patch_file.bytes).map_err(|error| {
        format!(
            "effective source patch {} is not UTF-8: {error}",
            patch_file.source_path.display()
        )
    })?;
    let patch = Patch::from_str(patch_source).map_err(|error| {
        format!(
            "failed to parse effective source patch {}: {error}",
            patch_file.source_path.display()
        )
    })?;
    if patch.original() != Some(declared.path.as_str())
        || patch.modified() != Some(declared.path.as_str())
    {
        return Err(format!(
            "effective source patch {} must name {:?} in both headers",
            patch_file.source_path.display(),
            declared.path
        ));
    }

    let upstream = std::str::from_utf8(&source.source_bytes).map_err(|error| {
        format!(
            "reviewed upstream source {} is not UTF-8: {error}",
            source.source_path.display()
        )
    })?;
    let effective = apply(upstream, &patch).map_err(|error| {
        format!(
            "failed to apply effective source patch {} to {}: {error}",
            patch_file.source_path.display(),
            declared.path
        )
    })?;
    let effective_bytes = effective.into_bytes();
    let effective_sha256 = sha256_bytes(&effective_bytes);
    if effective_sha256 != declared.effective_sha256 {
        return Err(format!(
            "effective source {} SHA-256 mismatch: expected {}, found {effective_sha256}",
            declared.path, declared.effective_sha256
        ));
    }
    Ok((effective_bytes, patch_file))
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
        for patch in SOURCE_PATCHES {
            let destination = directory.path().join(patch.patch);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(repository.join(patch.patch), destination).unwrap();
        }

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
        adapter_fixture(directory.path());
        directory
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

    fn remove_last_patch(value: String) -> String {
        let index = value.rfind("\n[[patches]]").unwrap();
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
            "f6cf3bb1ca240888879a5c26ad468f98e98d5275018717505b2b0becfacd7497"
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
        let patched_by_path = SOURCE_PATCHES
            .iter()
            .map(|patch| {
                (
                    patch.path,
                    fs::read_to_string(materialized.root.join(patch.path)).unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let snapshot = patched_by_path.get(WORLD_SNAPSHOT_SOURCE).unwrap();
        assert!(snapshot.contains("b2Array_Clear( world->bodyMoveEvents );"));
        assert!(snapshot.contains("b2Array_Clear( world->jointEvents );"));
        assert!(!snapshot.contains("bool restored = b2DeserializeIntoShell"));
        assert!(!snapshot.contains("b2World_GetStateHash( b2WorldId worldId )"));

        let replay = patched_by_path.get(RECORDING_REPLAY_SOURCE).unwrap();
        assert!(replay.contains("static bool b2RecGrow("));
        assert!(replay.contains("static bool b2RecFreeArray("));
        assert!(!replay.contains("static void b2RecGrow("));
        assert!(!replay.contains("static char s_bufs"));
        assert!(!replay.contains("static int s_next"));
        assert!(replay.contains("char* buf = rdr->stringBuffer;"));
        assert_eq!(replay.matches("if ( q == NULL )").count(), 9);
        assert!(replay.contains("size_t validatedBytes = metadataBytes;"));
        assert!(replay.contains("player->keyframeBytes += plannedMetadataBytes - metadataBytes;"));
        assert!(!replay.contains("player->keyframeBytes + newBytes > player->keyframeBudget"));

        let recording = patched_by_path.get(RECORDING_SOURCE).unwrap();
        assert!(!recording.contains("b2RecBuffer blob = { 0 };"));
        assert!(recording.contains("int snapshotStart = recording->buffer.size;"));
        assert!(recording.contains("b2SerializeWorld( world, &recording->buffer );"));
        assert!(recording.contains("memcpy( recording->buffer.data, &hdr, sizeof( hdr ) );"));

        let replay_header = patched_by_path.get(RECORDING_REPLAY_HEADER).unwrap();
        assert!(replay_header.contains("#include \"box2d/constants.h\""));
        assert!(replay_header.contains("char stringBuffer[B2_NAME_LENGTH + 1]"));

        let core = patched_by_path.get(CORE_SOURCE).unwrap();
        assert_eq!(
            core.matches("if ( ptr == NULL || ( (uintptr_t)ptr & 0x1F ) != 0 )")
                .count(),
            2
        );
        assert_eq!(core.matches("abort();").count(), 3);

        let timer = patched_by_path.get(TIMER_SOURCE).unwrap();
        assert!(!timer.contains("static double s_invFrequency"));
        assert!(!timer.contains("static b2SetThreadDescriptionFn pfn"));
        assert!(!timer.contains("static int resolved"));
        assert_eq!(timer.matches("b2GetMillisecondsPerTick").count(), 6);
    }

    #[test]
    fn materialized_trees_use_independent_immutable_generations() {
        let repository = repository_manifest_dir();
        let output = tempfile::tempdir().unwrap();
        let first = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        first.revalidate().unwrap();
        assert_eq!(first.root.parent(), Some(output.path()));
        assert!(
            first
                .root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(MATERIALIZED_SOURCE_DIRECTORY)
        );
        let tracked = first.root.join("src/aabb.c");
        let bytes = fs::read(&tracked).unwrap();
        fs::write(&tracked, "stale bytes\n").unwrap();

        let second = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        second.revalidate().unwrap();
        assert_eq!(second.identity, first.identity);
        assert_ne!(second.root, first.root);
        assert_eq!(fs::read(&tracked).unwrap(), b"stale bytes\n");
        assert_eq!(fs::read(second.root.join("src/aabb.c")).unwrap(), bytes);
        assert!(first.root.is_dir());
        assert!(second.root.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn materialization_rejects_symlinked_output_directories_and_protects_generations() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let repository = repository_manifest_dir();
        let container = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let output = container.path().join("symlinked-output");
        symlink(external.path(), &output).unwrap();
        let error = materialize_effective_box2d_sources(&repository, &output).unwrap_err();
        assert!(error.contains("must be a real non-symlink directory"));
        assert!(
            fs::symlink_metadata(&output)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        let output = tempfile::tempdir().unwrap();
        let mut permissions = fs::metadata(output.path()).unwrap().permissions();
        permissions.set_mode(0o770);
        fs::set_permissions(output.path(), permissions).unwrap();
        let materialized = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let mode = fs::metadata(materialized.root)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
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
    fn adapter_materialization_creates_an_independent_current_generation() {
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
        assert!(first.native_include.is_dir());
        assert!(second.native_include.is_dir());
        assert_ne!(second.adapter_source_sha256, first_digest);
        assert_ne!(second.root, first.root);
        assert_eq!(
            fs::read(first.root.join("native/boxdd_adapter.c")).unwrap(),
            first_adapter
        );
        assert_eq!(
            fs::read(second.root.join("native/boxdd_adapter.c")).unwrap(),
            b"mutated adapter source\n"
        );

        let unavailable = directory.path().join("source-unavailable");
        fs::rename(&source, &unavailable).unwrap();
        assert_eq!(
            fs::read(second.root.join("native/boxdd_adapter.c")).unwrap(),
            b"mutated adapter source\n"
        );
    }

    #[test]
    fn build_input_capture_materializes_one_canonical_effective_manifest() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let manifest = directory.path().join(EFFECTIVE_SOURCE_MANIFEST);
        let expected = canonical_reviewed_text(&fs::read(&manifest).unwrap(), &manifest).unwrap();
        let inputs = materialize_build_inputs(directory.path(), output.path()).unwrap();

        let materialized_manifest =
            fs::read(inputs.adapter.root.join(EFFECTIVE_SOURCE_MANIFEST)).unwrap();
        assert_eq!(materialized_manifest, expected);
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
    fn crlf_checkout_preserves_canonical_source_identities_and_materialized_bytes() {
        fn rewrite_with_crlf(path: &Path) {
            let source = canonical_reviewed_text(&fs::read(path).unwrap(), path).unwrap();
            let mut crlf = Vec::with_capacity(source.len() + source.len() / 20);
            for byte in source {
                if byte == b'\n' {
                    crlf.push(b'\r');
                }
                crlf.push(byte);
            }
            fs::write(path, crlf).unwrap();
        }

        let repository = repository_manifest_dir();
        let expected_effective = effective_source_identity(&repository).unwrap();
        let expected_adapter = adapter_source_sha256(&repository).unwrap();
        let expected_vendored =
            captured_vendored_source_sha256(&prepare_effective_sources(&repository).unwrap());
        let directory = build_input_fixture();
        let inventory = load_upstream_inventory(directory.path()).unwrap();
        let mut text_inputs = BTreeSet::from([
            PathBuf::from(UPSTREAM_MANIFEST),
            PathBuf::from(EFFECTIVE_SOURCE_MANIFEST),
        ]);
        text_inputs.extend(
            SOURCE_PATCHES
                .iter()
                .map(|patch| PathBuf::from(patch.patch)),
        );
        text_inputs.extend(
            inventory
                .entries
                .iter()
                .map(|entry| PathBuf::from(SOURCE_ROOT).join(&entry.relative_path)),
        );
        text_inputs.extend(ADAPTER_SOURCE_PATHS.iter().map(PathBuf::from));
        for relative in text_inputs {
            rewrite_with_crlf(&directory.path().join(relative));
        }

        let output = tempfile::tempdir().unwrap();
        let inputs = materialize_build_inputs(directory.path(), output.path()).unwrap();
        assert_eq!(inputs.effective.identity, expected_effective);
        assert_eq!(inputs.adapter.adapter_source_sha256, expected_adapter);
        assert_eq!(inputs.vendored_source_sha256, expected_vendored);
        assert!(
            inputs
                .effective
                .files
                .iter()
                .chain(&inputs.adapter.files)
                .all(|file| !file.effective_bytes.contains(&b'\r'))
        );
        inputs.revalidate().unwrap();
    }

    #[test]
    fn materialized_build_inputs_ignore_later_live_source_drift() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let inputs = materialize_build_inputs(directory.path(), output.path()).unwrap();
        let adapter = inputs.adapter.root.join("native/boxdd_adapter.h");
        let effective = inputs.effective.root.join("src/aabb.c");
        let adapter_bytes = fs::read(&adapter).unwrap();
        let effective_bytes = fs::read(&effective).unwrap();

        for relative in [
            "native/boxdd_adapter.h",
            "patches/core-c.patch",
            "third-party/box2d/src/aabb.c",
            EFFECTIVE_SOURCE_MANIFEST,
        ] {
            let path = directory.path().join(relative);
            fs::write(&path, format!("drift in {relative}\n")).unwrap();
        }

        inputs.revalidate().unwrap();
        assert_eq!(fs::read(adapter).unwrap(), adapter_bytes);
        assert_eq!(fs::read(effective).unwrap(), effective_bytes);
    }

    #[test]
    fn materialized_build_inputs_remove_owned_generations_on_drop() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let (effective_root, adapter_root) = {
            let inputs = materialize_build_inputs(directory.path(), output.path()).unwrap();
            assert!(inputs.effective.root.is_dir());
            assert!(inputs.adapter.root.is_dir());
            (inputs.effective.root.clone(), inputs.adapter.root.clone())
        };

        assert!(!effective_root.exists());
        assert!(!adapter_root.exists());
    }

    #[test]
    fn build_input_revalidation_rejects_materialized_tree_tampering() {
        let directory = build_input_fixture();
        let output = tempfile::tempdir().unwrap();
        let inputs = materialize_build_inputs(directory.path(), output.path()).unwrap();

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
    fn materialized_tree_rejects_drift_without_mutating_prior_generations() {
        let repository = repository_manifest_dir();
        let files = prepare_effective_sources(&repository).unwrap().files;

        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let missing = tree.root.join("src/aabb.c");
        fs::remove_file(&missing).unwrap();
        let error = validate_materialized_tree(&tree.root, &files).unwrap_err();
        assert!(error.contains("missing expected file"));
        assert!(!missing.exists());
        let replacement = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        assert_ne!(replacement.root, tree.root);
        assert!(!missing.exists());
        replacement.revalidate().unwrap();

        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let extra = tree.root.join("src/unreviewed.c");
        fs::write(&extra, "unreviewed\n").unwrap();
        let error = validate_materialized_tree(&tree.root, &files).unwrap_err();
        assert!(error.contains("unexpected file"));
        assert_eq!(fs::read_to_string(&extra).unwrap(), "unreviewed\n");
        let replacement = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        assert_ne!(replacement.root, tree.root);
        assert_eq!(fs::read_to_string(&extra).unwrap(), "unreviewed\n");
        replacement.revalidate().unwrap();

        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let drifted = tree.root.join("src/aabb.c");
        fs::write(&drifted, "byte drift\n").unwrap();
        let error = validate_materialized_tree(&tree.root, &files).unwrap_err();
        assert!(error.contains("byte drift"));
        assert_eq!(fs::read_to_string(&drifted).unwrap(), "byte drift\n");
        let replacement = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        assert_ne!(replacement.root, tree.root);
        assert_eq!(fs::read_to_string(&drifted).unwrap(), "byte drift\n");
        replacement.revalidate().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn materialized_tree_rejects_symlinks_without_mutating_the_prior_generation() {
        use std::os::unix::fs::symlink;

        let repository = repository_manifest_dir();
        let output = tempfile::tempdir().unwrap();
        let tree = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        let target = tree.root.join("src/aabb.c");
        let external = output.path().join("external.c");
        fs::write(&external, "external\n").unwrap();
        fs::remove_file(&target).unwrap();
        symlink(&external, &target).unwrap();

        let files = prepare_effective_sources(&repository).unwrap().files;
        let error = validate_materialized_tree(&tree.root, &files).unwrap_err();
        assert!(error.contains("contains a symlink"));
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let replacement = materialize_effective_box2d_sources(&repository, output.path()).unwrap();
        assert_ne!(replacement.root, tree.root);
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        replacement.revalidate().unwrap();
    }

    #[test]
    fn effective_source_manifest_rejects_unknown_missing_duplicate_and_out_of_order_patches() {
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
            "path = \"src/recording.c\"",
            "path = \"src/core.c\"",
        );
        write_effective_manifest(&directory, duplicate);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("duplicate patch target")
        );

        let directory = fixture();
        let out_of_order = replace_once(
            replace_once(
                replace_once(
                    read_effective_manifest(&directory),
                    "path = \"src/core.c\"",
                    "path = \"temporary-source-path\"",
                ),
                "path = \"src/recording.c\"",
                "path = \"src/core.c\"",
            ),
            "path = \"temporary-source-path\"",
            "path = \"src/recording.c\"",
        );
        write_effective_manifest(&directory, out_of_order);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("must map")
        );

        let directory = fixture();
        write_effective_manifest(
            &directory,
            remove_last_patch(read_effective_manifest(&directory)),
        );
        let expected = format!("exactly {} ordered patches", SOURCE_PATCHES.len());
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains(&expected)
        );
    }

    #[test]
    fn effective_source_manifest_rejects_escape_identity_and_patch_hash_drift_without_writing() {
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
        let wrong_upstream_digest =
            mutate_hash_field(read_effective_manifest(&directory), "upstream_sha256");
        write_effective_manifest(&directory, wrong_upstream_digest);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("reviewed source include/box2d/box2d.h SHA-256 mismatch")
        );

        let directory = fixture();
        let wrong_effective_digest =
            mutate_hash_field(read_effective_manifest(&directory), "effective_sha256");
        write_effective_manifest(&directory, wrong_effective_digest);
        assert!(
            effective_source_identity(directory.path())
                .unwrap_err()
                .contains("effective source include/box2d/box2d.h SHA-256 mismatch")
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
    fn source_inventory_rejects_role_path_duplicate_order_byte_and_patch_target_drift() {
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
            .join(SOURCE_PATCHES[0].path);
        let mut changed = fs::read(&target).unwrap();
        changed.extend_from_slice(b"\n/* upstream drift */\n");
        fs::write(&target, changed).unwrap();
        let output = directory.path().join("out");
        let error = materialize_effective_box2d_sources(directory.path(), &output).unwrap_err();
        assert!(error.contains(SOURCE_PATCHES[0].path));
        assert!(error.contains("SHA-256 mismatch"));
        assert!(!output.join(MATERIALIZED_SOURCE_DIRECTORY).exists());
    }

    #[test]
    fn upstream_inventory_rejects_unsupported_schema() {
        let directory = fixture();
        let unsupported_schema_version = UPSTREAM_MANIFEST_SCHEMA - 1;
        let unsupported_schema_manifest = replace_once(
            fs::read_to_string(directory.path().join(UPSTREAM_MANIFEST)).unwrap(),
            &format!("schema_version = {UPSTREAM_MANIFEST_SCHEMA}"),
            &format!("schema_version = {unsupported_schema_version}"),
        );
        write_upstream_manifest(&directory, unsupported_schema_manifest);
        let error = effective_source_identity(directory.path()).unwrap_err();
        assert_eq!(
            error,
            format!(
                "unsupported upstream manifest schema {}; expected {UPSTREAM_MANIFEST_SCHEMA}",
                unsupported_schema_version
            )
        );
    }

    #[test]
    fn upstream_inventory_rejects_untrusted_repository() {
        let directory = fixture();
        let untrusted_repository = replace_once(
            fs::read_to_string(directory.path().join(UPSTREAM_MANIFEST)).unwrap(),
            &format!("repository = {UPSTREAM_REPOSITORY:?}"),
            "repository = \"https://example.invalid/box2d.git\"",
        );
        write_upstream_manifest(&directory, untrusted_repository);
        let error = effective_source_identity(directory.path()).unwrap_err();
        assert_eq!(
            error,
            format!(
                "upstream manifest repository must be the official Box2D repository {UPSTREAM_REPOSITORY:?}"
            )
        );
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
