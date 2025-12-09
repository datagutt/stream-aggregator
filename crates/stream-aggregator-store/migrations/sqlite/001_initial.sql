-- Initial schema for SQLite store
-- migrations/sqlite/001_initial.sql

CREATE TABLE IF NOT EXISTS streams (
    id TEXT PRIMARY KEY,
    platform TEXT NOT NULL,
    user_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    is_live BOOLEAN NOT NULL DEFAULT FALSE,
    title TEXT,
    viewer_count INTEGER,
    thumbnail_url TEXT,
    category TEXT,
    tags TEXT NOT NULL DEFAULT '[]',  -- JSON array
    language TEXT,
    started_at TEXT,  -- ISO 8601
    last_updated TEXT NOT NULL,  -- ISO 8601
    metadata TEXT NOT NULL DEFAULT '{}',  -- JSON object
    UNIQUE(platform, user_id)
);

CREATE INDEX IF NOT EXISTS idx_streams_platform ON streams(platform);
CREATE INDEX IF NOT EXISTS idx_streams_is_live ON streams(is_live);
CREATE INDEX IF NOT EXISTS idx_streams_viewer_count ON streams(viewer_count);
CREATE INDEX IF NOT EXISTS idx_streams_platform_user ON streams(platform, user_id);

CREATE TABLE IF NOT EXISTS tracked_streamers (
    platform TEXT NOT NULL,
    user_id TEXT NOT NULL,
    custom_name TEXT,
    group_name TEXT,
    priority INTEGER,
    labels TEXT NOT NULL DEFAULT '{}',  -- JSON object
    source TEXT NOT NULL DEFAULT 'manual',
    discovery_rule_id TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (platform, user_id)
);

CREATE INDEX IF NOT EXISTS idx_tracked_platform ON tracked_streamers(platform);
CREATE INDEX IF NOT EXISTS idx_tracked_group ON tracked_streamers(group_name);
CREATE INDEX IF NOT EXISTS idx_tracked_source ON tracked_streamers(source);

CREATE TABLE IF NOT EXISTS discovery_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    rule_type TEXT NOT NULL,  -- 'tag', 'category', 'game'
    rule_value TEXT NOT NULL,
    filters TEXT NOT NULL DEFAULT '{}',  -- JSON object
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discovery_platform ON discovery_rules(platform);
CREATE INDEX IF NOT EXISTS idx_discovery_enabled ON discovery_rules(enabled);