-- Search stores only the text that is visible in the meeting UI.
-- The index is rebuilt after migrations and before a query so legacy imports
-- and direct database writes cannot leave stale meeting results behind.
CREATE TABLE IF NOT EXISTS meeting_search_index (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('title', 'transcript', 'summary')),
    source_id TEXT,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    audio_start_time REAL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_meeting_search_index_meeting
    ON meeting_search_index(meeting_id);

CREATE INDEX IF NOT EXISTS idx_meeting_search_index_source
    ON meeting_search_index(source_type, source_id);
