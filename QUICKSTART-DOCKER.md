# Quick Start: Docker Deployment

Get StreamAggregator running in 5 minutes with Docker.

## Prerequisites

- Docker 20.10+ installed
- Docker Compose (comes with Docker Desktop)

## 1. Clone and Setup

```bash
# Clone repository
git clone https://github.com/yourusername/stream-aggregator.git
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

## Configuration File (Advanced)

Instead of `.env`, you can use `config.toml`:

```bash
# Copy example
cp config.example.toml config.toml

# Edit configuration
nano config.toml

# Mount in docker-compose.yml
# Uncomment the config.toml volume line
```

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
