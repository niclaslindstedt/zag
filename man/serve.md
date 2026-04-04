# zag serve

Start the zag HTTP/WebSocket server for remote access.

## Synopsis

    zag serve [flags]

## Description

Starts an HTTP and WebSocket server that exposes zag's orchestration API over the network. This enables remote clients (mobile apps, other machines) to spawn and monitor agent sessions on the host machine.

The server requires authentication via a bearer token on all endpoints except `/api/v1/health`.

## Flags

    --host <HOST>            Bind address (default: 0.0.0.0)
    --port <PORT>            Port to listen on (default: 2100)
    --token <TOKEN>          Authentication token (or set ZAG_SERVE_TOKEN env var)
    --generate-token         Generate a new random token, save it, and start
    --tls-cert <PATH>        TLS certificate file (PEM format)
    --tls-key <PATH>         TLS private key file (PEM format)

## Authentication

Every request (except `GET /api/v1/health`) must include an `Authorization: Bearer <token>` header. The token can be provided via:

1. `--token` flag
2. `ZAG_SERVE_TOKEN` environment variable
3. Saved in `~/.zag/serve.toml` (written by `--generate-token`)

## TLS

For encrypted connections, provide both `--tls-cert` and `--tls-key` pointing to PEM files. When TLS is enabled, the server listens on HTTPS. Without TLS flags, the server uses plain HTTP (recommended only behind a VPN or reverse proxy).

## REST API Endpoints

    GET  /api/v1/health                 Health check (no auth)
    GET  /api/v1/sessions               List sessions
    GET  /api/v1/sessions/:id           Get session details
    GET  /api/v1/sessions/:id/status    Get session status
    GET  /api/v1/sessions/:id/events    Query session events
    GET  /api/v1/sessions/:id/output    Get final result text
    POST /api/v1/sessions/spawn         Spawn a background session
    POST /api/v1/sessions/:id/input     Send a user message
    POST /api/v1/sessions/:id/cancel    Cancel a session
    POST /api/v1/sessions/collect       Collect results from sessions
    POST /api/v1/sessions/wait          Wait for sessions to complete
    GET  /api/v1/processes              List processes

## WebSocket Endpoints

    WS /api/v1/sessions/:id/stream     Real-time event stream for one session
    WS /api/v1/subscribe               Multiplexed event stream across sessions

## Examples

    # Generate a token and start the server
    zag serve --generate-token

    # Start with a specific token on a custom port
    zag serve --token mysecrettoken --port 8080

    # Start with TLS
    zag serve --token mysecrettoken --tls-cert cert.pem --tls-key key.pem

## Configuration

Server defaults can be saved in `~/.zag/serve.toml`:

    [server]
    host = "0.0.0.0"
    port = 2100
    token = "..."
    tls_cert = "/path/to/cert.pem"
    tls_key = "/path/to/key.pem"

## See Also

`zag connect`, `zag disconnect`
