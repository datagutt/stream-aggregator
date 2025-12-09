# Migration Guide: Node.js to Rust

This document guides you through migrating from the original Node.js LSND implementation to the new Rust StreamAggregator.

## Overview

| Aspect | Node.js (LSND) | Rust (StreamAggregator) |
|--------|----------------|------------------------|
| Config format | JSON (nconf) | TOML |
| Streamers list | `people.json` | `streamers.toml` or API |
| Environment vars | Direct usage | `STREAM_AGG_` prefix |
| API endpoints | `/streams`, `/platforms`, `/teams` | `/api/v1/streams`, etc. |
| Discovery | Manual only | Manual + automatic |
| Storage | In-memory only | Memory, SQLite, PostgreSQL |

---

## Step 1: Convert Configuration

### Old: config.json

```json
{
  "http": {
    "port": "9080",
    "ssl": false,
    "ssl_port": "9443",
    "ssl_cert": "",
    "ssl_privkey": ""
  },
  "twitch": {
    "client_id": "your_client_id",
    "client_secret": "your_secret"
  },
  "trovo": {
    "client_id": "your_trovo_id"
  }
}
```

### New: config.toml

```toml
[server]
host = "0.0.0.0"
port = 9080

[server.tls]
enabled = false
cert_path = ""
key_path = ""

[providers.twitch]
enabled = true
client_id = "your_client_id"
client_secret = "your_secret"

[providers.trovo]
enabled = true
client_id = "your_trovo_id"

# Enable all other providers you need
[providers.youtube]
enabled = true

[providers.kick]
enabled = true

[providers.tiktok]
enabled = true

[providers.dlive]
enabled = true

[providers.guac]
enabled = true

[providers.angelthump]
enabled = true

[providers.robotstreamer]
enabled = true
```

---

## Step 2: Convert Streamers List

### Old: people.json

The old format supported both array and object formats:

```json
[
  {
    "platform": "twitch",
    "userId": "ninja",
    "customUsername": "Ninja",
    "featuredRank": "1",
    "team": "fortnite-pros"
  },
  {
    "platform": "youtube",
    "userId": "UC-lHJZR3Gqxm24_Vd_AJ5Yw"
  },
  {
    "platform": "kick",
    "userId": "xqc"
  }
]
```

### New: streamers.toml

```toml
[[streamers]]
platform = "twitch"
user_id = "ninja"
custom_name = "Ninja"
priority = 1
group = "fortnite-pros"

[[streamers]]
platform = "youtube"
user_id = "UC-lHJZR3Gqxm24_Vd_AJ5Yw"

[[streamers]]
platform = "kick"
user_id = "xqc"
```

### Conversion Script

Run this Node.js script to convert your `people.json`:

```javascript
// convert-people.js
const fs = require('fs');

const people = JSON.parse(fs.readFileSync('people.json', 'utf8'));

let toml = '# Converted from people.json\n\n';

for (const person of people) {
    toml += '[[streamers]]\n';
    toml += `platform = "${person.platform}"\n`;
    toml += `user_id = "${person.userId}"\n`;
    
    if (person.customUsername) {
        toml += `custom_name = "${person.customUsername}"\n`;
    }
    
    if (person.featuredRank) {
        toml += `priority = ${parseInt(person.featuredRank)}\n`;
    }
    
    if (person.team) {
        toml += `group = "${person.team}"\n`;
    }
    
    toml += '\n';
}

fs.writeFileSync('streamers.toml', toml);
console.log('Converted people.json to streamers.toml');
```

```bash
node convert-people.js
```

---

## Step 3: Environment Variables

### Old Format

```bash
export TWITCH_CLIENT_ID="your_id"
export TWITCH_CLIENT_SECRET="your_secret"
export TROVO_CLIENT_ID="your_trovo_id"
```

### New Format

```bash
export STREAM_AGG_PROVIDERS_TWITCH_CLIENT_ID="your_id"
export STREAM_AGG_PROVIDERS_TWITCH_CLIENT_SECRET="your_secret"
export STREAM_AGG_PROVIDERS_TROVO_CLIENT_ID="your_trovo_id"
export STREAM_AGG_SERVER_PORT="9080"
```

---

## Step 4: API Endpoint Changes

### Endpoint Mapping

| Old Endpoint | New Endpoint | Notes |
|--------------|--------------|-------|
| `GET /` | `GET /health` | Health check |
| `GET /streams` | `GET /api/v1/streams` | Stream data |
| `GET /platforms` | `GET /api/v1/platforms` | Platform list |
| `GET /teams` | `GET /api/v1/groups` | Renamed from teams to groups |
| `GET /src` | Removed | Use `/health` for version info |

### Response Format Changes

#### Old: /streams

```json
[
  {
    "platform": "twitch",
    "id": "abc123...",
    "userId": "ninja",
    "name": "Ninja",
    "avatar": "https://...",
    "live": true,
    "title": "Playing Fortnite",
    "viewers": 45000,
    "featuredRank": "1",
    "team": "fortnite-pros"
  }
]
```

#### New: /api/v1/streams

```json
{
  "data": [
    {
      "id": "abc123...",
      "platform": "twitch",
      "user_id": "ninja",
      "display_name": "Ninja",
      "avatar_url": "https://...",
      "is_live": true,
      "title": "Playing Fortnite",
      "viewer_count": 45000,
      "thumbnail_url": "https://...",
      "category": "Fortnite",
      "tags": ["English"],
      "language": "en",
      "started_at": "2024-01-15T14:30:00Z",
      "last_updated": "2024-01-15T15:00:00Z",
      "metadata": {
        "priority": 1,
        "group": "fortnite-pros"
      }
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 50,
    "total": 1,
    "total_pages": 1
  }
}
```

### Client Code Updates

#### JavaScript/TypeScript Example

```typescript
// Old client code
async function getStreams() {
  const response = await fetch('http://localhost:9080/streams');
  const streams = await response.json();
  
  return streams.map(s => ({
    name: s.name,
    live: s.live,
    viewers: s.viewers,
    team: s.team,
  }));
}

// New client code
async function getStreams() {
  const response = await fetch('http://localhost:9080/api/v1/streams');
  const { data: streams, pagination } = await response.json();
  
  return streams.map(s => ({
    name: s.display_name,           // Renamed
    live: s.is_live,                // Renamed
    viewers: s.viewer_count,         // Renamed
    team: s.metadata?.group,         // Moved to metadata
  }));
}
```

---

## Step 5: Docker Migration

### Old: Dockerfile

```dockerfile
FROM node:18
# ...
EXPOSE 80
CMD ["npm", "start"]
```

### New: docker-compose.yml

```yaml
version: '3.8'
services:
  stream-aggregator:
    image: stream-aggregator:latest
    ports:
      - "9080:8080"
    volumes:
      - ./config.toml:/etc/stream-aggregator/config.toml:ro
      - ./streamers.toml:/etc/stream-aggregator/streamers.toml:ro
      - ./data:/data
    environment:
      - STREAM_AGG_PROVIDERS_TWITCH_CLIENT_ID=${TWITCH_CLIENT_ID}
      - STREAM_AGG_PROVIDERS_TWITCH_CLIENT_SECRET=${TWITCH_CLIENT_SECRET}
      - STREAM_AGG_STORAGE_BACKEND=sqlite
      - STREAM_AGG_STORAGE_SQLITE_PATH=/data/streams.db
```

---

## Step 6: Feature Mapping

### Teams → Groups + Labels

The new system is more flexible:

```toml
# Old: Simple team assignment
# { "team": "norwegian" }

# New: Group + Labels
[[streamers]]
platform = "twitch"
user_id = "streamer1"
group = "norwegian"
labels = { country = "no", language = "norwegian" }
```

### Featured Rank → Priority

```toml
# Old: featuredRank as string
# { "featuredRank": "1" }

# New: priority as integer
[[streamers]]
platform = "twitch"
user_id = "top_streamer"
priority = 1  # Lower number = higher priority
```

---

## Step 7: New Features to Utilize

### Automatic Discovery

Replace manual Norwegian streamer tracking with automatic discovery:

```toml
# streamers.toml

# Keep your core streamers manually
[[streamers]]
platform = "twitch"
user_id = "core_streamer"
group = "norwegian"

# Add discovery rule for others
[[discovery]]
id = "norwegian-auto"
name = "Auto-discover Norwegian Twitch streamers"
platform = "twitch"
enabled = true
interval_secs = 600

[discovery.filters]
languages = ["no"]
min_viewers = 5

[discovery.apply]
group = "norwegian"
labels = { source = "auto" }
```

### Category-Based Discovery

Track streamers by game:

```toml
[[discovery]]
id = "valorant"
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

### Persistent Storage

Enable SQLite for data persistence:

```toml
[storage]
backend = "sqlite"

[storage.sqlite]
path = "./data/streams.db"
wal_mode = true
```

### Real-time Updates

Use WebSocket for live updates:

```javascript
const ws = new WebSocket('ws://localhost:9080/api/v1/ws/streams');

ws.onmessage = (event) => {
  const { type, data } = JSON.parse(event.data);
  
  switch (type) {
    case 'stream_online':
      console.log(`${data.display_name} went live!`);
      break;
    case 'stream_offline':
      console.log(`${data.display_name} went offline`);
      break;
  }
};
```

---

## Migration Checklist

- [ ] Convert `config.json` to `config.toml`
- [ ] Convert `people.json` to `streamers.toml`
- [ ] Update environment variable names
- [ ] Update API client code for new endpoints
- [ ] Update API client code for new response format
- [ ] Update Docker configuration
- [ ] Test all platforms work correctly
- [ ] Configure persistent storage (optional)
- [ ] Set up discovery rules (optional)
- [ ] Configure WebSocket clients (optional)
- [ ] Update monitoring/alerting for new endpoints
- [ ] Update documentation

---

## Rollback Plan

If you need to rollback:

1. Keep the old Node.js deployment running in parallel initially
2. Use a load balancer to switch traffic
3. The conversion scripts are non-destructive (original files preserved)
4. API responses can be transformed with a simple proxy if needed

### Compatibility Proxy

If you can't update all clients immediately, run a compatibility proxy:

```rust
// Quick compatibility layer (can be a separate small service)
async fn compat_streams(
    State(api_url): State<String>,
) -> impl IntoResponse {
    let response = reqwest::get(&format!("{}/api/v1/streams", api_url))
        .await?
        .json::<NewResponse>()
        .await?;
    
    // Transform to old format
    let old_format: Vec<OldStream> = response.data.into_iter().map(|s| OldStream {
        platform: s.platform,
        id: s.id,
        userId: s.user_id,
        name: s.display_name,
        avatar: s.avatar_url,
        live: s.is_live,
        title: s.title,
        viewers: s.viewer_count,
        featuredRank: s.metadata.get("priority").map(|p| p.to_string()),
        team: s.metadata.get("group").cloned(),
    }).collect();
    
    Json(old_format)
}
```

---

## Getting Help

- GitHub Issues: Report bugs or request features
- Discussions: Ask questions about migration
- Documentation: See other docs in this folder

---

## Timeline Recommendation

| Phase | Duration | Tasks |
|-------|----------|-------|
| 1. Preparation | 1 week | Convert configs, test locally |
| 2. Parallel Run | 2 weeks | Run both systems, compare outputs |
| 3. Soft Switch | 1 week | Route 10% traffic to new system |
| 4. Full Migration | 1 week | Switch all traffic, monitor |
| 5. Cleanup | Ongoing | Remove old system, optimize |
