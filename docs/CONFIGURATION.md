# Configuration and Storage Design

This document details the configuration system and storage backends for StreamAggregator.

## Configuration System

### Overview

StreamAggregator uses a layered configuration system with the following priority (highest to lowest):

1. **Command-line arguments**
2. **Environment variables**
3. **Configuration file** (TOML)
4. **Default values**

### Configuration File Format

```toml
# config.toml - StreamAggregator Configuration

# =============================================================================
# Server Configuration
# =============================================================================
[server]
# Host to bind to
host = "0.0.0.0"
# HTTP port
port = 8080
# Number of worker threads (0 = auto-detect based on CPU cores)
workers = 0

[server.tls]
# Enable HTTPS
enabled = false
# Path to TLS certificate
cert_path = "/etc/ssl/certs/server.crt"
# Path to TLS private key
key_path = "/etc/ssl/private/server.key"

[server.cors]
# Allowed origins (use ["*"] for any)
allowed_origins = ["*"]
# Allowed methods
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
# Max age for preflight cache (seconds)
max_age = 86400

# =============================================================================
# API Configuration
# =============================================================================
[api]
# Base path for all API endpoints
base_path = "/api/v1"
# Enable OpenAPI documentation endpoint
enable_docs = true
# Docs endpoint path
docs_path = "/docs"

[api.rate_limit]
# Requests per window per IP
requests_per_window = 100
# Window duration in seconds
window_seconds = 60
# Burst size
burst_size = 20
# Enable rate limiting
enabled = true

[api.auth]
# Enable API authentication
enabled = false
# Authentication method: "api_key" or "jwt"
method = "api_key"
# API keys (if method = "api_key")
api_keys = []
# JWT secret (if method = "jwt")
# jwt_secret = ""

# =============================================================================
# Scraping Configuration
# =============================================================================
[scraping]
# Default interval between scrapes (seconds)
default_interval_secs = 300
# Stagger requests to avoid API bursts
stagger_requests = true
# Maximum concurrent requests across all providers
max_concurrent_requests = 50
# Request timeout (seconds)
request_timeout_secs = 30
# Retry failed requests
retry_failed = true
# Maximum retry attempts
max_retries = 3
# Retry backoff multiplier
retry_backoff_multiplier = 2.0

[scraping.circuit_breaker]
# Enable circuit breaker for failing providers
enabled = true
# Failure threshold before opening circuit
failure_threshold = 5
# Time to wait before attempting recovery (seconds)
recovery_timeout_secs = 60

# =============================================================================
# Storage Configuration
# =============================================================================
[storage]
# Storage backend: "memory", "sqlite", "postgres"
backend = "memory"

[storage.sqlite]
# SQLite database file path
path = "./data/streams.db"
# Enable WAL mode for better concurrency
wal_mode = true
# Connection pool size
pool_size = 5

[storage.postgres]
# PostgreSQL connection URL
url = "postgres://user:password@localhost:5432/stream_aggregator"
# Connection pool size
pool_size = 10
# Connection timeout (seconds)
connect_timeout_secs = 5

[storage.cache]
# Enable in-memory cache even with persistent storage
enabled = true
# Cache TTL for stream data (seconds)
stream_ttl_secs = 60
# Maximum cache entries
max_entries = 10000

# =============================================================================
# Discovery Configuration
# =============================================================================
[discovery]
# Enable automatic streamer discovery
enabled = false
# Default discovery interval (seconds)
default_interval_secs = 600
# Maximum streamers to track from discovery (per rule)
max_streamers_per_rule = 100
# Auto-remove streamers that haven't been live in X days (0 = never)
auto_remove_inactive_days = 0
# Protect manually added streamers from auto-removal
protect_manual_streamers = true

# =============================================================================
# Platform Provider Configurations
# =============================================================================
[providers.twitch]
enabled = true
client_id = ""
client_secret = ""
# Override default rate limits (optional)
# rate_limit_requests_per_minute = 800

[providers.youtube]
enabled = true
# Optional: YouTube Data API v3 key for discovery
api_key = ""
# Use HTML scraping instead of API
scraping_mode = "html"

[providers.kick]
enabled = true
# Request delay (seconds) - Kick is sensitive to rapid requests
request_delay_secs = 2

[providers.tiktok]
enabled = true

[providers.dlive]
enabled = true

[providers.trovo]
enabled = true
client_id = ""

[providers.guac]
enabled = true

[providers.angelthump]
enabled = true

[providers.robotstreamer]
enabled = true

[providers.brime]
enabled = true

# =============================================================================
# Observability Configuration
# =============================================================================
[observability]
# Log level: "trace", "debug", "info", "warn", "error"
log_level = "info"
# Log format: "json", "pretty"
log_format = "pretty"

[observability.metrics]
# Enable Prometheus metrics endpoint
enabled = true
# Metrics endpoint path
path = "/metrics"

[observability.tracing]
# Enable distributed tracing
enabled = false
# OTLP endpoint for trace export
otlp_endpoint = ""

# =============================================================================
# Health Check Configuration
# =============================================================================
[health]
# Health check endpoint path
path = "/health"
# Include detailed provider status
detailed = true
```

### Environment Variables

All configuration options can be set via environment variables with the prefix `STREAM_AGG_`:

```bash
# Server
STREAM_AGG_SERVER_HOST=0.0.0.0
STREAM_AGG_SERVER_PORT=8080

# TLS
STREAM_AGG_SERVER_TLS_ENABLED=true
STREAM_AGG_SERVER_TLS_CERT_PATH=/path/to/cert

# Providers
STREAM_AGG_PROVIDERS_TWITCH_CLIENT_ID=your_client_id
STREAM_AGG_PROVIDERS_TWITCH_CLIENT_SECRET=your_secret

# Storage
STREAM_AGG_STORAGE_BACKEND=postgres
STREAM_AGG_STORAGE_POSTGRES_URL=postgres://...
```

### Configuration Loading

```rust
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub api: ApiConfig,
    pub scraping: ScrapingConfig,
    pub storage: StorageConfig,
    pub discovery: DiscoveryConfig,
    pub providers: ProvidersConfig,
    pub observability: ObservabilityConfig,
    pub health: HealthConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with defaults
            .add_source(File::from_str(DEFAULT_CONFIG, config::FileFormat::Toml))
            // Load from config file if exists
            .add_source(File::with_name("config").required(false))
            // Load from environment
            .add_source(
                Environment::with_prefix("STREAM_AGG")
                    .separator("_")
                    .try_parsing(true)
            )
            .build()?;
            
        config.try_deserialize()
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Validate Twitch credentials if enabled
        if self.providers.twitch.enabled {
            if self.providers.twitch.client_id.is_empty() {
                errors.push("Twitch client_id is required when Twitch is enabled".into());
            }
            if self.providers.twitch.client_secret.is_empty() {
                errors.push("Twitch client_secret is required when Twitch is enabled".into());
            }
        }
        
        // Validate Trovo credentials if enabled
        if self.providers.trovo.enabled && self.providers.trovo.client_id.is_empty() {
            errors.push("Trovo client_id is required when Trovo is enabled".into());
        }
        
        // Validate storage
        match self.storage.backend.as_str() {
            "postgres" if self.storage.postgres.url.is_empty() => {
                errors.push("PostgreSQL URL is required when using postgres backend".into());
            }
            _ => {}
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

---

## Storage System

### Storage Trait

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait StreamStore: Send + Sync + 'static {
    // =========================================================================
    // Stream Operations
    // =========================================================================
    
    /// Insert or update stream information
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError>;
    
    /// Get stream by ID
    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError>;
    
    /// Get all streams with optional filtering
    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError>;
    
    /// Delete a stream
    async fn delete_stream(&self, id: &StreamId) -> Result<bool, StoreError>;
    
    /// Bulk upsert streams
    async fn upsert_streams(&self, streams: &[StreamInfo]) -> Result<usize, StoreError> {
        let mut count = 0;
        for stream in streams {
            self.upsert_stream(stream).await?;
            count += 1;
        }
        Ok(count)
    }
    
    // =========================================================================
    // Streamer Tracking Operations
    // =========================================================================
    
    /// Add a streamer to track
    async fn add_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError>;
    
    /// Remove a tracked streamer
    async fn remove_tracked_streamer(
        &self, 
        platform: &str, 
        user_id: &str
    ) -> Result<bool, StoreError>;
    
    /// Get all tracked streamers
    async fn get_tracked_streamers(
        &self, 
        query: &TrackedStreamerQuery
    ) -> Result<Vec<TrackedStreamer>, StoreError>;
    
    /// Update tracked streamer metadata
    async fn update_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
        update: &TrackedStreamerUpdate,
    ) -> Result<bool, StoreError>;
    
    // =========================================================================
    // Discovery Rule Operations
    // =========================================================================
    
    /// Add a discovery rule
    async fn add_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError>;
    
    /// Get all discovery rules
    async fn get_discovery_rules(&self) -> Result<Vec<DiscoveryRule>, StoreError>;
    
    /// Update a discovery rule
    async fn update_discovery_rule(&self, rule: &DiscoveryRule) -> Result<bool, StoreError>;
    
    /// Delete a discovery rule
    async fn delete_discovery_rule(&self, rule_id: &str) -> Result<bool, StoreError>;
    
    // =========================================================================
    // Statistics
    // =========================================================================
    
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StoreStats, StoreError>;
}

/// Query parameters for fetching streams
#[derive(Debug, Clone, Default)]
pub struct StreamQuery {
    /// Filter by platform
    pub platform: Option<String>,
    /// Filter by live status
    pub is_live: Option<bool>,
    /// Filter by labels (all must match)
    pub labels: HashMap<String, String>,
    /// Filter by group
    pub group: Option<String>,
    /// Search in display name or title
    pub search: Option<String>,
    /// Minimum viewer count
    pub min_viewers: Option<u64>,
    /// Sort field
    pub sort_by: SortField,
    /// Sort direction
    pub sort_order: SortOrder,
    /// Pagination: page number (1-indexed)
    pub page: u32,
    /// Pagination: items per page
    pub per_page: u32,
}

#[derive(Debug, Clone)]
pub enum SortField {
    DisplayName,
    ViewerCount,
    Platform,
    LastUpdated,
    Priority,
}

#[derive(Debug, Clone)]
pub struct StreamPage {
    pub streams: Vec<StreamInfo>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}
```

### In-Memory Store

```rust
use dashmap::DashMap;
use std::sync::Arc;

pub struct MemoryStore {
    streams: DashMap<StreamId, StreamInfo>,
    tracked_streamers: DashMap<(String, String), TrackedStreamer>,
    discovery_rules: DashMap<String, DiscoveryRule>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            streams: DashMap::new(),
            tracked_streamers: DashMap::new(),
            discovery_rules: DashMap::new(),
        }
    }
}

#[async_trait]
impl StreamStore for MemoryStore {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError> {
        self.streams.insert(stream.id.clone(), stream.clone());
        Ok(())
    }
    
    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError> {
        Ok(self.streams.get(id).map(|r| r.clone()))
    }
    
    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError> {
        let mut streams: Vec<_> = self.streams
            .iter()
            .map(|r| r.value().clone())
            .filter(|s| {
                // Apply filters
                if let Some(ref platform) = query.platform {
                    if &s.platform != platform { return false; }
                }
                if let Some(is_live) = query.is_live {
                    if s.is_live != is_live { return false; }
                }
                // ... more filters
                true
            })
            .collect();
        
        // Sort
        match query.sort_by {
            SortField::ViewerCount => {
                streams.sort_by(|a, b| {
                    b.viewer_count.unwrap_or(0).cmp(&a.viewer_count.unwrap_or(0))
                });
            }
            // ... other sort fields
        }
        
        // Paginate
        let total = streams.len() as u64;
        let start = ((query.page - 1) * query.per_page) as usize;
        let streams: Vec<_> = streams.into_iter().skip(start).take(query.per_page as usize).collect();
        
        Ok(StreamPage {
            streams,
            total,
            page: query.page,
            per_page: query.per_page,
            total_pages: ((total as f64) / (query.per_page as f64)).ceil() as u32,
        })
    }
    
    // ... other implementations
}
```

### SQLite Store

```rust
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new(config: &SqliteConfig) -> Result<Self, StoreError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.pool_size)
            .connect(&format!("sqlite:{}", config.path))
            .await?;
            
        // Run migrations
        sqlx::migrate!("./migrations/sqlite").run(&pool).await?;
        
        // Enable WAL mode if configured
        if config.wal_mode {
            sqlx::query("PRAGMA journal_mode=WAL")
                .execute(&pool)
                .await?;
        }
        
        Ok(Self { pool })
    }
}

#[async_trait]
impl StreamStore for SqliteStore {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError> {
        sqlx::query(r#"
            INSERT INTO streams (
                id, platform, user_id, display_name, avatar_url, 
                is_live, title, viewer_count, thumbnail_url, category,
                tags, language, started_at, last_updated, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                display_name = excluded.display_name,
                avatar_url = excluded.avatar_url,
                is_live = excluded.is_live,
                title = excluded.title,
                viewer_count = excluded.viewer_count,
                thumbnail_url = excluded.thumbnail_url,
                category = excluded.category,
                tags = excluded.tags,
                language = excluded.language,
                started_at = excluded.started_at,
                last_updated = excluded.last_updated,
                metadata = excluded.metadata
        "#)
        .bind(&stream.id.0)
        .bind(&stream.platform)
        .bind(&stream.user_id)
        .bind(&stream.display_name)
        .bind(&stream.avatar_url)
        .bind(stream.is_live)
        .bind(&stream.title)
        .bind(stream.viewer_count.map(|v| v as i64))
        .bind(&stream.thumbnail_url)
        .bind(&stream.category)
        .bind(serde_json::to_string(&stream.tags)?)
        .bind(&stream.language)
        .bind(stream.started_at)
        .bind(stream.last_updated)
        .bind(serde_json::to_string(&stream.metadata)?)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    // ... other implementations
}
```

### PostgreSQL Store

```rust
use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn new(config: &PostgresConfig) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.pool_size)
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .connect(&config.url)
            .await?;
            
        // Run migrations
        sqlx::migrate!("./migrations/postgres").run(&pool).await?;
        
        Ok(Self { pool })
    }
}

// Similar implementation to SQLite with PostgreSQL-specific optimizations
```

### Database Migrations

```sql
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
    metadata TEXT NOT NULL DEFAULT '{}'  -- JSON object
);

CREATE INDEX idx_streams_platform ON streams(platform);
CREATE INDEX idx_streams_is_live ON streams(is_live);
CREATE INDEX idx_streams_viewer_count ON streams(viewer_count);
CREATE UNIQUE INDEX idx_streams_platform_user ON streams(platform, user_id);

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

CREATE INDEX idx_tracked_platform ON tracked_streamers(platform);
CREATE INDEX idx_tracked_group ON tracked_streamers(group_name);
CREATE INDEX idx_tracked_source ON tracked_streamers(source);

CREATE TABLE IF NOT EXISTS discovery_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    filters TEXT NOT NULL,  -- JSON object
    interval_secs INTEGER NOT NULL,
    apply_labels TEXT NOT NULL DEFAULT '{}',  -- JSON object
    apply_group TEXT,
    created_at TEXT NOT NULL,
    last_run_at TEXT
);

CREATE INDEX idx_discovery_platform ON discovery_rules(platform);
CREATE INDEX idx_discovery_enabled ON discovery_rules(enabled);
```

---

## Caching Layer

```rust
use moka::future::Cache;
use std::time::Duration;

pub struct CachedStore<S: StreamStore> {
    inner: S,
    stream_cache: Cache<StreamId, StreamInfo>,
    query_cache: Cache<String, StreamPage>,
}

impl<S: StreamStore> CachedStore<S> {
    pub fn new(inner: S, config: &CacheConfig) -> Self {
        let stream_cache = Cache::builder()
            .max_capacity(config.max_entries as u64)
            .time_to_live(Duration::from_secs(config.stream_ttl_secs))
            .build();
            
        let query_cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(10))  // Short TTL for queries
            .build();
            
        Self {
            inner,
            stream_cache,
            query_cache,
        }
    }
}

#[async_trait]
impl<S: StreamStore> StreamStore for CachedStore<S> {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError> {
        // Update inner store
        self.inner.upsert_stream(stream).await?;
        
        // Update cache
        self.stream_cache.insert(stream.id.clone(), stream.clone()).await;
        
        // Invalidate query cache
        self.query_cache.invalidate_all();
        
        Ok(())
    }
    
    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError> {
        // Try cache first
        if let Some(stream) = self.stream_cache.get(id).await {
            return Ok(Some(stream));
        }
        
        // Fall back to inner store
        let stream = self.inner.get_stream(id).await?;
        
        // Populate cache
        if let Some(ref s) = stream {
            self.stream_cache.insert(id.clone(), s.clone()).await;
        }
        
        Ok(stream)
    }
    
    // ... other implementations
}
```

---

## Streamers Configuration File

For backward compatibility and simple deployments, support a `streamers.toml` file:

```toml
# streamers.toml - Manual Streamer List

# Simple format
[[streamers]]
platform = "twitch"
user_id = "ninja"

[[streamers]]
platform = "twitch"
user_id = "shroud"
custom_name = "Shroud Gaming"
group = "fps-pros"
priority = 1
labels = { country = "us", team = "sentinels" }

[[streamers]]
platform = "youtube"
user_id = "UC-lHJZR3Gqxm24_Vd_AJ5Yw"  # PewDiePie
group = "entertainers"

[[streamers]]
platform = "kick"
user_id = "xqc"

# Discovery rules
[[discovery]]
id = "norwegian-twitch"
name = "Norwegian Twitch Streamers"
platform = "twitch"
enabled = true
interval_secs = 600

[discovery.filters]
languages = ["no"]
min_viewers = 5

[discovery.apply]
group = "norwegian"
labels = { country = "no", source = "auto-discovery" }

[[discovery]]
id = "valorant-streamers"
name = "Valorant Streamers"
platform = "twitch"
enabled = true
interval_secs = 300

[discovery.filters]
categories = ["Valorant"]
min_viewers = 100
limit = 50

[discovery.apply]
group = "valorant"
```

### Loading Streamers File

```rust
#[derive(Debug, Deserialize)]
pub struct StreamersFile {
    #[serde(default)]
    pub streamers: Vec<StreamerEntry>,
    #[serde(default)]
    pub discovery: Vec<DiscoveryEntry>,
}

#[derive(Debug, Deserialize)]
pub struct StreamerEntry {
    pub platform: String,
    pub user_id: String,
    pub custom_name: Option<String>,
    pub group: Option<String>,
    pub priority: Option<i32>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

impl StreamersFile {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(Into::into)
    }
    
    pub async fn import_to_store(&self, store: &dyn StreamStore) -> Result<ImportResult, StoreError> {
        let mut imported = 0;
        let mut failed = 0;
        
        for entry in &self.streamers {
            let streamer = TrackedStreamer {
                platform: entry.platform.clone(),
                user_id: entry.user_id.clone(),
                custom_name: entry.custom_name.clone(),
                group: entry.group.clone(),
                priority: entry.priority,
                labels: entry.labels.clone(),
                source: StreamerSource::Manual,
                discovery_rule_id: None,
                created_at: Utc::now(),
            };
            
            match store.add_tracked_streamer(&streamer).await {
                Ok(_) => imported += 1,
                Err(e) => {
                    tracing::warn!(
                        platform = %entry.platform,
                        user_id = %entry.user_id,
                        error = %e,
                        "Failed to import streamer"
                    );
                    failed += 1;
                }
            }
        }
        
        // Import discovery rules
        for entry in &self.discovery {
            // Convert to DiscoveryRule and add to store
        }
        
        Ok(ImportResult { imported, failed })
    }
}
```
