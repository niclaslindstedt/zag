---
description: "Use when the user wants to update the Claude provider in zag-agent. Guides keeping the Claude Code CLI wrapper up to date with new models, flags, output formats, and streaming capabilities."
---

# Updating the Claude Provider

The Claude provider wraps Anthropic's `claude` CLI. It is the most feature-rich provider, supporting streaming I/O, native JSON schema, session resume, and MCP servers. It has a sub-module structure unlike other providers.

## Upstream References

- **Documentation**: https://docs.anthropic.com/en/docs/claude-code
- **Changelog**: https://docs.anthropic.com/en/docs/claude-code/changelog
- **GitHub repository**: https://github.com/anthropics/claude-code (open source)
- **Install/update binary**: `curl -fsSL https://claude.ai/install.sh | bash`
- **Discover flags**: `claude --help` and `claude <subcommand> --help`

## Discovery Process

1. Run `scripts/check-provider-status.sh claude` to snapshot current source state vs upstream CLI
2. Run `scripts/fetch-upstream-releases.sh claude` to check for new releases
3. Fetch the changelog to identify new features, flags, or models since the last update
4. Clone or pull https://github.com/anthropics/claude-code to read source code for detailed behavior
5. Install/update the `claude` binary and run `claude --help` to discover new or changed flags
6. Pay special attention to: new `--output-format` values, new `--model` options, changes to `--print` mode, streaming protocol changes, new `--input-format` values, MCP-related changes

## Automated Discovery

Run the discovery scripts before starting manual investigation:

```sh
scripts/check-provider-status.sh claude
scripts/fetch-upstream-releases.sh claude
```

The first script extracts the current source state (models, defaults, size mappings, flags)
and compares against the installed CLI's `--help` output. The second checks the latest
GitHub release. Review the report before proceeding with manual changes.

## Implementation Files

### Primary

- **Provider module**: `zag-agent/src/providers/claude/mod.rs` — main implementation with `build_run_args()`, `build_resume_args()`, `execute_streaming()`, output parsing
- **Models file**: `zag-agent/src/providers/claude/models.rs` — model definitions, size mappings, model validation
- **Tests**: `zag-agent/src/providers/claude/claude_tests.rs`

### Secondary (touch only if adding new capabilities)

- `zag-agent/src/agent.rs` — Agent trait (if new trait methods needed)
- `zag-agent/src/factory.rs` — AgentFactory (model resolution, validation dispatch)
- `zag-agent/src/builder.rs` — AgentBuilder (if new builder options, especially Claude-specific ones like `input_format`, `replay_user_messages`, `json_schema`)
- `zag-cli/src/cli.rs` — AgentArgs (if new CLI flags)
- `zag-cli/src/commands/agent_action.rs` — wiring of CLI args to agent configuration (Claude-specific downcast block)

## Implementation Patterns

### Argument construction

Claude uses `build_run_args(interactive, prompt, output_format)` which returns a `Vec<String>`. Non-interactive mode requires `--print --verbose`. Example flow:

```
claude --print --verbose --output-format json --model <model> \
  --dangerously-skip-permissions --append-system-prompt <prompt> \
  --add-dir <dir> --max-turns <n> <user_prompt>
```

### System prompt injection

Uses `--append-system-prompt <text>` flag directly (no file written). This is the cleanest method among all providers.

### Output parsing

Claude has the richest output: events-based JSON format with System, Assistant, User, Result event types. The streaming path uses `stream-json` output format with bidirectional I/O.

### Claude-specific features

These are Claude-only and use downcasting via `as_any_mut()` in `agent_action.rs`:
- `set_input_format()` — for `stream-json` input
- `set_replay_user_messages()` — re-emit stdin to stdout
- `set_include_partial_messages()` — include streaming chunks
- `set_json_schema()` — native structured output
- `set_event_handler()` — callback for streaming events

### Adding a new model

1. Add model name to `AVAILABLE_MODELS` in `claude/models.rs`
2. Update `model_for_size()` if the new model should be a size alias target
3. Update `default_model()` if it replaces the default

### Adding a new flag

1. Add field to `Claude` struct in `claude/mod.rs`
2. Add setter method (e.g., `set_new_flag()`)
3. Wire into `build_run_args()` and/or `build_resume_args()`
4. If exposed to users: add to `AgentArgs` in `cli.rs`, wire in `agent_action.rs`
5. If builder option: add to `AgentBuilder` in `builder.rs`, follow parity checklist

## Update Checklist

- [ ] Update the `// provider-updated: YYYY-MM-DD` comment at the top of `claude/mod.rs`
- [ ] Update `claude/models.rs` — new models, size alias changes
- [ ] Update `claude/mod.rs` — new flags in arg builders, output parsing changes
- [ ] Update `claude/claude_tests.rs` — test new arg combinations, new output formats
- [ ] Update `docs/providers.md` — feature matrix, available models, known limitations
- [ ] Update `man/run.md` and `man/exec.md` — if command behavior changes
- [ ] Update `man/zag.md` — if global provider capabilities change
- [ ] Update `README.md` — provider table, model size aliases
- [ ] Update `website/src/components/Providers.tsx` — feature tags, model lists
- [ ] Update `website/src/components/GettingStarted.tsx` — if install command changes
- [ ] If new builder option: update all six bindings (TypeScript, Python, C#, Swift, Java, Kotlin) + their tests + READMEs (see parity checklist in CLAUDE.md and the `update-bindings` skill)

## Verification

```sh
make build    # Must compile cleanly
make test     # All tests must pass
make clippy   # Zero warnings
make fmt      # Format code
```
