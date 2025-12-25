# Deployment Guide

StreamAggregator is designed for flexible, production-ready deployments using **Docker** and **Diesel ORM with SQLite**.

## Current Architecture

```
┌─────────────────────────────────────────────┐
│         StreamAggregator Service            │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │      Diesel ORM (StreamStore)       │   │
│  │   (platform-agnostic interface)     │   │
│  └─────────────────────────────────────┘   │
│                    │                        │
│                    ▼                        │
│              ┌──────────┐                   │
│              │  SQLite  │                   │
│              │ Database │                   │
│              └──────────┘                   │
│                    │                        │
└────────────────────┼────────────────────────┘
                     ▼
              Persistent Volume
           (/data/streams.db)
```

## Supported Deployment Platforms

| Platform | Storage | Cost | Best For |
|----------|---------|------|----------|
| **Docker** | SQLite file | Self-hosted | Full control, local dev |
| **Coolify** | SQLite | Self-hosted | Self-hosted PaaS with UI |
| **Fly.io** | SQLite + Volumes | Free tier (3 VMs) | Low-latency, global |
| **Railway/Render** | SQLite + Volumes | Free/Paid | Quick deploy, managed |
| **Any VPS** | SQLite | Varies | Traditional hosting |

---

## Quick Start: Docker

### Prerequisites

- Docker 20.10+
- Docker Compose (optional, recommended)

### Option 1: Docker Compose (Recommended)

```bash
# 1. Clone repository
git clone https://github.com/yourusername/stream-aggregator.git
cd stream-aggregator

# 2. Create .env file for secrets
cat > .env << EOF
# Optional: Twitch credentials
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret

# Optional: API authentication
API_KEYS=your-secret-key-1,your-secret-key-2

# Optional: Logging
RUST_LOG=info

# Optional: Scrape interval (seconds)
SCRAPE_INTERVAL_SECS=300
EOF

# 3. Start the service
docker-compose up -d

# 4. View logs
docker-compose logs -f

# 5. Check health
curl http://localhost:8080/health
```

### Option 2: Docker CLI

```bash
# Build image
docker build -t stream-aggregator .

# Run with volume for persistence
docker run -d \
  --name stream-aggregator \
  -p 8080:8080 \
  -v stream-data:/data \
  -e DATABASE_URL=/data/streams.db \
  -e TWITCH_CLIENT_ID=your_id \
  -e TWITCH_CLIENT_SECRET=your_secret \
  stream-aggregator

# View logs
docker logs -f stream-aggregator
```

### Database Migrations

Migrations run automatically on startup. The database schema is created in `/data/streams.db` on first run.

To run migrations manually:

```bash
# Install diesel_cli
cargo install diesel_cli --no-default-features --features sqlite

# Run migrations
DATABASE_URL=./data/streams.db diesel migration run
```

---

## Production Deployment: Coolify

**Best for**: Self-hosted with a beautiful UI, GitHub integration, automatic SSL

[Coolify](https://coolify.io/) is a self-hosted PaaS alternative to Heroku/Netlify.

### Prerequisites

- Coolify instance running (self-hosted or cloud)
- Git repository (GitHub/GitLab/Gitea)

### Deployment Steps

1. **Add New Resource** in Coolify dashboard
   - Click "New Resource" → "Public Repository"
   - Enter repository URL: `https://github.com/yourusername/stream-aggregator`

2. **Configure Build Settings**
   - Build Type: `Dockerfile`
   - Dockerfile Path: `./Dockerfile`
   - Port: `8080`

3. **Choose Configuration Method**

   You have two options for configuration: **Environment Variables** (simpler) or **config.toml file** (more organized).

#### Option A: Environment Variables (Recommended for Simple Setups)

Add environment variables in Coolify dashboard:

```
HOST=0.0.0.0
PORT=8080
RUST_LOG=info
STORE_BACKEND=diesel
DATABASE_URL=/data/streams.db
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret
API_KEYS=your-secret-key-here
SCRAPE_INTERVAL_SECS=300
```

#### Option B: config.toml File (Recommended for Complex Configurations)

**Step 1**: Create `config.production.toml` in your repository:

```toml
# config.production.toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
api_keys = ["your-secret-key-here"]
require_all = false

[scheduler]
interval_secs = 300
max_concurrent = 10

[store]
backend = "diesel"
database_url = "/data/streams.db"

[providers.twitch]
enabled = true
# Use environment variables for secrets
# client_id and client_secret will be loaded from env vars

[providers.youtube]
enabled = true

[providers.kick]
enabled = true

[providers.dlive]
enabled = true

[providers.trovo]
enabled = true

[providers.guac]
enabled = true

[providers.angelthump]
enabled = true

[providers.robotstreamer]
enabled = true
```

**Step 2**: Update Dockerfile to use config file:

Create a custom Dockerfile in your repo (or use a build hook):

```dockerfile
FROM stream-aggregator:latest
COPY config.production.toml /app/config.toml
CMD ["stream-aggregator", "--config", "/app/config.toml"]
```

**Step 3**: Set sensitive values as environment variables:

```
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret
```

**Why use config.toml?**
- Better organization for complex setups
- Version control your configuration
- Easier to manage multiple providers
- Less environment variables clutter
- Can still override with env vars for secrets

4. **Configure Persistent Storage**
   - Add Volume:
     - Source: `stream-data` (named volume)
     - Destination: `/data`
     - This ensures database persists across deployments

5. **Add Health Check**
   - Path: `/health`
   - Port: `8080`
   - Interval: `30s`

6. **Enable Automatic Deployments**
   - Enable "Auto Deploy on Push"
   - Coolify will rebuild on every git push to main branch

7. **Configure Domain** (optional)
   - Add custom domain or use Coolify-provided domain
   - SSL certificates are automatic via Let's Encrypt

8. **Deploy**
   - Click "Deploy" button
   - Monitor build logs in real-time
   - Service will be available at `https://your-domain.com`

### Coolify Configuration Best Practices

**For Secrets Management:**
```toml
# config.production.toml - Commit to git
[providers.twitch]
enabled = true
# Don't put secrets in config file!
# Use environment variables instead:
# TWITCH_CLIENT_ID (set in Coolify dashboard)
# TWITCH_CLIENT_SECRET (set in Coolify dashboard)
```

Then in Coolify, set environment variables:
```
TWITCH_CLIENT_ID=your_id
TWITCH_CLIENT_SECRET=your_secret
```

The application will merge config.toml with environment variables.

**For Multi-Environment Setup:**

Create different config files:
- `config.production.toml` - Production settings
- `config.staging.toml` - Staging settings
- `config.toml` - Local development (not committed)

Then point to the right config in Coolify build:
```dockerfile
# For production
COPY config.production.toml /app/config.toml

# Or use environment variable to select
ARG CONFIG_ENV=production
COPY config.${CONFIG_ENV}.toml /app/config.toml
```

### Coolify Tips

- **Logs**: View real-time logs in the Coolify dashboard
- **Shell Access**: Use built-in terminal to inspect container
- **Backups**: Enable automatic backups of the `/data` volume
- **Scaling**: Coolify supports horizontal scaling (multiple replicas)
- **Updates**: Push to git → Coolify rebuilds → Zero-downtime deployment
- **Config Updates**: Change config.toml → commit → push → auto-deploy

---

## Production Deployment: Fly.io

**Best for**: Global edge deployment with persistent storage

### Prerequisites

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
fly auth login
```

### Deployment Steps

```bash
# 1. Create app
fly apps create stream-aggregator

# 2. Create persistent volume for SQLite (1GB)
fly volumes create stream_data --size 1 --region ams

# 3. Create fly.toml
cat > fly.toml << 'EOF'
app = "stream-aggregator"
primary_region = "ams"

[build]
  dockerfile = "Dockerfile"

[env]
  HOST = "0.0.0.0"
  PORT = "8080"
  RUST_LOG = "info"
  STORE_BACKEND = "diesel"
  DATABASE_URL = "/data/streams.db"

[mounts]
  source = "stream_data"
  destination = "/data"

[[services]]
  internal_port = 8080
  protocol = "tcp"
  auto_stop_machines = false
  auto_start_machines = true

  [[services.ports]]
    port = 80
    handlers = ["http"]

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [services.concurrency]
    type = "connections"
    hard_limit = 25
    soft_limit = 20

  [[services.http_checks]]
    interval = 10000
    timeout = 2000
    method = "get"
    path = "/health"
EOF

# 4. Set secrets
fly secrets set \
  TWITCH_CLIENT_ID=your_id \
  TWITCH_CLIENT_SECRET=your_secret

# 5. Deploy
fly deploy

# 6. Check status
fly status
fly logs
```

### Fly.io Volume Management

```bash
# List volumes
fly volumes list

# Create snapshot backup
fly volumes snapshots create stream_data

# Scale to multiple regions (optional)
fly volumes create stream_data --size 1 --region lhr
fly scale count 2
```

---

## Production Deployment: VPS (Manual)

**Best for**: Traditional VPS hosting (DigitalOcean, Linode, etc.)

### Prerequisites

- Linux VPS with Docker installed
- Domain name (optional)
- Reverse proxy (nginx/caddy recommended)

### Setup Steps

```bash
# 1. SSH to VPS
ssh user@your-vps-ip

# 2. Install Docker (if not installed)
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# 3. Clone repository
git clone https://github.com/yourusername/stream-aggregator.git
cd stream-aggregator

# 4. Create environment file
nano .env
# Add your configuration (see Docker Compose example)

# 5. Start with Docker Compose
docker-compose up -d

# 6. Enable restart on boot
docker update --restart unless-stopped stream-aggregator
```

### Nginx Reverse Proxy

```nginx
# /etc/nginx/sites-available/stream-aggregator
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable and get SSL:

```bash
# Enable site
ln -s /etc/nginx/sites-available/stream-aggregator /etc/nginx/sites-enabled/
nginx -t
systemctl reload nginx

# Get SSL certificate with certbot
apt install certbot python3-certbot-nginx
certbot --nginx -d your-domain.com
```

---

## Environment Variables Reference

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `HOST` | Server bind address | `127.0.0.1` | No |
| `PORT` | Server port | `8080` | No |
| `RUST_LOG` | Log level (trace/debug/info/warn/error) | `info` | No |
| `STORE_BACKEND` | Storage backend (always use "diesel") | `diesel` | No |
| `DATABASE_URL` | SQLite database path | `/data/streams.db` | No |
| `TWITCH_CLIENT_ID` | Twitch API client ID | - | For Twitch |
| `TWITCH_CLIENT_SECRET` | Twitch API secret | - | For Twitch |
| `API_KEYS` | Comma-separated API keys for auth | - | No |
| `REQUIRE_AUTH_ALL` | Require auth for all endpoints | `false` | No |
| `SCRAPE_INTERVAL_SECS` | Scrape interval in seconds | `300` | No |

---

## Configuration File (Optional)

Instead of environment variables, you can use a `config.toml` file:

```toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
api_keys = ["secret-key-1", "secret-key-2"]
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
```

Mount in Docker:

```bash
docker run -v ./config.toml:/app/config.toml:ro stream-aggregator
```

---

## Monitoring and Maintenance

### Health Check

```bash
curl http://localhost:8080/health
# Response: {"status":"ok"}
```

### View Logs

```bash
# Docker Compose
docker-compose logs -f

# Docker
docker logs -f stream-aggregator

# Fly.io
fly logs

# Coolify
# Use web dashboard
```

### Database Backup

```bash
# Stop container
docker-compose stop

# Backup SQLite database
cp data/streams.db data/streams.db.backup

# Or use SQLite backup command
sqlite3 data/streams.db ".backup data/streams-$(date +%Y%m%d).db"

# Restart
docker-compose start
```

### Database Maintenance

```bash
# Access database
sqlite3 data/streams.db

# Vacuum (reclaim space)
sqlite3 data/streams.db "VACUUM;"

# Check integrity
sqlite3 data/streams.db "PRAGMA integrity_check;"
```

---

## Scaling Considerations

### Single Instance

For most use cases, a single instance handles thousands of tracked streamers:

- **Memory**: ~50-200MB RAM
- **CPU**: Low usage, spikes during scraping
- **Storage**: ~1-10MB database (depends on stream count)
- **Network**: Minimal, mainly API calls to platforms

### Horizontal Scaling (Future)

For massive scale (10,000+ streamers), horizontal scaling options:

1. **Multiple instances with shared database**
   - Use PostgreSQL instead of SQLite
   - Load balancer in front
   - Requires coordination for scheduler

2. **Sharding by platform**
   - Instance 1: Twitch + YouTube
   - Instance 2: Kick + DLive
   - Aggregate results via API gateway

---

## Troubleshooting

### Container won't start

```bash
# Check logs
docker-compose logs

# Common issues:
# - Port 8080 already in use: Change PORT env var
# - Volume permission denied: Check file permissions
# - Database locked: Stop all containers accessing DB
```

### Database errors

```bash
# Reset database (WARNING: deletes all data)
docker-compose down -v
docker-compose up -d

# Or manually delete
rm data/streams.db*
docker-compose restart
```

### Build fails

```bash
# Clear build cache
docker-compose build --no-cache

# Check Rust version in Dockerfile (requires 1.75+)
```

### Performance issues

```bash
# Increase scrape interval
echo "SCRAPE_INTERVAL_SECS=600" >> .env
docker-compose up -d

# Check CPU/memory usage
docker stats stream-aggregator
```

---

## Migration from Legacy Node.js Version

If migrating from the old Node.js implementation:

1. **Export data** from old version (if applicable)
2. **Deploy new Rust version** following guides above
3. **Track streamers** via API:
   ```bash
   curl -X POST http://localhost:8080/api/v1/streamers \
     -H "Content-Type: application/json" \
     -d '{"platform": "twitch", "username": "xqc"}'
   ```

See [MIGRATION.md](./MIGRATION.md) for detailed migration guide.

---

## Recommended Setup

### For Development
- Docker Compose with local SQLite
- Hot reload: Use `cargo watch` instead

### For Production (Small/Medium)
- **Coolify** (if self-hosting) - Best UI, automation, SSL
- **Fly.io** (if cloud hosting) - Best for global edge
- Both support automatic deployments, SSL, monitoring

### For Production (Large Scale)
- VPS with nginx reverse proxy
- PostgreSQL instead of SQLite (requires code changes)
- Monitoring: Prometheus + Grafana
- Backups: Automated daily SQLite dumps

---

## Security Best Practices

1. **Always use API keys** in production
   ```bash
   API_KEYS=randomly-generated-secret-key
   REQUIRE_AUTH_ALL=true
   ```

2. **Use HTTPS** (Let's Encrypt with nginx/Coolify/Fly.io)

3. **Restrict database access**
   - Volume permissions: `chown 1000:1000 data/`
   - No public database access

4. **Keep secrets out of git**
   - Use `.env` files (in `.gitignore`)
   - Use platform secret managers

5. **Regular updates**
   ```bash
   git pull
   docker-compose build
   docker-compose up -d
   ```

---

## Support

- Issues: [GitHub Issues](https://github.com/yourusername/stream-aggregator/issues)
- Docs: [docs/](.)
- Discord: [Join our community](#)
