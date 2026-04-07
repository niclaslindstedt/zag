# zag connect

Connect to a remote zag server.

## Synopsis

    zag connect <url> [flags]

## Description

Connects to a remote zag server and saves the connection configuration. Once connected, all subsequent zag commands transparently proxy through the remote server until `zag disconnect` is called.

If the URL does not include a scheme, `https://` is automatically prepended.

The connection state is stored in `~/.zag/connect.json`. While connected, commands like `zag spawn`, `zag status`, `zag listen`, `zag session list`, and others are forwarded to the remote server's REST/WebSocket API.

## Arguments

    url    Server URL (e.g., home.local:2100 or https://home.local:2100)

## Flags

    --token <TOKEN>    Authentication token (or set ZAG_CONNECT_TOKEN env var)

## Health Check

Before proxying each command, zag pings the remote server's health endpoint to verify it is reachable. If the server is unreachable, zag automatically disconnects and runs the command locally, printing a warning to stderr. The health check result is cached for 30 seconds to avoid adding latency on every invocation.

To disable this behavior, use the `--no-health-check` global flag or set the `ZAG_NO_HEALTH_CHECK=1` environment variable:

    zag --no-health-check status <session-id>

## Commands That Work Remotely

    zag spawn          Spawn a session on the remote machine
    zag status         Check session status
    zag events         Query session events
    zag listen         Stream session events in real-time (via WebSocket)
    zag subscribe      Subscribe to all session events
    zag cancel         Cancel a session
    zag collect        Collect results
    zag wait           Wait for sessions to complete
    zag output         Get session output
    zag session list   List sessions
    zag session show   Show session details
    zag ps             List processes
    zag input          Send a message to a session

## Commands That Always Run Locally

    zag serve          Start a local server
    zag connect        Connect to a server
    zag disconnect     Disconnect from a server
    zag run            Interactive sessions (local only)
    zag exec           Non-interactive execution (local only)
    zag config         Local configuration

## Examples

    # Connect to a remote server (https:// auto-prepended)
    zag connect home.local:2100 --token mysecrettoken

    # Connect with explicit scheme
    zag connect https://home.local:2100 --token mysecrettoken

    # Now all commands proxy through the remote server
    zag spawn "write tests for auth module"
    zag listen --latest
    zag session list

    # Spawn an interactive session on the remote machine
    sid=$(zag spawn --interactive --name worker -p claude)
    zag input --name worker "analyze the auth module"
    zag listen --name worker

    # Disconnect when done
    zag disconnect

## See Also

`zag serve`, `zag disconnect`
