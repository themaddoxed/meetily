ALTER TABLE transcript_settings ADD COLUMN whisperVocabulary TEXT;

CREATE TABLE meeting_transcript_vocabulary (
    meeting_id TEXT PRIMARY KEY NOT NULL,
    vocabulary TEXT,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
