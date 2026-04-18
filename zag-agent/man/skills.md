# zag skills

Manage provider-agnostic skills stored in `~/.zag/skills/`.

## Synopsis

    zag skills <command> [options]

## Description

Skills are modular, self-contained packages that extend agent capabilities with specialized knowledge, workflows, and tool integrations. They use the [Agent Skills](https://agentskills.io) open standard format: a directory containing a `SKILL.md` file with YAML frontmatter and optional bundled resources.

Skills are stored at `~/.zag/skills/` and automatically synced to each provider's native skill location when running an agent.

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
| Claude   | Symlink  | `~/.claude/skills/zag-<name>/` |
| Gemini   | Symlink  | `~/.gemini/skills/zag-<name>/` |
| Copilot  | Symlink  | `~/.copilot/skills/zag-<name>/` |
| Codex    | Symlink  | `~/.agents/skills/zag-<name>/` |
| Ollama   | System prompt injection | N/A |

Skill directories are symlinked with a `zag-` prefix to avoid collisions with provider-managed skills.

## Commands

### list

List all available skills.

    zag skills list [--json]

### show

Show details of a specific skill, including its full body content.

    zag skills show <name> [--json]

### add

Create a new skill skeleton.

    zag skills add <name> [--description <TEXT>]

Options:
- `--description` — Short description of the skill

### remove

Remove a skill and all its provider symlinks.

    zag skills remove <name>

### sync

Sync skills to all provider-specific locations. Runs automatically before each agent session.

    zag skills sync [-p provider]

Options:
- `-p, --provider` — Only sync for this provider (claude, gemini, copilot, codex)

### import

Import existing skills from a provider's native skill directory into `~/.zag/skills/`.

    zag skills import [--from <provider>]

Options:
- `--from` — Provider to import from (default: claude). Skips directories prefixed with `zag-`.

## Examples

    # Create a new skill
    zag skills add code-reviewer --description "Review code changes for quality and correctness"

    # List all skills
    zag skills list

    # List all skills as JSON
    zag skills list --json

    # Show a specific skill
    zag skills show code-reviewer

    # Show a specific skill as JSON
    zag skills show code-reviewer --json

    # Manually sync to all providers
    zag skills sync

    # Only sync to gemini
    zag skills sync -p gemini

    # Import your existing Claude skills
    zag skills import --from claude

    # Remove a skill
    zag skills remove code-reviewer

## See Also

    zag man zag       Global flags and providers overview
    zag man run       Start an interactive session (skills are synced automatically)
