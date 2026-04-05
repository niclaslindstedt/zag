# Skills and MCP Servers

zag provides two extension mechanisms for enhancing agent capabilities: **skills** (reusable instruction sets) and **MCP servers** (tool providers via the Model Context Protocol).

## Skills

Skills are provider-agnostic instruction files that teach agents domain-specific knowledge or workflows. They are stored centrally and synced to each provider's native skill format.

### Storage

Skills live at `~/.zag/skills/<skill-name>/` with this structure:

```
~/.zag/skills/my-skill/
  SKILL.md          # Required: skill definition
  scripts/          # Optional: helper scripts
  references/       # Optional: reference materials
  assets/           # Optional: other files
```

### Skill format

A `SKILL.md` file contains YAML frontmatter followed by markdown content:

```markdown
---
name: my-skill
description: "A short description of what this skill does"
---

# My Skill

Instructions and knowledge for the agent...
```

### Managing skills

```bash
# List all skills
zag skills list

# Show a skill's content
zag skills show my-skill

# Add a new skill from a directory or file
zag skills add my-skill /path/to/skill/

# Remove a skill
zag skills remove my-skill

# Sync skills to all provider-native locations
zag skills sync
```

### Provider sync

When you run `zag skills sync`, zag symlinks each skill to provider-native locations with a `zag-` prefix:

| Provider | Sync location |
|----------|--------------|
| Claude | `~/.claude/skills/zag-<name>/` |
| Gemini | Provider settings |
| Copilot | Provider instructions |
| Codex | Provider config |
| Ollama | System prompt injection (no native skill support) |

The `zag-` prefix prevents name collisions with provider-native skills.

### Importing skills

Import existing provider-native skills into zag's unified store:

```bash
zag skills import --provider claude
```

This copies the skill content and tracks the source via `.import-metadata.json` (source provider, content hash, timestamp) so future imports can detect changes.

## MCP Servers

The [Model Context Protocol](https://modelcontextprotocol.io) (MCP) lets agents access external tools and data sources through a standardized interface. zag manages MCP server configurations and syncs them to providers that support MCP.

### Storage

MCP server configs are TOML files stored at two scopes:

| Scope | Path |
|-------|------|
| Global | `~/.zag/mcp/<server-name>.toml` |
| Project-scoped | `~/.zag/projects/<sanitized-path>/mcp/<server-name>.toml` |

Project-scoped servers override global ones with the same name.

### Server configuration

Each MCP server config supports two transport types:

**stdio** -- runs a local process:

```toml
name = "my-server"
description = "My custom tool server"
transport = "stdio"
command = "npx"
args = ["-y", "my-mcp-server"]

[env]
MY_API_KEY = "..."
```

**http** -- connects to a remote server:

```toml
name = "remote-tools"
description = "Remote tool server"
transport = "http"
url = "https://tools.example.com/mcp"
bearer_token_env_var = "MCP_TOKEN"

[headers]
X-Custom-Header = "value"
```

### Managing MCP servers

```bash
# List all configured servers
zag mcp list

# Show a server's config
zag mcp show my-server

# Add a server from a TOML file
zag mcp add my-server /path/to/config.toml

# Remove a server
zag mcp remove my-server

# Sync all configs to provider-native locations
zag mcp sync
```

### Provider sync

`zag mcp sync` injects server configs with a `zag-` prefix into each provider's native config format:

| Provider | Config location | Format |
|----------|----------------|--------|
| Claude | `~/.claude.json` | JSON |
| Gemini | `~/.gemini/settings.json` | JSON |
| Copilot | `~/.copilot/mcp-config.json` | JSON |
| Codex | `~/.codex/config.toml` | TOML |
| Ollama | Not supported | -- |

### Importing MCP servers

Import existing provider-native MCP configurations:

```bash
zag mcp import --provider claude
```

### Provider support

| Provider | MCP support |
|----------|------------|
| Claude | Yes |
| Codex | Yes |
| Gemini | Yes |
| Copilot | Yes |
| Ollama | No |

### Using MCP servers in sessions

Pass MCP config directly to a session:

```bash
zag exec --mcp-config /path/to/config.toml "use the tools to analyze the data"
```

Or rely on synced configs -- providers automatically pick up their native MCP configurations.

## Related

- [Providers](providers.md) -- MCP support per provider
- [Configuration](configuration.md) -- Config file paths and precedence
- `zag man skills` -- Skills command reference
- `zag man mcp` -- MCP command reference
