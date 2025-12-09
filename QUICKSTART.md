# StreamAggregator - Quick Start Guide

## Prerequisites

- Rust 1.75+ (install from https://rustup.rs)
- Twitch API credentials (get from https://dev.twitch.tv/console/apps)

## Getting Started

### 1. Get Twitch API Credentials

1. Go to https://dev.twitch.tv/console/apps
2. Click "Register Your Application"
3. Fill in:
   - Name: "My StreamAggregator"
   - OAuth Redirect URLs: `http://localhost`
   - Category: "Application Integration"
4. Copy your **Client ID** and **Client Secret**

### 2. Run the Server

#### Option A: Using Environment Variables

```bash
# Set your Twitch credentials
export TWITCH_CLIENT_ID="your_client_id_here"
export TWITCH_CLIENT_SECRET="your_client_secret_here"

# Run the server
cargo run --release
```

#### Option B: Using Command Line Arguments

```bash
cargo run --release -- \
  --twitch-client-id "your_client_id_here" \
  --twitch-client-secret "your_client_secret_here"
```

#### Option C: With API Key Authentication

```bash
# Protect write operations with API keys
export TWITCH_CLIENT_ID="your_client_id_here"
export TWITCH_CLIENT_SECRET="your_client_secret_here"
export API_KEYS="secret-key-123,another-key-456"

cargo run --release
```

### 3. Test the API

Once the server is running (default: http://127.0.0.1:8080), try these commands:

#### Health Check
```bash
curl http://127.0.0.1:8080/health
```

#### List Platforms
```bash
curl http://127.0.0.1:8080/api/v1/platforms
```

#### Add a Streamer to Track
```bash
# Without authentication (if no API keys set)
curl -X POST http://127.0.0.1:8080/api/v1/streamers \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "twitch",
    "user_id": "ninja"
  }'

# With authentication (if API keys are set)
curl -X POST http://127.0.0.1:8080/api/v1/streamers \
  -H "Content-Type: application/json" \
  -H "X-API-Key: secret-key-123" \
  -d '{
    "platform": "twitch",
    "user_id": "ninja"
  }'
```

#### List Tracked Streamers
```bash
curl http://127.0.0.1:8080/api/v1/streamers
```

#### Get Stream Information
```bash
# This will fetch live data from Twitch
curl http://127.0.0.1:8080/api/v1/streams
```

## Configuration Options

All options can be set via environment variables or command line arguments:

| Environment Variable | CLI Argument | Default | Description |
|---------------------|--------------|---------|-------------|
| `HOST` | `--host` | `127.0.0.1` | Server host to bind to |
| `PORT` | `--port` | `8080` | Server port |
| `RUST_LOG` | `--log-level` | `info` | Log level (trace, debug, info, warn, error) |
| `API_KEYS` | `--api-keys` | None | Comma-separated API keys for authentication |
| `REQUIRE_AUTH_ALL` | `--require-auth-all` | `false` | Require auth for all requests (including reads) |
| `TWITCH_CLIENT_ID` | `--twitch-client-id` | None | Twitch application client ID |
| `TWITCH_CLIENT_SECRET` | `--twitch-client-secret` | None | Twitch application client secret |

## Usage Examples

### Development Mode (Public Access)

```bash
# Anyone can read and write
export TWITCH_CLIENT_ID="..."
export TWITCH_CLIENT_SECRET="..."

cargo run --release
```

### Production Mode (Protected Writes)

```bash
# Public can read (GET /streams), but writes require API key
export TWITCH_CLIENT_ID="..."
export TWITCH_CLIENT_SECRET="..."
export API_KEYS="my-secret-key"

cargo run --release
```

### Strict Mode (All Requests Require Auth)

```bash
# All requests require API key
export TWITCH_CLIENT_ID="..."
export TWITCH_CLIENT_SECRET="..."
export API_KEYS="my-secret-key"
export REQUIRE_AUTH_ALL="true"

cargo run --release
```

### Custom Port and Host

```bash
# Bind to all interfaces on port 3000
export HOST="0.0.0.0"
export PORT="3000"
export TWITCH_CLIENT_ID="..."
export TWITCH_CLIENT_SECRET="..."

cargo run --release
```

## Frontend Integration

For a public-facing frontend that needs to fetch streams client-side:

1. **Don't set API_KEYS** - This allows public read access
2. Fetch streams from the public API:

```javascript
// Fetch all live streams
fetch('http://your-server:8080/api/v1/streams?live=true')
  .then(res => res.json())
  .then(data => {
    console.log('Live streams:', data.data);
  });

// Fetch tracked streamers
fetch('http://your-server:8080/api/v1/streamers')
  .then(res => res.json())
  .then(data => {
    console.log('Tracked streamers:', data.data);
  });
```

If you want to protect write operations but keep reads public:

```bash
# Set API keys - reads are still public, writes require the key
export API_KEYS="server-only-secret"
```

Then only server-side operations (adding/removing streamers) need the API key:

```javascript
// Server-side only (requires API key)
fetch('http://your-server:8080/api/v1/streamers', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'X-API-Key': process.env.API_KEY  // Never expose this to client!
  },
  body: JSON.stringify({
    platform: 'twitch',
    user_id: 'ninja'
  })
});
```

## API Endpoints

### Public Endpoints (Always Accessible)
- `GET /health` - Health check
- `GET /api/v1/health` - Health check (alias)

### Read Endpoints (Public by default, unless `--require-auth-all` is set)
- `GET /api/v1/streams` - List all streams
- `GET /api/v1/streams/:id` - Get stream by ID
- `GET /api/v1/streamers` - List tracked streamers
- `GET /api/v1/platforms` - List supported platforms

### Write Endpoints (Require API key if configured)
- `POST /api/v1/streamers` - Add streamer to track
- `DELETE /api/v1/streamers/:platform/:user_id` - Remove tracked streamer

## Building for Production

```bash
# Build optimized release binary
cargo build --release

# Binary will be at: target/release/stream-aggregator

# Run the binary directly
./target/release/stream-aggregator \
  --twitch-client-id "..." \
  --twitch-client-secret "..." \
  --host "0.0.0.0" \
  --port "8080"
```

## Troubleshooting

### "No providers configured" Error

Make sure you've set your Twitch credentials:
```bash
export TWITCH_CLIENT_ID="your_client_id"
export TWITCH_CLIENT_SECRET="your_client_secret"
```

### "Twitch provider health check failed"

Your credentials may be invalid. Double-check:
1. Client ID and Secret are correct
2. You're using the right credentials (not OAuth tokens)
3. Your app is registered at https://dev.twitch.tv/console/apps

### Port Already in Use

Change the port:
```bash
cargo run --release -- --port 3000
```

## Next Steps

- See `docs/API.md` for complete API documentation
- See `docs/ARCHITECTURE.md` for system architecture
- Add more streamers to track via the API
- Integrate with your frontend application

## Support

For issues or questions, please open an issue on GitHub.
