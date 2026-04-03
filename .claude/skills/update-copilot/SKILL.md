---
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

1. Check GitHub Copilot CLI documentation for announcements of new features
2. Run `gh extension upgrade gh-copilot` to get the latest version
3. Run `copilot --help` to discover new or changed flags
4. Focus on: new `--model` options (Copilot supports models from multiple providers), changes to `-p`/`-i` flags, new `--resume` behavior, changes to `events.jsonl` format, new session state directory structure, MCP-related changes
5. Since it's closed source, test behavior empirically by running commands and inspecting output

## Implementation Files

### Primary

- **Provider**: `zag-agent/src/providers/copilot.rs` — `build_exec_args()`, `build_interactive_args()`, `build_resume_args()`, event parsing, workspace discovery
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
copilot --allow-all-tools --model <model> [--add-dir <dir>] \
  [--max-turns <n>] -p <prompt>
```

**Interactive**:
```
copilot [--allow-all-tools] --model <model> [--add-dir <dir>] \
  [--max-turns <n>] -i <prompt>
```

**Resume**:
```
copilot --resume [<session_id>] [--allow-all-tools] --model <model> [--add-dir <dir>]
copilot --continue [--allow-all-tools] --model <model> [--add-dir <dir>]
```

### System prompt injection

Writes system prompt to `.github/instructions/agent/agent.instructions.md`. This path is specific to GitHub Copilot's instruction file convention.

### Output format limitation

Copilot does **not** support the `--output` flag. If `set_output_format()` is called, it will error. JSON output is handled by zag's own capture-and-parse layer, not Copilot's CLI.

### Event parsing

Copilot writes session events to `events.jsonl` in its session state directory (`~/.copilot/session-state/`). The provider includes deduplication logic for events. The event format includes workspace path information that must be matched to find the correct session.

### Permission skipping

Uses `--allow-all-tools` (required in non-interactive mode). In interactive mode it can be optional.

### Multi-provider models

Copilot uniquely supports models from Claude, GPT, and Gemini families. When any of these upstream providers release new models, check if Copilot has added support. The model list is the largest of all providers.

### Adding a new model

1. Add model name to `AVAILABLE_MODELS` array in `copilot.rs`
2. Update `model_for_size()` if the new model should be a size alias target
3. Update `default_model()` if it replaces the default

### Adding a new flag

1. Add field to `Copilot` struct
2. Wire into the appropriate arg builder (`build_exec_args()`, `build_interactive_args()`, or `build_resume_args()`)
3. Add setter method following the existing pattern
4. If user-facing: add to CLI args and agent_action wiring

## Update Checklist

- [ ] Update `copilot.rs` — new flags in arg builders, model list, event parsing, workspace discovery
- [ ] Update `copilot_tests.rs` — test new arg combinations, event format changes
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
