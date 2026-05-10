---
name: update-docs
description: "Use when docs may be stale. Discovers commits since the last docs update, identifies what changed (providers, orchestration, sessions, config, events, isolation, skills, MCP, remote access, bindings, etc.), and updates the affected docs/*.md files to match the current implementation."
---

# Updating the Docs

The `docs/` directory contains conceptual documentation for zag's major features. Unlike man pages (command-level reference) or the README (overview), docs/ files explain concepts in depth with examples and cross-references. They get stale when features, providers, commands, or behaviors change without corresponding docs updates.

## Current Docs

| File | Covers |
|------|--------|
| `getting-started.md` | Installation, first steps, basic examples, next-steps links |
| `configuration.md` | TOML config system, keys, precedence, env vars |
| `providers.md` | Provider comparison, feature matrix, model lists, auto-selection |
| `events-and-logging.md` | Event types, output formats, session logs, streaming |
| `troubleshooting.md` | Common errors and solutions |
| `orchestration.md` | Multi-agent coordination patterns, primitives, DAG workflows |
| `sessions.md` | Session lifecycle, naming, tagging, resume, dependencies |
| `isolation.md` | Worktree and sandbox isolation modes |
| `skills-and-mcp.md` | Skills management and MCP server configuration |
| `remote-access.md` | serve/connect for remote operation |
| `language-bindings.md` | SDK bindings for 7 languages |

## Tracking Mechanism

The file `.claude/skills/update-docs/.last-updated` contains the git commit hash from the last time the docs were comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-docs/.last-updated)
   ```

2. List all commits since the baseline:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. Check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the docs mapping below to determine which docs need updating.

5. Read the affected docs and the corresponding source-of-truth files. Fix any discrepancies.

## Docs Mapping

Use this table to map changed files/scopes to affected docs:

| Changed files / commit scope | Doc(s) to update |
|------------------------------|-----------------|
| `zag-agent/src/providers/*/mod.rs` (models, features) | `providers.md` (model lists, feature matrix), `getting-started.md` (install commands) |
| `zag-agent/src/providers/*/mod.rs` (new provider) | `providers.md`, `getting-started.md`, `isolation.md` (support matrix), `skills-and-mcp.md` (sync locations), `remote-access.md` |
| `zag-agent/src/builder.rs` (new option) | `language-bindings.md` (builder methods table) |
| `zag-agent/src/config.rs` | `configuration.md` (config keys, defaults, TOML reference) |
| `zag-agent/src/output.rs` (event types) | `events-and-logging.md` (event types, fields) |
| `zag-agent/src/session.rs` | `sessions.md` (session fields, lifecycle) |
| `zag-agent/src/skills.rs` | `skills-and-mcp.md` (skills section) |
| `zag-agent/src/mcp.rs` | `skills-and-mcp.md` (MCP section) |
| `zag-agent/src/sandbox.rs` | `isolation.md` (sandbox templates, behavior) |
| `zag-agent/src/session_log.rs` | `events-and-logging.md` (log format, event kinds) |
| `zag-orch/src/` (new primitives) | `orchestration.md` (primitives table, patterns) |
| `zag-orch/src/` (behavior changes) | `orchestration.md`, `sessions.md` |
| `zag-cli/src/cli.rs` (AgentArgs) | `getting-started.md`, `providers.md`, `orchestration.md` |
| `zag-cli/src/cli.rs` (SessionIsolationArgs) | `isolation.md`, `getting-started.md` |
| `zag-cli/src/cli.rs` (SessionMetadataArgs) | `sessions.md` |
| `zag-cli/src/cli.rs` (Commands::Spawn) | `orchestration.md` |
| `zag-cli/src/cli.rs` (Commands::Session) | `sessions.md` |
| `zag-cli/src/cli.rs` (Commands::Config) | `configuration.md` |
| `zag-cli/src/cli.rs` (Commands::Listen/Subscribe/Watch) | `orchestration.md`, `events-and-logging.md` |
| `zag-cli/src/cli.rs` (Commands::Serve) | `remote-access.md` |
| `zag-cli/src/cli.rs` (Commands::Connect) | `remote-access.md` |
| `zag-cli/src/cli.rs` (Commands::Skills) | `skills-and-mcp.md` |
| `zag-cli/src/cli.rs` (Commands::Mcp) | `skills-and-mcp.md` |
| `zag-cli/src/commands/serve.rs` | `remote-access.md` |
| `zag-cli/src/commands/connect.rs` | `remote-access.md` |
| `zag-cli/src/commands/session/` | `sessions.md` |
| `zag-cli/src/commands/skills/` | `skills-and-mcp.md` |
| `zag-cli/src/commands/mcp/` | `skills-and-mcp.md` |
| `bindings/*/src/` (new method) | `language-bindings.md` (builder methods, examples) |
| `bindings/*/README.md` | `language-bindings.md` (install commands, versions) |
| New CLI command | `getting-started.md` (if user-facing), relevant concept doc |
| Installation changes | `getting-started.md` |
| New output format | `events-and-logging.md` |
| New environment variable | `configuration.md` (env vars table), `sessions.md` (if session-related) |

## Implementation Files

### Primary (docs being updated)

- `docs/*.md` -- the 11 documentation files

### Secondary (read-only, sources of truth)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zag-cli/src/cli.rs` | CLI flags, commands, subcommands |
| `zag-agent/src/builder.rs` | Builder options (maps to binding methods) |
| `zag-agent/src/providers/*/mod.rs` | Provider models, defaults, size aliases, features |
| `zag-agent/src/config.rs` | Config keys and defaults |
| `zag-agent/src/output.rs` | Unified event types |
| `zag-agent/src/session.rs` | Session fields and storage |
| `zag-agent/src/skills.rs` | Skills storage and format |
| `zag-agent/src/mcp.rs` | MCP storage and format |
| `zag-agent/src/sandbox.rs` | Sandbox templates and behavior |
| `zag-orch/src/lib.rs` | Orchestration primitives |
| `bindings/*/src/` | Binding implementations |
| `bindings/*/README.md` | Binding install instructions and versions |
| `man/*.md` | Command-level reference (should be consistent) |
| `README.md` | High-level overview (should be consistent) |

## Implementation Patterns

### Adding a new provider

1. Update `providers.md`: add to overview table, model size aliases, feature matrix, available models, known limitations
2. Update `getting-started.md`: add install command
3. Update `isolation.md`: add to provider support matrix
4. Update `skills-and-mcp.md`: add provider sync location and MCP config path
5. Update `remote-access.md` if remote behavior differs

### Updating provider models

1. Update `providers.md`: model size aliases table and available models list
2. Verify examples in other docs still use valid model names

### Adding a new builder option

1. Update `language-bindings.md`: add to builder methods table
2. If the option maps to a CLI flag with conceptual significance, update the relevant concept doc

### Adding a new orchestration primitive

1. Update `orchestration.md`: add to primitives table, add usage examples
2. Update patterns that can use the new primitive

### Changing configuration

1. Update `configuration.md`: config reference, valid keys table, example configs
2. If new env var, add to env vars table in `configuration.md` and `sessions.md` if session-related

### Adding a new event type

1. Update `events-and-logging.md`: add event type with JSON example

### Adding a new isolation mode

1. Update `isolation.md`: add section with how-it-works, requirements, provider support
2. Update `getting-started.md` isolation examples

### Adding a new language binding

1. Update `language-bindings.md`: add to overview table, add quick start example, add binding README link

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read all affected docs and source-of-truth files
- [ ] Update `providers.md` if models, features, or providers changed
- [ ] Update `configuration.md` if config keys, defaults, or env vars changed
- [ ] Update `events-and-logging.md` if event types or output formats changed
- [ ] Update `orchestration.md` if orchestration primitives or patterns changed
- [ ] Update `sessions.md` if session fields, lifecycle, or commands changed
- [ ] Update `isolation.md` if worktree or sandbox behavior changed
- [ ] Update `skills-and-mcp.md` if skills or MCP management changed
- [ ] Update `remote-access.md` if serve/connect behavior changed
- [ ] Update `language-bindings.md` if builder options or bindings changed
- [ ] Update `getting-started.md` if install, first-steps, or links changed
- [ ] Update `troubleshooting.md` if new common errors or log paths changed
- [ ] Verify all cross-links between docs are valid
- [ ] Verify examples use correct current syntax
- [ ] Consider whether `update-readme` and `update-manpages` skills should also be run
- [ ] Update `.claude/skills/update-docs/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-docs/.last-updated
  ```

## Verification

1. Read each updated doc and verify facts against source code
2. Check all internal cross-links (relative markdown links between docs)
3. Verify provider-specific details match the provider source files
4. Ensure no sections were accidentally deleted or corrupted
5. Confirm `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update the mapping table**: If new source-of-truth files or doc sections were discovered, add them.
2. **Add new patterns**: If you found a recurring update pattern not documented here, add it to Implementation Patterns.
3. **Update the current docs table**: If new docs were added, update the table at the top.
4. **Commit the skill update** along with the docs updates so improvements are preserved.
