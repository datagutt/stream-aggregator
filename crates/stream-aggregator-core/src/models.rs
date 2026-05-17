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

    /// Platform-specific user ID. Stable internal identifier (e.g. Twitch
    /// numeric ID, YouTube channel ID starting with `UC...`). NOT necessarily
    /// safe to use in a platform URL — see `login` for that.
    pub user_id: String,

    /// URL-safe login / handle for the streamer on the platform (Twitch's
    /// lowercase `user_login`, YouTube's channel ID, Kick's slug, TikTok's
    /// unique handle without the `@`, etc). Providers populate this so the
    /// frontend can build platform URLs without guessing.
    #[serde(default)]
    pub login: Option<String>,

    /// Display name (human-readable, may have spaces, mixed case, unicode).
    /// NOT URL-safe.
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

    /// Last time this data was fetched from the upstream platform.
    /// This is when our system polled the platform, not when the stream
    /// went live. See `last_live_at` for the latter.
    pub last_fetched_at: DateTime<Utc>,

    /// Last time the streamer was observed live.
    /// While `is_live` is true, this mirrors `started_at` when the platform
    /// surfaces it. When the streamer goes offline, this sticks at the
    /// previous value (when they were last seen live). `None` if never
    /// observed live.
    pub last_live_at: Option<DateTime<Utc>>,

    /// Custom metadata (platform-specific data, user-defined labels, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StreamInfo {
    /// Compute `last_live_at` for an incoming observation, given the prior
    /// stored value (if any) and whether the streamer was live in that prior
    /// observation.
    ///
    /// Providers build `StreamInfo` fresh per poll with no view of prior state,
    /// so storage layers call this when persisting:
    ///   * Live with `started_at` from the platform: use that timestamp.
    ///   * Live with no `started_at` (most providers): stamp `now` on
    ///     offline→live transitions; otherwise carry the prior value.
    ///   * Offline: keep the prior value (sticky).
    pub fn merge_last_live_at(
        is_live: bool,
        started_at: Option<DateTime<Utc>>,
        prior_last_live_at: Option<DateTime<Utc>>,
        prior_was_live: bool,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        if !is_live {
            return prior_last_live_at;
        }
        if let Some(started) = started_at {
            return Some(started);
        }
        if prior_was_live {
            prior_last_live_at
        } else {
            Some(now)
        }
    }

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
            login: None,
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
            last_fetched_at: Utc::now(),
            last_live_at: None,
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
    /// Filter by platforms. Empty vec means "any platform".
    /// At the HTTP layer this is parsed from a comma-separated list, e.g. `?platform=twitch,youtube`.
    #[serde(default)]
    pub platforms: Vec<String>,

    /// Filter by live status
    pub is_live: Option<bool>,

    /// Filter by group/team name
    pub group: Option<String>,

    /// Filter by labels (key-value pairs)
    #[serde(default)]
    pub labels: HashMap<String, String>,

    /// Search in display name and title
    pub search: Option<String>,

    /// Filter by languages. Empty vec means "any language".
    /// At the HTTP layer this is parsed from a comma-separated list, e.g. `?language=no,sv,da`.
    #[serde(default)]
    pub languages: Vec<String>,

    /// Filter by categories. Empty vec means "any category".
    #[serde(default)]
    pub categories: Vec<String>,

    /// Filter by tags. A stream matches when it contains AT LEAST ONE of these tags.
    /// Empty vec means "any tag".
    #[serde(default)]
    pub tags: Vec<String>,

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

/// Default theme for a community's public surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

impl std::fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeMode::Dark => write!(f, "dark"),
            ThemeMode::Light => write!(f, "light"),
        }
    }
}

impl std::str::FromStr for ThemeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(ThemeMode::Dark),
            "light" => Ok(ThemeMode::Light),
            other => Err(format!("invalid theme: {other}")),
        }
    }
}

/// Filter recipe that defines which slice of the global stream pool a
/// community surfaces. Every field is "any-of"; the empty/missing form means
/// "no constraint." Constraints are combined with AND across fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommunityFilter {
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub min_viewers: Option<u64>,
}

/// A brandable directory tenant. Communities are persisted in the backend and
/// consumed by the Next.js frontend; the host->community map is used by the
/// frontend middleware to render the right brand on the right domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// Stable, URL-safe identifier (e.g. "livestreamnorge").
    pub slug: String,

    /// Display name (e.g. "LiveStreamNorge").
    pub name: String,

    /// Optional short subtitle.
    pub tagline: Option<String>,

    /// OKLCH triplet "L C H" stored as text (e.g. "0.58 0.20 250").
    /// The frontend wraps this in `oklch(...)` when applying to CSS.
    pub accent: String,

    /// Optional OKLCH triplet for foreground contrast against the accent.
    pub accent_contrast: Option<String>,

    /// Optional logo URL.
    pub logo_url: Option<String>,

    /// Default theme for visitors that haven't explicitly toggled.
    pub default_theme: ThemeMode,

    /// Hostnames that map to this community. Populated by the store from the
    /// `community_domains` join table on read.
    #[serde(default)]
    pub domains: Vec<String>,

    /// Filter recipe that selects this community's slice of streams.
    pub filter: CommunityFilter,

    /// Optional markdown body for the community's About page.
    pub about_md: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Community {
    /// Create a new minimal community with sensible defaults.
    pub fn new(slug: impl Into<String>, name: impl Into<String>, accent: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            slug: slug.into(),
            name: name.into(),
            tagline: None,
            accent: accent.into(),
            accent_contrast: None,
            logo_url: None,
            default_theme: ThemeMode::Dark,
            domains: Vec::new(),
            filter: CommunityFilter::default(),
            about_md: None,
            created_at: now,
            updated_at: now,
        }
    }
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
