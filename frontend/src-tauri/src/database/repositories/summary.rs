use crate::database::models::SummaryProcess;
use crate::database::repositories::meeting::{
    MeetingsRepository, LIFECYCLE_OPERATION_SUMMARY, MEETING_STATUS_ERROR, MEETING_STATUS_READY,
};
use crate::database::repositories::search::SearchRepository;
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{error, info as log_info};
use uuid::Uuid;

const SUMMARY_EXPORT_PENDING_PREFIX: &str = "summary_export_pending:";

pub struct SummaryProcessesRepository;

#[derive(Clone, Copy)]
pub struct SummaryProcessingMetrics {
    pub chunk_count: i64,
    pub processing_time: f64,
}

impl SummaryProcessesRepository {
    pub async fn recover_interrupted_processes(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'failed',
                error = CASE
                    WHEN error LIKE 'summary_export_pending:%'
                    THEN error || char(10) || 'summary_generation_error:Summary generation was interrupted by application restart'
                    ELSE 'Summary generation was interrupted by application restart'
                END,
                updated_at = ?,
                end_time = ?,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE status = 'PENDING'
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        let recovered = result.rows_affected();
        if recovered > 0 {
            log_info!(
                "Recovered {} interrupted summary process(es) after restart",
                recovered
            );
        }
        Ok(recovered)
    }

    /// Retrieves the current summary process state for a given meeting ID.
    pub async fn get_summary_data(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<SummaryProcess>, sqlx::Error> {
        sqlx::query_as::<_, SummaryProcess>("SELECT * FROM summary_processes WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn update_meeting_summary(
        pool: &SqlitePool,
        meeting_id: &str,
        summary: &Value,
    ) -> Result<bool, sqlx::Error> {
        let mut transaction = pool.begin().await?;

        let meeting_exists: bool = sqlx::query("SELECT 1 FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();

        if !meeting_exists {
            log_info!(
                "Attempted to save summary for a non-existent meeting_id: {}",
                meeting_id
            );
            transaction.rollback().await?;
            return Ok(false);
        }

        let result_json = serde_json::to_string(summary);
        if result_json.is_err() {
            error!("Can't convert the json to string for saving to Database");
            transaction.rollback().await?;
            return Ok(false);
        }
        let now = Utc::now();

        sqlx::query("UPDATE summary_processes SET result = ?, updated_at = ? WHERE meeting_id = ?")
            .bind(result_json.unwrap())
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query("UPDATE meetings SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        SearchRepository::rebuild_in_transaction(&mut transaction).await?;
        transaction.commit().await?;

        log_info!(
            "Successfully updated summary and timestamp for meeting_id: {}",
            meeting_id
        );
        Ok(true)
    }

    /// Accept a user-edited summary and give it a new lifecycle owner so any
    /// still-running generation can no longer publish over the edit.
    pub async fn save_manual_summary(
        pool: &SqlitePool,
        meeting_id: &str,
        summary: &Value,
        export_pending: bool,
    ) -> Result<(String, Option<String>), sqlx::Error> {
        let result_json = serde_json::to_string(summary)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize summary: {}", e)))?;
        let run_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let pending_error = export_pending
            .then(|| format!("{}manual summary export", SUMMARY_EXPORT_PENDING_PREFIX));
        let mut transaction = pool.begin().await?;

        let previous_owner = sqlx::query_scalar::<_, String>(
            "SELECT lifecycle_run_id FROM meetings
             WHERE id = ? AND lifecycle_operation = ? AND status = ?",
        )
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(crate::database::repositories::meeting::MEETING_STATUS_PROCESSING)
        .fetch_optional(&mut *transaction)
        .await?;

        let meeting_update = sqlx::query(
            "UPDATE meetings
             SET status = ?, lifecycle_error = NULL, lifecycle_operation = ?, lifecycle_run_id = ?, updated_at = ?
             WHERE id = ?
               AND NOT (status IN ('recording', 'processing') AND lifecycle_operation = 'recording')",
        )
        .bind(MEETING_STATUS_READY)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(&run_id)
        .bind(now)
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;
        if meeting_update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        sqlx::query(
            r#"
            INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, start_time, end_time, result, error, result_backup, result_backup_timestamp)
            VALUES (?, 'completed', ?, ?, ?, ?, ?, ?, NULL, NULL)
            ON CONFLICT(meeting_id) DO UPDATE SET
                status = 'completed', updated_at = excluded.updated_at,
                start_time = excluded.start_time, end_time = excluded.end_time,
                result_backup = CASE WHEN ? THEN summary_processes.result ELSE NULL END,
                result_backup_timestamp = CASE WHEN ? THEN excluded.updated_at ELSE NULL END,
                result = excluded.result, error = excluded.error
            "#,
        )
        .bind(meeting_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(result_json)
        .bind(pending_error)
        .bind(export_pending)
        .bind(export_pending)
        .execute(&mut *transaction)
        .await?;

        SearchRepository::rebuild_in_transaction(&mut transaction).await?;
        transaction.commit().await?;
        Ok((run_id, previous_owner))
    }

    /// Replace the lifecycle owner and summary row atomically before generation
    /// starts. The previous owner is returned so its provider call can stop.
    pub async fn start_process(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(String, Option<String>), sqlx::Error> {
        log_info!(
            "Creating or resetting summary process for meeting_id: {}",
            meeting_id
        );
        let run_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let mut transaction = pool.begin().await?;

        let previous_owner = sqlx::query_scalar::<_, String>(
            "SELECT lifecycle_run_id FROM meetings
             WHERE id = ? AND lifecycle_operation = ? AND status = ?",
        )
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(crate::database::repositories::meeting::MEETING_STATUS_PROCESSING)
        .fetch_optional(&mut *transaction)
        .await?;

        let claim = sqlx::query(
            "UPDATE meetings
             SET status = ?, lifecycle_error = NULL, lifecycle_operation = ?, lifecycle_run_id = ?, updated_at = ?
             WHERE id = ?
               AND NOT (status IN ('recording', 'processing') AND lifecycle_operation = 'recording')",
        )
        .bind(crate::database::repositories::meeting::MEETING_STATUS_PROCESSING)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(&run_id)
        .bind(now)
        .bind(meeting_id)
        .execute(&mut *transaction)
        .await?;
        if claim.rows_affected() != 1 {
            transaction.rollback().await?;
            return Err(sqlx::Error::Protocol(
                "Meeting is still being recorded or does not exist".to_string(),
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, start_time, result, error)
            VALUES (?, 'PENDING', ?, ?, ?, NULL, NULL)
            ON CONFLICT(meeting_id) DO UPDATE SET
                status = 'PENDING', updated_at = excluded.updated_at,
                start_time = excluded.start_time,
                result_backup = summary_processes.result,
                result_backup_timestamp = excluded.updated_at,
                error = CASE
                    WHEN summary_processes.error LIKE 'summary_export_pending:%' THEN
                        CASE
                            WHEN instr(summary_processes.error, char(10) || 'summary_generation_error:') > 0
                            THEN substr(summary_processes.error, 1, instr(summary_processes.error, char(10) || 'summary_generation_error:') - 1)
                            ELSE summary_processes.error
                        END
                    ELSE NULL
                END
            "#,
        )
        .bind(meeting_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        log_info!(
            "Backed up existing summary and claimed run {} for meeting_id: {}",
            run_id,
            meeting_id
        );
        Ok((run_id, previous_owner))
    }

    pub async fn get_summary_data_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<SummaryProcess>, sqlx::Error> {
        sqlx::query_as::<_, SummaryProcess>("SELECT * FROM summary_processes WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create_or_reset_process(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<String, sqlx::Error> {
        Self::start_process(pool, meeting_id)
            .await
            .map(|(run_id, _)| run_id)
    }

    pub async fn update_process_completed(
        pool: &SqlitePool,
        meeting_id: &str,
        result: Value, // Keep this as Value to handle both old and new formats if needed
        chunk_count: i64,
        processing_time: f64,
    ) -> Result<(), sqlx::Error> {
        let lifecycle_run_id = current_summary_run_id(pool, meeting_id).await?;
        Self::update_process_completed_for_run(
            pool,
            meeting_id,
            &lifecycle_run_id,
            result,
            chunk_count,
            processing_time,
        )
        .await
    }

    pub async fn update_process_completed_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        result: Value,
        chunk_count: i64,
        processing_time: f64,
    ) -> Result<(), sqlx::Error> {
        Self::accept_process_completed_for_run(
            pool,
            meeting_id,
            lifecycle_run_id,
            result,
            SummaryProcessingMetrics {
                chunk_count,
                processing_time,
            },
            None,
            false,
        )
        .await
    }

    pub async fn accept_process_completed_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        result: Value,
        metrics: SummaryProcessingMetrics,
        suggested_title: Option<&str>,
        export_pending: bool,
    ) -> Result<(), sqlx::Error> {
        let SummaryProcessingMetrics {
            chunk_count,
            processing_time,
        } = metrics;
        let now = Utc::now();
        let result_str = serde_json::to_string(&result)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize result: {}", e)))?;
        let pending_error = export_pending.then(|| {
            format!(
                "{}summary.md publication pending",
                SUMMARY_EXPORT_PENDING_PREFIX
            )
        });
        let mut transaction = pool.begin().await?;

        let update = sqlx::query(
            r#"
            UPDATE summary_processes
            SET status = 'completed', result = ?, updated_at = ?, end_time = ?, chunk_count = ?, processing_time = ?, error = ?,
                result_backup = CASE WHEN ? THEN result_backup ELSE NULL END,
                result_backup_timestamp = CASE WHEN ? THEN result_backup_timestamp ELSE NULL END
            WHERE meeting_id = ?
              AND EXISTS (
                  SELECT 1 FROM meetings m
                  WHERE m.id = summary_processes.meeting_id
                    AND m.lifecycle_operation = ?
                    AND m.lifecycle_run_id = ?
                    AND m.status = 'processing'
              )
            "#
        )
        .bind(result_str)
        .bind(now)
        .bind(now)
        .bind(chunk_count)
        .bind(processing_time)
        .bind(pending_error)
        .bind(export_pending)
        .bind(export_pending)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(&mut *transaction)
        .await?;
        if update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }

        if let Some(title) = suggested_title {
            sqlx::query(
                "UPDATE meetings SET title = ?, title_source = 'suggested', updated_at = ?
                 WHERE id = ? AND lifecycle_operation = ? AND lifecycle_run_id = ? AND title_source = 'suggested'",
            )
            .bind(title)
            .bind(now)
            .bind(meeting_id)
            .bind(LIFECYCLE_OPERATION_SUMMARY)
            .bind(lifecycle_run_id)
            .execute(&mut *transaction)
            .await?;
        }

        let lifecycle_update = sqlx::query(
            "UPDATE meetings SET status = ?, lifecycle_error = NULL, updated_at = ?
             WHERE id = ? AND lifecycle_operation = ? AND lifecycle_run_id = ? AND status = 'processing'",
        )
        .bind(MEETING_STATUS_READY)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(&mut *transaction)
        .await?;
        if lifecycle_update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Err(sqlx::Error::RowNotFound);
        }
        SearchRepository::rebuild_in_transaction(&mut transaction).await?;
        transaction.commit().await?;
        log_info!("Summary accepted for meeting_id: {}", meeting_id);
        Ok(())
    }

    pub async fn is_run_processing(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let active = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM meetings WHERE id = ? AND lifecycle_operation = ? AND lifecycle_run_id = ? AND status = 'processing'",
        )
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .fetch_optional(pool)
        .await?;
        Ok(active.is_some())
    }

    pub async fn mark_export_pending_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        reason: &str,
    ) -> Result<bool, sqlx::Error> {
        let error = format!("{}{}", SUMMARY_EXPORT_PENDING_PREFIX, reason);
        let result = sqlx::query(
            "UPDATE summary_processes SET error = ?, updated_at = ?
             WHERE meeting_id = ? AND status = 'completed'
               AND EXISTS (SELECT 1 FROM meetings m WHERE m.id = summary_processes.meeting_id AND m.lifecycle_operation = ? AND m.lifecycle_run_id = ?)",
        )
        .bind(error)
        .bind(Utc::now())
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn clear_export_pending_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE summary_processes
             SET error = CASE
                    WHEN instr(error, char(10) || 'summary_generation_error:') > 0
                    THEN substr(error, instr(error, char(10) || 'summary_generation_error:') + 1)
                    ELSE NULL
                 END,
                 result_backup = NULL, result_backup_timestamp = NULL, updated_at = ?
             WHERE meeting_id = ? AND status IN ('completed', 'failed', 'cancelled') AND error LIKE 'summary_export_pending:%'
               AND EXISTS (SELECT 1 FROM meetings m WHERE m.id = summary_processes.meeting_id AND m.lifecycle_operation = ? AND m.lifecycle_run_id = ?)",
        )
        .bind(Utc::now())
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Clear an export marker after startup recovery has removed the interrupted
    /// lifecycle owner. Keep the generation error that followed the marker.
    pub async fn clear_terminal_export_pending_without_owner(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE summary_processes
             SET error = CASE
                    WHEN instr(error, char(10) || 'summary_generation_error:') > 0
                    THEN substr(error, instr(error, char(10) || 'summary_generation_error:') + 1)
                    ELSE NULL
                 END,
                 result_backup = NULL,
                 result_backup_timestamp = NULL,
                 updated_at = ?
             WHERE meeting_id = ?
               AND status IN ('completed', 'failed', 'cancelled')
               AND error LIKE 'summary_export_pending:%'
               AND NOT EXISTS (
                   SELECT 1 FROM meetings m
                   WHERE m.id = summary_processes.meeting_id
                     AND m.status = 'processing'
                     AND m.lifecycle_operation = 'summary'
               )",
        )
        .bind(Utc::now())
        .bind(meeting_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn pending_export_meeting_ids(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT meeting_id FROM summary_processes
             WHERE status IN ('completed', 'failed', 'cancelled') AND error LIKE 'summary_export_pending:%'
             ORDER BY updated_at",
        )
        .fetch_all(pool)
        .await
    }

    /// Cancel whichever summary attempt is current and terminalize both rows in
    /// one transaction. The returned id is the only token the caller may cancel.
    pub async fn cancel_current_process(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let mut transaction = pool.begin().await?;
        let run_id = sqlx::query_scalar::<_, String>(
            "SELECT lifecycle_run_id FROM meetings WHERE id = ? AND lifecycle_operation = ? AND status = 'processing'",
        )
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(run_id) = run_id else {
            transaction.rollback().await?;
            return Ok(None);
        };
        let now = Utc::now();
        let updated = sqlx::query(
            "UPDATE summary_processes SET status = 'cancelled', updated_at = ?, end_time = ?,
                error = CASE WHEN error LIKE 'summary_export_pending:%'
                    THEN error || char(10) || 'summary_generation_error:Generation was cancelled by user'
                    ELSE 'Generation was cancelled by user' END,
                result = COALESCE(result_backup, result), result_backup = NULL, result_backup_timestamp = NULL
             WHERE meeting_id = ? AND status = 'PENDING'
               AND EXISTS (
                   SELECT 1 FROM meetings m
                   WHERE m.id = summary_processes.meeting_id
                     AND m.lifecycle_operation = ?
                     AND m.lifecycle_run_id = ?
                     AND m.status = 'processing'
               )",
        )
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(&run_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        let lifecycle = sqlx::query(
            "UPDATE meetings SET status = ?, lifecycle_error = 'Generation was cancelled by user', updated_at = ?
             WHERE id = ? AND lifecycle_operation = ? AND lifecycle_run_id = ? AND status = 'processing'",
        )
        .bind(MEETING_STATUS_ERROR)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(&run_id)
        .execute(&mut *transaction)
        .await?;
        if lifecycle.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        transaction.commit().await?;
        Ok(Some(run_id))
    }

    pub async fn update_process_failed(
        pool: &SqlitePool,
        meeting_id: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let lifecycle_run_id = current_summary_run_id(pool, meeting_id).await?;
        Self::update_process_failed_for_run(pool, meeting_id, &lifecycle_run_id, error).await
    }

    pub async fn update_process_failed_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        let mut transaction = pool.begin().await?;
        // Restore only while this attempt still owns the in-progress row.
        let summary_update = sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'failed',
                error = CASE WHEN error LIKE 'summary_export_pending:%'
                    THEN error || char(10) || 'summary_generation_error:' || ?
                    ELSE ? END,
                updated_at = ?,
                end_time = ?,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE meeting_id = ?
              AND EXISTS (
                  SELECT 1 FROM meetings m
                  WHERE m.id = summary_processes.meeting_id
                    AND m.lifecycle_operation = ?
                    AND m.lifecycle_run_id = ?
                    AND m.status = 'processing'
              )
            "#,
        )
        .bind(error)
        .bind(error)
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(&mut *transaction)
        .await?;
        if summary_update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(());
        }
        let lifecycle_update = sqlx::query(
            "UPDATE meetings SET status = ?, lifecycle_error = ?, updated_at = ?
             WHERE id = ? AND lifecycle_operation = ? AND lifecycle_run_id = ? AND status = 'processing'",
        )
        .bind(MEETING_STATUS_ERROR)
        .bind(error)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(&mut *transaction)
        .await?;
        if lifecycle_update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(());
        }
        SearchRepository::rebuild_in_transaction(&mut transaction).await?;
        transaction.commit().await?;
        log_info!(
            "Summary generation failed and backup restored for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }

    pub async fn update_process_cancelled(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(), sqlx::Error> {
        let lifecycle_run_id = current_summary_run_id(pool, meeting_id).await?;
        Self::update_process_cancelled_for_run(pool, meeting_id, &lifecycle_run_id).await
    }

    pub async fn update_process_cancelled_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        let mut transaction = pool.begin().await?;
        // Restore only while this attempt still owns the in-progress row.
        let summary_update = sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'cancelled',
                updated_at = ?,
                end_time = ?,
                error = CASE WHEN error LIKE 'summary_export_pending:%'
                    THEN error || char(10) || 'summary_generation_error:Generation was cancelled by user'
                    ELSE 'Generation was cancelled by user' END,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE meeting_id = ?
              AND EXISTS (
                  SELECT 1 FROM meetings m
                  WHERE m.id = summary_processes.meeting_id
                    AND m.lifecycle_operation = ?
                    AND m.lifecycle_run_id = ?
                    AND m.status = 'processing'
              )
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(&mut *transaction)
        .await?;
        if summary_update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(());
        }
        let lifecycle_update = sqlx::query(
            "UPDATE meetings SET status = ?, lifecycle_error = 'Generation was cancelled by user', updated_at = ?
             WHERE id = ? AND lifecycle_operation = ? AND lifecycle_run_id = ? AND status = 'processing'",
        )
        .bind(MEETING_STATUS_ERROR)
        .bind(now)
        .bind(meeting_id)
        .bind(LIFECYCLE_OPERATION_SUMMARY)
        .bind(lifecycle_run_id)
        .execute(&mut *transaction)
        .await?;
        if lifecycle_update.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(());
        }
        SearchRepository::rebuild_in_transaction(&mut transaction).await?;
        transaction.commit().await?;
        log_info!(
            "Marked summary process as cancelled and restored backup for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }
}

async fn current_summary_run_id(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<String, sqlx::Error> {
    MeetingsRepository::get_lifecycle_run_id(pool, meeting_id, LIFECYCLE_OPERATION_SUMMARY)
        .await?
        .ok_or_else(|| {
            sqlx::Error::Protocol("Summary lifecycle owner is no longer active".to_string())
        })
}
