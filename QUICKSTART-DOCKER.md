# Quick Start: Docker Deployment

Get StreamAggregator running in 5 minutes with Docker.

## Prerequisites

- Docker 20.10+ installed
- Docker Compose (comes with Docker Desktop)

## Fast Start: Using Pre-built Image

The fastest way to get started is using our pre-built Docker images from GitHub Container Registry:

### 1. Create docker-compose.yml

```yaml
version: '3.8'

services:
  stream-aggregator:
    image: ghcr.io/datagutt/stream-aggregator:latest
    container_name: stream-aggregator
    ports:
      - "8080:8080"
    volumes:
      - stream-data:/data
    environment:
      - HOST=0.0.0.0
      - PORT=8080
      - RUST_LOG=info
      - STORE_BACKEND=diesel
      - DATABASE_URL=/data/streams.db
      - TWITCH_CLIENT_ID=${TWITCH_CLIENT_ID}
      - TWITCH_CLIENT_SECRET=${TWITCH_CLIENT_SECRET}
    restart: unless-stopped

volumes:
  stream-data:
```

### 2. Create .env file

```bash
cat > .env << EOF
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret
EOF
```

### 3. Start service

```bash
docker-compose up -d
```

That's it! Skip to "Verify Running" section below.

---

## Build from Source (Alternative)

If you want to build from source instead of using pre-built images:

### 1. Clone and Setup

```bash
# Clone repository
git clone https://github.com/datagutt/stream-aggregator.git
cd stream-aggregator

# Copy environment template
cp .env.example .env
```

## 2. Configure Environment

Edit `.env` file with your credentials:

```bash
# Minimal configuration (required)
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret

# Optional: Add API authentication
API_KEYS=your-secret-key-here
```

Get Twitch credentials at: https://dev.twitch.tv/console/apps

## 3. Start Service

```bash
# Start in background
docker-compose up -d

# View logs
docker-compose logs -f stream-aggregator
```

## 4. Verify Running

```bash
# Check health
curl http://localhost:8080/health
# Response: {"status":"ok"}

# List platforms
curl http://localhost:8080/api/v1/platforms
```

## 5. Add Streamers to Track

```bash
# Track a Twitch streamer
curl -X POST http://localhost:8080/api/v1/streamers \
  -H "Content-Type: application/json" \
  -d '{"platform": "twitch", "username": "xqc"}'

# Track a YouTube channel
curl -X POST http://localhost:8080/api/v1/streamers \
  -H "Content-Type: application/json" \
  -d '{"platform": "youtube", "username": "@MrBeast"}'

# If you enabled API_KEYS, add header:
# -H "X-API-Key: your-secret-key-here"
```

## 6. View Live Streams

```bash
# Get all live streams
curl http://localhost:8080/api/v1/streams

# Get specific stream by ID
curl http://localhost:8080/api/v1/streams/STREAM_ID
```

## Management Commands

```bash
# View logs
docker-compose logs -f

# Restart service
docker-compose restart

# Stop service
docker-compose stop

# Stop and remove containers (keeps data)
docker-compose down

# Stop and remove everything (including database)
docker-compose down -v
```

## Database Location

SQLite database is stored in a Docker volume named `stream-data`.

To access:

```bash
# Find volume location
docker volume inspect stream-aggregator_stream-data

# Backup database
docker run --rm -v stream-aggregator_stream-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/streams-backup.tar.gz -C /data .

# Restore database
docker run --rm -v stream-aggregator_stream-data:/data -v $(pwd):/backup \
  alpine tar xzf /backup/streams-backup.tar.gz -C /data
```

## Troubleshooting

### Port already in use

Change port in `.env`:
```bash
PORT=8081
```

### Container won't start

Check logs:
```bash
docker-compose logs
```

### Reset everything

```bash
# Stop and remove all data
docker-compose down -v

# Rebuild from scratch
docker-compose build --no-cache
docker-compose up -d
```

## Next Steps

- Read [DEPLOYMENT.md](docs/DEPLOYMENT.md) for production deployment
- Read [API.md](docs/API.md) for full API documentation
- Configure providers in [config.toml](config.example.toml)

## Configuration File (Alternative to .env)

Instead of using `.env` for environment variables, you can use a `config.toml` file for more organized configuration.

### Why use config.toml?

- More organized for complex setups
- Version control your configuration (except secrets)
- Enable/disable providers easily
- Configure scheduler, auth, and all providers in one place
- Less cluttered than many environment variables

### How to use config.toml

**Step 1: Create config.toml**

```bash
# Copy example
cp config.example.toml config.toml
```

**Step 2: Edit your configuration**

```toml
# config.toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
api_keys = ["your-secret-key"]
require_all = false

[scheduler]
interval_secs = 300
max_concurrent = 10

[store]
backend = "diesel"
database_url = "/data/streams.db"

[providers.twitch]
enabled = true
client_id = "your_twitch_client_id"
client_secret = "your_twitch_client_secret"

[providers.youtube]
enabled = true

[providers.kick]
enabled = true

[providers.dlive]
enabled = false  # Easy to disable providers

[providers.trovo]
enabled = true

[providers.guac]
enabled = true

[providers.angelthump]
enabled = true

[providers.robotstreamer]
enabled = true
```

**Step 3: Enable config.toml in docker-compose.yml**

Edit `docker-compose.yml` and uncomment the config.toml volume:

```yaml
volumes:
  - stream-data:/data
  - ./config.toml:/app/config.toml:ro  # Uncomment this line
```

**Step 4: Start the service**

```bash
docker-compose up -d
```

### Best Practice: Separate Secrets

For better security, keep secrets out of config.toml:

**config.toml** (commit to git):
```toml
[providers.twitch]
enabled = true
# Don't put secrets here!
```

**.env** (in .gitignore):
```bash
TWITCH_CLIENT_ID=your_id
TWITCH_CLIENT_SECRET=your_secret
```

The application will load settings from config.toml and override with environment variables from .env.

### Config Priority

1. Environment variables (highest priority)
2. config.toml file
3. Default values

This means you can set defaults in config.toml and override sensitive values with .env.

## Production Checklist

Before deploying to production:

- [ ] Set strong `API_KEYS` in `.env`
- [ ] Set `REQUIRE_AUTH_ALL=true` for private API
- [ ] Use HTTPS reverse proxy (nginx, Caddy, or Coolify)
- [ ] Set up database backups
- [ ] Monitor logs for errors
- [ ] Test health endpoint: `/health`

## Quick Deploy to Coolify

1. Push code to GitHub/GitLab
2. Add repository in Coolify
3. Copy environment variables from `.env`
4. Add persistent volume: `/data`
5. Deploy

See [DEPLOYMENT.md](docs/DEPLOYMENT.md) for detailed Coolify guide.
