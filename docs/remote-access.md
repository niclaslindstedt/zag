# Remote Access

zag can expose its orchestration API over HTTPS and WebSocket, letting you control agents from remote machines, mobile devices, or other applications.

## Architecture

The remote access system has two parts:

1. **Server** (`zag serve`) -- runs on a machine with agent CLIs installed, exposes a REST + WebSocket API
2. **Client** (`zag connect`) -- connects your local CLI to a remote server; subsequent commands proxy transparently

## Starting a server

```bash
zag serve
```

By default, the server listens on `0.0.0.0:2100` with HTTPS (self-signed certificate) and auto-generated bearer token authentication.

### Options

```bash
# Custom port
zag serve --port 8443

# Explicit auth token
zag serve --token my-secret-token

# Generate a new token
zag serve --generate-token

# Custom TLS certificates
zag serve --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem
```

### Server config

Server settings can be persisted in `~/.zag/serve.toml`:

```toml
host = "0.0.0.0"
port = 2100
token = "your-token"
tls_cert = "/path/to/cert.pem"
tls_key = "/path/to/key.pem"
```

### TLS

The server always uses HTTPS. If no custom certificates are provided, self-signed certificates are auto-generated and stored at `~/.zag/tls/`.

### Authentication

All API endpoints (except `/api/v1/health`) require a bearer token. The token is resolved in order:

1. `--token` flag
2. `ZAG_SERVE_TOKEN` environment variable
3. `~/.zag/serve.toml` config file

## Connecting to a server

From another machine (or the same machine):

```bash
zag connect https://server.example.com:2100 --token my-secret-token
```

The URL auto-prepends `https://` if no scheme is provided. Connection state is stored in `~/.zag/connect.json`.

The token can also be provided via the `ZAG_CONNECT_TOKEN` environment variable.

## Transparent proxying

Once connected, most commands automatically proxy through the remote server:

```bash
zag connect myserver:2100 --token abc123

# These now run on the remote machine
zag spawn -p claude "analyze the codebase"
zag status "$sid"
zag listen "$sid"
zag session list
```

### Local-only commands

These commands always run locally, even when connected:

- `serve`, `connect`, `disconnect`
- `run`, `exec` (direct agent execution)
- `config`

## REST API

The server exposes these REST endpoints under `/api/v1/`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check (no auth required) |
| `/sessions` | GET | List sessions |
| `/sessions` | POST | Spawn a new session |
| `/sessions/:id/events` | GET | Query session events |
| `/sessions/:id/output` | GET | Get session output |
| `/sessions/:id/cancel` | POST | Cancel a session |
| `/processes` | GET | List running processes |

## WebSocket API

Real-time streaming is available via WebSocket:

- `/api/v1/sessions/:id/stream` -- stream events from a single session
- `/api/v1/subscribe` -- multiplexed event stream from all sessions

## Disconnecting

```bash
zag disconnect
```

This removes the connection state from `~/.zag/connect.json`. Commands return to local execution.

## Swift binding: remote mode

The Swift binding supports remote mode natively, enabling iOS and macOS apps to control agents without a local CLI binary:

```swift
let builder = ZagBuilder()
    .remote(url: "https://server:2100", token: "abc123")
    .provider("claude")
    .model("sonnet")
let output = try await builder.exec("analyze this code")
```

See [Language Bindings](language-bindings.md) for details.

## Related

- [Orchestration](orchestration.md) -- Commands that work over remote
- [Sessions](sessions.md) -- Remote session management
- `zag man serve` -- Server command reference
- `zag man connect` -- Connect command reference
