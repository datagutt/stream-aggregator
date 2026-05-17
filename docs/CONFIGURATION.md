# Configuration Guide

This document explains how to configure StreamAggregator using TOML configuration files and environment variables.

## Configuration System

StreamAggregator uses a layered configuration system with the following priority (highest to lowest):

1. **Command-line arguments** - Highest priority
2. **Environment variables** - Override config file  
3. **Configuration file** (TOML) - Base configuration
4. **Default values** - Fallback defaults

This means you can set defaults in `config.toml`, override with environment variables, and further override with CLI flags.

## Quick Start

### Option 1: Environment Variables (Simplest)

```bash
# Create .env file
cat > .env << EOF
HOST=0.0.0.0
PORT=8080
STORE_BACKEND=diesel
DATABASE_URL=/data/streams.db
TWITCH_CLIENT_ID=your_id
TWITCH_CLIENT_SECRET=your_secret
EOF

# Run
docker-compose up  # Or cargo run
```

### Option 2: Configuration File (Recommended)

```bash
# Copy example
cp config.example.toml config.toml

# Edit your config
nano config.toml

# Run
cargo run -- --config config.toml
```

## Configuration File Format

### Basic Example

```toml
# config.toml

[server]
host = "0.0.0.0"
port = 8080

[auth]
api_keys = []  # Empty = public access
require_all = false  # false = public reads, auth for writes

[scheduler]
interval_secs = 300  # Scrape every 5 minutes
max_concurrent = 10

[store]
backend = "diesel"  # Use Diesel ORM
database_url = "stream_aggregator.db"  # SQLite file path

[providers.twitch]
enabled = true
client_id = "your_twitch_client_id"
client_secret = "your_twitch_client_secret"

[providers.youtube]
enabled = true

[providers.kick]
enabled = true
```

### Complete Example

See `config.example.toml` in the repository root for a complete example with all available options.

## Configuration Options

### Server Settings

```toml
[server]
host = "0.0.0.0"     # Bind address (0.0.0.0 for Docker, 127.0.0.1 for local)
port = 8080           # HTTP port
```

### Authentication

```toml
[auth]
# API keys for authentication (generate with: openssl rand -hex 32)
api_keys = ["secret-key-1", "secret-key-2"]

# Require authentication for ALL requests (including GET)
# false = Public reads, auth for writes (default)
# true = All endpoints require authentication
require_all = false
```

**Environment variable:**
```bash
API_KEYS=secret-key-1,secret-key-2
REQUIRE_AUTH_ALL=true
```

### Scheduler

```toml
[scheduler]
interval_secs = 300      # Scrape interval (seconds)
max_concurrent = 10      # Max concurrent scrape tasks
```

**Recommended intervals:**
- Development: 60 seconds (fast feedback)
- Production: 300 seconds (5 minutes, balanced)
- Low traffic: 600 seconds (10 minutes, reduce API usage)

### Storage Backend

```toml
[store]
backend = "diesel"  # Options: "memory" or "diesel"
database_url = "stream_aggregator.db"  # SQLite file path
```

**SQLite (default):**
```toml
database_url = "stream_aggregator.db"          # Relative path
database_url = "/data/streams.db"              # Absolute path
database_url = "/var/lib/streamagg/data.db"    # Custom location
```

**PostgreSQL (future support):**
```toml
database_url = "postgres://user:password@localhost:5432/dbname"
```

**In-memory (development only):**
```toml
backend = "memory"  # Data lost on restart
```

### Platform Providers

All providers are enabled by default. Disable unwanted platforms:

```toml
[providers.twitch]
enabled = true
client_id = "your_client_id"       # Required
client_secret = "your_client_secret"  # Required

[providers.youtube]
enabled = true  # No credentials needed

[providers.kick]
enabled = true  # No credentials needed

[providers.guac]
enabled = true  # No credentials needed

[providers.angelthump]
enabled = true  # No credentials needed

[providers.robotstreamer]
enabled = true  # No credentials needed
```

## Environment Variables

All configuration options can be set via environment variables. The application automatically reads from:

1. System environment variables
2. `.env` file in the working directory

### Basic Variables

```bash
# Server
HOST=0.0.0.0
PORT=8080

# Logging
RUST_LOG=info  # Options: trace, debug, info, warn, error

# Storage
STORE_BACKEND=diesel
DATABASE_URL=/data/streams.db

# Scheduler
SCRAPE_INTERVAL_SECS=300

# Authentication
API_KEYS=key1,key2,key3
REQUIRE_AUTH_ALL=false
```

### Provider Credentials

```bash
# Twitch (required)
TWITCH_CLIENT_ID=your_client_id
TWITCH_CLIENT_SECRET=your_client_secret
```

### Docker-Specific

When using Docker, environment variables take priority over config file:

```yaml
# docker-compose.yml
environment:
  - HOST=0.0.0.0
  - PORT=8080
  - DATABASE_URL=/data/streams.db
  - TWITCH_CLIENT_ID=${TWITCH_CLIENT_ID}
  - TWITCH_CLIENT_SECRET=${TWITCH_CLIENT_SECRET}
```

## Configuration Patterns

### Pattern 1: File + Environment Secrets

**Best for**: Production deployments

**config.toml** (committed to git):
```toml
[server]
host = "0.0.0.0"
port = 8080

[scheduler]
interval_secs = 300

[store]
backend = "diesel"
database_url = "/data/streams.db"

[providers.twitch]
enabled = true
# Don't put secrets in config file!
```

**.env** (in .gitignore):
```bash
TWITCH_CLIENT_ID=your_id
TWITCH_CLIENT_SECRET=your_secret
API_KEYS=your-secret-key
```

### Pattern 2: All Environment Variables

**Best for**: Docker, Coolify, cloud platforms

```bash
# .env
HOST=0.0.0.0
PORT=8080
STORE_BACKEND=diesel
DATABASE_URL=/data/streams.db
SCRAPE_INTERVAL_SECS=300
TWITCH_CLIENT_ID=your_id
TWITCH_CLIENT_SECRET=your_secret
API_KEYS=your-secret-key
```

No config.toml file needed.

### Pattern 3: Development Config

**Best for**: Local development

**config.dev.toml**:
```toml
[server]
host = "127.0.0.1"  # Localhost only
port = 3000

[auth]
api_keys = ["dev-key"]
require_all = false

[scheduler]
interval_secs = 60  # Fast updates

[store]
backend = "memory"  # No persistence needed

[providers.twitch]
enabled = true
client_id = "dev_client_id"
client_secret = "dev_client_secret"

[providers.youtube]
enabled = false  # Disable for faster dev
```

Usage:
```bash
cargo run -- --config config.dev.toml
```

## Using with Docker

### Option 1: Mount config.toml

```bash
# docker-compose.yml
volumes:
  - ./config.toml:/app/config.toml:ro
  - stream-data:/data
```

### Option 2: Environment variables

```bash
# docker-compose.yml
environment:
  - HOST=0.0.0.0
  - DATABASE_URL=/data/streams.db
  - TWITCH_CLIENT_ID=${TWITCH_CLIENT_ID}
```

### Option 3: Build into image

```dockerfile
FROM ghcr.io/datagutt/stream-aggregator:latest
COPY config.production.toml /app/config.toml
CMD ["stream-aggregator", "--config", "/app/config.toml"]
```

## Verifying Configuration

### Check current configuration

```bash
# Run with verbose logging
RUST_LOG=debug cargo run

# Check which config file is loaded
# Look for: "Loading configuration from file: config.toml"
```

### Test database connection

```bash
# Set DATABASE_URL
export DATABASE_URL=./test.db

# Run application
cargo run

# Check for: "Database connection established"
```

### Validate Twitch credentials

```bash
# Run with Twitch enabled
# Check logs for: "Twitch provider initialized"
# If credentials are invalid: "Failed to authenticate with Twitch"
```

## Troubleshooting

### Config file not found

```
Error: Failed to load config from config.toml: No such file or directory
```

**Solutions:**
- Run from correct directory: `cd /path/to/project`
- Specify full path: `--config /path/to/config.toml`
- Use environment variables instead

### Invalid TOML syntax

```
Error: TOML parse error at line 10, column 5
```

**Solutions:**
- Validate TOML: https://www.toml-lint.com/
- Check quotes, brackets, commas
- Compare with `config.example.toml`

### Database connection failed

```
Error: Failed to connect to database: unable to open database file
```

**Solutions:**
- Check DATABASE_URL path exists
- Ensure parent directory is writable
- For Docker, verify volume is mounted

### Twitch authentication failed

```
Error: Failed to authenticate with Twitch: invalid client
```

**Solutions:**
- Verify TWITCH_CLIENT_ID and TWITCH_CLIENT_SECRET
- Check credentials at: https://dev.twitch.tv/console/apps
- Ensure credentials are not quoted incorrectly

## Best Practices

### Security

1. **Never commit secrets to git**
   - Use `.env` for secrets (add to `.gitignore`)
   - Use environment variables in production
   - Rotate API keys regularly

2. **Use strong API keys**
   ```bash
   # Generate secure keys
   openssl rand -hex 32
   ```

3. **Enable authentication in production**
   ```toml
   [auth]
   api_keys = ["strong-random-key"]
   require_all = true  # Require auth for everything
   ```

### Performance

1. **Adjust scrape interval based on load**
   - More streamers = longer interval
   - Fewer streamers = shorter interval

2. **Limit concurrent tasks**
   ```toml
   [scheduler]
   max_concurrent = 5  # Don't overwhelm APIs
   ```

3. **Use Diesel backend in production**
   ```toml
   [store]
   backend = "diesel"  # Persistent storage
   ```

### Organization

1. **Use different configs per environment**
   - `config.dev.toml` - Development
   - `config.staging.toml` - Staging
   - `config.production.toml` - Production

2. **Document your config**
   - Add comments explaining non-obvious settings
   - Keep `config.example.toml` updated

3. **Version control**
   - Commit: `config.example.toml`, `config.production.example.toml`
   - Ignore: `config.toml`, `.env`

## See Also

- [config.example.toml](../config.example.toml) - Complete configuration example
- [.env.example](../.env.example) - Environment variables template
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Production deployment guide
- [API.md](./API.md) - API documentation
