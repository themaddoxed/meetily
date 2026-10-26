PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    folder_path TEXT
);

CREATE TABLE IF NOT EXISTS transcripts (
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
    FOREIGN KEY (meeting_id) REFERENCES meetings(id)
);

CREATE TABLE IF NOT EXISTS summary_processes (
    meeting_id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    error TEXT,
    result TEXT,
    start_time TEXT,
    end_time TEXT,
    chunk_count INTEGER DEFAULT 0,
    processing_time REAL DEFAULT 0.0,
    metadata TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id)
);

CREATE TABLE IF NOT EXISTS transcript_chunks (
    meeting_id TEXT PRIMARY KEY,
    meeting_name TEXT,
    transcript_text TEXT NOT NULL,
    model TEXT NOT NULL,
    model_name TEXT NOT NULL,
    chunk_size INTEGER,
    overlap INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id)
);

CREATE TABLE IF NOT EXISTS settings (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    whisperModel TEXT NOT NULL,
    groqApiKey TEXT,
    openaiApiKey TEXT,
    anthropicApiKey TEXT,
    ollamaApiKey TEXT
);

CREATE TABLE IF NOT EXISTS transcript_settings (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    whisperApiKey TEXT,
    deepgramApiKey TEXT,
    elevenLabsApiKey TEXT,
    groqApiKey TEXT,
    openaiApiKey TEXT
);

INSERT INTO meetings VALUES
    ('meeting-a', 'Planning', '2026-01-01T10:00:00Z', '2026-01-01T10:30:00Z', '/recordings/a'),
    ('meeting-b', 'Обзор продукта', '2026-01-02T10:00:00Z', '2026-01-02T10:45:00Z', NULL),
    ('meeting-c', '', '2026-01-03T10:00:00Z', '2026-01-03T11:00:00Z', '');

INSERT INTO transcripts VALUES
    ('transcript-a', 'meeting-a', 'First transcript', '2026-01-01T10:05:00Z', '{"summary":"ok"}', X'00FF10', '["alpha","beta"]', 0.0, 1.25, 1.25),
    ('transcript-b', 'meeting-b', 'Юникод и emoji: 🎙️', '2026-01-02T10:05:00Z', NULL, '', '{}', 2.5, 5.75, 3.25),
    ('transcript-c', 'meeting-c', '', '2026-01-03T10:05:00Z', '', NULL, NULL, NULL, NULL, NULL);

INSERT INTO summary_processes VALUES
    ('meeting-a', 'COMPLETED', '2026-01-01T10:10:00Z', '2026-01-01T10:20:00Z', NULL, '{"result":[1,2]}', '2026-01-01T10:10:00Z', '2026-01-01T10:20:00Z', 4, 12.5, '{"source":"legacy"}'),
    ('meeting-b', 'FAILED', '2026-01-02T10:10:00Z', '2026-01-02T10:20:00Z', 'ошибка', NULL, '2026-01-02T10:10:00Z', NULL, -2, -0.5, '{}'),
    ('meeting-c', 'PENDING', '2026-01-03T10:10:00Z', '2026-01-03T10:10:00Z', '', '', NULL, NULL, 0, 0.0, NULL);

INSERT INTO transcript_chunks VALUES
    ('meeting-a', 'Planning', 'First partial', 'localWhisper', 'small', 30, 5, '2026-01-01T10:06:00Z'),
    ('meeting-b', 'Обзор продукта', 'Второй фрагмент', 'deepgram', 'nova', 42, 0, '2026-01-02T10:06:00Z'),
    ('meeting-c', NULL, '', 'localWhisper', 'tiny', NULL, NULL, '2026-01-03T10:06:00Z');

INSERT INTO settings VALUES
    ('settings-a', 'ollama', 'llama3', 'small', NULL, NULL, NULL, ''),
    ('settings-b', 'openai', 'gpt-test', 'medium', 'groq-placeholder', 'openai-placeholder', 'anthropic-placeholder', NULL),
    ('settings-c', 'customOpenAI', '', 'tiny', '', '', '', 'ollama-placeholder');

INSERT INTO transcript_settings VALUES
    ('transcript-settings-a', 'localWhisper', 'small', NULL, NULL, NULL, NULL, NULL),
    ('transcript-settings-b', 'deepgram', 'nova', 'whisper-placeholder', 'deepgram-placeholder', 'eleven-placeholder', 'groq-placeholder', 'openai-placeholder'),
    ('transcript-settings-c', 'groq', '', '', '', '', '', '');
