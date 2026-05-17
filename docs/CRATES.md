# Crate Structure and Dependencies

This document details the Rust workspace structure, crate organization, and dependency choices.

## Workspace Layout

```
stream-aggregator/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── config.toml                   # Default configuration
├── streamers.toml                # Streamer list (optional)
│
├── crates/
│   ├── stream-aggregator/        # Main binary crate
│   ├── stream-aggregator-core/   # Core types and traits
│   ├── stream-aggregator-api/    # HTTP API layer
│   ├── stream-aggregator-store/  # Storage abstraction
│   ├── stream-aggregator-scheduler/ # Scraping scheduler
│   │
│   ├── tiktok-live/              # TikTok Live WebSocket client (custom crate)
│   │
│   └── providers/                # Platform providers
│       ├── stream-aggregator-provider-twitch/
│       ├── stream-aggregator-provider-youtube/
│       ├── stream-aggregator-provider-kick/
│       ├── stream-aggregator-provider-tiktok/
│       ├── stream-aggregator-provider-guac/
│       ├── stream-aggregator-provider-angelthump/
│       └── stream-aggregator-provider-robotstreamer/
│
├── migrations/
│   ├── sqlite/
│   └── postgres/
│
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml
│
└── docs/
```

---

## Root Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/stream-aggregator",
    "crates/stream-aggregator-core",
    "crates/stream-aggregator-api",
    "crates/stream-aggregator-store",
    "crates/stream-aggregator-scheduler",
    "crates/providers/*",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT"
repository = "https://github.com/datagutt/stream-aggregator"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# HTTP Client
# wreq is a fork of reqwest with JA3/JA4/HTTP2 fingerprint emulation
# API-compatible with reqwest, so we use it as the default for ALL platforms
# This gives us anti-bot bypass capabilities (required for Kick) everywhere
# https://github.com/0x676e67/wreq
wreq = { workspace = true }
wreq-util = { workspace = true } # Browser emulation presets (Chrome, Safari, Firefox, etc.)

# Optional: standard reqwest for simpler builds without TLS fingerprinting
# Can be used via feature flag for platforms that don't need anti-bot bypass
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "cookies", "gzip"], optional = true }

# Web framework
axum = { version = "0.7", features = ["ws", "macros"] }
tower = { version = "0.4" }
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Protocol Buffers (for TikTok WebSocket messages)
prost = "0.12"
prost-types = "0.12"

# Database - Diesel ORM (supports SQLite and PostgreSQL)
diesel = { version = "2.2", features = ["sqlite", "postgres", "r2d2", "chrono", "serde_json"] }
diesel_migrations = "2.2"

# Caching
moka = { version = "0.12", features = ["future"] }
dashmap = "5.5"

# Configuration
config = { version = "0.14", features = ["toml"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging & tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Async utilities
async-trait = "0.1"
futures = "0.3"

# WebSocket (for TikTok)
tokio-tungstenite = { version = "0.21", features = ["rustls-tls-native-roots"] }

# Rate limiting
governor = "0.6"

# Cryptography (for ID hashing)
sha2 = "0.10"
hex = "0.4"

# Metrics
metrics = "0.22"
metrics-exporter-prometheus = "0.13"

# Testing
wiremock = "0.6"
tokio-test = "0.4"
```

---

## Crate Details

### 1. stream-aggregator-core

Core types, traits, and shared utilities.

```toml
# crates/stream-aggregator-core/Cargo.toml
[package]
name = "stream-aggregator-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }
```

**Contents:**

- `StreamInfo` - Stream data model
- `TrackedStreamer` - Tracked streamer model
- `DiscoveryRule` - Discovery rule model
- `PlatformProvider` trait
- `StreamStore` trait
- Error types
- ID generation utilities

```rust
// crates/stream-aggregator-core/src/lib.rs
pub mod models;
pub mod traits;
pub mod errors;
pub mod id;

pub use models::*;
pub use traits::*;
pub use errors::*;
```

---

### 2. stream-aggregator-store

Storage implementations using Diesel ORM.

```toml
# crates/stream-aggregator-store/Cargo.toml
[package]
name = "stream-aggregator-store"
version.workspace = true
edition.workspace = true

[features]
default = ["memory"]
memory = ["dashmap"]
diesel = ["dep:diesel", "dep:r2d2", "dep:diesel_migrations"]
diesel-sqlite = ["diesel"]
diesel-postgres = ["diesel"]
all = ["memory", "diesel-sqlite", "diesel-postgres"]

[dependencies]
stream-aggregator-core = { path = "../stream-aggregator-core" }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# Optional
dashmap = { workspace = true, optional = true }
moka = { workspace = true }

# Diesel ORM with SQLite and PostgreSQL support
diesel = { workspace = true, optional = true }
r2d2 = { version = "0.8", optional = true }
diesel_migrations = { workspace = true, optional = true }
```

**Contents:**

- `MemoryStore` - In-memory implementation (DashMap-based)
- `DieselStore` - Diesel ORM-based implementation
  - Supports SQLite (via `diesel-sqlite` feature)
  - Supports PostgreSQL (via `diesel-postgres` feature)
  - Automatic migrations on startup
  - Connection pooling (r2d2)
  - Type-safe queries

---

### 3. stream-aggregator-api

HTTP API layer.

```toml
# crates/stream-aggregator-api/Cargo.toml
[package]
name = "stream-aggregator-api"
version.workspace = true
edition.workspace = true

[dependencies]
stream-aggregator-core = { path = "../stream-aggregator-core" }
stream-aggregator-store = { path = "../stream-aggregator-store" }
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
governor = { workspace = true }
utoipa = { version = "4.2", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "6.0", features = ["axum"] }
```

**Contents:**

- Router setup
- Request handlers
- Middleware (auth, rate limiting, CORS)
- WebSocket handler
- OpenAPI generation

---

### 4. stream-aggregator-scheduler

Scraping scheduler and orchestrator.

```toml
# crates/stream-aggregator-scheduler/Cargo.toml
[package]
name = "stream-aggregator-scheduler"
version.workspace = true
edition.workspace = true

[dependencies]
stream-aggregator-core = { path = "../stream-aggregator-core" }
stream-aggregator-store = { path = "../stream-aggregator-store" }
tokio = { workspace = true }
futures = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }
governor = { workspace = true }
metrics = { workspace = true }
```

**Contents:**

- `Scheduler` - Main scheduling loop
- `RateLimiter` - Per-provider rate limiting
- `CircuitBreaker` - Failure protection
- `DiscoveryCoordinator` - Discovery rule execution

---

### 5. Platform Provider Crates

Each provider follows the same structure:

```toml
# crates/providers/stream-aggregator-provider-twitch/Cargo.toml
[package]
name = "stream-aggregator-provider-twitch"
version.workspace = true
edition.workspace = true

[dependencies]
stream-aggregator-core = { path = "../../stream-aggregator-core" }
tokio = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
wiremock = { workspace = true }
tokio-test = { workspace = true }
```

**Special Dependencies by Provider:**

| Provider | Special Dependencies | Notes |
|----------|---------------------|-------|
| Twitch | (none - standard OAuth2) | |
| YouTube | `scraper = "0.18"` for HTML parsing | |
| Kick | Browser emulation via `wreq-util` | TLS fingerprinting critical |
| TikTok | `tiktok-live` (internal crate), `prost`, `tokio-tungstenite` | WebSocket-based |
| Guac | (none) | |
| AngelThump | (none) | |
| RobotStreamer | (none) | HTTP only (not HTTPS) |

> **Note:** All providers use `wreq` as the default HTTP client. This provides TLS fingerprinting
> capabilities across the board, which is required for Kick and beneficial for other platforms
> that may add anti-bot protection in the future. `wreq` is API-compatible with `reqwest`.

> **Note:** Brime has been removed as the platform is no longer active.

---

### 6. Main Binary Crate

```toml
# crates/stream-aggregator/Cargo.toml
[package]
name = "stream-aggregator"
version.workspace = true
edition.workspace = true

[[bin]]
name = "stream-aggregator"
path = "src/main.rs"

[features]
default = ["all-providers", "sqlite"]
all-providers = [
    "provider-twitch",
    "provider-youtube",
    "provider-kick",
    "provider-tiktok",
    "provider-guac",
    "provider-angelthump",
    "provider-robotstreamer",
]
provider-twitch = ["stream-aggregator-provider-twitch"]
provider-youtube = ["stream-aggregator-provider-youtube"]
provider-kick = ["stream-aggregator-provider-kick"]
provider-tiktok = ["stream-aggregator-provider-tiktok"]
provider-guac = ["stream-aggregator-provider-guac"]
provider-angelthump = ["stream-aggregator-provider-angelthump"]
provider-robotstreamer = ["stream-aggregator-provider-robotstreamer"]
memory = ["stream-aggregator-store/memory"]
sqlite = ["stream-aggregator-store/sqlite"]
postgres = ["stream-aggregator-store/postgres"]

[dependencies]
# Internal crates
stream-aggregator-core = { path = "../stream-aggregator-core" }
stream-aggregator-api = { path = "../stream-aggregator-api" }
stream-aggregator-store = { path = "../stream-aggregator-store" }
stream-aggregator-scheduler = { path = "../stream-aggregator-scheduler" }

# Optional providers
stream-aggregator-provider-twitch = { path = "../providers/stream-aggregator-provider-twitch", optional = true }
stream-aggregator-provider-youtube = { path = "../providers/stream-aggregator-provider-youtube", optional = true }
stream-aggregator-provider-kick = { path = "../providers/stream-aggregator-provider-kick", optional = true }
stream-aggregator-provider-tiktok = { path = "../providers/stream-aggregator-provider-tiktok", optional = true }
stream-aggregator-provider-guac = { path = "../providers/stream-aggregator-provider-guac", optional = true }
stream-aggregator-provider-angelthump = { path = "../providers/stream-aggregator-provider-angelthump", optional = true }
stream-aggregator-provider-robotstreamer = { path = "../providers/stream-aggregator-provider-robotstreamer", optional = true }

# Runtime & config
tokio = { workspace = true }
config = { workspace = true }
toml = { workspace = true }
serde = { workspace = true }

# Logging
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

# Metrics
metrics = { workspace = true }
metrics-exporter-prometheus = { workspace = true }

# CLI
clap = { version = "4.4", features = ["derive", "env"] }

[dev-dependencies]
wiremock = { workspace = true }
```

**Main Entry Point:**

```rust
// crates/stream-aggregator/src/main.rs
use clap::Parser;
use tracing_subscriber::prelude::*;

#[derive(Parser)]
#[command(name = "stream-aggregator")]
#[command(about = "Multi-platform live stream aggregator")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    /// Path to streamers file
    #[arg(short, long)]
    streamers: Option<String>,
    
    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(&cli.log_level))
        .init();
    
    // Load configuration
    let config = stream_aggregator::config::load(&cli.config)?;
    config.validate()?;
    
    // Initialize components
    let store = stream_aggregator::store::create(&config.storage).await?;
    let providers = stream_aggregator::providers::create_registry(&config.providers)?;
    let scheduler = stream_aggregator::scheduler::Scheduler::new(
        store.clone(),
        providers.clone(),
        &config.scraping,
    );
    
    // Load streamers file if provided
    if let Some(path) = cli.streamers {
        stream_aggregator::import::load_streamers_file(&path, store.as_ref()).await?;
    }
    
    // Start scheduler
    let scheduler_handle = tokio::spawn(scheduler.run());
    
    // Start API server
    let api = stream_aggregator_api::create_router(
        store,
        providers,
        &config.api,
    );
    
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, api).await?;
    
    scheduler_handle.abort();
    Ok(())
}
```

---

## Internal Crates

### tiktok-live

A Rust implementation of the TikTok Live WebSocket protocol, based on [TikTok-Live-Connector](https://github.com/zerodytrash/TikTok-Live-Connector).

```toml
# crates/tiktok-live/Cargo.toml
[package]
name = "tiktok-live"
version.workspace = true
edition.workspace = true
description = "TikTok Live WebSocket client for Rust"

[dependencies]
tokio = { workspace = true }
tokio-tungstenite = { workspace = true }
futures = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
prost = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
reqwest = { workspace = true }  # For initial room info fetch

[build-dependencies]
prost-build = "0.12"
```

**Features:**

- Connect to TikTok LIVE via WebSocket
- Decode protobuf messages (chat, gifts, likes, member joins, etc.)
- Room info fetching
- Reconnection handling

**Protocol Overview:**

TikTok Live uses a protobuf-based WebSocket protocol:

1. **Room Discovery**: Fetch room ID from `https://www.tiktok.com/@{username}/live`
2. **WebSocket Connection**: Connect to TikTok's Webcast push service
3. **Message Decoding**: Protobuf messages for events (chat, gifts, etc.)

```rust
// crates/tiktok-live/src/lib.rs
pub mod client;
pub mod messages;
pub mod proto;  // Generated from .proto files
pub mod error;

pub use client::TikTokLiveClient;
pub use messages::*;

/// Events emitted by TikTok Live
#[derive(Debug, Clone)]
pub enum TikTokEvent {
    Connected { room_id: String },
    Chat { user: User, comment: String },
    Gift { user: User, gift_id: u32, count: u32 },
    Like { user: User, count: u32 },
    Member { user: User },
    ViewerCount { count: u64 },
    StreamEnd,
    Disconnected,
}

/// Minimal user info
#[derive(Debug, Clone)]
pub struct User {
    pub user_id: String,
    pub unique_id: String,  // @username
    pub nickname: String,
    pub avatar_url: Option<String>,
}
```

---

## Dependency Rationale

### Async Runtime: Tokio

- Industry standard for async Rust
- Excellent ecosystem support
- Multi-threaded by default

### HTTP Client: wreq (Default)

We use **wreq** as the default HTTP client for **all platforms**:

- Fork of `reqwest` - **100% API compatible**
- JA3/JA4/HTTP2 TLS fingerprint emulation
- Uses BoringSSL for precise TLS control
- Browser emulation presets via `wreq-util`
- See: <https://github.com/0x676e67/wreq>

**Why wreq for everything?**

1. **Future-proofing**: Any platform could add Cloudflare/anti-bot protection
2. **Consistency**: One HTTP client across all providers
3. **No downside**: Same API as reqwest, just more capabilities
4. **Required for Kick**: Cloudflare blocks standard TLS fingerprints

```rust
// Example: Using wreq with browser emulation
use wreq::Client;
use wreq_util::Emulation;

// Default client (no emulation needed for most APIs)
let client = Client::new();

// Or with browser emulation for anti-bot protected sites
let client = Client::builder()
    .emulation(Emulation::Chrome131)
    .build()?;

let response = client.get("https://api.twitch.tv/helix/streams")
    .header("Authorization", "Bearer ...")
    .send()
    .await?;
```

#### Optional: reqwest fallback

For simpler builds without BoringSSL compilation, `reqwest` can be enabled via feature flag:

```toml
[features]
default = ["wreq-client"]
wreq-client = ["wreq", "wreq-util"]
reqwest-client = ["reqwest"]  # Lighter build, but Kick won't work
```

### Web Framework: Axum

- Built by Tokio team
- Tower middleware ecosystem
- Type-safe extractors
- WebSocket support built-in

### Database: SQLx

- Compile-time query checking
- Async native
- Multi-database support
- No ORM overhead

### Caching: Moka + DashMap

- Moka: TTL-based caching with async support
- DashMap: Concurrent HashMap for simple cases

### Serialization: Serde + Prost

- Serde: De facto standard for JSON/TOML
- Prost: Protocol Buffers for TikTok WebSocket messages

### WebSocket: tokio-tungstenite

- Native async WebSocket support
- TLS support via rustls
- Used for TikTok Live real-time connection

### Rate Limiting: Governor

- Token bucket algorithm
- Async support
- Flexible configuration

### Metrics: metrics + prometheus

- Lightweight facade pattern
- Prometheus exporter included
- Easy to add custom metrics

---

## Build Configurations

### Development

```bash
# Build with all features
cargo build

# Build with specific providers only
cargo build --no-default-features --features "provider-twitch,provider-youtube,sqlite"
```

### Release

```bash
# Optimized release build
cargo build --release

# With LTO for smaller binary
RUSTFLAGS="-C lto=thin" cargo build --release
```

### Docker

```dockerfile
# docker/Dockerfile
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/stream-aggregator /usr/local/bin/
COPY config.toml /etc/stream-aggregator/

EXPOSE 8080
CMD ["stream-aggregator", "--config", "/etc/stream-aggregator/config.toml"]
```

---

## Testing Strategy

### Unit Tests

Each crate has its own unit tests:

```bash
cargo test -p stream-aggregator-core
cargo test -p stream-aggregator-store
cargo test -p stream-aggregator-provider-twitch
```

### Integration Tests

```bash
# Requires test credentials
cargo test --test integration
```

### End-to-End Tests

```bash
# Start test server
cargo run &
# Run e2e tests
cargo test --test e2e
```

---

## CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --all-features

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - run: cargo clippy --all-features -- -D warnings

  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --check

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release

  docker:
    runs-on: ubuntu-latest
    needs: [check, test]
    steps:
      - uses: actions/checkout@v4
      - uses: docker/build-push-action@v5
        with:
          context: .
          file: docker/Dockerfile
          push: false
          tags: stream-aggregator:latest
```
