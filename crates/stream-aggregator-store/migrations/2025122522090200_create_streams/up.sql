-- Streams table: stores current state of all tracked streams
CREATE TABLE streams (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL,
    user_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    is_live BOOLEAN NOT NULL DEFAULT FALSE,
    title TEXT,
    viewer_count INTEGER,
    thumbnail_url TEXT,
    category TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    language TEXT,
    started_at TEXT,
    last_updated TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    UNIQUE(platform, user_id)
);

-- Indexes for query performance
CREATE INDEX idx_streams_platform ON streams(platform);
CREATE INDEX idx_streams_is_live ON streams(is_live);
CREATE INDEX idx_streams_viewer_count ON streams(viewer_count);
CREATE INDEX idx_streams_last_updated ON streams(last_updated);
