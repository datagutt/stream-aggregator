# StreamAggregator

High-performance Rust service for aggregating live stream information across multiple platforms.

> Rewrite of the [original Node.js implementation](https://github.com/livestreamnorge/lsnd) with improved performance and type safety.

## Features

- **Multi-platform**: Twitch, YouTube, Kick, TikTok, and more
- **Fast**: Rust with async/await and efficient scraping  
- **Persistent**: SQLite with Diesel ORM
- **Production-ready**: Docker images for amd64/arm64

## Quick Start

### Docker (5 Minutes)

```bash
docker run -d \
  --name stream-aggregator \
  -p 8080:8080 \
  -v stream-data:/data \
  -e TWITCH_CLIENT_ID=your_id \
  -e TWITCH_CLIENT_SECRET=your_secret \
  ghcr.io/datagutt/stream-aggregator:latest
```

**Or with docker-compose:**

```yaml
services:
  stream-aggregator:
    image: ghcr.io/datagutt/stream-aggregator:latest
    ports:
      - "8080:8080"
    volumes:
      - stream-data:/data
    environment:
      - DATABASE_URL=/data/streams.db
      - TWITCH_CLIENT_ID=${TWITCH_CLIENT_ID}
      - TWITCH_CLIENT_SECRET=${TWITCH_CLIENT_SECRET}
    restart: unless-stopped

volumes:
  stream-data:
```

**Get Twitch credentials:** https://dev.twitch.tv/console/apps

### From Source

```bash
git clone https://github.com/datagutt/stream-aggregator.git
cd stream-aggregator
cargo build --release
./target/release/stream-aggregator
```

## Usage

### Track a Streamer

```bash
curl -X POST http://localhost:8080/api/v1/streamers \
  -H "Content-Type: application/json" \
  -d '{"platform": "twitch", "username": "ninja"}'
```

### Get Live Streams

```bash
# All streams
curl http://localhost:8080/api/v1/streams

# By platform (multi-value via bracket notation)
curl "http://localhost:8080/api/v1/streams?platform[]=twitch&platform[]=youtube"

# Health check
curl http://localhost:8080/health
```

## Supported Platforms

| Platform | Auth Required | Notes |
|----------|---------------|-------|
| Twitch | Yes (OAuth) | https://dev.twitch.tv/console/apps |
| YouTube | No | HTML scraping |
| Kick | No | Browser emulation (Cloudflare bypass) |
| TikTok | No | Node.js bridge (included in Docker, auto-detected) |
| Guac | No | REST API |
| AngelThump | No | REST API |
| RobotStreamer | No | REST API |

## Configuration

### Environment Variables (.env file)

```bash
HOST=0.0.0.0
PORT=8080
DATABASE_URL=/data/streams.db
TWITCH_CLIENT_ID=your_id
TWITCH_CLIENT_SECRET=your_secret
SCRAPE_INTERVAL_SECS=300
API_KEYS=secret-key  # Optional: Enable authentication
```

### Configuration File (config.toml)

```toml
[server]
host = "0.0.0.0"
port = 8080

[scheduler]
interval_secs = 300  # Scrape every 5 minutes
max_concurrent = 10

[store]
backend = "diesel"
database_url = "stream_aggregator.db"

[providers.twitch]
enabled = true
client_id = "your_id"
client_secret = "your_secret"

[providers.youtube]
enabled = true

[providers.tiktok]
enabled = true
# Bridge auto-detected in Docker (/app/nodejs-bridge)
# For local development, ensure nodejs-bridge directory exists
```

See `config.example.toml` for all options.

**Note on TikTok**: The TikTok provider uses a Node.js bridge that is automatically included in the Docker image. For local development from source, the bridge is auto-detected in the `crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge` directory. If you get a warning about TikTok bridge not found, either disable it with `enabled = false` or ensure Node.js 24+ LTS is installed.

## Deployment

### Docker Images (GitHub Container Registry)

```bash
# Latest stable
ghcr.io/datagutt/stream-aggregator:latest

# Specific version
ghcr.io/datagutt/stream-aggregator:v1.0.0

# Development
ghcr.io/datagutt/stream-aggregator:develop
```

Multi-platform: linux/amd64, linux/arm64

### Platforms

- **Coolify** - Self-hosted PaaS with UI
- **Fly.io** - Global edge deployment  
- **Docker Compose** - Self-hosted
- **Railway / Render** - Managed platforms

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for full guides.

## API

### Endpoints

```
GET  /health                    Health check
GET  /api/v1/platforms          List platforms
GET  /api/v1/streams            Get all streams
GET  /api/v1/streams?platform=X Filter by platform
GET  /api/v1/streamers          List tracked streamers
POST /api/v1/streamers          Add streamer
DELETE /api/v1/streamers/:id    Remove streamer
```

### Authentication (Optional)

```bash
# Enable with environment variable
API_KEYS=secret-key-1,secret-key-2

# Use in requests
curl -H "X-API-Key: secret-key-1" http://localhost:8080/api/v1/streamers
```

See [docs/API.md](docs/API.md) for complete API documentation.

## Documentation

- **[API.md](docs/API.md)** - Complete REST API reference
- **[CONFIGURATION.md](docs/CONFIGURATION.md)** - All configuration options
- **[DEPLOYMENT.md](docs/DEPLOYMENT.md)** - Production deployment guides
- **[DEVELOPMENT.md](docs/DEVELOPMENT.md)** - Architecture and contributing

## Development

```bash
cargo build              # Build
cargo test               # Run tests
cargo fmt                # Format code
cargo clippy             # Lint
RUST_LOG=debug cargo run # Run with debug logging
```

See [AGENTS.md](AGENTS.md) for build commands and [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for architecture.

## Migrating from Node.js

This is a rewrite of the [original Node.js implementation](https://github.com/livestreamnorge/lsnd) with a similar API design but improved architecture.

**Benefits:**
- Improved performance and efficiency
- Type safety with Rust
- Persistent storage with migrations
- Better error handling
- Easier deployment with Docker

## License

GNU Affero General Public License v3.0 - see [LICENSE](LICENSE)

## Credits

- Original: [livestreamnorge/lsnd](https://github.com/livestreamnorge/lsnd)
- Maintainer: [@datagutt](https://github.com/datagutt)

## Support

- [GitHub Issues](https://github.com/datagutt/stream-aggregator/issues)
- [GitHub Discussions](https://github.com/datagutt/stream-aggregator/discussions)
