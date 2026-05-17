-- Rename streams.last_updated to last_fetched_at and add nullable last_live_at.
-- last_updated was ambiguous: API consumers could read it as "last time the
-- stream was live", but it actually meant "last time we polled the platform".
-- Split into two explicit columns.

DROP INDEX IF EXISTS idx_streams_last_updated;

ALTER TABLE streams RENAME COLUMN last_updated TO last_fetched_at;
ALTER TABLE streams ADD COLUMN last_live_at TEXT;

CREATE INDEX idx_streams_last_fetched_at ON streams(last_fetched_at);
CREATE INDEX idx_streams_last_live_at ON streams(last_live_at);
