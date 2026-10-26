use crate::api::{MeetingDetails, MeetingTranscript};
use crate::database::models::{MeetingModel, Transcript};
use crate::database::repositories::diarization::clear_meeting_diarization;
use crate::database::repositories::search::SearchRepository;
use chrono::Utc;
use sqlx::{Connection, Error as SqlxError, SqliteConnection, SqlitePool};
use std::path::PathBuf;
use tracing::{error, info};
use uuid::Uuid;

pub struct MeetingsRepository;

pub const MEETING_STATUS_RECORDING: &str = "recording";
pub const MEETING_STATUS_PROCESSING: &str = "processing";
pub const MEETING_STATUS_READY: &str = "ready";
pub const MEETING_STATUS_ERROR: &str = "error";
pub const LIFECYCLE_OPERATION_RECORDING: &str = "recording";
pub const LIFECYCLE_OPERATION_IMPORT: &str = "import";
pub const LIFECYCLE_OPERATION_SUMMARY: &str = "summary";
pub const LIFECYCLE_OPERATION_RETRANSCRIPTION: &str = "retranscription";
pub const INTERRUPTED_MEETING_ERROR: &str =
    "This meeting was interrupted while it was being processed";

const MEETING_COLUMNS: &str =
    "id, title, created_at, updated_at, folder_path, title_source, started_at_ms, status, duration_seconds, lifecycle_error, lifecycle_operation, lifecycle_run_id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeetingRecordingResolution {
    Ready(PathBuf),
    MeetingNotFound,
    RecordingActive,
    FolderUnavailable,
    RecordingUnavailable,
    RecordingIncomplete,
}

impl MeetingsRepository {
    pub async fn resolve_completed_recording(
        pool: &SqlitePool,
        meeting_id: &str,
        recording_active: bool,
    ) -> Result<MeetingRecordingResolution, SqlxError> {
        let folder_path = sqlx::query_scalar::<_, Option<String>>(
            "SELECT folder_path FROM meetings WHERE id = ?",
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await?;

        let Some(folder_path) = folder_path else {
            return Ok(MeetingRecordingResolution::MeetingNotFound);
        };
        if recording_active {
            return Ok(MeetingRecordingResolution::RecordingActive);
        }
        let Some(folder_path) = folder_path.filter(|path| !path.trim().is_empty()) else {
            return Ok(MeetingRecordingResolution::FolderUnavailable);
        };

        let folder = PathBuf::from(folder_path);
        if !folder.is_dir() {
            return Ok(MeetingRecordingResolution::FolderUnavailable);
        }

        for file_name in ["audio.mp4", "audio.wav"] {
            let candidate = folder.join(file_name);
            if !candidate.exists() {
                continue;
            }
            let metadata = match std::fs::metadata(&candidate) {
                Ok(metadata) => metadata,
                Err(_) => return Ok(MeetingRecordingResolution::RecordingUnavailable),
            };
            if !metadata.is_file() || metadata.len() == 0 {
                return Ok(MeetingRecordingResolution::RecordingIncomplete);
            }
            return Ok(MeetingRecordingResolution::Ready(candidate));
        }

        if folder.join(".checkpoints").is_dir() {
            Ok(MeetingRecordingResolution::RecordingIncomplete)
        } else {
            Ok(MeetingRecordingResolution::RecordingUnavailable)
        }
    }

    pub async fn get_meetings(pool: &SqlitePool) -> Result<Vec<MeetingModel>, sqlx::Error> {
        let meetings = sqlx::query_as::<_, MeetingModel>(&format!(
            "SELECT {} FROM meetings ORDER BY COALESCE(started_at_ms, CAST(strftime('%s', created_at) AS INTEGER) * 1000) DESC, created_at DESC",
            MEETING_COLUMNS
        ))
        .fetch_all(pool)
        .await?;
        Ok(meetings)
    }

    /// Create the durable row before a recording or batch job can produce its
    /// first transcript. Reusing an id is safe and keeps manual title/time
    /// provenance intact.
    pub async fn ensure_lifecycle_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
        title: &str,
        started_at_ms: Option<i64>,
        title_source: &str,
        status: &str,
        lifecycle_operation: &str,
    ) -> Result<String, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let now = Utc::now();
        let lifecycle_run_id = Uuid::new_v4().to_string();
        let created_at = started_at_ms
            .and_then(chrono::DateTime::<Utc>::from_timestamp_millis)
            .unwrap_or(now);
        sqlx::query(
            "INSERT INTO meetings (
                id, title, created_at, updated_at, folder_path, title_source,
                started_at_ms, status, duration_seconds, lifecycle_error, lifecycle_operation,
                lifecycle_run_id
             ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, 0.0, NULL, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                title = CASE WHEN meetings.title_source = 'manual' THEN meetings.title ELSE excluded.title END,
                title_source = CASE WHEN meetings.title_source = 'manual' THEN meetings.title_source ELSE excluded.title_source END,
                started_at_ms = COALESCE(meetings.started_at_ms, excluded.started_at_ms),
                status = excluded.status,
                lifecycle_error = NULL,
                lifecycle_operation = excluded.lifecycle_operation,
                lifecycle_run_id = excluded.lifecycle_run_id,
                updated_at = excluded.updated_at",
        )
        .bind(meeting_id)
        .bind(title)
        .bind(created_at)
        .bind(now)
        .bind(title_source)
        .bind(started_at_ms)
        .bind(status)
        .bind(lifecycle_operation)
        .bind(&lifecycle_run_id)
        .execute(pool)
        .await?;
        Ok(lifecycle_run_id)
    }

    /// Update the list-facing lifecycle without touching transcripts,
    /// summaries, title provenance, or the original start time.
    pub async fn update_lifecycle(
        pool: &SqlitePool,
        meeting_id: &str,
        status: &str,
        duration_seconds: Option<f64>,
        lifecycle_error: Option<&str>,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "UPDATE meetings
             SET status = ?,
                 duration_seconds = COALESCE(?, duration_seconds),
                 lifecycle_error = ?,
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(status)
        .bind(duration_seconds)
        .bind(lifecycle_error)
        .bind(Utc::now())
        .bind(meeting_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Update a lifecycle row only while the requested operation run still owns it.
    /// The run id remains stored after a terminal state so a late result can
    /// never take ownership merely because the row is no longer processing.
    pub async fn update_lifecycle_for_operation(
        pool: &SqlitePool,
        meeting_id: &str,
        operation: &str,
        lifecycle_run_id: &str,
        status: &str,
        duration_seconds: Option<f64>,
        lifecycle_error: Option<&str>,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "UPDATE meetings
             SET status = ?,
                 duration_seconds = COALESCE(?, duration_seconds),
                 lifecycle_error = ?,
                 updated_at = ?
             WHERE id = ?
               AND lifecycle_operation = ?
               AND lifecycle_run_id = ?
               AND status IN ('recording', 'processing')",
        )
        .bind(status)
        .bind(duration_seconds)
        .bind(lifecycle_error)
        .bind(Utc::now())
        .bind(meeting_id)
        .bind(operation)
        .bind(lifecycle_run_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Complete an interrupted recording recovery only when the failed row is
    /// still owned by the same recording run. Terminal errors from a newer
    /// operation remain untouched.
    pub async fn recover_lifecycle_for_operation(
        pool: &SqlitePool,
        meeting_id: &str,
        operation: &str,
        lifecycle_run_id: &str,
        duration_seconds: Option<f64>,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "UPDATE meetings
             SET status = ?,
                 duration_seconds = COALESCE(?, duration_seconds),
                 lifecycle_error = NULL,
                 updated_at = ?
             WHERE id = ?
               AND lifecycle_operation = ?
               AND lifecycle_run_id = ?
               AND status = ?",
        )
        .bind(MEETING_STATUS_READY)
        .bind(duration_seconds)
        .bind(Utc::now())
        .bind(meeting_id)
        .bind(operation)
        .bind(lifecycle_run_id)
        .bind(MEETING_STATUS_ERROR)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Claim a new processing owner and return its unique run id. A newer batch
    /// operation may replace an older batch owner, while an active recording
    /// remains authoritative until capture has finished.
    pub async fn claim_lifecycle_operation(
        pool: &SqlitePool,
        meeting_id: &str,
        operation: &str,
    ) -> Result<Option<String>, SqlxError> {
        let lifecycle_run_id = Uuid::new_v4().to_string();
        let result = sqlx::query(
            "UPDATE meetings
             SET status = ?,
                 lifecycle_error = NULL,
                 lifecycle_operation = ?,
                 lifecycle_run_id = ?,
                 updated_at = ?
             WHERE id = ?
               AND NOT (
                   status IN ('recording', 'processing')
                   AND lifecycle_operation = 'recording'
               )",
        )
        .bind(MEETING_STATUS_PROCESSING)
        .bind(operation)
        .bind(&lifecycle_run_id)
        .bind(Utc::now())
        .bind(meeting_id)
        .execute(pool)
        .await?;
        Ok((result.rows_affected() == 1).then_some(lifecycle_run_id))
    }

    pub async fn get_lifecycle_run_id(
        pool: &SqlitePool,
        meeting_id: &str,
        operation: &str,
    ) -> Result<Option<String>, SqlxError> {
        sqlx::query_scalar(
            "SELECT lifecycle_run_id
             FROM meetings
             WHERE id = ? AND lifecycle_operation = ?",
        )
        .bind(meeting_id)
        .bind(operation)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_lifecycle_folder(
        pool: &SqlitePool,
        meeting_id: &str,
        folder_path: &str,
    ) -> Result<bool, SqlxError> {
        let result =
            sqlx::query("UPDATE meetings SET folder_path = ?, updated_at = ? WHERE id = ?")
                .bind(folder_path)
                .bind(Utc::now())
                .bind(meeting_id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Mark rows that had no live owner when the database was reopened. Their
    /// transcript and summary rows are deliberately left untouched.
    pub async fn recover_interrupted_lifecycle(pool: &SqlitePool) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            "UPDATE meetings
             SET status = ?, lifecycle_error = ?, updated_at = ?, lifecycle_operation = NULL,
                 lifecycle_run_id = NULL
             WHERE status IN (?, ?)",
        )
        .bind(MEETING_STATUS_ERROR)
        .bind(INTERRUPTED_MEETING_ERROR)
        .bind(Utc::now())
        .bind(MEETING_STATUS_RECORDING)
        .bind(MEETING_STATUS_PROCESSING)
        .execute(pool)
        .await?;
        if result.rows_affected() > 0 {
            info!(
                "Recovered {} interrupted meeting lifecycle row(s)",
                result.rows_affected()
            );
        }
        Ok(result.rows_affected())
    }

    pub async fn delete_meeting(pool: &SqlitePool, meeting_id: &str) -> Result<bool, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let mut conn = pool.acquire().await?;
        let mut transaction = conn.begin().await?;

        match delete_meeting_with_transaction(&mut transaction, meeting_id).await {
            Ok(success) => {
                if success {
                    transaction.commit().await?;
                    drop(conn);
                    SearchRepository::rebuild(pool).await?;
                    info!(
                        "Successfully deleted meeting {} and all associated data",
                        meeting_id
                    );
                    Ok(true)
                } else {
                    transaction.rollback().await?;
                    Ok(false)
                }
            }
            Err(e) => {
                let _ = transaction.rollback().await;
                error!("Failed to delete meeting {}: {}", meeting_id, e);
                Err(e)
            }
        }
    }

    pub async fn get_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<MeetingDetails>, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let mut conn = pool.acquire().await?;
        let mut transaction = conn.begin().await?;

        // Get meeting details
        let meeting: Option<MeetingModel> = sqlx::query_as(&format!(
            "SELECT {} FROM meetings WHERE id = ?",
            MEETING_COLUMNS
        ))
        .bind(meeting_id)
        .fetch_optional(&mut *transaction)
        .await?;

        if meeting.is_none() {
            transaction.rollback().await?;
            return Err(SqlxError::RowNotFound);
        }

        if let Some(meeting) = meeting {
            // Get all transcripts for this meeting
            let transcripts =
                sqlx::query_as::<_, Transcript>("SELECT * FROM transcripts WHERE meeting_id = ?")
                    .bind(meeting_id)
                    .fetch_all(&mut *transaction)
                    .await?;

            transaction.commit().await?;

            // Convert Transcript to MeetingTranscript
            let meeting_transcripts = transcripts
                .into_iter()
                .map(|t| MeetingTranscript {
                    id: t.id,
                    text: t.transcript,
                    timestamp: t.timestamp,
                    audio_start_time: t.audio_start_time,
                    audio_end_time: t.audio_end_time,
                    duration: t.duration,
                    speaker_id: t.speaker_id,
                    speaker_label: t.speaker_label,
                    speaker_color: t.speaker_color,
                    is_overlap: t.is_overlap.map(|value| value != 0),
                    diarization_status: t.diarization_status,
                    diarization_method: t.diarization_method,
                    diarization_confidence: t.diarization_confidence,
                })
                .collect::<Vec<_>>();

            Ok(Some(MeetingDetails {
                id: meeting.id,
                title: meeting.title,
                created_at: meeting.created_at.0.to_rfc3339(),
                updated_at: meeting.updated_at.0.to_rfc3339(),
                transcripts: meeting_transcripts,
            }))
        } else {
            transaction.rollback().await?;
            Ok(None)
        }
    }

    /// Get meeting metadata without transcripts (for pagination)
    pub async fn get_meeting_metadata(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<MeetingModel>, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let meeting: Option<MeetingModel> = sqlx::query_as(&format!(
            "SELECT {} FROM meetings WHERE id = ?",
            MEETING_COLUMNS
        ))
        .bind(meeting_id)
        .fetch_optional(pool)
        .await?;

        Ok(meeting)
    }

    /// Get meeting transcripts with pagination support
    pub async fn get_meeting_transcripts_paginated(
        pool: &SqlitePool,
        meeting_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Transcript>, i64), SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        // Get total count of transcripts for this meeting
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_one(pool)
            .await?;

        // Get paginated transcripts ordered by audio_start_time
        let transcripts = sqlx::query_as::<_, Transcript>(
            "SELECT * FROM transcripts
             WHERE meeting_id = ?
             ORDER BY audio_start_time ASC
             LIMIT ? OFFSET ?",
        )
        .bind(meeting_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok((transcripts, total.0))
    }

    pub async fn update_meeting_title(
        pool: &SqlitePool,
        meeting_id: &str,
        new_title: &str,
    ) -> Result<bool, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let mut conn = pool.acquire().await?;
        let mut transaction = conn.begin().await?;

        let now = Utc::now().naive_utc();

        let rows_affected = sqlx::query(
            "UPDATE meetings
             SET title = ?, title_source = 'manual', updated_at = ?
             WHERE id = ?",
        )
        .bind(new_title)
        .bind(now)
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;
        if rows_affected.rows_affected() == 0 {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction.commit().await?;
        drop(conn);
        SearchRepository::rebuild(pool).await?;
        Ok(true)
    }

    pub async fn update_meeting_name(
        pool: &SqlitePool,
        meeting_id: &str,
        new_title: &str,
    ) -> Result<bool, SqlxError> {
        Self::update_suggested_meeting_name(pool, meeting_id, new_title).await
    }

    /// Apply a title suggested by automatic summary generation.
    ///
    /// The title and transcript chunk update share one transaction and the
    /// manual-title predicate is part of the UPDATE itself. This prevents a
    /// late summary task from replacing a title saved by the user after the
    /// summary started.
    pub async fn update_suggested_meeting_name(
        pool: &SqlitePool,
        meeting_id: &str,
        new_title: &str,
    ) -> Result<bool, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let mut transaction = pool.begin().await?;
        let now = Utc::now();

        // Update meetings table
        let meeting_update = sqlx::query(
            "UPDATE meetings
             SET title = ?, title_source = 'suggested', updated_at = ?
             WHERE id = ? AND title_source = 'suggested'",
        )
        .bind(new_title)
        .bind(now)
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;

        if meeting_update.rows_affected() == 0 {
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT 1 FROM meetings WHERE id = ? LIMIT 1")
                    .bind(meeting_id)
                    .fetch_optional(&mut *transaction)
                    .await?
                    .is_some();
            transaction.commit().await?;
            return Ok(exists);
        }

        // Update transcript_chunks table
        sqlx::query("UPDATE transcript_chunks SET meeting_name = ? WHERE meeting_id = ?")
            .bind(new_title)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        SearchRepository::rebuild(pool).await?;
        Ok(true)
    }
}

async fn delete_meeting_with_transaction(
    transaction: &mut SqliteConnection,
    meeting_id: &str,
) -> Result<bool, SqlxError> {
    // Check if meeting exists
    let meeting_exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM meetings WHERE id = ?")
        .bind(meeting_id)
        .fetch_optional(&mut *transaction)
        .await?;

    if meeting_exists.is_none() {
        error!("Meeting {} not found for deletion", meeting_id);
        return Ok(false);
    }

    // Delete from related tables in proper order
    // 1. Explicitly clear diarization even when SQLite foreign keys are disabled.
    clear_meeting_diarization(&mut *transaction, meeting_id).await?;

    // 2. Delete from transcript_chunks
    sqlx::query("DELETE FROM transcript_chunks WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;

    // 3. Delete from summary_processes
    sqlx::query("DELETE FROM summary_processes WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;

    // 4. Delete from transcripts
    sqlx::query("DELETE FROM transcripts WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;

    // 5. Finally, delete the meeting
    let result = sqlx::query("DELETE FROM meetings WHERE id = ?")
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;

    Ok(result.rows_affected() > 0)
}
