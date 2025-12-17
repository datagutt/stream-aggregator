# TikTok Platform Provider

Provides integration with TikTok Live using a Node.js bridge wrapper around the [tiktok-live-connector](https://github.com/zerodytrash/TikTok-Live-Connector) package.

## Architecture

Unlike other providers that use HTTP APIs, TikTok requires a WebSocket-based connection to their Webcast push service. Since the best implementation of this protocol is in Node.js (tiktok-live-connector), this provider uses a bridge architecture:

```
┌─────────────────────┐
│   Rust Provider     │
│  (TikTokProvider)   │
└──────────┬──────────┘
           │ JSON over stdin/stdout
           ▼
┌─────────────────────┐
│   Node.js Bridge    │
│    (index.js)       │
└──────────┬──────────┘
           │ WebSocket + HTTP
           ▼
┌─────────────────────┐
│   TikTok APIs       │
│  (Webcast Service)  │
└─────────────────────┘
```

## Prerequisites

- **Node.js**: Version 18.0.0 or higher
- **npm**: For installing dependencies

## Installation

1. Navigate to the bridge directory:
```bash
cd crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge
```

2. Install dependencies:
```bash
npm install
```

This will install `tiktok-live-connector` and its dependencies.

## Usage

### Basic Configuration

The TikTok provider is enabled by default. In your `config.toml`:

```toml
[providers.tiktok]
enabled = true
```

### Advanced Configuration

You can customize the Node.js executable and bridge script paths:

```toml
[providers.tiktok]
enabled = true
node_path = "/usr/bin/node"  # Optional: custom Node.js path
bridge_path = "/custom/path/to/index.js"  # Optional: custom bridge script path
```

### Environment Variables

```bash
# Optional: Custom Node.js path
export NODE_PATH=/usr/bin/node
```

## How It Works

### 1. Process Lifecycle

When the provider is initialized:
1. The Rust code spawns a Node.js child process running `nodejs-bridge/index.js`
2. The bridge process stays alive for the lifetime of the provider
3. Communication happens via JSON-over-stdio (stdin/stdout)

### 2. Communication Protocol

#### Request Format
The Rust provider sends JSON commands to the bridge's stdin:

```json
{"action": "get_room_info", "username": "someuser"}
```

Available actions:
- `get_room_info` - Fetch stream information for a username
- `ping` - Health check

#### Response Format
The bridge responds with JSON on stdout:

```json
{
  "success": true,
  "data": {
    "live": true,
    "name": "someuser",
    "avatar": "https://...",
    "thumbnail_url": "https://...",
    "viewers": 1234,
    "title": "Stream title"
  }
}
```

Error response:
```json
{
  "success": false,
  "error": "Error message"
}
```

### 3. Stream Data Retrieval

The tiktok-live-connector package:
1. Fetches the room ID from TikTok's web interface
2. Connects to TikTok's Webcast push service
3. Retrieves real-time stream information including:
   - Live status
   - Viewer count
   - Stream title
   - User avatar
   - Thumbnail image

## Rate Limiting

The provider is configured with conservative rate limits:

- **Requests per minute**: 30
- **Burst size**: 5

This prevents overwhelming TikTok's services and reduces the chance of being rate-limited.

## Error Handling

The provider handles several error scenarios:

1. **Bridge Not Found**: If the Node.js bridge script doesn't exist
2. **Node.js Not Found**: If Node.js is not installed or not in PATH
3. **Bridge Crash**: If the Node.js process exits unexpectedly
4. **Parse Errors**: If the bridge returns invalid JSON
5. **TikTok API Errors**: If TikTok rejects the request

All errors are propagated as `ProviderError` variants.

## Health Checks

The provider implements health checks via the `ping` action:

```rust
let status = provider.health_check().await;
match status {
    HealthStatus::Healthy => println!("TikTok provider is working"),
    HealthStatus::Unhealthy => println!("TikTok provider has issues"),
}
```

## Troubleshooting

### Bridge Process Won't Start

**Problem**: Error about bridge not found

**Solution**: Make sure you ran `npm install` in the `nodejs-bridge` directory

```bash
cd crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge
npm install
```

### Node.js Not Found

**Problem**: Error about Node.js executable not found

**Solutions**:
1. Install Node.js 18+ from https://nodejs.org
2. Add Node.js to your PATH
3. Or specify custom path in config:

```toml
[providers.tiktok]
node_path = "/path/to/node"
```

### Bridge Returns Errors

**Problem**: Bridge successfully starts but returns errors for all requests

**Possible causes**:
1. TikTok changed their API (tiktok-live-connector needs update)
2. Network issues preventing connection to TikTok
3. Rate limiting from TikTok

**Solutions**:
1. Update tiktok-live-connector: `cd nodejs-bridge && npm update`
2. Check network connectivity
3. Reduce request frequency

### Process Keeps Crashing

**Problem**: Node.js bridge process repeatedly crashes

**Debug steps**:
1. Test the bridge manually:
```bash
cd nodejs-bridge
node index.js
# Then type: {"action":"ping"}
```

2. Check Node.js version:
```bash
node --version  # Should be >= 18.0.0
```

3. Check dependencies:
```bash
npm list
```

## Development

### Testing the Bridge Manually

You can test the bridge directly:

```bash
cd nodejs-bridge
node index.js
```

Then send JSON commands via stdin:
```
{"action":"ping"}
{"action":"get_room_info","username":"some_tiktok_username"}
```

### Debugging

To see stderr output from the bridge, modify the Rust provider to not suppress stderr:

In `client.rs`, change:
```rust
.stderr(Stdio::null())  // Change this
```

to:
```rust
.stderr(Stdio::inherit())  // To this
```

Then you'll see console.error() output from the Node.js bridge.

## Limitations

1. **Node.js Dependency**: Requires Node.js runtime to be installed
2. **Process Overhead**: Spawns an additional process per provider instance
3. **No WebSocket Events**: Currently only fetches current state, doesn't subscribe to real-time events
4. **Username Only**: Requires TikTok username, not user ID

## Future Improvements

Potential enhancements:

1. **Event Streaming**: Subscribe to live chat, gifts, viewer joins/leaves
2. **Connection Pooling**: Share a single bridge process across multiple provider instances
3. **Caching**: Cache room info to reduce requests to TikTok
4. **Pure Rust**: Reimplement the WebSocket protocol in pure Rust (complex)

## License

Same as the main StreamAggregator project (AGPLv3).

## Credits

- [tiktok-live-connector](https://github.com/zerodytrash/TikTok-Live-Connector) by zerodytrash
- Original lsnd implementation from the Node.js version of this project
