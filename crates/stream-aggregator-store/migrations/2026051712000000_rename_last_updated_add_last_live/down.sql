DROP INDEX IF EXISTS idx_streams_last_live_at;
DROP INDEX IF EXISTS idx_streams_last_fetched_at;

ALTER TABLE streams DROP COLUMN last_live_at;
ALTER TABLE streams RENAME COLUMN last_fetched_at TO last_updated;

CREATE INDEX idx_streams_last_updated ON streams(last_updated);
