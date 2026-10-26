use app_lib::api::TranscriptSegment;
use app_lib::database::repositories::diarization::clear_meeting_diarization;
use app_lib::database::repositories::meeting::MeetingsRepository;
use app_lib::database::repositories::transcript::TranscriptsRepository;
use sqlx::migrate::{Migration, Migrator};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Connection, Row, SqlitePool};
use std::borrow::Cow;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

const LAST_PRE_DIARIZATION_MIGRATION: i64 = 20251229000000;
const LEGACY_DIARIZATION_MIGRATION: i64 = 20260626000100;
const HARDENED_DIARIZATION_MIGRATION: i64 = 20260713000100;
const LEGACY_DIARIZATION_SHA384: &str =
    "befb157ec5e25b01c426334d91fd9ade222a62d86c4083723f79de931f413b526fca4882da9393837c1a63ad6b60c08f";

async fn database(foreign_keys: bool) -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let pragma = if foreign_keys {
        "PRAGMA foreign_keys = ON"
    } else {
        "PRAGMA foreign_keys = OFF"
    };
    sqlx::query(pragma).execute(&pool).await.unwrap();
    pool
}

fn pre_diarization_migrator() -> Migrator {
    let migrations: Vec<Migration> = MIGRATOR
        .iter()
        .filter(|migration| migration.version <= LAST_PRE_DIARIZATION_MIGRATION)
        .cloned()
        .collect();

    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

fn legacy_diarization_migrator() -> Migrator {
    let migrations: Vec<Migration> = MIGRATOR
        .iter()
        .filter(|migration| migration.version <= LEGACY_DIARIZATION_MIGRATION)
        .cloned()
        .collect();

    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

fn migration_checksum(version: i64) -> String {
    MIGRATOR
        .iter()
        .find(|migration| migration.version == version)
        .unwrap_or_else(|| panic!("missing migration {version}"))
        .checksum
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn object_exists(pool: &SqlitePool, object_type: &str, name: &str) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE type = ? AND name = ?")
        .bind(object_type)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
        == 1
}

async fn insert_legacy_meeting_and_transcript(pool: &SqlitePool) {
    sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind("meeting-legacy")
        .bind("Legacy meeting")
        .bind("2026-01-01T00:00:00Z")
        .bind("2026-01-01T00:00:00Z")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO transcripts (
            id, meeting_id, transcript, timestamp, speaker,
            audio_start_time, audio_end_time, duration
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("transcript-legacy")
    .bind("meeting-legacy")
    .bind("Legacy transcript")
    .bind("2026-01-01T00:00:00Z")
    .bind("mic")
    .bind(0.0_f64)
    .bind(1.0_f64)
    .bind(1.0_f64)
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_old_row_defaults(pool: &SqlitePool) {
    let row = sqlx::query(
        "SELECT speaker, speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
         FROM transcripts WHERE id = ?",
    )
    .bind("transcript-legacy")
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(
        row.get::<Option<String>, _>("speaker").as_deref(),
        Some("mic")
    );
    assert_eq!(row.get::<Option<String>, _>("speaker_id"), None);
    assert_eq!(row.get::<Option<String>, _>("speaker_label"), None);
    assert_eq!(row.get::<Option<String>, _>("speaker_color"), None);
    assert_eq!(row.get::<i64, _>("is_overlap"), 0);
    assert_eq!(row.get::<String, _>("diarization_status"), "none");
    assert_eq!(row.get::<Option<String>, _>("diarization_method"), None);
    assert_eq!(row.get::<Option<f64>, _>("diarization_confidence"), None);
}

async fn seed_diarization(pool: &SqlitePool, meeting_id: &str, transcript_id: &str) {
    sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind(meeting_id)
        .bind("Diarized meeting")
        .bind("2026-01-01T00:00:00Z")
        .bind("2026-01-01T00:00:00Z")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO transcripts (
            id, meeting_id, transcript, timestamp, speaker,
            audio_start_time, audio_end_time, duration,
            speaker_id, speaker_label, speaker_color, is_overlap,
            diarization_status, diarization_method, diarization_confidence
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(transcript_id)
    .bind(meeting_id)
    .bind("Diarized transcript")
    .bind("2026-01-01T00:00:00Z")
    .bind("mic")
    .bind(0.0_f64)
    .bind(1.0_f64)
    .bind(1.0_f64)
    .bind("speaker-1")
    .bind("Speaker 1")
    .bind("#123456")
    .bind(0_i64)
    .bind("final")
    .bind("test")
    .bind(0.9_f64)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO meeting_diarization_status (
            meeting_id, enabled, live_enabled, post_call_enabled, current_status
         ) VALUES (?, 1, 1, 1, 'final')",
    )
    .bind(meeting_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO speaker_segments (
            id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
            confidence, is_overlap, diarization_status, diarization_method
         ) VALUES (?, ?, 'system', 0.0, 1.0, 'speaker-1', 'Speaker 1', 0.9, 0, 'final', 'test')",
    )
    .bind(format!("segment-{meeting_id}"))
    .bind(meeting_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_diarization_rows(pool: &SqlitePool, meeting_id: &str, expected: i64) {
    for table in ["speaker_segments", "meeting_diarization_status"] {
        let count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} WHERE meeting_id = ?"
        ))
        .bind(meeting_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(count, expected, "unexpected {table} row count");
    }
}

fn replacement_segment(id: &str) -> TranscriptSegment {
    TranscriptSegment {
        id: id.to_string(),
        text: "Replacement transcript".to_string(),
        timestamp: "2026-01-01T00:01:00Z".to_string(),
        audio_start_time: Some(2.0),
        audio_end_time: Some(3.0),
        duration: Some(1.0),
        speaker_id: None,
        speaker_label: None,
        speaker_color: None,
        is_overlap: Some(false),
        diarization_status: Some("none".to_string()),
        diarization_method: None,
        diarization_confidence: None,
    }
}

#[tokio::test]
async fn fresh_database_has_diarization_tables_columns_indexes_and_defaults() {
    let pool = database(true).await;
    MIGRATOR.run(&pool).await.unwrap();

    for table in [
        "diarization_settings",
        "meeting_diarization_status",
        "speaker_segments",
    ] {
        assert!(
            object_exists(&pool, "table", table).await,
            "missing table {table}"
        );
    }
    for index in [
        "idx_speaker_segments_meeting_time",
        "idx_transcripts_meeting_diarization",
    ] {
        assert!(
            object_exists(&pool, "index", index).await,
            "missing index {index}"
        );
    }

    let transcript_columns: Vec<String> = sqlx::query("PRAGMA table_info(transcripts)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    for column in [
        "speaker_id",
        "speaker_label",
        "speaker_color",
        "is_overlap",
        "diarization_status",
        "diarization_method",
        "diarization_confidence",
    ] {
        assert!(
            transcript_columns.iter().any(|actual| actual == column),
            "missing transcripts.{column}"
        );
    }

    insert_legacy_meeting_and_transcript(&pool).await;
    assert_old_row_defaults(&pool).await;

    let settings = sqlx::query(
        "SELECT id, enabled, mode, show_provisional_labels,
                post_call_refinement_enabled, overlap_handling, speaker_review_enabled
         FROM diarization_settings WHERE id = '1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(settings.get::<String, _>("id"), "1");
    assert_eq!(settings.get::<i64, _>("enabled"), 0);
    assert_eq!(settings.get::<String, _>("mode"), "live_plus_post_call");
    assert_eq!(settings.get::<i64, _>("show_provisional_labels"), 1);
    assert_eq!(settings.get::<i64, _>("post_call_refinement_enabled"), 1);
    assert_eq!(
        settings.get::<String, _>("overlap_handling"),
        "multiple_speakers"
    );
    assert_eq!(settings.get::<i64, _>("speaker_review_enabled"), 1);
}

#[tokio::test]
async fn upgrade_from_last_pre_diarization_migration_preserves_old_rows() {
    let pool = database(true).await;
    pre_diarization_migrator().run(&pool).await.unwrap();
    insert_legacy_meeting_and_transcript(&pool).await;

    MIGRATOR.run(&pool).await.unwrap();

    assert_old_row_defaults(&pool).await;
    assert!(object_exists(&pool, "table", "speaker_segments").await);
    let migration_applied = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 20260626000100",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(migration_applied, 1);
}

#[tokio::test]
async fn migration_constraints_reject_invalid_persisted_values() {
    let pool = database(true).await;
    MIGRATOR.run(&pool).await.unwrap();
    insert_legacy_meeting_and_transcript(&pool).await;

    assert!(object_exists(&pool, "table", "diarization_settings").await);
    assert!(object_exists(&pool, "table", "meeting_diarization_status").await);
    assert!(object_exists(&pool, "table", "speaker_segments").await);

    for statement in [
        "UPDATE transcripts SET is_overlap = 2 WHERE id = 'transcript-legacy'",
        "UPDATE transcripts SET diarization_status = 'invalid' WHERE id = 'transcript-legacy'",
        "UPDATE transcripts SET diarization_confidence = -0.01 WHERE id = 'transcript-legacy'",
        "UPDATE transcripts SET diarization_confidence = 1.01 WHERE id = 'transcript-legacy'",
        "UPDATE diarization_settings SET enabled = 2 WHERE id = '1'",
        "UPDATE diarization_settings SET show_provisional_labels = 2 WHERE id = '1'",
        "UPDATE diarization_settings SET post_call_refinement_enabled = 2 WHERE id = '1'",
        "UPDATE diarization_settings SET speaker_review_enabled = 2 WHERE id = '1'",
        "UPDATE diarization_settings SET mode = 'invalid' WHERE id = '1'",
        "UPDATE diarization_settings SET overlap_handling = 'speaker_ids' WHERE id = '1'",
    ] {
        assert!(
            sqlx::query(statement).execute(&pool).await.is_err(),
            "accepted {statement}"
        );
    }

    for statement in [
        "INSERT INTO meeting_diarization_status (meeting_id, enabled) VALUES ('meeting-legacy', 2)",
        "INSERT INTO meeting_diarization_status (meeting_id, live_enabled) VALUES ('meeting-legacy', 2)",
        "INSERT INTO meeting_diarization_status (meeting_id, post_call_enabled) VALUES ('meeting-legacy', 2)",
        "INSERT INTO meeting_diarization_status (meeting_id, current_status) VALUES ('meeting-legacy', 'invalid')",
        "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, is_overlap, diarization_status) VALUES ('bad-time', 'meeting-legacy', 'system', 1.0, 1.0, 0, 'final')",
        "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, confidence, is_overlap, diarization_status) VALUES ('bad-low-confidence', 'meeting-legacy', 'system', 0.0, 1.0, -0.01, 0, 'final')",
        "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, confidence, is_overlap, diarization_status) VALUES ('bad-high-confidence', 'meeting-legacy', 'system', 0.0, 1.0, 1.01, 0, 'final')",
        "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, is_overlap, diarization_status) VALUES ('bad-overlap', 'meeting-legacy', 'system', 0.0, 1.0, 2, 'final')",
        "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, is_overlap, diarization_status) VALUES ('bad-status', 'meeting-legacy', 'system', 0.0, 1.0, 0, 'invalid')",
    ] {
        assert!(sqlx::query(statement).execute(&pool).await.is_err(), "accepted {statement}");
    }
}

#[tokio::test]
async fn immutable_legacy_diarization_database_upgrades_without_checksum_or_data_loss() {
    assert_eq!(
        migration_checksum(LEGACY_DIARIZATION_MIGRATION),
        LEGACY_DIARIZATION_SHA384
    );

    for foreign_keys in [true, false] {
        let pool = database(foreign_keys).await;
        legacy_diarization_migrator().run(&pool).await.unwrap();

        // Legacy databases can contain values written before persistence constraints existed.
        // Disable FK enforcement only while seeding an orphan fixture, then restore the
        // requested mode before the real hardening migrator runs.
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at, folder_path)
             VALUES ('meeting-historical', 'Historical title', 'created', 'updated', '/recordings/historical')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transcripts (
                id, meeting_id, transcript, timestamp, summary, action_items, key_points,
                audio_start_time, audio_end_time, duration, speaker,
                speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
             ) VALUES (
                'transcript-historical', 'meeting-historical', 'Never lose this transcript',
                'wall-clock', 'summary', 'actions', 'key points', 4.0, 7.0, 3.0, 'mic',
                'stale-speaker', 'Stale speaker', '#123456', 7, 'invalid', 'legacy', 1.5
             )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transcripts (
                id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration,
                speaker, speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
             ) VALUES (
                'transcript-overlap', 'meeting-historical', 'Preserve overlap transcript',
                'overlap-clock', 7.0, 8.0, 1.0, 'system',
                'concrete-identity', 'Concrete identity', '#badbad', 1,
                'final', 'legacy-overlap', 0.8
             )",
        )
        .execute(&pool)
        .await
        .unwrap();

        for statement in [
            "UPDATE diarization_settings SET enabled = 7, mode = 'invalid', show_provisional_labels = 7, post_call_refinement_enabled = 7, overlap_handling = 'identities', speaker_review_enabled = 7 WHERE id = '1'",
            "INSERT INTO meeting_diarization_status (meeting_id, enabled, live_enabled, post_call_enabled, current_status, quality_flags) VALUES ('meeting-historical', 7, 7, 7, 'invalid', 'keep-flags')",
            "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, speaker_id, speaker_label, confidence, is_overlap, diarization_status, diarization_method) VALUES ('segment-normalize', 'meeting-historical', 'system', 0.0, 1.0, 'speaker-1', 'Speaker 1', 2.0, 7, 'invalid', 'legacy')",
            "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, speaker_id, speaker_label, confidence, is_overlap, diarization_status, diarization_method) VALUES ('segment-overlap', 'meeting-historical', 'system', 1.0, 2.0, 'speaker-2', 'Speaker 2', 0.8, 1, 'final', 'legacy')",
            "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, confidence, is_overlap, diarization_status) VALUES ('segment-malformed', 'meeting-historical', 'system', 2.0, 2.0, 0.8, 0, 'final')",
            "INSERT INTO speaker_segments (id, meeting_id, source, start_time, end_time, confidence, is_overlap, diarization_status) VALUES ('segment-orphan', 'missing-meeting', 'system', 0.0, 1.0, 0.8, 0, 'final')",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }

        if foreign_keys {
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&pool)
                .await
                .unwrap();
        }

        MIGRATOR.run(&pool).await.unwrap();

        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM _sqlx_migrations WHERE version = ?",
            )
            .bind(HARDENED_DIARIZATION_MIGRATION)
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );

        let transcript = sqlx::query(
            "SELECT transcript, timestamp, summary, action_items, key_points,
                    audio_start_time, audio_end_time, duration, speaker,
                    speaker_id, speaker_label, speaker_color, is_overlap,
                    diarization_status, diarization_method, diarization_confidence
             FROM transcripts WHERE id = 'transcript-historical'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            transcript.get::<String, _>("transcript"),
            "Never lose this transcript"
        );
        assert_eq!(transcript.get::<String, _>("timestamp"), "wall-clock");
        assert_eq!(
            transcript.get::<Option<String>, _>("summary").as_deref(),
            Some("summary")
        );
        assert_eq!(
            transcript
                .get::<Option<String>, _>("action_items")
                .as_deref(),
            Some("actions")
        );
        assert_eq!(
            transcript.get::<Option<String>, _>("key_points").as_deref(),
            Some("key points")
        );
        assert_eq!(
            transcript.get::<Option<f64>, _>("audio_start_time"),
            Some(4.0)
        );
        assert_eq!(
            transcript.get::<Option<f64>, _>("audio_end_time"),
            Some(7.0)
        );
        assert_eq!(transcript.get::<Option<f64>, _>("duration"), Some(3.0));
        assert_eq!(
            transcript.get::<Option<String>, _>("speaker").as_deref(),
            Some("mic")
        );
        assert_eq!(transcript.get::<Option<String>, _>("speaker_id"), None);
        assert_eq!(transcript.get::<Option<String>, _>("speaker_label"), None);
        assert_eq!(transcript.get::<Option<String>, _>("speaker_color"), None);
        assert_eq!(transcript.get::<i64, _>("is_overlap"), 0);
        assert_eq!(transcript.get::<String, _>("diarization_status"), "none");
        assert_eq!(
            transcript
                .get::<Option<String>, _>("diarization_method")
                .as_deref(),
            Some("legacy")
        );
        assert_eq!(
            transcript.get::<Option<f64>, _>("diarization_confidence"),
            None
        );

        let overlap_transcript = sqlx::query(
            "SELECT transcript, timestamp, speaker, speaker_id, speaker_label, speaker_color,
                    is_overlap, diarization_status, diarization_method, diarization_confidence
             FROM transcripts WHERE id = 'transcript-overlap'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            overlap_transcript.get::<String, _>("transcript"),
            "Preserve overlap transcript"
        );
        assert_eq!(
            overlap_transcript.get::<String, _>("timestamp"),
            "overlap-clock"
        );
        assert_eq!(
            overlap_transcript
                .get::<Option<String>, _>("speaker")
                .as_deref(),
            Some("system")
        );
        assert_eq!(
            overlap_transcript.get::<Option<String>, _>("speaker_id"),
            None
        );
        assert_eq!(
            overlap_transcript
                .get::<Option<String>, _>("speaker_label")
                .as_deref(),
            Some("Multiple speakers")
        );
        assert_eq!(
            overlap_transcript
                .get::<Option<String>, _>("speaker_color")
                .as_deref(),
            Some("#f97316")
        );
        assert_eq!(overlap_transcript.get::<i64, _>("is_overlap"), 1);
        assert_eq!(
            overlap_transcript.get::<String, _>("diarization_status"),
            "final"
        );
        assert_eq!(
            overlap_transcript
                .get::<Option<String>, _>("diarization_method")
                .as_deref(),
            Some("legacy-overlap")
        );
        assert_eq!(
            overlap_transcript.get::<Option<f64>, _>("diarization_confidence"),
            Some(0.8)
        );

        let settings = sqlx::query(
            "SELECT enabled, mode, show_provisional_labels, post_call_refinement_enabled,
                    overlap_handling, speaker_review_enabled
             FROM diarization_settings WHERE id = '1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(settings.get::<i64, _>("enabled"), 0);
        assert_eq!(settings.get::<String, _>("mode"), "off");
        assert_eq!(settings.get::<i64, _>("show_provisional_labels"), 1);
        assert_eq!(settings.get::<i64, _>("post_call_refinement_enabled"), 1);
        assert_eq!(
            settings.get::<String, _>("overlap_handling"),
            "multiple_speakers"
        );
        assert_eq!(settings.get::<i64, _>("speaker_review_enabled"), 1);

        let status = sqlx::query(
            "SELECT enabled, live_enabled, post_call_enabled, current_status, quality_flags
             FROM meeting_diarization_status WHERE meeting_id = 'meeting-historical'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status.get::<i64, _>("enabled"), 0);
        assert_eq!(status.get::<i64, _>("live_enabled"), 0);
        assert_eq!(status.get::<i64, _>("post_call_enabled"), 0);
        assert_eq!(status.get::<String, _>("current_status"), "none");
        assert_eq!(
            status.get::<Option<String>, _>("quality_flags").as_deref(),
            Some("keep-flags")
        );

        let segment_ids =
            sqlx::query_scalar::<_, String>("SELECT id FROM speaker_segments ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(segment_ids, vec!["segment-normalize", "segment-overlap"]);

        let normalized = sqlx::query(
            "SELECT confidence, is_overlap, diarization_status
             FROM speaker_segments WHERE id = 'segment-normalize'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(normalized.get::<Option<f64>, _>("confidence"), None);
        assert_eq!(normalized.get::<i64, _>("is_overlap"), 0);
        assert_eq!(normalized.get::<String, _>("diarization_status"), "none");

        let overlap = sqlx::query(
            "SELECT speaker_id, speaker_label, is_overlap
             FROM speaker_segments WHERE id = 'segment-overlap'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(overlap.get::<Option<String>, _>("speaker_id"), None);
        assert_eq!(overlap.get::<Option<String>, _>("speaker_label"), None);
        assert_eq!(overlap.get::<i64, _>("is_overlap"), 1);

        for statement in [
            "UPDATE transcripts SET is_overlap = 2 WHERE id = 'transcript-historical'",
            "UPDATE transcripts SET diarization_status = 'invalid' WHERE id = 'transcript-historical'",
            "UPDATE transcripts SET diarization_confidence = 1.5 WHERE id = 'transcript-historical'",
            "UPDATE diarization_settings SET enabled = 2 WHERE id = '1'",
            "UPDATE diarization_settings SET mode = 'invalid' WHERE id = '1'",
            "UPDATE diarization_settings SET overlap_handling = 'invalid' WHERE id = '1'",
            "UPDATE meeting_diarization_status SET current_status = 'invalid' WHERE meeting_id = 'meeting-historical'",
            "UPDATE speaker_segments SET end_time = start_time WHERE id = 'segment-normalize'",
            "UPDATE speaker_segments SET confidence = -1.0 WHERE id = 'segment-normalize'",
            "UPDATE speaker_segments SET is_overlap = 2 WHERE id = 'segment-normalize'",
            "UPDATE speaker_segments SET diarization_status = 'invalid' WHERE id = 'segment-normalize'",
        ] {
            assert!(sqlx::query(statement).execute(&pool).await.is_err(), "accepted {statement}");
        }
    }
}

#[tokio::test]
async fn cleanup_clears_only_diarization_metadata_with_foreign_keys_on_and_off() {
    for foreign_keys in [true, false] {
        let pool = database(foreign_keys).await;
        MIGRATOR.run(&pool).await.unwrap();
        seed_diarization(&pool, "meeting-cleanup", "transcript-cleanup").await;

        let mut connection = pool.acquire().await.unwrap();
        let mut transaction = connection.begin().await.unwrap();
        clear_meeting_diarization(&mut *transaction, "meeting-cleanup")
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        drop(connection);

        assert_diarization_rows(&pool, "meeting-cleanup", 0).await;
        let row = sqlx::query(
            "SELECT speaker, speaker_id, speaker_label, speaker_color, is_overlap,
                    diarization_status, diarization_method, diarization_confidence
             FROM transcripts WHERE id = 'transcript-cleanup'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row.get::<Option<String>, _>("speaker").as_deref(),
            Some("mic")
        );
        assert_eq!(row.get::<Option<String>, _>("speaker_id"), None);
        assert_eq!(row.get::<Option<String>, _>("speaker_label"), None);
        assert_eq!(row.get::<Option<String>, _>("speaker_color"), None);
        assert_eq!(row.get::<i64, _>("is_overlap"), 0);
        assert_eq!(row.get::<String, _>("diarization_status"), "none");
        assert_eq!(row.get::<Option<String>, _>("diarization_method"), None);
        assert_eq!(row.get::<Option<f64>, _>("diarization_confidence"), None);
    }
}

#[tokio::test]
async fn meeting_deletion_explicitly_removes_diarization_with_foreign_keys_on_and_off() {
    for foreign_keys in [true, false] {
        let pool = database(foreign_keys).await;
        MIGRATOR.run(&pool).await.unwrap();
        seed_diarization(&pool, "meeting-delete", "transcript-delete").await;

        assert!(MeetingsRepository::delete_meeting(&pool, "meeting-delete")
            .await
            .unwrap());

        assert_diarization_rows(&pool, "meeting-delete", 0).await;
        for table in ["meetings", "transcripts"] {
            let count = sqlx::query_scalar::<_, i64>(&format!(
                "SELECT COUNT(*) FROM {table} WHERE {} = ?",
                if table == "meetings" {
                    "id"
                } else {
                    "meeting_id"
                }
            ))
            .bind("meeting-delete")
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(count, 0, "unexpected {table} row remains");
        }
    }
}

#[tokio::test]
async fn retranscription_replaces_transcripts_and_diarization_with_foreign_keys_on_and_off() {
    for foreign_keys in [true, false] {
        let pool = database(foreign_keys).await;
        MIGRATOR.run(&pool).await.unwrap();
        seed_diarization(&pool, "meeting-retranscribe", "transcript-old").await;

        let mut replacement = replacement_segment("transcript-new");
        replacement.speaker_id = Some("stale-speaker".to_string());
        replacement.speaker_label = Some("Stale speaker".to_string());
        replacement.speaker_color = Some("#abcdef".to_string());
        replacement.is_overlap = Some(true);
        replacement.diarization_status = Some("final".to_string());
        replacement.diarization_method = Some("stale".to_string());
        replacement.diarization_confidence = Some(0.99);

        let mut connection = pool.acquire().await.unwrap();
        let mut transaction = connection.begin().await.unwrap();
        TranscriptsRepository::replace_for_retranscription(
            &mut transaction,
            "meeting-retranscribe",
            &[replacement],
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        drop(connection);

        assert_diarization_rows(&pool, "meeting-retranscribe", 0).await;
        let rows = sqlx::query(
            "SELECT id, speaker, speaker_id, is_overlap, diarization_status
             FROM transcripts WHERE meeting_id = ?",
        )
        .bind("meeting-retranscribe")
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<String, _>("id"), "transcript-new");
        assert_eq!(rows[0].get::<Option<String>, _>("speaker"), None);
        assert_eq!(rows[0].get::<Option<String>, _>("speaker_id"), None);
        assert_eq!(rows[0].get::<i64, _>("is_overlap"), 0);
        assert_eq!(rows[0].get::<String, _>("diarization_status"), "none");
    }
}

#[tokio::test]
async fn failed_retranscription_transaction_preserves_existing_diarization() {
    let pool = database(true).await;
    MIGRATOR.run(&pool).await.unwrap();
    seed_diarization(&pool, "meeting-rollback", "transcript-original").await;
    sqlx::query(
        "CREATE TRIGGER reject_replacement
         BEFORE INSERT ON transcripts
         WHEN NEW.id = 'transcript-rejected'
         BEGIN
             SELECT RAISE(ABORT, 'forced replacement failure');
         END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut connection = pool.acquire().await.unwrap();
    let mut transaction = connection.begin().await.unwrap();
    let result = TranscriptsRepository::replace_for_retranscription(
        &mut transaction,
        "meeting-rollback",
        &[replacement_segment("transcript-rejected")],
    )
    .await;
    assert!(result.is_err());
    transaction.rollback().await.unwrap();
    drop(connection);

    assert_diarization_rows(&pool, "meeting-rollback", 1).await;
    let row = sqlx::query(
        "SELECT id, speaker, speaker_id, diarization_status
         FROM transcripts WHERE meeting_id = 'meeting-rollback'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("id"), "transcript-original");
    assert_eq!(
        row.get::<Option<String>, _>("speaker").as_deref(),
        Some("mic")
    );
    assert_eq!(
        row.get::<Option<String>, _>("speaker_id").as_deref(),
        Some("speaker-1")
    );
    assert_eq!(row.get::<String, _>("diarization_status"), "final");
}

#[tokio::test]
async fn transcript_repository_persists_diarization_dto_fields() {
    let pool = database(true).await;
    MIGRATOR.run(&pool).await.unwrap();
    let segment = TranscriptSegment {
        speaker_id: Some("speaker-1".to_string()),
        speaker_label: Some("Speaker 1".to_string()),
        speaker_color: Some("#123456".to_string()),
        is_overlap: Some(false),
        diarization_status: Some("provisional".to_string()),
        diarization_method: Some("repository_test".to_string()),
        diarization_confidence: Some(0.75),
        ..replacement_segment("ignored-by-save")
    };

    let meeting_id =
        TranscriptsRepository::save_transcript(&pool, "Repository test", &[segment], None)
            .await
            .unwrap();

    let row = sqlx::query(
        "SELECT speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
         FROM transcripts WHERE meeting_id = ?",
    )
    .bind(meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        row.get::<Option<String>, _>("speaker_id").as_deref(),
        Some("speaker-1")
    );
    assert_eq!(
        row.get::<Option<String>, _>("speaker_label").as_deref(),
        Some("Speaker 1")
    );
    assert_eq!(
        row.get::<Option<String>, _>("speaker_color").as_deref(),
        Some("#123456")
    );
    assert_eq!(row.get::<i64, _>("is_overlap"), 0);
    assert_eq!(row.get::<String, _>("diarization_status"), "provisional");
    assert_eq!(
        row.get::<Option<String>, _>("diarization_method")
            .as_deref(),
        Some("repository_test")
    );
    assert_eq!(
        row.get::<Option<f64>, _>("diarization_confidence"),
        Some(0.75)
    );
}
