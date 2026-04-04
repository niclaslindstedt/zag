---
description: "Use when the user wants to update the Gemini provider in zag-agent. Guides keeping the Google Gemini CLI wrapper up to date with new models, flags, output formats, and session discovery behavior."
---

# Updating the Gemini Provider

The Gemini provider wraps Google's `gemini` CLI. It uses a file-based system prompt, supports output format flags, and discovers sessions by scanning the Gemini temp directory.

## Upstream References

- **GitHub repository**: https://github.com/google-gemini/gemini-cli (open source)
- **Changelog**: https://github.com/google-gemini/gemini-cli/releases
- **Install/update binary**: `npm install -g @anthropic-ai/gemini-cli`
- **Discover flags**: `gemini --help`

## Discovery Process

1. Run `scripts/check-provider-status.sh gemini` to snapshot current source state vs upstream CLI
2. Run `scripts/fetch-upstream-releases.sh gemini` to check for new releases
3. Check https://github.com/google-gemini/gemini-cli/releases for new releases
4. Clone or pull the repo to read source code for detailed behavior
5. Install/update `gemini` and run `gemini --help` to discover new or changed flags
6. Focus on: new `--output-format` values, new model names, changes to `--approval-mode`, changes to session storage format in `~/.gemini/tmp/`, MCP-related changes, new `--resume` behavior

## Automated Discovery

Run the discovery scripts before starting manual investigation:

```sh
scripts/check-provider-status.sh gemini
scripts/fetch-upstream-releases.sh gemini
```

The first script extracts the current source state (models, defaults, size mappings, flags)
and compares against the installed CLI's `--help` output. The second checks the latest
GitHub release. Review the report before proceeding with manual changes.

## Implementation Files

### Primary

- **Provider**: `zag-agent/src/providers/gemini.rs` — `build_run_args()`, `build_resume_args()`, session discovery, output parsing
- **Tests**: `zag-agent/src/providers/gemini_tests.rs`

### Secondary (touch only if adding new capabilities)

- `zag-agent/src/factory.rs` — model resolution, validation
- `zag-agent/src/builder.rs` — if new builder options needed
- `zag-cli/src/cli.rs` — if new CLI flags
- `zag-cli/src/commands/agent_action.rs` — if new wiring needed

## Implementation Patterns

### Argument construction

Gemini uses a single `build_run_args()` method:

**Non-interactive**:
```
gemini [--approval-mode yolo] [--model <model>] \
  [--include-directories <dir>] [--output-format <format>] \
  [--max-turns <n>] <prompt>
```

**Resume**:
```
gemini --resume [<session_id> | latest] [--approval-mode yolo] \
  [--model <model>] [--include-directories <dir>]
```

Note: the `--model` flag is skipped when model is `"auto"` (Gemini's default, which lets the CLI choose).

### System prompt injection

Writes system prompt to `.gemini/system.md` in the working directory and sets the environment variable `GEMINI_SYSTEM_MD=true`. Both are required — the env var tells the Gemini CLI to read the file.

### Permission skipping

Uses `--approval-mode yolo` (not a `--dangerously-*` flag like Claude/Codex).

### Directory inclusion

Uses `--include-directories <dir>` (not `--add-dir` like other providers). Multiple directories are comma-separated in a single flag value.

### Session discovery

Gemini does not have a native session list command. The provider scans `~/.gemini/tmp/*/chats/` sorted by modification time to find sessions. Session files are JSON with conversation history including reasoning/thinking support. If Gemini changes its session storage location or format, the discovery logic must be updated.

### Adding a new model

1. Add model name to `AVAILABLE_MODELS` array in `gemini.rs`
2. Update `model_for_size()` if the new model should be a size alias target
3. Update `default_model()` if it replaces the default

### Adding a new flag

1. Add field to `Gemini` struct
2. Wire into `build_run_args()` and/or `build_resume_args()`
3. Add setter method following the existing pattern
4. If user-facing: add to CLI args and agent_action wiring

## Update Checklist

- [ ] Update the `// provider-updated: YYYY-MM-DD` comment at the top of `gemini.rs`
- [ ] Update `gemini.rs` — new flags in arg builders, model list, session discovery, output parsing
- [ ] Update `gemini_tests.rs` — test new arg combinations, session format changes
- [ ] Update `docs/providers.md` — feature matrix, available models, known limitations
- [ ] Update `man/run.md` and `man/exec.md` — if command behavior changes
- [ ] Update `README.md` — provider table, model size aliases
- [ ] Update `website/src/components/Providers.tsx` — feature tags, model lists
- [ ] Update `website/src/components/GettingStarted.tsx` — if install command changes
- [ ] If new builder option: update all three bindings (see parity checklist in CLAUDE.md)

## Verification

```sh
make build    # Must compile cleanly
make test     # All tests must pass
make clippy   # Zero warnings
make fmt      # Format code
```
