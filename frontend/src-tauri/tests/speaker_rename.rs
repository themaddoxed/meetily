use app_lib::database::repositories::diarization::DiarizationRepository;
use app_lib::diarization::commands::{rename_speaker_with_pool, RenameSpeakerRequest};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

struct TestDatabase {
    _directory: TempDir,
    path: PathBuf,
    pool: SqlitePool,
}

impl TestDatabase {
    async fn migrated() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("speaker-rename.sqlite");
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

async fn seed(pool: &SqlitePool) {
    for meeting_id in ["meeting-1", "meeting-2"] {
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at)
             VALUES (?, 'Meeting', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .bind(meeting_id)
        .execute(pool)
        .await
        .unwrap();
    }

    for (id, meeting_id, speaker_id, label, text, start) in [
        (
            "t1",
            "meeting-1",
            "speaker-1",
            "Speaker 1",
            "Keep text one",
            0.0,
        ),
        (
            "t2",
            "meeting-1",
            "speaker-1",
            "Speaker 1",
            "Keep text two",
            1.0,
        ),
        (
            "t3",
            "meeting-1",
            "speaker-2",
            "Speaker 2",
            "Other speaker",
            2.0,
        ),
        (
            "t4",
            "meeting-2",
            "speaker-1",
            "Speaker 1",
            "Other meeting",
            0.0,
        ),
    ] {
        sqlx::query(
            "INSERT INTO transcripts (
                id, meeting_id, transcript, timestamp, speaker,
                audio_start_time, audio_end_time, duration,
                speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
             ) VALUES (?, ?, ?, '2026-01-01T00:00:00Z', 'system', ?, ?, 1.0,
                       ?, ?, '#2563eb', 0, 'final', 'test', 0.9)",
        )
        .bind(id)
        .bind(meeting_id)
        .bind(text)
        .bind(start)
        .bind(start + 1.0)
        .bind(speaker_id)
        .bind(label)
        .execute(pool)
        .await
        .unwrap();
    }

    for (id, meeting_id, speaker_id, label, start) in [
        ("s1", "meeting-1", "speaker-1", "Speaker 1", 0.0),
        ("s2", "meeting-1", "speaker-1", "Speaker 1", 1.0),
        ("s3", "meeting-1", "speaker-2", "Speaker 2", 2.0),
        ("s4", "meeting-2", "speaker-1", "Speaker 1", 0.0),
    ] {
        sqlx::query(
            "INSERT INTO speaker_segments (
                id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
                confidence, is_overlap, diarization_status, diarization_method
             ) VALUES (?, ?, 'system', ?, ?, ?, ?, 0.9, 0, 'final', 'test')",
        )
        .bind(id)
        .bind(meeting_id)
        .bind(start)
        .bind(start + 1.0)
        .bind(speaker_id)
        .bind(label)
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn transcript_snapshot(pool: &SqlitePool) -> Vec<(String, String, f64, f64, String)> {
    sqlx::query_as(
        "SELECT id, transcript, audio_start_time, audio_end_time, speaker_label
         FROM transcripts ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

fn request(meeting_id: &str, speaker_id: &str, label: &str) -> RenameSpeakerRequest {
    RenameSpeakerRequest {
        meeting_id: meeting_id.to_string(),
        speaker_id: speaker_id.to_string(),
        label: label.to_string(),
    }
}

#[test]
fn rename_request_serializes_camel_case_fields() {
    let json = serde_json::to_value(request("meeting-1", "speaker-1", "Alice")).unwrap();
    assert_eq!(json["meetingId"], "meeting-1");
    assert_eq!(json["speakerId"], "speaker-1");
    assert!(json.get("meeting_id").is_none());
    assert!(json.get("speaker_id").is_none());
}

#[tokio::test]
async fn invalid_requests_are_rejected_before_mutation() {
    let database = TestDatabase::migrated().await;
    seed(&database.pool).await;
    let before = transcript_snapshot(&database.pool).await;

    for invalid in [
        request(" ", "speaker-1", "Alice"),
        request("meeting-1", " ", "Alice"),
        request("meeting-1", "multiple_speakers", "Alice"),
        request("meeting-1", "speaker-1", " "),
        request("meeting-1", "speaker-1", "Bad\nName"),
        request("meeting-1", "speaker-1", &"я".repeat(129)),
    ] {
        let error = rename_speaker_with_pool(&database.pool, invalid)
            .await
            .unwrap_err();
        assert_eq!(error, "invalid_request");
        assert_eq!(transcript_snapshot(&database.pool).await, before);
    }
}

#[tokio::test]
async fn command_trims_valid_unicode_label_and_reports_missing_speaker() {
    let database = TestDatabase::migrated().await;
    seed(&database.pool).await;

    rename_speaker_with_pool(
        &database.pool,
        request("meeting-1", "speaker-1", "  Алиса 👋  "),
    )
    .await
    .unwrap();
    let label: String = sqlx::query_scalar("SELECT speaker_label FROM transcripts WHERE id = 't1'")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(label, "Алиса 👋");

    let error = rename_speaker_with_pool(
        &database.pool,
        request("meeting-1", "speaker-missing", "Nobody"),
    )
    .await
    .unwrap_err();
    assert_eq!(error, "speaker_not_found");
}

#[tokio::test]
async fn command_sanitizes_persistence_failure() {
    let database = TestDatabase::migrated().await;
    seed(&database.pool).await;
    sqlx::query(
        "CREATE TRIGGER fail_rename_command
         BEFORE UPDATE OF speaker_label ON transcripts
         WHEN NEW.speaker_label = 'Fail'
         BEGIN
             SELECT RAISE(ABORT, 'secret sql detail');
         END",
    )
    .execute(&database.pool)
    .await
    .unwrap();

    let error = rename_speaker_with_pool(&database.pool, request("meeting-1", "speaker-1", "Fail"))
        .await
        .unwrap_err();

    assert_eq!(error, "persistence_failed");
    assert!(!error.contains("secret sql detail"));
}

#[tokio::test]
async fn rename_updates_only_one_meeting_speaker_and_survives_reopen() {
    let mut database = TestDatabase::migrated().await;
    seed(&database.pool).await;

    let updated =
        DiarizationRepository::rename_speaker(&database.pool, "meeting-1", "speaker-1", "Alice")
            .await
            .unwrap();
    assert_eq!(updated, 4);

    database.reopen().await;

    let meeting_one: Vec<String> = sqlx::query_scalar(
        "SELECT speaker_label FROM transcripts
         WHERE meeting_id = 'meeting-1' AND speaker_id = 'speaker-1' ORDER BY id",
    )
    .fetch_all(&database.pool)
    .await
    .unwrap();
    assert_eq!(meeting_one, vec!["Alice", "Alice"]);

    let raw_segments: Vec<String> = sqlx::query_scalar(
        "SELECT speaker_label FROM speaker_segments
         WHERE meeting_id = 'meeting-1' AND speaker_id = 'speaker-1' ORDER BY id",
    )
    .fetch_all(&database.pool)
    .await
    .unwrap();
    assert_eq!(raw_segments, vec!["Alice", "Alice"]);

    let untouched: Vec<String> = sqlx::query_scalar(
        "SELECT speaker_label FROM transcripts
         WHERE id IN ('t3', 't4') ORDER BY id",
    )
    .fetch_all(&database.pool)
    .await
    .unwrap();
    assert_eq!(untouched, vec!["Speaker 2", "Speaker 1"]);

    let text_and_timing: Vec<(String, f64, f64)> = sqlx::query_as(
        "SELECT transcript, audio_start_time, audio_end_time
         FROM transcripts WHERE id IN ('t1', 't2') ORDER BY id",
    )
    .fetch_all(&database.pool)
    .await
    .unwrap();
    assert_eq!(
        text_and_timing,
        vec![
            ("Keep text one".to_string(), 0.0, 1.0),
            ("Keep text two".to_string(), 1.0, 2.0),
        ]
    );
}

#[tokio::test]
async fn missing_speaker_updates_nothing() {
    let database = TestDatabase::migrated().await;
    seed(&database.pool).await;
    let before = transcript_snapshot(&database.pool).await;

    let updated =
        DiarizationRepository::rename_speaker(&database.pool, "meeting-1", "missing", "Nobody")
            .await
            .unwrap();

    assert_eq!(updated, 0);
    assert_eq!(transcript_snapshot(&database.pool).await, before);
}

#[tokio::test]
async fn second_update_failure_rolls_back_transcript_labels() {
    let database = TestDatabase::migrated().await;
    seed(&database.pool).await;
    let before = transcript_snapshot(&database.pool).await;
    sqlx::query(
        "CREATE TRIGGER fail_speaker_segment_rename
         BEFORE UPDATE OF speaker_label ON speaker_segments
         WHEN NEW.speaker_label = 'Rollback'
         BEGIN
             SELECT RAISE(ABORT, 'injected speaker segment failure');
         END",
    )
    .execute(&database.pool)
    .await
    .unwrap();

    let error =
        DiarizationRepository::rename_speaker(&database.pool, "meeting-1", "speaker-1", "Rollback")
            .await
            .unwrap_err();

    assert!(error
        .to_string()
        .contains("injected speaker segment failure"));
    assert_eq!(transcript_snapshot(&database.pool).await, before);
}
