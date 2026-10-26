-- Preserve the immutable 20260626000100 migration checksum and harden its schema here.
-- User transcript content and legacy transcripts.speaker are copied without modification.

DROP INDEX IF EXISTS idx_transcripts_meeting_diarization;

CREATE TABLE transcripts_diarization_hardened (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    transcript TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    summary TEXT,
    action_items TEXT,
    key_points TEXT,
    audio_start_time REAL,
    audio_end_time REAL,
    duration REAL,
    speaker TEXT,
    speaker_id TEXT,
    speaker_label TEXT,
    speaker_color TEXT,
    is_overlap INTEGER NOT NULL DEFAULT 0 CHECK (is_overlap IN (0, 1)),
    diarization_status TEXT NOT NULL DEFAULT 'none'
        CHECK (diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        )),
    diarization_method TEXT,
    diarization_confidence REAL
        CHECK (diarization_confidence IS NULL OR diarization_confidence BETWEEN 0.0 AND 1.0),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

INSERT INTO transcripts_diarization_hardened (
    id, meeting_id, transcript, timestamp, summary, action_items, key_points,
    audio_start_time, audio_end_time, duration, speaker,
    speaker_id, speaker_label, speaker_color, is_overlap,
    diarization_status, diarization_method, diarization_confidence
)
SELECT
    id, meeting_id, transcript, timestamp, summary, action_items, key_points,
    audio_start_time, audio_end_time, duration, speaker,
    CASE
        WHEN is_overlap = 0 AND diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN speaker_id
        ELSE NULL
    END,
    CASE
        WHEN is_overlap = 0 AND diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN speaker_label
        WHEN is_overlap = 1 AND diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN 'Multiple speakers'
        ELSE NULL
    END,
    CASE
        WHEN is_overlap = 0 AND diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN speaker_color
        WHEN is_overlap = 1 AND diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN '#f97316'
        ELSE NULL
    END,
    CASE
        WHEN is_overlap IN (0, 1) AND diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN is_overlap
        ELSE 0
    END,
    CASE
        WHEN diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN diarization_status
        ELSE 'none'
    END,
    diarization_method,
    CASE
        WHEN diarization_confidence BETWEEN 0.0 AND 1.0 THEN diarization_confidence
        ELSE NULL
    END
FROM transcripts;

DROP TABLE transcripts;
ALTER TABLE transcripts_diarization_hardened RENAME TO transcripts;

CREATE INDEX idx_transcripts_meeting_diarization
ON transcripts(meeting_id, diarization_status);

CREATE TABLE diarization_settings_hardened (
    id TEXT PRIMARY KEY NOT NULL DEFAULT '1',
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    mode TEXT NOT NULL DEFAULT 'live_plus_post_call'
        CHECK (mode IN ('live_plus_post_call', 'post_call_only', 'off')),
    show_provisional_labels INTEGER NOT NULL DEFAULT 1
        CHECK (show_provisional_labels IN (0, 1)),
    post_call_refinement_enabled INTEGER NOT NULL DEFAULT 1
        CHECK (post_call_refinement_enabled IN (0, 1)),
    overlap_handling TEXT NOT NULL DEFAULT 'multiple_speakers'
        CHECK (overlap_handling = 'multiple_speakers'),
    speaker_review_enabled INTEGER NOT NULL DEFAULT 1
        CHECK (speaker_review_enabled IN (0, 1)),
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO diarization_settings_hardened (
    id, enabled, mode, show_provisional_labels, post_call_refinement_enabled,
    overlap_handling, speaker_review_enabled, updated_at
)
SELECT
    id,
    CASE WHEN enabled IN (0, 1) THEN enabled ELSE 0 END,
    CASE
        WHEN mode IN ('live_plus_post_call', 'post_call_only', 'off') THEN mode
        ELSE 'off'
    END,
    CASE WHEN show_provisional_labels IN (0, 1) THEN show_provisional_labels ELSE 1 END,
    CASE
        WHEN post_call_refinement_enabled IN (0, 1) THEN post_call_refinement_enabled
        ELSE 1
    END,
    'multiple_speakers',
    CASE WHEN speaker_review_enabled IN (0, 1) THEN speaker_review_enabled ELSE 1 END,
    COALESCE(updated_at, CURRENT_TIMESTAMP)
FROM diarization_settings;

DROP TABLE diarization_settings;
ALTER TABLE diarization_settings_hardened RENAME TO diarization_settings;

CREATE TABLE meeting_diarization_status_hardened (
    meeting_id TEXT PRIMARY KEY NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    live_enabled INTEGER NOT NULL DEFAULT 0 CHECK (live_enabled IN (0, 1)),
    post_call_enabled INTEGER NOT NULL DEFAULT 0 CHECK (post_call_enabled IN (0, 1)),
    current_status TEXT NOT NULL DEFAULT 'none'
        CHECK (current_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        )),
    quality_flags TEXT,
    processed_at DATETIME,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

INSERT INTO meeting_diarization_status_hardened (
    meeting_id, enabled, live_enabled, post_call_enabled, current_status,
    quality_flags, processed_at, updated_at
)
SELECT
    status.meeting_id,
    CASE WHEN status.enabled IN (0, 1) THEN status.enabled ELSE 0 END,
    CASE WHEN status.live_enabled IN (0, 1) THEN status.live_enabled ELSE 0 END,
    CASE WHEN status.post_call_enabled IN (0, 1) THEN status.post_call_enabled ELSE 0 END,
    CASE
        WHEN status.current_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN status.current_status
        ELSE 'none'
    END,
    status.quality_flags,
    status.processed_at,
    COALESCE(status.updated_at, CURRENT_TIMESTAMP)
FROM meeting_diarization_status AS status
JOIN meetings ON meetings.id = status.meeting_id;

DROP TABLE meeting_diarization_status;
ALTER TABLE meeting_diarization_status_hardened RENAME TO meeting_diarization_status;

DROP INDEX IF EXISTS idx_speaker_segments_meeting_time;

CREATE TABLE speaker_segments_hardened (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL,
    source TEXT NOT NULL,
    start_time REAL NOT NULL,
    end_time REAL NOT NULL CHECK (end_time > start_time),
    speaker_id TEXT,
    speaker_label TEXT,
    confidence REAL CHECK (confidence IS NULL OR confidence BETWEEN 0.0 AND 1.0),
    is_overlap INTEGER NOT NULL DEFAULT 0 CHECK (is_overlap IN (0, 1)),
    diarization_status TEXT NOT NULL
        CHECK (diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        )),
    diarization_method TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (is_overlap = 0 OR (speaker_id IS NULL AND speaker_label IS NULL)),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

INSERT INTO speaker_segments_hardened (
    id, meeting_id, source, start_time, end_time, speaker_id, speaker_label,
    confidence, is_overlap, diarization_status, diarization_method, created_at
)
SELECT
    segment.id,
    segment.meeting_id,
    segment.source,
    segment.start_time,
    segment.end_time,
    CASE
        WHEN segment.is_overlap = 0 AND segment.diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN segment.speaker_id
        ELSE NULL
    END,
    CASE
        WHEN segment.is_overlap = 0 AND segment.diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN segment.speaker_label
        ELSE NULL
    END,
    CASE WHEN segment.confidence BETWEEN 0.0 AND 1.0 THEN segment.confidence ELSE NULL END,
    CASE
        WHEN segment.is_overlap IN (0, 1) AND segment.diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN segment.is_overlap
        ELSE 0
    END,
    CASE
        WHEN segment.diarization_status IN (
            'none', 'provisional', 'final', 'fallback_to_live', 'failed', 'needs_review'
        ) THEN segment.diarization_status
        ELSE 'none'
    END,
    segment.diarization_method,
    COALESCE(segment.created_at, CURRENT_TIMESTAMP)
FROM speaker_segments AS segment
JOIN meetings ON meetings.id = segment.meeting_id
WHERE segment.end_time > segment.start_time;

DROP TABLE speaker_segments;
ALTER TABLE speaker_segments_hardened RENAME TO speaker_segments;

CREATE INDEX idx_speaker_segments_meeting_time
ON speaker_segments(meeting_id, start_time, end_time);
