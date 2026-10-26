use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::migrate::{Migration, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Connection, Row, SqlitePool, TypeInfo, ValueRef};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
static RECOVERY_GATES: Lazy<Mutex<HashMap<PathBuf, Weak<AsyncMutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static ACTIVE_TARGETS: Lazy<Mutex<HashMap<PathBuf, Weak<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const IMPORT_MARKER: &str = "meeting_minutes.sqlite.import-v1.json";
const PYTHON_SCHEMA_FIXTURE: &str = include_str!("../../tests/fixtures/legacy_fastapi_schema.sql");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatabaseClass {
    ExactLedgerlessPython,
    AuthenticSqlxPrefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnSignature {
    cid: i64,
    name: String,
    declared_type: String,
    not_null: i64,
    default_value: Option<String>,
    primary_key_position: i64,
    hidden: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForeignKeySignature {
    id: i64,
    sequence: i64,
    target_table: String,
    source_column: String,
    target_column: Option<String>,
    on_update: String,
    on_delete: String,
    match_clause: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableSignature {
    name: String,
    columns: Vec<ColumnSignature>,
    foreign_keys: Vec<ForeignKeySignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchemaSignature {
    objects: Vec<(String, String, String)>,
    tables: Vec<TableSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileIdentity {
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TrioIdentity {
    main: Option<FileIdentity>,
    wal: Option<FileIdentity>,
    shm: Option<FileIdentity>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum ImportStage {
    Prepared,
    BackupComplete,
    OldMainMoved,
    OldWalMoved,
    OldShmMoved,
    OldQuarantined,
    PublishedUnverified,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LegacyImportOperation {
    Snapshot,
    CandidateCreate,
    CandidateMigrate,
    CandidateCopy,
    CandidateCheckpoint,
    CandidateClose,
    CandidateSidecarCleanup,
    CandidateSync,
    CandidateHashVerify,
    CandidateFinalVerify,
    Backup(usize),
    IntentWrite(ImportStage),
    IntentFileSync(ImportStage),
    IntentDirectorySync(ImportStage),
    QuarantineMove(usize),
    QuarantineDirectorySync(usize),
    PublishLink,
    PublishedFileSync,
    PostPublishDirectorySync,
    CandidateUnlink,
    PublishedReopen,
    PublishedVerify,
    MarkerRemove,
}

trait LegacyImportOps: Send + Sync {
    fn before(&self, _operation: LegacyImportOperation) -> Result<(), sqlx::Error> {
        Ok(())
    }

    fn after(&self, _operation: LegacyImportOperation) -> Result<(), sqlx::Error> {
        Ok(())
    }

    fn simulates_process_death(&self) -> bool {
        false
    }
}

struct RealLegacyImportOps;

impl LegacyImportOps for RealLegacyImportOps {}

fn run_sync<T>(
    ops: &dyn LegacyImportOps,
    operation: LegacyImportOperation,
    action: impl FnOnce() -> Result<T, sqlx::Error>,
) -> Result<T, sqlx::Error> {
    ops.before(operation)?;
    let result = action()?;
    ops.after(operation)?;
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ImportIntent {
    version: u8,
    operation_id: String,
    source_path: PathBuf,
    snapshot_path: PathBuf,
    candidate_path: PathBuf,
    live_path: PathBuf,
    backup_paths: [PathBuf; 3],
    quarantine_paths: [PathBuf; 3],
    original_live: TrioIdentity,
    source_before: TrioIdentity,
    snapshot_trio: TrioIdentity,
    app_legacy_path: PathBuf,
    app_legacy_before: TrioIdentity,
    legacy_digest: String,
    candidate_main: FileIdentity,
    stage: ImportStage,
}

struct OwnedImportPaths {
    snapshot: PathBuf,
    candidate: PathBuf,
    backups: [PathBuf; 3],
    quarantines: [PathBuf; 3],
}

const LEGACY_TABLES: &[(&str, &str, &[&str])] = &[
    (
        "meetings",
        "id",
        &["id", "title", "created_at", "updated_at", "folder_path"],
    ),
    (
        "transcripts",
        "id",
        &[
            "id",
            "meeting_id",
            "transcript",
            "timestamp",
            "summary",
            "action_items",
            "key_points",
            "audio_start_time",
            "audio_end_time",
            "duration",
        ],
    ),
    (
        "summary_processes",
        "meeting_id",
        &[
            "meeting_id",
            "status",
            "created_at",
            "updated_at",
            "error",
            "result",
            "start_time",
            "end_time",
            "chunk_count",
            "processing_time",
            "metadata",
        ],
    ),
    (
        "transcript_chunks",
        "meeting_id",
        &[
            "meeting_id",
            "meeting_name",
            "transcript_text",
            "model",
            "model_name",
            "chunk_size",
            "overlap",
            "created_at",
        ],
    ),
    (
        "settings",
        "id",
        &[
            "id",
            "provider",
            "model",
            "whisperModel",
            "groqApiKey",
            "openaiApiKey",
            "anthropicApiKey",
            "ollamaApiKey",
        ],
    ),
    (
        "transcript_settings",
        "id",
        &[
            "id",
            "provider",
            "model",
            "whisperApiKey",
            "deepgramApiKey",
            "elevenLabsApiKey",
            "groqApiKey",
            "openaiApiKey",
        ],
    ),
];

fn protocol_error(message: impl Into<String>) -> sqlx::Error {
    sqlx::Error::Protocol(message.into())
}

fn sidecar_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

fn trio_paths(database_path: &Path) -> [PathBuf; 3] {
    [
        database_path.to_path_buf(),
        sidecar_path(database_path, "-wal"),
        sidecar_path(database_path, "-shm"),
    ]
}

fn validate_owned_source_trio(database_path: &Path) -> Result<(), sqlx::Error> {
    for (index, component) in trio_paths(database_path).into_iter().enumerate() {
        let metadata = match fs::symlink_metadata(&component) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound && index > 0 => continue,
            Err(error) => return Err(sqlx::Error::Io(error)),
        };
        if !metadata.file_type().is_file() {
            return Err(protocol_error(format!(
                "selected SQLite source component is not an owned regular file: {}",
                component.display()
            )));
        }
        #[cfg(unix)]
        if metadata.nlink() != 1 {
            return Err(protocol_error(format!(
                "selected SQLite source component has multiple hard links: {}",
                component.display()
            )));
        }
    }
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn file_identity(path: &Path) -> Result<Option<FileIdentity>, sqlx::Error> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(FileIdentity {
            length: bytes.len() as u64,
            sha256: hash_bytes(&bytes),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(sqlx::Error::Io(error)),
    }
}

fn trio_identity(path: &Path) -> Result<TrioIdentity, sqlx::Error> {
    let paths = trio_paths(path);
    Ok(TrioIdentity {
        main: file_identity(&paths[0])?,
        wal: file_identity(&paths[1])?,
        shm: file_identity(&paths[2])?,
    })
}

fn identity_matches(path: &Path, expected: &FileIdentity) -> Result<bool, sqlx::Error> {
    Ok(file_identity(path)?.as_ref() == Some(expected))
}

fn sync_file(path: &Path) -> Result<(), sqlx::Error> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(sqlx::Error::Io)
}

fn sync_directory(path: &Path) -> Result<(), sqlx::Error> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(sqlx::Error::Io)
}

fn remove_if_exists(path: &Path) -> Result<(), sqlx::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(sqlx::Error::Io(error)),
    }
}

fn remove_trio_if_exists(path: &Path) -> Result<(), sqlx::Error> {
    for component in trio_paths(path) {
        remove_if_exists(&component)?;
    }
    Ok(())
}

fn owned_import_paths(
    app_data_dir: &Path,
    operation_id: &str,
) -> Result<OwnedImportPaths, sqlx::Error> {
    let parsed = Uuid::parse_str(operation_id)
        .map_err(|_| protocol_error("legacy import marker has an invalid operation ID"))?;
    if parsed.to_string() != operation_id {
        return Err(protocol_error(
            "legacy import marker operation ID is not canonical",
        ));
    }
    let suffixes = ["main", "wal", "shm"];
    Ok(OwnedImportPaths {
        snapshot: app_data_dir.join(format!("meeting_minutes.sqlite.snapshot-{operation_id}")),
        candidate: app_data_dir.join(format!("meeting_minutes.sqlite.candidate-{operation_id}")),
        backups: suffixes.map(|suffix| {
            app_data_dir.join(format!(
                "meeting_minutes.sqlite.backup-{operation_id}-{suffix}"
            ))
        }),
        quarantines: suffixes.map(|suffix| {
            app_data_dir.join(format!(
                "meeting_minutes.sqlite.quarantine-{operation_id}-{suffix}"
            ))
        }),
    })
}

fn no_clobber_link(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    fs::hard_link(source, destination).map_err(sqlx::Error::Io)
}

fn no_clobber_copy(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    let mut source = File::open(source).map_err(sqlx::Error::Io)?;
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(sqlx::Error::Io)?;
    io::copy(&mut source, &mut destination).map_err(sqlx::Error::Io)?;
    destination.sync_all().map_err(sqlx::Error::Io)
}

fn no_clobber_move(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    no_clobber_link(source, destination)?;
    sync_file(destination)?;
    fs::remove_file(source).map_err(sqlx::Error::Io)
}

fn marker_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(IMPORT_MARKER)
}

fn write_intent(
    app_data_dir: &Path,
    intent: &ImportIntent,
    ops: &dyn LegacyImportOps,
) -> Result<(), sqlx::Error> {
    let stage = intent.stage;
    ops.before(LegacyImportOperation::IntentWrite(stage))?;
    let marker = marker_path(app_data_dir);
    let temporary = app_data_dir.join(format!("{}.tmp-{}", IMPORT_MARKER, Uuid::new_v4()));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(intent)
            .map_err(|error| protocol_error(format!("cannot serialize import intent: {error}")))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(sqlx::Error::Io)?;
        file.write_all(&bytes).map_err(sqlx::Error::Io)?;
        run_sync(ops, LegacyImportOperation::IntentFileSync(stage), || {
            file.sync_all().map_err(sqlx::Error::Io)
        })?;
        fs::rename(&temporary, &marker).map_err(sqlx::Error::Io)?;
        run_sync(
            ops,
            LegacyImportOperation::IntentDirectorySync(stage),
            || sync_directory(app_data_dir),
        )
    })();
    if result.is_err() && !ops.simulates_process_death() {
        let _ = remove_if_exists(&temporary);
    }
    result?;
    ops.after(LegacyImportOperation::IntentWrite(stage))
}

fn parse_intent(app_data_dir: &Path, bytes: &[u8]) -> Result<ImportIntent, sqlx::Error> {
    let intent: ImportIntent = serde_json::from_slice(bytes)
        .map_err(|error| protocol_error(format!("malformed legacy import marker: {error}")))?;
    if intent.version != 1
        || intent.live_path != app_data_dir.join("meeting_minutes.sqlite")
        || intent.app_legacy_path != app_data_dir.join("meeting_minutes.db")
    {
        return Err(protocol_error(
            "legacy import marker does not match this app-data directory",
        ));
    }
    let expected = owned_import_paths(app_data_dir, &intent.operation_id)?;
    if intent.snapshot_path != expected.snapshot
        || intent.candidate_path != expected.candidate
        || intent.backup_paths != expected.backups
        || intent.quarantine_paths != expected.quarantines
    {
        return Err(protocol_error(
            "legacy import marker contains paths outside its operation namespace",
        ));
    }
    validate_protected_paths_are_not_managed(app_data_dir, &intent)?;
    Ok(intent)
}

fn paths_alias(left: &Path, right: &Path) -> Result<bool, sqlx::Error> {
    if left == right {
        return Ok(true);
    }
    if left.exists() && right.exists() {
        return Ok(fs::canonicalize(left).map_err(sqlx::Error::Io)?
            == fs::canonicalize(right).map_err(sqlx::Error::Io)?);
    }
    Ok(false)
}

fn validate_trio_is_not_managed(
    label: &str,
    protected_path: &Path,
    managed_paths: &[PathBuf],
) -> Result<(), sqlx::Error> {
    for protected in &trio_paths(protected_path) {
        for managed in managed_paths {
            if paths_alias(protected, managed)? {
                return Err(protocol_error(format!(
                    "{label} aliases a managed operation path"
                )));
            }
        }
    }
    Ok(())
}

fn validate_protected_paths_are_not_managed(
    app_data_dir: &Path,
    intent: &ImportIntent,
) -> Result<(), sqlx::Error> {
    if intent.source_before.main.is_none() {
        return Err(protocol_error(
            "legacy import marker source main file is missing",
        ));
    }

    let mut managed_paths = Vec::new();
    managed_paths.extend(trio_paths(&intent.snapshot_path));
    managed_paths.extend(trio_paths(&intent.candidate_path));
    managed_paths.extend(trio_paths(&intent.live_path));
    managed_paths.extend(intent.backup_paths.iter().cloned());
    managed_paths.extend(intent.quarantine_paths.iter().cloned());
    managed_paths.push(marker_path(app_data_dir));

    validate_trio_is_not_managed(
        "legacy import marker source",
        &intent.source_path,
        &managed_paths,
    )?;
    validate_trio_is_not_managed(
        "app-data legacy SQLite trio",
        &intent.app_legacy_path,
        &managed_paths,
    )
}

fn read_intent(app_data_dir: &Path) -> Result<Option<ImportIntent>, sqlx::Error> {
    let marker = marker_path(app_data_dir);
    let bytes = match fs::read(&marker) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(sqlx::Error::Io(error)),
    };
    Ok(Some(parse_intent(app_data_dir, &bytes)?))
}

pub(crate) async fn acquire_app_data_gate(
    app_data_dir: &Path,
) -> Result<OwnedMutexGuard<()>, sqlx::Error> {
    let canonical = canonical_gate_key(app_data_dir)?;
    let gate = {
        let mut registry = RECOVERY_GATES
            .lock()
            .map_err(|_| protocol_error("database recovery gate registry is poisoned"))?;
        registry.retain(|_, value| value.strong_count() > 0);
        if let Some(gate) = registry.get(&canonical).and_then(Weak::upgrade) {
            gate
        } else {
            let gate = Arc::new(AsyncMutex::new(()));
            registry.insert(canonical, Arc::downgrade(&gate));
            gate
        }
    };
    Ok(gate.lock_owned().await)
}

fn canonical_gate_key(path: &Path) -> Result<PathBuf, sqlx::Error> {
    if path.exists() {
        return fs::canonicalize(path).map_err(sqlx::Error::Io);
    }

    let mut missing = Vec::new();
    let mut ancestor = path;
    while !ancestor.exists() {
        let name = ancestor.file_name().ok_or_else(|| {
            protocol_error(format!("cannot resolve app-data path: {}", path.display()))
        })?;
        missing.push(name.to_os_string());
        ancestor = ancestor.parent().ok_or_else(|| {
            protocol_error(format!("cannot resolve app-data path: {}", path.display()))
        })?;
    }

    let mut canonical = fs::canonicalize(ancestor).map_err(sqlx::Error::Io)?;
    for component in missing.iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

pub(crate) fn target_is_live(path: &Path) -> Result<bool, sqlx::Error> {
    let canonical = if path.exists() {
        fs::canonicalize(path).map_err(sqlx::Error::Io)?
    } else {
        path.to_path_buf()
    };
    let mut registry = ACTIVE_TARGETS
        .lock()
        .map_err(|_| protocol_error("active database registry is poisoned"))?;
    registry.retain(|_, value| value.strong_count() > 0);
    Ok(registry.get(&canonical).and_then(Weak::upgrade).is_some())
}

pub(crate) fn register_live_target(path: &Path, token: &Arc<()>) -> Result<PathBuf, sqlx::Error> {
    let canonical = fs::canonicalize(path).map_err(sqlx::Error::Io)?;
    let mut registry = ACTIVE_TARGETS
        .lock()
        .map_err(|_| protocol_error("active database registry is poisoned"))?;
    registry.retain(|_, value| value.strong_count() > 0);
    if registry.get(&canonical).and_then(Weak::upgrade).is_some() {
        return Err(protocol_error(format!(
            "database already has a live pool: {}",
            canonical.display()
        )));
    }
    registry.insert(canonical.clone(), Arc::downgrade(token));
    Ok(canonical)
}

pub(crate) fn unregister_live_target(path: &Path, token: &Arc<()>) -> Result<(), sqlx::Error> {
    let mut registry = ACTIVE_TARGETS
        .lock()
        .map_err(|_| protocol_error("active database registry is poisoned"))?;
    let should_remove = registry
        .get(path)
        .and_then(Weak::upgrade)
        .is_some_and(|registered| Arc::ptr_eq(&registered, token));
    if should_remove {
        registry.remove(path);
    }
    Ok(())
}

fn schema_only_fixture() -> Result<&'static str, sqlx::Error> {
    PYTHON_SCHEMA_FIXTURE
        .split_once("INSERT INTO meetings")
        .map(|(schema, _)| schema)
        .ok_or_else(|| protocol_error("legacy FastAPI fixture has no seed boundary"))
}

fn canonicalize_schema_sql(sql: &str) -> String {
    let mut canonical = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut quote = None;
    while let Some(character) = chars.next() {
        if let Some(delimiter) = quote {
            canonical.push(character);
            let closes = (delimiter == '[' && character == ']')
                || (delimiter != '[' && character == delimiter);
            if closes {
                if delimiter != '[' && chars.peek() == Some(&delimiter) {
                    canonical.push(chars.next().expect("peeked quote must exist"));
                } else {
                    quote = None;
                }
            }
            continue;
        }
        match character {
            '\'' | '"' | '`' | '[' => {
                quote = Some(character);
                canonical.push(character);
            }
            character if character.is_ascii_whitespace() => {}
            character => canonical.extend(character.to_lowercase()),
        }
    }
    canonical
}

fn quote_sqlite_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

async fn table_signature(
    pool: &SqlitePool,
    table_name: &str,
) -> Result<TableSignature, sqlx::Error> {
    let quoted_name = quote_sqlite_identifier(table_name);
    let columns = sqlx::query(&format!("PRAGMA table_xinfo({quoted_name})"))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ColumnSignature {
                cid: row.try_get("cid")?,
                name: row.try_get("name")?,
                declared_type: row.try_get("type")?,
                not_null: row.try_get("notnull")?,
                default_value: row.try_get("dflt_value")?,
                primary_key_position: row.try_get("pk")?,
                hidden: row.try_get("hidden")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let foreign_keys = sqlx::query(&format!("PRAGMA foreign_key_list({quoted_name})"))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ForeignKeySignature {
                id: row.try_get("id")?,
                sequence: row.try_get("seq")?,
                target_table: row.try_get("table")?,
                source_column: row.try_get("from")?,
                target_column: row.try_get("to")?,
                on_update: row.try_get("on_update")?,
                on_delete: row.try_get("on_delete")?,
                match_clause: row.try_get("match")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(TableSignature {
        name: table_name.to_string(),
        columns,
        foreign_keys,
    })
}

async fn schema_signature(pool: &SqlitePool) -> Result<SchemaSignature, sqlx::Error> {
    let raw_objects = sqlx::query_as::<_, (String, String, String)>(
        "SELECT type, name, COALESCE(sql, '')
         FROM sqlite_master
         WHERE name NOT LIKE 'sqlite_%'
         ORDER BY type, name",
    )
    .fetch_all(pool)
    .await?;
    let objects = raw_objects
        .iter()
        .map(|(object_type, name, sql)| {
            (
                object_type.clone(),
                name.clone(),
                canonicalize_schema_sql(sql),
            )
        })
        .collect();
    let mut tables = Vec::new();
    for (_, name, _) in raw_objects
        .iter()
        .filter(|(object_type, _, _)| object_type == "table")
    {
        tables.push(table_signature(pool, name).await?);
    }
    Ok(SchemaSignature { objects, tables })
}

async fn python_schema_signature() -> Result<SchemaSignature, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::raw_sql(schema_only_fixture()?).execute(&pool).await?;
    let signature = schema_signature(&pool).await?;
    pool.close().await;
    Ok(signature)
}

fn prefix_migrator(count: usize) -> Migrator {
    let migrations: Vec<Migration> = MIGRATOR.iter().take(count).cloned().collect();
    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

async fn native_prefix_schema_signature(count: usize) -> Result<SchemaSignature, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    prefix_migrator(count).run(&pool).await?;
    let signature = schema_signature(&pool).await?;
    pool.close().await;
    Ok(signature)
}

async fn classify_pool(pool: &SqlitePool) -> Result<DatabaseClass, sqlx::Error> {
    let ledger_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?
        == 1;
    let actual_schema = schema_signature(pool).await?;
    if !ledger_exists {
        if actual_schema == python_schema_signature().await? {
            return Ok(DatabaseClass::ExactLedgerlessPython);
        }
        return Err(protocol_error(
            "existing ledgerless database is not the exact admitted FastAPI schema",
        ));
    }

    let rows =
        sqlx::query("SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await?;
    if rows.is_empty() || rows.len() > MIGRATOR.iter().count() {
        return Err(protocol_error(
            "SQLx migration ledger is empty or from a future release",
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        let migration = MIGRATOR
            .iter()
            .nth(index)
            .ok_or_else(|| protocol_error("SQLx migration ledger has an unknown version"))?;
        let version: i64 = row.try_get("version")?;
        let success: bool = row.try_get("success")?;
        let checksum: Vec<u8> = row.try_get("checksum")?;
        if version != migration.version
            || !success
            || checksum.as_slice() != migration.checksum.as_ref()
        {
            return Err(protocol_error(
                "SQLx migration ledger is dirty, divergent, gapped, or has a checksum mismatch",
            ));
        }
    }
    if actual_schema != native_prefix_schema_signature(rows.len()).await? {
        return Err(protocol_error(
            "SQLx database schema does not match its terminal migration ledger",
        ));
    }
    Ok(DatabaseClass::AuthenticSqlxPrefix)
}

#[cfg(test)]
async fn classify_path(path: &Path) -> Result<DatabaseClass, sqlx::Error> {
    if !path.is_file() {
        return Err(protocol_error(format!(
            "database does not exist: {}",
            path.display()
        )));
    }
    let options = SqliteConnectOptions::from_str(path.to_string_lossy().as_ref())?
        .read_only(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let result = classify_pool(&pool).await;
    pool.close().await;
    result
}

fn update_framed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn update_cell(hasher: &mut Sha256, row: &SqliteRow, index: usize) -> Result<(), sqlx::Error> {
    let raw = row.try_get_raw(index)?;
    if raw.is_null() {
        update_framed(hasher, b"null");
        update_framed(hasher, &[]);
        return Ok(());
    }
    let type_name = raw.type_info().name().to_ascii_uppercase();
    match type_name.as_str() {
        "INTEGER" => {
            update_framed(hasher, b"integer");
            update_framed(hasher, &row.try_get::<i64, _>(index)?.to_be_bytes());
        }
        "REAL" => {
            update_framed(hasher, b"real");
            update_framed(
                hasher,
                &row.try_get::<f64, _>(index)?.to_bits().to_be_bytes(),
            );
        }
        "TEXT" => {
            update_framed(hasher, b"text");
            update_framed(hasher, row.try_get::<String, _>(index)?.as_bytes());
        }
        "BLOB" => {
            update_framed(hasher, b"blob");
            update_framed(hasher, &row.try_get::<Vec<u8>, _>(index)?);
        }
        other => {
            return Err(protocol_error(format!(
                "unsupported SQLite storage class in legacy digest: {other}"
            )));
        }
    }
    Ok(())
}

pub(crate) async fn canonical_legacy_digest(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let mut hasher = Sha256::new();
    update_framed(&mut hasher, b"meetily-legacy-fastapi-digest-v1");
    for (table, primary_key, columns) in LEGACY_TABLES {
        update_framed(&mut hasher, table.as_bytes());
        let available_columns: HashSet<String> = table_signature(pool, table)
            .await?
            .columns
            .into_iter()
            .map(|column| column.name)
            .collect();
        let projection = columns
            .iter()
            .map(|column| {
                let quoted = quote_sqlite_identifier(column);
                if available_columns.contains(*column) {
                    quoted
                } else {
                    format!("NULL AS {quoted}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {} FROM {} ORDER BY {}",
            projection,
            quote_sqlite_identifier(table),
            quote_sqlite_identifier(primary_key),
        );
        let rows = sqlx::query(&sql).fetch_all(pool).await?;
        update_framed(&mut hasher, &(rows.len() as u64).to_be_bytes());
        for row in rows {
            for (index, column) in row.columns().iter().enumerate() {
                update_framed(&mut hasher, column.name().as_bytes());
                update_cell(&mut hasher, &row, index)?;
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn open_read_only_pool(path: &Path, immutable: bool) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(path.to_string_lossy().as_ref())?
        .read_only(true)
        .immutable(immutable)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
}

async fn snapshot_source_with_ops(
    source: &Path,
    snapshot: &Path,
    staging: &Path,
    expected_source: &TrioIdentity,
    ops: &dyn LegacyImportOps,
) -> Result<(DatabaseClass, String), sqlx::Error> {
    ops.before(LegacyImportOperation::Snapshot)?;
    if trio_paths(snapshot).iter().any(|path| path.exists())
        || trio_paths(staging).iter().any(|path| path.exists())
    {
        return Err(protocol_error("legacy snapshot path already exists"));
    }
    let source_paths = trio_paths(source);
    let staging_paths = trio_paths(staging);
    let expected = [
        expected_source.main.as_ref(),
        expected_source.wal.as_ref(),
        expected_source.shm.as_ref(),
    ];
    let result = async {
        for index in 0..3 {
            match expected[index] {
                Some(identity) => {
                    fs::copy(&source_paths[index], &staging_paths[index])
                        .map_err(sqlx::Error::Io)?;
                    sync_file(&staging_paths[index])?;
                    if !identity_matches(&staging_paths[index], identity)? {
                        return Err(protocol_error(
                            "raw SQLite staging copy differs from the selected source",
                        ));
                    }
                }
                None if source_paths[index].exists() => {
                    return Err(protocol_error(
                        "selected source SQLite trio changed during staging",
                    ));
                }
                None => {}
            }
        }
        if trio_identity(source)? != *expected_source {
            return Err(protocol_error(
                "selected source SQLite trio changed during staging",
            ));
        }

        // SQLite may update a WAL shared-memory file even for a read-only
        // connection. Open only the private byte-for-byte staging trio, never
        // the user-selected source, and materialize the coherent main-only
        // snapshot from there.
        let options = SqliteConnectOptions::from_str(staging.to_string_lossy().as_ref())?
            .read_only(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let class = classify_pool(&pool).await?;
        let digest = canonical_legacy_digest(&pool).await?;
        sqlx::query("VACUUM INTO ?")
            .bind(snapshot.to_string_lossy().to_string())
            .execute(&pool)
            .await?;
        pool.close().await;
        sync_file(snapshot)?;
        if trio_identity(source)? != *expected_source {
            return Err(protocol_error(
                "selected source SQLite trio changed during snapshot",
            ));
        }
        Ok((class, digest))
    }
    .await;

    if ops.simulates_process_death() {
        for path in staging_paths {
            let _ = remove_if_exists(&path);
        }
    } else {
        remove_trio_if_exists(staging)?;
    }
    if result.is_err() && !ops.simulates_process_death() {
        remove_trio_if_exists(snapshot)?;
    }
    let result = result?;
    ops.after(LegacyImportOperation::Snapshot)?;
    Ok(result)
}

pub(crate) async fn classify_path_without_mutation(
    path: &Path,
    app_data_dir: &Path,
) -> Result<DatabaseClass, sqlx::Error> {
    let source_before = trio_identity(path)?;
    if source_before.main.is_none() {
        return Err(protocol_error(format!(
            "database does not exist: {}",
            path.display()
        )));
    }
    let operation_id = Uuid::new_v4().to_string();
    let owned = owned_import_paths(app_data_dir, &operation_id)?;
    let (class, _) = snapshot_source_with_ops(
        path,
        &owned.snapshot,
        &owned.candidate,
        &source_before,
        &RealLegacyImportOps,
    )
    .await?;
    remove_trio_if_exists(&owned.snapshot)?;
    sync_directory(app_data_dir)?;
    if trio_identity(path)? != source_before {
        return Err(protocol_error(
            "database trio changed during non-mutating classification",
        ));
    }
    Ok(class)
}

async fn copy_legacy_projection(
    candidate: &SqlitePool,
    snapshot: &Path,
) -> Result<(), sqlx::Error> {
    let mut connection = candidate.acquire().await?;
    sqlx::query("ATTACH DATABASE ? AS legacy")
        .bind(snapshot.to_string_lossy().to_string())
        .execute(&mut *connection)
        .await?;
    let mut transaction = connection.begin().await?;
    for (table, primary_key, columns) in LEGACY_TABLES {
        let columns = columns.join(", ");
        let sql = format!(
            "INSERT INTO {table} ({columns}) SELECT {columns} FROM legacy.{table} ORDER BY {primary_key}"
        );
        sqlx::query(&sql).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    sqlx::query("DETACH DATABASE legacy")
        .execute(&mut *connection)
        .await?;
    Ok(())
}

async fn finish_candidate(
    candidate_path: &Path,
    ops: &dyn LegacyImportOps,
) -> Result<FileIdentity, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(candidate_path.to_string_lossy().as_ref())?
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    ops.before(LegacyImportOperation::CandidateCheckpoint)?;
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await?;
    ops.after(LegacyImportOperation::CandidateCheckpoint)?;
    ops.before(LegacyImportOperation::CandidateClose)?;
    pool.close().await;
    ops.after(LegacyImportOperation::CandidateClose)?;
    ops.before(LegacyImportOperation::CandidateSidecarCleanup)?;
    for sidecar in [
        sidecar_path(candidate_path, "-wal"),
        sidecar_path(candidate_path, "-shm"),
    ] {
        if sidecar.exists() {
            if fs::metadata(&sidecar).map_err(sqlx::Error::Io)?.len() != 0 {
                return Err(protocol_error(format!(
                    "candidate sidecar is not empty after checkpoint: {}",
                    sidecar.display()
                )));
            }
            remove_if_exists(&sidecar)?;
        }
    }
    ops.after(LegacyImportOperation::CandidateSidecarCleanup)?;
    run_sync(ops, LegacyImportOperation::CandidateSync, || {
        sync_file(candidate_path)
    })?;
    run_sync(ops, LegacyImportOperation::CandidateHashVerify, || {
        file_identity(candidate_path)?
            .ok_or_else(|| protocol_error("candidate disappeared before publication"))
    })
}

async fn verify_published(
    intent: &ImportIntent,
    ops: &dyn LegacyImportOps,
) -> Result<(), sqlx::Error> {
    ops.before(LegacyImportOperation::PublishedReopen)?;
    if !identity_matches(&intent.live_path, &intent.candidate_main)? {
        return Err(protocol_error(
            "published database hash does not match immutable candidate",
        ));
    }
    if sidecar_path(&intent.live_path, "-wal").exists()
        || sidecar_path(&intent.live_path, "-shm").exists()
    {
        return Err(protocol_error(
            "published database has an unverified WAL or SHM sidecar",
        ));
    }
    let pool = open_read_only_pool(&intent.live_path, true).await?;
    ops.after(LegacyImportOperation::PublishedReopen)?;
    ops.before(LegacyImportOperation::PublishedVerify)?;
    if classify_pool(&pool).await? != DatabaseClass::AuthenticSqlxPrefix {
        pool.close().await;
        return Err(protocol_error(
            "published database is not a complete native SQLx database",
        ));
    }
    let digest = canonical_legacy_digest(&pool).await?;
    pool.close().await;
    if digest != intent.legacy_digest {
        return Err(protocol_error(
            "published database legacy projection digest mismatch",
        ));
    }
    ops.after(LegacyImportOperation::PublishedVerify)
}

fn restore_original(intent: &ImportIntent, app_data_dir: &Path) -> Result<(), sqlx::Error> {
    let live_paths = trio_paths(&intent.live_path);
    let expected = [
        intent.original_live.main.as_ref(),
        intent.original_live.wal.as_ref(),
        intent.original_live.shm.as_ref(),
    ];
    for index in 0..3 {
        let live = &live_paths[index];
        match expected[index] {
            Some(expected_identity) => {
                if identity_matches(live, expected_identity)? {
                    #[cfg(unix)]
                    if fs::symlink_metadata(live).map_err(sqlx::Error::Io)?.nlink() != 1 {
                        let source = if identity_matches(
                            &intent.quarantine_paths[index],
                            expected_identity,
                        )? {
                            &intent.quarantine_paths[index]
                        } else if identity_matches(&intent.backup_paths[index], expected_identity)?
                        {
                            &intent.backup_paths[index]
                        } else {
                            return Err(protocol_error(format!(
                                "cannot detach restored SQLite hard link without a recorded copy: {}",
                                live.display()
                            )));
                        };
                        remove_if_exists(live)?;
                        no_clobber_copy(source, live)?;
                        sync_file(live)?;
                    }
                    continue;
                }
                if live.exists() {
                    if index == 0 && identity_matches(live, &intent.candidate_main)? {
                        remove_if_exists(live)?;
                    } else {
                        return Err(protocol_error(format!(
                            "cannot restore original SQLite file without overwriting: {}",
                            live.display()
                        )));
                    }
                }
                let source =
                    if identity_matches(&intent.quarantine_paths[index], expected_identity)? {
                        &intent.quarantine_paths[index]
                    } else if identity_matches(&intent.backup_paths[index], expected_identity)? {
                        &intent.backup_paths[index]
                    } else {
                        return Err(protocol_error(format!(
                            "neither quarantine nor backup matches original SQLite file: {}",
                            live.display()
                        )));
                    };
                no_clobber_copy(source, live)?;
                sync_file(live)?;
            }
            None => {
                if live.exists() {
                    if index == 0 && identity_matches(live, &intent.candidate_main)? {
                        remove_if_exists(live)?;
                    } else {
                        return Err(protocol_error(format!(
                            "unexpected live SQLite file prevents recovery: {}",
                            live.display()
                        )));
                    }
                }
            }
        }
    }
    sync_directory(app_data_dir)?;
    if trio_identity(&intent.live_path)? != intent.original_live {
        return Err(protocol_error(
            "restored live SQLite trio does not match the recorded original",
        ));
    }
    Ok(())
}

fn validate_regular_single_link(path: &Path) -> Result<(), sqlx::Error> {
    let metadata = fs::symlink_metadata(path).map_err(sqlx::Error::Io)?;
    if !metadata.file_type().is_file() {
        return Err(protocol_error(format!(
            "legacy import artifact is not a regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err(protocol_error(format!(
            "unrecorded legacy import artifact has multiple hard links: {}",
            path.display()
        )));
    }
    Ok(())
}

fn same_import_operation(left: &ImportIntent, right: &ImportIntent) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.stage = ImportStage::Prepared;
    right.stage = ImportStage::Prepared;
    left == right
}

fn temporary_marker_paths(app_data_dir: &Path) -> Result<Vec<PathBuf>, sqlx::Error> {
    let prefix = format!("{IMPORT_MARKER}.tmp-");
    let mut paths = Vec::new();
    for entry in fs::read_dir(app_data_dir).map_err(sqlx::Error::Io)? {
        let path = entry.map_err(sqlx::Error::Io)?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(id) = name.strip_prefix(&prefix) else {
            continue;
        };
        let parsed = Uuid::parse_str(id)
            .map_err(|_| protocol_error("temporary import marker has an invalid UUID"))?;
        if parsed.to_string() != id {
            return Err(protocol_error(
                "temporary import marker UUID is not canonical",
            ));
        }
        validate_regular_single_link(&path)?;
        paths.push(path);
    }
    Ok(paths)
}

fn validate_recorded_artifacts(
    app_data_dir: &Path,
    intent: &ImportIntent,
) -> Result<Vec<PathBuf>, sqlx::Error> {
    if trio_identity(&intent.source_path)? != intent.source_before
        || trio_identity(&intent.app_legacy_path)? != intent.app_legacy_before
    {
        return Err(protocol_error(
            "source or app-data legacy SQLite trio changed before recovery",
        ));
    }

    let snapshot = trio_identity(&intent.snapshot_path)?;
    if snapshot != intent.snapshot_trio
        && !(intent.stage == ImportStage::Verified
            && snapshot
                == (TrioIdentity {
                    main: None,
                    wal: None,
                    shm: None,
                }))
    {
        return Err(protocol_error(
            "recorded SQLite snapshot is missing or has changed",
        ));
    }

    let candidate_paths = trio_paths(&intent.candidate_path);
    if candidate_paths[1].exists() || candidate_paths[2].exists() {
        return Err(protocol_error(
            "recorded candidate has an unexpected WAL or SHM sidecar",
        ));
    }
    let candidate_present = candidate_paths[0].exists();
    if candidate_present && !identity_matches(&candidate_paths[0], &intent.candidate_main)? {
        return Err(protocol_error("recorded candidate main file has changed"));
    }
    if !candidate_present
        && !matches!(
            intent.stage,
            ImportStage::PublishedUnverified | ImportStage::Verified
        )
    {
        return Err(protocol_error("recorded candidate main file is missing"));
    }

    let original = [
        intent.original_live.main.as_ref(),
        intent.original_live.wal.as_ref(),
        intent.original_live.shm.as_ref(),
    ];
    for (paths, label) in [
        (&intent.backup_paths, "backup"),
        (&intent.quarantine_paths, "quarantine"),
    ] {
        for index in 0..3 {
            if paths[index].exists()
                && original[index]
                    .map(|identity| identity_matches(&paths[index], identity))
                    .transpose()?
                    != Some(true)
            {
                return Err(protocol_error(format!(
                    "recorded SQLite {label} component has changed: {}",
                    paths[index].display()
                )));
            }
        }
    }

    let live_paths = trio_paths(&intent.live_path);
    let published_candidate = identity_matches(&live_paths[0], &intent.candidate_main)?;
    if published_candidate {
        if live_paths[1].exists() || live_paths[2].exists() {
            return Err(protocol_error(
                "published candidate is mixed with old SQLite sidecars",
            ));
        }
    } else {
        for index in 0..3 {
            if live_paths[index].exists()
                && original[index]
                    .map(|identity| identity_matches(&live_paths[index], identity))
                    .transpose()?
                    != Some(true)
            {
                return Err(protocol_error(format!(
                    "live SQLite component is not recorded by the import marker: {}",
                    live_paths[index].display()
                )));
            }
        }
    }

    let temporary_markers = temporary_marker_paths(app_data_dir)?;
    for temporary in &temporary_markers {
        let bytes = fs::read(temporary).map_err(sqlx::Error::Io)?;
        let temporary_intent = parse_intent(app_data_dir, &bytes)?;
        if !same_import_operation(intent, &temporary_intent) {
            return Err(protocol_error(
                "temporary import marker conflicts with the canonical marker",
            ));
        }
    }

    let owned = owned_import_paths(app_data_dir, &intent.operation_id)?;
    let mut allowed = vec![
        owned.snapshot.clone(),
        sidecar_path(&owned.snapshot, "-wal"),
        sidecar_path(&owned.snapshot, "-shm"),
        owned.candidate.clone(),
        sidecar_path(&owned.candidate, "-wal"),
        sidecar_path(&owned.candidate, "-shm"),
    ];
    allowed.extend(owned.backups);
    allowed.extend(owned.quarantines);
    allowed.extend(validated_archival_paths(
        app_data_dir,
        Some(&intent.operation_id),
    )?);
    let reserved_prefixes = [
        "meeting_minutes.sqlite.snapshot-",
        "meeting_minutes.sqlite.candidate-",
        "meeting_minutes.sqlite.backup-",
        "meeting_minutes.sqlite.quarantine-",
    ];
    for entry in fs::read_dir(app_data_dir).map_err(sqlx::Error::Io)? {
        let path = entry.map_err(sqlx::Error::Io)?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if reserved_prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix))
            && !allowed.contains(&path)
        {
            return Err(protocol_error(format!(
                "unrecorded legacy import artifact blocks recovery: {}",
                path.display()
            )));
        }
    }
    Ok(temporary_markers)
}

fn retain_valid_archival_pairs(intent: &ImportIntent) -> Result<(), sqlx::Error> {
    let live_paths = trio_paths(&intent.live_path);
    let original = [
        intent.original_live.main.as_ref(),
        intent.original_live.wal.as_ref(),
        intent.original_live.shm.as_ref(),
    ];
    for index in 0..3 {
        let backup = &intent.backup_paths[index];
        let quarantine = &intent.quarantine_paths[index];
        let backup_identity = file_identity(backup)?;
        let quarantine_identity = file_identity(quarantine)?;
        match original[index] {
            Some(expected) => match (backup_identity, quarantine_identity) {
                (Some(backup_identity), Some(quarantine_identity))
                    if backup_identity == *expected && quarantine_identity == *expected => {}
                (Some(backup_identity), None)
                    if backup_identity == *expected
                        && identity_matches(&live_paths[index], expected)? =>
                {
                    remove_if_exists(backup)?;
                }
                (None, None) if identity_matches(&live_paths[index], expected)? => {}
                _ => {
                    return Err(protocol_error(
                        "backup and quarantine archive pair is incomplete or does not match the recorded original",
                    ));
                }
            },
            None => {
                if backup_identity.is_some() || quarantine_identity.is_some() {
                    return Err(protocol_error(
                        "archive exists for an originally absent SQLite component",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn remove_recorded_temporaries(
    app_data_dir: &Path,
    intent: &ImportIntent,
    temporary_markers: &[PathBuf],
) -> Result<(), sqlx::Error> {
    remove_trio_if_exists(&intent.candidate_path)?;
    remove_trio_if_exists(&intent.snapshot_path)?;
    for path in temporary_markers {
        remove_if_exists(path)?;
    }
    sync_directory(app_data_dir)
}

fn archived_component(name: &str, prefix: &str) -> Result<Option<(String, usize)>, sqlx::Error> {
    let Some(value) = name.strip_prefix(prefix) else {
        return Ok(None);
    };
    let (operation_id, index) = if let Some(id) = value.strip_suffix("-main") {
        (id, 0)
    } else if let Some(id) = value.strip_suffix("-wal") {
        (id, 1)
    } else if let Some(id) = value.strip_suffix("-shm") {
        (id, 2)
    } else {
        return Err(protocol_error(
            "archived import artifact has an invalid suffix",
        ));
    };
    let parsed = Uuid::parse_str(operation_id)
        .map_err(|_| protocol_error("archived import artifact has an invalid operation ID"))?;
    if parsed.to_string() != operation_id {
        return Err(protocol_error(
            "archived import artifact operation ID is not canonical",
        ));
    }
    Ok(Some((operation_id.to_string(), index)))
}

fn validated_archival_paths(
    app_data_dir: &Path,
    excluded_operation: Option<&str>,
) -> Result<HashSet<PathBuf>, sqlx::Error> {
    let mut pairs: HashMap<(String, usize), [Option<PathBuf>; 2]> = HashMap::new();
    for entry in fs::read_dir(app_data_dir).map_err(sqlx::Error::Io)? {
        let path = entry.map_err(sqlx::Error::Io)?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let component = archived_component(name, "meeting_minutes.sqlite.backup-")?
            .map(|key| (key, 0))
            .or(
                archived_component(name, "meeting_minutes.sqlite.quarantine-")?.map(|key| (key, 1)),
            );
        let Some(((operation_id, index), kind)) = component else {
            continue;
        };
        if excluded_operation == Some(operation_id.as_str()) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(sqlx::Error::Io)?;
        if !metadata.file_type().is_file() {
            return Err(protocol_error(format!(
                "archived import artifact is not a regular file: {}",
                path.display()
            )));
        }
        pairs.entry((operation_id, index)).or_default()[kind] = Some(path);
    }

    let mut validated = HashSet::new();
    for paths in pairs.values() {
        let (Some(backup), Some(quarantine)) = (&paths[0], &paths[1]) else {
            return Err(protocol_error(
                "backup or quarantine exists without its validated archive pair",
            ));
        };
        if file_identity(backup)? != file_identity(quarantine)? {
            return Err(protocol_error(
                "retained backup and quarantine archive hashes differ",
            ));
        }
        validated.insert(backup.clone());
        validated.insert(quarantine.clone());
    }
    Ok(validated)
}

fn canonical_artifact_operation_id(
    name: &str,
    prefix: &str,
) -> Result<Option<String>, sqlx::Error> {
    let Some(mut operation_id) = name.strip_prefix(prefix) else {
        return Ok(None);
    };
    for suffix in ["-wal", "-shm"] {
        if let Some(stripped) = operation_id.strip_suffix(suffix) {
            operation_id = stripped;
            break;
        }
    }
    let parsed = Uuid::parse_str(operation_id)
        .map_err(|_| protocol_error("orphaned import artifact has an invalid operation ID"))?;
    if parsed.to_string() != operation_id {
        return Err(protocol_error(
            "orphaned import artifact operation ID is not canonical",
        ));
    }
    Ok(Some(operation_id.to_string()))
}

fn cleanup_prepared_orphans(app_data_dir: &Path) -> Result<(), sqlx::Error> {
    let temporary_markers = temporary_marker_paths(app_data_dir)?;
    if let Some(first) = temporary_markers.first() {
        let intent = parse_intent(app_data_dir, &fs::read(first).map_err(sqlx::Error::Io)?)?;
        let validated_temporaries = validate_recorded_artifacts(app_data_dir, &intent)?;
        if trio_identity(&intent.live_path)? != intent.original_live {
            return Err(protocol_error(
                "live SQLite trio changed before the first durable import marker",
            ));
        }
        remove_recorded_temporaries(app_data_dir, &intent, &validated_temporaries)?;
        return Ok(());
    }

    validated_archival_paths(app_data_dir, None)?;
    let mut operation_ids = HashSet::new();
    let mut artifacts = Vec::new();
    for entry in fs::read_dir(app_data_dir).map_err(sqlx::Error::Io)? {
        let path = entry.map_err(sqlx::Error::Io)?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let operation_id =
            canonical_artifact_operation_id(name, "meeting_minutes.sqlite.snapshot-")?.or(
                canonical_artifact_operation_id(name, "meeting_minutes.sqlite.candidate-")?,
            );
        if let Some(operation_id) = operation_id {
            validate_regular_single_link(&path)?;
            operation_ids.insert(operation_id);
            artifacts.push(path);
        }
    }
    if operation_ids.len() > 1 {
        return Err(protocol_error(
            "multiple pre-Prepared legacy import operations require manual recovery",
        ));
    }
    if let Some(operation_id) = operation_ids.into_iter().next() {
        let owned = owned_import_paths(app_data_dir, &operation_id)?;
        let allowed = [
            owned.snapshot.clone(),
            sidecar_path(&owned.snapshot, "-wal"),
            sidecar_path(&owned.snapshot, "-shm"),
            owned.candidate.clone(),
            sidecar_path(&owned.candidate, "-wal"),
            sidecar_path(&owned.candidate, "-shm"),
        ];
        if artifacts.iter().any(|path| !allowed.contains(path)) {
            return Err(protocol_error(
                "pre-Prepared import artifact is outside its exact namespace",
            ));
        }
        for path in artifacts {
            remove_if_exists(&path)?;
        }
        sync_directory(app_data_dir)?;
    }
    Ok(())
}

async fn recover_with_ops(
    app_data_dir: &Path,
    ops: &dyn LegacyImportOps,
) -> Result<(), sqlx::Error> {
    let Some(intent) = read_intent(app_data_dir)? else {
        return cleanup_prepared_orphans(app_data_dir);
    };
    let temporary_markers = validate_recorded_artifacts(app_data_dir, &intent)?;

    let published_candidate = identity_matches(&intent.live_path, &intent.candidate_main)?;
    if published_candidate && verify_published(&intent, ops).await.is_ok() {
        retain_valid_archival_pairs(&intent)?;
        remove_recorded_temporaries(app_data_dir, &intent, &temporary_markers)?;
        remove_if_exists(&marker_path(app_data_dir))?;
        sync_directory(app_data_dir)?;
        return Ok(());
    }

    restore_original(&intent, app_data_dir)?;
    retain_valid_archival_pairs(&intent)?;
    remove_recorded_temporaries(app_data_dir, &intent, &temporary_markers)?;
    remove_if_exists(&marker_path(app_data_dir))?;
    sync_directory(app_data_dir)
}

pub(crate) async fn recover_import_already_gated(app_data_dir: &Path) -> Result<(), sqlx::Error> {
    recover_with_ops(app_data_dir, &RealLegacyImportOps).await
}

async fn backup_original_trio(
    intent: &ImportIntent,
    app_data_dir: &Path,
    ops: &dyn LegacyImportOps,
) -> Result<(), sqlx::Error> {
    let live_paths = trio_paths(&intent.live_path);
    let expected = [
        intent.original_live.main.as_ref(),
        intent.original_live.wal.as_ref(),
        intent.original_live.shm.as_ref(),
    ];
    for index in 0..3 {
        if let Some(expected_identity) = expected[index] {
            if !identity_matches(&live_paths[index], expected_identity)? {
                return Err(protocol_error("live SQLite trio changed before backup"));
            }
            run_sync(ops, LegacyImportOperation::Backup(index), || {
                fs::copy(&live_paths[index], &intent.backup_paths[index])
                    .map_err(sqlx::Error::Io)?;
                sync_file(&intent.backup_paths[index])
            })?;
            if !identity_matches(&intent.backup_paths[index], expected_identity)? {
                return Err(protocol_error("durable SQLite backup hash mismatch"));
            }
        }
    }
    sync_directory(app_data_dir)
}

fn move_original_component(
    intent: &ImportIntent,
    index: usize,
    app_data_dir: &Path,
    ops: &dyn LegacyImportOps,
) -> Result<(), sqlx::Error> {
    let expected = [
        intent.original_live.main.as_ref(),
        intent.original_live.wal.as_ref(),
        intent.original_live.shm.as_ref(),
    ][index];
    let live = &trio_paths(&intent.live_path)[index];
    if let Some(expected_identity) = expected {
        if !identity_matches(live, expected_identity)? {
            return Err(protocol_error(
                "live SQLite component changed before quarantine",
            ));
        }
        run_sync(ops, LegacyImportOperation::QuarantineMove(index), || {
            no_clobber_move(live, &intent.quarantine_paths[index])
        })?;
        if !identity_matches(&intent.quarantine_paths[index], expected_identity)? {
            return Err(protocol_error("quarantined SQLite component hash mismatch"));
        }
    } else if live.exists() {
        return Err(protocol_error(
            "unexpected live SQLite component before quarantine",
        ));
    }
    run_sync(
        ops,
        LegacyImportOperation::QuarantineDirectorySync(index),
        || sync_directory(app_data_dir),
    )
}

async fn publish_candidate_with_ops(
    app_data_dir: &Path,
    intent: &mut ImportIntent,
    ops: &dyn LegacyImportOps,
) -> Result<(), sqlx::Error> {
    write_intent(app_data_dir, intent, ops)?;
    backup_original_trio(intent, app_data_dir, ops).await?;
    intent.stage = ImportStage::BackupComplete;
    write_intent(app_data_dir, intent, ops)?;

    move_original_component(intent, 0, app_data_dir, ops)?;
    intent.stage = ImportStage::OldMainMoved;
    write_intent(app_data_dir, intent, ops)?;
    move_original_component(intent, 1, app_data_dir, ops)?;
    intent.stage = ImportStage::OldWalMoved;
    write_intent(app_data_dir, intent, ops)?;
    move_original_component(intent, 2, app_data_dir, ops)?;
    intent.stage = ImportStage::OldShmMoved;
    write_intent(app_data_dir, intent, ops)?;
    intent.stage = ImportStage::OldQuarantined;
    write_intent(app_data_dir, intent, ops)?;

    run_sync(ops, LegacyImportOperation::PublishLink, || {
        no_clobber_link(&intent.candidate_path, &intent.live_path)
    })?;
    if !identity_matches(&intent.live_path, &intent.candidate_main)? {
        return Err(protocol_error(
            "candidate changed during no-clobber publication",
        ));
    }
    run_sync(ops, LegacyImportOperation::PublishedFileSync, || {
        sync_file(&intent.live_path)
    })?;
    run_sync(ops, LegacyImportOperation::PostPublishDirectorySync, || {
        sync_directory(app_data_dir)
    })?;
    intent.stage = ImportStage::PublishedUnverified;
    write_intent(app_data_dir, intent, ops)?;
    run_sync(ops, LegacyImportOperation::CandidateUnlink, || {
        remove_if_exists(&intent.candidate_path)
    })?;
    sync_directory(app_data_dir)?;

    verify_published(intent, ops).await?;
    if trio_identity(&intent.source_path)? != intent.source_before
        || trio_identity(&intent.app_legacy_path)? != intent.app_legacy_before
    {
        return Err(protocol_error(
            "source or app-data legacy SQLite trio changed before import completion",
        ));
    }
    intent.stage = ImportStage::Verified;
    write_intent(app_data_dir, intent, ops)?;
    let temporary_markers = validate_recorded_artifacts(app_data_dir, intent)?;
    retain_valid_archival_pairs(intent)?;
    remove_recorded_temporaries(app_data_dir, intent, &temporary_markers)?;
    run_sync(ops, LegacyImportOperation::MarkerRemove, || {
        remove_if_exists(&marker_path(app_data_dir))
    })?;
    sync_directory(app_data_dir)
}

async fn build_python_candidate(
    snapshot_path: &Path,
    candidate_path: &Path,
    source_digest: &str,
    ops: &dyn LegacyImportOps,
) -> Result<(String, FileIdentity), sqlx::Error> {
    let snapshot_pool = open_read_only_pool(snapshot_path, false).await?;
    if classify_pool(&snapshot_pool).await? != DatabaseClass::ExactLedgerlessPython {
        snapshot_pool.close().await;
        return Err(protocol_error(
            "coherent snapshot is not the exact FastAPI schema",
        ));
    }
    let snapshot_digest = canonical_legacy_digest(&snapshot_pool).await?;
    snapshot_pool.close().await;
    if snapshot_digest != source_digest {
        return Err(protocol_error(
            "coherent snapshot digest differs from the live source",
        ));
    }

    ops.before(LegacyImportOperation::CandidateCreate)?;
    let options = SqliteConnectOptions::from_str(candidate_path.to_string_lossy().as_ref())?
        .create_if_missing(true)
        .foreign_keys(true);
    let candidate_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    ops.after(LegacyImportOperation::CandidateCreate)?;
    ops.before(LegacyImportOperation::CandidateMigrate)?;
    MIGRATOR.run(&candidate_pool).await?;
    ops.after(LegacyImportOperation::CandidateMigrate)?;
    ops.before(LegacyImportOperation::CandidateCopy)?;
    copy_legacy_projection(&candidate_pool, snapshot_path).await?;
    ops.after(LegacyImportOperation::CandidateCopy)?;
    let imported_digest = canonical_legacy_digest(&candidate_pool).await?;
    if snapshot_digest != imported_digest {
        candidate_pool.close().await;
        return Err(protocol_error(
            "legacy projection digest changed during import",
        ));
    }
    if classify_pool(&candidate_pool).await? != DatabaseClass::AuthenticSqlxPrefix {
        candidate_pool.close().await;
        return Err(protocol_error(
            "candidate schema or migration ledger is incomplete",
        ));
    }
    candidate_pool.close().await;
    let identity = finish_candidate(candidate_path, ops).await?;
    ops.before(LegacyImportOperation::CandidateFinalVerify)?;
    let verification_pool = open_read_only_pool(candidate_path, true).await?;
    let verified_digest = canonical_legacy_digest(&verification_pool).await?;
    let verified_class = classify_pool(&verification_pool).await?;
    verification_pool.close().await;
    if verified_digest != snapshot_digest || verified_class != DatabaseClass::AuthenticSqlxPrefix {
        return Err(protocol_error(
            "final immutable candidate verification failed",
        ));
    }
    if !identity_matches(candidate_path, &identity)? {
        return Err(protocol_error("candidate changed after final hash"));
    }
    ops.after(LegacyImportOperation::CandidateFinalVerify)?;
    Ok((snapshot_digest, identity))
}

async fn build_native_candidate(
    snapshot_path: &Path,
    candidate_path: &Path,
    source_digest: &str,
    ops: &dyn LegacyImportOps,
) -> Result<(String, FileIdentity), sqlx::Error> {
    ops.before(LegacyImportOperation::CandidateCreate)?;
    fs::copy(snapshot_path, candidate_path).map_err(sqlx::Error::Io)?;
    ops.after(LegacyImportOperation::CandidateCreate)?;

    let options = SqliteConnectOptions::from_str(candidate_path.to_string_lossy().as_ref())?
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    ops.before(LegacyImportOperation::CandidateMigrate)?;
    MIGRATOR.run(&pool).await?;
    ops.after(LegacyImportOperation::CandidateMigrate)?;
    if classify_pool(&pool).await? != DatabaseClass::AuthenticSqlxPrefix {
        pool.close().await;
        return Err(protocol_error(
            "migrated native candidate is not an authentic SQLx database",
        ));
    }
    let imported_digest = canonical_legacy_digest(&pool).await?;
    pool.close().await;
    if imported_digest != source_digest {
        return Err(protocol_error(
            "native migration changed the existing data projection",
        ));
    }

    let identity = finish_candidate(candidate_path, ops).await?;
    ops.before(LegacyImportOperation::CandidateFinalVerify)?;
    let verification_pool = open_read_only_pool(candidate_path, true).await?;
    let verified_class = classify_pool(&verification_pool).await?;
    let verified_digest = canonical_legacy_digest(&verification_pool).await?;
    verification_pool.close().await;
    if verified_class != DatabaseClass::AuthenticSqlxPrefix || verified_digest != source_digest {
        return Err(protocol_error(
            "final immutable native candidate verification failed",
        ));
    }
    if !identity_matches(candidate_path, &identity)? {
        return Err(protocol_error("native candidate changed after final hash"));
    }
    ops.after(LegacyImportOperation::CandidateFinalVerify)?;
    Ok((source_digest.to_string(), identity))
}

async fn normalize_with_ops(
    source_path: &Path,
    app_data_dir: &Path,
    ops: &dyn LegacyImportOps,
) -> Result<PathBuf, sqlx::Error> {
    recover_with_ops(app_data_dir, ops).await?;
    let current_path = app_data_dir.join("meeting_minutes.sqlite");
    if target_is_live(&current_path)? {
        return Err(protocol_error(
            "legacy import is forbidden while the target pool is live",
        ));
    }
    let operation_id = Uuid::new_v4().to_string();
    let owned_paths = owned_import_paths(app_data_dir, &operation_id)?;
    let snapshot_path = owned_paths.snapshot;
    let candidate_path = owned_paths.candidate;
    let app_legacy_path = app_data_dir.join("meeting_minutes.db");
    validate_owned_source_trio(source_path)?;
    let source_before = trio_identity(source_path)?;
    let app_legacy_before = trio_identity(&app_legacy_path)?;
    let (source_class, source_digest) = snapshot_source_with_ops(
        source_path,
        &snapshot_path,
        &candidate_path,
        &source_before,
        ops,
    )
    .await?;
    if trio_identity(source_path)? != source_before
        || trio_identity(&app_legacy_path)? != app_legacy_before
    {
        remove_if_exists(&snapshot_path)?;
        return Err(protocol_error(
            "source or app-data legacy SQLite trio changed during snapshot",
        ));
    }

    let build_result = match source_class {
        DatabaseClass::ExactLedgerlessPython => {
            build_python_candidate(&snapshot_path, &candidate_path, &source_digest, ops).await
        }
        DatabaseClass::AuthenticSqlxPrefix => {
            build_native_candidate(&snapshot_path, &candidate_path, &source_digest, ops).await
        }
    };
    let (legacy_digest, candidate_main) = match build_result {
        Ok(result) => result,
        Err(error) => {
            if ops.simulates_process_death() {
                return Err(error);
            }
            remove_trio_if_exists(&candidate_path)?;
            remove_trio_if_exists(&snapshot_path)?;
            return Err(error);
        }
    };

    let mut intent = ImportIntent {
        version: 1,
        operation_id,
        source_path: source_path.to_path_buf(),
        snapshot_path: snapshot_path.clone(),
        candidate_path: candidate_path.clone(),
        live_path: current_path.clone(),
        backup_paths: owned_paths.backups,
        quarantine_paths: owned_paths.quarantines,
        original_live: trio_identity(&current_path)?,
        source_before,
        snapshot_trio: trio_identity(&snapshot_path)?,
        app_legacy_path: app_legacy_path.clone(),
        app_legacy_before,
        legacy_digest,
        candidate_main,
        stage: ImportStage::Prepared,
    };

    if let Err(error) = validate_protected_paths_are_not_managed(app_data_dir, &intent) {
        remove_trio_if_exists(&candidate_path)?;
        remove_trio_if_exists(&snapshot_path)?;
        return Err(error);
    }

    if let Err(error) = publish_candidate_with_ops(app_data_dir, &mut intent, ops).await {
        if ops.simulates_process_death() {
            return Err(error);
        }
        if marker_path(app_data_dir).exists() {
            recover_import_already_gated(app_data_dir).await?;
        } else {
            remove_if_exists(&candidate_path)?;
            remove_if_exists(&snapshot_path)?;
        }
        return Err(error);
    }
    remove_trio_if_exists(&snapshot_path)?;
    if trio_identity(source_path)? != intent.source_before
        || trio_identity(&app_legacy_path)? != intent.app_legacy_before
    {
        return Err(protocol_error(
            "source or app-data legacy SQLite trio changed during import",
        ));
    }
    Ok(current_path)
}

pub(crate) async fn migrate_legacy_source_already_gated(
    source_path: &Path,
    app_data_dir: &Path,
) -> Result<PathBuf, sqlx::Error> {
    normalize_with_ops(source_path, app_data_dir, &RealLegacyImportOps).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct FaultInjectingLegacyImportOps {
        operation: LegacyImportOperation,
        fired: AtomicBool,
    }

    impl FaultInjectingLegacyImportOps {
        fn new(operation: LegacyImportOperation) -> Self {
            Self {
                operation,
                fired: AtomicBool::new(false),
            }
        }
    }

    impl LegacyImportOps for FaultInjectingLegacyImportOps {
        fn before(&self, operation: LegacyImportOperation) -> Result<(), sqlx::Error> {
            if operation == self.operation && !self.fired.swap(true, Ordering::SeqCst) {
                return Err(protocol_error(format!(
                    "injected operation failure: {operation:?}"
                )));
            }
            Ok(())
        }
    }

    struct CrashAfterBoundaryOps {
        operation: LegacyImportOperation,
        point: CrashPoint,
        fired: AtomicBool,
    }

    #[derive(Clone, Copy, Debug)]
    enum CrashPoint {
        Before,
        After,
    }

    impl CrashAfterBoundaryOps {
        fn new(operation: LegacyImportOperation) -> Self {
            Self::at(operation, CrashPoint::After)
        }

        fn at(operation: LegacyImportOperation, point: CrashPoint) -> Self {
            Self {
                operation,
                point,
                fired: AtomicBool::new(false),
            }
        }
    }

    impl LegacyImportOps for CrashAfterBoundaryOps {
        fn before(&self, operation: LegacyImportOperation) -> Result<(), sqlx::Error> {
            if matches!(self.point, CrashPoint::Before)
                && operation == self.operation
                && !self.fired.swap(true, Ordering::SeqCst)
            {
                return Err(protocol_error(format!(
                    "simulated process death before: {operation:?}"
                )));
            }
            Ok(())
        }

        fn after(&self, operation: LegacyImportOperation) -> Result<(), sqlx::Error> {
            if matches!(self.point, CrashPoint::After)
                && operation == self.operation
                && !self.fired.swap(true, Ordering::SeqCst)
            {
                return Err(protocol_error(format!(
                    "simulated process death after: {operation:?}"
                )));
            }
            Ok(())
        }

        fn simulates_process_death(&self) -> bool {
            true
        }
    }

    struct ReplaceSnapshotOps {
        app_data_dir: PathBuf,
        replacement: PathBuf,
        fired: AtomicBool,
    }

    impl LegacyImportOps for ReplaceSnapshotOps {
        fn after(&self, operation: LegacyImportOperation) -> Result<(), sqlx::Error> {
            if operation == LegacyImportOperation::Snapshot
                && !self.fired.swap(true, Ordering::SeqCst)
            {
                let snapshot = fs::read_dir(&self.app_data_dir)
                    .map_err(sqlx::Error::Io)?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.contains(".snapshot-"))
                    })
                    .ok_or_else(|| protocol_error("snapshot was not created"))?;
                fs::copy(&self.replacement, snapshot).map_err(sqlx::Error::Io)?;
            }
            Ok(())
        }
    }

    struct ImportCase {
        _directory: tempfile::TempDir,
        source_path: PathBuf,
        app_data_dir: PathBuf,
        current_path: PathBuf,
        source_before: TrioIdentity,
        app_legacy_before: TrioIdentity,
        current_before: TrioIdentity,
    }

    async fn import_case() -> ImportCase {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("selected-source.db");
        let app_data_dir = directory.path().join("app-data");
        fs::create_dir(&app_data_dir).unwrap();
        let source_options = SqliteConnectOptions::from_str(source_path.to_string_lossy().as_ref())
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
        let source_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(source_options)
            .await
            .unwrap();
        sqlx::raw_sql(PYTHON_SCHEMA_FIXTURE)
            .execute(&source_pool)
            .await
            .unwrap();
        source_pool.close().await;

        let app_legacy_path = app_data_dir.join("meeting_minutes.db");
        fs::write(&app_legacy_path, b"legacy-main").unwrap();
        fs::write(sidecar_path(&app_legacy_path, "-wal"), b"legacy-wal").unwrap();
        fs::write(sidecar_path(&app_legacy_path, "-shm"), b"legacy-shm").unwrap();

        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let current_options =
            SqliteConnectOptions::from_str(current_path.to_string_lossy().as_ref())
                .unwrap()
                .create_if_missing(true)
                .foreign_keys(true);
        let current_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(current_options)
            .await
            .unwrap();
        MIGRATOR.run(&current_pool).await.unwrap();
        current_pool.close().await;
        fs::write(sidecar_path(&current_path, "-wal"), b"old-wal").unwrap();
        fs::write(sidecar_path(&current_path, "-shm"), b"old-shm").unwrap();

        ImportCase {
            source_before: trio_identity(&source_path).unwrap(),
            app_legacy_before: trio_identity(&app_legacy_path).unwrap(),
            current_before: trio_identity(&current_path).unwrap(),
            _directory: directory,
            source_path,
            app_data_dir,
            current_path,
        }
    }

    fn fault_operations() -> Vec<LegacyImportOperation> {
        let stages = [
            ImportStage::Prepared,
            ImportStage::BackupComplete,
            ImportStage::OldMainMoved,
            ImportStage::OldWalMoved,
            ImportStage::OldShmMoved,
            ImportStage::OldQuarantined,
            ImportStage::PublishedUnverified,
            ImportStage::Verified,
        ];
        let mut operations = vec![
            LegacyImportOperation::Snapshot,
            LegacyImportOperation::CandidateCreate,
            LegacyImportOperation::CandidateMigrate,
            LegacyImportOperation::CandidateCopy,
            LegacyImportOperation::CandidateCheckpoint,
            LegacyImportOperation::CandidateClose,
            LegacyImportOperation::CandidateSidecarCleanup,
            LegacyImportOperation::CandidateSync,
            LegacyImportOperation::CandidateHashVerify,
            LegacyImportOperation::CandidateFinalVerify,
            LegacyImportOperation::Backup(0),
            LegacyImportOperation::Backup(1),
            LegacyImportOperation::Backup(2),
            LegacyImportOperation::QuarantineMove(0),
            LegacyImportOperation::QuarantineDirectorySync(0),
            LegacyImportOperation::QuarantineMove(1),
            LegacyImportOperation::QuarantineDirectorySync(1),
            LegacyImportOperation::QuarantineMove(2),
            LegacyImportOperation::QuarantineDirectorySync(2),
            LegacyImportOperation::PublishLink,
            LegacyImportOperation::PublishedFileSync,
            LegacyImportOperation::PostPublishDirectorySync,
            LegacyImportOperation::CandidateUnlink,
            LegacyImportOperation::PublishedReopen,
            LegacyImportOperation::PublishedVerify,
            LegacyImportOperation::MarkerRemove,
        ];
        for stage in stages {
            operations.push(LegacyImportOperation::IntentWrite(stage));
            operations.push(LegacyImportOperation::IntentFileSync(stage));
            operations.push(LegacyImportOperation::IntentDirectorySync(stage));
        }
        operations
    }

    async fn assert_recovered(case: &ImportCase) {
        recover_import_already_gated(&case.app_data_dir)
            .await
            .unwrap();
        assert_eq!(
            trio_identity(&case.source_path).unwrap(),
            case.source_before
        );
        assert_eq!(
            trio_identity(&case.app_data_dir.join("meeting_minutes.db")).unwrap(),
            case.app_legacy_before
        );
        let current = trio_identity(&case.current_path).unwrap();
        #[cfg(unix)]
        for component in trio_paths(&case.current_path) {
            if component.exists() {
                assert_eq!(
                    fs::symlink_metadata(&component).unwrap().nlink(),
                    1,
                    "recovery retained a hard-linked live component: {}",
                    component.display()
                );
            }
        }
        if current != case.current_before {
            assert_eq!(
                classify_path(&case.current_path).await.unwrap(),
                DatabaseClass::AuthenticSqlxPrefix
            );
            assert!(!sidecar_path(&case.current_path, "-wal").exists());
            assert!(!sidecar_path(&case.current_path, "-shm").exists());
        }
        assert!(!marker_path(&case.app_data_dir).exists());
        let leaked_operation_artifact = fs::read_dir(&case.app_data_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .any(|name| {
                name.contains(".candidate-")
                    || name.contains(".snapshot-")
                    || name.contains(".tmp-")
            });
        assert!(
            !leaked_operation_artifact,
            "recovery leaked an import artifact"
        );
    }

    #[tokio::test]
    async fn import_rejects_snapshot_whose_rows_differ_from_live_source() {
        let case = import_case().await;
        let replacement = case._directory.path().join("different-source.db");
        fs::copy(&case.source_path, &replacement).unwrap();
        let replacement_options =
            SqliteConnectOptions::from_str(replacement.to_string_lossy().as_ref()).unwrap();
        let replacement_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(replacement_options)
            .await
            .unwrap();
        sqlx::query("UPDATE meetings SET title = 'different' WHERE id = 'meeting-a'")
            .execute(&replacement_pool)
            .await
            .unwrap();
        replacement_pool.close().await;

        let ops = ReplaceSnapshotOps {
            app_data_dir: case.app_data_dir.clone(),
            replacement,
            fired: AtomicBool::new(false),
        };
        let result = normalize_with_ops(&case.source_path, &case.app_data_dir, &ops).await;

        assert!(ops.fired.load(Ordering::SeqCst));
        assert!(result.is_err(), "a divergent snapshot was imported");
        assert_eq!(
            trio_identity(&case.source_path).unwrap(),
            case.source_before
        );
    }

    #[tokio::test]
    async fn fault_matrix_restores_target_at_every_operation_error() {
        for operation in fault_operations() {
            let case = import_case().await;
            let ops = FaultInjectingLegacyImportOps::new(operation);
            let result = normalize_with_ops(&case.source_path, &case.app_data_dir, &ops).await;
            assert!(
                ops.fired.load(Ordering::SeqCst),
                "operation not reached: {operation:?}"
            );
            assert!(result.is_err(), "operation did not fail: {operation:?}");
            assert_recovered(&case).await;

            let leaked_temporary = fs::read_dir(&case.app_data_dir)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .any(|name| {
                    name.contains(".candidate-")
                        || name.contains(".snapshot-")
                        || name.contains(".tmp-")
                });
            assert!(
                !leaked_temporary,
                "temporary file leaked after {operation:?}"
            );
        }
    }

    #[tokio::test]
    async fn restart_recovers_every_durable_import_boundary() {
        for operation in fault_operations() {
            for point in [CrashPoint::Before, CrashPoint::After] {
                let case = import_case().await;
                let ops = CrashAfterBoundaryOps::at(operation, point);
                let result = normalize_with_ops(&case.source_path, &case.app_data_dir, &ops).await;
                assert!(
                    ops.fired.load(Ordering::SeqCst),
                    "boundary not reached: {point:?} {operation:?}"
                );
                assert!(
                    result.is_err(),
                    "boundary did not stop: {point:?} {operation:?}"
                );

                if let Some(intent) = read_intent(&case.app_data_dir).unwrap() {
                    let candidate_or_live_matches =
                        identity_matches(&intent.candidate_path, &intent.candidate_main).unwrap()
                            || identity_matches(&intent.live_path, &intent.candidate_main).unwrap();
                    assert!(
                        candidate_or_live_matches,
                        "candidate hash changed at {point:?} {operation:?}"
                    );
                }
                assert_recovered(&case).await;
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn public_startup_recovers_a_published_hard_link_before_ownership_validation() {
        use crate::database::manager::DatabaseManager;

        let case = import_case().await;
        let ops = CrashAfterBoundaryOps::at(LegacyImportOperation::PublishLink, CrashPoint::After);
        assert!(
            normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                .await
                .is_err()
        );
        assert!(ops.fired.load(Ordering::SeqCst));
        assert!(marker_path(&case.app_data_dir).exists());
        assert!(fs::symlink_metadata(&case.current_path).unwrap().nlink() > 1);

        let manager = DatabaseManager::new(
            case.current_path.to_str().unwrap(),
            case.app_data_dir
                .join("meeting_minutes.db")
                .to_str()
                .unwrap(),
        )
        .await
        .expect("public startup did not recover the valid published hard-link state");
        manager.cleanup().await.unwrap();

        assert_recovered(&case).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn first_launch_recovers_a_published_hard_link_before_ownership_validation() {
        use crate::database::manager::DatabaseManager;

        let case = import_case().await;
        let ops = CrashAfterBoundaryOps::at(LegacyImportOperation::PublishLink, CrashPoint::After);
        assert!(
            normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                .await
                .is_err()
        );
        assert!(ops.fired.load(Ordering::SeqCst));
        assert!(fs::symlink_metadata(&case.current_path).unwrap().nlink() > 1);

        let first_launch = DatabaseManager::is_first_launch_already_gated(&case.app_data_dir)
            .await
            .expect("first-launch rejected a valid published hard-link recovery state");

        assert!(!first_launch);
        assert_recovered(&case).await;
    }

    #[tokio::test]
    async fn recovery_refuses_to_clear_marker_after_protected_input_changes() {
        for mutate_source in [true, false] {
            let case = import_case().await;
            let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::IntentWrite(
                ImportStage::Prepared,
            ));
            assert!(
                normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                    .await
                    .is_err()
            );
            assert!(marker_path(&case.app_data_dir).exists());

            let protected_path = if mutate_source {
                case.source_path.clone()
            } else {
                case.app_data_dir.join("meeting_minutes.db")
            };
            fs::write(&protected_path, b"changed-after-crash").unwrap();

            let recovery = recover_import_already_gated(&case.app_data_dir).await;
            assert!(
                recovery.is_err(),
                "recovery accepted a changed {} trio",
                if mutate_source {
                    "source"
                } else {
                    "app legacy"
                }
            );
            assert!(
                marker_path(&case.app_data_dir).exists(),
                "recovery cleared the marker after protected input changed"
            );
        }
    }

    #[tokio::test]
    async fn recovery_rejects_every_corrupted_recorded_copy_without_cleanup() {
        for snapshot in [true, false] {
            let case = import_case().await;
            let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::IntentWrite(
                ImportStage::Prepared,
            ));
            assert!(
                normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                    .await
                    .is_err()
            );
            let intent = read_intent(&case.app_data_dir).unwrap().unwrap();
            let corrupted = if snapshot {
                intent.snapshot_path.clone()
            } else {
                intent.candidate_path.clone()
            };
            fs::write(&corrupted, b"corrupted-recorded-copy").unwrap();

            assert!(recover_import_already_gated(&case.app_data_dir)
                .await
                .is_err());
            assert_eq!(fs::read(&corrupted).unwrap(), b"corrupted-recorded-copy");
            assert!(marker_path(&case.app_data_dir).exists());
        }

        for index in 0..3 {
            let case = import_case().await;
            let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::Backup(index));
            assert!(
                normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                    .await
                    .is_err()
            );
            let intent = read_intent(&case.app_data_dir).unwrap().unwrap();
            fs::write(&intent.backup_paths[index], b"corrupted-backup").unwrap();

            assert!(recover_import_already_gated(&case.app_data_dir)
                .await
                .is_err());
            assert_eq!(
                fs::read(&intent.backup_paths[index]).unwrap(),
                b"corrupted-backup"
            );
            assert!(marker_path(&case.app_data_dir).exists());
        }

        for index in 0..3 {
            let case = import_case().await;
            let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::QuarantineMove(index));
            assert!(
                normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                    .await
                    .is_err()
            );
            let intent = read_intent(&case.app_data_dir).unwrap().unwrap();
            fs::write(&intent.quarantine_paths[index], b"corrupted-quarantine").unwrap();

            assert!(recover_import_already_gated(&case.app_data_dir)
                .await
                .is_err());
            assert_eq!(
                fs::read(&intent.quarantine_paths[index]).unwrap(),
                b"corrupted-quarantine"
            );
            assert!(marker_path(&case.app_data_dir).exists());
        }

        let case = import_case().await;
        let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::IntentFileSync(
            ImportStage::Prepared,
        ));
        assert!(
            normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                .await
                .is_err()
        );
        assert!(!marker_path(&case.app_data_dir).exists());
        let temporary = temporary_marker_paths(&case.app_data_dir).unwrap();
        assert_eq!(temporary.len(), 1);
        fs::write(&temporary[0], b"corrupted-temporary-marker").unwrap();

        assert!(recover_import_already_gated(&case.app_data_dir)
            .await
            .is_err());
        assert_eq!(
            fs::read(&temporary[0]).unwrap(),
            b"corrupted-temporary-marker"
        );
        assert_eq!(
            trio_identity(&case.source_path).unwrap(),
            case.source_before
        );
        assert_eq!(
            trio_identity(&case.app_data_dir.join("meeting_minutes.db")).unwrap(),
            case.app_legacy_before
        );
    }

    #[tokio::test]
    async fn recovery_rejects_forged_marker_paths_before_touching_siblings() {
        for operation_id in [String::new(), Uuid::new_v4().to_string()] {
            let case = import_case().await;
            let victim_name = format!("user-notes-{operation_id}");
            let victim = case.app_data_dir.join(victim_name);
            fs::write(&victim, b"do-not-delete").unwrap();
            let victim_identity = file_identity(&victim).unwrap().unwrap();
            let intent = ImportIntent {
                version: 1,
                operation_id,
                source_path: case.source_path.clone(),
                snapshot_path: victim.clone(),
                candidate_path: victim.clone(),
                live_path: case.current_path.clone(),
                backup_paths: [victim.clone(), victim.clone(), victim.clone()],
                quarantine_paths: [victim.clone(), victim.clone(), victim.clone()],
                original_live: case.current_before.clone(),
                source_before: case.source_before.clone(),
                snapshot_trio: TrioIdentity {
                    main: None,
                    wal: None,
                    shm: None,
                },
                app_legacy_path: case.app_data_dir.join("meeting_minutes.db"),
                app_legacy_before: case.app_legacy_before.clone(),
                legacy_digest: "forged".to_string(),
                candidate_main: victim_identity,
                stage: ImportStage::Prepared,
            };
            fs::write(
                marker_path(&case.app_data_dir),
                serde_json::to_vec(&intent).unwrap(),
            )
            .unwrap();

            let recovery = recover_import_already_gated(&case.app_data_dir).await;

            assert!(recovery.is_err(), "forged marker paths were accepted");
            assert_eq!(fs::read(&victim).unwrap(), b"do-not-delete");
            assert!(marker_path(&case.app_data_dir).exists());
            assert_eq!(
                trio_identity(&case.current_path).unwrap(),
                case.current_before
            );
        }
    }

    #[tokio::test]
    async fn recovery_rejects_source_aliases_to_every_mutable_operation_path() {
        for alias_index in 0..9 {
            let case = import_case().await;
            let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::IntentWrite(
                ImportStage::Prepared,
            ));
            assert!(
                normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                    .await
                    .is_err()
            );
            let mut intent = read_intent(&case.app_data_dir).unwrap().unwrap();
            let alias = match alias_index {
                0 => intent.snapshot_path.clone(),
                1 => intent.candidate_path.clone(),
                2 => intent.live_path.clone(),
                3..=5 => intent.backup_paths[alias_index - 3].clone(),
                6..=8 => intent.quarantine_paths[alias_index - 6].clone(),
                _ => unreachable!(),
            };
            let alias_before = trio_identity(&alias).unwrap();
            intent.source_path = alias.clone();
            intent.source_before = alias_before.clone();
            fs::write(
                marker_path(&case.app_data_dir),
                serde_json::to_vec(&intent).unwrap(),
            )
            .unwrap();

            let recovery = recover_import_already_gated(&case.app_data_dir).await;

            assert!(recovery.is_err(), "source alias {alias_index} was accepted");
            assert_eq!(trio_identity(&alias).unwrap(), alias_before);
            assert!(marker_path(&case.app_data_dir).exists());
        }
    }

    #[tokio::test]
    async fn recovery_preserves_incomplete_archive_and_fails_closed() {
        let case = import_case().await;
        let ops = CrashAfterBoundaryOps::new(LegacyImportOperation::IntentWrite(
            ImportStage::PublishedUnverified,
        ));
        assert!(
            normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                .await
                .is_err()
        );
        let intent = read_intent(&case.app_data_dir).unwrap().unwrap();
        let backup_before = fs::read(&intent.backup_paths[0]).unwrap();
        fs::remove_file(&intent.quarantine_paths[0]).unwrap();

        let recovery = recover_import_already_gated(&case.app_data_dir).await;

        assert!(recovery.is_err(), "incomplete archive pair was accepted");
        assert_eq!(fs::read(&intent.backup_paths[0]).unwrap(), backup_before);
        assert!(marker_path(&case.app_data_dir).exists());
        assert!(identity_matches(&intent.live_path, &intent.candidate_main).unwrap());
    }

    async fn gated_first_launch_for_test(
        app_data_dir: PathBuf,
        recovery_runs: Arc<AtomicUsize>,
        start: Arc<tokio::sync::Barrier>,
    ) -> Result<bool, sqlx::Error> {
        start.wait().await;
        let _gate = acquire_app_data_gate(&app_data_dir).await?;
        if marker_path(&app_data_dir).exists() {
            recovery_runs.fetch_add(1, Ordering::SeqCst);
        }
        recover_import_already_gated(&app_data_dir).await?;
        Ok(!app_data_dir.join("meeting_minutes.sqlite").exists())
    }

    #[tokio::test]
    async fn concurrent_first_launch_checks_share_recovery_and_never_report_fresh() {
        let boundaries = [
            LegacyImportOperation::QuarantineMove(0),
            LegacyImportOperation::QuarantineDirectorySync(0),
            LegacyImportOperation::QuarantineMove(1),
            LegacyImportOperation::QuarantineDirectorySync(1),
            LegacyImportOperation::QuarantineMove(2),
            LegacyImportOperation::QuarantineDirectorySync(2),
            LegacyImportOperation::PublishLink,
            LegacyImportOperation::PublishedFileSync,
            LegacyImportOperation::PostPublishDirectorySync,
            LegacyImportOperation::IntentWrite(ImportStage::PublishedUnverified),
            LegacyImportOperation::CandidateUnlink,
            LegacyImportOperation::PublishedReopen,
            LegacyImportOperation::PublishedVerify,
        ];

        for operation in boundaries {
            let case = import_case().await;
            let ops = CrashAfterBoundaryOps::new(operation);
            assert!(
                normalize_with_ops(&case.source_path, &case.app_data_dir, &ops)
                    .await
                    .is_err()
            );
            assert!(marker_path(&case.app_data_dir).exists());

            let recovery_runs = Arc::new(AtomicUsize::new(0));
            let start = Arc::new(tokio::sync::Barrier::new(3));
            let first = tokio::spawn(gated_first_launch_for_test(
                case.app_data_dir.clone(),
                recovery_runs.clone(),
                start.clone(),
            ));
            let second = tokio::spawn(gated_first_launch_for_test(
                case.app_data_dir.clone(),
                recovery_runs.clone(),
                start.clone(),
            ));
            start.wait().await;
            let (first, second) = tokio::join!(first, second);
            assert!(
                !first.unwrap().unwrap(),
                "first caller reported fresh at {operation:?}"
            );
            assert!(
                !second.unwrap().unwrap(),
                "second caller reported fresh at {operation:?}"
            );
            assert_eq!(
                recovery_runs.load(Ordering::SeqCst),
                1,
                "recovery did not single-flight at {operation:?}"
            );
            assert_recovered(&case).await;
        }

        let directory = tempfile::tempdir().unwrap();
        let app_data_dir = directory.path().join("app-data");
        fs::create_dir(&app_data_dir).unwrap();
        fs::write(marker_path(&app_data_dir), b"not-json").unwrap();
        let recovery_runs = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(tokio::sync::Barrier::new(3));
        let first = tokio::spawn(gated_first_launch_for_test(
            app_data_dir.clone(),
            recovery_runs.clone(),
            start.clone(),
        ));
        let second = tokio::spawn(gated_first_launch_for_test(
            app_data_dir.clone(),
            recovery_runs,
            start.clone(),
        ));
        start.wait().await;
        let (first, second) = tokio::join!(first, second);
        assert!(first.unwrap().is_err());
        assert!(second.unwrap().is_err());
        assert!(marker_path(&app_data_dir).exists());
        assert!(!app_data_dir.join("meeting_minutes.sqlite").exists());
    }
}
