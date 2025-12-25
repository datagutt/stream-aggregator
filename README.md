# StreamAggregator

**StreamAggregator** is a high-performance, extensible service for aggregating live stream information across multiple platforms.

- **Platform-agnostic**: Support any streaming platform through a plugin system
- **Flexible discovery**: Support both manual streamer lists AND automatic discovery (tags, categories, games)
- **Multi-tenant ready**: Run multiple independent configurations simultaneously
- **Horizontally scalable**: Distribute scraping workloads across multiple instances
- **Observable**: Built-in metrics, tracing, and health checks

## Quick Start

### Docker (Recommended)

```bash
# Pull and run pre-built image
docker run -d \
  --name stream-aggregator \
  -p 8080:8080 \
  -v stream-data:/data \
  -e TWITCH_CLIENT_ID=your_id \
  -e TWITCH_CLIENT_SECRET=your_secret \
  ghcr.io/datagutt/stream-aggregator:latest

# Or use docker-compose (see QUICKSTART-DOCKER.md)
```

### From Source

```bash
# Clone and build
git clone https://github.com/datagutt/stream-aggregator.git
cd stream-aggregator
cargo build --release

# Run
./target/release/stream-aggregator
```

## Documentation

- [QUICKSTART.md](./QUICKSTART.md) - Build and run from source
- [QUICKSTART-DOCKER.md](./QUICKSTART-DOCKER.md) - Docker deployment guide
- [docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md) - Production deployment (Docker, Coolify, Fly.io)
- [docs/API.md](./docs/API.md) - REST API documentation
- [docs/CONFIGURATION.md](./docs/CONFIGURATION.md) - Configuration options

## Docker Images

Pre-built multi-platform images (linux/amd64, linux/arm64) are available on GitHub Container Registry:

```bash
# Latest stable
docker pull ghcr.io/datagutt/stream-aggregator:latest

# Specific version
docker pull ghcr.io/datagutt/stream-aggregator:v1.0.0

# Development builds
docker pull ghcr.io/datagutt/stream-aggregator:develop
```
