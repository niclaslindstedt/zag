# agent skills

Manage provider-agnostic skills stored in `~/.agent/skills/`.

## Synopsis

```
agent skills <command> [options]
```

## Description

Skills are modular, self-contained packages that extend agent capabilities with specialized knowledge, workflows, and tool integrations. They use the [Agent Skills](https://agentskills.io) open standard format: a directory containing a `SKILL.md` file with YAML frontmatter and optional bundled resources.

Skills are stored at `~/.agent/skills/` and automatically synced to each provider's native skill location when running an agent.

## Skill Format

Each skill is a directory containing:

```
skill-name/
├── SKILL.md       (required) Instructions and YAML metadata
├── scripts/       (optional) Executable scripts
├── references/    (optional) Static documentation
└── assets/        (optional) Templates and resources
```

`SKILL.md` format:

```markdown
---
name: skill-name
description: When and how this skill should be used.
---

# Skill Name

Instructions for the agent...
```

## Provider Integration

| Provider | Strategy | Location |
|----------|----------|----------|
| Claude   | Symlink  | `~/.claude/skills/agent-<name>/` |
| Gemini   | Symlink  | `~/.gemini/skills/agent-<name>/` |
| Copilot  | Symlink  | `~/.copilot/skills/agent-<name>/` |
| Codex    | Symlink  | `~/.agents/skills/agent-<name>/` |
| Ollama   | System prompt injection | N/A |

Skill directories are symlinked with an `agent-` prefix to avoid collisions with provider-managed skills.

## Commands

### list

List all available skills.

```
agent skills list
```

### add

Create a new skill skeleton.

```
agent skills add <name> [-d description]
```

Options:
- `-d, --description` — Short description of the skill

### remove

Remove a skill and all its provider symlinks.

```
agent skills remove <name>
```

### sync

Sync skills to all provider-specific locations. Runs automatically before each agent session.

```
agent skills sync [-p provider]
```

Options:
- `-p, --provider` — Only sync for this provider (claude, gemini, copilot, codex)

### import

Import existing skills from a provider's native skill directory into `~/.agent/skills/`.

```
agent skills import [--from <provider>]
```

Options:
- `--from` — Provider to import from (default: claude). Skips directories prefixed with `agent-`.

## Examples

```bash
# Create a new skill
agent skills add code-reviewer -d "Review code changes for quality and correctness"

# List all skills
agent skills list

# Manually sync to all providers
agent skills sync

# Only sync to gemini
agent skills sync -p gemini

# Import your existing Claude skills
agent skills import --from claude

# Remove a skill
agent skills remove code-reviewer
```

## See Also

- `agent man` — Show all available manpages
- `agent run` — Start an interactive session (skills are synced automatically)
