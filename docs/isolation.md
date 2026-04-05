# Isolation Modes

zag supports two isolation modes that let agents work in sandboxed environments: **worktree** isolation (git-based) and **sandbox** isolation (Docker-based). Both protect your working directory from unintended changes.

## Worktree isolation

Git worktree isolation creates a separate copy of your repository for the agent to work in. Changes stay in the worktree until you explicitly merge them.

```bash
zag -w exec "experiment with a new approach"
```

### How it works

1. zag creates a git worktree under `~/.zag/worktrees/<repo>/<name>/` with a detached HEAD
2. The agent runs in the worktree directory instead of your main working tree
3. After the session, you're prompted to keep or discard the worktree
4. If kept, you can review changes and merge them into your branch

### Named worktrees

Give the worktree a name for easy identification:

```bash
zag -w refactor exec "refactor the auth module"
```

This creates the worktree at `~/.zag/worktrees/<repo>/refactor/`.

### Requirements

- Must be inside a git repository
- Git must be installed

## Sandbox isolation

Sandbox isolation runs the agent inside a Docker microVM, providing full filesystem and network isolation.

```bash
zag --sandbox exec "run untrusted code"
```

### How it works

1. zag launches a Docker sandbox using `docker sandbox run` with a provider-specific template
2. The agent runs inside the sandbox with access to a workspace directory
3. The sandbox is automatically removed when the session ends

### Provider templates

Each provider has a dedicated Docker template:

| Provider | Template |
|----------|----------|
| claude | `docker/sandbox-templates:claude-code` |
| codex | `docker/sandbox-templates:codex` |
| gemini | `docker/sandbox-templates:gemini` |
| copilot | `docker/sandbox-templates:copilot` |
| ollama | `shell` |

### Named sandboxes

```bash
zag --sandbox test-env exec "run the test suite"
```

### Requirements

- Docker must be installed and running (`docker info` to verify)
- The `docker sandbox` command must be available

## When to use which

| Scenario | Recommended mode |
|----------|-----------------|
| Code experiments you might want to keep | Worktree (`-w`) |
| Running untrusted or generated code | Sandbox (`--sandbox`) |
| Parallel experiments on same repo | Worktree with names |
| Full filesystem isolation | Sandbox |
| No Docker available | Worktree |
| Not a git repository | Sandbox |

## Combining with other features

Isolation works with all other zag features:

```bash
# Worktree + orchestration
sid=$(zag spawn -w -p claude "implement feature X")

# Sandbox + JSON output
zag --sandbox exec --json "analyze the codebase"

# Worktree + specific provider and model
zag -w -p gemini -m large exec "deep refactoring"
```

## Provider support

All five providers support both isolation modes:

| Provider | Worktree | Sandbox |
|----------|----------|---------|
| Claude | Yes | Yes |
| Codex | Yes | Yes |
| Gemini | Yes | Yes |
| Copilot | Yes | Yes |
| Ollama | Yes | Yes |

## Related

- [Providers](providers.md) -- Provider feature matrix
- [Sessions](sessions.md) -- Sessions track isolation state
- [Troubleshooting](troubleshooting.md) -- Worktree and sandbox error resolution
- `zag man exec` -- Isolation flags reference
