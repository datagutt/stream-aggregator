# StreamAggregator - Architecture Overview

A modern, modular Rust rewrite of the LSND (LiveStream Norway Daemon) project.

## Vision

**StreamAggregator** is a high-performance, extensible service for aggregating live stream information across multiple platforms. Unlike the original LSND (which was tailored for Norwegian streamers), this rewrite is designed to be:

- **Platform-agnostic**: Support any streaming platform through a plugin system
- **Flexible discovery**: Support both manual streamer lists AND automatic discovery (tags, categories, games)
- **Multi-tenant ready**: Run multiple independent configurations simultaneously
- **Horizontally scalable**: Distribute scraping workloads across multiple instances
- **Observable**: Built-in metrics, tracing, and health checks

---

## High-Level Architecture

```
                                    ┌─────────────────────────────────────────┐
                                    │           Configuration Layer           │
                                    │  (TOML/YAML files, env vars, DB, API)   │
                                    └─────────────────────────────────────────┘
                                                        │
                                                        ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                    Core Orchestrator                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐   │
│  │   Scheduler  │  │ Rate Limiter │  │ Health Check │  │   Discovery Coordinator  │   │
│  │  (tokio)     │  │ (governor)   │  │   Manager    │  │  (tag/category filters)  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────────────────┘
                                                        │
                    ┌───────────────────────────────────┼───────────────────────────────────┐
                    │                                   │                                   │
                    ▼                                   ▼                                   ▼
        ┌───────────────────┐               ┌───────────────────┐               ┌───────────────────┐
        │  Platform Provider │               │  Platform Provider │               │  Platform Provider │
        │      (Twitch)      │               │     (YouTube)      │               │      (Kick)        │
        │                    │               │                    │               │                    │
        │ ┌────────────────┐ │               │ ┌────────────────┐ │               │ ┌────────────────┐ │
        │ │ Auth Manager   │ │               │ │ HTML Scraper   │ │               │ │ TLS Spoofer    │ │
        │ │ (OAuth2)       │ │               │ │                │ │               │ │                │ │
        │ └────────────────┘ │               │ └────────────────┘ │               │ └────────────────┘ │
        │ ┌────────────────┐ │               │ ┌────────────────┐ │               │ ┌────────────────┐ │
        │ │ Discovery      │ │               │ │ Discovery      │ │               │ │ Discovery      │ │
        │ │ (tags/games)   │ │               │ │ (n/a)          │ │               │ │ (categories)   │ │
        │ └────────────────┘ │               │ └────────────────┘ │               │ └────────────────┘ │
        │ ┌────────────────┐ │               │ ┌────────────────┐ │               │ ┌────────────────┐ │
        │ │ Stream Fetcher │ │               │ │ Stream Fetcher │ │               │ │ Stream Fetcher │ │
        │ └────────────────┘ │               │ └────────────────┘ │               │ └────────────────┘ │
        └───────────────────┘               └───────────────────┘               └───────────────────┘
                    │                                   │                                   │
                    └───────────────────────────────────┼───────────────────────────────────┘
                                                        │
                                                        ▼
                                    ┌─────────────────────────────────────────┐
                                    │              Data Store                 │
                                    │  (In-memory cache + optional SQLite/    │
                                    │   PostgreSQL for persistence)           │
                                    └─────────────────────────────────────────┘
                                                        │
                    ┌───────────────────────────────────┼───────────────────────────────────┐
                    │                                   │                                   │
                    ▼                                   ▼                                   ▼
        ┌───────────────────┐               ┌───────────────────┐               ┌───────────────────┐
        │    REST API       │               │   WebSocket API   │               │    Webhooks       │
        │   (axum)          │               │  (real-time)      │               │  (notifications)  │
        └───────────────────┘               └───────────────────┘               └───────────────────┘
```

---

## Core Components

### 1. Configuration Layer

The configuration system supports multiple sources with priority-based merging:

1. **Default values** (compiled in)
2. **Configuration files** (TOML/YAML)
3. **Environment variables** (12-factor app compliance)
4. **Runtime API** (hot-reload capable)

```toml
# Example: config.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[server.tls]
enabled = false
cert_path = ""
key_path = ""

[scraping]
default_interval_secs = 300
stagger_requests = true
max_concurrent_requests = 50

[storage]
type = "memory"  # or "sqlite", "postgres"
# database_url = "postgres://..."

[rate_limiting]
api_requests_per_minute = 100
api_burst_size = 20
```

### 2. Core Orchestrator

The brain of the system, responsible for:

- **Scheduling**: Coordinating when each streamer/discovery task runs
- **Rate Limiting**: Ensuring we don't exceed platform API limits
- **Health Monitoring**: Tracking platform API health, circuit breaking
- **Discovery Coordination**: Managing automatic streamer discovery

### 3. Platform Providers (Plugin System)

Each platform is implemented as a separate crate implementing the `PlatformProvider` trait:

```rust
#[async_trait]
pub trait PlatformProvider: Send + Sync {
    /// Unique identifier for this platform
    fn platform_id(&self) -> &'static str;
    
    /// Human-readable name
    fn display_name(&self) -> &'static str;
    
    /// Fetch stream info for a specific streamer
    async fn fetch_stream(&self, streamer_id: &str) -> Result<StreamInfo, ProviderError>;
    
    /// Batch fetch multiple streamers (optimization)
    async fn fetch_streams(&self, streamer_ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        // Default implementation calls fetch_stream for each
        futures::future::join_all(streamer_ids.iter().map(|id| self.fetch_stream(id))).await
    }
    
    /// Check if this provider supports discovery
    fn supports_discovery(&self) -> bool { false }
    
    /// Discover streamers by filters (tags, categories, etc.)
    async fn discover_streamers(&self, filters: &DiscoveryFilters) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }
    
    /// Platform-specific rate limit info
    fn rate_limit_config(&self) -> RateLimitConfig;
    
    /// Health check
    async fn health_check(&self) -> HealthStatus;
}
```

### 4. Data Store

Flexible storage abstraction supporting:

- **In-memory** (default): Fast, ephemeral, suitable for single-instance deployments
- **SQLite**: Persistent, single-file, good for small-to-medium deployments
- **PostgreSQL**: Scalable, suitable for multi-instance deployments

```rust
#[async_trait]
pub trait StreamStore: Send + Sync {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError>;
    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError>;
    async fn get_all_streams(&self, filters: &StreamFilters) -> Result<Vec<StreamInfo>, StoreError>;
    async fn delete_stream(&self, id: &StreamId) -> Result<(), StoreError>;
    
    // Streamer management
    async fn add_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError>;
    async fn remove_streamer(&self, id: &StreamerId) -> Result<(), StoreError>;
    async fn get_tracked_streamers(&self, platform: Option<&str>) -> Result<Vec<TrackedStreamer>, StoreError>;
}
```

### 5. API Layer

RESTful API built with **axum**, featuring:

- OpenAPI/Swagger documentation
- JSON responses
- Query parameter filtering
- Pagination support
- WebSocket support for real-time updates

---

## Data Models

### StreamInfo

```rust
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
    
    /// Custom metadata (user-defined labels, groups, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### TrackedStreamer

```rust
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
    
    /// Priority for display ordering
    pub priority: Option<i32>,
    
    /// Custom labels for filtering
    pub labels: HashMap<String, String>,
    
    /// Source: "manual" or "discovery"
    pub source: StreamerSource,
    
    /// If discovered, which rule discovered it
    pub discovery_rule_id: Option<String>,
    
    /// When this streamer was added
    pub created_at: DateTime<Utc>,
}
```

### DiscoveryRule

```rust
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
    pub apply_labels: HashMap<String, String>,
    
    /// Group to assign to discovered streamers
    pub apply_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFilters {
    /// Filter by tags (Twitch tags, etc.)
    pub tags: Vec<String>,
    
    /// Filter by game/category ID or name
    pub categories: Vec<String>,
    
    /// Filter by language
    pub languages: Vec<String>,
    
    /// Minimum viewer count
    pub min_viewers: Option<u64>,
    
    /// Maximum viewer count
    pub max_viewers: Option<u64>,
    
    /// Maximum number of streamers to discover
    pub limit: Option<usize>,
}
```

---

## Key Design Decisions

### 1. Async-First with Tokio

All I/O operations are async, using Tokio as the runtime. This enables:

- Efficient handling of many concurrent HTTP requests
- Non-blocking scraping operations
- Natural fit for WebSocket connections

### 2. Trait-Based Platform Abstraction

Platforms implement traits rather than inheriting from base classes:

- Easier to test with mocks
- Clear contract for what each platform must provide
- Optional features via default trait implementations

### 3. Separate Discovery from Tracking

Discovery (finding new streamers) is decoupled from tracking (monitoring known streamers):

- Can run discovery on different schedules
- Manual streamers never get removed by discovery rules
- Clear audit trail of how streamers were added

### 4. Labels Over Rigid Categories

Instead of hardcoded fields like "team" or "featuredRank", we use flexible labels:

- Users define their own organizational structure
- Filter by any combination of labels
- No schema migrations needed for new groupings

### 5. Configuration Hot-Reload

Configuration changes can be applied without restart:

- Add/remove streamers via API
- Update discovery rules
- Modify rate limits

---

## Security Considerations

1. **API Authentication**: Optional API key/JWT authentication
2. **Rate Limiting**: Per-IP and per-API-key rate limiting
3. **Input Validation**: Strict validation of all inputs
4. **Secret Management**: Credentials stored securely, never logged
5. **TLS**: Full TLS support for API and outgoing requests

---

## Observability

### Metrics (Prometheus-compatible)

- `stream_aggregator_scrapes_total{platform, status}`
- `stream_aggregator_scrape_duration_seconds{platform}`
- `stream_aggregator_streams_live{platform}`
- `stream_aggregator_api_requests_total{endpoint, status}`
- `stream_aggregator_discovery_runs_total{rule_id, status}`

### Structured Logging (tracing)

```rust
#[instrument(skip(self), fields(platform = %self.platform_id()))]
async fn fetch_stream(&self, streamer_id: &str) -> Result<StreamInfo, ProviderError> {
    info!(streamer_id, "Fetching stream info");
    // ...
}
```

### Health Endpoints

- `GET /health` - Overall service health
- `GET /health/ready` - Readiness probe (K8s)
- `GET /health/live` - Liveness probe (K8s)
- `GET /health/platforms` - Per-platform health status

---

## Deployment Options

1. **Single Binary**: Compile everything, run anywhere
2. **Docker**: Multi-stage build for minimal image size
3. **Kubernetes**: Helm chart with HPA support
4. **Systemd**: Native systemd service file included

---

## Next Steps

See the following documents for detailed designs:

- [PLATFORMS.md](./PLATFORMS.md) - Platform provider implementation details
- [API.md](./API.md) - REST API specification
- [CONFIGURATION.md](./CONFIGURATION.md) - Configuration and storage design
- [CRATES.md](./CRATES.md) - Crate structure and dependencies
- [MIGRATION.md](./MIGRATION.md) - Migration guide from Node.js version
