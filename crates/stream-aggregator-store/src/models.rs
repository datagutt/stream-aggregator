//! Diesel ORM models for database tables

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json;
use std::collections::HashMap;

use stream_aggregator_core::models::{
    DiscoveryFilters, DiscoveryRule, StreamId, StreamInfo, StreamerSource, TrackedStreamer,
};

use crate::schema::{discovery_rules, streams, tracked_streamers};

// ===== Stream Models =====

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = streams)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StreamRow {
    pub id: String,
    pub platform: String,
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_live: bool,
    pub title: Option<String>,
    pub viewer_count: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub category: Option<String>,
    pub tags: String,
    pub language: Option<String>,
    pub started_at: Option<String>,
    pub last_fetched_at: String,
    pub last_live_at: Option<String>,
    pub metadata: String,
}

#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = streams)]
pub struct NewStream<'a> {
    pub id: &'a str,
    pub platform: &'a str,
    pub user_id: &'a str,
    pub display_name: &'a str,
    pub avatar_url: Option<&'a str>,
    pub is_live: bool,
    pub title: Option<&'a str>,
    pub viewer_count: Option<i32>,
    pub thumbnail_url: Option<&'a str>,
    pub category: Option<&'a str>,
    pub tags: String,
    pub language: Option<&'a str>,
    pub started_at: Option<String>,
    pub last_fetched_at: String,
    pub last_live_at: Option<String>,
    pub metadata: String,
}

impl StreamRow {
    /// Convert database row to StreamInfo
    pub fn to_stream_info(&self) -> Result<StreamInfo, serde_json::Error> {
        let tags: Vec<String> = serde_json::from_str(&self.tags).unwrap_or_default();
        let metadata: HashMap<String, serde_json::Value> =
            serde_json::from_str(&self.metadata).unwrap_or_default();

        let started_at = self
            .started_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let last_fetched_at = DateTime::parse_from_rfc3339(&self.last_fetched_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let last_live_at = self
            .last_live_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(StreamInfo {
            id: StreamId(self.id.clone()),
            platform: self.platform.clone(),
            user_id: self.user_id.clone(),
            display_name: self.display_name.clone(),
            avatar_url: self.avatar_url.clone(),
            is_live: self.is_live,
            title: self.title.clone(),
            viewer_count: self.viewer_count.map(|v| v as u64),
            thumbnail_url: self.thumbnail_url.clone(),
            category: self.category.clone(),
            tags,
            language: self.language.clone(),
            started_at,
            last_fetched_at,
            last_live_at,
            metadata,
        })
    }
}

impl<'a> NewStream<'a> {
    /// Create a NewStream from StreamInfo
    pub fn from_stream_info(stream: &'a StreamInfo) -> Result<Self, serde_json::Error> {
        let tags = serde_json::to_string(&stream.tags)?;
        let metadata = serde_json::to_string(&stream.metadata)?;
        let started_at = stream.started_at.map(|dt| dt.to_rfc3339());
        let last_fetched_at = stream.last_fetched_at.to_rfc3339();
        let last_live_at = stream.last_live_at.map(|dt| dt.to_rfc3339());

        Ok(Self {
            id: &stream.id.0,
            platform: &stream.platform,
            user_id: &stream.user_id,
            display_name: &stream.display_name,
            avatar_url: stream.avatar_url.as_deref(),
            is_live: stream.is_live,
            title: stream.title.as_deref(),
            viewer_count: stream.viewer_count.map(|v| v as i32),
            thumbnail_url: stream.thumbnail_url.as_deref(),
            category: stream.category.as_deref(),
            tags,
            language: stream.language.as_deref(),
            started_at,
            last_fetched_at,
            last_live_at,
            metadata,
        })
    }
}

// ===== Tracked Streamer Models =====

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = tracked_streamers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TrackedStreamerRow {
    pub platform: String,
    pub user_id: String,
    pub custom_name: Option<String>,
    pub group_name: Option<String>,
    pub priority: Option<i32>,
    pub labels: String,
    pub source: String,
    pub discovery_rule_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = tracked_streamers)]
pub struct NewTrackedStreamer<'a> {
    pub platform: &'a str,
    pub user_id: &'a str,
    pub custom_name: Option<&'a str>,
    pub group_name: Option<&'a str>,
    pub priority: Option<i32>,
    pub labels: String,
    pub source: &'a str,
    pub discovery_rule_id: Option<&'a str>,
    pub created_at: String,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = tracked_streamers)]
pub struct UpdateTrackedStreamer<'a> {
    pub custom_name: Option<&'a str>,
    pub group_name: Option<&'a str>,
    pub priority: Option<i32>,
    pub labels: String,
    pub source: &'a str,
    pub discovery_rule_id: Option<&'a str>,
}

impl TrackedStreamerRow {
    /// Convert database row to TrackedStreamer
    pub fn to_tracked_streamer(&self) -> Result<TrackedStreamer, serde_json::Error> {
        let labels: HashMap<String, String> =
            serde_json::from_str(&self.labels).unwrap_or_default();

        let source = match self.source.as_str() {
            "manual" => StreamerSource::Manual,
            "discovery" => StreamerSource::Discovery,
            _ => StreamerSource::Manual,
        };

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(TrackedStreamer {
            platform: self.platform.clone(),
            user_id: self.user_id.clone(),
            custom_name: self.custom_name.clone(),
            group: self.group_name.clone(),
            priority: self.priority,
            labels,
            source,
            discovery_rule_id: self.discovery_rule_id.clone(),
            created_at,
        })
    }
}

impl<'a> NewTrackedStreamer<'a> {
    /// Create a NewTrackedStreamer from TrackedStreamer
    pub fn from_tracked_streamer(streamer: &'a TrackedStreamer) -> Result<Self, serde_json::Error> {
        let labels = serde_json::to_string(&streamer.labels)?;
        let source = match streamer.source {
            StreamerSource::Manual => "manual",
            StreamerSource::Discovery => "discovery",
        };
        let created_at = streamer.created_at.to_rfc3339();

        Ok(Self {
            platform: &streamer.platform,
            user_id: &streamer.user_id,
            custom_name: streamer.custom_name.as_deref(),
            group_name: streamer.group.as_deref(),
            priority: streamer.priority,
            labels,
            source,
            discovery_rule_id: streamer.discovery_rule_id.as_deref(),
            created_at,
        })
    }
}

impl<'a> UpdateTrackedStreamer<'a> {
    /// Create an UpdateTrackedStreamer from TrackedStreamer
    pub fn from_tracked_streamer(streamer: &'a TrackedStreamer) -> Result<Self, serde_json::Error> {
        let labels = serde_json::to_string(&streamer.labels)?;
        let source = match streamer.source {
            StreamerSource::Manual => "manual",
            StreamerSource::Discovery => "discovery",
        };

        Ok(Self {
            custom_name: streamer.custom_name.as_deref(),
            group_name: streamer.group.as_deref(),
            priority: streamer.priority,
            labels,
            source,
            discovery_rule_id: streamer.discovery_rule_id.as_deref(),
        })
    }
}

// ===== Discovery Rule Models =====

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = discovery_rules)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DiscoveryRuleRow {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub enabled: bool,
    pub filters: String,
    pub interval_secs: i32,
    pub apply_labels: String,
    pub apply_group: Option<String>,
    pub created_at: String,
    pub last_run_at: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = discovery_rules)]
pub struct NewDiscoveryRule<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub platform: &'a str,
    pub enabled: bool,
    pub filters: String,
    pub interval_secs: i32,
    pub apply_labels: String,
    pub apply_group: Option<&'a str>,
    pub created_at: String,
    pub last_run_at: Option<String>,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = discovery_rules)]
pub struct UpdateDiscoveryRule<'a> {
    pub name: &'a str,
    pub platform: &'a str,
    pub enabled: bool,
    pub filters: String,
    pub interval_secs: i32,
    pub apply_labels: String,
    pub apply_group: Option<&'a str>,
    pub last_run_at: Option<String>,
}

impl DiscoveryRuleRow {
    /// Convert database row to DiscoveryRule
    pub fn to_discovery_rule(&self) -> Result<DiscoveryRule, serde_json::Error> {
        let filters: DiscoveryFilters = serde_json::from_str(&self.filters).unwrap_or_default();
        let apply_labels: HashMap<String, String> =
            serde_json::from_str(&self.apply_labels).unwrap_or_default();

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let last_run_at = self
            .last_run_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(DiscoveryRule {
            id: self.id.clone(),
            name: self.name.clone(),
            platform: self.platform.clone(),
            enabled: self.enabled,
            filters,
            interval_secs: self.interval_secs as u64,
            apply_labels,
            apply_group: self.apply_group.clone(),
            created_at,
            last_run_at,
        })
    }
}

impl<'a> NewDiscoveryRule<'a> {
    /// Create a NewDiscoveryRule from DiscoveryRule
    pub fn from_discovery_rule(rule: &'a DiscoveryRule) -> Result<Self, serde_json::Error> {
        let filters = serde_json::to_string(&rule.filters)?;
        let apply_labels = serde_json::to_string(&rule.apply_labels)?;
        let created_at = rule.created_at.to_rfc3339();
        let last_run_at = rule.last_run_at.map(|dt| dt.to_rfc3339());

        Ok(Self {
            id: &rule.id,
            name: &rule.name,
            platform: &rule.platform,
            enabled: rule.enabled,
            filters,
            interval_secs: rule.interval_secs as i32,
            apply_labels,
            apply_group: rule.apply_group.as_deref(),
            created_at,
            last_run_at,
        })
    }
}

impl<'a> UpdateDiscoveryRule<'a> {
    /// Create an UpdateDiscoveryRule from DiscoveryRule
    pub fn from_discovery_rule(rule: &'a DiscoveryRule) -> Result<Self, serde_json::Error> {
        let filters = serde_json::to_string(&rule.filters)?;
        let apply_labels = serde_json::to_string(&rule.apply_labels)?;
        let last_run_at = rule.last_run_at.map(|dt| dt.to_rfc3339());

        Ok(Self {
            name: &rule.name,
            platform: &rule.platform,
            enabled: rule.enabled,
            filters,
            interval_secs: rule.interval_secs as i32,
            apply_labels,
            apply_group: rule.apply_group.as_deref(),
            last_run_at,
        })
    }
}
