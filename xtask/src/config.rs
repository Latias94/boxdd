use std::{
    collections::BTreeSet,
    fs,
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tempfile::{Builder as TempfileBuilder, NamedTempFile};

use crate::{Error, Result, qualified_git::repository_lock_path};

pub const API_CONTRACT_SCHEMA: u32 = 8;
pub const UPSTREAM_MANIFEST_SCHEMA: u32 = 4;
pub const RECORDING_WIRE_SCHEMA: u32 = 4;

pub fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let source = fs::read_to_string(path).map_err(|source| Error::io(path, source))?;
    toml::from_str(&source)
        .map_err(|error| Error::message(format!("{}: invalid TOML: {error}", path.display())))
}

pub fn render_toml<T: Serialize>(value: &T) -> Result<String> {
    toml::to_string_pretty(value)
        .map_err(|error| Error::message(format!("could not serialize TOML: {error}")))
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    write_atomic_bytes(path, content.as_bytes())
}

pub fn write_atomic_bytes(path: &Path, content: &[u8]) -> Result<()> {
    write_atomic_bytes_with(path, content, || Ok(()))
}

pub(crate) fn write_atomic_bytes_with<F>(
    path: &Path,
    content: &[u8],
    before_commit: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(Error::message(format!(
                "{} is not a regular file and cannot be replaced",
                path.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io(path, error)),
    }

    let target_permissions = fs::metadata(path)
        .map(|metadata| Some(metadata.permissions()))
        .or_else(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(default_file_permissions())
            } else {
                Err(error)
            }
        })
        .map_err(|source| Error::io(path, source))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    if let Some(target_permissions) = target_permissions {
        temporary
            .as_file()
            .set_permissions(target_permissions)
            .map_err(|source| Error::io(temporary.path(), source))?;
    }
    temporary
        .as_file_mut()
        .write_all(content)
        .map_err(|source| Error::io(path, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(path, source))?;
    before_commit()?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| Error::io(path, error.error))
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicFileUpdate<'a> {
    path: &'a Path,
    expected: ExpectedAtomicFileState<'a>,
    content: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
enum ExpectedAtomicFileState<'a> {
    Existing(&'a [u8]),
    Missing,
}

impl<'a> AtomicFileUpdate<'a> {
    pub(crate) const fn checked(path: &'a Path, expected: &'a [u8], content: &'a [u8]) -> Self {
        Self {
            path,
            expected: ExpectedAtomicFileState::Existing(expected),
            content,
        }
    }

    pub(crate) const fn missing(path: &'a Path, content: &'a [u8]) -> Self {
        Self {
            path,
            expected: ExpectedAtomicFileState::Missing,
            content,
        }
    }
}

#[derive(Debug)]
struct PreparedAtomicFileUpdate<'a> {
    path: PathBuf,
    parent_identity: FileIdentity,
    original: FileState,
    installed_permission_identity: PermissionIdentity,
    content: &'a [u8],
}

impl PreparedAtomicFileUpdate<'_> {
    fn original_expectation(&self) -> ExpectedFileState<'_> {
        self.original.expectation()
    }

    fn is_noop(&self) -> bool {
        matches!(&self.original, FileState::Existing(original) if original.bytes == self.content)
    }
}

#[derive(Debug)]
enum FileState {
    Missing,
    Existing(FileGeneration),
}

impl FileState {
    fn expectation(&self) -> ExpectedFileState<'_> {
        match self {
            Self::Missing => ExpectedFileState::Missing,
            Self::Existing(generation) => ExpectedFileState::Existing(generation.expectation()),
        }
    }

    fn matches(&self, expected: ExpectedFileState<'_>) -> bool {
        match (self, expected) {
            (Self::Missing, ExpectedFileState::Missing) => true,
            (Self::Existing(actual), ExpectedFileState::Existing(expected)) => {
                actual.matches(expected)
            }
            (Self::Missing, ExpectedFileState::Existing(_))
            | (Self::Existing(_), ExpectedFileState::Missing) => false,
        }
    }
}

#[derive(Debug)]
struct FileGeneration {
    bytes: Vec<u8>,
    permission_identity: PermissionIdentity,
    file_identity: FileIdentity,
}

impl FileGeneration {
    fn expectation(&self) -> ExpectedFileGeneration<'_> {
        ExpectedFileGeneration {
            bytes: &self.bytes,
            permissions: self.permission_identity,
            file_identity: self.file_identity,
        }
    }

    fn matches(&self, expected: ExpectedFileGeneration<'_>) -> bool {
        self.bytes == expected.bytes
            && self.permission_identity == expected.permissions
            && self.file_identity == expected.file_identity
    }
}

#[derive(Clone, Copy, Debug)]
struct ExpectedFileGeneration<'a> {
    bytes: &'a [u8],
    permissions: PermissionIdentity,
    file_identity: FileIdentity,
}

#[derive(Clone, Copy, Debug)]
enum ExpectedFileState<'a> {
    Missing,
    Existing(ExpectedFileGeneration<'a>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    volume_serial_number: u32,
    #[cfg(windows)]
    file_index: u64,
}

impl FileIdentity {
    fn from_metadata(_path: &Path, metadata: &fs::Metadata) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;

            Ok(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt as _;

            let volume_serial_number = metadata.volume_serial_number().ok_or_else(|| {
                Error::message(format!(
                    "could not determine the volume identity for {}",
                    _path.display()
                ))
            })?;
            let file_index = metadata.file_index().ok_or_else(|| {
                Error::message(format!(
                    "could not determine the file identity for {}",
                    _path.display()
                ))
            })?;
            Ok(Self {
                volume_serial_number,
                file_index,
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = metadata;
            Err(Error::message(format!(
                "atomic batch file identity is unsupported on this platform: {}",
                _path.display()
            )))
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PermissionIdentity {
    #[cfg(unix)]
    mode: u32,
    #[cfg(not(unix))]
    readonly: bool,
}

impl PermissionIdentity {
    fn from_permissions(permissions: &fs::Permissions) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            Self {
                mode: permissions.mode() & 0o7777,
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                readonly: permissions.readonly(),
            }
        }
    }

    fn default_file() -> Self {
        #[cfg(unix)]
        {
            Self { mode: 0o644 }
        }
        #[cfg(not(unix))]
        {
            Self { readonly: false }
        }
    }
}

const ATOMIC_BATCH_RECOVERY_DIRECTORY: &str = "boxdd-atomic-batches";
const ATOMIC_BATCH_DRAFT_PREFIX: &str = "draft-";
const ATOMIC_BATCH_TRANSACTION_PREFIX: &str = "transaction-";
const ATOMIC_BATCH_CLEANUP_PREFIX: &str = "cleanup-";
const ATOMIC_BATCH_JOURNAL_FILE: &str = "journal.toml";
const ATOMIC_BATCH_JOURNAL_SCHEMA: u32 = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum AtomicBatchJournalState {
    Applying,
    Committed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AtomicBatchDirectoryPhase {
    Draft,
    Published,
    Cleanup,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AtomicBatchJournal {
    schema: u32,
    state: AtomicBatchJournalState,
    workspace_root: PathBuf,
    entries: Vec<AtomicBatchJournalEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AtomicBatchJournalEntry {
    update_index: usize,
    target: PathBuf,
    parent_identity: FileIdentity,
    original: StoredFileState,
    desired: StoredGeneration,
    apply_capture: String,
    rollback_capture: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum StoredFileState {
    Missing,
    Existing {
        generation: StoredGeneration,
        baseline_identity: FileIdentity,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredGeneration {
    file: String,
    content_blake3: String,
    permissions: PermissionIdentity,
    file_identity: FileIdentity,
}

impl StoredGeneration {
    fn content_matches(&self, generation: &FileGeneration) -> bool {
        blake3::hash(&generation.bytes).to_hex().as_str() == self.content_blake3
    }

    fn matches(&self, generation: &FileGeneration) -> bool {
        generation.permission_identity == self.permissions && self.content_matches(generation)
    }

    fn owns(&self, generation: &FileGeneration) -> bool {
        self.matches(generation) && generation.file_identity == self.file_identity
    }

    fn load(&self, transaction: &Path, label: &str) -> Result<FileGeneration> {
        let path = transaction_file(transaction, &self.file)?;
        let generation = read_file_generation(&path, label)?;
        if !self.owns(&generation) {
            return Err(Error::message(format!(
                "{label} changed in recovery transaction {}; generation preserved at {}",
                transaction.display(),
                path.display()
            )));
        }
        Ok(generation)
    }
}

#[derive(Debug)]
struct AtomicBatchTransaction {
    path: PathBuf,
    journal: AtomicBatchJournal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AtomicBatchHook {
    AfterInstallQuarantine(usize),
    BeforeMissingInstall(usize),
    AfterRollbackQuarantine(usize),
}

pub(crate) fn atomic_batch_recovery_root(root: &Path) -> Result<PathBuf> {
    let lock_path = repository_lock_path(root, Path::new("boxdd-upstream-sync.lock"))
        .map_err(Error::message)?;
    let common_directory = lock_path.parent().ok_or_else(|| {
        Error::message(format!(
            "repository update lock has no parent directory: {}",
            lock_path.display()
        ))
    })?;
    ensure_private_recovery_directory(&common_directory.join(ATOMIC_BATCH_RECOVERY_DIRECTORY))
}

pub(crate) fn recover_atomic_batches(root: &Path) -> Result<()> {
    let recovery_root = atomic_batch_recovery_root(root)?;
    recover_atomic_batches_at(&recovery_root, true)
}

pub(crate) fn ensure_no_pending_atomic_batches_for_workspace(root: &Path) -> Result<()> {
    let workspace_root = root
        .canonicalize()
        .map_err(|source| Error::io(root, source))?;
    let recovery_root = atomic_batch_recovery_root(&workspace_root)?;
    let entries =
        fs::read_dir(&recovery_root).map_err(|source| Error::io(&recovery_root, source))?;
    for entry in entries {
        let path = entry
            .map_err(|source| Error::io(&recovery_root, source))?
            .path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let is_published = name.starts_with(ATOMIC_BATCH_TRANSACTION_PREFIX);
        let is_cleanup = name.starts_with(ATOMIC_BATCH_CLEANUP_PREFIX);
        if !is_published && !is_cleanup {
            continue;
        }
        let journal_path = path.join(ATOMIC_BATCH_JOURNAL_FILE);
        let journal = match fs::symlink_metadata(&journal_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && is_cleanup => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::message(format!(
                    "published atomic batch recovery has no journal, so isolated workspace cleanup is unsafe: {}",
                    path.display()
                )));
            }
            Err(error) => return Err(Error::io(&journal_path, error)),
            Ok(metadata)
                if !metadata.file_type().is_file() || metadata.file_type().is_symlink() =>
            {
                return Err(Error::message(format!(
                    "atomic batch recovery journal is not a regular non-symlink file: {}",
                    journal_path.display()
                )));
            }
            Ok(_) => read_toml::<AtomicBatchJournal>(&journal_path)?,
        };
        if journal.workspace_root == workspace_root {
            return Err(Error::message(format!(
                "atomic batch recovery is still pending for isolated workspace {}; recovery transaction preserved at {}",
                workspace_root.display(),
                path.display()
            )));
        }
    }
    Ok(())
}

/// Installs a batch while the caller holds the repository `UpdateLock`.
pub(crate) fn write_atomic_batch_checked<F>(
    root: &Path,
    updates: &[AtomicFileUpdate<'_>],
    after_install: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let recovery_root = atomic_batch_recovery_root(root)?;
    recover_atomic_batches_at(&recovery_root, true)?;
    write_atomic_batch_in(root, &recovery_root, updates, |_| Ok(()), after_install)
}

#[cfg(test)]
fn write_atomic_batch_with<F>(updates: &[AtomicFileUpdate<'_>], mut before_install: F) -> Result<()>
where
    F: FnMut(usize) -> Result<()>,
{
    let workspace_root = test_batch_workspace_root(updates)?;
    let recovery_root = workspace_root.join(".boxdd-atomic-batches-test");
    write_atomic_batch_in(
        &workspace_root,
        &recovery_root,
        updates,
        |hook| match hook {
            AtomicBatchHook::AfterInstallQuarantine(index)
            | AtomicBatchHook::BeforeMissingInstall(index) => before_install(index),
            AtomicBatchHook::AfterRollbackQuarantine(_) => Ok(()),
        },
        || Ok(()),
    )
}

#[cfg(test)]
fn write_atomic_batch_with_checks<F, G>(
    updates: &[AtomicFileUpdate<'_>],
    after_quarantine: F,
    after_install: G,
) -> Result<()>
where
    F: FnMut(AtomicBatchHook) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    let workspace_root = test_batch_workspace_root(updates)?;
    let recovery_root = workspace_root.join(".boxdd-atomic-batches-test");
    write_atomic_batch_in(
        &workspace_root,
        &recovery_root,
        updates,
        after_quarantine,
        after_install,
    )
}

#[cfg(test)]
fn test_batch_workspace_root(updates: &[AtomicFileUpdate<'_>]) -> Result<PathBuf> {
    updates
        .first()
        .and_then(|update| update.path.parent())
        .ok_or_else(|| Error::message("test atomic batch has no target parent"))?
        .canonicalize()
        .map_err(|source| Error::io(updates[0].path, source))
}

fn write_atomic_batch_in<F, G>(
    workspace_root: &Path,
    recovery_root: &Path,
    updates: &[AtomicFileUpdate<'_>],
    mut after_quarantine: F,
    after_install: G,
) -> Result<()>
where
    F: FnMut(AtomicBatchHook) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|source| Error::io(workspace_root, source))?;
    let recovery_root = ensure_private_recovery_directory(recovery_root)?;
    let prepared = prepare_atomic_file_updates(&workspace_root, updates)?;
    validate_original_batch_states(&prepared)?;

    if prepared.iter().all(PreparedAtomicFileUpdate::is_noop) {
        after_install()?;
        return validate_original_batch_states(&prepared);
    }

    let mut transaction =
        prepare_atomic_batch_transaction(&workspace_root, &recovery_root, &prepared)?;
    let apply_result = apply_atomic_batch(
        &prepared,
        &transaction,
        &mut after_quarantine,
        after_install,
    );
    if let Err(original) = apply_result {
        return match rollback_atomic_batch_transaction(&transaction, &mut after_quarantine) {
            Ok(()) => Err(original),
            Err(rollback) => Err(Error::message(format!(
                "atomic file batch failed: {original}\nrollback could not finish and preserved its reported recovery state: {rollback}"
            ))),
        };
    }

    transaction.journal.state = AtomicBatchJournalState::Committed;
    if let Err(commit_error) =
        write_atomic_batch_journal(&transaction.path, &transaction.journal, false)
    {
        return Err(Error::message(format!(
            "atomic batch commit publication has an ambiguous durable outcome: {commit_error}; recovery transaction preserved at {} and must be resolved from its on-disk journal while holding UpdateLock",
            transaction.path.display()
        )));
    }
    finish_committed_atomic_batch(&transaction)
}

fn prepare_atomic_file_updates<'a>(
    workspace_root: &Path,
    updates: &[AtomicFileUpdate<'a>],
) -> Result<Vec<PreparedAtomicFileUpdate<'a>>> {
    if updates.is_empty() {
        return Err(Error::message(
            "atomic file batch requires at least one update",
        ));
    }
    let mut paths = BTreeSet::new();
    let mut prepared = Vec::with_capacity(updates.len());
    for update in updates {
        let path = canonical_batch_target(update.path)?;
        if !path.starts_with(workspace_root) {
            return Err(Error::message(format!(
                "atomic batch target escapes workspace {}: {}",
                workspace_root.display(),
                path.display()
            )));
        }
        if !paths.insert(path.clone()) {
            return Err(Error::message(format!(
                "atomic file batch contains duplicate target {}",
                path.display()
            )));
        }
        let original = read_file_state(&path, "atomic batch target")?;
        let parent_identity = read_target_parent_identity(&path)?;
        validate_expected_atomic_file_state(&path, &original, update.expected)?;
        let installed_permission_identity = match &original {
            FileState::Missing => {
                let permissions = default_file_permissions();
                permissions
                    .as_ref()
                    .map(PermissionIdentity::from_permissions)
                    .unwrap_or_else(PermissionIdentity::default_file)
            }
            FileState::Existing(original) => original.permission_identity,
        };
        prepared.push(PreparedAtomicFileUpdate {
            original,
            parent_identity,
            installed_permission_identity,
            path,
            content: update.content,
        });
    }
    Ok(prepared)
}

fn prepare_atomic_batch_transaction(
    workspace_root: &Path,
    recovery_root: &Path,
    updates: &[PreparedAtomicFileUpdate<'_>],
) -> Result<AtomicBatchTransaction> {
    let directory = TempfileBuilder::new()
        .prefix(ATOMIC_BATCH_DRAFT_PREFIX)
        .tempdir_in(recovery_root)
        .map_err(|source| Error::io(recovery_root, source))?;
    sync_directory(recovery_root)?;
    for update in updates.iter().filter(|update| !update.is_noop()) {
        ensure_same_filesystem(
            directory.path(),
            update.path.parent().expect("canonical target"),
        )?;
    }

    let mut entries = Vec::new();
    for (index, update) in updates.iter().enumerate() {
        if update.is_noop() {
            continue;
        }
        let desired = store_recovery_generation(
            directory.path(),
            format!("desired-{index}.bin"),
            update.content,
            update.installed_permission_identity,
        )?;
        let original = match &update.original {
            FileState::Missing => StoredFileState::Missing,
            FileState::Existing(original) => StoredFileState::Existing {
                generation: store_recovery_generation(
                    directory.path(),
                    format!("original-{index}.bin"),
                    &original.bytes,
                    original.permission_identity,
                )?,
                baseline_identity: original.file_identity,
            },
        };
        entries.push(AtomicBatchJournalEntry {
            update_index: index,
            target: update.path.clone(),
            parent_identity: update.parent_identity,
            original,
            desired,
            apply_capture: format!("apply-capture-{index}.bin"),
            rollback_capture: format!("rollback-capture-{index}.bin"),
        });
    }
    let journal = AtomicBatchJournal {
        schema: ATOMIC_BATCH_JOURNAL_SCHEMA,
        state: AtomicBatchJournalState::Applying,
        workspace_root: workspace_root.to_owned(),
        entries,
    };
    validate_atomic_batch_journal(directory.path(), &journal)?;
    write_atomic_batch_journal(directory.path(), &journal, true)?;
    sync_directory(directory.path())?;
    sync_directory(recovery_root)?;
    let draft_path = directory.keep();
    let path = transaction_phase_path(
        &draft_path,
        ATOMIC_BATCH_DRAFT_PREFIX,
        ATOMIC_BATCH_TRANSACTION_PREFIX,
    )?;
    ensure_path_missing(&path, "atomic batch publication destination")?;
    fs::rename(&draft_path, &path).map_err(|source| {
        Error::message(format!(
            "could not publish atomic batch draft {} as {}: {source}; draft preserved for cleanup",
            draft_path.display(),
            path.display()
        ))
    })?;
    sync_directory(recovery_root)?;
    Ok(AtomicBatchTransaction { path, journal })
}

fn store_recovery_generation(
    transaction: &Path,
    file: String,
    bytes: &[u8],
    permissions: PermissionIdentity,
) -> Result<StoredGeneration> {
    let path = transaction_file(transaction, &file)?;
    write_durable_new_file(&path, bytes)?;
    let stored_file = fs::File::open(&path).map_err(|source| Error::io(&path, source))?;
    let metadata = stored_file
        .metadata()
        .map_err(|source| Error::io(&path, source))?;
    stored_file
        .set_permissions(permissions_for_metadata(&metadata, permissions))
        .map_err(|source| Error::io(&path, source))?;
    stored_file
        .sync_all()
        .map_err(|source| Error::io(&path, source))?;
    sync_directory(transaction)?;
    let stored = read_file_generation(&path, "stored atomic batch recovery generation")?;
    Ok(StoredGeneration {
        file,
        content_blake3: blake3::hash(bytes).to_hex().to_string(),
        permissions,
        file_identity: stored.file_identity,
    })
}

fn apply_atomic_batch<F, G>(
    updates: &[PreparedAtomicFileUpdate<'_>],
    transaction: &AtomicBatchTransaction,
    after_quarantine: &mut F,
    after_install: G,
) -> Result<()>
where
    F: FnMut(AtomicBatchHook) -> Result<()>,
    G: FnOnce() -> Result<()>,
{
    let mut installed = Vec::new();
    for entry in &transaction.journal.entries {
        let index = entry.update_index;
        let update = &updates[index];
        validate_transaction_batch_states(updates, transaction, &installed, false)?;
        match &update.original {
            FileState::Existing(original) => {
                let capture = transaction_file(&transaction.path, &entry.apply_capture)?;
                move_target_into_transaction(
                    &update.path,
                    update.parent_identity,
                    &capture,
                    &transaction.path,
                )?;
                after_quarantine(AtomicBatchHook::AfterInstallQuarantine(index))?;
                let captured = read_file_generation(&capture, "captured atomic batch target")?;
                if !captured.matches(original.expectation()) {
                    return Err(Error::message(format!(
                        "atomic batch target changed before conditional replacement: {}; captured generation preserved at {}",
                        update.path.display(),
                        capture.display()
                    )));
                }
            }
            FileState::Missing => {
                after_quarantine(AtomicBatchHook::BeforeMissingInstall(index))?;
            }
        }
        install_stored_generation(
            &transaction.path,
            &entry.desired,
            &update.path,
            update.parent_identity,
            "desired atomic batch generation",
        )?;
        installed.push(index);
        validate_transaction_batch_states(updates, transaction, &installed, false)?;
    }
    validate_transaction_batch_states(updates, transaction, &installed, true)?;
    after_install()?;
    validate_transaction_batch_states(updates, transaction, &installed, true)
}

fn canonical_batch_target(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    let name = path
        .file_name()
        .ok_or_else(|| Error::message(format!("{} has no normal file name", path.display())))?;
    let metadata = fs::symlink_metadata(parent).map_err(|source| Error::io(parent, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "atomic batch parent must be a real directory: {}",
            parent.display()
        )));
    }
    let canonical_parent = parent
        .canonicalize()
        .map_err(|source| Error::io(parent, source))?;
    let canonical_metadata =
        fs::symlink_metadata(&canonical_parent).map_err(|source| Error::io(parent, source))?;
    if !canonical_metadata.file_type().is_dir() || canonical_metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "atomic batch parent must remain a real directory: {}",
            parent.display()
        )));
    }
    Ok(canonical_parent.join(name))
}

fn read_target_parent_identity(path: &Path) -> Result<FileIdentity> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    let metadata = fs::symlink_metadata(parent).map_err(|source| Error::io(parent, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "atomic batch parent must be a real directory: {}",
            parent.display()
        )));
    }
    FileIdentity::from_metadata(parent, &metadata)
}

// This closes accidental/concurrent parent replacement around mutations. Defending against a
// same-UID attacker swapping the directory between this check and the syscall requires
// platform-specific directory capabilities and is outside the update command's threat model.
fn validate_target_parent_identity(path: &Path, expected: FileIdentity) -> Result<()> {
    let canonical = canonical_batch_target(path)?;
    let actual = read_target_parent_identity(&canonical)?;
    if canonical == path && actual == expected {
        Ok(())
    } else {
        Err(Error::message(format!(
            "atomic batch target parent changed concurrently: {}",
            path.display()
        )))
    }
}

fn validate_expected_atomic_file_state(
    path: &Path,
    original: &FileState,
    expected: ExpectedAtomicFileState<'_>,
) -> Result<()> {
    let matches = match (original, expected) {
        (FileState::Existing(original), ExpectedAtomicFileState::Existing(expected)) => {
            original.bytes == expected
        }
        (FileState::Missing, ExpectedAtomicFileState::Missing) => true,
        (FileState::Missing, ExpectedAtomicFileState::Existing(_))
        | (FileState::Existing(_), ExpectedAtomicFileState::Missing) => false,
    };
    if matches {
        Ok(())
    } else {
        Err(Error::message(format!(
            "atomic batch target changed since its baseline was captured: {}",
            path.display()
        )))
    }
}

fn read_file_state(path: &Path, label: &str) -> Result<FileState> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            read_file_generation_with_metadata(path, label, metadata).map(FileState::Existing)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileState::Missing),
        Err(error) => Err(Error::io(path, error)),
    }
}

fn read_file_generation(path: &Path, label: &str) -> Result<FileGeneration> {
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    read_file_generation_with_metadata(path, label, metadata)
}

fn read_file_generation_with_metadata(
    path: &Path,
    label: &str,
    metadata: fs::Metadata,
) -> Result<FileGeneration> {
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "{label} must be an existing regular non-symlink file: {}",
            path.display()
        )));
    }
    let mut file = fs::File::open(path).map_err(|source| Error::io(path, source))?;
    let opened_metadata = file.metadata().map_err(|source| Error::io(path, source))?;
    if !opened_metadata.file_type().is_file() {
        return Err(Error::message(format!(
            "{label} changed while it was being opened: {}",
            path.display()
        )));
    }
    let path_identity = FileIdentity::from_metadata(path, &metadata)?;
    let opened_identity = FileIdentity::from_metadata(path, &opened_metadata)?;
    if path_identity != opened_identity {
        return Err(Error::message(format!(
            "{label} changed while it was being opened: {}",
            path.display()
        )));
    }
    let permission_identity = PermissionIdentity::from_permissions(&opened_metadata.permissions());
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|source| Error::io(path, source))?;
    let final_opened_metadata = file.metadata().map_err(|source| Error::io(path, source))?;
    let final_path_metadata =
        fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    let final_opened_identity = FileIdentity::from_metadata(path, &final_opened_metadata)?;
    let final_path_identity = FileIdentity::from_metadata(path, &final_path_metadata)?;
    if final_opened_identity != opened_identity || final_path_identity != opened_identity {
        return Err(Error::message(format!(
            "{label} changed while it was being read: {}",
            path.display()
        )));
    }
    Ok(FileGeneration {
        bytes,
        permission_identity,
        file_identity: opened_identity,
    })
}

fn validate_original_batch_states(updates: &[PreparedAtomicFileUpdate<'_>]) -> Result<()> {
    for update in updates {
        validate_target_parent_identity(&update.path, update.parent_identity)?;
        let actual = read_file_state(&update.path, "atomic batch target")?;
        if !actual.matches(update.original_expectation()) {
            return Err(Error::message(format!(
                "atomic batch target changed concurrently: {}",
                update.path.display()
            )));
        }
    }
    Ok(())
}

fn validate_transaction_batch_states(
    updates: &[PreparedAtomicFileUpdate<'_>],
    transaction: &AtomicBatchTransaction,
    installed: &[usize],
    require_desired: bool,
) -> Result<()> {
    for (index, update) in updates.iter().enumerate() {
        validate_target_parent_identity(&update.path, update.parent_identity)?;
        let actual = read_file_state(&update.path, "atomic batch target")?;
        let matches = if require_desired || installed.contains(&index) {
            if update.is_noop() {
                actual.matches(update.original_expectation())
            } else {
                let entry = transaction
                    .journal
                    .entries
                    .iter()
                    .find(|entry| entry.update_index == index)
                    .expect("non-noop update must have a journal entry");
                file_state_is_owned_generation(&actual, &entry.desired)
            }
        } else {
            actual.matches(update.original_expectation())
        };
        if !matches {
            return Err(Error::message(format!(
                "atomic batch target changed concurrently: {}",
                update.path.display()
            )));
        }
    }
    Ok(())
}

fn ensure_private_recovery_directory(path: &Path) -> Result<PathBuf> {
    match fs::create_dir(path) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                    .map_err(|source| Error::io(path, source))?;
            }
            sync_target_parent(path)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(Error::io(path, error)),
    }
    let metadata = fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "atomic batch recovery root must be a real directory: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::message(format!(
                "atomic batch recovery root must not grant group or other access: {}",
                path.display()
            )));
        }
        let parent = path.parent().ok_or_else(|| {
            Error::message(format!(
                "atomic batch recovery root has no parent: {}",
                path.display()
            ))
        })?;
        let parent_metadata = fs::metadata(parent).map_err(|source| Error::io(parent, source))?;
        if metadata.uid() != parent_metadata.uid() {
            return Err(Error::message(format!(
                "atomic batch recovery root owner differs from its parent: {}",
                path.display()
            )));
        }
    }
    path.canonicalize()
        .map_err(|source| Error::io(path, source))
}

fn recover_atomic_batches_at(recovery_root: &Path, enforce_repository_scope: bool) -> Result<()> {
    let recovery_root = ensure_private_recovery_directory(recovery_root)?;
    let mut transactions = fs::read_dir(&recovery_root)
        .map_err(|source| Error::io(&recovery_root, source))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|source| Error::io(&recovery_root, source))?;
    transactions.sort();
    let mut errors = Vec::new();
    for transaction in transactions {
        let name = transaction
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let phase = if name.starts_with(ATOMIC_BATCH_DRAFT_PREFIX) {
            AtomicBatchDirectoryPhase::Draft
        } else if name.starts_with(ATOMIC_BATCH_TRANSACTION_PREFIX) {
            AtomicBatchDirectoryPhase::Published
        } else if name.starts_with(ATOMIC_BATCH_CLEANUP_PREFIX) {
            AtomicBatchDirectoryPhase::Cleanup
        } else {
            errors.push(format!(
                "unexpected entry in atomic batch recovery root; preserved at {}",
                transaction.display()
            ));
            continue;
        };
        let metadata = match fs::symlink_metadata(&transaction) {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(format!(
                    "could not inspect recovery transaction {}: {error}",
                    transaction.display()
                ));
                continue;
            }
        };
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            errors.push(format!(
                "recovery transaction is not a real directory; preserved at {}",
                transaction.display()
            ));
            continue;
        }
        if phase == AtomicBatchDirectoryPhase::Draft {
            if let Err(error) = remove_transaction_directory(&transaction) {
                errors.push(format!(
                    "could not remove atomic batch draft at {}: {error}",
                    transaction.display()
                ));
            }
            continue;
        }
        let journal_path = transaction.join(ATOMIC_BATCH_JOURNAL_FILE);
        let journal = match fs::symlink_metadata(&journal_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if phase == AtomicBatchDirectoryPhase::Cleanup {
                    if let Err(error) = remove_transaction_directory(&transaction) {
                        errors.push(format!(
                            "could not finish journal-free cleanup directory at {}: {error}",
                            transaction.display()
                        ));
                    }
                } else {
                    errors.push(format!(
                        "published recovery transaction has no journal and was preserved at {}",
                        transaction.display()
                    ));
                }
                continue;
            }
            Err(error) => {
                errors.push(format!(
                    "could not inspect recovery journal {}; transaction preserved at {}: {error}",
                    journal_path.display(),
                    transaction.display()
                ));
                continue;
            }
            Ok(metadata)
                if !metadata.file_type().is_file() || metadata.file_type().is_symlink() =>
            {
                errors.push(format!(
                    "recovery journal is not a regular non-symlink file; transaction preserved at {}",
                    transaction.display()
                ));
                continue;
            }
            Ok(_) => match read_toml::<AtomicBatchJournal>(&journal_path) {
                Ok(journal) => journal,
                Err(error) => {
                    errors.push(format!(
                        "invalid recovery journal {}; transaction preserved at {}: {error}",
                        journal_path.display(),
                        transaction.display()
                    ));
                    continue;
                }
            },
        };
        let recovery = (|| {
            validate_atomic_batch_journal(&transaction, &journal)?;
            if enforce_repository_scope {
                let journal_recovery_root = atomic_batch_recovery_root(&journal.workspace_root)?;
                if journal_recovery_root != recovery_root {
                    return Err(Error::message(format!(
                        "recovery journal workspace {} belongs to {}, not {}",
                        journal.workspace_root.display(),
                        journal_recovery_root.display(),
                        recovery_root.display()
                    )));
                }
            }
            let transaction = AtomicBatchTransaction {
                path: transaction.clone(),
                journal,
            };
            if phase == AtomicBatchDirectoryPhase::Cleanup {
                finish_cleanup_transaction(&transaction)
            } else {
                match transaction.journal.state {
                    AtomicBatchJournalState::Applying => {
                        rollback_atomic_batch_transaction(&transaction, &mut |_| Ok(()))
                    }
                    AtomicBatchJournalState::Committed => {
                        finish_committed_atomic_batch(&transaction)
                    }
                }
            }
        })();
        if let Err(error) = recovery {
            errors.push(format!(
                "recovery transaction preserved at {}: {error}",
                transaction.display()
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(format!(
            "atomic batch recovery could not finish:\n{}",
            errors.join("\n")
        )))
    }
}

fn validate_atomic_batch_journal(transaction: &Path, journal: &AtomicBatchJournal) -> Result<()> {
    if journal.schema != ATOMIC_BATCH_JOURNAL_SCHEMA {
        return Err(Error::message(format!(
            "unsupported atomic batch journal schema {} in {}",
            journal.schema,
            transaction.display()
        )));
    }
    if journal.entries.is_empty() {
        return Err(Error::message(format!(
            "atomic batch journal has no entries: {}",
            transaction.display()
        )));
    }
    let workspace_root = journal
        .workspace_root
        .canonicalize()
        .map_err(|source| Error::io(&journal.workspace_root, source))?;
    if workspace_root != journal.workspace_root {
        return Err(Error::message(format!(
            "atomic batch journal workspace is not canonical: {}",
            journal.workspace_root.display()
        )));
    }
    let mut targets = BTreeSet::new();
    let mut indexes = BTreeSet::new();
    let mut files = BTreeSet::new();
    for entry in &journal.entries {
        if !indexes.insert(entry.update_index) {
            return Err(Error::message(format!(
                "atomic batch journal contains duplicate update index {}: {}",
                entry.update_index,
                transaction.display()
            )));
        }
        let target = canonical_batch_target(&entry.target)?;
        if target != entry.target || !target.starts_with(&workspace_root) {
            return Err(Error::message(format!(
                "atomic batch journal target escapes its workspace: {}",
                entry.target.display()
            )));
        }
        validate_target_parent_identity(&target, entry.parent_identity)?;
        if !targets.insert(target.clone()) {
            return Err(Error::message(format!(
                "atomic batch journal contains duplicate target {}",
                target.display()
            )));
        }
        ensure_same_filesystem(transaction, target.parent().expect("canonical target"))?;
        validate_unique_transaction_file(transaction, &entry.desired.file, &mut files)?;
        if let StoredFileState::Existing { generation, .. } = &entry.original {
            validate_unique_transaction_file(transaction, &generation.file, &mut files)?;
        }
        validate_unique_transaction_file(transaction, &entry.apply_capture, &mut files)?;
        validate_unique_transaction_file(transaction, &entry.rollback_capture, &mut files)?;
    }
    Ok(())
}

fn validate_unique_transaction_file(
    transaction: &Path,
    file: &str,
    files: &mut BTreeSet<String>,
) -> Result<()> {
    let _ = transaction_file(transaction, file)?;
    if files.insert(file.to_owned()) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "atomic batch journal reuses recovery file {file}: {}",
            transaction.display()
        )))
    }
}

fn rollback_atomic_batch_transaction<F>(
    transaction: &AtomicBatchTransaction,
    after_quarantine: &mut F,
) -> Result<()>
where
    F: FnMut(AtomicBatchHook) -> Result<()>,
{
    if all_original_targets_restored(transaction)? {
        return remove_completed_transaction(transaction, false);
    }
    let mut errors = Vec::new();
    for entry in transaction.journal.entries.iter().rev() {
        if let Err(error) = rollback_atomic_batch_entry(transaction, entry, after_quarantine) {
            errors.push(error.to_string());
        }
    }
    if errors.is_empty() {
        for entry in &transaction.journal.entries {
            validate_target_parent_identity(&entry.target, entry.parent_identity)?;
            let current = read_file_state(&entry.target, "rolled back atomic batch target")?;
            if !stored_file_state_matches(&entry.original, &current) {
                errors.push(format!(
                    "rolled back target does not match its original generation: {}",
                    entry.target.display()
                ));
            }
        }
    }
    if errors.is_empty() {
        remove_completed_transaction(transaction, false)
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn all_original_targets_restored(transaction: &AtomicBatchTransaction) -> Result<bool> {
    for entry in &transaction.journal.entries {
        validate_target_parent_identity(&entry.target, entry.parent_identity)?;
        let current = read_file_state(&entry.target, "atomic batch rollback target")?;
        if stored_file_state_matches(&entry.original, &current) {
            continue;
        }
        if repair_original_permissions_if_only_mode_differs(entry, &current)? {
            continue;
        }
        return Ok(false);
    }
    Ok(true)
}

fn rollback_atomic_batch_entry<F>(
    transaction: &AtomicBatchTransaction,
    entry: &AtomicBatchJournalEntry,
    after_quarantine: &mut F,
) -> Result<()>
where
    F: FnMut(AtomicBatchHook) -> Result<()>,
{
    validate_target_parent_identity(&entry.target, entry.parent_identity)?;
    let apply_capture = transaction_file(&transaction.path, &entry.apply_capture)?;
    if let Some(captured) = read_optional_generation(&apply_capture, "atomic batch apply capture")?
    {
        let expected = match &entry.original {
            StoredFileState::Missing => false,
            StoredFileState::Existing {
                generation,
                baseline_identity,
            } => generation.matches(&captured) && captured.file_identity == *baseline_identity,
        };
        if !expected {
            restore_conflicting_capture(
                &entry.target,
                entry.parent_identity,
                &apply_capture,
                &captured,
            )?;
            return Err(Error::message(format!(
                "rollback conflict at {}: apply capture does not match the journaled original; conflicting generation preserved at {}; recovery transaction preserved at {}",
                entry.target.display(),
                apply_capture.display(),
                transaction.path.display()
            )));
        }
    }

    let rollback_capture = transaction_file(&transaction.path, &entry.rollback_capture)?;
    if let Some(captured) =
        read_optional_generation(&rollback_capture, "atomic batch rollback capture")?
        && !entry.desired.owns(&captured)
    {
        restore_conflicting_capture(
            &entry.target,
            entry.parent_identity,
            &rollback_capture,
            &captured,
        )?;
        return Err(Error::message(format!(
            "rollback conflict at {}: rollback capture does not match the journaled desired generation; conflicting generation preserved at {}; recovery transaction preserved at {}",
            entry.target.display(),
            rollback_capture.display(),
            transaction.path.display()
        )));
    }

    let current = read_file_state(&entry.target, "atomic batch rollback target")?;
    if stored_file_state_matches(&entry.original, &current) {
        return Ok(());
    }
    if repair_original_permissions_if_only_mode_differs(entry, &current)? {
        return Ok(());
    }
    if file_state_is_owned_generation(&current, &entry.desired) {
        if fs::symlink_metadata(&rollback_capture).is_ok() {
            return Err(Error::message(format!(
                "rollback conflict at {}: desired target was recreated after it was captured at {}; recovery transaction preserved at {}",
                entry.target.display(),
                rollback_capture.display(),
                transaction.path.display()
            )));
        }
        move_target_into_transaction(
            &entry.target,
            entry.parent_identity,
            &rollback_capture,
            &transaction.path,
        )?;
        let hook_error =
            after_quarantine(AtomicBatchHook::AfterRollbackQuarantine(entry.update_index)).err();
        let captured = read_file_generation(&rollback_capture, "atomic batch rollback capture")?;
        if !entry.desired.owns(&captured) {
            restore_conflicting_capture(
                &entry.target,
                entry.parent_identity,
                &rollback_capture,
                &captured,
            )?;
            return Err(Error::message(format!(
                "rollback conflict at {}: target changed while it was captured; conflicting generation preserved at {}; recovery transaction preserved at {}",
                entry.target.display(),
                rollback_capture.display(),
                transaction.path.display()
            )));
        }
        restore_original_generation(transaction, entry).map_err(|error| {
            Error::message(format!(
                "could not restore the original generation at {}; recovery transaction preserved at {}: {error}",
                entry.target.display(),
                transaction.path.display()
            ))
        })?;
        if let Some(error) = hook_error {
            return Err(Error::message(format!(
                "rollback hook failed for {}; recovery transaction preserved at {}: {error}",
                entry.target.display(),
                transaction.path.display()
            )));
        }
    } else if matches!(current, FileState::Missing) {
        restore_original_generation(transaction, entry).map_err(|error| {
            Error::message(format!(
                "could not restore the original generation at {}; recovery transaction preserved at {}: {error}",
                entry.target.display(),
                transaction.path.display()
            ))
        })?;
    } else {
        return Err(Error::message(format!(
            "rollback conflict at {}: target matches neither the original nor desired generation; recovery transaction preserved at {}",
            entry.target.display(),
            transaction.path.display()
        )));
    }

    let restored = read_file_state(&entry.target, "restored atomic batch target")?;
    if stored_file_state_matches(&entry.original, &restored) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "rollback conflict at {}: restored target changed; recovery transaction preserved at {}",
            entry.target.display(),
            transaction.path.display()
        )))
    }
}

fn restore_original_generation(
    transaction: &AtomicBatchTransaction,
    entry: &AtomicBatchJournalEntry,
) -> Result<()> {
    match &entry.original {
        StoredFileState::Missing => Ok(()),
        StoredFileState::Existing {
            generation,
            baseline_identity,
        } => {
            let apply_capture = transaction_file(&transaction.path, &entry.apply_capture)?;
            if let Some(captured) =
                read_optional_generation(&apply_capture, "atomic batch apply capture")?
            {
                if !generation.matches(&captured) || captured.file_identity != *baseline_identity {
                    return Err(Error::message(format!(
                        "journaled original capture changed; recovery transaction preserved at {}",
                        transaction.path.display()
                    )));
                }
                link_recovery_file_noclobber(&apply_capture, &entry.target, entry.parent_identity)
            } else {
                install_stored_generation(
                    &transaction.path,
                    generation,
                    &entry.target,
                    entry.parent_identity,
                    "journaled original generation",
                )
            }
        }
    }
}

fn restore_conflicting_capture(
    target: &Path,
    parent_identity: FileIdentity,
    capture: &Path,
    captured: &FileGeneration,
) -> Result<()> {
    validate_target_parent_identity(target, parent_identity)?;
    match read_file_state(target, "conflicting atomic batch target")? {
        FileState::Missing => {
            link_recovery_file_noclobber(capture, target, parent_identity)?;
            let restored = read_file_generation(target, "restored conflicting generation")?;
            if restored.matches(captured.expectation()) {
                Ok(())
            } else {
                Err(Error::message(format!(
                    "conflicting generation from {} changed while restoring {}",
                    capture.display(),
                    target.display()
                )))
            }
        }
        FileState::Existing(_) => Ok(()),
    }
}

fn install_stored_generation(
    transaction: &Path,
    generation: &StoredGeneration,
    target: &Path,
    parent_identity: FileIdentity,
    label: &str,
) -> Result<()> {
    let source = transaction_file(transaction, &generation.file)?;
    let _ = generation.load(transaction, label)?;
    link_recovery_file_noclobber(&source, target, parent_identity)?;
    let installed = read_file_generation(target, "installed atomic batch generation")?;
    if generation.owns(&installed) {
        Ok(())
    } else {
        Err(Error::message(format!(
            "installed atomic batch generation changed at {}; recovery source preserved at {}",
            target.display(),
            source.display()
        )))
    }
}

fn link_recovery_file_noclobber(
    source: &Path,
    target: &Path,
    parent_identity: FileIdentity,
) -> Result<()> {
    validate_target_parent_identity(target, parent_identity)?;
    match fs::symlink_metadata(target) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(Error::message(format!(
                "{} already exists; refusing to overwrite a concurrent state",
                target.display()
            )));
        }
        Err(error) => return Err(Error::io(target, error)),
    }
    fs::hard_link(source, target).map_err(|source_error| {
        if source_error.kind() == std::io::ErrorKind::AlreadyExists {
            Error::message(format!(
                "{} already exists; refusing to overwrite a concurrent state",
                target.display()
            ))
        } else {
            Error::message(format!(
                "could not link recovery generation {} to {}: {source_error}",
                source.display(),
                target.display()
            ))
        }
    })?;
    validate_target_parent_identity(target, parent_identity)?;
    sync_target_parent(target)
}

fn finish_committed_atomic_batch(transaction: &AtomicBatchTransaction) -> Result<()> {
    let mut errors = Vec::new();
    for entry in &transaction.journal.entries {
        if let Err(error) = validate_target_parent_identity(&entry.target, entry.parent_identity) {
            errors.push(error.to_string());
            continue;
        }
        match read_file_state(&entry.target, "committed atomic batch target") {
            Ok(current) if file_state_is_owned_generation(&current, &entry.desired) => {}
            Ok(FileState::Existing(current))
                if current.file_identity == entry.desired.file_identity
                    && entry.desired.content_matches(&current) =>
            {
                if let Err(error) = set_target_permissions(
                    &entry.target,
                    entry.parent_identity,
                    current.file_identity,
                    entry.desired.permissions,
                ) {
                    errors.push(error.to_string());
                } else {
                    match read_file_generation(
                        &entry.target,
                        "permission-repaired committed target",
                    ) {
                        Ok(repaired) if entry.desired.owns(&repaired) => {}
                        Ok(_) => errors.push(format!(
                            "committed target changed while repairing permissions: {}",
                            entry.target.display()
                        )),
                        Err(error) => errors.push(error.to_string()),
                    }
                }
            }
            Ok(_) => errors.push(format!(
                "committed target no longer matches the desired generation: {}",
                entry.target.display()
            )),
            Err(error) => errors.push(error.to_string()),
        }
        let rollback_capture = transaction_file(&transaction.path, &entry.rollback_capture)?;
        if fs::symlink_metadata(&rollback_capture).is_ok() {
            errors.push(format!(
                "committed transaction unexpectedly contains a rollback capture: {}",
                rollback_capture.display()
            ));
        }
        let apply_capture = transaction_file(&transaction.path, &entry.apply_capture)?;
        if let Some(captured) =
            read_optional_generation(&apply_capture, "committed atomic batch apply capture")?
        {
            let matches = match &entry.original {
                StoredFileState::Missing => false,
                StoredFileState::Existing {
                    generation,
                    baseline_identity,
                } => generation.matches(&captured) && captured.file_identity == *baseline_identity,
            };
            if !matches {
                errors.push(format!(
                    "committed transaction apply capture changed; preserved at {}",
                    apply_capture.display()
                ));
            }
        }
    }
    if errors.is_empty() {
        remove_completed_transaction(transaction, true)
    } else {
        Err(Error::message(format!(
            "committed recovery transaction preserved at {}:\n{}",
            transaction.path.display(),
            errors.join("\n")
        )))
    }
}

fn stored_file_state_matches(expected: &StoredFileState, actual: &FileState) -> bool {
    match (expected, actual) {
        (StoredFileState::Missing, FileState::Missing) => true,
        (
            StoredFileState::Existing {
                generation: expected,
                baseline_identity,
            },
            FileState::Existing(actual),
        ) => {
            expected.matches(actual)
                && (actual.file_identity == *baseline_identity
                    || actual.file_identity == expected.file_identity)
        }
        (StoredFileState::Missing, FileState::Existing(_))
        | (StoredFileState::Existing { .. }, FileState::Missing) => false,
    }
}

fn file_state_is_owned_generation(actual: &FileState, expected: &StoredGeneration) -> bool {
    matches!(actual, FileState::Existing(actual) if expected.owns(actual))
}

fn repair_original_permissions_if_only_mode_differs(
    entry: &AtomicBatchJournalEntry,
    current: &FileState,
) -> Result<bool> {
    let (
        StoredFileState::Existing {
            generation: original,
            baseline_identity,
        },
        FileState::Existing(current),
    ) = (&entry.original, current)
    else {
        return Ok(false);
    };
    if !original.content_matches(current)
        || (current.file_identity != *baseline_identity
            && current.file_identity != original.file_identity)
    {
        return Ok(false);
    }
    set_target_permissions(
        &entry.target,
        entry.parent_identity,
        current.file_identity,
        original.permissions,
    )?;
    let repaired = read_file_generation(&entry.target, "permission-repaired original target")?;
    Ok(stored_file_state_matches(
        &entry.original,
        &FileState::Existing(repaired),
    ))
}

fn read_optional_generation(path: &Path, label: &str) -> Result<Option<FileGeneration>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => read_file_generation_with_metadata(path, label, metadata).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::io(path, error)),
    }
}

fn move_target_into_transaction(
    target: &Path,
    parent_identity: FileIdentity,
    destination: &Path,
    transaction: &Path,
) -> Result<()> {
    validate_target_parent_identity(target, parent_identity)?;
    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(Error::message(format!(
                "atomic batch recovery capture already exists: {}",
                destination.display()
            )));
        }
        Err(error) => return Err(Error::io(destination, error)),
    }
    fs::rename(target, destination).map_err(|source| Error::io(target, source))?;
    validate_target_parent_identity(target, parent_identity)?;
    sync_target_parent(target)?;
    sync_directory(transaction)
}

fn write_atomic_batch_journal(
    transaction: &Path,
    journal: &AtomicBatchJournal,
    create_new: bool,
) -> Result<()> {
    let path = transaction.join(ATOMIC_BATCH_JOURNAL_FILE);
    let source = render_toml(journal)?;
    if create_new {
        write_durable_new_file(&path, source.as_bytes())
    } else {
        write_durable_replacement(&path, source.as_bytes())
    }
}

fn write_durable_new_file(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    temporary
        .as_file_mut()
        .write_all(content)
        .map_err(|source| Error::io(path, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(path, source))?;
    let persisted = temporary
        .persist_noclobber(path)
        .map_err(|error| Error::io(path, error.error))?;
    drop(persisted);
    sync_directory(parent)
}

fn write_durable_replacement(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    temporary
        .as_file_mut()
        .write_all(content)
        .map_err(|source| Error::io(path, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(path, source))?;
    let persisted = temporary
        .persist(path)
        .map_err(|error| Error::io(path, error.error))?;
    drop(persisted);
    sync_directory(parent)
}

fn transaction_file(transaction: &Path, file: &str) -> Result<PathBuf> {
    let path = Path::new(file);
    if path.components().count() != 1
        || !matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(Error::message(format!(
            "atomic batch recovery file must be one normal path component: {file}"
        )));
    }
    Ok(transaction.join(path))
}

fn transaction_phase_path(path: &Path, from_prefix: &str, to_prefix: &str) -> Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Error::message(format!(
                "atomic batch directory name is not UTF-8: {}",
                path.display()
            ))
        })?;
    let suffix = name.strip_prefix(from_prefix).ok_or_else(|| {
        Error::message(format!(
            "atomic batch directory {} does not start with {from_prefix}",
            path.display()
        ))
    })?;
    if suffix.is_empty() {
        return Err(Error::message(format!(
            "atomic batch directory has no unique suffix: {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        Error::message(format!(
            "atomic batch directory has no parent: {}",
            path.display()
        ))
    })?;
    Ok(parent.join(format!("{to_prefix}{suffix}")))
}

fn ensure_path_missing(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(Error::message(format!(
            "{label} already exists and will not be replaced: {}",
            path.display()
        ))),
        Err(error) => Err(Error::io(path, error)),
    }
}

fn remove_completed_transaction(
    transaction: &AtomicBatchTransaction,
    keep_desired: bool,
) -> Result<()> {
    validate_completed_targets(transaction, keep_desired)?;
    let cleanup_path = transaction_phase_path(
        &transaction.path,
        ATOMIC_BATCH_TRANSACTION_PREFIX,
        ATOMIC_BATCH_CLEANUP_PREFIX,
    )?;
    ensure_path_missing(&cleanup_path, "atomic batch cleanup destination")?;
    fs::rename(&transaction.path, &cleanup_path).map_err(|source| {
        Error::message(format!(
            "could not transition completed recovery transaction {} to cleanup directory {}: {source}; published transaction preserved",
            transaction.path.display(),
            cleanup_path.display()
        ))
    })?;
    sync_target_parent(&cleanup_path)?;
    finish_cleanup_at(&cleanup_path, &transaction.journal)
}

fn finish_cleanup_transaction(transaction: &AtomicBatchTransaction) -> Result<()> {
    finish_cleanup_at(&transaction.path, &transaction.journal)
}

fn finish_cleanup_at(path: &Path, journal: &AtomicBatchJournal) -> Result<()> {
    let keep_desired = journal.state == AtomicBatchJournalState::Committed;
    validate_cleanup_targets_owned(path, journal, keep_desired)?;
    let journal_path = path.join(ATOMIC_BATCH_JOURNAL_FILE);
    let files = validated_transaction_files(path)?;
    for (path, permissions) in files.into_iter().filter(|(path, _)| path != &journal_path) {
        remove_recovery_file(&path, permissions)?;
    }
    sync_directory(path)?;

    for entry in &journal.entries {
        let permissions = if keep_desired {
            Some(entry.desired.permissions)
        } else {
            match &entry.original {
                StoredFileState::Missing => None,
                StoredFileState::Existing { generation, .. } => Some(generation.permissions),
            }
        };
        if let Some(permissions) = permissions {
            set_owned_target_permissions(entry, keep_desired, permissions)?;
        }
    }
    validate_completed_targets_at(path, journal, keep_desired)?;

    let journal_metadata =
        fs::symlink_metadata(&journal_path).map_err(|source| Error::io(&journal_path, source))?;
    remove_recovery_file(&journal_path, journal_metadata.permissions())?;
    sync_directory(path)?;
    fs::remove_dir(path).map_err(|source| Error::io(path, source))?;
    sync_target_parent(path)
}

fn validate_cleanup_targets_owned(
    recovery_path: &Path,
    journal: &AtomicBatchJournal,
    keep_desired: bool,
) -> Result<()> {
    for entry in &journal.entries {
        validate_target_parent_identity(&entry.target, entry.parent_identity)?;
        let current = read_file_state(&entry.target, "atomic batch cleanup target")?;
        let owned = if keep_desired {
            matches!(
                &current,
                FileState::Existing(current)
                    if current.file_identity == entry.desired.file_identity
                        && entry.desired.content_matches(current)
            )
        } else {
            match (&entry.original, &current) {
                (StoredFileState::Missing, FileState::Missing) => true,
                (
                    StoredFileState::Existing {
                        generation,
                        baseline_identity,
                    },
                    FileState::Existing(current),
                ) => {
                    generation.content_matches(current)
                        && (current.file_identity == *baseline_identity
                            || current.file_identity == generation.file_identity)
                }
                _ => false,
            }
        };
        if !owned {
            return Err(Error::message(format!(
                "cleanup target is no longer transaction-owned: {}; cleanup directory preserved at {}",
                entry.target.display(),
                recovery_path.display()
            )));
        }
    }
    Ok(())
}

fn validate_completed_targets(
    transaction: &AtomicBatchTransaction,
    keep_desired: bool,
) -> Result<()> {
    validate_completed_targets_at(&transaction.path, &transaction.journal, keep_desired)
}

fn validate_completed_targets_at(
    recovery_path: &Path,
    journal: &AtomicBatchJournal,
    keep_desired: bool,
) -> Result<()> {
    for entry in &journal.entries {
        validate_target_parent_identity(&entry.target, entry.parent_identity)?;
        let current = read_file_state(&entry.target, "completed atomic batch target")?;
        let matches = if keep_desired {
            file_state_is_owned_generation(&current, &entry.desired)
        } else {
            stored_file_state_matches(&entry.original, &current)
        };
        if !matches {
            return Err(Error::message(format!(
                "completed atomic batch target changed before journal cleanup: {}; recovery transaction preserved at {}",
                entry.target.display(),
                recovery_path.display()
            )));
        }
    }
    Ok(())
}

fn set_owned_target_permissions(
    entry: &AtomicBatchJournalEntry,
    keep_desired: bool,
    permissions: PermissionIdentity,
) -> Result<()> {
    validate_target_parent_identity(&entry.target, entry.parent_identity)?;
    let current = read_file_state(&entry.target, "completed atomic batch permission target")?;
    let expected_identity = if keep_desired {
        match &current {
            FileState::Existing(current)
                if current.file_identity == entry.desired.file_identity =>
            {
                Some(current.file_identity)
            }
            _ => None,
        }
    } else {
        match (&entry.original, &current) {
            (
                StoredFileState::Existing {
                    generation,
                    baseline_identity,
                },
                FileState::Existing(current),
            ) => (current.file_identity == *baseline_identity
                || current.file_identity == generation.file_identity)
                .then_some(current.file_identity),
            (StoredFileState::Missing, FileState::Missing) => return Ok(()),
            _ => None,
        }
    };
    let Some(expected_identity) = expected_identity else {
        return Err(Error::message(format!(
            "refusing to change permissions on a non-transaction-owned target {}; recovery state preserved",
            entry.target.display()
        )));
    };
    set_target_permissions(
        &entry.target,
        entry.parent_identity,
        expected_identity,
        permissions,
    )
}

fn remove_transaction_directory(transaction: &Path) -> Result<()> {
    let files = validated_transaction_files(transaction)?;
    for (path, permissions) in files {
        remove_recovery_file(&path, permissions)?;
    }
    sync_directory(transaction)?;
    fs::remove_dir(transaction).map_err(|source| Error::io(transaction, source))?;
    sync_target_parent(transaction)
}

fn validated_transaction_files(transaction: &Path) -> Result<Vec<(PathBuf, fs::Permissions)>> {
    let metadata =
        fs::symlink_metadata(transaction).map_err(|source| Error::io(transaction, source))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::message(format!(
            "refusing to remove a non-directory recovery transaction: {}",
            transaction.display()
        )));
    }
    let mut files = fs::read_dir(transaction)
        .map_err(|source| Error::io(transaction, source))?
        .map(|entry| {
            entry.and_then(|entry| {
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)?;
                Ok((path, metadata))
            })
        })
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|source| Error::io(transaction, source))?;
    files.sort_by_key(|(path, _)| {
        path.file_name() == Some(std::ffi::OsStr::new(ATOMIC_BATCH_JOURNAL_FILE))
    });
    for (path, metadata) in &files {
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(Error::message(format!(
                "unexpected non-file in recovery transaction; preserved at {}",
                path.display()
            )));
        }
    }
    Ok(files
        .into_iter()
        .map(|(path, metadata)| (path, metadata.permissions()))
        .collect())
}

#[cfg(windows)]
fn remove_recovery_file(path: &Path, mut permissions: fs::Permissions) -> Result<()> {
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).map_err(|source| Error::io(path, source))?;
    }
    fs::remove_file(path).map_err(|source| Error::io(path, source))
}

#[cfg(not(windows))]
fn remove_recovery_file(path: &Path, _permissions: fs::Permissions) -> Result<()> {
    fs::remove_file(path).map_err(|source| Error::io(path, source))
}

fn permissions_for_metadata(
    metadata: &fs::Metadata,
    identity: PermissionIdentity,
) -> fs::Permissions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = metadata;
        fs::Permissions::from_mode(identity.mode)
    }
    #[cfg(not(unix))]
    {
        let mut permissions = metadata.permissions();
        permissions.set_readonly(identity.readonly);
        permissions
    }
}

fn set_target_permissions(
    path: &Path,
    parent_identity: FileIdentity,
    expected_file_identity: FileIdentity,
    permission_identity: PermissionIdentity,
) -> Result<()> {
    validate_target_parent_identity(path, parent_identity)?;
    let file = fs::File::open(path).map_err(|source| Error::io(path, source))?;
    let opened_metadata = file.metadata().map_err(|source| Error::io(path, source))?;
    let opened_identity = FileIdentity::from_metadata(path, &opened_metadata)?;
    if opened_identity != expected_file_identity {
        return Err(Error::message(format!(
            "refusing to change permissions after target identity changed: {}",
            path.display()
        )));
    }
    let permissions = permissions_for_metadata(&opened_metadata, permission_identity);
    file.set_permissions(permissions)
        .map_err(|source| Error::io(path, source))?;
    file.sync_all().map_err(|source| Error::io(path, source))?;
    let final_path_metadata =
        fs::symlink_metadata(path).map_err(|source| Error::io(path, source))?;
    if FileIdentity::from_metadata(path, &final_path_metadata)? != expected_file_identity {
        return Err(Error::message(format!(
            "target identity changed while permissions were updated: {}",
            path.display()
        )));
    }
    validate_target_parent_identity(path, parent_identity)?;
    sync_target_parent(path)
}

fn ensure_same_filesystem(left: &Path, right: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let left_device = fs::metadata(left)
            .map_err(|source| Error::io(left, source))?
            .dev();
        let right_device = fs::metadata(right)
            .map_err(|source| Error::io(right, source))?
            .dev();
        if left_device != right_device {
            return Err(Error::message(format!(
                "atomic batch target and recovery transaction must be on the same filesystem: {} and {}",
                right.display(),
                left.display()
            )));
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        let left_volume = fs::metadata(left)
            .map_err(|source| Error::io(left, source))?
            .volume_serial_number();
        let right_volume = fs::metadata(right)
            .map_err(|source| Error::io(right, source))?
            .volume_serial_number();
        if left_volume.is_none() || left_volume != right_volume {
            return Err(Error::message(format!(
                "atomic batch target and recovery transaction must have the same known volume: {} and {}",
                right.display(),
                left.display()
            )));
        }
    }
    Ok(())
}

fn sync_target_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    sync_directory(parent)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| Error::io(path, source))
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<()> {
    use std::os::windows::fs::OpenOptionsExt as _;

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| Error::io(path, source))
}

#[cfg(not(any(unix, windows)))]
fn sync_directory(path: &Path) -> Result<()> {
    Err(Error::message(format!(
        "durable directory synchronization is unsupported on this platform: {}",
        path.display()
    )))
}

pub(crate) fn write_new_bytes_noclobber(
    path: &Path,
    content: &[u8],
    permissions: Option<fs::Permissions>,
) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::message(format!("{} has no parent directory", path.display())))?;
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(Error::message(format!(
                "{} already exists; refusing to overwrite a concurrent state",
                path.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io(path, error)),
    }
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|source| Error::io(parent, source))?;
    let permissions = permissions.or_else(default_file_permissions);
    temporary
        .as_file_mut()
        .write_all(content)
        .map_err(|source| Error::io(path, source))?;
    if let Some(permissions) = &permissions {
        temporary
            .as_file()
            .set_permissions(permissions.clone())
            .map_err(|source| Error::io(temporary.path(), source))?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| Error::io(path, source))?;
    let persisted = match temporary.persist_noclobber(path) {
        Ok(persisted) => persisted,
        Err(error) => {
            #[cfg(windows)]
            {
                let temporary_path = error.file.path().to_owned();
                let persist_error = error.error.to_string();
                let mut writable = match error.file.as_file().metadata() {
                    Ok(metadata) => metadata.permissions(),
                    Err(cleanup_error) => {
                        let _ = error.file.keep();
                        return Err(Error::message(format!(
                            "failed to install {} without clobbering: {}; failed to inspect staging permissions for cleanup: {cleanup_error}; staging file preserved at {}",
                            path.display(),
                            persist_error,
                            temporary_path.display()
                        )));
                    }
                };
                if writable.readonly() {
                    writable.set_readonly(false);
                    if let Err(cleanup_error) = error.file.as_file().set_permissions(writable) {
                        let _ = error.file.keep();
                        return Err(Error::message(format!(
                            "failed to install {} without clobbering: {}; failed to make readonly staging file deletable: {cleanup_error}; staging file preserved at {}",
                            path.display(),
                            persist_error,
                            temporary_path.display()
                        )));
                    }
                }
            }
            return Err(Error::io(path, error.error));
        }
    };
    drop(persisted);
    Ok(())
}

#[cfg(unix)]
fn default_file_permissions() -> Option<fs::Permissions> {
    use std::os::unix::fs::PermissionsExt as _;

    Some(fs::Permissions::from_mode(0o644))
}

#[cfg(not(unix))]
fn default_file_permissions() -> Option<fs::Permissions> {
    None
}

pub fn normalized(path: impl Into<PathBuf>) -> PathBuf {
    path.into().components().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recovery_directories(directory: &Path) -> Vec<PathBuf> {
        let recovery_root = directory.join(".boxdd-atomic-batches-test");
        let Ok(entries) = fs::read_dir(recovery_root) else {
            return Vec::new();
        };
        let mut quarantines = entries
            .filter_map(std::result::Result::ok)
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with(ATOMIC_BATCH_DRAFT_PREFIX)
                    || name.starts_with(ATOMIC_BATCH_TRANSACTION_PREFIX)
                    || name.starts_with(ATOMIC_BATCH_CLEANUP_PREFIX)
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        quarantines.sort();
        quarantines
    }

    fn quarantine_directories(directory: &Path) -> Vec<PathBuf> {
        recovery_directories(directory)
            .into_iter()
            .filter(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with(ATOMIC_BATCH_TRANSACTION_PREFIX)
                })
            })
            .collect()
    }

    fn only_recovery_transaction(directory: &Path) -> PathBuf {
        let transactions = quarantine_directories(directory);
        assert_eq!(transactions.len(), 1, "one preserved recovery transaction");
        transactions[0].clone()
    }

    #[test]
    fn atomic_write_replaces_an_existing_file() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-write-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("contract.toml");
        fs::write(&path, "old").expect("old fixture");

        write_atomic(&path, "new").expect("replace existing file");

        assert_eq!(fs::read_to_string(&path).expect("updated fixture"), "new");
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0, "temporary files must be cleaned up");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn failed_precommit_preserves_original_and_cleans_staging_file() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-failure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("contract.toml");
        fs::write(&path, "old").expect("old fixture");

        let error = write_atomic_bytes_with(&path, b"new", || {
            Err(Error::message("injected failure before atomic commit"))
        })
        .expect_err("precommit failure");

        assert!(error.to_string().contains("injected failure"));
        assert_eq!(fs::read_to_string(&path).expect("preserved fixture"), "old");
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0, "staging file must be discarded");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn failed_batch_install_rolls_back_every_committed_file() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let first = directory.path().join("first.toml");
        let second = directory.path().join("second.toml");
        fs::write(&first, b"first-old").unwrap();
        fs::write(&second, b"second-old").unwrap();
        let first_identity = read_file_generation(&first, "first original")
            .unwrap()
            .file_identity;
        let second_identity = read_file_generation(&second, "second original")
            .unwrap()
            .file_identity;
        let updates = [
            AtomicFileUpdate::checked(&first, b"first-old", b"first-new"),
            AtomicFileUpdate::checked(&second, b"second-old", b"second-new"),
        ];

        let error = write_atomic_batch_with(&updates, |index| {
            if index == 1 {
                Err(Error::message("injected second install failure"))
            } else {
                Ok(())
            }
        })
        .expect_err("second install must fail");

        assert!(
            error
                .to_string()
                .contains("injected second install failure")
        );
        assert_eq!(fs::read(&first).unwrap(), b"first-old");
        assert_eq!(fs::read(&second).unwrap(), b"second-old");
        assert_eq!(
            read_file_generation(&first, "restored first")
                .unwrap()
                .file_identity,
            first_identity
        );
        assert_eq!(
            read_file_generation(&second, "restored second")
                .unwrap()
                .file_identity,
            second_identity
        );
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[test]
    fn batch_install_preserves_a_concurrent_state_instead_of_rolling_it_back() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let first = directory.path().join("first.toml");
        let second = directory.path().join("second.toml");
        fs::write(&first, b"first-old").unwrap();
        fs::write(&second, b"second-old").unwrap();
        let updates = [
            AtomicFileUpdate::checked(&first, b"first-old", b"first-new"),
            AtomicFileUpdate::checked(&second, b"second-old", b"second-new"),
        ];

        let error = write_atomic_batch_with(&updates, |index| {
            if index == 1 {
                fs::write(&first, b"concurrent").unwrap();
                Err(Error::message("injected failure after concurrent write"))
            } else {
                Ok(())
            }
        })
        .expect_err("concurrent state must make rollback fail closed");

        assert!(error.to_string().contains("rollback conflict"));
        assert_eq!(fs::read(&first).unwrap(), b"concurrent");
        assert_eq!(fs::read(&second).unwrap(), b"second-old");
    }

    #[test]
    fn failed_batch_terminal_validation_rolls_back_all_files() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let first = directory.path().join("first.toml");
        let second = directory.path().join("second.toml");
        fs::write(&first, b"first-old").unwrap();
        fs::write(&second, b"second-old").unwrap();
        let updates = [
            AtomicFileUpdate::checked(&first, b"first-old", b"first-new"),
            AtomicFileUpdate::checked(&second, b"second-old", b"second-new"),
        ];

        let error = write_atomic_batch_with_checks(
            &updates,
            |_| Ok(()),
            || Err(Error::message("injected terminal validation failure")),
        )
        .expect_err("terminal validation must fail");

        assert!(error.to_string().contains("terminal validation failure"));
        assert_eq!(fs::read(&first).unwrap(), b"first-old");
        assert_eq!(fs::read(&second).unwrap(), b"second-old");
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[test]
    fn missing_batch_target_is_created_without_a_quarantine() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        let updates = [AtomicFileUpdate::missing(&path, b"generated")];

        write_atomic_batch_with_checks(&updates, |_| Ok(()), || Ok(()))
            .expect("missing target install");

        assert_eq!(fs::read(&path).unwrap(), b"generated");
        assert!(recovery_directories(directory.path()).is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
                0o644
            );
        }
    }

    #[test]
    fn committed_existing_generation_is_cleaned() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"original").unwrap();
        let updates = [AtomicFileUpdate::checked(&path, b"original", b"generated")];

        write_atomic_batch_with_checks(&updates, |_| Ok(()), || Ok(()))
            .expect("existing target install");

        assert_eq!(fs::read(&path).unwrap(), b"generated");
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[test]
    fn terminal_failure_restores_existing_and_removes_missing_targets() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let existing = directory.path().join("existing.toml");
        let missing = directory.path().join("missing.toml");
        fs::write(&existing, b"original").unwrap();
        let updates = [
            AtomicFileUpdate::checked(&existing, b"original", b"generated-existing"),
            AtomicFileUpdate::missing(&missing, b"generated-missing"),
        ];

        let error = write_atomic_batch_with_checks(
            &updates,
            |_| Ok(()),
            || Err(Error::message("injected terminal validation failure")),
        )
        .expect_err("terminal validation must roll back both states");

        assert!(error.to_string().contains("terminal validation failure"));
        assert_eq!(fs::read(&existing).unwrap(), b"original");
        assert_eq!(
            fs::symlink_metadata(&missing).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[test]
    fn missing_install_never_overwrites_a_concurrent_creation() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        let updates = [AtomicFileUpdate::missing(&path, b"transaction")];

        let error = write_atomic_batch_with_checks(
            &updates,
            |hook| {
                if hook == AtomicBatchHook::BeforeMissingInstall(0) {
                    fs::write(&path, b"concurrent").map_err(|source| Error::io(&path, source))?;
                }
                Ok(())
            },
            || Ok(()),
        )
        .expect_err("concurrent creation must make no-clobber installation fail");

        assert!(error.to_string().contains("refusing to overwrite"));
        assert_eq!(fs::read(&path).unwrap(), b"concurrent");
        let transaction = only_recovery_transaction(directory.path());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref()),
            "{error}"
        );
        let recovery_root = directory.path().join(".boxdd-atomic-batches-test");
        let recovery_error = recover_atomic_batches_at(&recovery_root, false)
            .expect_err("conflicting target must block automatic recovery");
        assert_eq!(fs::read(&path).unwrap(), b"concurrent");
        assert!(transaction.exists());
        assert!(
            recovery_error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn missing_install_preserves_a_concurrent_copy_of_the_desired_bytes() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        let updates = [AtomicFileUpdate::missing(&path, b"transaction")];

        let error = write_atomic_batch_with_checks(
            &updates,
            |hook| {
                if hook == AtomicBatchHook::BeforeMissingInstall(0) {
                    fs::write(&path, b"transaction").map_err(|source| Error::io(&path, source))?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt as _;

                        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
                            .map_err(|source| Error::io(&path, source))?;
                    }
                }
                Ok(())
            },
            || Ok(()),
        )
        .expect_err("same-content concurrent creation must remain distinguishable");

        assert_eq!(fs::read(&path).unwrap(), b"transaction");
        let transaction = only_recovery_transaction(directory.path());
        assert!(transaction.exists());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn missing_rollback_preserves_a_concurrent_recreation() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        let updates = [AtomicFileUpdate::missing(&path, b"transaction")];

        let error = write_atomic_batch_with_checks(
            &updates,
            |hook| {
                if hook == AtomicBatchHook::AfterRollbackQuarantine(0) {
                    fs::write(&path, b"concurrent").map_err(|source| Error::io(&path, source))?;
                }
                Ok(())
            },
            || Err(Error::message("force rollback")),
        )
        .expect_err("concurrent recreation must make rollback fail closed");

        assert!(error.to_string().contains("rollback conflict"));
        assert_eq!(fs::read(&path).unwrap(), b"concurrent");
        let transaction = only_recovery_transaction(directory.path());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn draft_recovery_directory_is_removed_without_touching_targets() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let recovery_root = directory.path().join("recovery");
        ensure_private_recovery_directory(&recovery_root).expect("recovery root");
        let draft = recovery_root.join(format!("{ATOMIC_BATCH_DRAFT_PREFIX}orphan"));
        fs::create_dir(&draft).expect("orphan draft");
        fs::write(draft.join("prepared.bin"), b"prepared").expect("prepared generation");

        recover_atomic_batches_at(&recovery_root, false).expect("remove unpublished draft");

        assert!(!draft.exists());
    }

    #[test]
    fn published_recovery_directory_without_journal_is_preserved_and_reported() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let recovery_root = directory.path().join("recovery");
        ensure_private_recovery_directory(&recovery_root).expect("recovery root");
        let transaction = recovery_root.join(format!("{ATOMIC_BATCH_TRANSACTION_PREFIX}orphan"));
        fs::create_dir(&transaction).expect("orphan published transaction");
        fs::write(transaction.join("prepared.bin"), b"prepared").expect("prepared generation");

        let error = recover_atomic_batches_at(&recovery_root, false)
            .expect_err("published transaction without journal must fail closed");

        assert!(transaction.exists());
        assert!(error.to_string().contains("has no journal"));
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn journal_free_cleanup_directory_is_removed() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let recovery_root = directory.path().join("recovery");
        ensure_private_recovery_directory(&recovery_root).expect("recovery root");
        let cleanup = recovery_root.join(format!("{ATOMIC_BATCH_CLEANUP_PREFIX}orphan"));
        fs::create_dir(&cleanup).expect("orphan cleanup directory");
        fs::write(cleanup.join("prepared.bin"), b"prepared").expect("cleanup residue");

        recover_atomic_batches_at(&recovery_root, false).expect("remove cleanup residue");

        assert!(!cleanup.exists());
    }

    #[test]
    fn invalid_recovery_journal_is_preserved_and_reported() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let recovery_root = directory.path().join("recovery");
        ensure_private_recovery_directory(&recovery_root).expect("recovery root");
        let transaction = recovery_root.join(format!("{ATOMIC_BATCH_TRANSACTION_PREFIX}invalid"));
        fs::create_dir(&transaction).expect("invalid transaction");
        fs::write(transaction.join(ATOMIC_BATCH_JOURNAL_FILE), b"not = [toml")
            .expect("invalid journal");

        let error = recover_atomic_batches_at(&recovery_root, false)
            .expect_err("invalid journal must fail closed");

        assert!(transaction.exists());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess crash helper"]
    fn atomic_batch_sigkill_child() {
        let Some(workspace_root) = std::env::var_os("BOXDD_ATOMIC_BATCH_SIGKILL_WORKSPACE") else {
            return;
        };
        let Some(recovery_root) = std::env::var_os("BOXDD_ATOMIC_BATCH_SIGKILL_RECOVERY") else {
            return;
        };
        let workspace_root = PathBuf::from(workspace_root);
        let recovery_root = PathBuf::from(recovery_root);
        let first = workspace_root.join("first.toml");
        let second = workspace_root.join("second.toml");
        let updates = [
            AtomicFileUpdate::checked(&first, b"first-old", b"first-new"),
            AtomicFileUpdate::checked(&second, b"second-old", b"second-new"),
        ];

        let result = write_atomic_batch_in(
            &workspace_root,
            &recovery_root,
            &updates,
            |hook| {
                if hook == AtomicBatchHook::AfterInstallQuarantine(1) {
                    let status = std::process::Command::new("kill")
                        .arg("-KILL")
                        .arg(std::process::id().to_string())
                        .status()
                        .map_err(|source| {
                            Error::message(format!("could not invoke kill: {source}"))
                        })?;
                    return Err(Error::message(format!(
                        "kill unexpectedly returned with {status}"
                    )));
                }
                Ok(())
            },
            || Ok(()),
        );
        panic!("SIGKILL helper unexpectedly returned: {result:?}");
    }

    #[cfg(unix)]
    #[test]
    fn update_lock_recovers_a_half_installed_batch_after_sigkill() {
        use std::os::unix::process::ExitStatusExt as _;

        use crate::{commands::upstream_sync::UpdateLock, qualified_git::qualified_git_command};

        let directory = tempfile::tempdir().expect("SIGKILL recovery fixture");
        let workspace_root = directory.path().canonicalize().expect("canonical fixture");
        let status = qualified_git_command()
            .expect("qualified Git")
            .arg("init")
            .arg(&workspace_root)
            .status()
            .expect("initialize fixture repository");
        assert!(status.success(), "git init failed with {status}");
        let recovery_root = atomic_batch_recovery_root(&workspace_root).expect("recovery root");
        let first = workspace_root.join("first.toml");
        let second = workspace_root.join("second.toml");
        fs::write(&first, b"first-old").expect("first original");
        fs::write(&second, b"second-old").expect("second original");

        let child_status =
            std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args([
                    "--ignored",
                    "--exact",
                    "config::tests::atomic_batch_sigkill_child",
                ])
                .env("BOXDD_ATOMIC_BATCH_SIGKILL_WORKSPACE", &workspace_root)
                .env("BOXDD_ATOMIC_BATCH_SIGKILL_RECOVERY", &recovery_root)
                .status()
                .expect("run SIGKILL child");
        assert_eq!(
            child_status.signal(),
            Some(9),
            "child must die from SIGKILL"
        );
        assert!(
            fs::read_dir(&recovery_root)
                .expect("recovery root entries")
                .filter_map(std::result::Result::ok)
                .next()
                .is_some(),
            "crashed batch must leave a recovery transaction"
        );

        let _lock = UpdateLock::acquire(&workspace_root).expect("lock must recover transaction");

        assert_eq!(fs::read(&first).unwrap(), b"first-old");
        assert_eq!(fs::read(&second).unwrap(), b"second-old");
        let remaining = fs::read_dir(&recovery_root)
            .expect("recovery root entries")
            .filter_map(std::result::Result::ok)
            .count();
        assert_eq!(
            remaining, 0,
            "successful recovery must remove the transaction"
        );
    }

    #[test]
    fn batch_install_never_overwrites_a_target_recreated_after_quarantine() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"original").unwrap();
        let updates = [AtomicFileUpdate::checked(
            &path,
            b"original",
            b"transaction",
        )];

        let error = write_atomic_batch_with_checks(
            &updates,
            |hook| {
                if hook == AtomicBatchHook::AfterInstallQuarantine(0) {
                    fs::write(&path, b"concurrent").map_err(|source| Error::io(&path, source))?;
                }
                Ok(())
            },
            || Ok(()),
        )
        .expect_err("recreated target must make installation fail closed");

        assert_eq!(fs::read(&path).unwrap(), b"concurrent");
        let transaction = only_recovery_transaction(directory.path());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn batch_install_preserves_a_recreated_copy_of_the_desired_bytes() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"original").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        }
        let updates = [AtomicFileUpdate::checked(
            &path,
            b"original",
            b"transaction",
        )];

        let error = write_atomic_batch_with_checks(
            &updates,
            |hook| {
                if hook == AtomicBatchHook::AfterInstallQuarantine(0) {
                    fs::write(&path, b"transaction").map_err(|source| Error::io(&path, source))?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt as _;

                        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
                            .map_err(|source| Error::io(&path, source))?;
                    }
                }
                Ok(())
            },
            || Ok(()),
        )
        .expect_err("same-content concurrent recreation must remain distinguishable");

        assert_eq!(fs::read(&path).unwrap(), b"transaction");
        let transaction = only_recovery_transaction(directory.path());
        assert!(transaction.exists());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn batch_rollback_preserves_every_generation_when_target_is_recreated() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"original").unwrap();
        let updates = [AtomicFileUpdate::checked(
            &path,
            b"original",
            b"transaction",
        )];

        let error = write_atomic_batch_with_checks(
            &updates,
            |hook| {
                if hook == AtomicBatchHook::AfterRollbackQuarantine(0) {
                    fs::write(&path, b"concurrent").map_err(|source| Error::io(&path, source))?;
                }
                Ok(())
            },
            || Err(Error::message("force rollback")),
        )
        .expect_err("recreated rollback target must fail closed");

        assert_eq!(fs::read(&path).unwrap(), b"concurrent");
        let transaction = only_recovery_transaction(directory.path());
        assert!(
            error
                .to_string()
                .contains(transaction.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn unchanged_batch_is_a_noop_without_quarantine() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"unchanged").unwrap();
        let updates = [AtomicFileUpdate::checked(&path, b"unchanged", b"unchanged")];
        let mut hook_calls = 0;

        write_atomic_batch_with_checks(
            &updates,
            |_| {
                hook_calls += 1;
                Ok(())
            },
            || Ok(()),
        )
        .expect("no-op batch");

        assert_eq!(hook_calls, 0);
        assert_eq!(fs::read(&path).unwrap(), b"unchanged");
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn batch_install_and_rollback_preserve_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).unwrap();
        let install = [AtomicFileUpdate::checked(&path, b"original", b"installed")];

        write_atomic_batch_with_checks(&install, |_| Ok(()), || Ok(())).expect("batch install");

        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o754
        );
        let rollback = [AtomicFileUpdate::checked(
            &path,
            b"installed",
            b"rolled-back",
        )];
        write_atomic_batch_with_checks(
            &rollback,
            |_| Ok(()),
            || Err(Error::message("force rollback")),
        )
        .expect_err("terminal failure must roll back");

        assert_eq!(fs::read(&path).unwrap(), b"installed");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o754
        );
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn batch_install_and_rollback_preserve_readonly_and_quarantines() {
        let directory = tempfile::tempdir().expect("batch fixture");
        let path = directory.path().join("contract.toml");
        fs::write(&path, b"original").unwrap();
        let mut readonly = fs::metadata(&path).unwrap().permissions();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let install = [AtomicFileUpdate::checked(&path, b"original", b"installed")];

        write_atomic_batch_with_checks(&install, |_| Ok(()), || Ok(()))
            .expect("readonly batch install");

        assert_eq!(fs::read(&path).unwrap(), b"installed");
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
        assert!(recovery_directories(directory.path()).is_empty());

        let rollback = [AtomicFileUpdate::checked(
            &path,
            b"installed",
            b"rollback-candidate",
        )];
        let error = write_atomic_batch_with_checks(
            &rollback,
            |_| Ok(()),
            || Err(Error::message("force rollback")),
        )
        .expect_err("terminal failure must roll back readonly target");

        assert!(error.to_string().contains("force rollback"));
        assert_eq!(fs::read(&path).unwrap(), b"installed");
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
        assert!(recovery_directories(directory.path()).is_empty());

        let mut writable = fs::metadata(&path).unwrap().permissions();
        writable.set_readonly(false);
        fs::set_permissions(&path, writable).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn batch_rejects_a_symlink_target_without_mutation() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("batch fixture");
        let real = directory.path().join("real.toml");
        let path = directory.path().join("contract.toml");
        fs::write(&real, b"original").unwrap();
        symlink(&real, &path).unwrap();
        let updates = [AtomicFileUpdate::checked(
            &path,
            b"original",
            b"transaction",
        )];

        let error = write_atomic_batch_with_checks(&updates, |_| Ok(()), || Ok(()))
            .expect_err("symlink target must be rejected");

        assert!(error.to_string().contains("regular non-symlink file"));
        assert!(
            fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&real).unwrap(), b"original");
        assert!(recovery_directories(directory.path()).is_empty());
    }

    #[test]
    fn atomic_write_rejects_non_regular_targets_without_mutation() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-nonregular-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let target = directory.join("target");
        fs::create_dir_all(&target).expect("directory target");

        let error = write_atomic(&target, "new").expect_err("directory target must fail");

        assert!(error.to_string().contains("not a regular file"));
        assert!(target.is_dir());
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn failed_atomic_replacement_cleans_staging_file() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-replace-failure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("contract.toml");
        fs::write(&path, "old").expect("old fixture");

        let error = write_atomic_bytes_with(&path, b"new", || {
            fs::remove_file(&path).map_err(|source| Error::io(&path, source))?;
            fs::create_dir(&path).map_err(|source| Error::io(&path, source))?;
            Ok(())
        })
        .expect_err("replacing a directory must fail");

        assert!(error.to_string().contains("contract.toml"));
        assert!(path.is_dir());
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0, "failed persist must discard its staging file");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replacement_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = std::env::temp_dir().join(format!(
            "boxdd-atomic-permissions-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("artifact");
        fs::write(&path, "old").expect("old fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).expect("fixture permissions");

        write_atomic(&path, "new").expect("atomic replacement");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o754);
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn noclobber_write_preserves_a_concurrent_target_and_cleans_staging() {
        let directory = std::env::temp_dir().join(format!(
            "boxdd-noclobber-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let path = directory.join("artifact");
        fs::write(&path, "concurrent").expect("concurrent target");

        let error = write_new_bytes_noclobber(&path, b"transaction", None)
            .expect_err("noclobber write must reject an existing path");

        assert!(error.to_string().contains("refusing to overwrite"));
        assert_eq!(
            fs::read_to_string(&path).expect("preserved target"),
            "concurrent"
        );
        let leftovers = fs::read_dir(&directory)
            .expect("fixture entries")
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path() != path)
            .count();
        assert_eq!(leftovers, 0);
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }
}
