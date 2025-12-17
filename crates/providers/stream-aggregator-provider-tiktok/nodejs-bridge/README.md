# TikTok Bridge

Node.js bridge for communicating with the TikTok Live Connector library.

## Installation

```bash
npm install
```

## Usage

The bridge communicates via JSON over stdin/stdout. Send commands as JSON lines and receive responses as JSON lines.

### Commands

#### Get Room Info
```json
{"action": "get_room_info", "username": "someuser"}
```

Response:
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

#### Ping
```json
{"action": "ping"}
```

Response:
```json
{"success": true, "pong": true}
```

## Protocol

- Each command is a single line of JSON
- Each response is a single line of JSON
- Errors are reported with `"success": false` and an `"error"` field
- The bridge runs until stdin is closed

## Testing

```bash
node index.js
```

Then type commands like:
```
{"action":"ping"}
{"action":"get_room_info","username":"someuser"}
```
