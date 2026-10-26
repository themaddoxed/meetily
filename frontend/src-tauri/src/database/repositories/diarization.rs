use crate::database::models::DiarizationSetting;
use crate::diarization::{
    types::{DiarizationStatus, SpeakerAssignment, SpeakerSegment},
    DiarizationMode, OverlapHandling,
};
use sqlx::{Executor, Sqlite, SqlitePool, Transaction};

pub struct DiarizationRepository;

impl DiarizationRepository {
    pub async fn get_settings(pool: &SqlitePool) -> Result<DiarizationSetting, sqlx::Error> {
        let existing = sqlx::query_as::<_, DiarizationSetting>(
            "SELECT id, enabled, mode, show_provisional_labels,
                    post_call_refinement_enabled, overlap_handling,
                    speaker_review_enabled, updated_at
             FROM diarization_settings WHERE id = '1'",
        )
        .fetch_optional(pool)
        .await?;

        if let Some(settings) = existing {
            return Ok(settings);
        }

        sqlx::query(
            "INSERT INTO diarization_settings (id) VALUES ('1') ON CONFLICT(id) DO NOTHING",
        )
        .execute(pool)
        .await?;

        sqlx::query_as::<_, DiarizationSetting>(
            "SELECT id, enabled, mode, show_provisional_labels,
                    post_call_refinement_enabled, overlap_handling,
                    speaker_review_enabled, updated_at
             FROM diarization_settings WHERE id = '1'",
        )
        .fetch_one(pool)
        .await
    }

    pub(crate) async fn get_settings_in_transaction(
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<DiarizationSetting, sqlx::Error> {
        sqlx::query_as::<_, DiarizationSetting>(
            "SELECT id, enabled, mode, show_provisional_labels,
                    post_call_refinement_enabled, overlap_handling,
                    speaker_review_enabled, updated_at
             FROM diarization_settings WHERE id = '1'",
        )
        .fetch_one(&mut **transaction)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn save_settings(
        pool: &SqlitePool,
        enabled: bool,
        mode: DiarizationMode,
        show_provisional_labels: bool,
        post_call_refinement_enabled: bool,
        overlap_handling: OverlapHandling,
        speaker_review_enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO diarization_settings (
                id, enabled, mode, show_provisional_labels,
                post_call_refinement_enabled, overlap_handling,
                speaker_review_enabled, updated_at
             ) VALUES ('1', ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET
                enabled = excluded.enabled,
                mode = excluded.mode,
                show_provisional_labels = excluded.show_provisional_labels,
                post_call_refinement_enabled = excluded.post_call_refinement_enabled,
                overlap_handling = excluded.overlap_handling,
                speaker_review_enabled = excluded.speaker_review_enabled,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(enabled as i64)
        .bind(mode.as_database_value())
        .bind(show_provisional_labels as i64)
        .bind(post_call_refinement_enabled as i64)
        .bind(overlap_handling.as_database_value())
        .bind(speaker_review_enabled as i64)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn replace_speaker_segments(
        transaction: &mut Transaction<'_, Sqlite>,
        meeting_id: &str,
        method: &str,
        segments: &[SpeakerSegment],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM speaker_segments WHERE meeting_id = ?")
            .bind(meeting_id)
            .execute(&mut **transaction)
            .await?;

        for (index, segment) in segments.iter().enumerate() {
            let segment_id = format!("diarization:{meeting_id}:{index}");
            let segment_method = segment.diarization_method.as_deref().unwrap_or(method);

            sqlx::query(
                "INSERT INTO speaker_segments (
                    id, meeting_id, source, start_time, end_time,
                    speaker_id, speaker_label, confidence, is_overlap,
                    diarization_status, diarization_method
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(segment_id)
            .bind(meeting_id)
            .bind(&segment.source)
            .bind(segment.start_time)
            .bind(segment.end_time)
            .bind(&segment.speaker_id)
            .bind(&segment.speaker_label)
            .bind(segment.confidence)
            .bind(segment.is_overlap as i64)
            .bind(diarization_status_to_database_value(
                segment.diarization_status,
            ))
            .bind(segment_method)
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn upsert_meeting_diarization_status(
        transaction: &mut Transaction<'_, Sqlite>,
        meeting_id: &str,
        live_enabled: bool,
        post_call_enabled: bool,
        current_status: DiarizationStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO meeting_diarization_status (
                meeting_id, enabled, live_enabled, post_call_enabled,
                current_status, quality_flags, processed_at, updated_at
             ) VALUES (?, 1, ?, ?, ?, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(meeting_id) DO UPDATE SET
                enabled = 1,
                live_enabled = excluded.live_enabled,
                post_call_enabled = excluded.post_call_enabled,
                current_status = excluded.current_status,
                quality_flags = NULL,
                processed_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(meeting_id)
        .bind(live_enabled as i64)
        .bind(post_call_enabled as i64)
        .bind(diarization_status_to_database_value(current_status))
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    pub(crate) async fn update_transcript_assignment(
        transaction: &mut Transaction<'_, Sqlite>,
        meeting_id: &str,
        transcript_id: &str,
        assignment: &SpeakerAssignment,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            "UPDATE transcripts SET
                speaker_id = ?,
                speaker_label = ?,
                speaker_color = ?,
                is_overlap = ?,
                diarization_status = ?,
                diarization_method = ?,
                diarization_confidence = ?
             WHERE id = ? AND meeting_id = ?",
        )
        .bind(&assignment.speaker_id)
        .bind(&assignment.speaker_label)
        .bind(&assignment.speaker_color)
        .bind(assignment.is_overlap as i64)
        .bind(diarization_status_to_database_value(
            assignment.diarization_status,
        ))
        .bind(&assignment.diarization_method)
        .bind(assignment.diarization_confidence)
        .bind(transcript_id)
        .bind(meeting_id)
        .execute(&mut **transaction)
        .await?;

        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    pub async fn rename_speaker(
        pool: &SqlitePool,
        meeting_id: &str,
        speaker_id: &str,
        speaker_label: &str,
    ) -> Result<u64, sqlx::Error> {
        let mut transaction = pool.begin().await?;

        let transcript_result = sqlx::query(
            "UPDATE transcripts
             SET speaker_label = ?
             WHERE meeting_id = ? AND speaker_id = ? AND is_overlap = 0",
        )
        .bind(speaker_label)
        .bind(meeting_id)
        .bind(speaker_id)
        .execute(&mut *transaction)
        .await?;

        let segment_result = sqlx::query(
            "UPDATE speaker_segments
             SET speaker_label = ?
             WHERE meeting_id = ? AND speaker_id = ? AND is_overlap = 0",
        )
        .bind(speaker_label)
        .bind(meeting_id)
        .bind(speaker_id)
        .execute(&mut *transaction)
        .await?;

        let updated_count = transcript_result.rows_affected() + segment_result.rows_affected();
        if updated_count == 0 {
            transaction.rollback().await?;
            return Ok(0);
        }

        transaction.commit().await?;
        Ok(updated_count)
    }
}

pub(crate) const fn diarization_status_to_database_value(
    status: DiarizationStatus,
) -> &'static str {
    match status {
        DiarizationStatus::None => "none",
        DiarizationStatus::Provisional => "provisional",
        DiarizationStatus::Final => "final",
        DiarizationStatus::FallbackToLive => "fallback_to_live",
        DiarizationStatus::Failed => "failed",
        DiarizationStatus::NeedsReview => "needs_review",
    }
}

/// Remove all persisted diarization for a meeting using the caller's connection.
///
/// The mutable executor is intentionally reborrowed for each statement so callers can
/// keep cleanup in the same deletion or retranscription transaction.
pub async fn clear_meeting_diarization<E>(
    executor: &mut E,
    meeting_id: &str,
) -> Result<(), sqlx::Error>
where
    for<'connection> &'connection mut E: Executor<'connection, Database = Sqlite>,
{
    sqlx::query("DELETE FROM speaker_segments WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *executor)
        .await?;

    sqlx::query("DELETE FROM meeting_diarization_status WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *executor)
        .await?;

    sqlx::query(
        "UPDATE transcripts
         SET speaker_id = NULL,
             speaker_label = NULL,
             speaker_color = NULL,
             is_overlap = 0,
             diarization_status = 'none',
             diarization_method = NULL,
             diarization_confidence = NULL
         WHERE meeting_id = ?",
    )
    .bind(meeting_id)
    .execute(&mut *executor)
    .await?;

    Ok(())
}
