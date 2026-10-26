use app_lib::api::TranscriptSegment;
use app_lib::database::manager::DatabaseManager;
use app_lib::database::repositories::meeting::MeetingsRepository;
use app_lib::database::repositories::summary::SummaryProcessesRepository;
use app_lib::database::repositories::transcript::TranscriptsRepository;
use app_lib::database::repositories::transcript_chunk::TranscriptChunksRepository;
use serde_json::json;
use sqlx::migrate::{Migration, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::borrow::Cow;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
const PREVIOUS_SCHEMA_VERSION: i64 = 20251229000000;

struct TestDatabase {
    _directory: TempDir,
    path: PathBuf,
    pool: SqlitePool,
}

impl TestDatabase {
    async fn migrated() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("meeting_minutes.sqlite");
        let pool = open_pool(&path).await;
        MIGRATOR.run(&pool).await.unwrap();
        Self {
            _directory: directory,
            path,
            pool,
        }
    }

    async fn reopen(&mut self) {
        self.pool.close().await;
        self.pool = open_pool(&self.path).await;
        MIGRATOR.run(&self.pool).await.unwrap();
    }
}

async fn open_pool(path: &Path) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap()
}

async fn materialize_legacy_fixture(path: &Path) {
    let pool = open_pool(path).await;
    sqlx::raw_sql(include_str!("fixtures/legacy_fastapi_schema.sql"))
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

async fn materialize_reformatted_legacy_fixture(path: &Path) {
    let reformatted = include_str!("fixtures/legacy_fastapi_schema.sql")
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    let pool = open_pool(path).await;
    sqlx::raw_sql(&reformatted).execute(&pool).await.unwrap();
    pool.close().await;
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    sidecar.into()
}

fn sqlite_trio_bytes(path: &Path) -> [Option<Vec<u8>>; 3] {
    [
        std::fs::read(path).ok(),
        std::fs::read(sqlite_sidecar_path(path, "-wal")).ok(),
        std::fs::read(sqlite_sidecar_path(path, "-shm")).ok(),
    ]
}

fn assert_sqlite_trio_unchanged(label: &str, path: &Path, expected: &[Option<Vec<u8>>; 3]) {
    let actual = sqlite_trio_bytes(path);
    for (index, suffix) in ["", "-wal", "-shm"].iter().enumerate() {
        assert!(
            actual[index] == expected[index],
            "{label}{suffix} changed: before={} bytes, after={} bytes",
            expected[index].as_ref().map_or(0, Vec::len),
            actual[index].as_ref().map_or(0, Vec::len),
        );
    }
}

fn previous_migrator() -> Migrator {
    let migrations: Vec<Migration> = MIGRATOR
        .iter()
        .filter(|migration| migration.version <= PREVIOUS_SCHEMA_VERSION)
        .cloned()
        .collect();

    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

fn migrator_prefix(count: usize) -> Migrator {
    let migrations: Vec<Migration> = MIGRATOR.iter().take(count).cloned().collect();
    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

fn segment(id: &str, text: &str, start: f64, speaker_label: Option<&str>) -> TranscriptSegment {
    TranscriptSegment {
        id: id.to_string(),
        text: text.to_string(),
        timestamp: "2026-07-14T09:00:00Z".to_string(),
        audio_start_time: Some(start),
        audio_end_time: Some(start + 1.25),
        duration: Some(1.25),
        speaker_id: speaker_label.map(|_| "speaker-1".to_string()),
        speaker_label: speaker_label.map(str::to_string),
        speaker_color: speaker_label.map(|_| "#336699".to_string()),
        is_overlap: Some(false),
        diarization_status: Some(
            if speaker_label.is_some() {
                "final"
            } else {
                "none"
            }
            .to_string(),
        ),
        diarization_method: speaker_label.map(|_| "test-diarizer".to_string()),
        diarization_confidence: speaker_label.map(|_| 0.93),
    }
}

async fn seed_meeting(pool: &SqlitePool, title: &str, text: &str) -> String {
    TranscriptsRepository::save_transcript(
        pool,
        title,
        &[segment("input-1", text, 0.0, None)],
        Some("/tmp/meetily-test-recording".to_string()),
    )
    .await
    .unwrap()
}

async fn transcript_bytes(pool: &SqlitePool, meeting_id: &str) -> Vec<u8> {
    sqlx::query_scalar::<_, String>(
        "SELECT transcript FROM transcripts WHERE meeting_id = ? ORDER BY id LIMIT 1",
    )
    .bind(meeting_id)
    .fetch_one(pool)
    .await
    .unwrap()
    .into_bytes()
}

#[tokio::test]
async fn fresh_database_applies_every_migration_once_and_second_run_is_idempotent() {
    let database = TestDatabase::migrated().await;
    let expected = MIGRATOR.iter().count() as i64;
    let first_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(first_count, expected);

    let meeting_id = seed_meeting(&database.pool, "Migration control", "Never lose this row").await;
    MIGRATOR.run(&database.pool).await.unwrap();

    let second_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(second_count, expected);
    assert_eq!(
        transcript_bytes(&database.pool, &meeting_id).await,
        b"Never lose this row"
    );
}

#[tokio::test]
async fn fresh_database_refuses_orphaned_target_sidecars() {
    let directory = tempfile::tempdir().unwrap();
    let current_path = directory.path().join("meeting_minutes.sqlite");
    let legacy_path = directory.path().join("meeting_minutes.db");
    std::fs::write(sqlite_sidecar_path(&current_path, "-wal"), b"orphan-wal").unwrap();
    std::fs::write(sqlite_sidecar_path(&current_path, "-shm"), b"orphan-shm").unwrap();
    let before = sqlite_trio_bytes(&current_path);

    let result = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await;

    let error = result
        .err()
        .expect("orphaned target sidecars were accepted");
    assert!(
        error
            .to_string()
            .contains("orphaned SQLite target sidecars"),
        "unexpected rejection: {error}"
    );
    assert_sqlite_trio_unchanged("fresh target", &current_path, &before);
}

#[tokio::test]
async fn native_copy_refuses_orphaned_target_sidecars() {
    let directory = tempfile::tempdir().unwrap();
    let current_path = directory.path().join("meeting_minutes.sqlite");
    let legacy_path = directory.path().join("meeting_minutes.db");
    let legacy_pool = open_pool(&legacy_path).await;
    MIGRATOR.run(&legacy_pool).await.unwrap();
    legacy_pool.close().await;
    std::fs::write(sqlite_sidecar_path(&current_path, "-wal"), b"orphan-wal").unwrap();
    std::fs::write(sqlite_sidecar_path(&current_path, "-shm"), b"orphan-shm").unwrap();
    let before = sqlite_trio_bytes(&current_path);

    let result = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await;

    let error = result
        .err()
        .expect("orphaned target sidecars were accepted");
    assert!(
        error
            .to_string()
            .contains("orphaned SQLite target sidecars"),
        "unexpected rejection: {error}"
    );
    assert_sqlite_trio_unchanged("native-copy target", &current_path, &before);
}

#[cfg(unix)]
#[tokio::test]
async fn current_database_rejects_linked_or_dangling_sidecars_without_mutation() {
    use std::os::unix::fs::symlink;

    for scenario in ["wal-symlink", "wal-hardlink", "shm-dangling"] {
        let directory = tempfile::tempdir().unwrap();
        let app_data_dir = directory.path().join("app-data");
        std::fs::create_dir(&app_data_dir).unwrap();
        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        let pool = open_pool(&current_path).await;
        MIGRATOR.run(&pool).await.unwrap();
        pool.close().await;
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(sqlite_sidecar_path(&current_path, suffix));
        }

        let suffix = if scenario.starts_with("wal") {
            "-wal"
        } else {
            "-shm"
        };
        let sidecar = sqlite_sidecar_path(&current_path, suffix);
        let external = directory.path().join(format!("external{suffix}"));
        if scenario == "shm-dangling" {
            symlink(&external, &sidecar).unwrap();
        } else {
            std::fs::write(&external, b"protected-sidecar").unwrap();
            if scenario == "wal-hardlink" {
                std::fs::hard_link(&external, &sidecar).unwrap();
            } else {
                symlink(&external, &sidecar).unwrap();
            }
        }
        let current_before = std::fs::read(&current_path).unwrap();
        let external_before = std::fs::read(&external).ok();

        let result = DatabaseManager::new(
            current_path.to_str().unwrap(),
            legacy_path.to_str().unwrap(),
        )
        .await;

        assert!(result.is_err(), "linked sidecar {scenario} was accepted");
        assert_eq!(std::fs::read(&current_path).unwrap(), current_before);
        assert_eq!(std::fs::read(&external).ok(), external_before);
        assert!(std::fs::symlink_metadata(&sidecar).is_ok());
    }
}

#[tokio::test]
async fn previous_database_is_copied_upgraded_and_preserves_meeting_and_transcript() {
    let directory = tempfile::tempdir().unwrap();
    let legacy_path = directory.path().join("meeting_minutes.db");
    let current_path = directory.path().join("meeting_minutes.sqlite");
    let legacy_pool = open_pool(&legacy_path).await;
    previous_migrator().run(&legacy_pool).await.unwrap();

    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at, folder_path)
         VALUES ('legacy-meeting', 'Previous release', '2026-01-01T00:00:00Z',
                 '2026-01-01T00:00:00Z', '/recordings/previous')",
    )
    .execute(&legacy_pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO transcripts (
            id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time,
            duration, speaker
         ) VALUES (
            'legacy-transcript', 'legacy-meeting', 'Previous transcript survives',
            '2026-01-01T00:00:00Z', 0.0, 2.0, 2.0, 'mic'
         )",
    )
    .execute(&legacy_pool)
    .await
    .unwrap();
    legacy_pool.close().await;

    let manager = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await
    .unwrap();

    let text = sqlx::query_scalar::<_, String>(
        "SELECT transcript FROM transcripts WHERE id = 'legacy-transcript'",
    )
    .fetch_one(manager.pool())
    .await
    .unwrap();
    assert_eq!(text, "Previous transcript survives");
    let status = sqlx::query_scalar::<_, String>(
        "SELECT diarization_status FROM transcripts WHERE id = 'legacy-transcript'",
    )
    .fetch_one(manager.pool())
    .await
    .unwrap();
    assert_eq!(status, "none");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(manager.pool())
            .await
            .unwrap(),
        MIGRATOR.iter().count() as i64
    );
    let meeting_before = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT CAST(title AS BLOB) FROM meetings WHERE id = 'legacy-meeting'",
    )
    .fetch_one(manager.pool())
    .await
    .unwrap();
    let transcript_before = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT CAST(transcript AS BLOB) FROM transcripts WHERE id = 'legacy-transcript'",
    )
    .fetch_one(manager.pool())
    .await
    .unwrap();
    manager.cleanup().await.unwrap();

    let reopened = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT CAST(title AS BLOB) FROM meetings WHERE id = 'legacy-meeting'",
        )
        .fetch_one(reopened.pool())
        .await
        .unwrap(),
        meeting_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT CAST(transcript AS BLOB) FROM transcripts WHERE id = 'legacy-transcript'",
        )
        .fetch_one(reopened.pool())
        .await
        .unwrap(),
        transcript_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1",)
            .fetch_one(reopened.pool())
            .await
            .unwrap(),
        MIGRATOR.iter().count() as i64
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) - COUNT(DISTINCT version) FROM _sqlx_migrations",
        )
        .fetch_one(reopened.pool())
        .await
        .unwrap(),
        0
    );
    reopened.cleanup().await.unwrap();
}

#[tokio::test]
async fn divergent_sqlx_ledger_is_rejected_without_mutation() {
    for case in ["checksum", "future"] {
        let directory = tempfile::tempdir().unwrap();
        let legacy_path = directory.path().join(format!("{case}.db"));
        let current_path = directory.path().join("meeting_minutes.sqlite");
        let pool = open_pool(&legacy_path).await;
        previous_migrator().run(&pool).await.unwrap();
        match case {
            "checksum" => {
                sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = ?")
                    .bind(PREVIOUS_SCHEMA_VERSION)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "future" => {
                sqlx::query(
                    "INSERT INTO _sqlx_migrations
                     (version, description, installed_on, success, checksum, execution_time)
                     VALUES (?, 'future', CURRENT_TIMESTAMP, 1, X'00', 0)",
                )
                .bind(9_999_999_999_999_i64)
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        pool.close().await;
        let before = sqlite_trio_bytes(&legacy_path);

        assert!(
            DatabaseManager::new(
                current_path.to_str().unwrap(),
                legacy_path.to_str().unwrap(),
            )
            .await
            .is_err(),
            "{case} ledger unexpectedly opened"
        );
        assert_sqlite_trio_unchanged(case, &legacy_path, &before);
        assert!(!current_path.exists());
    }
}

#[tokio::test]
async fn divergent_current_wal_trio_is_rejected_without_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let current_path = directory.path().join("meeting_minutes.sqlite");
    let legacy_path = directory.path().join("meeting_minutes.db");
    let options = SqliteConnectOptions::new()
        .filename(&current_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let writer = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    MIGRATOR.run(&writer).await.unwrap();
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&writer)
        .await
        .unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint = 0")
        .execute(&writer)
        .await
        .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = ?")
        .bind(PREVIOUS_SCHEMA_VERSION)
        .execute(&writer)
        .await
        .unwrap();
    let before = sqlite_trio_bytes(&current_path);
    assert!(before[1].is_some(), "test requires a live WAL");
    assert!(before[2].is_some(), "test requires a live SHM");

    let result = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await;

    assert!(result.is_err(), "divergent current ledger was accepted");
    assert_sqlite_trio_unchanged("divergent current", &current_path, &before);
    writer.close().await;
}

#[tokio::test]
async fn every_authentic_sqlx_prefix_upgrades_without_mutation() {
    let migration_count = MIGRATOR.iter().count();
    assert_eq!(
        migration_count, 16,
        "fixture must cover every embedded prefix"
    );

    for count in 1..=migration_count {
        let directory = tempfile::tempdir().unwrap();
        let legacy_path = directory.path().join("meeting_minutes.db");
        let current_path = directory.path().join("meeting_minutes.sqlite");
        let legacy_pool = open_pool(&legacy_path).await;
        migrator_prefix(count).run(&legacy_pool).await.unwrap();
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at)
             VALUES ('prefix-meeting', 'Prefix title', '2026-01-01', '2026-01-01')",
        )
        .execute(&legacy_pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transcripts (id, meeting_id, transcript, timestamp)
             VALUES ('prefix-transcript', 'prefix-meeting', 'Prefix transcript', '2026-01-01')",
        )
        .execute(&legacy_pool)
        .await
        .unwrap();
        legacy_pool.close().await;
        let legacy_before = sqlite_trio_bytes(&legacy_path);

        let manager = DatabaseManager::new(
            current_path.to_str().unwrap(),
            legacy_path.to_str().unwrap(),
        )
        .await
        .unwrap_or_else(|error| panic!("prefix {count} failed to upgrade: {error}"));

        let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(manager.pool())
            .await
            .unwrap();
        assert_eq!(applied as usize, migration_count, "prefix {count}");
        let row = sqlx::query(
            "SELECT meetings.title, meetings.folder_path, transcripts.transcript,
                    transcripts.audio_start_time, transcripts.audio_end_time, transcripts.duration
             FROM meetings JOIN transcripts ON transcripts.meeting_id = meetings.id
             WHERE meetings.id = 'prefix-meeting'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap();
        assert_eq!(row.try_get::<String, _>("title").unwrap(), "Prefix title");
        assert_eq!(
            row.try_get::<String, _>("transcript").unwrap(),
            "Prefix transcript"
        );
        assert_eq!(
            row.try_get::<Option<String>, _>("folder_path").unwrap(),
            None
        );
        assert_eq!(
            row.try_get::<Option<f64>, _>("audio_start_time").unwrap(),
            None
        );
        assert_eq!(
            row.try_get::<Option<f64>, _>("audio_end_time").unwrap(),
            None
        );
        assert_eq!(row.try_get::<Option<f64>, _>("duration").unwrap(), None);
        manager.cleanup().await.unwrap();
        assert_sqlite_trio_unchanged(
            &format!("authentic SQLx prefix {count}"),
            &legacy_path,
            &legacy_before,
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn legacy_import_rejects_app_legacy_alias_to_current_without_mutation() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let app_data_dir = directory.path().join("app-data");
    std::fs::create_dir(&app_data_dir).unwrap();
    let source_path = directory.path().join("selected-source.db");
    materialize_legacy_fixture(&source_path).await;
    let current_path = app_data_dir.join("meeting_minutes.sqlite");
    let current_pool = open_pool(&current_path).await;
    MIGRATOR.run(&current_pool).await.unwrap();
    current_pool.close().await;
    let legacy_path = app_data_dir.join("meeting_minutes.db");
    symlink(&current_path, &legacy_path).unwrap();
    let source_before = sqlite_trio_bytes(&source_path);
    let current_before = sqlite_trio_bytes(&current_path);

    let error = match DatabaseManager::import_legacy_database_to_app_data(
        &source_path,
        &app_data_dir,
    )
    .await
    {
        Err(error) => error,
        Ok(manager) => {
            manager.cleanup().await.unwrap();
            panic!("app-data legacy alias to current target was accepted");
        }
    };

    assert!(error
        .to_string()
        .contains("legacy database path must be an owned regular file"));
    assert_sqlite_trio_unchanged("selected source", &source_path, &source_before);
    assert_sqlite_trio_unchanged("current target", &current_path, &current_before);
    assert!(std::fs::symlink_metadata(&legacy_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!app_data_dir
        .join("meeting_minutes.sqlite.import-v1.json")
        .exists());
}

#[cfg(unix)]
#[tokio::test]
async fn current_database_target_rejects_symlink_aliases_without_mutation() {
    use std::os::unix::fs::symlink;

    for scenario in [
        "external",
        "external-hardlink",
        "legacy",
        "hardlink",
        "dangling",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let app_data_dir = directory.path().join("app-data");
        std::fs::create_dir(&app_data_dir).unwrap();
        let current_path = app_data_dir.join("meeting_minutes.sqlite");
        let legacy_path = app_data_dir.join("meeting_minutes.db");
        let external_path = directory.path().join("external.sqlite");
        let protected_path = if matches!(scenario, "legacy" | "hardlink") {
            &legacy_path
        } else {
            &external_path
        };

        let protected_before = if scenario == "dangling" {
            None
        } else {
            let pool = open_pool(protected_path).await;
            previous_migrator().run(&pool).await.unwrap();
            pool.close().await;
            Some(sqlite_trio_bytes(protected_path))
        };
        if matches!(scenario, "hardlink" | "external-hardlink") {
            std::fs::hard_link(protected_path, &current_path).unwrap();
        } else {
            symlink(protected_path, &current_path).unwrap();
        }

        let error = match DatabaseManager::new(
            current_path.to_str().unwrap(),
            legacy_path.to_str().unwrap(),
        )
        .await
        {
            Err(error) => error,
            Ok(manager) => {
                manager.cleanup().await.unwrap();
                panic!("current target symlink scenario {scenario} was accepted");
            }
        };

        assert!(
            error.to_string().contains("current"),
            "unexpected {scenario} rejection: {error}"
        );
        if matches!(scenario, "hardlink" | "external-hardlink") {
            assert!(std::fs::symlink_metadata(&current_path).unwrap().is_file());
        } else {
            assert!(std::fs::symlink_metadata(&current_path)
                .unwrap()
                .file_type()
                .is_symlink());
        }
        if let Some(before) = protected_before {
            assert_sqlite_trio_unchanged(scenario, protected_path, &before);
        } else {
            assert!(!protected_path.exists());
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn selected_source_rejects_linked_main_or_sidecars_without_mutation() {
    use std::os::unix::fs::symlink;

    for scenario in [
        "main-symlink",
        "main-hardlink",
        "wal-symlink",
        "wal-hardlink",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let app_data_dir = directory.path().join("app-data");
        std::fs::create_dir(&app_data_dir).unwrap();
        let source_path = directory.path().join("selected-source.db");
        let protected_path = directory.path().join("protected-source.db");

        if scenario.starts_with("main") {
            materialize_legacy_fixture(&protected_path).await;
            if scenario == "main-hardlink" {
                std::fs::hard_link(&protected_path, &source_path).unwrap();
            } else {
                symlink(&protected_path, &source_path).unwrap();
            }
        } else {
            materialize_legacy_fixture(&source_path).await;
            std::fs::write(&protected_path, b"protected-wal").unwrap();
            let source_wal = sqlite_sidecar_path(&source_path, "-wal");
            if scenario == "wal-hardlink" {
                std::fs::hard_link(&protected_path, &source_wal).unwrap();
            } else {
                symlink(&protected_path, &source_wal).unwrap();
            }
        }
        let source_before = sqlite_trio_bytes(&source_path);
        let protected_before = std::fs::read(&protected_path).unwrap();

        let result =
            DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir).await;

        assert!(
            result.is_err(),
            "linked selected source {scenario} was accepted"
        );
        assert_sqlite_trio_unchanged(scenario, &source_path, &source_before);
        assert_eq!(std::fs::read(&protected_path).unwrap(), protected_before);
        assert!(!app_data_dir.join("meeting_minutes.sqlite").exists());
        assert!(!app_data_dir
            .join("meeting_minutes.sqlite.import-v1.json")
            .exists());
    }
}

#[tokio::test]
async fn legacy_import_rejects_unknown_ledgerless_schemas_without_mutation() {
    let fixture = include_str!("fixtures/legacy_fastapi_schema.sql");
    let variants = [
        (
            "partial",
            "CREATE TABLE meetings (id TEXT PRIMARY KEY);".to_string(),
        ),
        (
            "extra-table",
            format!("{fixture}\nCREATE TABLE surprise (id INTEGER);"),
        ),
        (
            "extra-column",
            format!("{fixture}\nALTER TABLE meetings ADD COLUMN surprise TEXT;"),
        ),
        (
            "altered-type",
            fixture.replace("folder_path TEXT", "folder_path BLOB"),
        ),
        (
            "altered-default",
            fixture.replace(
                "status TEXT NOT NULL",
                "status TEXT NOT NULL DEFAULT 'PENDING'",
            ),
        ),
        (
            "altered-foreign-key",
            fixture.replace(
                "REFERENCES meetings(id)",
                "REFERENCES meetings(id) ON DELETE CASCADE",
            ),
        ),
        (
            "unrelated",
            "CREATE TABLE unrelated (value TEXT);".to_string(),
        ),
    ];

    for (case, schema) in variants {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join(format!("{case}.db"));
        let app_data_dir = directory.path().join("app-data");
        std::fs::create_dir(&app_data_dir).unwrap();
        let pool = open_pool(&source_path).await;
        sqlx::raw_sql(&schema).execute(&pool).await.unwrap();
        pool.close().await;
        let before = sqlite_trio_bytes(&source_path);

        assert!(
            DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir)
                .await
                .is_err(),
            "{case} schema unexpectedly imported"
        );
        assert_sqlite_trio_unchanged(case, &source_path, &before);
        assert!(!app_data_dir.join("meeting_minutes.sqlite").exists());
    }

    let directory = tempfile::tempdir().unwrap();
    let malformed = directory.path().join("malformed.db");
    let app_data_dir = directory.path().join("app-data");
    std::fs::create_dir(&app_data_dir).unwrap();
    std::fs::write(&malformed, b"not a sqlite database").unwrap();
    let before = sqlite_trio_bytes(&malformed);
    assert!(
        DatabaseManager::import_legacy_database_to_app_data(&malformed, &app_data_dir)
            .await
            .is_err()
    );
    assert_sqlite_trio_unchanged("malformed", &malformed, &before);
}

#[tokio::test]
async fn ledgerless_fastapi_schema_import_preserves_all_values_and_reopens() {
    let directory = tempfile::tempdir().unwrap();
    let legacy_path = directory.path().join("meeting_minutes.db");
    let current_path = directory.path().join("meeting_minutes.sqlite");
    let legacy_pool = open_pool(&legacy_path).await;
    sqlx::raw_sql(include_str!("fixtures/legacy_fastapi_schema.sql"))
        .execute(&legacy_pool)
        .await
        .unwrap();
    legacy_pool.close().await;

    let manager = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(manager.pool())
            .await
            .unwrap(),
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT transcript FROM transcripts WHERE id = 'transcript-b'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap(),
        "Юникод и emoji: 🎙️"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT hex(action_items) FROM transcripts WHERE id = 'transcript-a'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap(),
        "00FF10"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT chunk_count FROM summary_processes WHERE meeting_id = 'meeting-b'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap(),
        -2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(manager.pool())
            .await
            .unwrap(),
        MIGRATOR.iter().count() as i64
    );
    manager.cleanup().await.unwrap();

    let reopened = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM transcript_settings")
            .fetch_one(reopened.pool())
            .await
            .unwrap(),
        3
    );
    reopened.cleanup().await.unwrap();
}

#[tokio::test]
async fn legacy_import_accepts_semantic_python_schema_with_different_ddl_formatting() {
    let directory = tempfile::tempdir().unwrap();
    let legacy_path = directory.path().join("meeting_minutes.db");
    let current_path = directory.path().join("meeting_minutes.sqlite");
    materialize_reformatted_legacy_fixture(&legacy_path).await;

    let manager = DatabaseManager::new(
        current_path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(manager.pool())
            .await
            .unwrap(),
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT transcript FROM transcripts WHERE id = 'transcript-b'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap(),
        "Юникод и emoji: 🎙️"
    );
    manager.cleanup().await.unwrap();
}

#[tokio::test]
async fn public_legacy_import_includes_wal_rows_and_preserves_sqlite_trios() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("selected-source.db");
    let app_data_dir = directory.path().join("app-data");
    std::fs::create_dir(&app_data_dir).unwrap();
    let app_legacy_path = app_data_dir.join("meeting_minutes.db");
    let current_path = app_data_dir.join("meeting_minutes.sqlite");

    let source_options = SqliteConnectOptions::new()
        .filename(&source_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let source_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(source_options)
        .await
        .unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint = 0")
        .execute(&source_pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("fixtures/legacy_fastapi_schema.sql"))
        .execute(&source_pool)
        .await
        .unwrap();
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&source_pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO meetings VALUES (
            'meeting-wal', 'Committed only in WAL', '2026-01-04T10:00:00Z',
            '2026-01-04T10:05:00Z', '/recordings/wal'
         )",
    )
    .execute(&source_pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO transcripts VALUES (
            'transcript-wal', 'meeting-wal', 'WAL row survives', '2026-01-04T10:01:00Z',
            NULL, NULL, NULL, 0.0, 1.0, 1.0
         )",
    )
    .execute(&source_pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO summary_processes (
            meeting_id, status, created_at, updated_at, result, chunk_count, processing_time
         ) VALUES (
            'meeting-wal', 'COMPLETED', '2026-01-04T10:02:00Z',
            '2026-01-04T10:03:00Z', '{\"wal\":true}', 1, 0.25
         )",
    )
    .execute(&source_pool)
    .await
    .unwrap();

    std::fs::write(&app_legacy_path, b"pre-existing app legacy bytes").unwrap();
    let current_pool = open_pool(&current_path).await;
    MIGRATOR.run(&current_pool).await.unwrap();
    current_pool.close().await;

    let source_before = sqlite_trio_bytes(&source_path);
    let app_legacy_before = sqlite_trio_bytes(&app_legacy_path);
    let current_before = sqlite_trio_bytes(&current_path);

    let manager = DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir)
        .await
        .unwrap();

    assert_sqlite_trio_unchanged("selected source", &source_path, &source_before);
    assert_sqlite_trio_unchanged("app-data legacy", &app_legacy_path, &app_legacy_before);
    assert_ne!(sqlite_trio_bytes(&current_path), current_before);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT transcript FROM transcripts WHERE id = 'transcript-wal'",
        )
        .fetch_one(manager.pool())
        .await
        .unwrap(),
        "WAL row survives"
    );
    manager.cleanup().await.unwrap();
    source_pool.close().await;
}

#[tokio::test]
async fn legacy_import_digest_rejects_nonrepresentative_row_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("selected-source.db");
    let app_data_dir = directory.path().join("app-data");
    std::fs::create_dir(&app_data_dir).unwrap();
    materialize_legacy_fixture(&source_path).await;
    let source_pool = open_pool(&source_path).await;
    let large_middle_row = format!("Юникод и emoji: 🎙️{}", "x".repeat(8 * 1024 * 1024));
    sqlx::query("UPDATE transcripts SET transcript = ? WHERE id = 'transcript-b'")
        .bind(&large_middle_row)
        .execute(&source_pool)
        .await
        .unwrap();
    source_pool.close().await;

    let current_path = app_data_dir.join("meeting_minutes.sqlite");
    let current_pool = open_pool(&current_path).await;
    MIGRATOR.run(&current_pool).await.unwrap();
    seed_meeting(&current_pool, "Original target", "Must be restored").await;
    current_pool.close().await;
    let source_before = sqlite_trio_bytes(&source_path);
    let current_before = sqlite_trio_bytes(&current_path);
    let watched_dir = app_data_dir.clone();

    let corrupter = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let needle = "Юникод и emoji: 🎙️".as_bytes();
        while Instant::now() < deadline {
            let candidates = std::fs::read_dir(&watched_dir)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.contains(".candidate-"))
                });
            for candidate in candidates {
                if let Ok(bytes) = std::fs::read(&candidate) {
                    if let Some(offset) = bytes.windows(needle.len()).position(|w| w == needle) {
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .open(&candidate)
                            .unwrap();
                        file.seek(SeekFrom::Start(offset as u64)).unwrap();
                        file.write_all(b"X").unwrap();
                        file.sync_all().unwrap();
                        return true;
                    }
                }
            }
            std::thread::yield_now();
        }
        false
    });

    let result =
        DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir).await;
    assert!(
        corrupter.join().unwrap(),
        "did not reach published candidate"
    );
    assert!(result.is_err(), "corrupted candidate unexpectedly imported");
    assert_sqlite_trio_unchanged("selected source", &source_path, &source_before);
    assert_sqlite_trio_unchanged("restored target", &current_path, &current_before);
    assert!(!app_data_dir
        .join("meeting_minutes.sqlite.import-v1.json")
        .exists());
}

#[tokio::test]
async fn legacy_import_replacement_quarantines_stale_sidecars_and_reopens() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("selected-source.db");
    let app_data_dir = directory.path().join("app-data");
    std::fs::create_dir(&app_data_dir).unwrap();
    materialize_legacy_fixture(&source_path).await;

    let current_path = app_data_dir.join("meeting_minutes.sqlite");
    let current_pool = open_pool(&current_path).await;
    MIGRATOR.run(&current_pool).await.unwrap();
    seed_meeting(&current_pool, "Old target", "Old transcript").await;
    current_pool.close().await;
    let mut wal_path = current_path.as_os_str().to_os_string();
    wal_path.push("-wal");
    let wal_path = PathBuf::from(wal_path);
    let mut shm_path = current_path.as_os_str().to_os_string();
    shm_path.push("-shm");
    let shm_path = PathBuf::from(shm_path);
    std::fs::write(&wal_path, b"stale-wal-bytes").unwrap();
    std::fs::write(&shm_path, b"stale-shm-bytes").unwrap();
    let original = sqlite_trio_bytes(&current_path);

    let manager = DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(manager.pool())
            .await
            .unwrap(),
        3
    );
    let imported = sqlite_trio_bytes(&current_path);
    assert_ne!(imported[0], original[0]);
    assert!(
        DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir)
            .await
            .is_err(),
        "a live target pool must forbid repeated import"
    );
    manager.cleanup().await.unwrap();

    let names: Vec<String> = std::fs::read_dir(&app_data_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names
        .iter()
        .any(|name| name.contains(".backup-") && name.ends_with("-wal")));
    assert!(names
        .iter()
        .any(|name| name.contains(".quarantine-") && name.ends_with("-shm")));

    let reopened = DatabaseManager::new(
        current_path.to_str().unwrap(),
        app_data_dir.join("meeting_minutes.db").to_str().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT transcript FROM transcripts WHERE id = 'transcript-b'",
        )
        .fetch_one(reopened.pool())
        .await
        .unwrap(),
        "Юникод и emoji: 🎙️"
    );
    reopened.cleanup().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn legacy_import_real_filesystem_failures_preserve_or_quarantine_target() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("selected-source.db");
    let app_data_dir = directory.path().join("app-data");
    std::fs::create_dir(&app_data_dir).unwrap();
    materialize_legacy_fixture(&source_path).await;
    let current_path = app_data_dir.join("meeting_minutes.sqlite");
    let pool = open_pool(&current_path).await;
    MIGRATOR.run(&pool).await.unwrap();
    seed_meeting(&pool, "Protected", "Must remain").await;
    pool.close().await;
    let source_before = sqlite_trio_bytes(&source_path);
    let current_before = sqlite_trio_bytes(&current_path);

    let original_mode = std::fs::metadata(&app_data_dir)
        .unwrap()
        .permissions()
        .mode();
    std::fs::set_permissions(&app_data_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let result =
        DatabaseManager::import_legacy_database_to_app_data(&source_path, &app_data_dir).await;
    std::fs::set_permissions(
        &app_data_dir,
        std::fs::Permissions::from_mode(original_mode),
    )
    .unwrap();

    assert!(
        result.is_err(),
        "read-only app-data directory unexpectedly imported"
    );
    assert_sqlite_trio_unchanged("selected source", &source_path, &source_before);
    assert_sqlite_trio_unchanged("protected target", &current_path, &current_before);
    assert!(!app_data_dir
        .join("meeting_minutes.sqlite.import-v1.json")
        .exists());
}

#[tokio::test]
async fn saved_meeting_timing_and_speaker_assignment_survive_pool_restart() {
    let mut database = TestDatabase::migrated().await;
    let meeting_id = TranscriptsRepository::save_transcript(
        &database.pool,
        "Restart meeting",
        &[
            segment("input-1", "First sentence", 0.0, Some("Alice")),
            segment("input-2", "Second sentence", 1.25, Some("Alice")),
        ],
        Some("/recordings/restart-meeting".to_string()),
    )
    .await
    .unwrap();

    database.reopen().await;
    let meeting = MeetingsRepository::get_meeting(&database.pool, &meeting_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(meeting.title, "Restart meeting");
    assert_eq!(meeting.transcripts.len(), 2);
    assert_eq!(meeting.transcripts[0].text, "First sentence");
    assert_eq!(meeting.transcripts[0].audio_start_time, Some(0.0));
    assert_eq!(
        meeting.transcripts[0].speaker_label.as_deref(),
        Some("Alice")
    );
    assert_eq!(
        meeting.transcripts[0].diarization_status.as_deref(),
        Some("final")
    );
}

#[tokio::test]
async fn empty_transcript_creates_reopenable_meeting_without_touching_existing_data() {
    let mut database = TestDatabase::migrated().await;
    let existing_id = seed_meeting(&database.pool, "Existing", "Existing transcript").await;
    let empty_id =
        TranscriptsRepository::save_transcript(&database.pool, "Silent meeting", &[], None)
            .await
            .unwrap();

    database.reopen().await;
    let empty = MeetingsRepository::get_meeting(&database.pool, &empty_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(empty.title, "Silent meeting");
    assert!(empty.transcripts.is_empty());
    assert_eq!(
        transcript_bytes(&database.pool, &existing_id).await,
        b"Existing transcript"
    );
}

#[tokio::test]
async fn intermediate_transcript_checkpoint_upsert_survives_restart() {
    let mut database = TestDatabase::migrated().await;
    let meeting_id = seed_meeting(&database.pool, "Checkpoint", "first persisted segment").await;

    TranscriptChunksRepository::save_transcript_data(
        &database.pool,
        &meeting_id,
        "first persisted segment",
        "localWhisper",
        "small",
        30,
        5,
    )
    .await
    .unwrap();
    TranscriptChunksRepository::save_transcript_data(
        &database.pool,
        &meeting_id,
        "first persisted segment second persisted segment",
        "localWhisper",
        "small",
        30,
        5,
    )
    .await
    .unwrap();

    database.reopen().await;
    let checkpoint = sqlx::query(
        "SELECT transcript_text, model_name FROM transcript_chunks WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(
        checkpoint.get::<String, _>("transcript_text"),
        "first persisted segment second persisted segment"
    );
    assert_eq!(checkpoint.get::<String, _>("model_name"), "small");
}

#[tokio::test]
async fn failed_transcript_insert_rolls_back_new_meeting_and_preserves_existing_rows() {
    let database = TestDatabase::migrated().await;
    let existing_id = seed_meeting(&database.pool, "Existing", "Keep this transcript").await;
    sqlx::query(
        "CREATE TRIGGER force_transcript_failure
         BEFORE INSERT ON transcripts
         BEGIN
           SELECT RAISE(ABORT, 'forced transcript failure');
         END",
    )
    .execute(&database.pool)
    .await
    .unwrap();

    let result = TranscriptsRepository::save_transcript(
        &database.pool,
        "Must roll back",
        &[segment("failure", "Do not partially save", 0.0, None)],
        None,
    )
    .await;
    assert!(result.is_err());

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&database.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        transcript_bytes(&database.pool, &existing_id).await,
        b"Keep this transcript"
    );
}

#[tokio::test]
async fn provided_meeting_id_retry_is_idempotent_and_preserves_first_payload() {
    let database = TestDatabase::migrated().await;
    let meeting_id = "meeting-recovery-idempotent";

    let first_id = TranscriptsRepository::save_transcript_with_id(
        &database.pool,
        meeting_id,
        "First recovery",
        &[segment("first", "Primary transcript survives", 0.0, None)],
        Some("/recordings/first".to_string()),
    )
    .await
    .unwrap();
    let retry_id = TranscriptsRepository::save_transcript_with_id(
        &database.pool,
        meeting_id,
        "Retry must not overwrite",
        &[segment(
            "retry",
            "Duplicate transcript must not appear",
            1.0,
            None,
        )],
        Some("/recordings/retry".to_string()),
    )
    .await
    .unwrap();

    assert_eq!(first_id, meeting_id);
    assert_eq!(retry_id, meeting_id);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT transcript FROM transcripts WHERE meeting_id = ?",)
            .bind(meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap(),
        "Primary transcript survives"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT title FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap(),
        "First recovery"
    );
}

async fn seed_completed_summary(pool: &SqlitePool, meeting_id: &str, result: &str) {
    sqlx::query(
        "INSERT INTO summary_processes (
            meeting_id, status, created_at, updated_at, result, chunk_count, processing_time
         ) VALUES (?, 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, ?, 1, 0.25)",
    )
    .bind(meeting_id)
    .bind(result)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_summary_regeneration_restores_previous_summary_and_preserves_transcript() {
    let mut database = TestDatabase::migrated().await;
    let meeting_id = seed_meeting(&database.pool, "Summary", "Transcript is primary data").await;
    let old_summary = json!({"markdown": "Old safe summary"}).to_string();
    seed_completed_summary(&database.pool, &meeting_id, &old_summary).await;
    let before = transcript_bytes(&database.pool, &meeting_id).await;

    SummaryProcessesRepository::create_or_reset_process(&database.pool, &meeting_id)
        .await
        .unwrap();
    SummaryProcessesRepository::update_process_failed(
        &database.pool,
        &meeting_id,
        "local model unavailable",
    )
    .await
    .unwrap();

    database.reopen().await;
    let summary =
        sqlx::query("SELECT status, result, error FROM summary_processes WHERE meeting_id = ?")
            .bind(&meeting_id)
            .fetch_one(&database.pool)
            .await
            .unwrap();
    assert_eq!(summary.get::<String, _>("status"), "failed");
    assert_eq!(summary.get::<String, _>("result"), old_summary);
    assert_eq!(summary.get::<String, _>("error"), "local model unavailable");
    assert_eq!(transcript_bytes(&database.pool, &meeting_id).await, before);
}

#[tokio::test]
async fn interrupted_summary_regeneration_does_not_remain_pending_after_restart() {
    let mut database = TestDatabase::migrated().await;
    let meeting_id = seed_meeting(
        &database.pool,
        "Interrupted summary",
        "Transcript survives interrupted regeneration",
    )
    .await;
    let old_summary = json!({"markdown": "Previous summary survives"}).to_string();
    seed_completed_summary(&database.pool, &meeting_id, &old_summary).await;
    let before = transcript_bytes(&database.pool, &meeting_id).await;

    SummaryProcessesRepository::create_or_reset_process(&database.pool, &meeting_id)
        .await
        .unwrap();
    TranscriptChunksRepository::save_transcript_data(
        &database.pool,
        &meeting_id,
        "Transcript survives interrupted regeneration",
        "builtin-ai",
        "gemma4:e2b",
        40_000,
        1_000,
    )
    .await
    .unwrap();

    database.pool.close().await;
    let legacy_path = database.path.with_extension("legacy.db");
    let manager = DatabaseManager::new(
        database.path.to_str().unwrap(),
        legacy_path.to_str().unwrap(),
    )
    .await
    .unwrap();
    database.pool = manager.pool().clone();
    let summary = sqlx::query("SELECT status, result FROM summary_processes WHERE meeting_id = ?")
        .bind(&meeting_id)
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(summary.get::<String, _>("result"), old_summary);
    assert_ne!(
        summary.get::<String, _>("status").to_ascii_uppercase(),
        "PENDING",
        "a restarted process cannot resume the destroyed background task"
    );
    assert_eq!(transcript_bytes(&database.pool, &meeting_id).await, before);
}

#[tokio::test]
async fn first_summary_failure_keeps_meeting_and_transcript_reopenable() {
    let mut database = TestDatabase::migrated().await;
    let meeting_id = seed_meeting(&database.pool, "No summary yet", "Keep without summary").await;
    let before = transcript_bytes(&database.pool, &meeting_id).await;

    SummaryProcessesRepository::create_or_reset_process(&database.pool, &meeting_id)
        .await
        .unwrap();
    SummaryProcessesRepository::update_process_failed(
        &database.pool,
        &meeting_id,
        "LLM returned invalid output",
    )
    .await
    .unwrap();

    database.reopen().await;
    let meeting = MeetingsRepository::get_meeting(&database.pool, &meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(meeting.transcripts.len(), 1);
    assert_eq!(transcript_bytes(&database.pool, &meeting_id).await, before);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM summary_processes WHERE meeting_id = ?",
        )
        .bind(&meeting_id)
        .fetch_one(&database.pool)
        .await
        .unwrap(),
        "failed"
    );
}
