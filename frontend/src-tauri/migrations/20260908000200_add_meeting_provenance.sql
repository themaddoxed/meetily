-- Keep user-entered titles safe from automatic summary suggestions and retain
-- the timestamp captured at the beginning of a recording/import.
ALTER TABLE meetings ADD COLUMN title_source TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE meetings ADD COLUMN started_at_ms INTEGER;
