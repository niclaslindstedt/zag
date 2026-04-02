# Troubleshooting

Common issues and how to resolve them.

## CLI not found in PATH

```
Error: CLI not found in PATH: claude
```

The agent CLI binary isn't installed or isn't in your `PATH`. Install it using the commands in the [provider table](providers.md), then verify:

```bash
which claude    # or codex, gemini, gh, ollama
```

If installed via `npm install -g`, ensure your npm global bin directory is in your `PATH`.

## Invalid model

```
Error: Invalid model 'X' for provider Y
```

You specified a model name that the provider doesn't recognize. Check available models:

```bash
zag capability -p <provider> --pretty
```

Or use size aliases (`small`, `medium`, `large`) instead of provider-specific model names.

## Worktree fails

```
Error: Not a git repository
```

The `--worktree` (`-w`) flag requires you to be inside a git repository. Worktrees are created under `~/.zag/worktrees/`.

## Sandbox fails

```
Error: Docker is not running
```

Docker must be installed and running. Sandbox mode uses `docker sandbox run` for microVM isolation. Verify with:

```bash
docker info
```

## Config not taking effect

Check which config file is being used:

```bash
zag config path
```

Config is per-project (based on git repo root). CLI flags always override config values. See [Configuration](configuration.md) for the full precedence order.

## JSON validation failure

When using `--json-schema`, zag validates the agent's output against your schema. On failure, zag retries up to 3 times via session resume. If all retries fail:

- Simplify your schema -- complex nested schemas are harder for agents to satisfy
- Try a larger model (`-m large`)
- Use `--json` without a schema first to see what format the agent naturally produces

## Session resume fails

```
Error: Session not found
```

Session data is stored under `~/.zag/projects/<sanitized-path>/`. If the session was created in a different directory or with a different `--root`, zag won't find it. List available sessions:

```bash
zag session list
```

## API key not set

If the upstream CLI reports authentication errors, ensure the relevant API key is configured:

- **Claude**: `ANTHROPIC_API_KEY` or `claude` CLI login
- **Codex**: OpenAI authentication
- **Gemini**: Google authentication
- **Copilot**: `gh auth login`
- **Ollama**: No API key needed (local)

These are managed by the upstream CLIs, not by zag.

## Debug mode

Run with `-d` to see detailed debug output:

```bash
zag -d exec "your prompt"
```

This shows the exact CLI commands zag constructs, subprocess output, and event parsing details.

## Log file locations

- **Session logs**: `~/.zag/projects/<sanitized-path>/sessions/`
- **Event markers**: `~/.zag/events/` (`.started` and `.ended` files for external orchestrators)
- **Provider logs**:
  - Claude: `~/.claude/projects/`
  - Codex: `~/.codex/history.jsonl`, `~/.codex/log/codex-tui.log`
  - Copilot: `~/.copilot/session-state/`
  - Gemini: `~/.gemini/tmp/`

## Filing a bug report

If you can't resolve the issue, [open a GitHub issue](https://github.com/niclaslindstedt/zag/issues/new?template=bug_report.md) with:

1. Your zag version (`zag --version`)
2. OS and Rust version (`rustc --version`)
3. The provider and model you're using
4. Steps to reproduce
5. Debug output (`zag -d ...`)
