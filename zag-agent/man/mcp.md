# zag mcp

Manage MCP (Model Context Protocol) servers across providers.

## Synopsis

    zag mcp <command> [options]

## Description

MCP servers extend agent capabilities with external tools and data sources via the Model Context Protocol. Zag stores MCP server configurations as individual TOML files and syncs them into each provider's native config format.

Servers are stored at:
- **Global**: `~/.zag/mcp/<server-name>.toml`
- **Project-scoped**: `~/.zag/projects/<sanitized-path>/mcp/<server-name>.toml`

Project-scoped servers override global servers with the same name.

## Server Format

Each server is a single TOML file:

```toml
# ~/.zag/mcp/github.toml
name = "github"
description = "GitHub MCP server"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"
```

HTTP transport example:

```toml
# ~/.zag/mcp/sentry.toml
name = "sentry"
transport = "http"
url = "https://mcp.sentry.dev/sse"
bearer_token_env_var = "SENTRY_AUTH_TOKEN"
```

## Provider Integration

During sync, servers are injected with a `zag-` prefix into each provider's native config. User-managed entries are never modified.

| Provider | Config File | Format |
|----------|-----------|--------|
| Claude   | `~/.claude.json` | JSON `mcpServers` |
| Gemini   | `~/.gemini/settings.json` | JSON `mcpServers` |
| Copilot  | `~/.copilot/mcp-config.json` | JSON `mcpServers` |
| Codex    | `~/.codex/config.toml` | TOML `[mcp_servers]` |
| Ollama   | N/A | Not supported |

## Commands

### list

List all configured MCP servers (global + project-scoped).

    zag mcp list [--json] [-r root]

### show

Show details of a specific MCP server.

    zag mcp show <name> [--json] [-r root]

### add

Add a new MCP server configuration.

    zag mcp add <name> [options]

Options:
- `--transport` — Transport type: `stdio` or `http` (default: stdio)
- `--command` — Command to start the server (stdio)
- `--args` — Arguments for the command
- `--url` — URL endpoint (http)
- `--env KEY=VALUE` — Environment variables (repeatable)
- `--description` — Short description
- `--global` — Store in global directory (`~/.zag/mcp/`) instead of project-scoped

### remove

Remove an MCP server and clean up all provider configs.

    zag mcp remove <name>

### sync

Sync MCP servers to all provider-specific configs. Runs automatically before each agent session.

    zag mcp sync [-p provider]

Options:
- `-p, --provider` — Only sync for this provider (claude, gemini, copilot, codex)

### import

Import MCP servers from a provider's native config into `~/.zag/mcp/`.

    zag mcp import [--from <provider>]

Options:
- `--from` — Provider to import from (default: claude). Skips entries prefixed with `zag-`.

## Examples

    # Add a stdio MCP server
    zag mcp add github --command npx --args -y @modelcontextprotocol/server-github

    # Add with environment variables
    zag mcp add github --command npx --args -y @modelcontextprotocol/server-github --env GITHUB_TOKEN='${GITHUB_TOKEN}'

    # Add an HTTP MCP server
    zag mcp add sentry --transport http --url https://mcp.sentry.dev/sse

    # Add a global server
    zag mcp add my-db --command npx --args -y db-mcp --global

    # List all servers
    zag mcp list

    # List as JSON
    zag mcp list --json

    # Show a specific server
    zag mcp show github

    # Manually sync to all providers
    zag mcp sync

    # Only sync to Claude
    zag mcp sync -p claude

    # Import your existing Claude MCP servers
    zag mcp import --from claude

    # Import from Codex
    zag mcp import --from codex

    # Remove a server
    zag mcp remove github

## See Also

    zag man zag       Global flags and providers overview
    zag man skills    Manage provider-agnostic skills
    zag man run       Start an interactive session (MCP servers are synced automatically)
