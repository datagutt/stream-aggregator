# Deployment Guide

StreamAggregator is designed for maximum deployment flexibility with **zero vendor lock-in**. Deploy anywhere with the same codebase.

## Deployment Options

| Platform | Storage | Cost | Best For |
|----------|---------|------|----------|
| **Cloudflare Workers** | Durable Objects + SQLite | Free tier available | Global edge, zero ops |
| **Docker** | SQLite file | Self-hosted | Full control |
| **Fly.io** | SQLite + Litestream | Free tier (3 VMs) | Low-latency, global |
| **Railway/Render** | SQLite or Turso | Free/Paid | Quick deploy |
| **Coolify** | SQLite | Self-hosted | Self-hosted PaaS |
| **Kubernetes** | SQLite or PostgreSQL | Varies | Enterprise scale |

## Architecture

```
                    ┌─────────────────────────────────────────────┐
                    │           StreamAggregator                  │
                    │                                             │
                    │  ┌─────────────────────────────────────┐   │
                    │  │         StreamStore Trait           │   │
                    │  │   (platform-agnostic interface)     │   │
                    │  └─────────────────────────────────────┘   │
                    │                    │                        │
    ┌───────────────┼────────────────────┼────────────────────┐  │
    │               │                    │                    │  │
    ▼               ▼                    ▼                    ▼  │
┌────────┐    ┌──────────┐        ┌───────────┐        ┌──────────┐
│ Memory │    │  SQLite  │        │  libSQL   │        │ Durable  │
│ Store  │    │  Store   │        │  Store    │        │ Object   │
│        │    │ (sqlx)   │        │ (Turso)   │        │ (CF)     │
└────────┘    └──────────┘        └───────────┘        └──────────┘
    │               │                    │                    │
    ▼               ▼                    ▼                    ▼
  Tests         Docker              Fly.io/Edge           Cloudflare
              Local/VPS             Turso Cloud             Workers
```

---

## 1. Cloudflare Workers (Free Tier)

**Best for**: Zero-ops global deployment, free hosting

### Limits (Free Tier)
- 100,000 requests/day
- 13,000 GB-s compute/day
- 5 million row reads/day
- 100,000 row writes/day
- 5 GB storage

### Setup

```bash
# Prerequisites
npm install -g wrangler
rustup target add wasm32-unknown-unknown
cargo install worker-build

# Navigate to worker crate
cd crates/stream-aggregator-worker

# Login to Cloudflare
wrangler login

# Deploy
wrangler deploy
```

### Configuration

Edit `crates/stream-aggregator-worker/wrangler.toml`:

```toml
name = "stream-aggregator"
main = "build/worker/shim.mjs"
compatibility_date = "2024-12-01"

[[durable_objects.bindings]]
name = "STREAMS_DO"
class_name = "StreamsDO"

[[migrations]]
tag = "v1"
new_sqlite_classes = ["StreamsDO"]

# Cron for scraping (every 5 minutes)
[triggers]
crons = ["*/5 * * * *"]
```

### Secrets

```bash
# Set API secrets
wrangler secret put TWITCH_CLIENT_ID
wrangler secret put TWITCH_CLIENT_SECRET
wrangler secret put API_KEY  # Optional: for authentication
```

---

## 2. Docker

**Best for**: Self-hosted, full control, local development

### Quick Start

```bash
# Build
docker build -t stream-aggregator .

# Run with SQLite
docker run -d \
  --name stream-aggregator \
  -p 8080:8080 \
  -v ./data:/data \
  -e DATABASE_URL=/data/streams.db \
  -e TWITCH_CLIENT_ID=your_id \
  -e TWITCH_CLIENT_SECRET=your_secret \
  stream-aggregator
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'
services:
  stream-aggregator:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data
    environment:
      - DATABASE_URL=/data/streams.db
      - TWITCH_CLIENT_ID=${TWITCH_CLIENT_ID}
      - TWITCH_CLIENT_SECRET=${TWITCH_CLIENT_SECRET}
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3
```

---

## 3. Fly.io (Free Tier)

**Best for**: Global deployment with persistent storage

### Setup

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Create app
fly apps create stream-aggregator

# Create volume for SQLite
fly volumes create data --size 1

# Deploy
fly deploy
```

### fly.toml

```toml
app = "stream-aggregator"
primary_region = "ams"

[build]
  dockerfile = "Dockerfile"

[env]
  HOST = "0.0.0.0"
  PORT = "8080"
  DATABASE_URL = "/data/streams.db"
  RUST_LOG = "info"

[mounts]
  source = "data"
  destination = "/data"

[[services]]
  internal_port = 8080
  protocol = "tcp"

  [[services.ports]]
    port = 80
    handlers = ["http"]

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.http_checks]]
    interval = 10000
    timeout = 2000
    path = "/health"
```

### Secrets

```bash
fly secrets set TWITCH_CLIENT_ID=your_id
fly secrets set TWITCH_CLIENT_SECRET=your_secret
```

---

## 4. Turso (Edge SQLite)

**Best for**: Edge deployment with managed database

Turso provides a globally-distributed SQLite database. Use it with any deployment target.

### Setup

```bash
# Install Turso CLI
curl -sSfL https://get.tur.so/install.sh | bash

# Login
turso auth login

# Create database
turso db create stream-aggregator

# Get connection info
turso db show stream-aggregator --url
turso db tokens create stream-aggregator
```

### Usage with Docker

```bash
docker run -d \
  -p 8080:8080 \
  -e STORE_BACKEND=libsql \
  -e TURSO_URL=libsql://stream-aggregator-xxx.turso.io \
  -e TURSO_TOKEN=your_token \
  stream-aggregator
```

### Usage with Fly.io

```bash
fly secrets set TURSO_URL=libsql://stream-aggregator-xxx.turso.io
fly secrets set TURSO_TOKEN=your_token
```

---

## 5. Coolify (Self-Hosted PaaS)

**Best for**: Self-hosted with nice UI

1. Add your Git repository to Coolify
2. Set build command: `docker build -t stream-aggregator .`
3. Configure environment variables
4. Add persistent storage for `/data`
5. Deploy

---

## 6. Railway

**Best for**: Quick deployment with GitHub integration

1. Connect GitHub repository
2. Railway auto-detects Dockerfile
3. Add environment variables:
   - `DATABASE_URL`: `/data/streams.db`
   - `TWITCH_CLIENT_ID`: Your ID
   - `TWITCH_CLIENT_SECRET`: Your secret
4. Add persistent volume at `/data`
5. Deploy

---

## Migration Between Platforms

All platforms use the same SQL schema, making migration trivial:

### Export from Cloudflare Workers

```bash
curl https://your-worker.workers.dev/export > backup.json
```

### Export from Docker/SQLite

```bash
sqlite3 /data/streams.db ".dump" > backup.sql
```

### Import to new platform

```bash
# Via API
curl -X POST https://new-deployment/api/v1/import \
  -H "Content-Type: application/json" \
  -d @backup.json

# Or via SQLite directly
sqlite3 /data/streams.db < backup.sql
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `HOST` | Server bind address | `127.0.0.1` |
| `PORT` | Server port | `8080` |
| `RUST_LOG` | Log level | `info` |
| `STORE_BACKEND` | Storage backend | `sqlite` |
| `DATABASE_URL` | SQLite path | `./data/streams.db` |
| `TURSO_URL` | Turso connection URL | - |
| `TURSO_TOKEN` | Turso auth token | - |
| `TWITCH_CLIENT_ID` | Twitch API client ID | - |
| `TWITCH_CLIENT_SECRET` | Twitch API secret | - |
| `API_KEYS` | Comma-separated API keys | - |
| `SCRAPE_INTERVAL_SECS` | Scrape interval | `300` |

---

## Recommended Stack

### For Free Hosting
1. **Cloudflare Workers** (API + Storage) - 100% free
2. **Or** Fly.io + SQLite - 3 free VMs

### For Production
1. **Fly.io** + Turso - Low latency, global
2. **Or** Docker + any VPS - Full control

### For Development
1. `cargo run` with SQLite
2. Docker Compose for full stack
