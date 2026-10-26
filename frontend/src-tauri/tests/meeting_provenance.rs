use app_lib::api::TranscriptSegment;
use app_lib::database::manager::DatabaseManager;
use app_lib::database::repositories::meeting::{
    MeetingsRepository, LIFECYCLE_OPERATION_RECORDING, LIFECYCLE_OPERATION_RETRANSCRIPTION,
    LIFECYCLE_OPERATION_SUMMARY, MEETING_STATUS_ERROR, MEETING_STATUS_PROCESSING,
    MEETING_STATUS_READY,
};
use app_lib::database::repositories::summary::{
    SummaryProcessesRepository, SummaryProcessingMetrics,
};
use app_lib::database::repositories::transcript::TranscriptsRepository;
use app_lib::summary::service::SummaryService;
use chrono::{DateTime, Utc};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::sync::Arc;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

const PRE_PROVENANCE_MIGRATIONS: &[&str] = &[
    include_str!("../migrations/20250916100000_initial_schema.sql"),
    include_str!("../migrations/20250920155811_add_openrouter_api_key.sql"),
    include_str!("../migrations/20251006000000_add_audio_sync_fields.sql"),
    include_str!("../migrations/20251010153942_add_ollama_endpoint.sql"),
    include_str!("../migrations/20251101000000_add_summary_backup.sql"),
    include_str!("../migrations/20251105120000_add_pro_license_custom_openai.sql"),
    include_str!("../migrations/20251110000000_add_grace_period_to_licensing.sql"),
    include_str!("../migrations/20251110000001_add_speaker_field.sql"),
    include_str!("../migrations/20251223000000_add_meeting_notes.sql"),
    include_str!("../migrations/20251229000000_add_gemini_api_key.sql"),
    include_str!("../migrations/20260626000100_add_diarization_foundation.sql"),
    include_str!("../migrations/20260713000100_harden_diarization_foundation.sql"),
    include_str!("../migrations/20260908000100_add_transcription_vocabulary.sql"),
];

async fn open_pool(path: &Path) -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap()
}

fn segment(text: &str) -> TranscriptSegment {
    TranscriptSegment {
        id: "transcript-1".into(),
        text: text.into(),
        timestamp: "2026-09-08T09:00:00Z".into(),
        audio_start_time: Some(0.0),
        audio_end_time: Some(1.0),
        duration: Some(1.0),
        speaker_id: None,
        speaker_label: None,
        speaker_color: None,
        is_overlap: Some(false),
        diarization_status: Some("none".into()),
        diarization_method: None,
        diarization_confidence: None,
    }
}

#[tokio::test]
async fn manual_title_and_recording_time_survive_reopen_summary_and_retry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("meeting.sqlite");
    let started_at_ms = 1_725_000_123_456_u64;

    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = TranscriptsRepository::save_transcript_with_started_at(
        &pool,
        "Suggested title",
        &[segment("first")],
        Some("/tmp/recording".into()),
        Some(started_at_ms),
    )
    .await
    .unwrap();

    let initial = sqlx::query(
        "SELECT title, title_source, started_at_ms, created_at
         FROM meetings WHERE id = ?",
    )
    .bind(&meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(initial.get::<String, _>("title"), "Suggested title");
    assert_eq!(initial.get::<String, _>("title_source"), "suggested");
    assert_eq!(initial.get::<i64, _>("started_at_ms"), started_at_ms as i64);
    assert_eq!(
        initial
            .get::<DateTime<Utc>, _>("created_at")
            .timestamp_millis(),
        started_at_ms as i64
    );

    pool.close().await;
    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();

    assert!(
        MeetingsRepository::update_meeting_title(&pool, &meeting_id, "User title")
            .await
            .unwrap()
    );
    // A late automatic summary sees the manual provenance and becomes a no-op.
    assert!(MeetingsRepository::update_suggested_meeting_name(
        &pool,
        &meeting_id,
        "Late summary title"
    )
    .await
    .unwrap());

    // Retrying with a different title or processing timestamp cannot overwrite
    // the row that was already committed.
    let retried = TranscriptsRepository::save_transcript_with_id_and_started_at(
        &pool,
        &meeting_id,
        "Retry title",
        &[segment("retry")],
        Some("/tmp/other-recording".into()),
        Some(started_at_ms + 99_000),
    )
    .await
    .unwrap();
    assert_eq!(retried, meeting_id);

    let final_row = sqlx::query(
        "SELECT title, title_source, started_at_ms, created_at,
                (SELECT COUNT(*) FROM transcripts WHERE meeting_id = meetings.id) AS transcript_count
         FROM meetings WHERE id = ?",
    )
    .bind(&meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(final_row.get::<String, _>("title"), "User title");
    assert_eq!(final_row.get::<String, _>("title_source"), "manual");
    assert_eq!(
        final_row.get::<i64, _>("started_at_ms"),
        started_at_ms as i64
    );
    assert_eq!(
        final_row
            .get::<DateTime<Utc>, _>("created_at")
            .timestamp_millis(),
        started_at_ms as i64
    );
    assert_eq!(final_row.get::<i64, _>("transcript_count"), 1);
}

#[tokio::test]
async fn suggested_title_updates_before_manual_title_is_set() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id =
        TranscriptsRepository::save_transcript(&pool, "Initial title", &[segment("text")], None)
            .await
            .unwrap();

    assert!(MeetingsRepository::update_suggested_meeting_name(
        &pool,
        &meeting_id,
        "Suggested from summary"
    )
    .await
    .unwrap());
    let title = sqlx::query_scalar::<_, String>("SELECT title FROM meetings WHERE id = ?")
        .bind(&meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Suggested from summary");
}

#[tokio::test]
async fn provenance_upgrade_preserves_titles_with_unknown_legacy_source() {
    let pool = open_pool(Path::new(":memory:")).await;
    for migration in PRE_PROVENANCE_MIGRATIONS {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at, folder_path)
         VALUES ('legacy-title', 'User title before upgrade', '2026-01-01T00:00:00Z',
                 '2026-01-01T00:00:00Z', '/recordings/legacy-title')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!(
        "../migrations/20260908000200_add_meeting_provenance.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/20260913000100_add_meeting_lifecycle.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    // Current repositories run after startup has applied the search migration.
    // Keep the legacy-title fixture, then advance it to the complete current schema.
    sqlx::raw_sql(include_str!(
        "../migrations/20260926000000_add_meeting_search_index.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        sqlx::query_as::<_, (String, String)>(
            "SELECT title, title_source FROM meetings WHERE id = 'legacy-title'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        (
            "User title before upgrade".to_string(),
            "legacy".to_string()
        )
    );

    // Automatic summary output is a no-op for a legacy row whose provenance
    // is unknown. New rows created by the current save path are explicit
    // suggested rows and remain eligible for automatic naming.
    assert!(MeetingsRepository::update_suggested_meeting_name(
        &pool,
        "legacy-title",
        "AI replacement",
    )
    .await
    .unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT title FROM meetings WHERE id = 'legacy-title'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "User title before upgrade"
    );

    let new_id =
        TranscriptsRepository::save_transcript(&pool, "Generated title", &[segment("text")], None)
            .await
            .unwrap();
    assert!(
        MeetingsRepository::update_suggested_meeting_name(&pool, &new_id, "AI generated")
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT title FROM meetings WHERE id = ?")
            .bind(new_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "AI generated"
    );
}

#[tokio::test]
async fn lifecycle_upgrade_marks_pending_legacy_summary_as_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy-summary.sqlite");
    let pool = open_pool(&path).await;
    for migration in PRE_PROVENANCE_MIGRATIONS {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at, folder_path)
         VALUES ('legacy-summary', 'Legacy summary', '2026-01-01T00:00:00Z',
                 '2026-01-01T00:00:00Z', '/recordings/legacy-summary')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO summary_processes (
             meeting_id, status, created_at, updated_at, result, result_backup
         ) VALUES (?, 'PENDING', ?, ?, NULL, ?)",
    )
    .bind("legacy-summary")
    .bind("2026-01-01T00:00:00Z")
    .bind("2026-01-01T00:00:00Z")
    .bind(r#"{"markdown":"legacy summary"}"#)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = open_pool(&path).await;
    sqlx::raw_sql(include_str!(
        "../migrations/20260908000200_add_meeting_provenance.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/20260913000100_add_meeting_lifecycle.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let lifecycle = sqlx::query(
        "SELECT title, status, lifecycle_error, started_at_ms
         FROM meetings WHERE id = 'legacy-summary'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lifecycle.get::<String, _>("title"), "Legacy summary");
    assert_eq!(lifecycle.get::<String, _>("status"), MEETING_STATUS_ERROR);
    assert_eq!(
        lifecycle.get::<String, _>("lifecycle_error"),
        app_lib::database::repositories::meeting::INTERRUPTED_MEETING_ERROR
    );
    assert!(lifecycle
        .try_get::<Option<i64>, _>("started_at_ms")
        .unwrap()
        .is_none());

    assert_eq!(
        SummaryProcessesRepository::recover_interrupted_processes(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        MeetingsRepository::recover_interrupted_lifecycle(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT result FROM summary_processes WHERE meeting_id = 'legacy-summary'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        r#"{"markdown":"legacy summary"}"#
    );
}

#[tokio::test]
async fn lifecycle_reopen_marks_ownerless_work_error_and_preserves_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("meeting.sqlite");
    let started_at_ms = 1_725_000_123_456_i64;
    let meeting_id = "meeting-lifecycle-reopen";

    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Stable lifecycle title",
        Some(started_at_ms),
        "suggested",
        MEETING_STATUS_PROCESSING,
        LIFECYCLE_OPERATION_SUMMARY,
    )
    .await
    .unwrap();
    TranscriptsRepository::save_transcript_with_id(
        &pool,
        meeting_id,
        "Retry title must not win",
        &[segment("preserved transcript")],
        Some("/recordings/lifecycle".into()),
    )
    .await
    .unwrap();
    SummaryProcessesRepository::create_or_reset_process(&pool, meeting_id)
        .await
        .unwrap();
    sqlx::query("UPDATE meetings SET duration_seconds = 42.5 WHERE id = ?")
        .bind(meeting_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE summary_processes
         SET result = NULL, result_backup = ?
         WHERE meeting_id = ?",
    )
    .bind(r#"{"markdown":"old summary"}"#)
    .bind(meeting_id)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();
    SummaryProcessesRepository::recover_interrupted_processes(&pool)
        .await
        .unwrap();
    assert_eq!(
        MeetingsRepository::recover_interrupted_lifecycle(&pool)
            .await
            .unwrap(),
        1
    );

    let row = sqlx::query(
        "SELECT title, started_at_ms, duration_seconds, status, lifecycle_error,
                lifecycle_operation
         FROM meetings WHERE id = ?",
    )
    .bind(meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("title"), "Stable lifecycle title");
    assert_eq!(row.get::<i64, _>("started_at_ms"), started_at_ms);
    assert_eq!(row.get::<f64, _>("duration_seconds"), 42.5);
    assert_eq!(row.get::<String, _>("status"), MEETING_STATUS_ERROR);
    assert_eq!(
        row.get::<String, _>("lifecycle_error"),
        app_lib::database::repositories::meeting::INTERRUPTED_MEETING_ERROR
    );
    assert!(row
        .try_get::<Option<String>, _>("lifecycle_operation")
        .unwrap()
        .is_none());
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT transcript FROM transcripts WHERE meeting_id = ?",)
            .bind(meeting_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "preserved transcript"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT result FROM summary_processes WHERE meeting_id = ?",
        )
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        r#"{"markdown":"old summary"}"#
    );
}

#[tokio::test]
async fn lifecycle_owner_blocks_stale_summary_and_duplicate_transcript_updates() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = "meeting-lifecycle-owner";
    let summary_run_id = MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Owner test",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_PROCESSING,
        LIFECYCLE_OPERATION_SUMMARY,
    )
    .await
    .unwrap();

    let retranscription_run_id = MeetingsRepository::claim_lifecycle_operation(
        &pool,
        meeting_id,
        LIFECYCLE_OPERATION_RETRANSCRIPTION,
    )
    .await
    .unwrap()
    .expect("retranscription should claim the processing row");
    assert!(!MeetingsRepository::update_lifecycle_for_operation(
        &pool,
        meeting_id,
        LIFECYCLE_OPERATION_SUMMARY,
        &summary_run_id,
        "ready",
        None,
        None,
    )
    .await
    .unwrap());
    TranscriptsRepository::save_transcript_with_id(
        &pool,
        meeting_id,
        "Late duplicate",
        &[segment("new transcript")],
        None,
    )
    .await
    .unwrap();

    let processing_row =
        sqlx::query("SELECT status, lifecycle_operation FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        processing_row.get::<String, _>("status"),
        MEETING_STATUS_PROCESSING
    );
    assert_eq!(
        processing_row.get::<String, _>("lifecycle_operation"),
        LIFECYCLE_OPERATION_RETRANSCRIPTION
    );

    assert!(MeetingsRepository::update_lifecycle_for_operation(
        &pool,
        meeting_id,
        LIFECYCLE_OPERATION_RETRANSCRIPTION,
        &retranscription_run_id,
        MEETING_STATUS_ERROR,
        None,
        Some("retranscription failed"),
    )
    .await
    .unwrap());
    TranscriptsRepository::save_transcript_with_id(
        &pool,
        meeting_id,
        "Late duplicate after failure",
        &[segment("still preserved")],
        None,
    )
    .await
    .unwrap();
    let error_row = sqlx::query("SELECT status, lifecycle_error FROM meetings WHERE id = ?")
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(error_row.get::<String, _>("status"), MEETING_STATUS_ERROR);
    assert_eq!(
        error_row.get::<String, _>("lifecycle_error"),
        "retranscription failed"
    );

    // The summary result belongs to the superseded run. A terminal error from
    // the newer retranscription still retains its owner, so the late result
    // cannot turn the meeting back to ready.
    assert!(!MeetingsRepository::update_lifecycle_for_operation(
        &pool,
        meeting_id,
        LIFECYCLE_OPERATION_SUMMARY,
        &summary_run_id,
        MEETING_STATUS_READY,
        None,
        None,
    )
    .await
    .unwrap());
    let error_row = sqlx::query(
        "SELECT status, lifecycle_operation, lifecycle_run_id FROM meetings WHERE id = ?",
    )
    .bind(meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(error_row.get::<String, _>("status"), MEETING_STATUS_ERROR);
    assert_eq!(
        error_row.get::<String, _>("lifecycle_operation"),
        LIFECYCLE_OPERATION_RETRANSCRIPTION
    );
    assert_eq!(
        error_row.get::<String, _>("lifecycle_run_id"),
        retranscription_run_id
    );
}

#[tokio::test]
async fn first_transcript_save_fills_ready_recording_placeholder_once() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ready-placeholder.sqlite");
    let meeting_id = "meeting-ready-placeholder";
    let started_at_ms = 1_725_000_123_456_i64;

    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();
    let recording_run_id = MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Recording placeholder",
        Some(started_at_ms),
        "suggested",
        "recording",
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();
    assert!(MeetingsRepository::update_lifecycle_for_operation(
        &pool,
        meeting_id,
        LIFECYCLE_OPERATION_RECORDING,
        &recording_run_id,
        MEETING_STATUS_READY,
        Some(12.5),
        None,
    )
    .await
    .unwrap());

    TranscriptsRepository::save_transcript_with_id_and_started_at_and_vocabulary(
        &pool,
        meeting_id,
        "Retry title must not replace the placeholder title",
        &[segment("first persisted transcript")],
        Some("/recordings/placeholder".into()),
        Some(started_at_ms as u64),
        Some("Acme; roadmap"),
    )
    .await
    .unwrap();

    // Repeating the same save after the first commit is an idempotent no-op.
    TranscriptsRepository::save_transcript_with_id_and_started_at_and_vocabulary(
        &pool,
        meeting_id,
        "A duplicate title",
        &[segment("duplicate must not be inserted")],
        Some("/recordings/other".into()),
        Some((started_at_ms + 999) as u64),
        Some("different vocabulary"),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?",)
            .bind(meeting_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );

    pool.close().await;
    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();
    let row = sqlx::query(
        "SELECT title, started_at_ms, status, lifecycle_operation, duration_seconds
         FROM meetings WHERE id = ?",
    )
    .bind(meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("title"), "Recording placeholder");
    assert_eq!(row.get::<i64, _>("started_at_ms"), started_at_ms);
    assert_eq!(row.get::<String, _>("status"), MEETING_STATUS_READY);
    assert_eq!(
        row.get::<String, _>("lifecycle_operation"),
        LIFECYCLE_OPERATION_RECORDING
    );
    assert_eq!(row.get::<f64, _>("duration_seconds"), 12.5);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT transcript FROM transcripts WHERE meeting_id = ?",)
            .bind(meeting_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "first persisted transcript"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT vocabulary FROM meeting_transcript_vocabulary WHERE meeting_id = ?",
        )
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "Acme; roadmap"
    );
}

#[tokio::test]
async fn summary_terminal_update_requires_the_current_run_id() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = "meeting-summary-run-owner";
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Summary run owner",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();

    let first_run_id = SummaryProcessesRepository::create_or_reset_process(&pool, meeting_id)
        .await
        .unwrap();
    let second_run_id = SummaryProcessesRepository::create_or_reset_process(&pool, meeting_id)
        .await
        .unwrap();
    assert_ne!(first_run_id, second_run_id);

    let stale_result = SummaryProcessesRepository::update_process_completed_for_run(
        &pool,
        meeting_id,
        &first_run_id,
        serde_json::json!({ "markdown": "stale" }),
        1,
        0.1,
    )
    .await;
    assert!(matches!(stale_result, Err(sqlx::Error::RowNotFound)));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM summary_processes WHERE meeting_id = ?",
        )
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "PENDING"
    );

    SummaryProcessesRepository::update_process_failed_for_run(
        &pool,
        meeting_id,
        &second_run_id,
        "current run failed",
    )
    .await
    .unwrap();
    let row = sqlx::query(
        "SELECT status, lifecycle_operation, lifecycle_run_id, lifecycle_error
         FROM meetings WHERE id = ?",
    )
    .bind(meeting_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("status"), MEETING_STATUS_ERROR);
    assert_eq!(
        row.get::<String, _>("lifecycle_operation"),
        LIFECYCLE_OPERATION_SUMMARY
    );
    assert_eq!(row.get::<String, _>("lifecycle_run_id"), second_run_id);
    assert_eq!(
        row.get::<String, _>("lifecycle_error"),
        "current run failed"
    );
}

#[tokio::test]
async fn delayed_fake_provider_result_cannot_replace_a_newer_summary_attempt() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("meeting.sqlite");
    let recording_folder = directory.path().join("recording");
    std::fs::create_dir(&recording_folder).unwrap();
    std::fs::write(
        recording_folder.join("summary.md"),
        "previous accepted summary",
    )
    .unwrap();
    let pool = open_pool(&database_path).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = "meeting-delayed-summary-a-b";
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Delayed summary",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE meetings SET folder_path = ? WHERE id = ?")
        .bind(recording_folder.to_string_lossy().as_ref())
        .bind(meeting_id)
        .execute(&pool)
        .await
        .unwrap();

    let (attempt_a, _) = SummaryProcessesRepository::start_process(&pool, meeting_id)
        .await
        .unwrap();
    let release_provider_a = Arc::new(tokio::sync::Barrier::new(2));
    let (provider_started, wait_for_provider) = tokio::sync::oneshot::channel();
    let delayed_pool = pool.clone();
    let delayed_meeting_id = meeting_id.to_string();
    let delayed_attempt_a = attempt_a.clone();
    let provider_gate = release_provider_a.clone();
    let fake_provider_a = tokio::spawn(async move {
        provider_started.send(()).unwrap();
        provider_gate.wait().await;
        SummaryService::accept_generated_summary(
            &delayed_pool,
            &delayed_meeting_id,
            &delayed_attempt_a,
            serde_json::json!({ "markdown": "stale result from A" }),
            "stale result from A",
            SummaryProcessingMetrics {
                chunk_count: 1,
                processing_time: 0.1,
            },
            None,
        )
        .await
    });
    wait_for_provider.await.unwrap();

    // While the fake provider for A is paused, a repeated request replaces A
    // with B. Releasing A later must not change B's canonical database result.
    let (attempt_b, replaced) = SummaryProcessesRepository::start_process(&pool, meeting_id)
        .await
        .unwrap();
    assert_eq!(replaced.as_deref(), Some(attempt_a.as_str()));
    let canonical_export = "# Delayed summary\n\n## Decisions\nKeep the accepted result.";
    assert!(SummaryService::accept_generated_summary(
        &pool,
        meeting_id,
        &attempt_b,
        serde_json::json!({
            "markdown": "## Decisions\nKeep the accepted result.",
            "export_markdown": canonical_export,
        }),
        canonical_export,
        SummaryProcessingMetrics {
            chunk_count: 1,
            processing_time: 0.2
        },
        None,
    )
    .await
    .unwrap());
    SummaryService::reconcile_accepted_summary(&pool, meeting_id)
        .await
        .unwrap();
    release_provider_a.wait().await;
    assert!(!fake_provider_a.await.unwrap().unwrap());
    assert_eq!(
        std::fs::read_to_string(recording_folder.join("summary.md")).unwrap(),
        canonical_export
    );

    let summary = SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(summary.status, "completed");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(summary.result.as_deref().unwrap()).unwrap()
            ["markdown"],
        "## Decisions\nKeep the accepted result."
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(summary.result.as_deref().unwrap()).unwrap()
            ["export_markdown"],
        canonical_export
    );
    let owner =
        MeetingsRepository::get_lifecycle_run_id(&pool, meeting_id, LIFECYCLE_OPERATION_SUMMARY)
            .await
            .unwrap();
    assert_eq!(owner.as_deref(), Some(attempt_b.as_str()));
}

#[tokio::test]
async fn legacy_summary_reconciliation_preserves_matching_export_title() {
    let recording_folder = tempfile::tempdir().unwrap();
    let full_markdown = "# Legacy meeting title\n\n## Decisions\nKeep the accepted text.";
    let display_markdown = "## Decisions\nKeep the accepted text.";
    std::fs::write(recording_folder.path().join("summary.md"), full_markdown).unwrap();

    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = "meeting-legacy-summary-export";
    let lifecycle_run_id = MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Legacy meeting title",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_SUMMARY,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE meetings SET folder_path = ? WHERE id = ?")
        .bind(recording_folder.path().to_string_lossy().as_ref())
        .bind(meeting_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, result)
         VALUES (?, 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, ?)",
    )
    .bind(meeting_id)
    .bind(serde_json::json!({ "markdown": display_markdown }).to_string())
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        MeetingsRepository::get_lifecycle_run_id(&pool, meeting_id, LIFECYCLE_OPERATION_SUMMARY)
            .await
            .unwrap()
            .as_deref(),
        Some(lifecycle_run_id.as_str())
    );
    SummaryService::reconcile_accepted_summary(&pool, meeting_id)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(recording_folder.path().join("summary.md")).unwrap(),
        full_markdown
    );
}

#[tokio::test]
async fn manual_summary_acceptance_invalidates_generation_and_keeps_pending_export_canonical() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = "meeting-manual-summary-owner";
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Manual summary",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();

    let (generation_run, _) = SummaryProcessesRepository::start_process(&pool, meeting_id)
        .await
        .unwrap();
    let manual = serde_json::json!({ "markdown": "new manual text", "summary_json": [] });
    let (manual_run, replaced_run) =
        SummaryProcessesRepository::save_manual_summary(&pool, meeting_id, &manual, true)
            .await
            .unwrap();
    assert_eq!(replaced_run.as_deref(), Some(generation_run.as_str()));

    SummaryProcessesRepository::update_process_failed_for_run(
        &pool,
        meeting_id,
        &generation_run,
        "late provider failure",
    )
    .await
    .unwrap();
    assert!(matches!(
        SummaryProcessesRepository::update_process_completed_for_run(
            &pool,
            meeting_id,
            &generation_run,
            serde_json::json!({ "markdown": "late generated text" }),
            1,
            0.1,
        )
        .await,
        Err(sqlx::Error::RowNotFound)
    ));

    let summary = SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(summary.status, "completed");
    assert!(summary
        .error
        .as_deref()
        .unwrap()
        .starts_with("summary_export_pending:"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(summary.result.as_deref().unwrap()).unwrap()
            ["markdown"],
        "new manual text"
    );
    assert_eq!(
        MeetingsRepository::get_lifecycle_run_id(&pool, meeting_id, LIFECYCLE_OPERATION_SUMMARY)
            .await
            .unwrap()
            .as_deref(),
        Some(manual_run.as_str())
    );
}

#[tokio::test]
async fn manual_summary_rejects_recording_and_recording_finalization() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();

    for (meeting_id, status) in [
        ("meeting-manual-during-recording", "recording"),
        ("meeting-manual-during-recording-finalization", "processing"),
    ] {
        MeetingsRepository::ensure_lifecycle_meeting(
            &pool,
            meeting_id,
            "Recording must keep ownership",
            Some(1_725_000_123_456),
            "suggested",
            status,
            LIFECYCLE_OPERATION_RECORDING,
        )
        .await
        .unwrap();
        sqlx::query("UPDATE meetings SET lifecycle_run_id = 'recording-owner' WHERE id = ?")
            .bind(meeting_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(SummaryProcessesRepository::save_manual_summary(
            &pool,
            meeting_id,
            &serde_json::json!({ "markdown": "must not replace recording" }),
            false,
        )
        .await
        .is_err());

        let lifecycle = sqlx::query(
            "SELECT status, lifecycle_operation, lifecycle_run_id FROM meetings WHERE id = ?",
        )
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(lifecycle.get::<String, _>("status"), status);
        assert_eq!(
            lifecycle.get::<String, _>("lifecycle_operation"),
            LIFECYCLE_OPERATION_RECORDING
        );
        assert_eq!(
            lifecycle.get::<String, _>("lifecycle_run_id"),
            "recording-owner"
        );
        assert!(
            SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn summary_attempt_setup_failures_terminalize_their_run_and_remove_its_token() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();

    let chunk_failure_meeting = "meeting-summary-chunk-save-failure";
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        chunk_failure_meeting,
        "Chunk failure",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_transcript_chunk BEFORE INSERT ON transcript_chunks
         BEGIN SELECT RAISE(FAIL, 'injected transcript persistence failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = SummaryService::prepare_summary_attempt(
        &pool,
        chunk_failure_meeting,
        "transcript",
        "openai",
        "test-model",
        40_000,
        1_000,
    )
    .await
    .unwrap_err();
    assert!(error.contains("injected transcript persistence failure"));
    let chunk_failure_run = MeetingsRepository::get_lifecycle_run_id(
        &pool,
        chunk_failure_meeting,
        LIFECYCLE_OPERATION_SUMMARY,
    )
    .await
    .unwrap()
    .unwrap();
    let failed = SummaryProcessesRepository::get_summary_data(&pool, chunk_failure_meeting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.error.as_deref(), Some(error.as_str()));
    assert!(!SummaryService::cancel_summary(
        chunk_failure_meeting,
        &chunk_failure_run
    ));
    sqlx::query("DROP TRIGGER reject_transcript_chunk")
        .execute(&pool)
        .await
        .unwrap();

    let provider_failure_meeting = "meeting-summary-provider-setup-failure";
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        provider_failure_meeting,
        "Provider setup failure",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();
    let (provider_run, token) = SummaryService::prepare_summary_attempt(
        &pool,
        provider_failure_meeting,
        "transcript",
        "openai",
        "test-model",
        40_000,
        1_000,
    )
    .await
    .unwrap();
    assert!(!token.is_cancelled());
    SummaryService::fail_summary_attempt(
        &pool,
        provider_failure_meeting,
        &provider_run,
        "injected provider setup failure",
    )
    .await
    .unwrap();
    let failed = SummaryProcessesRepository::get_summary_data(&pool, provider_failure_meeting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.error.as_deref(),
        Some("injected provider setup failure")
    );
    assert!(!SummaryService::cancel_summary(
        provider_failure_meeting,
        &provider_run
    ));
}

#[tokio::test]
async fn cancelling_summary_targets_the_current_attempt_token() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();
    let meeting_id = "meeting-summary-cancel-owner";
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Cancel current summary",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();

    let (attempt_a, token_a) = SummaryService::prepare_summary_attempt(
        &pool,
        meeting_id,
        "transcript A",
        "openai",
        "test-model",
        40_000,
        1_000,
    )
    .await
    .unwrap();
    let (attempt_b, token_b) = SummaryService::prepare_summary_attempt(
        &pool,
        meeting_id,
        "transcript B",
        "openai",
        "test-model",
        40_000,
        1_000,
    )
    .await
    .unwrap();
    assert!(token_a.is_cancelled());
    assert!(!token_b.is_cancelled());
    assert!(SummaryService::cancel_summary(meeting_id, &attempt_a));
    assert!(token_a.is_cancelled());
    assert!(!token_b.is_cancelled());

    let cancelled_run = SummaryService::cancel_current_summary_attempt(&pool, meeting_id)
        .await
        .unwrap();
    assert_eq!(cancelled_run.as_deref(), Some(attempt_b.as_str()));
    assert!(token_b.is_cancelled());

    let summary = SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(summary.status, "cancelled");
    assert_eq!(
        MeetingsRepository::get_lifecycle_run_id(&pool, meeting_id, LIFECYCLE_OPERATION_SUMMARY)
            .await
            .unwrap()
            .as_deref(),
        Some(attempt_b.as_str())
    );
    SummaryService::cleanup_cancellation_token(meeting_id, &attempt_a);
    SummaryService::cleanup_cancellation_token(meeting_id, &attempt_b);
}

#[tokio::test]
async fn summary_export_marker_survives_a_retry_failure_or_cancellation() {
    let pool = open_pool(Path::new(":memory:")).await;
    MIGRATOR.run(&pool).await.unwrap();

    for (meeting_id, terminalize_as_cancelled) in [
        ("meeting-export-retry-failed", false),
        ("meeting-export-retry-cancelled", true),
    ] {
        MeetingsRepository::ensure_lifecycle_meeting(
            &pool,
            meeting_id,
            "Retried summary",
            Some(1_725_000_123_456),
            "suggested",
            MEETING_STATUS_READY,
            LIFECYCLE_OPERATION_RECORDING,
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, result, error)
             VALUES (?, 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, ?, 'summary_export_pending:injected export failure')",
        )
        .bind(meeting_id)
        .bind(serde_json::json!({ "markdown": "previous accepted result" }).to_string())
        .execute(&pool)
        .await
        .unwrap();

        let (run_id, _) = SummaryProcessesRepository::start_process(&pool, meeting_id)
            .await
            .unwrap();
        if terminalize_as_cancelled {
            SummaryProcessesRepository::update_process_cancelled_for_run(
                &pool, meeting_id, &run_id,
            )
            .await
            .unwrap();
        } else {
            SummaryProcessesRepository::update_process_failed_for_run(
                &pool,
                meeting_id,
                &run_id,
                "injected retry failure",
            )
            .await
            .unwrap();
        }

        let summary = SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            summary.status,
            if terminalize_as_cancelled {
                "cancelled"
            } else {
                "failed"
            }
        );
        assert!(summary
            .error
            .as_deref()
            .unwrap()
            .starts_with("summary_export_pending:"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(summary.result.as_deref().unwrap()).unwrap()
                ["markdown"],
            "previous accepted result"
        );
    }
}

#[tokio::test]
async fn reopen_retries_pending_summary_export_from_the_accepted_database_value() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("meeting.sqlite");
    let recordings = directory.path().join("recording");
    std::fs::create_dir(&recordings).unwrap();
    let meeting_id = "meeting-summary-export-reopen";
    let pool = open_pool(&path).await;
    MIGRATOR.run(&pool).await.unwrap();
    MeetingsRepository::ensure_lifecycle_meeting(
        &pool,
        meeting_id,
        "Export retry",
        Some(1_725_000_123_456),
        "suggested",
        MEETING_STATUS_READY,
        LIFECYCLE_OPERATION_RECORDING,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE meetings SET folder_path = ?, lifecycle_operation = ? WHERE id = ?")
        .bind(recordings.to_string_lossy().as_ref())
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(meeting_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO summary_processes (
            meeting_id, status, created_at, updated_at, result
         ) VALUES (?, 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, ?)",
    )
    .bind(meeting_id)
    .bind(serde_json::json!({ "markdown": "previous accepted" }).to_string())
    .execute(&pool)
    .await
    .unwrap();
    let (run_id, _) = SummaryProcessesRepository::start_process(&pool, meeting_id)
        .await
        .unwrap();
    std::fs::create_dir(recordings.join("summary.md")).unwrap();
    let canonical_export = "# Export retry\n\n## Decisions\nThe saved decisions.";
    assert!(SummaryService::accept_generated_summary(
        &pool,
        meeting_id,
        &run_id,
        serde_json::json!({
            "markdown": "## Decisions\nThe saved decisions.",
            "export_markdown": canonical_export,
        }),
        canonical_export,
        SummaryProcessingMetrics {
            chunk_count: 1,
            processing_time: 0.2
        },
        None,
    )
    .await
    .unwrap());
    let accepted_pending = SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert!(accepted_pending
        .error
        .as_deref()
        .unwrap()
        .starts_with("summary_export_pending:"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            accepted_pending.result_backup.as_deref().unwrap()
        )
        .unwrap()["markdown"],
        "previous accepted"
    );
    std::fs::remove_dir(recordings.join("summary.md")).unwrap();
    std::fs::write(recordings.join("summary.md"), "previous accepted file").unwrap();
    let (interrupted_retry, _) = SummaryProcessesRepository::start_process(&pool, meeting_id)
        .await
        .unwrap();
    let retrying = SummaryProcessesRepository::get_summary_data(&pool, meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrying.status, "PENDING");
    assert!(retrying
        .error
        .as_deref()
        .unwrap()
        .starts_with("summary_export_pending:"));
    assert_ne!(interrupted_retry, run_id);
    pool.close().await;

    let legacy_path = directory.path().join("legacy.sqlite");
    let manager = DatabaseManager::new(path.to_str().unwrap(), legacy_path.to_str().unwrap())
        .await
        .unwrap();
    let reopened = manager.pool();

    assert_eq!(
        std::fs::read_to_string(recordings.join("summary.md")).unwrap(),
        canonical_export
    );
    let summary = SummaryProcessesRepository::get_summary_data(reopened, meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(summary.status, "failed");
    assert!(summary
        .error
        .as_deref()
        .is_some_and(|error| error.contains("interrupted by application restart")));
    assert!(summary.result_backup.is_none());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(summary.result.as_deref().unwrap()).unwrap()
            ["markdown"],
        "## Decisions\nThe saved decisions."
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(summary.result.as_deref().unwrap()).unwrap()
            ["export_markdown"],
        canonical_export
    );
    assert!(MeetingsRepository::get_lifecycle_run_id(
        reopened,
        meeting_id,
        LIFECYCLE_OPERATION_SUMMARY
    )
    .await
    .unwrap()
    .is_none());
}
