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
[server]
host = "0.0.0.0"
port = 2100
token = "your-token"
tls_cert = "/path/to/cert.pem"
tls_key = "/path/to/key.pem"
# Force every connected user's agent sessions to run inside a Docker sandbox,
# regardless of what the client asked for. Useful when exposing zag on the
# public internet.
force_sandbox = false
```

The `--force-sandbox` flag on `zag serve` overrides the config value at the
command line.

### TLS

The server always uses HTTPS. If no custom certificates are provided, self-signed certificates are auto-generated and stored at `~/.zag/tls/`.

### Authentication

zag supports two authentication modes, which can be used in combination:

1. **User accounts** — when `~/.zag/users.json` exists, the server requires
   clients to log in via `POST /api/v1/login` with a username and password.
   On success it hands out a session token that is validated on every
   subsequent request; each authenticated request also carries the user's
   home directory so the server can chroot agent sessions into that
   directory. Manage accounts with `zag user add | remove | list | passwd`
   (see below).
2. **Legacy single token** — a shared bearer token loaded from the
   `--token` flag, the `ZAG_SERVE_TOKEN` environment variable, or
   `~/.zag/serve.toml` (in that precedence order). The legacy token always
   acts as a **super token**: even when user accounts are configured, the
   legacy token bypasses per-user restrictions and is the only credential
   that can call the user-management endpoints.

All API endpoints except `/api/v1/health` and `/api/v1/login` require a
valid token (either a user session token or the legacy/super token).

### Managing user accounts

```bash
# Create a user locked to a specific home directory
zag user add -u alice --home-dir /srv/zag-users/alice

# List all accounts
zag user list --json

# Change a password
zag user passwd alice

# Remove an account
zag user remove alice
```

Clients then authenticate with `zag connect`:

```bash
# User-account mode (prompts for password if not passed)
zag connect https://server.example.com:2100 -u alice

# Legacy super-token mode
zag connect https://server.example.com:2100 --token my-secret-token
```

Passwords are hashed with bcrypt and stored in `~/.zag/users.json`; session
tokens are stored under `~/.zag/tokens.json`.

## Connecting to a server

From another machine (or the same machine):

```bash
zag connect https://server.example.com:2100 --token my-secret-token
```

The URL auto-prepends `https://` if no scheme is provided. Connection state is stored in `~/.zag/connect.json`.

The token can also be provided via the `ZAG_CONNECT_TOKEN` environment variable.

## Health check and auto-disconnect

Every proxied command runs a cheap `GET /api/v1/health` probe before it
forwards the actual request. If the probe fails (network unreachable, TLS
mismatch, server not running), the CLI automatically drops the stored
connection and re-runs the command locally, so a stale `~/.zag/connect.json`
doesn't lock you out of your own machine. Health check results are cached
in `~/.zag/health_cache` for a short TTL so back-to-back commands don't
incur per-call HTTPS overhead.

Pass `--no-health-check` on any command to skip the probe when you know the
server is reachable and want to shave the latency.

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
| `/login` | POST | Exchange username/password for a session token (no auth) |
| `/logout` | POST | Invalidate the current session token |
| `/sessions` | GET | List sessions |
| `/sessions/spawn` | POST | Spawn a new session |
| `/sessions/collect` | POST | Collect results from multiple sessions |
| `/sessions/wait` | POST | Wait for a set of sessions to finish |
| `/sessions/summary` | POST | Summarize multiple sessions |
| `/sessions/retry` | POST | Re-run failed sessions |
| `/sessions/pipe` | POST | Pipe session results into a new session |
| `/sessions/broadcast` | POST | Broadcast a message to matching sessions |
| `/sessions/:id` | GET / DELETE / PATCH | Show, delete, or update a session |
| `/sessions/:id/status` | GET | Current session status |
| `/sessions/:id/events` | GET | Query session events |
| `/sessions/:id/output` | GET | Get the session's final output |
| `/sessions/:id/cancel` | POST | Cancel a running session |
| `/sessions/:id/input` | POST | Send a user message to an interactive session |
| `/sessions/:id/log` | POST | Append a custom event to a session log |
| `/sessions/:id/env` | GET | Export session environment variables |
| `/search` | POST | Search session logs |
| `/gc` | POST | Run garbage collection |
| `/review` | POST | Run a code review |
| `/config` | POST | Read or write config values |
| `/capability` | GET | Provider capability reports |
| `/skills` | POST | Skills management |
| `/mcp` | POST | MCP server management |
| `/users` | GET | List user accounts (super token only) |
| `/users/add` | POST | Add a user (super token only) |
| `/users/remove` | POST | Remove a user (super token only) |
| `/users/passwd` | POST | Change a user password (super token only) |
| `/processes` | GET | List running processes |
| `/processes/:id` | GET | Show a process |
| `/processes/:id/stop` | POST | Stop a process gracefully |
| `/processes/:id/kill` | POST | Force-kill a process |

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
- `zag man user` -- User account management reference
