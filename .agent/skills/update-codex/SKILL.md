---
name: update-codex
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

- **Provider**: `zag-agent/src/providers/codex.rs` — `build_run_args()`, `run_resume()`, `run_resume_with_prompt()`, `parse_ndjson_output()`
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
codex exec --skip-git-repo-check [--json] [--ephemeral] --cd <root> --model <model> \
  [--add-dir <dir>] [--max-turns <n>] [--output-schema <path>] \
  [--full-auto] <prompt>
```

Note: Permission skipping uses `--full-auto` (not `--dangerously-bypass-approvals-and-sandbox`).

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

### Structured output

Codex supports `--output-schema <path>` to constrain the model's response to a JSON schema. Unlike Claude's `--json-schema` which takes inline JSON, Codex takes a file path to a schema file. The `set_output_schema()` setter stores this path and wires it into `build_run_args()` for non-interactive mode only.

### Adding a new flag

1. Add field to `Codex` struct
2. Wire into `build_run_args()` (handles both interactive and non-interactive via the `interactive` parameter) and/or `run_resume()` / `run_resume_with_prompt()`
3. Add setter method following the existing pattern
4. If user-facing: add to CLI args and agent_action wiring

## Update Checklist

- [ ] Update the `// provider-updated: YYYY-MM-DD` comment at the top of `codex.rs`
- [ ] Update `codex.rs` — new flags in arg builders, model list, NDJSON parsing
- [ ] Update `codex_tests.rs` — test new arg combinations, new NDJSON event types
- [ ] Update `docs/providers.md` — feature matrix, available models, known limitations
- [ ] Update `zag-agent/man/run.md` and `zag-agent/man/exec.md` — if command behavior changes
- [ ] Update `README.md` — provider table, model size aliases
- [ ] Update `website/src/components/Providers.tsx` — feature tags, model lists
- [ ] Update `website/src/components/GettingStarted.tsx` — if install command changes
- [ ] If new builder option: update all six bindings (see parity checklist in CLAUDE.md and the `update-bindings` skill)

## Web Discovery Tips

- Codex has both JS/npm releases and Rust releases. The Rust CLI releases use tags like `rust-v0.118.0`. The npm releases are separate.
- The Rust CLI source is at `codex-rs/exec/src/cli.rs` in the repo. Use `https://raw.githubusercontent.com/openai/codex/main/codex-rs/exec/src/cli.rs` to fetch the CLI definition directly.
- The GitHub releases page sometimes has loading errors. If detailed release notes aren't visible, check individual release tags.
- Alpha releases (e.g., `0.119.0-alpha.x`) are published frequently but should not be tracked — focus on stable releases.

## Verification

```sh
make build    # Must compile cleanly
make test     # All tests must pass
make clippy   # Zero warnings
make fmt      # Format code
```

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Fix inaccuracies**: Correct any wrong URLs, flag names, method names, or behavioral descriptions discovered during the update.
2. **Add discovery tips**: If you found useful search queries, source file paths, or workarounds for 404s, add them to the "Web Discovery Tips" section.
3. **Update implementation patterns**: If the actual code differs from what's documented here (e.g., method names changed, new patterns emerged), update the patterns section.
4. **Record known limitations**: Document any verified behavioral limitations with the version they were checked against.
5. **Commit the skill update** along with the provider update so the improvements are preserved.
