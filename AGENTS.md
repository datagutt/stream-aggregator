# AGENTS.md

## Build/Test Commands

- `cargo build` - Build the project
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test (e.g., `cargo test library_test`)
- `cargo clippy` - Lint code
- `cargo fmt` - Format code

## Code Style

- **Imports**: Group std, external crates, then local modules with blank lines between
- **Formatting**: Use `cargo fmt` - tabs for indentation, snake_case for variables/functions. DO NOT use emojis at all.
- **Types**: Explicit types preferred, use `Result<T, E>` for error handling with `thiserror`
- **Naming**: snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE_CASE for constants
- **Error Handling**: Use `Result` types, `thiserror` for custom errors, `anyhow` for application errors
- **Async**: Use `async/await`, prefer `tokio` primitives, avoid blocking operations
- **Database**: Use sqlx, async queries, proper error propagation
- **Comments**: Minimal inline comments, focus on why not what, no TODO comments in production code

## Reference Materials

### Design Documents (in this repo)

- `docs/ARCHITECTURE.md` - High-level system architecture, core components, data models
- `docs/PLATFORMS.md` - Platform provider implementations, HTTP client strategy, TikTok WebSocket protocol
- `docs/CONFIGURATION.md` - Config file format (TOML), storage backends, caching
- `docs/API.md` - REST API specification, WebSocket API, endpoints
- `docs/CRATES.md` - Workspace structure, dependencies, feature flags
- `docs/MIGRATION.md` - Migration guide from Node.js version

### Legacy Codebase Reference

The original Node.js implementation is at: <https://github.com/livestreamnorge/lsnd>

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

## Technical Decisions

### HTTP Client: wreq

Use **wreq** (not reqwest) as the HTTP client for ALL platforms:

- Fork of reqwest, 100% API compatible
- JA3/JA4/HTTP2 TLS fingerprint emulation
- Required for Kick (Cloudflare bypass), beneficial everywhere
- <https://github.com/0x676e67/wreq>

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
