use crate::database::repositories::meeting::MeetingsRepository;
use crate::database::repositories::summary::SummaryProcessesRepository;
use crate::summary::service::SummaryService;
use sqlx::{Result, Sqlite, SqlitePool, Transaction};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tauri::Manager;
use uuid::Uuid;

use super::legacy_fastapi::{
    acquire_app_data_gate, classify_path_without_mutation, migrate_legacy_source_already_gated,
    recover_import_already_gated, register_live_target, target_is_live, unregister_live_target,
    DatabaseClass,
};
use super::migration::ensure_no_incomplete_migrations;

fn ensure_database_startup_allowed(app_data_dir: &Path) -> Result<()> {
    ensure_no_incomplete_migrations(app_data_dir).map_err(|error| {
        sqlx::Error::Protocol(format!(
            "refusing database access while data migration is incomplete: {error}"
        ))
    })
}

fn sqlite_sidecar_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

fn reject_orphaned_sidecars(database_path: &Path) -> Result<()> {
    if optional_symlink_metadata(database_path)?.is_none()
        && (optional_symlink_metadata(&sqlite_sidecar_path(database_path, "-wal"))?.is_some()
            || optional_symlink_metadata(&sqlite_sidecar_path(database_path, "-shm"))?.is_some())
    {
        return Err(sqlx::Error::Protocol(
            "refusing orphaned SQLite target sidecars".to_string(),
        ));
    }
    Ok(())
}

fn optional_symlink_metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(sqlx::Error::Io(error)),
    }
}

#[cfg(unix)]
fn metadata_identifies_same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(unix)]
fn metadata_is_uniquely_linked(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() == 1
}

#[cfg(not(unix))]
fn metadata_is_uniquely_linked(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(not(unix))]
fn metadata_identifies_same_file(_left: &fs::Metadata, _right: &fs::Metadata) -> bool {
    false
}

fn validate_owned_database_paths(current_path: &Path, legacy_path: &Path) -> Result<()> {
    validate_database_path_shapes(current_path, legacy_path)?;
    for component in sqlite_trio_paths(current_path) {
        if let Some(metadata) = optional_symlink_metadata(&component)? {
            if !metadata_is_uniquely_linked(&metadata) {
                return Err(sqlx::Error::Protocol(
                    "current database target must be an owned regular file".to_string(),
                ));
            }
        }
    }
    for component in sqlite_trio_paths(legacy_path) {
        if let Some(metadata) = optional_symlink_metadata(&component)? {
            if !metadata_is_uniquely_linked(&metadata) {
                return Err(sqlx::Error::Protocol(
                    "legacy database path must be an owned regular file".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn sqlite_trio_paths(database_path: &Path) -> [PathBuf; 3] {
    [
        database_path.to_path_buf(),
        sqlite_sidecar_path(database_path, "-wal"),
        sqlite_sidecar_path(database_path, "-shm"),
    ]
}

fn validate_database_path_shapes(current_path: &Path, legacy_path: &Path) -> Result<()> {
    let current = optional_symlink_metadata(current_path)?;
    for component in sqlite_trio_paths(current_path) {
        if let Some(metadata) = optional_symlink_metadata(&component)? {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(sqlx::Error::Protocol(
                    "current database target must be an owned regular file".to_string(),
                ));
            }
        }
    }

    let legacy = optional_symlink_metadata(legacy_path)?;
    for component in sqlite_trio_paths(legacy_path) {
        if let Some(metadata) = optional_symlink_metadata(&component)? {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(sqlx::Error::Protocol(
                    "legacy database path must be an owned regular file".to_string(),
                ));
            }
        }
    }

    if let (Some(current), Some(legacy)) = (&current, &legacy) {
        let canonical_alias = fs::canonicalize(current_path).map_err(sqlx::Error::Io)?
            == fs::canonicalize(legacy_path).map_err(sqlx::Error::Io)?;
        if canonical_alias || metadata_identifies_same_file(current, legacy) {
            return Err(sqlx::Error::Protocol(
                "current and legacy database paths alias the same file".to_string(),
            ));
        }
    }
    Ok(())
}

/// Preserve the database and every existing WAL sidecar before destructive
/// recovery. A backup failure leaves all originals untouched and aborts the
/// cleanup path.
pub fn backup_sqlite_recovery_files(database_path: &Path) -> io::Result<PathBuf> {
    let parent = database_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "database path has no parent")
    })?;
    let recovery_root = parent.join("database-recovery");
    fs::create_dir_all(&recovery_root)?;
    let backup_dir = recovery_root.join(format!("sqlite-recovery-{}", Uuid::new_v4()));
    fs::create_dir(&backup_dir)?;

    let sources = [
        database_path.to_path_buf(),
        sqlite_sidecar_path(database_path, "-wal"),
        sqlite_sidecar_path(database_path, "-shm"),
    ];
    let mut copied = 0;
    for source in sources.iter().filter(|path| path.exists()) {
        let file_name = source.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "SQLite file has no file name")
        })?;
        let destination = backup_dir.join(file_name);
        fs::copy(source, &destination)?;
        fs::File::open(&destination)?.sync_all()?;
        copied += 1;
    }

    if copied == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no SQLite recovery files exist",
        ));
    }
    fs::File::open(&backup_dir)?.sync_all()?;
    fs::File::open(&recovery_root)?.sync_all()?;
    Ok(backup_dir)
}

#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
    live_token: Arc<()>,
    registered_path: PathBuf,
}

impl DatabaseManager {
    pub async fn new(tauri_db_path: &str, backend_db_path: &str) -> Result<Self> {
        let current_path = Path::new(tauri_db_path);
        let app_data_dir = current_path.parent().ok_or_else(|| {
            sqlx::Error::Protocol("database path has no parent directory".to_string())
        })?;
        let _gate = acquire_app_data_gate(app_data_dir).await?;
        fs::create_dir_all(app_data_dir).map_err(sqlx::Error::Io)?;
        Self::new_already_gated(current_path, Path::new(backend_db_path), app_data_dir).await
    }

    async fn open_existing_native(path: &Path) -> Result<Self> {
        let app_data_dir = path.parent().ok_or_else(|| {
            sqlx::Error::Protocol("database path has no parent directory".to_string())
        })?;
        if classify_path_without_mutation(path, app_data_dir).await?
            != DatabaseClass::AuthenticSqlxPrefix
        {
            return Err(sqlx::Error::Protocol(
                "current database is not an authentic SQLx migration prefix".to_string(),
            ));
        }
        let options =
            sqlx::sqlite::SqliteConnectOptions::from_str(path.to_string_lossy().as_ref())?
                .foreign_keys(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        SummaryProcessesRepository::recover_interrupted_processes(&pool).await?;
        MeetingsRepository::recover_interrupted_lifecycle(&pool).await?;
        for meeting_id in SummaryProcessesRepository::pending_export_meeting_ids(&pool).await? {
            if let Err(error) = SummaryService::reconcile_accepted_summary(&pool, &meeting_id).await
            {
                log::warn!(
                    "Summary export remains pending after database reopen for {}: {}",
                    meeting_id,
                    error
                );
            }
        }
        crate::database::repositories::search::SearchRepository::rebuild(&pool).await?;
        Self::from_pool(path, pool)
    }

    async fn create_fresh(path: &Path) -> Result<Self> {
        if path.exists() {
            return Err(sqlx::Error::Protocol(
                "fresh database path already exists".to_string(),
            ));
        }
        reject_orphaned_sidecars(path)?;
        let options =
            sqlx::sqlite::SqliteConnectOptions::from_str(path.to_string_lossy().as_ref())?
                .create_if_missing(true)
                .foreign_keys(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        crate::database::repositories::search::SearchRepository::rebuild(&pool).await?;
        Self::from_pool(path, pool)
    }

    fn from_pool(path: &Path, pool: SqlitePool) -> Result<Self> {
        let live_token = Arc::new(());
        let registered_path = register_live_target(path, &live_token)?;
        Ok(Self {
            pool,
            live_token,
            registered_path,
        })
    }

    pub(crate) async fn new_already_gated(
        current_path: &Path,
        legacy_path: &Path,
        app_data_dir: &Path,
    ) -> Result<Self> {
        ensure_database_startup_allowed(app_data_dir)?;
        validate_database_path_shapes(current_path, legacy_path)?;
        recover_import_already_gated(app_data_dir).await?;
        validate_owned_database_paths(current_path, legacy_path)?;
        if target_is_live(current_path)? {
            return Err(sqlx::Error::Protocol(
                "database already has a live pool".to_string(),
            ));
        }
        if current_path.exists() {
            return Self::open_existing_native(current_path).await;
        }
        if legacy_path.exists() {
            reject_orphaned_sidecars(current_path)?;
            migrate_legacy_source_already_gated(legacy_path, app_data_dir).await?;
            return Self::open_existing_native(current_path).await;
        }
        Self::create_fresh(current_path).await
    }

    // NOTE: So for the first time users they needs to start the application
    // after they can just delete the existing .sqlite file and then copy the existing .db file to
    // the current app dir, So the system detects legacy db and copy it and starts with that data
    // (Newly created .sqlite with the copied content from .db)
    pub async fn new_from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        // Resolve the app's data directory
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");
        let _gate = acquire_app_data_gate(&app_data_dir).await?;
        fs::create_dir_all(&app_data_dir).map_err(sqlx::Error::Io)?;
        Self::new_from_app_handle_already_gated(app_handle, &app_data_dir).await
    }

    pub(crate) async fn new_from_app_handle_already_gated(
        _app_handle: &tauri::AppHandle,
        app_data_dir: &Path,
    ) -> Result<Self> {
        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        Self::new_already_gated(&current_path, &legacy_path, app_data_dir).await
    }

    pub async fn import_legacy_database_to_app_data(
        source_path: &Path,
        app_data_dir: &Path,
    ) -> Result<Self> {
        let _gate = acquire_app_data_gate(app_data_dir).await?;
        fs::create_dir_all(app_data_dir).map_err(sqlx::Error::Io)?;
        Self::import_legacy_database_to_app_data_already_gated(source_path, app_data_dir).await
    }

    pub(crate) async fn import_legacy_database_to_app_data_already_gated(
        source_path: &Path,
        app_data_dir: &Path,
    ) -> Result<Self> {
        ensure_database_startup_allowed(app_data_dir)?;
        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        validate_database_path_shapes(&current_path, &legacy_path)?;
        recover_import_already_gated(app_data_dir).await?;
        validate_owned_database_paths(&current_path, &legacy_path)?;
        if target_is_live(&current_path)? {
            return Err(sqlx::Error::Protocol(
                "legacy import is forbidden while the target pool is live".to_string(),
            ));
        }
        reject_orphaned_sidecars(&current_path)?;
        migrate_legacy_source_already_gated(source_path, app_data_dir).await?;
        Self::open_existing_native(&current_path).await
    }

    /// Check if this is the first launch (sqlite database doesn't exist yet)
    pub async fn is_first_launch(app_handle: &tauri::AppHandle) -> Result<bool> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        let _gate = acquire_app_data_gate(&app_data_dir).await?;
        Self::is_first_launch_already_gated(&app_data_dir).await
    }

    pub(crate) async fn is_first_launch_already_gated(app_data_dir: &Path) -> Result<bool> {
        ensure_database_startup_allowed(app_data_dir)?;
        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        validate_database_path_shapes(&current_path, &legacy_path)?;
        recover_import_already_gated(app_data_dir).await?;
        validate_owned_database_paths(&current_path, &legacy_path)?;
        if target_is_live(&current_path)? {
            return Ok(false);
        }
        reject_orphaned_sidecars(&current_path)?;
        Ok(!current_path.exists())
    }

    /// Import a legacy database from the specified path and initialize
    pub async fn import_legacy_database(
        app_handle: &tauri::AppHandle,
        legacy_db_path: &str,
    ) -> Result<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        Self::import_legacy_database_to_app_data(Path::new(legacy_db_path), &app_data_dir).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn with_transaction<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction<'_, Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = f(&mut tx).await;

        match result {
            Ok(val) => {
                tx.commit().await?;
                Ok(val)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    /// Cleanup database connection and checkpoint WAL
    /// This should be called on application shutdown to ensure:
    /// - All WAL changes are written to the main database file
    /// - The .wal and .shm files are deleted
    /// - Connection pool is gracefully closed
    pub async fn cleanup(&self) -> Result<()> {
        log::info!("Starting database cleanup...");

        // Force checkpoint of WAL to main database file and remove WAL file
        // TRUNCATE mode: checkpoints all pages AND deletes the WAL file
        match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
        {
            Ok(_) => log::info!("WAL checkpoint completed successfully"),
            Err(e) => log::warn!("WAL checkpoint failed (non-fatal): {}", e),
        }

        // Close the connection pool gracefully
        self.pool.close().await;
        unregister_live_target(&self.registered_path, &self.live_token)?;
        log::info!("Database connection pool closed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration::{
        persist_journal, MigrationJournal, MigrationPhase, MIGRATION_FORMAT_VERSION,
        MIGRATION_JOURNAL_FILE, MIGRATION_NAMESPACE_DIR,
    };
    use uuid::Uuid;

    fn write_incomplete_migration(app_data_dir: &Path) {
        let journal = MigrationJournal {
            format_version: MIGRATION_FORMAT_VERSION,
            operation_id: Uuid::new_v4(),
            source_identity: "com.old.meetily".to_string(),
            target_identity: "com.new.meetily".to_string(),
            phase: MigrationPhase::Preparing,
            files: Vec::new(),
        };
        let journal_path = app_data_dir
            .join(MIGRATION_NAMESPACE_DIR)
            .join(journal.operation_id.to_string())
            .join(MIGRATION_JOURNAL_FILE);
        persist_journal(&journal_path, &journal).unwrap();
    }

    fn assert_migration_guard_error(result: Result<DatabaseManager>) {
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("database entry point bypassed migration guard"),
        };
        assert!(error.to_string().contains("data migration is incomplete"));
    }

    #[tokio::test]
    async fn database_entry_points_refuse_incomplete_migration() {
        let directory = tempfile::tempdir().unwrap();
        let app_data_dir = directory.path();
        write_incomplete_migration(app_data_dir);

        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        let first_launch = DatabaseManager::is_first_launch_already_gated(app_data_dir).await;
        assert!(first_launch
            .expect_err("first-launch check bypassed migration guard")
            .to_string()
            .contains("data migration is incomplete"));

        let startup =
            DatabaseManager::new_already_gated(&current_path, &legacy_path, app_data_dir).await;
        assert_migration_guard_error(startup);

        let import = DatabaseManager::import_legacy_database_to_app_data_already_gated(
            app_data_dir.join("legacy.db").as_path(),
            app_data_dir,
        )
        .await;
        assert_migration_guard_error(import);
    }

    #[tokio::test]
    async fn first_launch_rejects_orphaned_current_sidecars() {
        for suffix in ["-wal", "-shm"] {
            let directory = tempfile::tempdir().unwrap();
            let app_data_dir = directory.path();
            let current_path = app_data_dir.join("meeting_minutes.sqlite");
            let sidecar = sqlite_sidecar_path(&current_path, suffix);
            fs::write(&sidecar, b"orphan-sidecar").unwrap();

            let result = DatabaseManager::is_first_launch_already_gated(app_data_dir).await;

            let error = result.expect_err("orphaned sidecar was reported as first launch");
            assert!(
                error
                    .to_string()
                    .contains("orphaned SQLite target sidecars"),
                "unexpected rejection: {error}"
            );
            assert_eq!(fs::read(&sidecar).unwrap(), b"orphan-sidecar");
            assert!(!current_path.exists());
        }
    }
}
