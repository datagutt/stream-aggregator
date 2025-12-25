-- Tracked streamers table: list of streamers to monitor
CREATE TABLE tracked_streamers (
    platform TEXT NOT NULL,
    user_id TEXT NOT NULL,
    custom_name TEXT,
    group_name TEXT,
    priority INTEGER,
    labels TEXT NOT NULL DEFAULT '{}',
    source TEXT NOT NULL DEFAULT 'manual',
    discovery_rule_id TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (platform, user_id)
);

-- Discovery rules table: automatic streamer discovery configuration
CREATE TABLE discovery_rules (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    filters TEXT NOT NULL DEFAULT '{}',
    interval_secs INTEGER NOT NULL,
    apply_labels TEXT NOT NULL DEFAULT '{}',
    apply_group TEXT,
    created_at TEXT NOT NULL,
    last_run_at TEXT
);

-- Indexes for query performance
CREATE INDEX idx_tracked_streamers_platform ON tracked_streamers(platform);
CREATE INDEX idx_tracked_streamers_group ON tracked_streamers(group_name);
CREATE INDEX idx_discovery_rules_platform ON discovery_rules(platform);
CREATE INDEX idx_discovery_rules_enabled ON discovery_rules(enabled);
