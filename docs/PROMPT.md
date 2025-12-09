# StreamAggregator - Implementation Prompt

You are implementing a modern Rust rewrite of a Node.js live stream aggregation service. This document provides context and guidance for implementation.

## Project Overview

**StreamAggregator** is a high-performance, modular service for aggregating live stream information across multiple platforms. It replaces the legacy [LSND](https://github.com/livestreamnorge/lsnd) Node.js project.

### Key Goals
- **Platform-agnostic**: Support any streaming platform through a plugin system
- **Flexible discovery**: Support both manual streamer lists AND automatic discovery (tags, categories, games)
- **Not locked to any region**: Unlike the original (Norwegian streamers), this is generic
- **Modular crate-based architecture**: Each platform is a separate crate

## Reference Materials

### Design Documents (in this repo)
- `docs/ARCHITECTURE.md` - High-level system architecture, core components, data models
- `docs/PLATFORMS.md` - Platform provider implementations, HTTP client strategy, TikTok WebSocket protocol
- `docs/CONFIGURATION.md` - Config file format (TOML), storage backends, caching
- `docs/API.md` - REST API specification, WebSocket API, endpoints
- `docs/CRATES.md` - Workspace structure, dependencies, feature flags
- `docs/MIGRATION.md` - Migration guide from Node.js version

### Legacy Codebase Reference
The original Node.js implementation is at: https://github.com/livestreamnorge/lsnd

Key files to reference for API behavior and scraping logic:
- `index.js` - Express server, scheduling logic, endpoint definitions
- `scrapers.js` - Scraper orchestration
- `scrapers/*.js` - Individual platform implementations:
  - `twitch.js` - OAuth2 token management, Helix API
  - `youtube.js` - HTML scraping with regex
  - `kick.js` - XSRF tokens, TLS fingerprint spoofing (cycletls)
  - `tiktok.js` - Uses tiktok-live-connector library
  - `dlive.js` - GraphQL API
  - `trovo.js` - Two-step lookup (username → channel_id → data)
  - `guac.js`, `angelthump.js`, `robotstreamer.js` - Simple REST APIs

## Technical Decisions (Already Made)

### HTTP Client: wreq
Use **wreq** (not reqwest) as the HTTP client for ALL platforms:
- Fork of reqwest, 100% API compatible
- JA3/JA4/HTTP2 TLS fingerprint emulation
- Required for Kick (Cloudflare bypass), beneficial everywhere
- https://github.com/0x676e67/wreq

```rust
use wreq::Client;
use wreq_util::Emulation;

// With browser emulation (required for Kick)
let client = Client::builder()
    .emulation(Emulation::Chrome131)
    .build()?;

// Or simple client for standard APIs
let client = Client::new();
```

### TikTok: Custom WebSocket Crate
Create an internal `tiktok-live` crate based on https://github.com/zerodytrash/TikTok-Live-Connector:
- WebSocket connection to TikTok's Webcast push service
- Protobuf message decoding (prost)
- Real-time events: chat, gifts, viewer counts, stream end

### Platforms to Support (9 total)
1. **Twitch** - OAuth2, Helix API, full discovery support
2. **YouTube** - HTML scraping (or Data API v3 with key)
3. **Kick** - wreq with Chrome emulation (required)
4. **TikTok** - WebSocket + Protobuf via tiktok-live crate
5. **DLive** - GraphQL API
6. **Trovo** - REST API with Client ID
7. **Guac** - Simple REST API
8. **AngelThump** - Two-endpoint lookup
9. **RobotStreamer** - Simple REST API (HTTP only, not HTTPS)

**Removed:** Brime (platform no longer active)

### Storage Backends
- **Memory** (default) - In-memory with dashmap
- **SQLite** - Persistent, single-file
- **PostgreSQL** - Scalable, multi-instance

### Web Framework
- **axum** for HTTP API
- **tokio-tungstenite** for WebSocket (TikTok, and client-facing real-time API)

## Workspace Structure

```
stream-aggregator/
├── Cargo.toml                    # Workspace root
├── config.toml                   # Default configuration
├── streamers.toml                # Streamer list (optional)
│
├── crates/
│   ├── stream-aggregator/        # Main binary
│   ├── stream-aggregator-core/   # Core types, traits
│   ├── stream-aggregator-api/    # HTTP API (axum)
│   ├── stream-aggregator-store/  # Storage abstraction
│   ├── stream-aggregator-scheduler/ # Scraping scheduler
│   ├── tiktok-live/              # TikTok WebSocket client
│   │
│   └── providers/
│       ├── stream-aggregator-provider-twitch/
│       ├── stream-aggregator-provider-youtube/
│       ├── stream-aggregator-provider-kick/
│       ├── stream-aggregator-provider-tiktok/
│       ├── stream-aggregator-provider-dlive/
│       ├── stream-aggregator-provider-trovo/
│       ├── stream-aggregator-provider-guac/
│       ├── stream-aggregator-provider-angelthump/
│       └── stream-aggregator-provider-robotstreamer/
│
├── migrations/
│   ├── sqlite/
│   └── postgres/
│
└── proto/                        # TikTok protobuf definitions
    └── webcast.proto
```

## Implementation Order (Suggested)

### Phase 1: Foundation
1. Set up workspace with Cargo.toml
2. Implement `stream-aggregator-core`:
   - `StreamInfo`, `TrackedStreamer`, `DiscoveryRule` models
   - `PlatformProvider` trait
   - `StreamStore` trait
   - Error types
3. Implement `stream-aggregator-store` (memory backend first)

### Phase 2: First Provider
4. Implement `stream-aggregator-provider-twitch`:
   - OAuth2 token management
   - Helix API integration
   - This validates the provider trait design

### Phase 3: API Layer
5. Implement `stream-aggregator-api`:
   - Basic endpoints: `/streams`, `/platforms`, `/health`
   - Rate limiting middleware

### Phase 4: Scheduler
6. Implement `stream-aggregator-scheduler`:
   - Periodic scraping
   - Rate limiting per provider
   - Staggered requests

### Phase 5: Main Binary
7. Wire everything together in `stream-aggregator`:
   - Config loading
   - Provider registration
   - Server startup

### Phase 6: Remaining Providers
8. Implement remaining providers (easiest to hardest):
   - DLive (simple GraphQL)
   - Guac, AngelThump, RobotStreamer (simple REST)
   - Trovo (two-step lookup)
   - YouTube (HTML scraping)
   - Kick (requires wreq emulation)
   - TikTok (requires tiktok-live crate)

### Phase 7: TikTok Crate
9. Implement `tiktok-live`:
   - Room ID discovery
   - WebSocket connection
   - Protobuf decoding

### Phase 8: Advanced Features
10. Discovery system (Twitch tags/categories)
11. SQLite/PostgreSQL storage backends
12. WebSocket API for real-time updates
13. OpenAPI documentation

## Key Traits

### PlatformProvider
```rust
#[async_trait]
pub trait PlatformProvider: Send + Sync + 'static {
    fn platform_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    
    async fn fetch_stream(&self, streamer_id: &str) -> Result<StreamInfo, ProviderError>;
    
    async fn fetch_streams_batch(&self, ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        // Default: sequential calls
    }
    
    fn supports_discovery(&self) -> bool { false }
    async fn discover_streamers(&self, filters: &DiscoveryFilters) -> Result<Vec<DiscoveredStreamer>, ProviderError>;
    
    fn rate_limit_config(&self) -> RateLimitConfig;
    async fn health_check(&self) -> HealthStatus;
}
```

### StreamStore
```rust
#[async_trait]
pub trait StreamStore: Send + Sync + 'static {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError>;
    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError>;
    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError>;
    
    async fn add_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError>;
    async fn get_tracked_streamers(&self, query: &TrackedStreamerQuery) -> Result<Vec<TrackedStreamer>, StoreError>;
    
    async fn add_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError>;
    async fn get_discovery_rules(&self) -> Result<Vec<DiscoveryRule>, StoreError>;
}
```

## Core Data Models

```rust
pub struct StreamInfo {
    pub id: StreamId,              // Hash of platform + user_id
    pub platform: String,
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_live: bool,
    pub title: Option<String>,
    pub viewer_count: Option<u64>,
    pub thumbnail_url: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub language: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct TrackedStreamer {
    pub platform: String,
    pub user_id: String,
    pub custom_name: Option<String>,
    pub group: Option<String>,
    pub priority: Option<i32>,
    pub labels: HashMap<String, String>,
    pub source: StreamerSource,  // Manual or Discovery
    pub created_at: DateTime<Utc>,
}
```

## API Endpoints (Core)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/streams` | List streams (with filtering) |
| GET | `/api/v1/streams/:id` | Get single stream |
| GET | `/api/v1/streamers` | List tracked streamers |
| POST | `/api/v1/streamers` | Add streamer to track |
| DELETE | `/api/v1/streamers/:platform/:user_id` | Remove streamer |
| GET | `/api/v1/platforms` | List supported platforms |
| GET | `/api/v1/health` | Health check |

## Build Requirements

wreq requires BoringSSL:
```bash
# Ubuntu/Debian
sudo apt-get install build-essential cmake perl pkg-config libclang-dev musl-tools -y

# macOS  
brew install cmake perl llvm
```

## Testing Approach

1. **Unit tests**: Mock HTTP responses with wiremock
2. **Integration tests**: Real API calls (ignored by default, run in CI with credentials)
3. **Each provider**: Has its own test suite

## Questions to Consider During Implementation

1. How should we handle provider initialization failures? (fail fast vs. graceful degradation)
2. Should discovery rules persist across restarts? (yes, in store)
3. How to handle WebSocket reconnection for TikTok? (exponential backoff)
4. Rate limit sharing across provider instances? (global rate limiter in scheduler)

## Getting Help

- Design docs are the source of truth
- Reference the Node.js implementation for edge cases
- The TikTok-Live-Connector TypeScript code has protocol details

---

**Start with Phase 1.** Set up the workspace and implement core types first. The trait definitions will drive everything else.
