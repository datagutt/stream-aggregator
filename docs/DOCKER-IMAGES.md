# Docker Images Guide

StreamAggregator provides pre-built Docker images via GitHub Container Registry (GHCR) for quick deployment.

## Available Images

Images are automatically built and published on every commit to `main` and `develop` branches, and on version tags.

### Image Tags

| Tag | Description | Use Case |
|-----|-------------|----------|
| `latest` | Latest stable release from main branch | Production |
| `v1.0.0` | Specific version (semver) | Production (pinned version) |
| `v1.0` | Latest patch version of v1.0.x | Production (auto-patch updates) |
| `v1` | Latest minor version of v1.x.x | Production (auto-minor updates) |
| `main` | Latest commit on main branch | Staging/Testing |
| `develop` | Latest commit on develop branch | Development/Testing |
| `main-abc123` | Specific commit SHA | Debugging specific builds |

## Pulling Images

### Latest Stable Version

```bash
docker pull ghcr.io/yourusername/stream-aggregator:latest
```

### Specific Version

```bash
docker pull ghcr.io/yourusername/stream-aggregator:v1.0.0
```

### Development Version

```bash
docker pull ghcr.io/yourusername/stream-aggregator:develop
```

## Platform Support

All images are built for multiple platforms:
- `linux/amd64` - x86_64 processors (Intel, AMD)
- `linux/arm64` - ARM processors (Apple Silicon, Raspberry Pi 4+, AWS Graviton)

Docker automatically pulls the correct image for your platform.

## Using in Docker Compose

### Production (Pinned Version)

```yaml
version: '3.8'

services:
  stream-aggregator:
    image: ghcr.io/yourusername/stream-aggregator:v1.0.0
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

### Production (Auto-update patches)

```yaml
services:
  stream-aggregator:
    image: ghcr.io/yourusername/stream-aggregator:v1.0
    # ... rest of config
```

### Staging/Development

```yaml
services:
  stream-aggregator:
    image: ghcr.io/yourusername/stream-aggregator:main
    # ... rest of config
```

## Using with Coolify

### Option 1: Direct Docker Image (Recommended)

1. In Coolify dashboard, click "New Resource" → "Docker Image"
2. Enter image: `ghcr.io/yourusername/stream-aggregator:latest`
3. Configure port: `8080`
4. Add environment variables
5. Add persistent volume: `/data`
6. Deploy

### Option 2: Auto-update on new releases

1. Use Docker Image deployment (as above)
2. Enable "Check for updates" in Coolify
3. Set update schedule (e.g., daily at 2 AM)
4. Coolify will automatically pull new images and redeploy

## Authentication

GitHub Container Registry is public by default for public repositories. No authentication needed to pull images.

For private repositories, authenticate first:

```bash
# Create GitHub Personal Access Token with 'read:packages' scope
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Then pull
docker pull ghcr.io/yourusername/stream-aggregator:latest
```

## Image Details

### Image Size

- Compressed: ~100-150 MB
- Uncompressed: ~300-400 MB

### Base Image

- Runtime: `debian:trixie-slim`
- Minimal dependencies for small size
- Security updates applied automatically

### Security

- Non-root user (UID 1000)
- Read-only filesystem (except `/data`)
- Health checks built-in
- Provenance attestations included

## Updating Images

### Manual Update

```bash
# Pull latest image
docker pull ghcr.io/yourusername/stream-aggregator:latest

# Stop current container
docker stop stream-aggregator
docker rm stream-aggregator

# Start with new image
docker run -d \
  --name stream-aggregator \
  -p 8080:8080 \
  -v stream-data:/data \
  -e DATABASE_URL=/data/streams.db \
  ghcr.io/yourusername/stream-aggregator:latest
```

### With Docker Compose

```bash
# Pull latest images
docker-compose pull

# Recreate containers
docker-compose up -d
```

### Automatic Updates with Watchtower

```yaml
version: '3.8'

services:
  stream-aggregator:
    image: ghcr.io/yourusername/stream-aggregator:latest
    # ... your config

  watchtower:
    image: containrrr/watchtower
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    command: --interval 3600 --cleanup
    # Checks for updates every hour
```

## Verifying Images

### Check Image Digest

```bash
docker inspect ghcr.io/yourusername/stream-aggregator:latest \
  --format='{{.RepoDigests}}'
```

### Verify Provenance (Security)

GitHub automatically generates provenance attestations for all images:

```bash
# Install GitHub CLI
gh attestation verify \
  oci://ghcr.io/yourusername/stream-aggregator:latest \
  --owner yourusername
```

## Build Information

Images are built using GitHub Actions with the following workflow:

- **Trigger**: Push to main/develop, version tags, manual
- **Build time**: ~10-15 minutes
- **Platforms**: linux/amd64, linux/arm64 (parallel builds)
- **Caching**: Enabled for faster builds
- **Testing**: Health check verification before publish

## Troubleshooting

### Image Pull Fails

```bash
# Check if image exists
docker manifest inspect ghcr.io/yourusername/stream-aggregator:latest

# Try with full digest
docker pull ghcr.io/yourusername/stream-aggregator@sha256:abc123...
```

### Wrong Architecture

```bash
# Force specific platform
docker pull --platform linux/amd64 \
  ghcr.io/yourusername/stream-aggregator:latest
```

### Rate Limiting

GHCR has generous rate limits, but if you hit them:

```bash
# Authenticate (increases limits)
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

## Migration from Other Registries

### From Docker Hub

Old:
```yaml
image: yourusername/stream-aggregator:latest
```

New:
```yaml
image: ghcr.io/yourusername/stream-aggregator:latest
```

### Benefits of GHCR

- Free for public repositories
- Unlimited storage
- Integrated with GitHub (same auth)
- Better rate limits
- Provenance attestations
- Dependency graph integration

## CI/CD Integration

### Pull in CI

```yaml
# GitHub Actions
- name: Pull image
  run: docker pull ghcr.io/yourusername/stream-aggregator:latest

# GitLab CI
script:
  - docker pull ghcr.io/yourusername/stream-aggregator:latest
```

### Use in Tests

```yaml
services:
  stream-aggregator:
    image: ghcr.io/yourusername/stream-aggregator:develop
    ports:
      - "8080:8080"
```

## Support

- Issues: [GitHub Issues](https://github.com/yourusername/stream-aggregator/issues)
- Security: Report to security@yourproject.com
- Updates: Watch GitHub releases
