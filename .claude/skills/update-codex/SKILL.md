---
description: "Use when the user wants to update the Codex provider in zag-agent. Guides keeping the OpenAI Codex CLI wrapper up to date with new models, flags, NDJSON output changes, and session resume behavior."
---

# Updating the Codex Provider

The Codex provider wraps OpenAI's `codex` CLI. It uses NDJSON output parsing, supports session resume via thread IDs, and writes system prompts to a file.

## Upstream References

- **GitHub repository**: https://github.com/openai/codex (open source)
- **Changelog**: https://github.com/openai/codex/releases
- **Install/update binary**: `npm install -g @openai/codex`
- **Discover flags**: `codex --help`, `codex exec --help`, `codex resume --help`

## Discovery Process

1. Run `scripts/check-provider-status.sh codex` to snapshot current source state vs upstream CLI
2. Run `scripts/fetch-upstream-releases.sh codex` to check for new releases
3. Check https://github.com/openai/codex/releases for new releases and changelogs
4. Clone or pull the repo to inspect source code for detailed CLI behavior
5. Install/update `codex` and run `codex --help`, `codex exec --help` to discover new flags
6. Focus on: new `exec` subcommand flags, changes to NDJSON event format, new model names, changes to `--sandbox` options, changes to `--json` output structure

## Automated Discovery

Run the discovery scripts before starting manual investigation:

```sh
scripts/check-provider-status.sh codex
scripts/fetch-upstream-releases.sh codex
```

The first script extracts the current source state (models, defaults, size mappings, flags)
and compares against the installed CLI's `--help` output. The second checks the latest
GitHub release. Review the report before proceeding with manual changes.

## Implementation Files

### Primary

- **Provider**: `zag-agent/src/providers/codex.rs` — `build_exec_args()`, `build_interactive_args()`, `build_resume_args()`, `parse_ndjson_output()`
- **Tests**: `zag-agent/src/providers/codex_tests.rs`

### Secondary (touch only if adding new capabilities)

- `zag-agent/src/factory.rs` — model resolution, validation
- `zag-agent/src/builder.rs` — if new builder options needed
- `zag-cli/src/cli.rs` — if new CLI flags
- `zag-cli/src/commands/agent_action.rs` — if new wiring needed

## Implementation Patterns

### Argument construction

Codex uses separate methods for different modes:

**Non-interactive (exec)**:
```
codex exec --skip-git-repo-check [--json] --cd <root> --model <model> \
  [--add-dir <dir>] [--max-turns <n>] \
  [--dangerously-bypass-approvals-and-sandbox --sandbox danger-full-access] <prompt>
```

**Interactive**:
```
codex [--cd <root> --model <model> --add-dir <dir>] <prompt>
```

**Resume**:
```
codex resume [<session_id> | --last] --cd <root> --model <model> [--add-dir <dir>]
```

### System prompt injection

Writes system prompt to `.codex/AGENTS.md` in the working directory before execution. The file is created/overwritten each run. Keep this pattern — Codex reads this file automatically.

### Output parsing (NDJSON)

Codex emits newline-delimited JSON events. Key event types:
- `thread.started` — contains `thread_id` (used for resume)
- `item.completed` — contains agent messages, tool results
- `turn.started` / `turn.completed` — turn boundaries

The `parse_ndjson_output()` function extracts the thread ID and final agent message text. When Codex changes its event format, this parser must be updated.

### Session resume

Uses thread IDs extracted from NDJSON output. The `resume` subcommand takes a session ID or `--last`. Resume-with-prompt uses `exec --resume <thread_id>`.

### Adding a new model

1. Add model name to `AVAILABLE_MODELS` array in `codex.rs`
2. Update `model_for_size()` if the new model should be a size alias target
3. Update `default_model()` if it replaces the default

### Adding a new flag

1. Add field to `Codex` struct
2. Wire into the appropriate arg builder (`build_exec_args()`, `build_interactive_args()`, or `build_resume_args()`)
3. Add setter method following the existing pattern
4. If user-facing: add to CLI args and agent_action wiring

## Update Checklist

- [ ] Update the `// provider-updated: YYYY-MM-DD` comment at the top of `codex.rs`
- [ ] Update `codex.rs` — new flags in arg builders, model list, NDJSON parsing
- [ ] Update `codex_tests.rs` — test new arg combinations, new NDJSON event types
- [ ] Update `docs/providers.md` — feature matrix, available models, known limitations
- [ ] Update `man/run.md` and `man/exec.md` — if command behavior changes
- [ ] Update `README.md` — provider table, model size aliases
- [ ] Update `website/src/components/Providers.tsx` — feature tags, model lists
- [ ] Update `website/src/components/GettingStarted.tsx` — if install command changes
- [ ] If new builder option: update all six bindings (see parity checklist in CLAUDE.md and the `update-bindings` skill)

## Verification

```sh
make build    # Must compile cleanly
make test     # All tests must pass
make clippy   # Zero warnings
make fmt      # Format code
```
