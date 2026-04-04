# zag serve

Start the zag HTTPS/WebSocket server for remote access.

## Synopsis

    zag serve [flags]

## Description

Starts an HTTPS and WebSocket server that exposes zag's orchestration API over the network. This enables remote clients (mobile apps, other machines) to spawn and monitor agent sessions on the host machine.

The server always uses TLS (HTTPS). If no TLS certificate is provided, a self-signed certificate is automatically generated and saved to `~/.zag/tls/`. For production deployments, provide your own certificate via `--tls-cert` and `--tls-key`.

The server requires authentication via a bearer token on all endpoints except `/api/v1/health`.

## Flags

    --host <HOST>            Bind address (default: 0.0.0.0)
    --port <PORT>            Port to listen on (default: 2100)
    --token <TOKEN>          Authentication token (or set ZAG_SERVE_TOKEN env var)
    --generate-token         Generate a new random token, save it, and start
    --tls-cert <PATH>        TLS certificate file (PEM format); overrides auto-generated cert
    --tls-key <PATH>         TLS private key file (PEM format); overrides auto-generated cert

## Authentication

Every request (except `GET /api/v1/health`) must include an `Authorization: Bearer <token>` header. The token is resolved in this order:

1. `--token` flag
2. `ZAG_SERVE_TOKEN` environment variable
3. Saved in `~/.zag/serve.toml` (written by `--generate-token` or auto-generated)

If no token is found from any source, one is automatically generated, saved to `~/.zag/serve.toml`, and printed to stderr.

## TLS

The server always uses HTTPS. TLS certificates are resolved in this order:

1. `--tls-cert` and `--tls-key` flags (must be provided together)
2. `tls_cert` and `tls_key` in `~/.zag/serve.toml`
3. Auto-generated self-signed certificate (saved to `~/.zag/tls/`)

When using auto-generated certificates, a warning is printed. Self-signed certificates are suitable for local networks and development but should not be used in production.

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

    # Start with auto-generated certificate and token (simplest)
    zag serve

    # Start with a specific token
    zag serve --token mysecrettoken

    # Start with custom TLS certificate
    zag serve --tls-cert cert.pem --tls-key key.pem

    # Start on a custom port
    zag serve --port 8080

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
