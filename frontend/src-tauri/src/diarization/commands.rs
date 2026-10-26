use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::future::Future;
use tauri::State;
use tracing::error;

use crate::{
    database::repositories::diarization::DiarizationRepository,
    diarization::{
        alignment::assign_speaker_to_transcript,
        types::{DiarizationStatus, SpeakerSegment, TranscriptWindow},
    },
    state::AppState,
};

pub use super::{DiarizationMode, OverlapHandling};

const LOAD_SETTINGS_ERROR: &str = "Unable to load diarization settings";
const SAVE_SETTINGS_ERROR: &str = "Unable to save diarization settings";
const FEATURE_DISABLED_ERROR: &str = "feature_disabled";
const INVALID_REQUEST_ERROR: &str = "invalid_request";
const MEETING_NOT_FOUND_ERROR: &str = "meeting_not_found";
const NO_ALIGNABLE_TRANSCRIPTS_ERROR: &str = "no_alignable_transcripts";
const PERSISTENCE_FAILED_ERROR: &str = "persistence_failed";
const SPEAKER_NOT_FOUND_ERROR: &str = "speaker_not_found";
const MAX_REQUEST_TEXT_CHARS: usize = 128;
const MIN_TRANSCRIPT_OVERLAP_RATIO: f64 = 0.1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiarizationSettingsDto {
    pub enabled: bool,
    pub mode: DiarizationMode,
    pub show_provisional_labels: bool,
    pub post_call_refinement_enabled: bool,
    pub overlap_handling: OverlapHandling,
    pub speaker_review_enabled: bool,
}

impl DiarizationSettingsDto {
    fn is_valid(&self) -> bool {
        !(self.enabled && self.mode == DiarizationMode::Off)
    }
}

fn decode_database_bool(field: &str, value: i64) -> Result<bool, String> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => {
            error!("Invalid boolean value for diarization_settings.{field}: {value}");
            Err(LOAD_SETTINGS_ERROR.to_string())
        }
    }
}

async fn load_diarization_settings(pool: &SqlitePool) -> Result<DiarizationSettingsDto, String> {
    let stored = DiarizationRepository::get_settings(pool)
        .await
        .map_err(|error| {
            error!("Unable to load diarization settings from SQLite: {error}");
            LOAD_SETTINGS_ERROR.to_string()
        })?;

    decode_diarization_settings(stored)
}

fn decode_diarization_settings(
    stored: crate::database::models::DiarizationSetting,
) -> Result<DiarizationSettingsDto, String> {
    let settings = DiarizationSettingsDto {
        enabled: decode_database_bool("enabled", stored.enabled)?,
        mode: DiarizationMode::try_from(stored.mode.as_str()).map_err(|()| {
            error!("Invalid diarization mode persisted in SQLite");
            LOAD_SETTINGS_ERROR.to_string()
        })?,
        show_provisional_labels: decode_database_bool(
            "show_provisional_labels",
            stored.show_provisional_labels,
        )?,
        post_call_refinement_enabled: decode_database_bool(
            "post_call_refinement_enabled",
            stored.post_call_refinement_enabled,
        )?,
        overlap_handling: OverlapHandling::try_from(stored.overlap_handling.as_str()).map_err(
            |()| {
                error!("Invalid diarization overlap handling persisted in SQLite");
                LOAD_SETTINGS_ERROR.to_string()
            },
        )?,
        speaker_review_enabled: decode_database_bool(
            "speaker_review_enabled",
            stored.speaker_review_enabled,
        )?,
    };

    if !settings.is_valid() {
        error!("Invalid diarization settings combination persisted in SQLite");
        return Err(LOAD_SETTINGS_ERROR.to_string());
    }

    Ok(settings)
}

async fn store_diarization_settings(
    pool: &SqlitePool,
    settings: DiarizationSettingsDto,
) -> Result<(), String> {
    if !settings.is_valid() {
        return Err(SAVE_SETTINGS_ERROR.to_string());
    }

    DiarizationRepository::save_settings(
        pool,
        settings.enabled,
        settings.mode,
        settings.show_provisional_labels,
        settings.post_call_refinement_enabled,
        settings.overlap_handling,
        settings.speaker_review_enabled,
    )
    .await
    .map_err(|error| {
        error!("Unable to save diarization settings to SQLite: {error}");
        SAVE_SETTINGS_ERROR.to_string()
    })
}

#[tauri::command]
pub async fn get_diarization_settings(
    state: State<'_, AppState>,
) -> Result<DiarizationSettingsDto, String> {
    load_diarization_settings(state.db_manager.pool()).await
}

#[tauri::command]
pub async fn save_diarization_settings(
    state: State<'_, AppState>,
    settings: DiarizationSettingsDto,
) -> Result<(), String> {
    store_diarization_settings(state.db_manager.pool(), settings).await
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RenameSpeakerRequest {
    pub meeting_id: String,
    pub speaker_id: String,
    pub label: String,
}

#[tauri::command]
pub async fn rename_speaker(
    state: State<'_, AppState>,
    request: RenameSpeakerRequest,
) -> Result<(), String> {
    rename_speaker_with_pool(state.db_manager.pool(), request).await
}

pub async fn rename_speaker_with_pool(
    pool: &SqlitePool,
    request: RenameSpeakerRequest,
) -> Result<(), String> {
    let label = request.label.trim();
    if !is_valid_identifier(&request.meeting_id)
        || !is_valid_identifier(&request.speaker_id)
        || is_overlap_identity(&request.speaker_id)
        || !is_valid_request_text(label)
    {
        return Err(INVALID_REQUEST_ERROR.to_string());
    }

    let updated_count = DiarizationRepository::rename_speaker(
        pool,
        &request.meeting_id,
        &request.speaker_id,
        label,
    )
    .await
    .map_err(|error| persistence_error("rename speaker", error))?;

    if updated_count == 0 {
        return Err(SPEAKER_NOT_FOUND_ERROR.to_string());
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationRunKind {
    Live,
    PostCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyDiarizationRequest {
    pub meeting_id: String,
    pub method: String,
    pub run_kind: DiarizationRunKind,
    pub segments: Vec<SpeakerSegment>,
}

#[tauri::command]
pub async fn apply_diarization_segments(
    state: State<'_, AppState>,
    request: ApplyDiarizationRequest,
) -> Result<(), String> {
    apply_diarization_segments_with_pool(state.db_manager.pool(), request).await
}

pub async fn apply_diarization_segments_with_pool(
    pool: &SqlitePool,
    request: ApplyDiarizationRequest,
) -> Result<(), String> {
    apply_diarization_segments_with_hook(pool, request, || async {}).await
}

async fn apply_diarization_segments_with_hook<F, Fut>(
    pool: &SqlitePool,
    request: ApplyDiarizationRequest,
    before_transaction: F,
) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    let settings = load_diarization_settings(pool)
        .await
        .map_err(|_| PERSISTENCE_FAILED_ERROR.to_string())?;

    if !settings_allow_run(&settings, request.run_kind) {
        return Err(FEATURE_DISABLED_ERROR.to_string());
    }

    if !is_valid_request_text(&request.meeting_id)
        || !is_valid_request_text(&request.method)
        || request.segments.is_empty()
        || request
            .segments
            .iter()
            .any(|segment| segment.validate_for_meeting(&request.meeting_id).is_err())
    {
        return Err(INVALID_REQUEST_ERROR.to_string());
    }

    let meeting_exists =
        sqlx::query_scalar::<_, i64>("SELECT EXISTS(SELECT 1 FROM meetings WHERE id = ?)")
            .bind(&request.meeting_id)
            .fetch_one(pool)
            .await
            .map_err(|error| persistence_error("check meeting existence", error))?;

    if meeting_exists != 1 {
        return Err(MEETING_NOT_FOUND_ERROR.to_string());
    }

    let validation_transcripts = sqlx::query_as::<_, (String, Option<f64>, Option<f64>)>(
        "SELECT id, audio_start_time, audio_end_time
         FROM transcripts
         WHERE meeting_id = ?
         ORDER BY audio_start_time ASC, id ASC",
    )
    .bind(&request.meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|error| persistence_error("load transcript windows", error))?;

    if !validation_transcripts.iter().any(|(_, start, end)| {
        matches!((start, end), (Some(start), Some(end)) if is_valid_transcript_window(*start, *end))
    }) {
        return Err(NO_ALIGNABLE_TRANSCRIPTS_ERROR.to_string());
    }

    before_transaction().await;

    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| persistence_error("begin diarization apply transaction", error))?;

    let transaction_settings =
        match DiarizationRepository::get_settings_in_transaction(&mut transaction).await {
            Ok(stored) => match decode_diarization_settings(stored) {
                Ok(settings) => settings,
                Err(_) => {
                    rollback_transaction(transaction, "invalid settings recheck").await;
                    return Err(PERSISTENCE_FAILED_ERROR.to_string());
                }
            },
            Err(settings_error) => {
                error!("Unable to recheck diarization settings in transaction: {settings_error}");
                rollback_transaction(transaction, "settings recheck failure").await;
                return Err(PERSISTENCE_FAILED_ERROR.to_string());
            }
        };

    if !settings_allow_run(&transaction_settings, request.run_kind) {
        rollback_transaction(transaction, "feature disabled during preflight").await;
        return Err(FEATURE_DISABLED_ERROR.to_string());
    }

    let transaction_result: Result<(), sqlx::Error> = async {
        let meeting_still_exists = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(SELECT 1 FROM meetings WHERE id = ?)",
        )
        .bind(&request.meeting_id)
        .fetch_one(&mut *transaction)
        .await?;
        if meeting_still_exists != 1 {
            return Err(sqlx::Error::RowNotFound);
        }

        let transcripts = sqlx::query_as::<_, (String, Option<f64>, Option<f64>)>(
            "SELECT id, audio_start_time, audio_end_time
             FROM transcripts
             WHERE meeting_id = ?
             ORDER BY audio_start_time ASC, id ASC",
        )
        .bind(&request.meeting_id)
        .fetch_all(&mut *transaction)
        .await?;
        if !transcripts.iter().any(|(_, start, end)| {
            matches!((start, end), (Some(start), Some(end)) if is_valid_transcript_window(*start, *end))
        }) {
            return Err(sqlx::Error::Protocol(
                "alignable transcripts changed during diarization apply".to_string(),
            ));
        }

        DiarizationRepository::replace_speaker_segments(
            &mut transaction,
            &request.meeting_id,
            &request.method,
            &request.segments,
        )
        .await?;

        let (live_enabled, post_call_enabled) = match request.run_kind {
            DiarizationRunKind::Live => (true, false),
            DiarizationRunKind::PostCall => (false, true),
        };
        DiarizationRepository::upsert_meeting_diarization_status(
            &mut transaction,
            &request.meeting_id,
            live_enabled,
            post_call_enabled,
            meeting_status_from_segments(&request.segments),
        )
        .await?;

        for (transcript_id, audio_start_time, audio_end_time) in &transcripts {
            let window = TranscriptWindow {
                transcript_id: transcript_id.clone(),
                audio_start_time: *audio_start_time,
                audio_end_time: *audio_end_time,
            };
            let assignment = assign_speaker_to_transcript(
                &window,
                &request.segments,
                MIN_TRANSCRIPT_OVERLAP_RATIO,
                &request.method,
            );

            DiarizationRepository::update_transcript_assignment(
                &mut transaction,
                &request.meeting_id,
                transcript_id,
                &assignment,
            )
            .await?;
        }

        Ok(())
    }
    .await;

    if let Err(apply_error) = transaction_result {
        error!("Unable to apply diarization transaction: {apply_error}");
        rollback_transaction(transaction, "apply failure").await;
        return Err(PERSISTENCE_FAILED_ERROR.to_string());
    }

    transaction
        .commit()
        .await
        .map_err(|error| persistence_error("commit diarization apply transaction", error))?;

    Ok(())
}

fn settings_allow_run(settings: &DiarizationSettingsDto, run_kind: DiarizationRunKind) -> bool {
    settings.enabled
        && match (settings.mode, run_kind) {
            (DiarizationMode::LivePlusPostCall, DiarizationRunKind::Live) => true,
            (DiarizationMode::LivePlusPostCall, DiarizationRunKind::PostCall)
            | (DiarizationMode::PostCallOnly, DiarizationRunKind::PostCall) => {
                settings.post_call_refinement_enabled
            }
            (DiarizationMode::PostCallOnly | DiarizationMode::Off, _) => false,
        }
}

pub(crate) async fn ensure_post_call_run_allowed(pool: &SqlitePool) -> Result<(), String> {
    let settings = load_diarization_settings(pool)
        .await
        .map_err(|_| PERSISTENCE_FAILED_ERROR.to_string())?;
    if settings_allow_run(&settings, DiarizationRunKind::PostCall) {
        Ok(())
    } else {
        Err(FEATURE_DISABLED_ERROR.to_string())
    }
}

fn is_valid_request_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= MAX_REQUEST_TEXT_CHARS
        && !value.chars().any(char::is_control)
}

fn is_valid_identifier(value: &str) -> bool {
    value == value.trim() && is_valid_request_text(value)
}

fn is_overlap_identity(speaker_id: &str) -> bool {
    matches!(
        speaker_id.to_ascii_lowercase().as_str(),
        "multiple_speakers" | "multiple-speakers" | "multiple speakers" | "overlap"
    )
}

fn is_valid_transcript_window(start: f64, end: f64) -> bool {
    start.is_finite() && end.is_finite() && start >= 0.0 && end > start
}

fn meeting_status_from_segments(segments: &[SpeakerSegment]) -> DiarizationStatus {
    for status in [
        DiarizationStatus::Failed,
        DiarizationStatus::NeedsReview,
        DiarizationStatus::FallbackToLive,
        DiarizationStatus::Provisional,
    ] {
        if segments
            .iter()
            .any(|segment| segment.diarization_status == status)
        {
            return status;
        }
    }

    if segments
        .iter()
        .all(|segment| segment.diarization_status == DiarizationStatus::Final)
    {
        DiarizationStatus::Final
    } else {
        DiarizationStatus::None
    }
}

fn persistence_error(context: &str, error: sqlx::Error) -> String {
    error!("Unable to {context}: {error}");
    PERSISTENCE_FAILED_ERROR.to_string()
}

async fn rollback_transaction(transaction: Transaction<'_, Sqlite>, context: &str) {
    if let Err(rollback_error) = transaction.rollback().await {
        error!("Unable to roll back diarization transaction after {context}: {rollback_error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_diarization_segments_with_hook, load_diarization_settings,
        store_diarization_settings, ApplyDiarizationRequest, DiarizationMode, DiarizationRunKind,
        DiarizationSettingsDto, OverlapHandling, FEATURE_DISABLED_ERROR, LOAD_SETTINGS_ERROR,
        SAVE_SETTINGS_ERROR,
    };
    use crate::diarization::types::{DiarizationStatus, SpeakerSegment};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

    async fn database() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    async fn concurrent_database() -> (tempfile::TempDir, SqlitePool) {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("settings.sqlite"))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        (directory, pool)
    }

    fn live_settings() -> DiarizationSettingsDto {
        DiarizationSettingsDto {
            enabled: true,
            mode: DiarizationMode::LivePlusPostCall,
            show_provisional_labels: true,
            post_call_refinement_enabled: false,
            overlap_handling: OverlapHandling::MultipleSpeakers,
            speaker_review_enabled: false,
        }
    }

    #[tokio::test]
    async fn settings_defaults_are_disabled_and_typed() {
        let pool = database().await;
        sqlx::query("DELETE FROM diarization_settings")
            .execute(&pool)
            .await
            .unwrap();

        let settings = load_diarization_settings(&pool).await.unwrap();

        assert!(!settings.enabled);
        assert_eq!(settings.mode, DiarizationMode::LivePlusPostCall);
        assert_eq!(settings.overlap_handling, OverlapHandling::MultipleSpeakers);
        assert!(settings.show_provisional_labels);
        assert!(settings.post_call_refinement_enabled);
        assert!(settings.speaker_review_enabled);
    }

    #[tokio::test]
    async fn settings_valid_typed_round_trip_uses_camel_case_dto() {
        let pool = database().await;
        let expected = live_settings();

        store_diarization_settings(&pool, expected.clone())
            .await
            .unwrap();

        assert_eq!(load_diarization_settings(&pool).await.unwrap(), expected);
        let json = serde_json::to_value(&expected).unwrap();
        assert_eq!(json["mode"], "live_plus_post_call");
        assert_eq!(json["overlapHandling"], "multiple_speakers");
        assert!(json.get("showProvisionalLabels").is_some());
        assert!(json.get("show_provisional_labels").is_none());
    }

    #[tokio::test]
    async fn settings_concurrent_saves_leave_one_complete_row() {
        let (_directory, pool) = concurrent_database().await;
        let first = live_settings();
        let second = DiarizationSettingsDto {
            enabled: false,
            mode: DiarizationMode::Off,
            show_provisional_labels: false,
            post_call_refinement_enabled: true,
            overlap_handling: OverlapHandling::MultipleSpeakers,
            speaker_review_enabled: true,
        };

        let (first_result, second_result) = tokio::join!(
            store_diarization_settings(&pool, first.clone()),
            store_diarization_settings(&pool, second.clone()),
        );
        first_result.unwrap();
        second_result.unwrap();

        let saved = load_diarization_settings(&pool).await.unwrap();
        assert!(
            saved == first || saved == second,
            "mixed settings row: {saved:?}"
        );
    }

    #[test]
    fn settings_raw_invalid_mode_and_overlap_are_rejected_by_serde() {
        let base = serde_json::json!({
            "enabled": false,
            "mode": "live_plus_post_call",
            "showProvisionalLabels": true,
            "postCallRefinementEnabled": true,
            "overlapHandling": "multiple_speakers",
            "speakerReviewEnabled": true
        });

        let mut invalid_mode = base.clone();
        invalid_mode["mode"] = serde_json::json!("surprise_mode");
        assert!(serde_json::from_value::<DiarizationSettingsDto>(invalid_mode).is_err());

        let mut invalid_overlap = base;
        invalid_overlap["overlapHandling"] = serde_json::json!("speaker_identities");
        assert!(serde_json::from_value::<DiarizationSettingsDto>(invalid_overlap).is_err());
    }

    #[tokio::test]
    async fn settings_enabled_with_off_is_rejected_before_sql_and_preserves_row() {
        let pool = database().await;
        let existing = live_settings();
        store_diarization_settings(&pool, existing.clone())
            .await
            .unwrap();

        let invalid = DiarizationSettingsDto {
            enabled: true,
            mode: DiarizationMode::Off,
            ..existing.clone()
        };
        let error = store_diarization_settings(&pool, invalid)
            .await
            .unwrap_err();

        assert_eq!(error, SAVE_SETTINGS_ERROR);
        assert_eq!(load_diarization_settings(&pool).await.unwrap(), existing);
    }

    #[tokio::test]
    async fn settings_database_errors_are_exact_and_sanitized() {
        let load_pool = database().await;
        sqlx::query("DROP TABLE diarization_settings")
            .execute(&load_pool)
            .await
            .unwrap();
        let load_error = load_diarization_settings(&load_pool).await.unwrap_err();
        assert_eq!(load_error, LOAD_SETTINGS_ERROR);
        assert!(!load_error.contains("no such table"));

        let save_pool = database().await;
        sqlx::query(
            "CREATE TRIGGER reject_diarization_settings
             BEFORE INSERT ON diarization_settings
             BEGIN
                 SELECT RAISE(ABORT, 'secret_table rejected persistence');
             END",
        )
        .execute(&save_pool)
        .await
        .unwrap();
        let save_error = store_diarization_settings(&save_pool, live_settings())
            .await
            .unwrap_err();
        assert_eq!(save_error, SAVE_SETTINGS_ERROR);
        assert!(!save_error.contains("secret_table"));
        assert!(!save_error.contains("trigger"));
    }

    #[tokio::test]
    async fn settings_corrupt_boolean_columns_return_exact_sanitized_load_error() {
        let pool = database().await;

        for (column, valid_value) in [
            ("enabled", 0),
            ("show_provisional_labels", 1),
            ("post_call_refinement_enabled", 1),
            ("speaker_review_enabled", 1),
        ] {
            sqlx::query("PRAGMA ignore_check_constraints = ON")
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query(&format!(
                "UPDATE diarization_settings SET {column} = 2 WHERE id = '1'"
            ))
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query("PRAGMA ignore_check_constraints = OFF")
                .execute(&pool)
                .await
                .unwrap();

            let error = load_diarization_settings(&pool).await.unwrap_err();
            assert_eq!(error, LOAD_SETTINGS_ERROR, "accepted corrupt {column}");
            assert!(!error.contains("SELECT"));
            assert!(!error.contains("SQLite"));

            sqlx::query("PRAGMA ignore_check_constraints = ON")
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query(&format!(
                "UPDATE diarization_settings SET {column} = {valid_value} WHERE id = '1'"
            ))
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query("PRAGMA ignore_check_constraints = OFF")
                .execute(&pool)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn concurrent_disable_between_preflight_and_transaction_preserves_diarization() {
        let (_directory, pool) = concurrent_database().await;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at)
             VALUES ('meeting-race', 'Race', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO transcripts (
                id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time,
                speaker_id, speaker_label, is_overlap, diarization_status, diarization_method
             ) VALUES ('transcript-race', 'meeting-race', 'Existing',
                       '2026-01-01T00:00:00Z', 0.0, 1.0, 'old-speaker', 'Old Speaker', 0,
                       'provisional', 'old-engine')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO speaker_segments (
                id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
                confidence, is_overlap, diarization_status, diarization_method
             ) VALUES ('old-segment', 'meeting-race', 'old-source', 0.0, 1.0,
                       'old-speaker', 'Old Speaker', 0.4, 0, 'provisional', 'old-engine')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO meeting_diarization_status (
                meeting_id, enabled, live_enabled, post_call_enabled, current_status
             ) VALUES ('meeting-race', 1, 1, 0, 'provisional')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE diarization_settings
             SET enabled = 1, mode = 'live_plus_post_call' WHERE id = '1'",
        )
        .execute(&pool)
        .await
        .unwrap();

        let request = ApplyDiarizationRequest {
            meeting_id: "meeting-race".to_string(),
            method: "race-engine".to_string(),
            run_kind: DiarizationRunKind::Live,
            segments: vec![SpeakerSegment {
                meeting_id: "meeting-race".to_string(),
                source: "new-source".to_string(),
                start_time: 0.0,
                end_time: 1.0,
                speaker_id: Some("new-speaker".to_string()),
                speaker_label: Some("New Speaker".to_string()),
                confidence: Some(0.9),
                is_overlap: false,
                diarization_status: DiarizationStatus::Final,
                diarization_method: None,
            }],
        };
        let preflight_reached = Arc::new(Barrier::new(2));
        let continue_apply = Arc::new(Barrier::new(2));
        let apply_pool = pool.clone();
        let apply_reached = Arc::clone(&preflight_reached);
        let apply_continue = Arc::clone(&continue_apply);
        let apply_task = tokio::spawn(async move {
            apply_diarization_segments_with_hook(&apply_pool, request, move || async move {
                apply_reached.wait().await;
                apply_continue.wait().await;
            })
            .await
        });

        preflight_reached.wait().await;
        sqlx::query("UPDATE diarization_settings SET enabled = 0, mode = 'off' WHERE id = '1'")
            .execute(&pool)
            .await
            .unwrap();
        continue_apply.wait().await;

        assert_eq!(
            apply_task.await.unwrap().unwrap_err(),
            FEATURE_DISABLED_ERROR
        );
        let segment_id: String =
            sqlx::query_scalar("SELECT id FROM speaker_segments WHERE meeting_id = 'meeting-race'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let status: String = sqlx::query_scalar(
            "SELECT current_status FROM meeting_diarization_status
             WHERE meeting_id = 'meeting-race'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let transcript: (Option<String>, Option<String>, String) = sqlx::query_as(
            "SELECT speaker_id, diarization_method, diarization_status
             FROM transcripts WHERE id = 'transcript-race'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(segment_id, "old-segment");
        assert_eq!(status, "provisional");
        assert_eq!(
            transcript,
            (
                Some("old-speaker".to_string()),
                Some("old-engine".to_string()),
                "provisional".to_string()
            )
        );
    }
}
