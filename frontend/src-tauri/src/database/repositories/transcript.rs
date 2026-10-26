use crate::api::TranscriptSegment;
use crate::database::repositories::diarization::clear_meeting_diarization;
use crate::database::repositories::search::SearchRepository;
use chrono::Utc;
use sqlx::{Connection, Error as SqlxError, SqliteConnection, SqlitePool};
use tracing::{error, info};
use uuid::Uuid;

pub struct TranscriptsRepository;

impl TranscriptsRepository {
    /// Saves a new meeting and its associated transcript segments.
    /// This function uses a transaction to ensure that either both the meeting
    /// and all its transcripts are saved, or none of them are.
    pub async fn save_transcript(
        pool: &SqlitePool,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
    ) -> Result<String, SqlxError> {
        Self::save_transcript_with_started_at(pool, meeting_title, transcripts, folder_path, None)
            .await
    }

    /// Saves a new meeting while preserving the source timestamp captured by
    /// the recording/import boundary when one is available.
    pub async fn save_transcript_with_started_at(
        pool: &SqlitePool,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
        started_at_ms: Option<u64>,
    ) -> Result<String, SqlxError> {
        Self::save_transcript_with_started_at_and_vocabulary(
            pool,
            meeting_title,
            transcripts,
            folder_path,
            started_at_ms,
            None,
        )
        .await
    }

    /// Saves a new meeting while preserving the source timestamp and the
    /// vocabulary captured for the recording, when one is available.
    pub async fn save_transcript_with_started_at_and_vocabulary(
        pool: &SqlitePool,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
        started_at_ms: Option<u64>,
        meeting_vocabulary: Option<&str>,
    ) -> Result<String, SqlxError> {
        let meeting_id = format!("meeting-{}", Uuid::new_v4());

        Self::save_transcript_with_id_and_started_at_and_vocabulary(
            pool,
            &meeting_id,
            meeting_title,
            transcripts,
            folder_path,
            started_at_ms,
            meeting_vocabulary,
        )
        .await
    }

    /// Saves a meeting using a caller-provided id. Repeating the same id is a
    /// successful no-op, allowing crash recovery to retry without duplicates.
    pub async fn save_transcript_with_id(
        pool: &SqlitePool,
        meeting_id: &str,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
    ) -> Result<String, SqlxError> {
        Self::save_transcript_with_id_and_started_at(
            pool,
            meeting_id,
            meeting_title,
            transcripts,
            folder_path,
            None,
        )
        .await
    }

    /// Saves a meeting using a caller-provided id and source timestamp.
    pub async fn save_transcript_with_id_and_started_at(
        pool: &SqlitePool,
        meeting_id: &str,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
        started_at_ms: Option<u64>,
    ) -> Result<String, SqlxError> {
        Self::save_transcript_with_id_and_started_at_and_vocabulary(
            pool,
            meeting_id,
            meeting_title,
            transcripts,
            folder_path,
            started_at_ms,
            None,
        )
        .await
    }

    /// Saves a meeting using a caller-provided id and preserves the exact
    /// vocabulary captured for that recording when the row is first created.
    pub async fn save_transcript_with_id_and_started_at_and_vocabulary(
        pool: &SqlitePool,
        meeting_id: &str,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
        started_at_ms: Option<u64>,
        meeting_vocabulary: Option<&str>,
    ) -> Result<String, SqlxError> {
        if meeting_id.trim().is_empty() {
            return Err(SqlxError::Protocol(
                "meeting_id cannot be empty".to_string(),
            ));
        }

        let meeting_vocabulary =
            crate::database::repositories::setting::normalize_vocabulary(meeting_vocabulary)
                .map_err(SqlxError::Protocol)?;

        let mut conn = pool.acquire().await?;
        let mut transaction = conn.begin().await?;

        let started_at_ms = started_at_ms.and_then(|value| i64::try_from(value).ok());
        let now = Utc::now();
        let created_at = started_at_ms
            .and_then(chrono::DateTime::<Utc>::from_timestamp_millis)
            .unwrap_or(now);

        // 1. Create the new meeting
        let result = sqlx::query(
            "INSERT INTO meetings (
                id, title, created_at, updated_at, folder_path, title_source, started_at_ms,
                status, duration_seconds, lifecycle_error
             ) VALUES (?, ?, ?, ?, ?, 'suggested', ?, 'ready', NULL, NULL)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(meeting_id)
        .bind(meeting_title)
        .bind(created_at)
        .bind(now)
        .bind(&folder_path)
        .bind(started_at_ms)
        .execute(&mut *transaction)
        .await;

        let result = match result {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to create meeting '{}': {}", meeting_title, e);
                transaction.rollback().await?;
                return Err(e);
            }
        };

        let existing_placeholder = if result.rows_affected() == 0 {
            let transcript_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?",
            )
            .bind(meeting_id)
            .fetch_one(&mut *transaction)
            .await?;
            // The lifecycle row may already be terminal before this first
            // save (recording finalization persists `ready` early). The
            // transcript rows are the durable first-save marker; lifecycle
            // status must not decide whether the payload is discarded.
            transcript_count == 0
        } else {
            false
        };

        if result.rows_affected() == 0 && !existing_placeholder {
            // A retry must not replace title or timestamps. If an older save
            // lacked timing, fill that one missing value from a later recovery
            // attempt; otherwise leave the existing row unchanged.
            if let Some(started_at_ms) = started_at_ms {
                let created_at =
                    chrono::DateTime::<Utc>::from_timestamp_millis(started_at_ms).unwrap_or(now);
                sqlx::query(
                    "UPDATE meetings
                     SET started_at_ms = ?, created_at = ?
                     WHERE id = ? AND started_at_ms IS NULL",
                )
                .bind(started_at_ms)
                .bind(created_at)
                .bind(meeting_id)
                .execute(&mut *transaction)
                .await?;
            }
            // A duplicate save may arrive after another operation claimed the
            // row. Preserve that active/error owner so a late frontend retry
            // cannot make processing appear complete or hide a stop failure.
            sqlx::query(
                "UPDATE meetings
                 SET folder_path = COALESCE(?, folder_path),
                     status = CASE
                         WHEN status IN ('recording', 'processing', 'error') THEN status
                         ELSE 'ready'
                     END,
                     lifecycle_error = CASE
                         WHEN status IN ('recording', 'processing', 'error') THEN lifecycle_error
                         ELSE NULL
                     END,
                     updated_at = ?
                 WHERE id = ?",
            )
            .bind(&folder_path)
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
            info!(
                "Meeting {} already exists; treating save as idempotent",
                meeting_id
            );
            return Ok(meeting_id.to_string());
        }

        info!("Successfully created meeting with id: {}", meeting_id);

        if let Some(vocabulary) = meeting_vocabulary {
            sqlx::query(
                "INSERT INTO meeting_transcript_vocabulary (meeting_id, vocabulary, updated_at)
                 VALUES (?, ?, CURRENT_TIMESTAMP)
                 ON CONFLICT(meeting_id) DO UPDATE SET
                     vocabulary = excluded.vocabulary,
                     updated_at = CURRENT_TIMESTAMP",
            )
            .bind(meeting_id)
            .bind(vocabulary)
            .execute(&mut *transaction)
            .await?;
        }

        // 2. Save each transcript segment with audio timing fields
        for segment in transcripts {
            let transcript_id = format!("transcript-{}", Uuid::new_v4());
            let result = sqlx::query(
                "INSERT INTO transcripts (
                    id, meeting_id, transcript, timestamp,
                    audio_start_time, audio_end_time, duration,
                    speaker_id, speaker_label, speaker_color, is_overlap,
                    diarization_status, diarization_method, diarization_confidence
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&transcript_id)
            .bind(meeting_id)
            .bind(&segment.text)
            .bind(&segment.timestamp)
            .bind(segment.audio_start_time)
            .bind(segment.audio_end_time)
            .bind(segment.duration)
            .bind(&segment.speaker_id)
            .bind(&segment.speaker_label)
            .bind(&segment.speaker_color)
            .bind(segment.is_overlap.unwrap_or(false) as i64)
            .bind(segment.diarization_status.as_deref().unwrap_or("none"))
            .bind(&segment.diarization_method)
            .bind(segment.diarization_confidence)
            .execute(&mut *transaction)
            .await;

            if let Err(e) = result {
                error!(
                    "Failed to save transcript segment for meeting {}: {}",
                    meeting_id, e
                );
                transaction.rollback().await?;
                return Err(e);
            }
        }

        info!(
            "Successfully saved {} transcript segments for meeting {}",
            transcripts.len(),
            meeting_id
        );

        // A transcript save is terminal only while no newer operation owns the
        // row. Keep recording, processing, and error visible to the list while
        // preserving the original title, start time, duration, and artifacts.
        sqlx::query(
            "UPDATE meetings
             SET folder_path = COALESCE(?, folder_path),
                 status = CASE
                     WHEN status IN ('recording', 'processing', 'error') THEN status
                     ELSE 'ready'
                 END,
                 lifecycle_error = CASE
                     WHEN status IN ('recording', 'processing', 'error') THEN lifecycle_error
                     ELSE NULL
                 END,
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(&folder_path)
        .bind(now)
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;

        // Commit the transaction
        transaction.commit().await?;
        drop(conn);

        // Keep the durable search index current for callers that search
        // immediately after saving a recording. Search also rebuilds before
        // reading, which repairs any legacy/direct-write path.
        SearchRepository::rebuild(pool).await?;

        Ok(meeting_id.to_string())
    }

    /// Replace transcript rows during retranscription inside the caller's transaction.
    /// Incoming diarization metadata is deliberately ignored because the new audio pass
    /// has not been diarized yet.
    pub async fn replace_for_retranscription(
        connection: &mut SqliteConnection,
        meeting_id: &str,
        transcripts: &[TranscriptSegment],
    ) -> Result<(), SqlxError> {
        clear_meeting_diarization(&mut *connection, meeting_id).await?;

        sqlx::query("DELETE FROM transcripts WHERE meeting_id = ?")
            .bind(meeting_id)
            .execute(&mut *connection)
            .await?;

        for segment in transcripts {
            sqlx::query(
                "INSERT INTO transcripts (
                    id, meeting_id, transcript, timestamp,
                    audio_start_time, audio_end_time, duration,
                    speaker_id, speaker_label, speaker_color, is_overlap,
                    diarization_status, diarization_method, diarization_confidence
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, 0, 'none', NULL, NULL)",
            )
            .bind(&segment.id)
            .bind(meeting_id)
            .bind(&segment.text)
            .bind(&segment.timestamp)
            .bind(segment.audio_start_time)
            .bind(segment.audio_end_time)
            .bind(segment.duration)
            .execute(&mut *connection)
            .await?;
        }

        Ok(())
    }

    /// Searches all user-visible meeting content.
    pub async fn search_transcripts(
        pool: &SqlitePool,
        query: &str,
    ) -> Result<Vec<crate::api::TranscriptSearchResult>, SqlxError> {
        SearchRepository::search(pool, query).await
    }
}
