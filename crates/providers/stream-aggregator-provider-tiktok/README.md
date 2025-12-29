# TikTok Provider

TikTok Live stream provider for StreamAggregator.

## Architecture

This provider uses a **Node.js bridge** to access TikTok Live data via the
[tiktok-live-connector](https://github.com/zerodytrash/TikTok-Live-Connector/tree/ts-rewrite) library.

```
┌─────────────────────┐     HTTP      ┌─────────────────────┐
│   Rust Provider     │◄─────────────►│   Node.js Bridge    │
│   (TikTokProvider)  │   localhost   │   (index.js)        │
└─────────────────────┘               └─────────────────────┘
                                              │
                                              ▼
                                      ┌─────────────────────┐
                                      │   TikTok Live API   │
                                      │   (via connector)   │
                                      └─────────────────────┘
```

## How it Works

1. When `TikTokProvider::new()` is called, it automatically spawns the Node.js bridge process
2. The bridge runs as an HTTP server on `127.0.0.1:3456` (configurable)
3. The Rust provider communicates with the bridge via HTTP requests
4. When the provider is dropped, the bridge process is automatically terminated

## Configuration

```toml
[providers.tiktok]
enabled = true
# Optional: Custom bridge URL (default: http://127.0.0.1:3456)
bridge_url = "http://127.0.0.1:3456"
# Optional: Custom path to the nodejs-bridge directory
bridge_path = "/path/to/nodejs-bridge"
```

## Bridge API

The Node.js bridge exposes the following HTTP endpoints:

### GET /health

Health check endpoint.

```json
{
  "status": "ok",
  "uptime": 12345,
  "activeConnections": 0,
  "totalRequests": 100,
  "cacheSize": 10
}
```

### POST /room

Get room info for a single user.

Request:

```json
{
  "username": "tiktok_user"
}
```

Response:

```json
{
  "success": true,
  "data": {
    "live": true,
    "username": "tiktok_user",
    "display_name": "TikTok User",
    "avatar_url": "https://...",
    "thumbnail_url": "https://...",
    "viewer_count": 1234,
    "title": "Stream Title",
    "room_id": "7123456789",
    "stream_url": "https://...",
    "bio": "User bio",
    "create_time": 1234567890
  }
}
```

### POST /batch

Get room info for multiple users.

Request:

```json
{
  "usernames": ["user1", "user2", "user3"]
}
```

Response:

```json
{
  "success": true,
  "results": [
    { "username": "user1", "success": true, "data": { ... } },
    { "username": "user2", "success": true, "data": { ... } },
    { "username": "user3", "success": false, "error": "...", "error_code": "user_offline" }
  ],
  "stats": {
    "total": 3,
    "successful": 2,
    "failed": 1,
    "duration_ms": 1234
  }
}
```

## Error Codes

The bridge returns structured error codes:

- `user_not_found` - User does not exist
- `user_offline` - User exists but is not live
- `rate_limited` - Rate limit exceeded
- `invalid_response` - Invalid response from TikTok
- `timeout` - Request timeout
- `network_error` - Network connectivity issue
- `captcha_required` - TikTok requires captcha verification
- `unknown_error` - Unknown error

## Requirements

- Node.js 24+ LTS must be installed and available in PATH
- The `nodejs-bridge` directory must be accessible (auto-detected or configured via `bridge_path`)

## Development

To run the bridge standalone for testing:

```bash
cd nodejs-bridge
npm install
npm start
```

Environment variables:

- `TIKTOK_BRIDGE_PORT` - HTTP server port (default: 3456)
- `TIKTOK_MAX_CONCURRENT` - Max concurrent requests (default: 10)
- `TIKTOK_REQUEST_TIMEOUT` - Request timeout in ms (default: 30000)
- `TIKTOK_RESPONSE_CACHE_TTL` - Response cache TTL in ms (default: 30000)
