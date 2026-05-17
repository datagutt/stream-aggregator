# REST API Design

This document specifies the REST API for StreamAggregator.

## Overview

- **Base URL**: `/api/v1` (configurable)
- **Content-Type**: `application/json`
- **Authentication**: Optional API key via `X-API-Key` header or `?api_key=` query param
- **Rate Limiting**: Configurable per-IP and per-API-key limits

---

## Endpoints

### Streams

#### GET /streams

Retrieve all streams with optional filtering and pagination.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `platform[]` | string | Filter by platform. Repeat for multi (`?platform[]=twitch&platform[]=youtube`) or use indexed form (`?platform[0]=twitch&platform[1]=youtube`). A single value also works (`?platform[]=twitch`). |
| `live` | boolean | Filter by live status (`true` or `false`) |
| `group` | string | Filter by group/team name |
| `labels[key]` | string | Filter by labels (repeat parameter for multiple: `?labels[country]=no&labels[team]=vikings`) |
| `search` | string | Search in display name and title |
| `min_viewers` | integer | Minimum viewer count |
| `max_viewers` | integer | Maximum viewer count |
| `category[]` | string | Filter by game/category. Multi-value via `?category[]=Fortnite&category[]=Valorant`. |
| `language[]` | string | Filter by stream language. Multi-value via `?language[]=no&language[]=sv`. |
| `tag[]` | string | Filter by stream tags. A stream matches when it contains at least one of the requested tags. |
| `sort` | string | Sort field: `viewers`, `name`, `platform`, `fetched` (when we last polled, alias `updated`), `live` (when streamer was last observed live). Default: `viewers`. |
| `order` | string | Sort order: `asc` or `desc` (default: `desc`) |
| `page` | integer | Page number (default: 1) |
| `per_page` | integer | Items per page (default: 50, max: 100) |

**Response:**

```json
{
  "data": [
    {
      "id": "a1b2c3d4e5f6...",
      "platform": "twitch",
      "user_id": "ninja",
      "display_name": "Ninja",
      "avatar_url": "https://...",
      "is_live": true,
      "title": "Playing Fortnite with friends!",
      "viewer_count": 45000,
      "thumbnail_url": "https://...",
      "category": "Fortnite",
      "tags": ["English", "FPS"],
      "language": "en",
      "started_at": "2024-01-15T14:30:00Z",
      "last_fetched_at": "2024-01-15T15:00:00Z",
      "last_live_at": "2024-01-15T14:30:00Z",
      "metadata": {
        "group": "fps-pros",
        "priority": 1,
        "labels": {
          "country": "us"
        }
      }
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 50,
    "total": 150,
    "total_pages": 3
  }
}
```

**Example Requests:**

```bash
# Get all live streams
curl "http://localhost:8080/api/v1/streams?live=true"

# Get Twitch streams sorted by viewers
curl "http://localhost:8080/api/v1/streams?platform[]=twitch&sort=viewers&order=desc"

# Multi-platform + multi-language (Scandinavian directory)
curl "http://localhost:8080/api/v1/streams?platform[]=twitch&platform[]=youtube&language[]=no&language[]=sv"

# Search for streams
curl "http://localhost:8080/api/v1/streams?search=fortnite&live=true"

# Filter by labels
curl "http://localhost:8080/api/v1/streams?labels[country]=no&labels[team]=vikings"
```

---

#### GET /streams/:id

Get a single stream by ID.

**Response:**

```json
{
  "data": {
    "id": "a1b2c3d4e5f6...",
    "platform": "twitch",
    "user_id": "ninja",
    "display_name": "Ninja",
    // ... full stream object
  }
}
```

**Error Response (404):**

```json
{
  "error": {
    "code": "STREAM_NOT_FOUND",
    "message": "Stream with ID 'xyz' not found"
  }
}
```

---

### Streamers (Tracked)

#### GET /streamers

Get all tracked streamers.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `platform` | string | Filter by platform |
| `group` | string | Filter by group |
| `source` | string | Filter by source: `manual` or `discovery` |
| `labels[key]` | string | Filter by labels (repeat parameter for multiple: `?labels[country]=no&labels[team]=vikings`) |

**Response:**

```json
{
  "data": [
    {
      "platform": "twitch",
      "user_id": "ninja",
      "custom_name": null,
      "group": "fps-pros",
      "priority": 1,
      "labels": {
        "country": "us"
      },
      "source": "manual",
      "discovery_rule_id": null,
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "total": 100
}
```

---

#### POST /streamers

Add a new streamer to track.

**Request Body:**

```json
{
  "platform": "twitch",
  "username": "shroud",
  "custom_name": "Shroud Gaming",
  "group": "fps-pros",
  "priority": 2,
  "labels": {
    "country": "ca",
    "team": "sentinels"
  }
}
```

```json
{
  "platform": "twitch",
  "user_id": "37402112",
  "custom_name": "Shroud Gaming",
  "group": "fps-pros",
  "priority": 2,
  "labels": {
    "country": "ca",
    "team": "sentinels"
  }
}
```

**Response (201 Created):**

```json
{
  "data": {
    "platform": "twitch",
    "user_id": "37402112",
    "custom_name": "Shroud Gaming",
    "group": "fps-pros",
    "priority": 2,
    "labels": {
      "country": "ca",
      "team": "sentinels"
    },
    "source": "manual",
    "discovery_rule_id": null,
    "created_at": "2024-01-15T12:00:00Z"
  }
}
```

**Error Response (409 Conflict):**

```json
{
  "error": {
    "code": "STREAMER_EXISTS",
    "message": "Streamer 'twitch/shroud' is already being tracked"
  }
}
```

---

#### PUT /streamers/:platform/:user_id

Update a tracked streamer.

**Request Body:**

```json
{
  "custom_name": "Updated Name",
  "group": "new-group",
  "priority": 5,
  "labels": {
    "country": "us"
  }
}
```

**Response:**

```json
{
  "data": {
    "platform": "twitch",
    "user_id": "shroud",
    "custom_name": "Updated Name",
    // ... updated streamer
  }
}
```

---

#### DELETE /streamers/:platform/:user_id

Remove a tracked streamer.

**Response (204 No Content):** Empty body on success.

**Error Response (404):**

```json
{
  "error": {
    "code": "STREAMER_NOT_FOUND",
    "message": "Streamer 'twitch/unknown' is not being tracked"
  }
}
```

---

#### POST /streamers/bulk

Bulk add streamers.

**Request Body:**

```json
{
  "streamers": [
    { "platform": "twitch", "user_id": "ninja" },
    { "platform": "twitch", "user_id": "shroud" },
    { "platform": "youtube", "user_id": "UC-lHJZR3Gqxm24_Vd_AJ5Yw" }
  ],
  "defaults": {
    "group": "imported",
    "labels": { "batch": "2024-01" }
  }
}
```

**Response:**

```json
{
  "data": {
    "added": 2,
    "skipped": 1,
    "errors": [
      {
        "platform": "youtube",
        "user_id": "UC-lHJZR3Gqxm24_Vd_AJ5Yw",
        "error": "Already exists"
      }
    ]
  }
}
```

---

### Discovery Rules

#### GET /discovery/rules

Get all discovery rules.

**Response:**

```json
{
  "data": [
    {
      "id": "norwegian-twitch",
      "name": "Norwegian Twitch Streamers",
      "platform": "twitch",
      "enabled": true,
      "filters": {
        "languages": ["no"],
        "min_viewers": 5
      },
      "interval_secs": 600,
      "apply_labels": { "country": "no" },
      "apply_group": "norwegian",
      "last_run_at": "2024-01-15T15:00:00Z",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

---

#### POST /discovery/rules

Create a new discovery rule.

**Request Body:**

```json
{
  "id": "valorant-pros",
  "name": "Valorant Pro Streamers",
  "platform": "twitch",
  "enabled": true,
  "filters": {
    "categories": ["Valorant"],
    "tags": ["Esports", "Competitive"],
    "min_viewers": 1000,
    "limit": 25
  },
  "interval_secs": 300,
  "apply_labels": { "game": "valorant", "tier": "pro" },
  "apply_group": "esports"
}
```

**Response (201 Created):**

```json
{
  "data": {
    "id": "valorant-pros",
    // ... full rule object
  }
}
```

---

#### PUT /discovery/rules/:id

Update a discovery rule.

---

#### DELETE /discovery/rules/:id

Delete a discovery rule.

---

#### POST /discovery/rules/:id/run

Manually trigger a discovery rule.

**Response:**

```json
{
  "data": {
    "rule_id": "valorant-pros",
    "discovered": 18,
    "new_streamers": 5,
    "duration_ms": 1250
  }
}
```

---

### Communities

Communities are brandable directory tenants. Each one defines a brand (name,
accent, theme, logo) and a filter recipe (`CommunityFilter`) that selects a
slice of the global stream pool. Hostnames are tracked separately in
`community_domains` and exposed back via the `domains` array on the
`Community` payload.

Reads are public. Writes (POST, PUT, DELETE) require the standard API key
when authentication is enabled.

#### GET /communities

```json
{
  "data": [
    {
      "slug": "livestreamnorge",
      "name": "LiveStreamNorge",
      "tagline": "Norske strømmere live nå",
      "accent": "0.58 0.20 250",
      "accent_contrast": null,
      "logo_url": null,
      "default_theme": "dark",
      "domains": ["livestreamnorge.example.com", "lsn.example.com"],
      "filter": {
        "platforms": [],
        "languages": ["no"],
        "categories": [],
        "tags": [],
        "groups": [],
        "labels": {},
        "min_viewers": null
      },
      "about_md": null,
      "created_at": "2026-05-17T14:00:00Z",
      "updated_at": "2026-05-17T14:00:00Z"
    }
  ]
}
```

#### GET /communities/{slug}

Returns one community or `404`.

#### GET /communities/by-domain/{host}

Resolves a hostname to its owning community (used by the Next.js middleware on
every request, cached client-side for 60s). `404` when no mapping exists.

#### POST /communities

Create. Requires authentication when enabled.

Request body matches the `Community` response shape minus `created_at` and
`updated_at` (server stamps them). `domains` defaults to `[]`. A `409`-style
error returns when a slug already exists or a domain is claimed by another
community.

#### PUT /communities/{slug}

Replace. The `slug` in the path is authoritative (any `slug` field in the body
is ignored). The `domains` array replaces the community's full domain set
atomically. Returns the updated community.

#### DELETE /communities/{slug}

Remove. `204 No Content` on success, `404` when missing. The `community_domains`
foreign key cascades, so domain mappings are cleaned automatically.

---

### Platforms

#### GET /platforms

Get list of supported platforms and their status.

**Response:**

```json
{
  "data": [
    {
      "id": "twitch",
      "name": "Twitch",
      "enabled": true,
      "supports_discovery": true,
      "discovery_capabilities": {
        "tags": true,
        "categories": true,
        "languages": true,
        "viewer_count_filter": true
      },
      "health": "healthy",
      "tracked_count": 50,
      "live_count": 12
    },
    {
      "id": "youtube",
      "name": "YouTube",
      "enabled": true,
      "supports_discovery": false,
      "health": "healthy",
      "tracked_count": 10,
      "live_count": 2
    },
    {
      "id": "kick",
      "name": "Kick",
      "enabled": true,
      "supports_discovery": true,
      "health": "degraded",
      "health_message": "High latency detected",
      "tracked_count": 5,
      "live_count": 1
    }
  ]
}
```

---

#### GET /platforms/:id/categories

Get available categories for a platform (if supported).

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `search` | string | Search categories by name |
| `limit` | integer | Max results (default: 20) |

**Response:**

```json
{
  "data": [
    { "id": "509658", "name": "Just Chatting" },
    { "id": "33214", "name": "Fortnite" },
    { "id": "516575", "name": "Valorant" }
  ]
}
```

---

#### GET /platforms/:id/tags

Get available tags for a platform (if supported).

**Response:**

```json
{
  "data": [
    { "id": "tag123", "name": "English" },
    { "id": "tag456", "name": "Esports" }
  ]
}
```

---

### Groups

#### GET /groups

Get all unique groups with stream counts.

**Response:**

```json
{
  "data": [
    { "name": "fps-pros", "total": 25, "live": 8 },
    { "name": "norwegian", "total": 100, "live": 15 },
    { "name": "entertainers", "total": 30, "live": 5 }
  ]
}
```

---

### Statistics

#### GET /stats

Get overall service statistics.

**Response:**

```json
{
  "data": {
    "uptime_secs": 86400,
    "total_streamers": 185,
    "total_live": 45,
    "by_platform": {
      "twitch": { "tracked": 150, "live": 40 },
      "youtube": { "tracked": 20, "live": 3 },
      "kick": { "tracked": 15, "live": 2 }
    },
    "scraping": {
      "total_scrapes": 50000,
      "failed_scrapes": 150,
      "avg_scrape_duration_ms": 250
    },
    "discovery": {
      "rules_count": 5,
      "last_discovery_run": "2024-01-15T15:00:00Z",
      "streamers_discovered_total": 85
    }
  }
}
```

---

### Health

#### GET /health

Basic health check.

**Response (200 OK):**

```json
{
  "status": "healthy",
  "version": "1.0.0"
}
```

---

#### GET /health/ready

Kubernetes readiness probe.

**Response (200 OK when ready, 503 when not):**

```json
{
  "status": "ready",
  "checks": {
    "storage": "ok",
    "providers": "ok"
  }
}
```

---

#### GET /health/live

Kubernetes liveness probe.

**Response (200 OK when alive):**

```json
{
  "status": "alive"
}
```

---

#### GET /health/detailed

Detailed health status (requires authentication if enabled).

**Response:**

```json
{
  "status": "degraded",
  "version": "1.0.0",
  "uptime_secs": 86400,
  "storage": {
    "status": "healthy",
    "type": "postgres",
    "latency_ms": 5
  },
  "providers": [
    {
      "id": "twitch",
      "status": "healthy",
      "last_scrape": "2024-01-15T15:05:00Z",
      "avg_latency_ms": 150
    },
    {
      "id": "kick",
      "status": "degraded",
      "message": "Rate limited",
      "last_scrape": "2024-01-15T15:00:00Z"
    }
  ]
}
```

---

### WebSocket API

#### WS /ws/streams

Real-time stream updates via WebSocket.

**Connection:**

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/ws/streams');

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log(message);
};
```

**Message Types:**

```json
// Stream went live
{
  "type": "stream_online",
  "data": {
    "id": "abc123",
    "platform": "twitch",
    "user_id": "ninja",
    "display_name": "Ninja",
    "title": "Playing Fortnite!",
    "viewer_count": 0,
    "started_at": "2024-01-15T15:00:00Z"
  }
}

// Stream went offline
{
  "type": "stream_offline",
  "data": {
    "id": "abc123",
    "platform": "twitch",
    "user_id": "ninja"
  }
}

// Stream updated (title, viewers, etc.)
{
  "type": "stream_updated",
  "data": {
    "id": "abc123",
    "changes": {
      "viewer_count": { "old": 1000, "new": 1500 },
      "title": { "old": "Old title", "new": "New title" }
    }
  }
}

// New streamer discovered
{
  "type": "streamer_discovered",
  "data": {
    "platform": "twitch",
    "user_id": "new_streamer",
    "rule_id": "norwegian-twitch"
  }
}
```

**Subscription Filters:**

Send a message to filter which updates you receive:

```json
{
  "type": "subscribe",
  "filters": {
    "platforms": ["twitch", "youtube"],
    "groups": ["norwegian"],
    "events": ["stream_online", "stream_offline"]
  }
}
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable message",
    "details": {}  // Optional additional details
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Invalid request parameters |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `CONFLICT` | 409 | Resource already exists |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `PROVIDER_ERROR` | 502 | Platform API error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

---

## OpenAPI Specification

The API provides an OpenAPI 3.0 specification at `/api/v1/docs/openapi.json` and interactive documentation at `/api/v1/docs` (when enabled).

---

## Implementation Notes

### Axum Router Structure

```rust
use axum::{
    Router,
    routing::{get, post, put, delete},
    middleware,
};

pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        // Streams
        .route("/streams", get(handlers::get_streams))
        .route("/streams/:id", get(handlers::get_stream))
        
        // Streamers
        .route("/streamers", get(handlers::get_streamers))
        .route("/streamers", post(handlers::add_streamer))
        .route("/streamers/bulk", post(handlers::bulk_add_streamers))
        .route("/streamers/:platform/:user_id", put(handlers::update_streamer))
        .route("/streamers/:platform/:user_id", delete(handlers::delete_streamer))
        
        // Discovery
        .route("/discovery/rules", get(handlers::get_discovery_rules))
        .route("/discovery/rules", post(handlers::create_discovery_rule))
        .route("/discovery/rules/:id", put(handlers::update_discovery_rule))
        .route("/discovery/rules/:id", delete(handlers::delete_discovery_rule))
        .route("/discovery/rules/:id/run", post(handlers::run_discovery_rule))
        
        // Platforms
        .route("/platforms", get(handlers::get_platforms))
        .route("/platforms/:id/categories", get(handlers::get_categories))
        .route("/platforms/:id/tags", get(handlers::get_tags))
        
        // Groups
        .route("/groups", get(handlers::get_groups))
        
        // Stats
        .route("/stats", get(handlers::get_stats))
        
        // Health
        .route("/health", get(handlers::health))
        .route("/health/ready", get(handlers::readiness))
        .route("/health/live", get(handlers::liveness))
        .route("/health/detailed", get(handlers::detailed_health))
        
        // WebSocket
        .route("/ws/streams", get(handlers::ws_streams))
        
        // Apply middleware
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));
    
    Router::new()
        .nest("/api/v1", api_routes)
        .with_state(state)
}
```
