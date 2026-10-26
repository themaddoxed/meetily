//! Bundle-identity migration primitives.
//!
//! This module deliberately stops at a verified, no-clobber staging copy. The
//! caller must provide the final source/target identity and must not publish a
//! staged tree until the SQLite, WebView export, and runtime gates have passed.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[cfg(unix)]
use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};

pub const MIGRATION_FORMAT_VERSION: u32 = 1;
pub const MIGRATION_NAMESPACE_DIR: &str = "migration";
pub const MIGRATION_JOURNAL_FILE: &str = "journal.json";
const FIXED_MIGRATION_OVERHEAD_BYTES: u64 = 128 * 1024 * 1024;
const MIN_MIGRATION_RESERVE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationNamespace {
    LegacyAppData,
    LegacyNamedConfig,
    LegacyNamedData,
    WebExport,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationPhase {
    Preparing,
    Prepared,
    Publishing,
    Verified,
    Completed,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFile {
    pub namespace: MigrationNamespace,
    pub source_relative_path: PathBuf,
    pub target_relative_path: PathBuf,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationJournal {
    pub format_version: u32,
    pub operation_id: Uuid,
    pub source_identity: String,
    pub target_identity: String,
    pub phase: MigrationPhase,
    pub files: Vec<MigrationFile>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationRoots {
    pub legacy_app_data: PathBuf,
    pub legacy_named_config: PathBuf,
    pub legacy_named_data: PathBuf,
    pub web_export: PathBuf,
    pub target_app_data: PathBuf,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationBudget {
    pub sqlite_bytes: u64,
    pub file_bytes: u64,
    pub web_export_bytes: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationBudgetEstimate {
    pub temporary_bytes: u64,
    pub reserve_bytes: u64,
    pub required_bytes: u64,
}

/// Stable filesystem locations for one bundle-identity migration operation.
///
/// The journal directory is named by the UUID and lives below the target
/// profile's `migration/` directory. Staging is a sibling tree on the same
/// filesystem, because the target profile itself must never overlap its
/// temporary source. Callers must still hold the process-wide lock while
/// creating or changing files at these paths.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationOperationPaths {
    pub operation_root: PathBuf,
    pub journal: PathBuf,
    pub staging: PathBuf,
    pub lock: PathBuf,
}

/// Holds the migration lock for the lifetime of the operation.
///
/// The lock file is intentionally kept after the process exits. The operating
/// system releases the advisory lock on a crash, while the next operation can
/// reuse the same regular file without deleting a path another process may
/// have replaced.
#[derive(Debug)]
pub struct MigrationLock {
    file: File,
}

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("invalid migration manifest: {0}")]
    InvalidManifest(String),
    #[error("migration source file is not a regular owned file: {0}")]
    UnsafeSource(PathBuf),
    #[error("migration source path is outside its namespace: {0}")]
    SourceEscapesRoot(PathBuf),
    #[error("migration destination path is not a regular owned path: {0}")]
    UnsafeDestination(PathBuf),
    #[error("migration destination already exists: {0}")]
    DestinationExists(PathBuf),
    #[error("migration file size mismatch for {path}: expected {expected}, got {actual}")]
    SizeMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("migration file hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("migration source and target roots overlap")]
    OverlappingRoots,
    #[error("SQLite source changed while its snapshot was being created")]
    SourceChanged,
    #[error("SQLite snapshot failed: {0}")]
    Sqlite(String),
    #[error("migration staging directory must be new and empty: {0}")]
    StagingNotEmpty(PathBuf),
    #[error("migration journal must be a regular file: {0}")]
    UnsafeJournal(PathBuf),
    #[error("migration lock must be a regular file: {0}")]
    UnsafeLock(PathBuf),
    #[error("migration lock is already held: {0}")]
    LockHeld(PathBuf),
    #[error("migration disk budget overflows a 64-bit byte count")]
    BudgetOverflow,
    #[error("migration requires {required} bytes but only {available} bytes are available")]
    InsufficientDiskSpace { required: u64, available: u64 },
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl MigrationRoots {
    fn source_root(&self, namespace: MigrationNamespace) -> &Path {
        match namespace {
            MigrationNamespace::LegacyAppData => &self.legacy_app_data,
            MigrationNamespace::LegacyNamedConfig => &self.legacy_named_config,
            MigrationNamespace::LegacyNamedData => &self.legacy_named_data,
            MigrationNamespace::WebExport => &self.web_export,
        }
    }

    fn source_roots(&self) -> [(&'static str, &Path); 4] {
        [
            ("legacy_app_data", &self.legacy_app_data),
            ("legacy_named_config", &self.legacy_named_config),
            ("legacy_named_data", &self.legacy_named_data),
            ("web_export", &self.web_export),
        ]
    }

    fn validate(
        &self,
        staging_root: &Path,
        used_namespaces: &BTreeSet<MigrationNamespace>,
    ) -> Result<(), MigrationError> {
        let target = canonical_for_relationship(&self.target_app_data)?;
        let staging = canonical_for_relationship(staging_root)?;
        let mut canonical_sources = Vec::new();

        for (name, source) in self.source_roots() {
            let namespace = match name {
                "legacy_app_data" => MigrationNamespace::LegacyAppData,
                "legacy_named_config" => MigrationNamespace::LegacyNamedConfig,
                "legacy_named_data" => MigrationNamespace::LegacyNamedData,
                "web_export" => MigrationNamespace::WebExport,
                _ => unreachable!("all migration source roots have a known namespace"),
            };
            let canonical = match canonical_existing_directory(source) {
                Ok(canonical) => canonical,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if used_namespaces.contains(&namespace) {
                        return Err(MigrationError::InvalidManifest(format!("{name}: {error}")));
                    }
                    continue;
                }
                Err(error) => {
                    return Err(MigrationError::InvalidManifest(format!("{name}: {error}")));
                }
            };
            if same_or_within(&canonical, &target)
                || same_or_within(&target, &canonical)
                || same_or_within(&canonical, &staging)
                || same_or_within(&staging, &canonical)
            {
                return Err(MigrationError::OverlappingRoots);
            }
            if canonical_sources.iter().any(|other: &PathBuf| {
                same_or_within(&canonical, other) || same_or_within(other, &canonical)
            }) {
                return Err(MigrationError::OverlappingRoots);
            }
            canonical_sources.push(canonical);
        }

        if same_or_within(&target, &staging) || same_or_within(&staging, &target) {
            return Err(MigrationError::OverlappingRoots);
        }
        Ok(())
    }
}

/// Return the journal, staging, and lock paths for an operation without
/// creating or mutating anything.
pub fn migration_operation_paths(
    target_app_data: &Path,
    operation_id: Uuid,
) -> Result<MigrationOperationPaths, MigrationError> {
    if operation_id.is_nil() {
        return Err(MigrationError::InvalidManifest(
            "operation id must not be nil".to_string(),
        ));
    }
    let migration_root = target_app_data.join(MIGRATION_NAMESPACE_DIR);
    let operation_root = migration_root.join(operation_id.to_string());
    let target_parent = target_app_data.parent().ok_or_else(|| {
        MigrationError::InvalidManifest("target app-data path has no parent".to_string())
    })?;
    let target_name = target_app_data.file_name().ok_or_else(|| {
        MigrationError::InvalidManifest("target app-data path has no file name".to_string())
    })?;
    let staging_root = target_parent.join(format!(
        ".{}.migration-staging",
        target_name.to_string_lossy()
    ));
    Ok(MigrationOperationPaths {
        journal: operation_root.join(MIGRATION_JOURNAL_FILE),
        staging: staging_root.join(operation_id.to_string()),
        operation_root,
        lock: migration_root.join("lock"),
    })
}

/// Validate the manifest before touching the destination.
pub fn validate_manifest(journal: &MigrationJournal) -> Result<(), MigrationError> {
    if journal.format_version != MIGRATION_FORMAT_VERSION {
        return Err(MigrationError::InvalidManifest(format!(
            "unsupported format version {}",
            journal.format_version
        )));
    }
    if journal.operation_id.is_nil() {
        return Err(MigrationError::InvalidManifest(
            "operation id must not be nil".to_string(),
        ));
    }
    if journal.source_identity.trim().is_empty()
        || journal.target_identity.trim().is_empty()
        || journal.source_identity == journal.target_identity
    {
        return Err(MigrationError::InvalidManifest(
            "source and target identities must be present and different".to_string(),
        ));
    }

    let mut target_paths = Vec::with_capacity(journal.files.len());
    for file in &journal.files {
        validate_relative_path(&file.source_relative_path)?;
        validate_relative_path(&file.target_relative_path)?;
        if file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(MigrationError::InvalidManifest(format!(
                "invalid SHA-256 for {}",
                file.source_relative_path.display()
            )));
        }
        if target_paths
            .iter()
            .any(|path: &PathBuf| path == &file.target_relative_path)
        {
            return Err(MigrationError::InvalidManifest(format!(
                "duplicate target path {}",
                file.target_relative_path.display()
            )));
        }
        target_paths.push(file.target_relative_path.clone());
    }
    Ok(())
}

/// Calculate the conservative temporary-space requirement from the migration
/// design. The result is pure arithmetic; the caller must compare
/// `required_bytes` with the current free space immediately before each phase.
pub fn estimate_migration_budget(
    budget: MigrationBudget,
) -> Result<MigrationBudgetEstimate, MigrationError> {
    let sqlite_copy = budget
        .sqlite_bytes
        .checked_mul(7)
        .ok_or(MigrationError::BudgetOverflow)?;
    let web_export_copy = budget
        .web_export_bytes
        .checked_mul(2)
        .ok_or(MigrationError::BudgetOverflow)?;
    let temporary_bytes = sqlite_copy
        .checked_add(budget.file_bytes)
        .and_then(|value| value.checked_add(web_export_copy))
        .and_then(|value| value.checked_add(FIXED_MIGRATION_OVERHEAD_BYTES))
        .ok_or(MigrationError::BudgetOverflow)?;
    let reserve_fraction = temporary_bytes
        .checked_add(9)
        .ok_or(MigrationError::BudgetOverflow)?
        / 10;
    let reserve_bytes = reserve_fraction.max(MIN_MIGRATION_RESERVE_BYTES);
    let required_bytes = temporary_bytes
        .checked_add(reserve_bytes)
        .ok_or(MigrationError::BudgetOverflow)?;
    Ok(MigrationBudgetEstimate {
        temporary_bytes,
        reserve_bytes,
        required_bytes,
    })
}

/// Check a freshly measured free-space value against the conservative budget.
///
/// The caller must measure the filesystem containing the staging root just
/// before each migration phase and pass that value here. Keeping the syscall at
/// the caller boundary avoids using a stale measurement inside this pure core.
pub fn ensure_migration_free_space(
    available_bytes: u64,
    estimate: MigrationBudgetEstimate,
) -> Result<(), MigrationError> {
    if available_bytes < estimate.required_bytes {
        return Err(MigrationError::InsufficientDiskSpace {
            required: estimate.required_bytes,
            available: available_bytes,
        });
    }
    Ok(())
}

/// Estimate the budget and reject it when the supplied free-space snapshot is
/// too small.
pub fn check_migration_budget(
    available_bytes: u64,
    budget: MigrationBudget,
) -> Result<MigrationBudgetEstimate, MigrationError> {
    let estimate = estimate_migration_budget(budget)?;
    ensure_migration_free_space(available_bytes, estimate)?;
    Ok(estimate)
}

/// Measure free bytes on the filesystem that contains `path`.
///
/// A path may be a file, an existing directory, or a destination that has not
/// been created yet. The probe resolves the nearest existing ancestor and uses
/// the filesystem's available-for-unprivileged-users byte count, so the
/// caller can check the budget immediately before each migration phase.
pub fn available_migration_free_space(path: &Path) -> Result<u64, MigrationError> {
    let mut existing = if path.is_dir() {
        path
    } else {
        path.parent().ok_or_else(|| {
            MigrationError::InvalidManifest("free-space path has no parent".to_string())
        })?
    };
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            MigrationError::InvalidManifest("free-space path has no existing ancestor".to_string())
        })?;
    }
    let canonical = fs::canonicalize(existing)?;

    #[cfg(unix)]
    {
        let bytes = canonical.as_os_str().as_bytes();
        let c_path = CString::new(bytes).map_err(|_| {
            MigrationError::InvalidManifest("free-space path contains a NUL byte".to_string())
        })?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        let result = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };
        if result != 0 {
            return Err(MigrationError::Io(io::Error::last_os_error()));
        }
        let stats = unsafe { stats.assume_init() };
        let available = u128::from(stats.f_bavail)
            .checked_mul(u128::from(stats.f_frsize))
            .ok_or(MigrationError::BudgetOverflow)?;
        u64::try_from(available).map_err(|_| MigrationError::BudgetOverflow)
    }

    #[cfg(not(unix))]
    {
        let _ = canonical;
        Err(MigrationError::InvalidManifest(
            "free-space measurement is unsupported on this platform".to_string(),
        ))
    }
}

/// Measure the current filesystem and enforce the conservative migration
/// budget in one caller-facing operation.
pub fn check_migration_budget_at_path(
    path: &Path,
    budget: MigrationBudget,
) -> Result<MigrationBudgetEstimate, MigrationError> {
    let available = available_migration_free_space(path)?;
    check_migration_budget(available, budget)
}

/// Acquire the process-wide migration lock without waiting.
///
/// The lock uses a stable regular file and an advisory OS lock. A crashed
/// process releases the OS lock automatically, so stale lock files do not
/// block recovery. The operation ID is written only after the lock is held and
/// is useful to diagnostics and recovery tooling.
pub fn acquire_migration_lock(
    path: &Path,
    operation_id: Uuid,
) -> Result<MigrationLock, MigrationError> {
    let parent = path.parent().ok_or_else(|| {
        MigrationError::InvalidManifest("migration lock has no parent directory".to_string())
    })?;
    fs::create_dir_all(parent)?;

    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(MigrationError::UnsafeLock(path.to_path_buf()));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(MigrationError::Io(error)),
    }

    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(MigrationError::UnsafeLock(path.to_path_buf()));
    }

    try_lock_file(&file).map_err(|error| {
        if error.kind() == io::ErrorKind::WouldBlock {
            MigrationError::LockHeld(path.to_path_buf())
        } else {
            MigrationError::Io(error)
        }
    })?;

    file.set_len(0)?;
    file.write_all(operation_id.to_string().as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    File::open(parent)?.sync_all()?;
    Ok(MigrationLock { file })
}

#[cfg(unix)]
fn try_lock_file(file: &File) -> io::Result<()> {
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    // `flock` is available on macOS and the Unix targets used by the test
    // harness. The file descriptor stays open in MigrationLock for the whole
    // operation, so the advisory lock remains held.
    let result = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

#[cfg(not(unix))]
fn try_lock_file(_file: &File) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "migration locking requires a Unix filesystem",
    ))
}

impl Drop for MigrationLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            const LOCK_UN: i32 = 8;
            // The descriptor is closed immediately after Drop, but explicitly
            // unlocking keeps the lifetime obvious and makes tests deterministic.
            let _ = unsafe { flock(self.file.as_raw_fd(), LOCK_UN) };
        }
    }
}

/// Persist a journal without replacing an existing operation.
///
/// The journal is written and synced under a unique temporary name, then
/// published with `hard_link`, which fails if another operation already owns
/// the destination. Repeating the exact same journal is idempotent; a
/// different journal receives a no-clobber conflict.
pub fn persist_journal(path: &Path, journal: &MigrationJournal) -> Result<(), MigrationError> {
    validate_manifest(journal)?;
    let parent = path.parent().ok_or_else(|| {
        MigrationError::InvalidManifest("migration journal has no parent directory".to_string())
    })?;
    fs::create_dir_all(parent)?;

    if path.exists() {
        if !is_regular_file(path)? {
            return Err(MigrationError::UnsafeJournal(path.to_path_buf()));
        }
        let existing = load_journal(path)?;
        return if existing == *journal {
            Ok(())
        } else {
            Err(MigrationError::DestinationExists(path.to_path_buf()))
        };
    }

    let file_name = path.file_name().ok_or_else(|| {
        MigrationError::InvalidManifest("migration journal has no file name".to_string())
    })?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        journal.operation_id
    ));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(journal).map_err(|error| {
            MigrationError::InvalidManifest(format!("cannot serialize migration journal: {error}"))
        })?;
        let mut staged = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        staged.write_all(&bytes)?;
        staged.write_all(b"\n")?;
        staged.sync_all()?;
        drop(staged);

        match fs::hard_link(&temporary, path) {
            Ok(()) => {
                File::open(parent)?.sync_all()?;
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if !is_regular_file(path)? {
                    return Err(MigrationError::UnsafeJournal(path.to_path_buf()));
                }
                let existing = load_journal(path)?;
                if existing == *journal {
                    Ok(())
                } else {
                    Err(MigrationError::DestinationExists(path.to_path_buf()))
                }
            }
            Err(error) => Err(MigrationError::Io(error)),
        }
    })();
    let _ = fs::remove_file(&temporary);
    if result.is_ok() {
        File::open(parent)?.sync_all()?;
    }
    result
}

fn migration_phase_rank(phase: MigrationPhase) -> u8 {
    match phase {
        MigrationPhase::Preparing => 0,
        MigrationPhase::Prepared => 1,
        MigrationPhase::Publishing => 2,
        MigrationPhase::Verified => 3,
        MigrationPhase::Completed => 4,
    }
}

/// Advance one operation journal through its durable phases.
///
/// Initial journal creation remains no-clobber via persist_journal. Once an
/// operation owns the journal, this helper permits only an idempotent write or
/// the next phase in sequence, and replaces the journal atomically after the
/// new bytes have been synced. The caller must hold the migration lock for the
/// whole phase transition.
pub fn update_journal_phase(path: &Path, phase: MigrationPhase) -> Result<(), MigrationError> {
    let mut journal = load_journal(path)?;
    let current_rank = migration_phase_rank(journal.phase);
    let next_rank = migration_phase_rank(phase);
    if next_rank == current_rank {
        return Ok(());
    }
    if next_rank != current_rank + 1 {
        return Err(MigrationError::InvalidManifest(format!(
            "migration phase cannot move from {:?} to {:?}",
            journal.phase, phase
        )));
    }
    journal.phase = phase;

    let parent = path.parent().ok_or_else(|| {
        MigrationError::InvalidManifest("migration journal has no parent directory".to_string())
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        MigrationError::InvalidManifest("migration journal has no file name".to_string())
    })?;
    let temporary = parent.join(format!(
        ".{}.{}.{}.phase.tmp",
        file_name.to_string_lossy(),
        journal.operation_id,
        Uuid::new_v4()
    ));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(&journal).map_err(|error| {
            MigrationError::InvalidManifest(format!("cannot serialize migration journal: {error}"))
        })?;
        let mut staged = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        staged.write_all(&bytes)?;
        staged.write_all(b"\n")?;
        staged.sync_all()?;
        drop(staged);
        fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Load and validate a previously persisted journal.
pub fn load_journal(path: &Path) -> Result<MigrationJournal, MigrationError> {
    if !is_regular_file(path)? {
        return Err(MigrationError::UnsafeJournal(path.to_path_buf()));
    }
    let bytes = fs::read(path)?;
    let journal = serde_json::from_slice::<MigrationJournal>(&bytes).map_err(|error| {
        MigrationError::InvalidManifest(format!("cannot parse migration journal: {error}"))
    })?;
    validate_manifest(&journal)?;
    Ok(journal)
}

/// Refuse to open the normal database while a bundle-identity migration is
/// still in progress.
///
/// Each operation owns a directory named by its UUID below `migration/` and
/// stores its journal as `journal.json`. Completed journals are retained for
/// auditability; any other phase blocks startup so a partially published tree
/// cannot be mistaken for a normal profile. This check is read-only.
pub fn ensure_no_incomplete_migrations(target_app_data: &Path) -> Result<(), MigrationError> {
    let namespace = target_app_data.join(MIGRATION_NAMESPACE_DIR);
    let metadata = match fs::symlink_metadata(&namespace) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(MigrationError::Io(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MigrationError::UnsafeJournal(namespace));
    }

    let mut operation_dirs = fs::read_dir(&namespace)
        .map_err(MigrationError::Io)?
        .collect::<Result<Vec<_>, io::Error>>()?;
    operation_dirs.sort_by_key(|entry| entry.file_name());

    for operation in operation_dirs {
        let operation_path = operation.path();
        let operation_metadata = fs::symlink_metadata(&operation_path)?;
        if operation_metadata.file_type().is_symlink() {
            return Err(MigrationError::UnsafeJournal(operation_path));
        }
        if !operation_metadata.is_dir() {
            continue;
        }

        let journal_path = operation_path.join(MIGRATION_JOURNAL_FILE);
        let journal_metadata = match fs::symlink_metadata(&journal_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(MigrationError::Io(error)),
        };
        if journal_metadata.file_type().is_symlink() || !journal_metadata.is_file() {
            return Err(MigrationError::UnsafeJournal(journal_path));
        }

        let journal = load_journal(&journal_path)?;
        let expected_operation = operation_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| MigrationError::UnsafeJournal(operation_path.clone()))?;
        if expected_operation != journal.operation_id.to_string() {
            return Err(MigrationError::InvalidManifest(format!(
                "migration journal operation directory does not match operation id: {}",
                journal_path.display()
            )));
        }
        if journal.phase != MigrationPhase::Completed {
            return Err(MigrationError::InvalidManifest(format!(
                "incomplete migration {} is in phase {:?}",
                journal.operation_id, journal.phase
            )));
        }
    }
    Ok(())
}

fn is_regular_file(path: &Path) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(!metadata.file_type().is_symlink() && metadata.is_file())
}

/// Copy the explicitly listed files into a new staging tree.
///
/// The source is read-only, symlinks are rejected, every copied byte is
/// checked against the manifest, and destination files are created with
/// `create_new` so an existing destination can never be overwritten.
pub fn stage_manifest(
    journal: &MigrationJournal,
    roots: &MigrationRoots,
    staging_root: &Path,
) -> Result<(), MigrationError> {
    validate_manifest(journal)?;
    ensure_real_directory(staging_root)?;
    let used_namespaces = journal
        .files
        .iter()
        .map(|file| file.namespace)
        .collect::<BTreeSet<_>>();
    roots.validate(staging_root, &used_namespaces)?;
    if staging_root.read_dir()?.next().transpose()?.is_some() {
        return Err(MigrationError::StagingNotEmpty(staging_root.to_path_buf()));
    }

    for file in &journal.files {
        let source_root = canonical_existing_directory(roots.source_root(file.namespace))
            .map_err(|error| MigrationError::InvalidManifest(error.to_string()))?;
        let source = source_root.join(&file.source_relative_path);
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| MigrationError::Io(io::Error::new(error.kind(), error.to_string())))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(MigrationError::UnsafeSource(source));
        }
        let canonical_source = fs::canonicalize(&source)?;
        if !same_or_within(&canonical_source, &source_root) {
            return Err(MigrationError::SourceEscapesRoot(source));
        }

        let destination = ensure_destination_parent(staging_root, &file.target_relative_path)?;
        if destination.exists() {
            return Err(MigrationError::DestinationExists(destination));
        }

        let copy_result = copy_and_verify(&canonical_source, &destination, file);
        copy_result?;
        if let Some(parent) = destination.parent() {
            File::open(parent)?.sync_all()?;
        }
    }
    File::open(staging_root)?.sync_all()?;
    Ok(())
}

/// Publish a verified staging tree into the target app-data root.
///
/// Every target file is created with `create_new`; an existing file, directory,
/// or symlink stops the operation before that path is touched. The staging
/// source is authenticated again while it is copied, so callers do not have
/// to trust an earlier directory walk. The caller must hold the migration lock
/// for the whole publish phase.
pub fn publish_manifest(
    journal: &MigrationJournal,
    roots: &MigrationRoots,
    staging_root: &Path,
) -> Result<(), MigrationError> {
    validate_manifest(journal)?;
    ensure_existing_directory(staging_root)?;
    let used_namespaces = journal
        .files
        .iter()
        .map(|file| file.namespace)
        .collect::<BTreeSet<_>>();
    roots.validate(staging_root, &used_namespaces)?;

    ensure_real_directory(&roots.target_app_data)?;

    // Check every destination before copying so a collision cannot leave a
    // partially published tree during a normal publish attempt.
    let destinations = journal
        .files
        .iter()
        .map(|file| {
            let destination =
                ensure_destination_parent(&roots.target_app_data, &file.target_relative_path)?;
            if fs::symlink_metadata(&destination).is_ok() {
                return Err(MigrationError::DestinationExists(destination));
            }
            Ok(destination)
        })
        .collect::<Result<Vec<_>, MigrationError>>()?;

    for (file, destination) in journal.files.iter().zip(destinations) {
        let source = staging_root.join(&file.target_relative_path);
        let source_metadata = fs::symlink_metadata(&source)
            .map_err(|error| MigrationError::Io(io::Error::new(error.kind(), error.to_string())))?;
        if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
            return Err(MigrationError::UnsafeSource(source));
        }
        let canonical_source = fs::canonicalize(&source)?;
        let canonical_staging = fs::canonicalize(staging_root)?;
        if !same_or_within(&canonical_source, &canonical_staging) {
            return Err(MigrationError::SourceEscapesRoot(source));
        }

        let copy_result = copy_and_verify(&canonical_source, &destination, file);
        copy_result?;
        if let Some(parent) = destination.parent() {
            File::open(parent)?.sync_all()?;
        }
    }
    File::open(&roots.target_app_data)?.sync_all()?;
    Ok(())
}

/// Resume a publish after a crash or interruption.
///
/// Existing destination files are accepted only when their size and SHA-256
/// exactly match the manifest. They are skipped in that case; a conflicting
/// file still stops the operation without overwriting it. New files use the
/// same no-clobber copy path as the initial publish.
pub fn resume_published_manifest(
    journal: &MigrationJournal,
    roots: &MigrationRoots,
    staging_root: &Path,
) -> Result<(), MigrationError> {
    validate_manifest(journal)?;
    ensure_existing_directory(staging_root)?;
    let used_namespaces = journal
        .files
        .iter()
        .map(|file| file.namespace)
        .collect::<BTreeSet<_>>();
    roots.validate(staging_root, &used_namespaces)?;
    ensure_real_directory(&roots.target_app_data)?;

    for file in &journal.files {
        let source = staging_root.join(&file.target_relative_path);
        let source_metadata = fs::symlink_metadata(&source)
            .map_err(|error| MigrationError::Io(io::Error::new(error.kind(), error.to_string())))?;
        if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
            return Err(MigrationError::UnsafeSource(source));
        }
        let canonical_source = fs::canonicalize(&source)?;
        let canonical_staging = fs::canonicalize(staging_root)?;
        if !same_or_within(&canonical_source, &canonical_staging) {
            return Err(MigrationError::SourceEscapesRoot(source));
        }

        let destination =
            ensure_destination_parent(&roots.target_app_data, &file.target_relative_path)?;
        match fs::symlink_metadata(&destination) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(MigrationError::UnsafeDestination(destination));
                }
                let canonical_destination = fs::canonicalize(&destination)?;
                let (actual_length, actual_hash) = sha256_file(&canonical_destination)?;
                if actual_length == file.byte_length
                    && actual_hash.eq_ignore_ascii_case(&file.sha256)
                {
                    continue;
                }
                return Err(MigrationError::DestinationExists(destination));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(MigrationError::Io(error)),
        }

        copy_and_verify(&canonical_source, &destination, file)?;
        if let Some(parent) = destination.parent() {
            File::open(parent)?.sync_all()?;
        }
    }
    verify_published_manifest(journal, &roots.target_app_data)
}

/// Re-open and authenticate every file published by a migration journal.
pub fn verify_published_manifest(
    journal: &MigrationJournal,
    target_app_data: &Path,
) -> Result<(), MigrationError> {
    validate_manifest(journal)?;
    let target_metadata = fs::symlink_metadata(target_app_data)?;
    if target_metadata.file_type().is_symlink() || !target_metadata.is_dir() {
        return Err(MigrationError::UnsafeSource(target_app_data.to_path_buf()));
    }
    let canonical_target = fs::canonicalize(target_app_data)?;

    for file in &journal.files {
        let path = target_app_data.join(&file.target_relative_path);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(MigrationError::UnsafeSource(path));
        }
        let canonical_path = fs::canonicalize(&path)?;
        if !same_or_within(&canonical_path, &canonical_target) {
            return Err(MigrationError::SourceEscapesRoot(path));
        }
        let (actual_length, actual_hash) = sha256_file(&canonical_path)?;
        if actual_length != file.byte_length {
            return Err(MigrationError::SizeMismatch {
                path: canonical_path,
                expected: file.byte_length,
                actual: actual_length,
            });
        }
        if !actual_hash.eq_ignore_ascii_case(&file.sha256) {
            return Err(MigrationError::HashMismatch {
                path: canonical_path,
                expected: file.sha256.clone(),
                actual: actual_hash,
            });
        }
    }
    Ok(())
}

fn copy_and_verify(
    source: &Path,
    destination: &Path,
    manifest: &MigrationFile,
) -> Result<(), MigrationError> {
    let mut input = File::open(source)?;
    let parent = destination.parent().ok_or_else(|| {
        MigrationError::InvalidManifest("migration destination has no parent directory".to_string())
    })?;
    let file_name = destination.file_name().ok_or_else(|| {
        MigrationError::InvalidManifest("migration destination has no file name".to_string())
    })?;
    let temporary = parent.join(format!(
        ".{}.{}.copy.tmp",
        file_name.to_string_lossy(),
        Uuid::new_v4()
    ));
    let mut output = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
    {
        Ok(output) => output,
        Err(error) => return Err(MigrationError::Io(error)),
    };

    let copy_result = (|| {
        let mut hasher = Sha256::new();
        let mut length = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
            length = length.checked_add(read as u64).ok_or_else(|| {
                MigrationError::InvalidManifest("source file is too large".to_string())
            })?;
        }
        output.sync_all()?;

        if length != manifest.byte_length {
            return Err(MigrationError::SizeMismatch {
                path: manifest.source_relative_path.clone(),
                expected: manifest.byte_length,
                actual: length,
            });
        }
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(&manifest.sha256) {
            return Err(MigrationError::HashMismatch {
                path: manifest.source_relative_path.clone(),
                expected: manifest.sha256.clone(),
                actual,
            });
        }
        Ok(())
    })();
    drop(output);

    if let Err(error) = copy_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    let publish_result = match fs::hard_link(&temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            Err(MigrationError::DestinationExists(destination.to_path_buf()))
        }
        Err(error) => Err(MigrationError::Io(error)),
    };
    let cleanup_result = fs::remove_file(&temporary);
    match publish_result {
        Err(error) => {
            let _ = cleanup_result;
            Err(error)
        }
        Ok(()) => cleanup_result.map_err(MigrationError::Io),
    }
}

pub fn sha256_file(path: &Path) -> io::Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "file is too large"))?;
    }
    Ok((length, format!("{:x}", hasher.finalize())))
}

fn sqlite_sidecar_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

fn sqlite_trio_paths(database_path: &Path) -> [PathBuf; 3] {
    [
        database_path.to_path_buf(),
        sqlite_sidecar_path(database_path, "-wal"),
        sqlite_sidecar_path(database_path, "-shm"),
    ]
}

type SqliteTrioFingerprint = [Option<(u64, String)>; 3];

fn fingerprint_sqlite_trio(database_path: &Path) -> Result<SqliteTrioFingerprint, MigrationError> {
    let mut fingerprint = [None, None, None];
    for (index, component) in sqlite_trio_paths(database_path).into_iter().enumerate() {
        let metadata = match fs::symlink_metadata(&component) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(MigrationError::Io(error)),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(MigrationError::UnsafeSource(component));
        }
        fingerprint[index] = Some(sha256_file(&component)?);
    }
    if fingerprint[0].is_none() {
        return Err(MigrationError::InvalidManifest(format!(
            "SQLite main database is missing: {}",
            database_path.display()
        )));
    }
    Ok(fingerprint)
}

fn remove_sqlite_trio_if_exists(database_path: &Path) -> Result<(), MigrationError> {
    for component in sqlite_trio_paths(database_path) {
        match fs::remove_file(&component) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(MigrationError::Io(error)),
        }
    }
    Ok(())
}

fn sqlite_error(error: sqlx::Error) -> MigrationError {
    MigrationError::Sqlite(error.to_string())
}

async fn verify_sqlite_snapshot(path: &Path, immutable: bool) -> Result<(), MigrationError> {
    let options = SqliteConnectOptions::from_str(path.to_string_lossy().as_ref())
        .map_err(sqlite_error)?
        .read_only(true)
        .immutable(immutable)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(sqlite_error)?;
    let result = sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await
        .map_err(sqlite_error);
    pool.close().await;
    match result? {
        value if value.eq_ignore_ascii_case("ok") => Ok(()),
        value => Err(MigrationError::Sqlite(format!(
            "SQLite integrity_check returned {value}"
        ))),
    }
}

/// Create a coherent, main-file SQLite snapshot without mutating the source.
///
/// The source main file and any existing `-wal`/`-shm` sidecars are copied to
/// a private staging trio first. SQLite is opened read-only on that private
/// trio, integrity-checked, and materialized with `VACUUM INTO`. The source
/// trio is fingerprinted again before publication; a concurrent writer causes
/// the snapshot to be discarded instead of publishing a mixed view. The
/// resulting main-only database is reopened with SQLite's immutable mode after
/// the staging connection is closed, which verifies the artifact survives a
/// real close/reopen boundary.
pub async fn snapshot_sqlite_database(
    source: &Path,
    snapshot: &Path,
    staging: &Path,
) -> Result<(), MigrationError> {
    let snapshot_root = canonical_for_relationship(snapshot.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "SQLite snapshot has no parent")
    })?)?;
    let staging_root = canonical_for_relationship(staging.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "SQLite staging has no parent")
    })?)?;
    let source_canonical = fs::canonicalize(source)?;
    let snapshot_canonical = snapshot_root.join(snapshot.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "SQLite snapshot has no file name",
        )
    })?);
    let staging_canonical = staging_root.join(staging.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "SQLite staging has no file name",
        )
    })?);
    if same_or_within(&source_canonical, &snapshot_canonical)
        || same_or_within(&snapshot_canonical, &source_canonical)
        || same_or_within(&source_canonical, &staging_canonical)
        || same_or_within(&staging_canonical, &source_canonical)
        || same_or_within(&snapshot_canonical, &staging_canonical)
        || same_or_within(&staging_canonical, &snapshot_canonical)
    {
        return Err(MigrationError::OverlappingRoots);
    }
    if sqlite_trio_paths(snapshot).iter().any(|path| path.exists())
        || sqlite_trio_paths(staging).iter().any(|path| path.exists())
    {
        return Err(MigrationError::DestinationExists(snapshot.to_path_buf()));
    }

    let source_before = fingerprint_sqlite_trio(source)?;
    if let Some(parent) = snapshot.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = staging.parent() {
        fs::create_dir_all(parent)?;
    }

    let result = async {
        for (index, source_component) in sqlite_trio_paths(source).into_iter().enumerate() {
            let Some(expected) = &source_before[index] else {
                continue;
            };
            let staging_component = sqlite_trio_paths(staging)[index].clone();
            fs::copy(&source_component, &staging_component)?;
            File::open(&staging_component)?.sync_all()?;
            if sha256_file(&staging_component)? != *expected {
                return Err(MigrationError::SourceChanged);
            }
        }
        if fingerprint_sqlite_trio(source)? != source_before {
            return Err(MigrationError::SourceChanged);
        }

        verify_sqlite_snapshot(staging, false).await?;
        let options = SqliteConnectOptions::from_str(staging.to_string_lossy().as_ref())
            .map_err(sqlite_error)?
            .read_only(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(sqlite_error)?;
        sqlx::query("VACUUM INTO ?")
            .bind(snapshot.to_string_lossy().to_string())
            .execute(&pool)
            .await
            .map_err(sqlite_error)?;
        pool.close().await;
        File::open(snapshot)?.sync_all()?;

        if fingerprint_sqlite_trio(source)? != source_before {
            return Err(MigrationError::SourceChanged);
        }
        verify_sqlite_snapshot(snapshot, true).await?;
        Ok(())
    }
    .await;

    let cleanup_result = remove_sqlite_trio_if_exists(staging);
    if result.is_err() {
        let _ = remove_sqlite_trio_if_exists(snapshot);
    }
    cleanup_result?;
    result
}

fn validate_relative_path(path: &Path) -> Result<(), MigrationError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(MigrationError::InvalidManifest(format!(
            "path must be non-empty and relative: {}",
            path.display()
        )));
    }
    Ok(())
}

fn canonical_existing_directory(path: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("root is not an owned directory: {}", path.display()),
        ));
    }
    fs::canonicalize(path)
}

fn ensure_existing_directory(path: &Path) -> Result<PathBuf, MigrationError> {
    let metadata = fs::symlink_metadata(path).map_err(MigrationError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MigrationError::UnsafeDestination(path.to_path_buf()));
    }
    Ok(fs::canonicalize(path)?)
}

/// Create a directory path one component at a time, rejecting symlinks in
/// every component. `create_dir_all` is deliberately avoided because it can
/// follow a symlinked intermediate directory outside the target root.
pub(crate) fn ensure_real_directory(path: &Path) -> Result<PathBuf, MigrationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(MigrationError::UnsafeDestination(path.to_path_buf()));
            }
            return Ok(fs::canonicalize(path)?);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(MigrationError::Io(error)),
    }

    let parent = path.parent().ok_or_else(|| {
        MigrationError::InvalidManifest("destination directory has no parent".to_string())
    })?;
    ensure_real_directory(parent)?;
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(MigrationError::Io(error)),
    }
    ensure_existing_directory(path)
}

/// Return a destination path after safely creating only its intermediate
/// directories below `root`.
fn ensure_destination_parent(root: &Path, relative: &Path) -> Result<PathBuf, MigrationError> {
    validate_relative_path(relative)?;
    let canonical_root = ensure_existing_directory(root)?;
    let mut current = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return Err(MigrationError::InvalidManifest(format!(
                "destination path must contain only normal components: {}",
                relative.display()
            )));
        };
        if components.peek().is_none() {
            break;
        }
        current.push(name);
        ensure_real_directory(&current)?;
        let canonical_current = fs::canonicalize(&current)?;
        if !same_or_within(&canonical_current, &canonical_root) {
            return Err(MigrationError::SourceEscapesRoot(current));
        }
    }
    Ok(root.join(relative))
}

fn canonical_for_relationship(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path);
    }
    let mut missing = Vec::new();
    let mut ancestor = path;
    while !ancestor.exists() {
        let name = ancestor.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "path has no existing ancestor")
        })?;
        missing.push(name.to_os_string());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    }
    let mut canonical = fs::canonicalize(ancestor)?;
    for component in missing.iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

fn same_or_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn roots(root: &Path) -> MigrationRoots {
        MigrationRoots {
            legacy_app_data: root.join("legacy-app"),
            legacy_named_config: root.join("legacy-config"),
            legacy_named_data: root.join("legacy-data"),
            web_export: root.join("web-export"),
            target_app_data: root.join("target-app"),
        }
    }

    fn journal(file: &Path, target: &str) -> MigrationJournal {
        let (byte_length, sha256) = sha256_file(file).unwrap();
        MigrationJournal {
            format_version: MIGRATION_FORMAT_VERSION,
            operation_id: Uuid::new_v4(),
            source_identity: "com.old.meetily".to_string(),
            target_identity: "com.new.meetily".to_string(),
            phase: MigrationPhase::Preparing,
            files: vec![MigrationFile {
                namespace: MigrationNamespace::LegacyAppData,
                source_relative_path: PathBuf::from("stores/preferences.json"),
                target_relative_path: PathBuf::from(target),
                byte_length,
                sha256,
            }],
        }
    }

    fn prepare_source(root: &Path, contents: &[u8]) -> (MigrationRoots, PathBuf) {
        let roots = roots(root);
        fs::create_dir_all(roots.legacy_app_data.join("stores")).unwrap();
        fs::create_dir_all(&roots.legacy_named_config).unwrap();
        fs::create_dir_all(&roots.legacy_named_data).unwrap();
        fs::create_dir_all(&roots.web_export).unwrap();
        let source = roots.legacy_app_data.join("stores/preferences.json");
        fs::write(&source, contents).unwrap();
        (roots, source)
    }

    #[test]
    fn migration_budget_uses_checked_formula_and_minimum_reserve() {
        let estimate = estimate_migration_budget(MigrationBudget {
            sqlite_bytes: 10,
            file_bytes: 20,
            web_export_bytes: 30,
        })
        .unwrap();

        assert_eq!(
            estimate.temporary_bytes,
            FIXED_MIGRATION_OVERHEAD_BYTES + 150
        );
        assert_eq!(estimate.reserve_bytes, MIN_MIGRATION_RESERVE_BYTES);
        assert_eq!(
            estimate.required_bytes,
            FIXED_MIGRATION_OVERHEAD_BYTES + 150 + MIN_MIGRATION_RESERVE_BYTES
        );
    }

    #[test]
    fn migration_budget_uses_rounded_fraction_for_large_transfers() {
        let estimate = estimate_migration_budget(MigrationBudget {
            sqlite_bytes: 400 * 1024 * 1024,
            file_bytes: 0,
            web_export_bytes: 0,
        })
        .unwrap();

        assert_eq!(
            estimate.temporary_bytes,
            7 * 400 * 1024 * 1024 + FIXED_MIGRATION_OVERHEAD_BYTES
        );
        assert_eq!(
            estimate.reserve_bytes,
            estimate.temporary_bytes.div_ceil(10)
        );
        assert!(estimate.reserve_bytes > MIN_MIGRATION_RESERVE_BYTES);
    }

    #[test]
    fn migration_budget_rejects_arithmetic_overflow() {
        assert!(matches!(
            estimate_migration_budget(MigrationBudget {
                sqlite_bytes: u64::MAX,
                file_bytes: 0,
                web_export_bytes: 0,
            }),
            Err(MigrationError::BudgetOverflow)
        ));
    }

    #[test]
    fn migration_budget_rejects_insufficient_free_space() {
        let estimate = estimate_migration_budget(MigrationBudget {
            sqlite_bytes: 10,
            file_bytes: 20,
            web_export_bytes: 30,
        })
        .unwrap();

        assert!(matches!(
            ensure_migration_free_space(estimate.required_bytes - 1, estimate),
            Err(MigrationError::InsufficientDiskSpace {
                required,
                available
            }) if required == estimate.required_bytes && available + 1 == required
        ));
    }

    #[test]
    fn migration_budget_accepts_exact_free_space_snapshot() {
        let estimate = check_migration_budget(
            FIXED_MIGRATION_OVERHEAD_BYTES + MIN_MIGRATION_RESERVE_BYTES,
            MigrationBudget {
                sqlite_bytes: 0,
                file_bytes: 0,
                web_export_bytes: 0,
            },
        )
        .unwrap();
        assert_eq!(
            estimate.required_bytes,
            FIXED_MIGRATION_OVERHEAD_BYTES + MIN_MIGRATION_RESERVE_BYTES
        );
    }

    #[cfg(unix)]
    #[test]
    fn caller_free_space_measurement_uses_existing_ancestor_for_new_path() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("new-target").join("database.sqlite");
        let available = available_migration_free_space(&destination).unwrap();

        assert!(available > 0);
        assert!(available_migration_free_space(directory.path()).unwrap() > 0);
    }

    #[test]
    fn migration_lock_serializes_operations_and_releases_on_drop() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("migration.lock");
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();
        let first = acquire_migration_lock(&path, first_id).unwrap();

        assert!(matches!(
            acquire_migration_lock(&path, second_id),
            Err(MigrationError::LockHeld(existing)) if existing == path
        ));
        drop(first);

        let _second = acquire_migration_lock(&path, second_id).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), format!("{second_id}\n"));
    }

    #[test]
    fn migration_lock_rejects_symlink_without_touching_target() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("target.lock");
        let path = directory.path().join("migration.lock");
        fs::write(&target, b"keep\n").unwrap();
        symlink(&target, &path).unwrap();

        assert!(matches!(
            acquire_migration_lock(&path, Uuid::new_v4()),
            Err(MigrationError::UnsafeLock(existing)) if existing == path
        ));
        assert_eq!(fs::read(&target).unwrap(), b"keep\n");
    }

    #[test]
    fn stage_manifest_copies_and_verifies_explicit_files() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), br#"{"theme":"dark"}"#);
        let journal = journal(&source, "stores/preferences.json");
        let staging = directory.path().join("staging");

        stage_manifest(&journal, &roots, &staging).unwrap();

        assert_eq!(
            fs::read(staging.join("stores/preferences.json")).unwrap(),
            fs::read(source).unwrap()
        );
    }

    #[test]
    fn publish_manifest_verifies_and_never_clobbers_target_files() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "stores/preferences.json");
        let staging = directory.path().join("staging");

        stage_manifest(&journal, &roots, &staging).unwrap();
        publish_manifest(&journal, &roots, &staging).unwrap();
        verify_published_manifest(&journal, &roots.target_app_data).unwrap();

        let destination = roots.target_app_data.join("stores/preferences.json");
        assert_eq!(fs::read(&destination).unwrap(), b"safe");
        assert!(matches!(
            publish_manifest(&journal, &roots, &staging),
            Err(MigrationError::DestinationExists(existing)) if existing == destination
        ));
        assert_eq!(fs::read(destination).unwrap(), b"safe");
    }

    #[test]
    fn publish_manifest_rejects_symlinked_destination_parent_without_writing_outside() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "stores/preferences.json");
        let staging = directory.path().join("staging");
        stage_manifest(&journal, &roots, &staging).unwrap();

        fs::create_dir_all(&roots.target_app_data).unwrap();
        let outside = directory.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, roots.target_app_data.join("stores")).unwrap();

        assert!(matches!(
            publish_manifest(&journal, &roots, &staging),
            Err(MigrationError::UnsafeDestination(path))
                if path == roots.target_app_data.join("stores")
        ));
        assert!(!outside.join("preferences.json").exists());
    }

    #[test]
    fn publish_manifest_accepts_uppercase_sha_on_readback() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let mut journal = journal(&source, "preferences.json");
        journal.files[0].sha256 = journal.files[0].sha256.to_uppercase();
        let staging = directory.path().join("staging");

        stage_manifest(&journal, &roots, &staging).unwrap();
        publish_manifest(&journal, &roots, &staging).unwrap();
        verify_published_manifest(&journal, &roots.target_app_data).unwrap();
    }

    #[test]
    fn resume_publish_skips_matching_files_and_finishes_missing_files() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let second_source = roots.legacy_app_data.join("stores/second.json");
        fs::write(&second_source, b"second").unwrap();
        let mut journal = journal(&source, "stores/preferences.json");
        let (second_length, second_sha256) = sha256_file(&second_source).unwrap();
        journal.files.push(MigrationFile {
            namespace: MigrationNamespace::LegacyAppData,
            source_relative_path: PathBuf::from("stores/second.json"),
            target_relative_path: PathBuf::from("stores/second.json"),
            byte_length: second_length,
            sha256: second_sha256,
        });
        let staging = directory.path().join("staging");
        stage_manifest(&journal, &roots, &staging).unwrap();

        let first_destination = roots.target_app_data.join("stores/preferences.json");
        fs::create_dir_all(first_destination.parent().unwrap()).unwrap();
        fs::copy(staging.join("stores/preferences.json"), &first_destination).unwrap();

        resume_published_manifest(&journal, &roots, &staging).unwrap();
        verify_published_manifest(&journal, &roots.target_app_data).unwrap();
        assert_eq!(
            fs::read(roots.target_app_data.join("stores/second.json")).unwrap(),
            b"second"
        );
    }

    #[test]
    fn copy_failure_never_removes_an_existing_destination() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "preferences.json");
        let destination = directory.path().join("existing.json");
        fs::write(&destination, b"keep").unwrap();

        assert!(matches!(
            copy_and_verify(&source, &destination, &journal.files[0]),
            Err(MigrationError::DestinationExists(existing)) if existing == destination
        ));
        assert_eq!(fs::read(destination).unwrap(), b"keep");
    }

    #[test]
    fn copy_failure_does_not_leave_a_partial_final_destination() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let mut journal = journal(&source, "preferences.json");
        journal.files[0].sha256 = "0".repeat(64);
        let destination = directory.path().join("new.json");

        assert!(matches!(
            copy_and_verify(&source, &destination, &journal.files[0]),
            Err(MigrationError::HashMismatch { .. })
        ));
        assert!(!destination.exists());
        let temporary_files = fs::read_dir(directory.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".copy.tmp"))
            .count();
        assert_eq!(temporary_files, 0);
    }

    #[test]
    fn journal_is_durable_and_repeated_identical_write_is_idempotent() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "preferences.json");
        let path = directory.path().join("migration/journal.json");

        persist_journal(&path, &journal).unwrap();
        assert_eq!(load_journal(&path).unwrap(), journal);
        persist_journal(&path, &journal).unwrap();
        let leftovers = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".journal.json.")
            })
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn journal_phase_updates_are_monotonic_and_idempotent() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "preferences.json");
        let path = directory.path().join("migration/journal.json");

        persist_journal(&path, &journal).unwrap();
        update_journal_phase(&path, MigrationPhase::Prepared).unwrap();
        assert_eq!(load_journal(&path).unwrap().phase, MigrationPhase::Prepared);
        update_journal_phase(&path, MigrationPhase::Prepared).unwrap();
        update_journal_phase(&path, MigrationPhase::Publishing).unwrap();
        assert_eq!(
            load_journal(&path).unwrap().phase,
            MigrationPhase::Publishing
        );
        assert!(matches!(
            update_journal_phase(&path, MigrationPhase::Preparing),
            Err(MigrationError::InvalidManifest(message))
                if message.contains("cannot move")
        ));
        assert_eq!(
            load_journal(&path).unwrap().phase,
            MigrationPhase::Publishing
        );
    }

    #[test]
    fn startup_guard_allows_missing_or_completed_operations() {
        let directory = tempdir().unwrap();
        ensure_no_incomplete_migrations(directory.path()).unwrap();

        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let mut completed = journal(&source, "preferences.json");
        completed.phase = MigrationPhase::Completed;
        let operation_dir = directory
            .path()
            .join(MIGRATION_NAMESPACE_DIR)
            .join(completed.operation_id.to_string());
        let journal_path = operation_dir.join(MIGRATION_JOURNAL_FILE);
        persist_journal(&journal_path, &completed).unwrap();

        ensure_no_incomplete_migrations(directory.path()).unwrap();
    }

    #[test]
    fn startup_guard_rejects_incomplete_operation_before_database_open() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let pending = journal(&source, "preferences.json");
        let operation_dir = directory
            .path()
            .join(MIGRATION_NAMESPACE_DIR)
            .join(pending.operation_id.to_string());
        let journal_path = operation_dir.join(MIGRATION_JOURNAL_FILE);
        persist_journal(&journal_path, &pending).unwrap();

        let error = ensure_no_incomplete_migrations(directory.path()).unwrap_err();
        assert!(
            matches!(error, MigrationError::InvalidManifest(message) if message.contains("incomplete migration") && message.contains("Preparing"))
        );
    }

    #[test]
    fn journal_never_overwrites_a_different_operation() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "preferences.json");
        let path = directory.path().join("journal.json");
        persist_journal(&path, &journal).unwrap();

        let mut replacement = journal.clone();
        replacement.operation_id = Uuid::new_v4();
        assert!(matches!(
            persist_journal(&path, &replacement),
            Err(MigrationError::DestinationExists(existing)) if existing == path
        ));
        assert_eq!(load_journal(&path).unwrap(), journal);
    }

    #[test]
    fn journal_symlink_is_rejected_without_touching_target() {
        let directory = tempdir().unwrap();
        let (_roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "preferences.json");
        let target = directory.path().join("target.json");
        let path = directory.path().join("journal.json");
        fs::write(&target, b"keep").unwrap();
        symlink(&target, &path).unwrap();

        assert!(matches!(
            persist_journal(&path, &journal),
            Err(MigrationError::UnsafeJournal(existing)) if existing == path
        ));
        assert_eq!(fs::read(&target).unwrap(), b"keep");
    }

    #[test]
    fn manifest_rejects_path_traversal_before_destination_creation() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let mut journal = journal(&source, "safe.txt");
        journal.files[0].target_relative_path = PathBuf::from("../escape.txt");
        let staging = directory.path().join("staging");

        assert!(matches!(
            stage_manifest(&journal, &roots, &staging),
            Err(MigrationError::InvalidManifest(_))
        ));
        assert!(!directory.path().join("escape.txt").exists());
    }

    #[test]
    fn source_symlink_is_rejected() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let outside = directory.path().join("outside.txt");
        fs::write(&outside, b"outside").unwrap();
        fs::remove_file(&source).unwrap();
        symlink(&outside, &source).unwrap();
        let journal = MigrationJournal {
            files: vec![MigrationFile {
                namespace: MigrationNamespace::LegacyAppData,
                source_relative_path: PathBuf::from("stores/preferences.json"),
                target_relative_path: PathBuf::from("preferences.json"),
                byte_length: 7,
                sha256: format!("{:x}", Sha256::digest(b"outside")),
            }],
            ..journal(&outside, "unused.txt")
        };

        assert!(matches!(
            stage_manifest(&journal, &roots, &directory.path().join("staging")),
            Err(MigrationError::UnsafeSource(_))
        ));
    }

    #[test]
    fn existing_destination_is_never_overwritten() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"new");
        let journal = journal(&source, "preferences.json");
        let staging = directory.path().join("staging");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("preferences.json"), b"old").unwrap();

        assert!(matches!(
            stage_manifest(&journal, &roots, &staging),
            Err(MigrationError::StagingNotEmpty(_))
        ));
        assert_eq!(fs::read(staging.join("preferences.json")).unwrap(), b"old");
    }

    #[test]
    fn source_change_is_detected_and_partial_destination_is_removed() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"original");
        let mut journal = journal(&source, "preferences.json");
        fs::write(&source, b"changed").unwrap();
        let staging = directory.path().join("staging");

        assert!(matches!(
            stage_manifest(&journal, &roots, &staging),
            Err(MigrationError::SizeMismatch { .. } | MigrationError::HashMismatch { .. })
        ));
        assert!(!staging.join("preferences.json").exists());
        journal.files[0].byte_length = 0;
    }

    #[test]
    fn overlapping_roots_are_rejected() {
        let directory = tempdir().unwrap();
        let (mut roots, source) = prepare_source(directory.path(), b"safe");
        roots.target_app_data = roots.legacy_app_data.join("new");
        let journal = journal(&source, "preferences.json");

        assert!(matches!(
            stage_manifest(&journal, &roots, &directory.path().join("staging")),
            Err(MigrationError::OverlappingRoots)
        ));
    }

    #[test]
    fn missing_unused_source_roots_are_allowed() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        fs::remove_dir_all(&roots.legacy_named_config).unwrap();
        fs::remove_dir_all(&roots.legacy_named_data).unwrap();
        fs::remove_dir_all(&roots.web_export).unwrap();
        let journal = journal(&source, "preferences.json");

        stage_manifest(&journal, &roots, &directory.path().join("staging")).unwrap();
        assert_eq!(
            fs::read(directory.path().join("staging/preferences.json")).unwrap(),
            b"safe"
        );
    }

    #[test]
    fn missing_used_source_root_is_rejected() {
        let directory = tempdir().unwrap();
        let (roots, source) = prepare_source(directory.path(), b"safe");
        let journal = journal(&source, "preferences.json");
        fs::remove_dir_all(&roots.legacy_app_data).unwrap();

        assert!(matches!(
            stage_manifest(&journal, &roots, &directory.path().join("staging")),
            Err(MigrationError::InvalidManifest(_))
        ));
    }

    #[tokio::test]
    async fn sqlite_snapshot_reopens_after_wal_staging_without_mutating_source() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.sqlite");
        let staging = directory.path().join("private.sqlite");
        let snapshot = directory.path().join("snapshot.sqlite");
        let options = SqliteConnectOptions::from_str(source.to_string_lossy().as_ref())
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA wal_autocheckpoint=1000000")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE meetings (id INTEGER PRIMARY KEY, title TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO meetings (title) VALUES ('kept across WAL')")
            .execute(&pool)
            .await
            .unwrap();

        let source_before = fingerprint_sqlite_trio(&source).unwrap();
        assert!(source_before[0].is_some());
        assert!(
            source_before[1].is_some(),
            "fixture did not leave a WAL sidecar"
        );

        snapshot_sqlite_database(&source, &snapshot, &staging)
            .await
            .unwrap();

        assert_eq!(fingerprint_sqlite_trio(&source).unwrap(), source_before);
        assert!(snapshot.exists());
        assert!(!staging.exists());
        assert!(!sqlite_sidecar_path(&staging, "-wal").exists());
        assert!(!sqlite_sidecar_path(&staging, "-shm").exists());

        let snapshot_options = SqliteConnectOptions::from_str(snapshot.to_string_lossy().as_ref())
            .unwrap()
            .read_only(true)
            .immutable(true)
            .foreign_keys(true);
        let reopened = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(snapshot_options)
            .await
            .unwrap();
        let title = sqlx::query_scalar::<_, String>("SELECT title FROM meetings WHERE id = 1")
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(title, "kept across WAL");
        reopened.close().await;
        pool.close().await;
    }

    #[tokio::test]
    async fn sqlite_snapshot_rejects_existing_destination_without_touching_it() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.sqlite");
        let staging = directory.path().join("private.sqlite");
        let snapshot = directory.path().join("snapshot.sqlite");
        fs::write(&source, b"not sqlite").unwrap();
        fs::write(&snapshot, b"keep this file").unwrap();

        let result = snapshot_sqlite_database(&source, &snapshot, &staging).await;

        assert!(matches!(result, Err(MigrationError::DestinationExists(path)) if path == snapshot));
        assert_eq!(fs::read(&snapshot).unwrap(), b"keep this file");
        assert!(!staging.exists());
    }
}
