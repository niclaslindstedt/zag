---
name: update-readme
description: "Use when the README.md may be stale. Discovers commits since the last README update, identifies what changed (commands, flags, providers, bindings, orchestration, etc.), and merges updates into README.md."
---

# Updating the README

The README.md is the primary user-facing documentation. It covers installation, commands, flags, providers, orchestration, session management, language bindings, configuration, skills, MCP, and more. It gets stale when new features land without corresponding README updates.

## Tracking Mechanism

The file `.claude/skills/update-readme/.last-updated` contains the git commit hash from the last time the README was comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-readme/.last-updated)
   ```

2. List all commits since the baseline, filtering for relevant types:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. For each relevant commit, check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the section mapping below to determine which README sections need updating.

5. Read the current README.md to understand existing content before making changes.

6. For each affected section, compare the current source code against what the README documents. Fix any discrepancies.

## Section Mapping

Use this table to map changed files/scopes to README sections:

| Changed files / commit scope | README section(s) to update |
|------------------------------|----------------------------|
| `zag-agent/src/providers/` | **Providers** table (~line 91) — models, size aliases, features |
| `zag-cli/src/cli.rs` (AgentArgs) | **Flags** table (~line 145) — new/changed flags |
| `zag-cli/src/commands/` | **Commands** list (~line 103) — new/renamed commands |
| `zag-orch/src/` | **Orchestration** section (~line 207) — new primitives, patterns |
| `zag-agent/src/builder.rs` | **Programmatic API** (~line 418) — builder example |
| `bindings/` | **Language Bindings** (~line 437) — new bindings, API changes |
| Session-related changes | **Session Management** (~line 171) — new session features |
| `zag-agent/src/skills.rs` | **Skills** section (~line 392) — skill management changes |
| `zag-agent/src/mcp.rs`, `zag-cli/src/commands/mcp/` | **MCP Servers** (~line 403) — MCP features |
| `zag-serve/`, remote commands | **Remote Access** (~line 278) — server/client changes |
| Isolation changes (worktree/sandbox) | **Worktree and Sandbox** (~line 310) |
| JSON/output format changes | **JSON Output** (~line 327) |
| Config changes | **Configuration** (~line 355) |
| Install method changes | **Install** (~line 27) |
| New examples in `examples/` | **Examples** (~line 555) |

## Implementation Files

### Primary

- **README.md** — the file being updated

### Secondary (read-only, for reference)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zag-cli/src/cli.rs` | All CLI flags, commands, subcommands |
| `zag-cli/src/commands/mod.rs` | Registered command list |
| `zag-agent/src/providers/*/mod.rs` | Provider models, size aliases, features |
| `zag-agent/src/builder.rs` | Builder API fields and methods |
| `zag-orch/src/lib.rs` | Orchestration primitives |
| `bindings/*/README.md` | Binding-specific examples and API |
| `zag-agent/man/*.md` | Detailed command documentation |
| `docs/*.md` | Feature-specific documentation |

## Implementation Patterns

### Adding a new command to the Commands list

Find the appropriate category (Core, Session, Orchestration, Remote, Provider/Plugin) and add a row:

```markdown
| `zag <command>` | Brief description |
```

Commands are grouped by functionality. Keep the grouping consistent.

### Adding a new flag to the Flags table

```markdown
| `--flag-name` | `-f` | Description of what it does |
```

Flags are listed roughly in order of importance. Place new flags near related existing ones.

### Updating the Providers table

```markdown
| Provider | Default Model | Small | Medium | Large |
```

Get current values from the provider's `mod.rs` file (`default_model()` and `model_for_size()` functions).

### Adding a new binding language

Add a new subsection under Language Bindings with:
1. Install command
2. Builder pattern example
3. Streaming example (if supported)

### Updating code examples

Ensure all code examples in the README actually work with the current CLI. Check flag names, command syntax, and output format.

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read `README.md` and all source-of-truth files for affected sections
- [ ] Update **Providers** table if provider models/aliases changed
- [ ] Update **Commands** list if commands were added/removed/renamed
- [ ] Update **Flags** table if CLI flags changed
- [ ] Update **Orchestration** section if new primitives added
- [ ] Update **Session Management** if session features changed
- [ ] Update **Language Bindings** if bindings were added/changed
- [ ] Update **Programmatic API** if builder API changed
- [ ] Update **Skills** section if skills system changed
- [ ] Update **MCP Servers** section if MCP features changed
- [ ] Update **Remote Access** section if remote features changed
- [ ] Update **Configuration** section if config keys changed
- [ ] Update **JSON Output** section if output formats changed
- [ ] Update **Install** section if installation methods changed
- [ ] Update **Examples** section if new examples were added
- [ ] Update **Troubleshooting** if new common issues discovered
- [ ] Verify all code examples are correct against current source
- [ ] Consider whether the **update-website** skill should also be run (use the `update-website` skill if website content is now stale relative to the README)
- [ ] Update `.claude/skills/update-readme/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-readme/.last-updated
  ```

## Verification

1. Read through the updated README sections and verify they match the current source code
2. Check that all command names, flag names, and examples are syntactically correct
3. Ensure no sections were accidentally deleted or corrupted
4. Confirm the `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update line numbers**: If README sections shifted significantly, update the approximate line numbers in the section mapping.
2. **Add new mappings**: If you discovered new source-of-truth files or README sections, add them to the mapping table.
3. **Record patterns**: If you found a recurring update pattern not documented here, add it to Implementation Patterns.
4. **Commit the skill update** along with the README update so improvements are preserved.
