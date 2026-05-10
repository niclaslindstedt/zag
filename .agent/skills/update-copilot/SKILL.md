---
name: update-copilot
description: "Use when the user wants to update the Copilot provider in zag-agent. Guides keeping the GitHub Copilot CLI wrapper up to date with new models, flags, event parsing, and session behavior."
---

# Updating the Copilot Provider

The Copilot provider wraps GitHub's `gh copilot` CLI extension. It is not open source, supports multi-provider models (Claude, GPT, Gemini), writes system prompts to a file, and parses session events from `events.jsonl`.

## Upstream References

- **Documentation**: https://docs.github.com/en/copilot/github-copilot-in-the-cli
- **Install/update**: `gh extension install github/gh-copilot` (or `gh extension upgrade gh-copilot`)
- **Discover flags**: `gh copilot --help`, `copilot --help`
- **Not open source**: No GitHub repo to clone. Rely on `--help` output, official docs, and observed behavior.

## Discovery Process

1. Run `scripts/check-provider-status.sh copilot` to snapshot current source state vs upstream CLI
2. Check GitHub Copilot CLI documentation for announcements of new features
3. Run `gh extension upgrade gh-copilot` to get the latest version
4. Run `copilot --help` to discover new or changed flags
5. Focus on: new `--model` options (Copilot supports models from multiple providers), changes to `-p`/`-i` flags, new `--resume` behavior, changes to `events.jsonl` format, new session state directory structure, MCP-related changes
6. Since it's closed source, test behavior empirically by running commands and inspecting output

## Automated Discovery

Run the discovery script before starting manual investigation:

```sh
scripts/check-provider-status.sh copilot
```

The script extracts the current source state (models, defaults, size mappings, flags)
and compares against the installed CLI's `--help` output. Note: Copilot is closed source,
so there is no GitHub release check. Review the report before proceeding with manual changes.

## Implementation Files

### Primary

- **Provider**: `zag-agent/src/providers/copilot.rs` — `build_run_args()`, `run_resume()`, event parsing, workspace discovery
- **Tests**: `zag-agent/src/providers/copilot_tests.rs`

### Secondary (touch only if adding new capabilities)

- `zag-agent/src/factory.rs` — model resolution, validation
- `zag-agent/src/builder.rs` — if new builder options needed
- `zag-cli/src/cli.rs` — if new CLI flags
- `zag-cli/src/commands/agent_action.rs` — if new wiring needed

## Implementation Patterns

### Argument construction

Copilot uses `-p` for non-interactive and `-i` for interactive mode:

**Non-interactive**:
```
copilot --allow-all --model <model> [--add-dir <dir>] \
  [--max-turns <n>] -p <prompt>
```

**Interactive**:
```
copilot [--allow-all] --model <model> [--add-dir <dir>] \
  [--max-turns <n>] -i <prompt>
```

**Resume**:
```
copilot --resume [<session_id>] [--allow-all] --model <model> [--add-dir <dir>]
copilot --continue [--allow-all] --model <model> [--add-dir <dir>]
```

Note: The flag name is `--allow-all` (not `--allow-all-tools`). Verify via `copilot --help` if this changes.

### System prompt injection

Writes system prompt to `.github/instructions/agent/agent.instructions.md`. This path is specific to GitHub Copilot's instruction file convention.

### Output format limitation

Copilot does **not** support the `--output` flag. If `set_output_format()` is called, it will error. JSON output is handled by zag's own capture-and-parse layer, not Copilot's CLI.

### Event parsing

Copilot writes session events to `events.jsonl` in its session state directory (`~/.copilot/session-state/`). The provider includes deduplication logic for events. The event format includes workspace path information that must be matched to find the correct session.

### Permission skipping

Uses `--allow-all` (required in non-interactive mode). In interactive mode it can be optional.

### Multi-provider models

Copilot uniquely supports models from Claude, GPT, and Gemini families. When any of these upstream providers release new models, check if Copilot has added support. The model list is the largest of all providers.

### Adding a new model

1. Add model name to `AVAILABLE_MODELS` array in `copilot.rs`
2. Update `model_for_size()` if the new model should be a size alias target
3. Update `default_model()` if it replaces the default

### Adding a new flag

1. Add field to `Copilot` struct
2. Wire into `build_run_args()` (handles both interactive and non-interactive via the `interactive` parameter) and/or `run_resume()`
3. Add setter method following the existing pattern
4. If user-facing: add to CLI args and agent_action wiring

### Model cross-referencing

Since Copilot is closed source, you cannot directly verify available models. When updating, cross-reference with models available in other providers:
- Check Codex's `AVAILABLE_MODELS` for new GPT models (Copilot adds these quickly since both are Microsoft/OpenAI)
- Check Gemini's `AVAILABLE_MODELS` for new Google models
- Claude models are typically available in Copilot when they reach GA

The docs URL `https://docs.github.com/en/copilot/github-copilot-in-the-cli` may redirect to a retirement notice for the old `gh copilot` extension. Look for the standalone `copilot` CLI documentation instead.

## Update Checklist

- [ ] Update the `// provider-updated: YYYY-MM-DD` comment at the top of `copilot.rs`
- [ ] Update `copilot.rs` — new flags in arg builders, model list, event parsing, workspace discovery
- [ ] Update `copilot_tests.rs` — test new arg combinations, event format changes
- [ ] Update `docs/providers.md` — feature matrix, available models, known limitations
- [ ] Update `zag-agent/man/run.md` and `zag-agent/man/exec.md` — if command behavior changes
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

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Fix inaccuracies**: Correct any wrong URLs, flag names, method names, or behavioral descriptions discovered during the update.
2. **Update model cross-referencing notes**: Record which models were confirmed available and which were inferred.
3. **Update implementation patterns**: If the actual code differs from what's documented here (e.g., method names changed, new patterns emerged), update the patterns section.
4. **Record known limitations**: Document any verified behavioral limitations with the version they were checked against.
5. **Commit the skill update** along with the provider update so the improvements are preserved.
