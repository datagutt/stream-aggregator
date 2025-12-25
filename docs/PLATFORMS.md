# Platform Providers Design

This document details the design and implementation of platform providers for StreamAggregator.

## Provider Trait Definition

```rust
use async_trait::async_trait;
use std::time::Duration;

/// Core trait that all platform providers must implement
#[async_trait]
pub trait PlatformProvider: Send + Sync + 'static {
    /// Unique lowercase identifier (e.g., "twitch", "youtube")
    fn platform_id(&self) -> &'static str;
    
    /// Human-readable display name (e.g., "Twitch", "YouTube")
    fn display_name(&self) -> &'static str;
    
    /// Platform icon URL or identifier
    fn icon(&self) -> Option<&'static str> { None }
    
    /// Base URL for the platform
    fn base_url(&self) -> &'static str;
    
    /// Fetch stream information for a single streamer
    async fn fetch_stream(&self, streamer_id: &str) -> Result<StreamInfo, ProviderError>;
    
    /// Batch fetch multiple streamers (default: sequential calls)
    async fn fetch_streams_batch(
        &self, 
        streamer_ids: &[String]
    ) -> Vec<(String, Result<StreamInfo, ProviderError>)> {
        let futures = streamer_ids.iter().map(|id| async move {
            (id.clone(), self.fetch_stream(id).await)
        });
        futures::future::join_all(futures).await
    }
    
    /// Maximum batch size for batch operations
    fn max_batch_size(&self) -> usize { 1 }
    
    /// Whether this provider supports automatic discovery
    fn supports_discovery(&self) -> bool { false }
    
    /// Available discovery filter types
    fn discovery_capabilities(&self) -> DiscoveryCapabilities {
        DiscoveryCapabilities::default()
    }
    
    /// Discover streamers based on filters
    async fn discover_streamers(
        &self, 
        filters: &DiscoveryFilters
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }
    
    /// Rate limiting configuration for this provider
    fn rate_limit_config(&self) -> RateLimitConfig;
    
    /// Perform health check
    async fn health_check(&self) -> HealthStatus {
        // Default: try to fetch a known good streamer
        HealthStatus::Unknown
    }
    
    /// Initialize the provider (called once at startup)
    async fn initialize(&mut self) -> Result<(), ProviderError> { Ok(()) }
    
    /// Shutdown the provider gracefully
    async fn shutdown(&self) -> Result<(), ProviderError> { Ok(()) }
}

/// Discovery capabilities for a platform
#[derive(Debug, Clone, Default)]
pub struct DiscoveryCapabilities {
    pub supports_tags: bool,
    pub supports_categories: bool,
    pub supports_languages: bool,
    pub supports_viewer_count_filter: bool,
    pub supports_title_search: bool,
    pub max_results_per_query: Option<usize>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per time window
    pub requests_per_window: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Additional delay between requests (for sensitive APIs)
    pub request_delay: Option<Duration>,
    /// Whether to use exponential backoff on rate limit errors
    pub exponential_backoff: bool,
}
```

---

## Platform Implementations

### 1. Twitch Provider

**Crate**: `stream-aggregator-provider-twitch`

#### Features

- OAuth2 Client Credentials authentication
- Helix API integration
- Full discovery support (tags, categories, languages)
- Batch API calls (up to 100 users per request)

#### Authentication Flow

```rust
pub struct TwitchProvider {
    client: reqwest::Client,
    config: TwitchConfig,
    token_manager: Arc<RwLock<TokenManager>>,
}

struct TokenManager {
    access_token: Option<String>,
    expires_at: Option<Instant>,
}

impl TwitchProvider {
    async fn ensure_valid_token(&self) -> Result<String, ProviderError> {
        let mut token_manager = self.token_manager.write().await;
        
        if let Some(ref token) = token_manager.access_token {
            if token_manager.expires_at.map(|e| e > Instant::now()).unwrap_or(false) {
                return Ok(token.clone());
            }
        }
        
        // Refresh token
        let response = self.client
            .post("https://id.twitch.tv/oauth2/token")
            .form(&[
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await?;
            
        let token_response: TokenResponse = response.json().await?;
        
        token_manager.access_token = Some(token_response.access_token.clone());
        token_manager.expires_at = Some(
            Instant::now() + Duration::from_secs(token_response.expires_in - 60)
        );
        
        Ok(token_response.access_token)
    }
}
```

#### Discovery Implementation

```rust
impl TwitchProvider {
    async fn discover_by_tags(&self, tags: &[String], limit: usize) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        let token = self.ensure_valid_token().await?;
        
        // Twitch allows filtering streams by tags
        let mut url = Url::parse("https://api.twitch.tv/helix/streams")?;
        url.query_pairs_mut()
            .append_pair("first", &limit.min(100).to_string());
        
        let response = self.client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Client-Id", &self.config.client_id)
            .send()
            .await?;
            
        let streams: TwitchStreamsResponse = response.json().await?;
        
        // Filter by tags client-side (Twitch removed tag filtering from API)
        let tag_set: HashSet<_> = tags.iter().map(|t| t.to_lowercase()).collect();
        
        Ok(streams.data
            .into_iter()
            .filter(|s| s.tags.iter().any(|t| tag_set.contains(&t.to_lowercase())))
            .map(|s| DiscoveredStreamer {
                platform: "twitch".to_string(),
                user_id: s.user_id,
                display_name: s.user_name,
                avatar_url: None, // Need separate user lookup
                is_live: true,
                viewer_count: Some(s.viewer_count),
                category: Some(s.game_name),
                tags: s.tags,
            })
            .collect())
    }
    
    async fn discover_by_category(&self, category_id: &str, limit: usize) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        let token = self.ensure_valid_token().await?;
        
        let response = self.client
            .get("https://api.twitch.tv/helix/streams")
            .query(&[
                ("game_id", category_id),
                ("first", &limit.min(100).to_string()),
            ])
            .header("Authorization", format!("Bearer {}", token))
            .header("Client-Id", &self.config.client_id)
            .send()
            .await?;
            
        // ... process response
    }
}
```

#### Rate Limits

```rust
fn rate_limit_config(&self) -> RateLimitConfig {
    RateLimitConfig {
        requests_per_window: 800,  // Twitch allows 800 requests per minute
        window_duration: Duration::from_secs(60),
        request_delay: None,
        exponential_backoff: true,
    }
}
```

---

### 2. YouTube Provider

**Crate**: `stream-aggregator-provider-youtube`

#### Features

- **Pure HTML scraping only** (no API key, no official API)
- Regex-based parsing matching lsnd implementation exactly
- Support for channel IDs

#### API Endpoints

- `GET https://www.youtube.com/channel/{id}` - Channel page for metadata
- `GET https://www.youtube.com/channel/{id}/live` - Live stream page

#### Implementation Notes

Uses pure regex pattern matching to extract data from HTML, matching the lsnd implementation exactly:

```rust
pub struct YouTubeProvider {
    client: reqwest::Client,
}

impl YouTubeProvider {
    async fn fetch_stream(&self, channel_id: &str) -> Result<StreamInfo, ProviderError> {
        // Fetch channel page for metadata (name, avatar)
        let channel_url = format!("https://www.youtube.com/channel/{}", channel_id);
        let channel_html = self.client.get(&channel_url).send().await?.text().await?;
        
        let (name, avatar) = self.parse_channel_metadata(&channel_html)?;
        
        // Fetch live page to check live status
        let live_url = format!("https://www.youtube.com/channel/{}/live", channel_id);
        let live_html = self.client.get(&live_url).send().await?.text().await?;
        
        let live_info = self.parse_live_status(&live_html)?;
        
        Ok(StreamInfo {
            platform: "youtube".to_string(),
            user_id: channel_id.to_string(),
            display_name: name,
            avatar_url: Some(avatar),
            is_live: live_info.is_live,
            title: live_info.title,
            viewer_count: live_info.viewers,
            last_updated: Utc::now(),
        })
    }
    
    fn parse_channel_metadata(&self, html: &str) -> Result<(String, String), ProviderError> {
        // Use regex patterns matching lsnd/scrapers/youtube.js
        let name_regex = Regex::new(r#""name"\s*:\s*"([^"]+)""#)?;
        let avatar_regex = Regex::new(r#""avatar"\s*:\s*\{\s*"thumbnails"\s*:\s*\[\s*\{\s*"url"\s*:\s*"([^"]+)""#)?;
        
        let name = name_regex.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or(ProviderError::ParseError("Could not find channel name"))?;
            
        let avatar = avatar_regex.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or(ProviderError::ParseError("Could not find avatar"))?;
            
        Ok((name, avatar))
    }
    
    fn parse_live_status(&self, html: &str) -> Result<LiveInfo, ProviderError> {
        // Check if actually streaming using regex patterns
        let is_live = html.contains("isLiveContent\":true");
        
        // Extract title and viewer count if live
        // ... regex patterns matching lsnd implementation
        
        Ok(LiveInfo { is_live, title, viewers })
    }
}
```

#### Notes

- **No official API support** - only HTML scraping per user requirement
- **No discovery support** - YouTube doesn't expose public discovery endpoints without API
- Parsing patterns match lsnd/scrapers/youtube.js exactly

---

### 3. Kick Provider

**Crate**: `stream-aggregator-provider-kick`

#### Features

- **JA3/JA4 TLS fingerprint spoofing** via `wreq` (required for Cloudflare bypass)
- Browser emulation presets via `wreq-util`
- XSRF token handling
- REST API v2

#### API Endpoint

- `GET /api/v2/channels/{username}` - Returns channel and livestream info

#### Why wreq is Required

Kick uses Cloudflare's anti-bot protection which detects non-browser TLS fingerprints.
Standard HTTP clients like `reqwest` will be blocked. We use [`wreq`](https://github.com/0x676e67/wreq),
a fork of reqwest with full JA3/JA4/HTTP2 fingerprint emulation capabilities.

#### Response Structure

```json
{
  "user": {
    "username": "string",
    "profile_pic": "string"
  },
  "livestream": {
    "is_live": boolean,
    "session_title": "string",
    "viewer_count": number
  }
}
```

#### Implementation Notes

```rust
use wreq::Client;
use wreq_util::Emulation;

pub struct KickProvider {
    /// wreq client with browser TLS fingerprint emulation
    client: Client,
    /// Cached XSRF token
    xsrf_token: Arc<RwLock<Option<String>>>,
}

impl KickProvider {
    pub fn new() -> Result<Self, ProviderError> {
        // Create wreq client with Chrome browser emulation
        // This spoofs the TLS fingerprint to match Chrome exactly
        let client = Client::builder()
            .emulation(Emulation::Chrome131)  // Emulate Chrome 131
            .cookie_store(true)
            .build()?;
            
        Ok(Self {
            client,
            xsrf_token: Arc::new(RwLock::new(None)),
        })
    }
    
    async fn fetch_channel(&self, username: &str) -> Result<KickChannel, ProviderError> {
        // First, get XSRF token if needed
        self.ensure_xsrf_token().await?;
        
        let url = format!("https://kick.com/api/v2/channels/{}", username);
        
        let response = self.client
            .get(&url)
            .header("X-XSRF-TOKEN", self.get_xsrf_token().await?)
            .send()
            .await?;
            
        if response.status() == 403 {
            return Err(ProviderError::RateLimited {
                retry_after: Some(Duration::from_secs(60)),
            });
        }
        
        response.json().await.map_err(Into::into)
    }
    
    async fn ensure_xsrf_token(&self) -> Result<(), ProviderError> {
        let mut token = self.xsrf_token.write().await;
        if token.is_some() {
            return Ok(());
        }
        
        // Visit kick.com to get XSRF cookie
        let response = self.client.get("https://kick.com").send().await?;
        
        // Extract XSRF token from cookies
        if let Some(cookie) = response.cookies().find(|c| c.name() == "XSRF-TOKEN") {
            *token = Some(cookie.value().to_string());
        }
        
        Ok(())
    }
}
```

#### Available Browser Emulations

`wreq-util` provides these browser emulation presets:

| Emulation | Description |
|-----------|-------------|
| `Emulation::Chrome131` | Chrome 131 (recommended) |
| `Emulation::Chrome130` | Chrome 130 |
| `Emulation::Safari18` | Safari 18 |
| `Emulation::Safari26` | Safari 26 |
| `Emulation::Firefox133` | Firefox 133 |
| `Emulation::Edge131` | Edge 131 |

#### Rate Limits

```rust
fn rate_limit_config(&self) -> RateLimitConfig {
    RateLimitConfig {
        requests_per_window: 30,
        window_duration: Duration::from_secs(60),
        request_delay: Some(Duration::from_secs(2)),  // Critical!
        exponential_backoff: true,
    }
}
```

#### Build Requirements

`wreq` requires BoringSSL. Install build dependencies:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential cmake perl pkg-config libclang-dev musl-tools -y

# macOS
brew install cmake perl llvm
```

---

### 4. TikTok Provider

**Crate**: `stream-aggregator-provider-tiktok`

**Internal Dependency**: `tiktok-live` (custom crate in workspace)

#### Features

- **WebSocket-based connection** to TikTok's Webcast push service
- **Protobuf message decoding** for real-time events
- Real-time viewer counts, chat, gifts, and more
- Based on [TikTok-Live-Connector](https://github.com/zerodytrash/TikTok-Live-Connector/tree/ts-rewrite)

#### Architecture

Unlike other providers that poll HTTP APIs, TikTok uses a WebSocket-based protocol:

```
┌─────────────┐     HTTP      ┌─────────────┐
│   Provider  │ ────────────► │  TikTok.com │  (Room ID discovery)
└─────────────┘               └─────────────┘
       │
       │ WebSocket (Protobuf)
       ▼
┌─────────────────────────────────────────────┐
│         TikTok Webcast Push Service          │
│  (Real-time: chat, gifts, viewer counts)     │
└─────────────────────────────────────────────┘
```

#### Implementation Notes

The provider uses our internal `tiktok-live` crate:

```rust
use tiktok_live::{TikTokLiveClient, TikTokEvent};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct TikTokProvider {
    /// HTTP client for room discovery
    http_client: reqwest::Client,
    /// Active WebSocket connections per username
    connections: Arc<RwLock<HashMap<String, Arc<TikTokLiveClient>>>>,
    /// Cached stream info (updated via WebSocket events)
    cache: Arc<RwLock<HashMap<String, CachedStreamInfo>>>,
}

#[async_trait]
impl PlatformProvider for TikTokProvider {
    fn platform_id(&self) -> &'static str { "tiktok" }
    fn display_name(&self) -> &'static str { "TikTok" }
    fn base_url(&self) -> &'static str { "https://www.tiktok.com" }
    
    async fn fetch_stream(&self, username: &str) -> Result<StreamInfo, ProviderError> {
        // Check cache first (populated by WebSocket events)
        if let Some(cached) = self.get_cached(username).await {
            if !cached.is_stale() {
                return Ok(cached.info.clone());
            }
        }
        
        // Get or create WebSocket connection
        let client = self.get_or_create_connection(username).await?;
        let state = client.get_state().await;
        
        Ok(StreamInfo {
            platform: "tiktok".to_string(),
            user_id: username.to_string(),
            display_name: state.nickname.clone(),
            avatar_url: state.avatar_url.clone(),
            is_live: state.is_connected,
            title: state.room_title.clone(),
            viewer_count: Some(state.viewer_count),
            last_updated: Utc::now(),
            ..Default::default()
        })
    }
    
    fn rate_limit_config(&self) -> RateLimitConfig {
        // WebSocket connections are persistent
        // Rate limits only apply to initial room discovery
        RateLimitConfig {
            requests_per_window: 60,
            window_duration: Duration::from_secs(60),
            request_delay: None,
            exponential_backoff: true,
        }
    }
}

impl TikTokProvider {
    async fn get_or_create_connection(
        &self,
        username: &str
    ) -> Result<Arc<TikTokLiveClient>, ProviderError> {
        let mut connections = self.connections.write().await;
        
        // Return existing connection if still active
        if let Some(client) = connections.get(username) {
            if client.is_connected().await {
                return Ok(Arc::clone(client));
            }
        }
        
        // Create new WebSocket connection
        let client = TikTokLiveClient::connect(username).await?;
        let client = Arc::new(client);
        
        // Store connection
        connections.insert(username.to_string(), Arc::clone(&client));
        
        // Spawn background task to update cache from WebSocket events
        self.spawn_event_handler(Arc::clone(&client), username.to_string());
        
        Ok(client)
    }
    
    fn spawn_event_handler(&self, client: Arc<TikTokLiveClient>, username: String) {
        let cache = Arc::clone(&self.cache);
        
        tokio::spawn(async move {
            let mut rx = client.subscribe();
            
            while let Ok(event) = rx.recv().await {
                let mut cache = cache.write().await;
                let entry = cache.entry(username.clone()).or_default();
                
                match event {
                    TikTokEvent::ViewerCount { count } => {
                        entry.info.viewer_count = Some(count);
                    }
                    TikTokEvent::StreamEnd => {
                        entry.info.is_live = false;
                        break;
                    }
                    TikTokEvent::Connected { room_id: _ } => {
                        entry.info.is_live = true;
                    }
                    _ => {}
                }
                entry.updated_at = std::time::Instant::now();
            }
        });
    }
}
```

#### The tiktok-live Crate

Our internal `tiktok-live` crate (in `crates/tiktok-live/`) handles the WebSocket protocol:

**Key Features:**

- Room ID discovery from TikTok page HTML
- WebSocket connection to Webcast push service
- Protobuf message decoding (chat, gifts, likes, viewer counts)
- Automatic reconnection handling
- Event broadcasting via `tokio::sync::broadcast`

**Protobuf Messages Supported:**

- `WebcastChatMessage` - Chat comments
- `WebcastGiftMessage` - Gift events
- `WebcastLikeMessage` - Like events
- `WebcastMemberMessage` - User joins
- `WebcastRoomUserSeqMessage` - Viewer count updates
- `WebcastControlMessage` - Stream end notifications

```rust
// Event types from tiktok-live crate
pub enum TikTokEvent {
    Connected { room_id: String },
    Disconnected,
    Chat { user: User, comment: String },
    Gift { user: User, gift_id: u32, count: u32, is_streak_end: bool },
    Like { user: User, count: u32 },
    Member { user: User },  // User joined stream
    ViewerCount { count: u64 },
    StreamEnd,
}
```

#### Build Note

The `tiktok-live` crate requires `prost-build` to compile `.proto` files:

```toml
[build-dependencies]
prost-build = "0.12"
```

---

### 5. DLive Provider

**Crate**: `stream-aggregator-provider-dlive`

#### Features

- GraphQL API
- Simple authentication-free access

#### API Endpoint

- `POST https://graphigo.prd.dlive.tv/` - GraphQL endpoint

#### Implementation Notes

```rust
pub struct DLiveProvider {
    client: reqwest::Client,
}

impl DLiveProvider {
    async fn fetch_stream(&self, username: &str) -> Result<StreamInfo, ProviderError> {
        let query = r#"
            query($name: String!) {
                userByDisplayName(displayname: $name) {
                    displayname
                    avatar
                    livestream {
                        title
                        watchingCount
                        view
                    }
                }
            }
        "#;
        
        let response = self.client
            .post("https://graphigo.prd.dlive.tv/")
            .json(&serde_json::json!({
                "query": query,
                "variables": { "name": username }
            }))
            .send()
            .await?;
            
        let data: DLiveResponse = response.json().await?;
        let user = data.data.user_by_display_name
            .ok_or(ProviderError::UserNotFound)?;
            
        Ok(StreamInfo {
            platform: "dlive".to_string(),
            user_id: username.to_string(),
            display_name: user.displayname,
            avatar_url: Some(user.avatar),
            is_live: user.livestream.is_some(),
            viewer_count: user.livestream.as_ref().map(|l| l.watching_count),
            title: user.livestream.as_ref().map(|l| l.title.clone()),
            // ...
        })
    }
}
```

---

### 6. Trovo Provider

**Crate**: `stream-aggregator-provider-trovo`

#### Features

- Official API with Client ID authentication
- Two-step lookup (username -> channel_id -> channel data)
- Rate limit: 120 requests/minute

#### API Endpoints

1. `POST /openplatform/getusers` - Get channel_id from username
   - Body: `{"user": ["username"]}`
   - Returns: `{"users": [{"channel_id": "...", "username": "...", ...}]}`

2. `POST /openplatform/channels/id` - Get channel data by channel_id
   - Body: `{"channel_id": "..."}`
   - Returns: Channel info with `is_live`, `live_title`, `current_viewers`, etc.

#### Implementation Notes

```rust
pub struct TrovoProvider {
    client: reqwest::Client,
    client_id: String,
    /// Cache channel IDs to avoid redundant lookups
    channel_id_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl TrovoProvider {
    async fn get_channel_id(&self, username: &str) -> Result<String, ProviderError> {
        // Check cache first
        if let Some(id) = self.channel_id_cache.read().await.get(username) {
            return Ok(id.clone());
        }
        
        let response = self.client
            .post("https://open-api.trovo.live/openplatform/getusers")
            .header("Client-ID", &self.client_id)
            .json(&serde_json::json!({ "user": [username] }))
            .send()
            .await?;
            
        let data: TrovoUsersResponse = response.json().await?;
        let channel_id = data.users.first()
            .ok_or(ProviderError::UserNotFound)?
            .channel_id.clone();
            
        // Cache for future use
        self.channel_id_cache.write().await.insert(username.to_string(), channel_id.clone());
        
        Ok(channel_id)
    }
    
    async fn fetch_channel_by_id(&self, channel_id: &str) -> Result<ChannelData, ProviderError> {
        let response = self.client
            .post("https://open-api.trovo.live/openplatform/channels/id")
            .header("Client-ID", &self.client_id)
            .json(&serde_json::json!({ "channel_id": channel_id }))
            .send()
            .await?;
            
        response.json().await.map_err(Into::into)
    }
}
```

---

### 7. Guac Provider

**Crate**: `stream-aggregator-provider-guac`

#### API Endpoint

- `GET https://api.guac.tv/v2/stream/{id}` - Get stream info

#### Response Structure

```json
{
  "data": {
    "live": boolean,
    "user": {
      "username": "string",
      "avatar": "string"
    },
    "type": "string",
    "viewers": number,
    "title": "string",
    "banner": "string"
  }
}
```

#### Implementation Notes

```rust
impl GuacProvider {
    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let url = format!("https://api.guac.tv/v2/stream/{}", user_id);
        let response: GuacResponse = self.client.get(&url).send().await?.json().await?;
        
        // Response has nested data.data structure
        let stream = response.data;
        
        Ok(StreamInfo {
            platform: "guac".to_string(),
            user_id: user_id.to_string(),
            display_name: stream.user.username,
            avatar_url: stream.user.avatar,
            is_live: stream.live,
            title: stream.title,
            viewer_count: stream.viewers,
            thumbnail_url: stream.banner,
            last_updated: Utc::now(),
        })
    }
}
```

### 8. AngelThump Provider

**Crate**: `stream-aggregator-provider-angelthump`

#### API Endpoints

- `GET https://api.angelthump.com/v3/users/?username={username}` - Get user info (returns array)
- `GET https://api.angelthump.com/v3/streams/?username={username}` - Get stream info (returns array)

#### Implementation Notes

```rust
impl AngelThumpProvider {
    async fn fetch_user(&self, username: &str) -> Result<AngelThumpUser, ProviderError> {
        let url = format!("https://api.angelthump.com/v3/users/?username={}", username);
        
        // API returns array, take first element
        let users: Vec<AngelThumpUser> = self.client.get(&url).send().await?.json().await?;
        users.into_iter().next()
            .ok_or_else(|| ProviderError::StreamerNotFound(username.to_string()))
    }
    
    async fn fetch_stream_info(&self, username: &str) -> Result<Option<AngelThumpStream>, ProviderError> {
        let url = format!("https://api.angelthump.com/v3/streams/?username={}", username);
        
        // API returns array, empty if offline
        let streams: Vec<AngelThumpStream> = self.client.get(&url).send().await?.json().await?;
        Ok(streams.into_iter().next())
    }
}
```

#### Notes

Both endpoints use query parameters and return arrays. Stream endpoint returns empty array if offline.

### 9. RobotStreamer Provider

**Crate**: `stream-aggregator-provider-robotstreamer`

#### API Endpoint

- `GET http://api.robotstreamer.com:8080/v1/get_robot/{id}` - Get robot info (returns array)

#### Response Structure

```json
[{
  "status": "live" | "offline",
  "robot_name": "string",
  "viewers": number
}]
```

#### Implementation Notes

```rust
impl RobotStreamerProvider {
    async fn fetch_robot(&self, robot_id: &str) -> Result<RobotStreamerRobot, ProviderError> {
        // Note: HTTP only (not HTTPS) on port 8080
        let url = format!("http://api.robotstreamer.com:8080/v1/get_robot/{}", robot_id);
        
        // API returns array, take first element
        let robots: Vec<RobotStreamerRobot> = self.client.get(&url).send().await?.json().await?;
        robots.into_iter().next()
            .ok_or_else(|| ProviderError::StreamerNotFound(robot_id.to_string()))
    }
    
    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let robot = self.fetch_robot(user_id).await?;
        
        // Status field indicates if live
        let is_live = robot.status.as_ref()
            .map(|s| s.to_lowercase() == "live" || s == "1")
            .unwrap_or(false);
            
        Ok(StreamInfo {
            platform: "robotstreamer".to_string(),
            user_id: user_id.to_string(),
            display_name: robot.robot_name.unwrap_or(user_id.to_string()),
            is_live,
            viewer_count: robot.viewers,
            last_updated: Utc::now(),
        })
    }
}
```

#### Notes

- Uses **HTTP only** (not HTTPS) on port 8080
- API returns an array with single element
- Status field checked for "live" string or "1"

> **Note:** Brime has been removed as the platform is no longer active.

---

## Provider Registration

Providers are registered at startup via a registry:

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn PlatformProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }
    
    pub fn register(&mut self, provider: impl PlatformProvider) {
        let id = provider.platform_id().to_string();
        self.providers.insert(id, Arc::new(provider));
    }
    
    pub fn get(&self, platform_id: &str) -> Option<Arc<dyn PlatformProvider>> {
        self.providers.get(platform_id).cloned()
    }
    
    pub fn all(&self) -> Vec<Arc<dyn PlatformProvider>> {
        self.providers.values().cloned().collect()
    }
    
    pub fn platforms(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}

// Builder pattern for easy registration
impl ProviderRegistry {
    pub fn builder() -> ProviderRegistryBuilder {
        ProviderRegistryBuilder::new()
    }
}

pub struct ProviderRegistryBuilder {
    registry: ProviderRegistry,
}

impl ProviderRegistryBuilder {
    pub fn with_twitch(mut self, config: TwitchConfig) -> Self {
        self.registry.register(TwitchProvider::new(config));
        self
    }
    
    pub fn with_youtube(mut self) -> Self {
        self.registry.register(YouTubeProvider::new());
        self
    }
    
    // ... other providers
    
    pub fn build(self) -> ProviderRegistry {
        self.registry
    }
}

// Usage
let registry = ProviderRegistry::builder()
    .with_twitch(twitch_config)
    .with_youtube()
    .with_kick()
    .build();
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },
    
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Discovery not supported for this platform")]
    DiscoveryNotSupported,
    
    #[error("Platform API error: {message}")]
    ApiError { code: Option<i32>, message: String },
    
    #[error("Provider not initialized")]
    NotInitialized,
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    
    #[tokio::test]
    async fn test_twitch_fetch_stream() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/helix/streams"))
            .respond_with(ResponseTemplate::new(200).set_body_json(/* ... */))
            .mount(&mock_server)
            .await;
            
        let provider = TwitchProvider::new_with_base_url(
            config,
            mock_server.uri(),
        );
        
        let result = provider.fetch_stream("test_user").await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Run manually or in CI with credentials
async fn test_twitch_real_api() {
    let config = TwitchConfig::from_env().unwrap();
    let provider = TwitchProvider::new(config);
    
    let result = provider.fetch_stream("twitchdev").await;
    assert!(result.is_ok());
}
```

---

## Adding a New Provider

1. Create a new crate: `stream-aggregator-provider-{name}`
2. Implement `PlatformProvider` trait
3. Add configuration struct
4. Add to `ProviderRegistryBuilder`
5. Add feature flag to main crate
6. Write tests
7. Document API requirements and rate limits

Template:

```rust
// stream-aggregator-provider-newplatform/src/lib.rs

use stream_aggregator_core::*;
use async_trait::async_trait;
use wreq::Client;
use wreq_util::Emulation;

pub struct NewPlatformConfig {
    pub api_key: Option<String>,
    /// Enable browser emulation (for anti-bot protected APIs)
    pub browser_emulation: bool,
}

pub struct NewPlatformProvider {
    client: Client,
    config: NewPlatformConfig,
}

impl NewPlatformProvider {
    pub fn new(config: NewPlatformConfig) -> Result<Self, ProviderError> {
        let client = if config.browser_emulation {
            Client::builder()
                .emulation(Emulation::Chrome131)
                .build()?
        } else {
            Client::new()
        };
        
        Ok(Self { client, config })
    }
}

#[async_trait]
impl PlatformProvider for NewPlatformProvider {
    fn platform_id(&self) -> &'static str { "newplatform" }
    fn display_name(&self) -> &'static str { "New Platform" }
    fn base_url(&self) -> &'static str { "https://newplatform.com" }
    
    async fn fetch_stream(&self, streamer_id: &str) -> Result<StreamInfo, ProviderError> {
        let url = format!("{}/api/users/{}", self.base_url(), streamer_id);
        let response = self.client.get(&url).send().await?;
        // ... parse response
        todo!()
    }
    
    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_window: 60,
            window_duration: Duration::from_secs(60),
            request_delay: None,
            exponential_backoff: true,
        }
    }
}
```

---

## HTTP Client Strategy

### Default: wreq for All Platforms

We use **wreq** as the HTTP client for **all platforms**. It's a fork of `reqwest` with:

- 100% API compatibility with reqwest
- JA3/JA4/HTTP2 TLS fingerprint emulation
- Browser emulation presets via `wreq-util`

| Platform | Protocol | Browser Emulation | Notes |
|----------|----------|-------------------|-------|
| Twitch | HTTPS | Optional | Standard OAuth2 API |
| YouTube | HTTPS | Optional | HTML scraping / Data API |
| **Kick** | HTTPS | **Required** | Cloudflare anti-bot protection |
| TikTok | HTTPS + WebSocket | Optional | Room discovery + Protobuf WebSocket |
| DLive | HTTPS | Optional | GraphQL API |
| Trovo | HTTPS | Optional | Standard REST API |
| Guac | HTTPS | Optional | Standard REST API |
| AngelThump | HTTPS | Optional | Standard REST API |
| RobotStreamer | HTTP | N/A | HTTP only (not HTTPS) |

### Why wreq for Everything?

1. **Future-proofing**: Any platform could add anti-bot protection at any time
2. **Consistency**: Single HTTP client across all providers, simpler codebase
3. **No downside**: Same ergonomic API as reqwest
4. **Required anyway**: Kick absolutely requires TLS fingerprinting

### Browser Emulation

For platforms with anti-bot protection (currently Kick), enable browser emulation:

```rust
use wreq::Client;
use wreq_util::Emulation;

// For Kick - Chrome emulation required
let kick_client = Client::builder()
    .emulation(Emulation::Chrome131)
    .build()?;

// For other platforms - no emulation needed, but doesn't hurt
let twitch_client = Client::builder()
    .emulation(Emulation::Chrome131)  // Optional but safe
    .build()?;

// Or just use default client for simple APIs
let simple_client = Client::new();
```

### WebSocket Connections

TikTok is the only platform using WebSocket (via `tokio-tungstenite`):

- Real-time push service for live stream events
- Protobuf-encoded messages
- Sub-second viewer count updates

All other platforms use HTTP polling at configurable intervals.
