-- Persist the state shown in the meeting list. Existing meetings are complete
-- records, so they start in the ready state and retain their provenance.
ALTER TABLE meetings ADD COLUMN status TEXT NOT NULL DEFAULT 'ready';
ALTER TABLE meetings ADD COLUMN duration_seconds REAL;
ALTER TABLE meetings ADD COLUMN lifecycle_error TEXT;
ALTER TABLE meetings ADD COLUMN lifecycle_operation TEXT;
ALTER TABLE meetings ADD COLUMN lifecycle_run_id TEXT;

CREATE INDEX IF NOT EXISTS idx_meetings_status ON meetings(status);

-- A legacy pending summary has no owner that can survive an application
-- restart. Surface it as an error while preserving the transcript and summary
-- artifacts for retry.
UPDATE meetings
SET status = 'error',
    lifecycle_error = 'This meeting was interrupted while it was being processed',
    lifecycle_operation = NULL
WHERE id IN (
    SELECT meeting_id
    FROM summary_processes
    WHERE status = 'PENDING'
);
