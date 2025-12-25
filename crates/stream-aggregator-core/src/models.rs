//! Core data models for StreamAggregator

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a stream (hash of platform + user_id)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub String);

impl StreamId {
    /// Create a new StreamId from platform and user_id
    pub fn new(platform: &str, user_id: &str) -> Self {
        StreamId(crate::id::generate_stream_id(platform, user_id))
    }
}

impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a streamer (platform + user_id)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamerId {
    pub platform: String,
    pub user_id: String,
}

/// Information about a live stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Unique identifier (hash of platform + user_id)
    pub id: StreamId,

    /// Platform identifier (e.g., "twitch", "youtube")
    pub platform: String,

    /// Platform-specific user ID
    pub user_id: String,

    /// Display name
    pub display_name: String,

    /// Avatar URL
    pub avatar_url: Option<String>,

    /// Whether currently live
    pub is_live: bool,

    /// Stream title (if live)
    pub title: Option<String>,

    /// Current viewer count (if live)
    pub viewer_count: Option<u64>,

    /// Stream thumbnail URL (if live)
    pub thumbnail_url: Option<String>,

    /// Game/category being streamed
    pub category: Option<String>,

    /// Tags associated with stream
    pub tags: Vec<String>,

    /// Stream language
    pub language: Option<String>,

    /// When the stream started (if live)
    pub started_at: Option<DateTime<Utc>>,

    /// Last time this data was fetched
    pub last_updated: DateTime<Utc>,

    /// Custom metadata (platform-specific data, user-defined labels, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StreamInfo {
    /// Create a new StreamInfo with required fields
    pub fn new(
        platform: impl Into<String>,
        user_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let platform = platform.into();
        let user_id = user_id.into();
        let id = StreamId::new(&platform, &user_id);

        Self {
            id,
            platform,
            user_id,
            display_name: display_name.into(),
            avatar_url: None,
            is_live: false,
            title: None,
            viewer_count: None,
            thumbnail_url: None,
            category: None,
            tags: Vec::new(),
            language: None,
            started_at: None,
            last_updated: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// Source of a tracked streamer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamerSource {
    /// Manually added by user
    Manual,
    /// Discovered by a discovery rule
    Discovery,
}

/// A streamer being tracked by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedStreamer {
    /// Platform identifier
    pub platform: String,

    /// Platform-specific user ID
    pub user_id: String,

    /// Optional custom display name override
    pub custom_name: Option<String>,

    /// Optional grouping/team
    pub group: Option<String>,

    /// Priority for display ordering (higher = more important)
    pub priority: Option<i32>,

    /// Custom labels for filtering
    pub labels: HashMap<String, String>,

    /// Source: manual or discovery
    pub source: StreamerSource,

    /// If discovered, which rule discovered it
    pub discovery_rule_id: Option<String>,

    /// When this streamer was added
    pub created_at: DateTime<Utc>,
}

impl TrackedStreamer {
    /// Create a new manually tracked streamer
    pub fn new_manual(platform: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            platform: platform.into(),
            user_id: user_id.into(),
            custom_name: None,
            group: None,
            priority: None,
            labels: HashMap::new(),
            source: StreamerSource::Manual,
            discovery_rule_id: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new discovered streamer
    pub fn new_discovered(
        platform: impl Into<String>,
        user_id: impl Into<String>,
        rule_id: impl Into<String>,
    ) -> Self {
        Self {
            platform: platform.into(),
            user_id: user_id.into(),
            custom_name: None,
            group: None,
            priority: None,
            labels: HashMap::new(),
            source: StreamerSource::Discovery,
            discovery_rule_id: Some(rule_id.into()),
            created_at: Utc::now(),
        }
    }
}

/// Filters for discovering streamers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryFilters {
    /// Filter by tags (Twitch tags, etc.)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Filter by game/category ID or name
    #[serde(default)]
    pub categories: Vec<String>,

    /// Filter by language
    #[serde(default)]
    pub languages: Vec<String>,

    /// Minimum viewer count
    pub min_viewers: Option<u64>,

    /// Maximum viewer count
    pub max_viewers: Option<u64>,

    /// Maximum number of streamers to discover
    pub limit: Option<usize>,
}

/// Rule for automatically discovering streamers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRule {
    /// Unique rule ID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Target platform
    pub platform: String,

    /// Whether this rule is active
    pub enabled: bool,

    /// Filters for discovery
    pub filters: DiscoveryFilters,

    /// How often to run discovery (seconds)
    pub interval_secs: u64,

    /// Labels to apply to discovered streamers
    #[serde(default)]
    pub apply_labels: HashMap<String, String>,

    /// Group to assign to discovered streamers
    pub apply_group: Option<String>,

    /// When this rule was created
    pub created_at: DateTime<Utc>,

    /// Last time this rule was executed
    pub last_run_at: Option<DateTime<Utc>>,
}

impl DiscoveryRule {
    /// Create a new discovery rule
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        platform: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            platform: platform.into(),
            enabled: true,
            filters: DiscoveryFilters::default(),
            interval_secs: 3600, // Default: 1 hour
            apply_labels: HashMap::new(),
            apply_group: None,
            created_at: Utc::now(),
            last_run_at: None,
        }
    }
}

/// Information about a discovered streamer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredStreamer {
    /// Platform identifier
    pub platform: String,

    /// Platform-specific user ID
    pub user_id: String,

    /// Display name
    pub display_name: String,

    /// Whether currently live
    pub is_live: bool,

    /// Viewer count (if live)
    pub viewer_count: Option<u64>,

    /// Category/game
    pub category: Option<String>,

    /// Tags
    pub tags: Vec<String>,

    /// Language
    pub language: Option<String>,
}

/// Rate limiting configuration for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,

    /// Burst capacity
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
        }
    }
}

/// Health status for a platform or service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is healthy and operational
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is down or unreachable
    Unhealthy,
}

/// Query parameters for listing streams
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamQuery {
    /// Filter by platform
    pub platform: Option<String>,

    /// Filter by live status
    pub is_live: Option<bool>,

    /// Filter by group/team name
    pub group: Option<String>,

    /// Filter by labels (key-value pairs)
    #[serde(default)]
    pub labels: HashMap<String, String>,

    /// Search in display name and title
    pub search: Option<String>,

    /// Filter by language
    pub language: Option<String>,

    /// Filter by category
    pub category: Option<String>,

    /// Filter by tag
    pub tag: Option<String>,

    /// Minimum viewer count
    pub min_viewers: Option<u64>,

    /// Maximum viewer count
    pub max_viewers: Option<u64>,

    /// Sort field: viewers, name, platform, updated
    pub sort: Option<String>,

    /// Sort order: asc or desc
    pub order: Option<String>,

    /// Pagination: page number (0-indexed)
    pub page: Option<usize>,

    /// Pagination: items per page
    pub page_size: Option<usize>,
}

/// Paginated result for stream queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPage {
    /// Streams on this page
    pub items: Vec<StreamInfo>,

    /// Total number of items matching the query
    pub total: usize,

    /// Current page (0-indexed)
    pub page: usize,

    /// Number of items per page
    pub page_size: usize,

    /// Total number of pages
    pub total_pages: usize,
}

/// Query parameters for listing tracked streamers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackedStreamerQuery {
    /// Filter by platform
    pub platform: Option<String>,

    /// Filter by group
    pub group: Option<String>,

    /// Filter by source
    pub source: Option<StreamerSource>,

    /// Filter by label key-value pair
    pub labels: HashMap<String, String>,
}
