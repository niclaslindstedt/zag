---
description: "Use when the user wants to update the Gemini provider in zag-agent. Guides keeping the Google Gemini CLI wrapper up to date with new models, flags, output formats, and session discovery behavior."
---

# Updating the Gemini Provider

The Gemini provider wraps Google's `gemini` CLI. It uses a file-based system prompt, supports output format flags, and discovers sessions by scanning the Gemini temp directory.

## Upstream References

- **GitHub repository**: https://github.com/google-gemini/gemini-cli (open source)
- **Changelog**: https://github.com/google-gemini/gemini-cli/releases
- **Install/update binary**: `npm install -g @google/gemini-cli`
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
  [--include-directories <dir>] [--output-format <format>] <prompt>
```

Note: Gemini CLI does **not** support `--max-turns` as a CLI flag (checked as of v0.36.0, no results in the repo). Max turns must be configured via Gemini's `settings.json` (`maxSessionTurns`). The `set_max_turns()` value is stored but not passed as an argument.

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

### Output format

Gemini supports `--output-format` with values including `json` and `stream-json`. The `stream-json` format enables streaming structured output similar to Claude's streaming mode.

### Directory inclusion

Uses `--include-directories <dir>` (not `--add-dir` like other providers). In the implementation, each directory is passed as a separate `--include-directories` argument (not comma-separated).

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
- [ ] Update `zag-cli/man/run.md` and `zag-cli/man/exec.md` — if command behavior changes
- [ ] Update `README.md` — provider table, model size aliases
- [ ] Update `website/src/components/Providers.tsx` — feature tags, model lists
- [ ] Update `website/src/components/GettingStarted.tsx` — if install command changes
- [ ] If new builder option: update all six bindings (see parity checklist in CLAUDE.md and the `update-bindings` skill)

## Web Discovery Tips

- The Gemini CLI source lives under `packages/` in the monorepo. Exact file paths for CLI arg definitions may change between releases — if fetching source files returns 404, try searching the repo via GitHub's search API instead.
- Use `https://github.com/google-gemini/gemini-cli/search?q=<term>` to search for specific flags or features (e.g., `max-turns`, `plan mode`).
- The Gemini CLI README at `https://github.com/google-gemini/gemini-cli` lists supported flags and output formats.
- Release notes at `https://github.com/google-gemini/gemini-cli/releases` include both stable and preview/nightly releases — focus on stable releases (no `-preview` or `-nightly` suffix).

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
4. **Record known limitations**: If you confirmed that a feature is still missing (e.g., `--max-turns` not supported), note the version where this was verified so future updates don't re-investigate.
5. **Commit the skill update** along with the provider update so the improvements are preserved.
