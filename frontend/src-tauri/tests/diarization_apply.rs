use app_lib::diarization::{
    commands::{apply_diarization_segments_with_pool, ApplyDiarizationRequest, DiarizationRunKind},
    types::{DiarizationStatus, SpeakerSegment},
};
use sqlx::{FromRow, SqlitePool};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

const FEATURE_DISABLED: &str = "feature_disabled";
const INVALID_REQUEST: &str = "invalid_request";
const MEETING_NOT_FOUND: &str = "meeting_not_found";
const NO_ALIGNABLE_TRANSCRIPTS: &str = "no_alignable_transcripts";
const PERSISTENCE_FAILED: &str = "persistence_failed";

#[derive(Debug, FromRow, PartialEq)]
struct SegmentState {
    id: String,
    meeting_id: String,
    source: String,
    start_time: f64,
    end_time: f64,
    speaker_id: Option<String>,
    speaker_label: Option<String>,
    confidence: Option<f64>,
    is_overlap: i64,
    diarization_status: String,
    diarization_method: Option<String>,
    created_at: String,
}

#[derive(Debug, FromRow, PartialEq)]
struct MeetingStatusState {
    meeting_id: String,
    enabled: i64,
    live_enabled: i64,
    post_call_enabled: i64,
    current_status: String,
    quality_flags: Option<String>,
    processed_at: Option<String>,
    updated_at: String,
}

#[derive(Debug, FromRow, PartialEq)]
struct TranscriptState {
    id: String,
    speaker_id: Option<String>,
    speaker_label: Option<String>,
    speaker_color: Option<String>,
    is_overlap: i64,
    diarization_status: String,
    diarization_method: Option<String>,
    diarization_confidence: Option<f64>,
}

#[derive(Debug, PartialEq)]
struct DiarizationSnapshot {
    segments: Vec<SegmentState>,
    statuses: Vec<MeetingStatusState>,
    transcripts: Vec<TranscriptState>,
}

async fn database() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    pool
}

async fn seed_existing_diarization(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at)
         VALUES ('meeting-1', 'Meeting', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();

    for (id, start, end) in [("transcript-1", 0.0, 1.0), ("transcript-2", 1.0, 2.0)] {
        sqlx::query(
            "INSERT INTO transcripts (
                id, meeting_id, transcript, timestamp, speaker,
                audio_start_time, audio_end_time, duration,
                speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
             ) VALUES (?, 'meeting-1', 'Existing transcript', '2026-01-01T00:00:00Z',
                       'mic', ?, ?, 1.0, 'old-speaker', 'Old Speaker', '#111111', 0,
                       'provisional', 'old-engine', 0.4)",
        )
        .bind(id)
        .bind(start)
        .bind(end)
        .execute(pool)
        .await
        .unwrap();
    }

    sqlx::query(
        "INSERT INTO speaker_segments (
            id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
            confidence, is_overlap, diarization_status, diarization_method, created_at
         ) VALUES ('old-segment', 'meeting-1', 'old-source', 0.0, 2.0,
                   'old-speaker', 'Old Speaker', 0.4, 0, 'provisional', 'old-engine',
                   '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO meeting_diarization_status (
            meeting_id, enabled, live_enabled, post_call_enabled, current_status,
            quality_flags, processed_at, updated_at
         ) VALUES ('meeting-1', 1, 1, 0, 'provisional', 'old-quality',
                   '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn set_settings(pool: &SqlitePool, enabled: bool, mode: &str) {
    sqlx::query("UPDATE diarization_settings SET enabled = ?, mode = ? WHERE id = '1'")
        .bind(enabled as i64)
        .bind(mode)
        .execute(pool)
        .await
        .unwrap();
}

fn segment(status: DiarizationStatus) -> SpeakerSegment {
    SpeakerSegment {
        meeting_id: "meeting-1".to_string(),
        source: "test-source".to_string(),
        start_time: 0.0,
        end_time: 1.0,
        speaker_id: Some("speaker-1".to_string()),
        speaker_label: Some("Speaker 1".to_string()),
        confidence: Some(0.9),
        is_overlap: false,
        diarization_status: status,
        diarization_method: None,
    }
}

fn request(run_kind: DiarizationRunKind) -> ApplyDiarizationRequest {
    ApplyDiarizationRequest {
        meeting_id: "meeting-1".to_string(),
        method: "test-engine".to_string(),
        run_kind,
        segments: vec![segment(DiarizationStatus::Final)],
    }
}

async fn snapshot(pool: &SqlitePool) -> DiarizationSnapshot {
    let segments = sqlx::query_as::<_, SegmentState>(
        "SELECT id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
                confidence, is_overlap, diarization_status, diarization_method,
                CAST(created_at AS TEXT) AS created_at
         FROM speaker_segments ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let statuses = sqlx::query_as::<_, MeetingStatusState>(
        "SELECT meeting_id, enabled, live_enabled, post_call_enabled, current_status,
                quality_flags, CAST(processed_at AS TEXT) AS processed_at,
                CAST(updated_at AS TEXT) AS updated_at
         FROM meeting_diarization_status ORDER BY meeting_id",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let transcripts = sqlx::query_as::<_, TranscriptState>(
        "SELECT id, speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
         FROM transcripts ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    DiarizationSnapshot {
        segments,
        statuses,
        transcripts,
    }
}

async fn assert_rejected_without_changes(
    pool: &SqlitePool,
    request: ApplyDiarizationRequest,
    expected_error: &str,
    expected_snapshot: &DiarizationSnapshot,
) {
    let error = apply_diarization_segments_with_pool(pool, request)
        .await
        .unwrap_err();
    assert_eq!(error, expected_error);
    assert_eq!(&snapshot(pool).await, expected_snapshot);
}

#[test]
fn apply_request_uses_explicit_snake_case_run_kind_and_camel_case_field() {
    assert_eq!(
        serde_json::to_value(DiarizationRunKind::Live).unwrap(),
        "live"
    );
    assert_eq!(
        serde_json::to_value(DiarizationRunKind::PostCall).unwrap(),
        "post_call"
    );
    assert!(serde_json::from_value::<DiarizationRunKind>(serde_json::json!("online")).is_err());

    let json = serde_json::to_value(request(DiarizationRunKind::Live)).unwrap();
    assert_eq!(json["runKind"], "live");
    assert!(json.get("run_kind").is_none());
}

#[tokio::test]
async fn disabled_off_incompatible_and_corrupt_settings_preserve_existing_data() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    let before = snapshot(&pool).await;

    set_settings(&pool, false, "live_plus_post_call").await;
    assert_rejected_without_changes(
        &pool,
        request(DiarizationRunKind::Live),
        FEATURE_DISABLED,
        &before,
    )
    .await;

    set_settings(&pool, false, "off").await;
    assert_rejected_without_changes(
        &pool,
        request(DiarizationRunKind::PostCall),
        FEATURE_DISABLED,
        &before,
    )
    .await;

    set_settings(&pool, true, "post_call_only").await;
    assert_rejected_without_changes(
        &pool,
        request(DiarizationRunKind::Live),
        FEATURE_DISABLED,
        &before,
    )
    .await;

    set_settings(&pool, true, "off").await;
    assert_rejected_without_changes(
        &pool,
        request(DiarizationRunKind::PostCall),
        PERSISTENCE_FAILED,
        &before,
    )
    .await;
}

fn invalid_requests() -> Vec<(&'static str, ApplyDiarizationRequest)> {
    let mut cases = Vec::new();

    for (name, value) in [
        ("blank meeting", " ".to_string()),
        ("control meeting", "meeting\n1".to_string()),
        ("long meeting", "m".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.meeting_id = value.clone();
        candidate.segments[0].meeting_id = value;
        cases.push((name, candidate));
    }

    for (name, value) in [
        ("blank method", " ".to_string()),
        ("control method", "engine\0name".to_string()),
        ("long method", "e".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.method = value;
        cases.push((name, candidate));
    }

    let mut empty = request(DiarizationRunKind::PostCall);
    empty.segments.clear();
    cases.push(("empty segments", empty));

    let mut cross_meeting = request(DiarizationRunKind::PostCall);
    let mut foreign = segment(DiarizationStatus::Final);
    foreign.meeting_id = "meeting-2".to_string();
    cross_meeting.segments.push(foreign);
    cases.push(("one cross-meeting segment", cross_meeting));

    for (name, value) in [
        ("blank segment meeting", " ".to_string()),
        ("control segment meeting", "meeting\n1".to_string()),
        ("long segment meeting", "m".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].meeting_id = value;
        cases.push((name, candidate));
    }

    for (name, value) in [
        ("blank source", " ".to_string()),
        ("control source", "source\nname".to_string()),
        ("long source", "s".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].source = value;
        cases.push((name, candidate));
    }

    for (name, value) in [
        ("blank speaker id", " ".to_string()),
        ("control speaker id", "speaker\nid".to_string()),
        ("long speaker id", "i".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].speaker_id = Some(value);
        cases.push((name, candidate));
    }

    for (name, value) in [
        ("blank speaker label", " ".to_string()),
        ("control speaker label", "Speaker\n1".to_string()),
        ("long speaker label", "l".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].speaker_label = Some(value);
        cases.push((name, candidate));
    }

    for (name, value) in [
        ("blank segment method", " ".to_string()),
        ("control segment method", "engine\tname".to_string()),
        ("long segment method", "d".repeat(129)),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].diarization_method = Some(value);
        cases.push((name, candidate));
    }

    for (name, start, end) in [
        ("nan start", f64::NAN, 1.0),
        ("infinite start", f64::INFINITY, 1.0),
        ("negative start", -0.1, 1.0),
        ("nan end", 0.0, f64::NAN),
        ("infinite end", 0.0, f64::INFINITY),
        ("negative end", 0.0, -0.1),
        ("equal times", 1.0, 1.0),
        ("reversed times", 2.0, 1.0),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].start_time = start;
        candidate.segments[0].end_time = end;
        cases.push((name, candidate));
    }

    for (name, confidence) in [
        ("nan confidence", f64::NAN),
        ("negative confidence", -0.1),
        ("high confidence", 1.1),
    ] {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments[0].confidence = Some(confidence);
        cases.push((name, candidate));
    }

    let mut overlap_id = request(DiarizationRunKind::PostCall);
    overlap_id.segments[0].is_overlap = true;
    overlap_id.segments[0].speaker_label = None;
    cases.push(("overlap concrete id", overlap_id));

    let mut overlap_label = request(DiarizationRunKind::PostCall);
    overlap_label.segments[0].is_overlap = true;
    overlap_label.segments[0].speaker_id = None;
    cases.push(("overlap concrete label", overlap_label));

    cases
}

#[tokio::test]
async fn every_invalid_request_is_rejected_before_mutation() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    set_settings(&pool, true, "live_plus_post_call").await;
    let before = snapshot(&pool).await;

    for (name, invalid) in invalid_requests() {
        let error = apply_diarization_segments_with_pool(&pool, invalid)
            .await
            .unwrap_err();
        assert_eq!(error, INVALID_REQUEST, "wrong error for {name}");
        assert_eq!(snapshot(&pool).await, before, "mutated data for {name}");
    }
}

#[tokio::test]
async fn nonexistent_meeting_preserves_existing_data() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    set_settings(&pool, true, "live_plus_post_call").await;
    let before = snapshot(&pool).await;
    let mut missing = request(DiarizationRunKind::PostCall);
    missing.meeting_id = "missing-meeting".to_string();
    missing.segments[0].meeting_id = "missing-meeting".to_string();

    assert_rejected_without_changes(&pool, missing, MEETING_NOT_FOUND, &before).await;
}

#[tokio::test]
async fn no_alignable_transcript_preserves_existing_data() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    set_settings(&pool, true, "live_plus_post_call").await;
    sqlx::query(
        "UPDATE transcripts
         SET audio_start_time = CASE WHEN id = 'transcript-1' THEN NULL ELSE 2.0 END,
             audio_end_time = CASE WHEN id = 'transcript-1' THEN NULL ELSE 1.0 END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;

    assert_rejected_without_changes(
        &pool,
        request(DiarizationRunKind::PostCall),
        NO_ALIGNABLE_TRANSCRIPTS,
        &before,
    )
    .await;
}

#[tokio::test]
async fn settings_read_failure_is_sanitized_and_preserves_existing_data() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    let before = snapshot(&pool).await;
    sqlx::query("DROP TABLE diarization_settings")
        .execute(&pool)
        .await
        .unwrap();

    assert_rejected_without_changes(
        &pool,
        request(DiarizationRunKind::PostCall),
        PERSISTENCE_FAILED,
        &before,
    )
    .await;
}

async fn assert_trigger_failure_rolls_back(trigger_sql: &str) {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    set_settings(&pool, true, "live_plus_post_call").await;
    let before = snapshot(&pool).await;
    sqlx::query(trigger_sql).execute(&pool).await.unwrap();

    let error = apply_diarization_segments_with_pool(&pool, request(DiarizationRunKind::PostCall))
        .await
        .unwrap_err();
    assert_eq!(error, PERSISTENCE_FAILED);
    assert!(!error.contains("secret"));
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
async fn segment_insert_failure_rolls_back_delete_and_all_other_changes() {
    assert_trigger_failure_rolls_back(
        "CREATE TRIGGER fail_segment_insert BEFORE INSERT ON speaker_segments
         BEGIN SELECT RAISE(ABORT, 'secret segment failure'); END",
    )
    .await;
}

#[tokio::test]
async fn status_update_failure_rolls_back_segments_and_transcripts() {
    assert_trigger_failure_rolls_back(
        "CREATE TRIGGER fail_status_update BEFORE UPDATE ON meeting_diarization_status
         BEGIN SELECT RAISE(ABORT, 'secret status failure'); END",
    )
    .await;
}

#[tokio::test]
async fn transcript_update_failure_rolls_back_segments_and_status() {
    assert_trigger_failure_rolls_back(
        "CREATE TRIGGER fail_transcript_update BEFORE UPDATE OF speaker_id ON transcripts
         BEGIN SELECT RAISE(ABORT, 'secret transcript failure'); END",
    )
    .await;
}

#[tokio::test]
async fn successful_apply_replaces_data_uses_deterministic_ids_and_explicit_run_flags() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    set_settings(&pool, true, "live_plus_post_call").await;

    let mut live = request(DiarizationRunKind::Live);
    live.method = "offline-post-engine".to_string();
    let mut overlap = segment(DiarizationStatus::NeedsReview);
    overlap.start_time = 1.0;
    overlap.end_time = 2.0;
    overlap.speaker_id = None;
    overlap.speaker_label = None;
    overlap.is_overlap = true;
    overlap.confidence = Some(0.8);
    live.segments.push(overlap);

    apply_diarization_segments_with_pool(&pool, live.clone())
        .await
        .unwrap();

    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM speaker_segments ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        ids,
        vec![
            "diarization:meeting-1:0".to_string(),
            "diarization:meeting-1:1".to_string()
        ]
    );

    let live_status: (i64, i64, String) = sqlx::query_as(
        "SELECT live_enabled, post_call_enabled, current_status
         FROM meeting_diarization_status WHERE meeting_id = 'meeting-1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(live_status, (1, 0, "needs_review".to_string()));

    let transcripts = sqlx::query_as::<_, TranscriptState>(
        "SELECT id, speaker_id, speaker_label, speaker_color, is_overlap,
                diarization_status, diarization_method, diarization_confidence
         FROM transcripts ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(transcripts[0].speaker_id.as_deref(), Some("speaker-1"));
    assert_eq!(transcripts[0].speaker_label.as_deref(), Some("Speaker 1"));
    assert_eq!(transcripts[0].diarization_status, "final");
    assert_eq!(
        transcripts[1].speaker_label.as_deref(),
        Some("Multiple speakers")
    );
    assert!(transcripts[1].speaker_id.is_none());
    assert_eq!(transcripts[1].speaker_color.as_deref(), Some("#f97316"));
    assert_eq!(transcripts[1].is_overlap, 1);

    apply_diarization_segments_with_pool(&pool, live.clone())
        .await
        .unwrap();
    let repeated_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM speaker_segments ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(repeated_ids, ids);

    live.run_kind = DiarizationRunKind::PostCall;
    live.method = "online-live-engine".to_string();
    set_settings(&pool, true, "post_call_only").await;
    apply_diarization_segments_with_pool(&pool, live)
        .await
        .unwrap();
    let post_call_flags: (i64, i64) = sqlx::query_as(
        "SELECT live_enabled, post_call_enabled
         FROM meeting_diarization_status WHERE meeting_id = 'meeting-1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(post_call_flags, (0, 1));
}

#[tokio::test]
async fn meeting_status_uses_explicit_precedence() {
    let pool = database().await;
    seed_existing_diarization(&pool).await;
    set_settings(&pool, true, "live_plus_post_call").await;

    let cases = [
        (
            vec![
                DiarizationStatus::Final,
                DiarizationStatus::Provisional,
                DiarizationStatus::FallbackToLive,
                DiarizationStatus::NeedsReview,
                DiarizationStatus::Failed,
            ],
            "failed",
        ),
        (
            vec![
                DiarizationStatus::Final,
                DiarizationStatus::Provisional,
                DiarizationStatus::FallbackToLive,
                DiarizationStatus::NeedsReview,
            ],
            "needs_review",
        ),
        (
            vec![
                DiarizationStatus::Final,
                DiarizationStatus::Provisional,
                DiarizationStatus::FallbackToLive,
            ],
            "fallback_to_live",
        ),
        (
            vec![DiarizationStatus::Final, DiarizationStatus::Provisional],
            "provisional",
        ),
        (
            vec![DiarizationStatus::Final, DiarizationStatus::Final],
            "final",
        ),
        (
            vec![DiarizationStatus::Final, DiarizationStatus::None],
            "none",
        ),
    ];

    for (statuses, expected) in cases {
        let mut candidate = request(DiarizationRunKind::PostCall);
        candidate.segments = statuses
            .into_iter()
            .enumerate()
            .map(|(index, status)| {
                let mut value = segment(status);
                value.speaker_id = Some(format!("speaker-{index}"));
                value.speaker_label = Some(format!("Speaker {index}"));
                value
            })
            .collect();

        apply_diarization_segments_with_pool(&pool, candidate)
            .await
            .unwrap();
        let actual: String = sqlx::query_scalar(
            "SELECT current_status FROM meeting_diarization_status
             WHERE meeting_id = 'meeting-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(actual, expected);
    }
}
