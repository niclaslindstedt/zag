---
name: update-ollama
description: "Use when the user wants to update the Ollama provider in zag-agent. Guides keeping the Ollama CLI wrapper up to date with new CLI flags, model defaults, size mappings, and Docker sandbox behavior."
---

# Updating the Ollama Provider

The Ollama provider wraps the `ollama` CLI for local model execution. It is the simplest provider — no session resume, no MCP, no streaming in structured format. Models use a `name:size` tag format and system prompts are prepended to the user prompt.

## Upstream References

- **GitHub repository**: https://github.com/ollama/ollama (open source)
- **Changelog**: https://github.com/ollama/ollama/releases
- **Website**: https://ollama.com
- **Model library**: https://ollama.com/library (browse available models)
- **Install/update**: https://ollama.com/download
- **Discover flags**: `ollama --help`, `ollama run --help`

## Discovery Process

1. Run `scripts/check-provider-status.sh ollama` to snapshot current source state vs upstream CLI
2. Run `scripts/fetch-upstream-releases.sh ollama` to check for new releases
3. Check https://github.com/ollama/ollama/releases for new releases
4. Clone or pull the repo to read source code for CLI flag changes
5. Install/update `ollama` and run `ollama run --help` to discover new flags
6. Focus on: new `ollama run` flags (especially `--system`, `--format`, `--json`), new default models on ollama.com/library, changes to model tag format, Docker/container changes
7. Check if `ollama` has added a `--system` flag (currently missing — system prompts are prepended to the user prompt as a workaround)
8. Check if `ollama` has added session/conversation resume capability

## Automated Discovery

Run the discovery scripts before starting manual investigation:

```sh
scripts/check-provider-status.sh ollama
scripts/fetch-upstream-releases.sh ollama
```

The first script extracts the current source state (sizes, defaults, size mappings, flags)
and compares against the installed CLI's `--help` output. The second checks the latest
GitHub release. Review the report before proceeding with manual changes.

## Implementation Files

### Primary

- **Provider**: `zag-agent/src/providers/ollama.rs` — `build_run_args()`, model/size handling, Docker sandbox wrapper
- **Tests**: `zag-agent/src/providers/ollama_tests.rs`

### Secondary (touch only if adding new capabilities)

- `zag-agent/src/factory.rs` — model resolution (Ollama uses config-based size resolution)
- `zag-agent/src/builder.rs` — Ollama-specific size resolution from config
- `zag-cli/src/cli.rs` — `--size` flag in AgentArgs (Ollama-specific)
- `zag-cli/src/commands/agent_action.rs` — if new wiring needed

## Implementation Patterns

### Argument construction

Ollama uses `ollama run` with model:size tags:

**Non-interactive with JSON**:
```
ollama run --format json --nowordwrap --hidethinking <model>:<size> "<prompt>"
```

**Non-interactive text**:
```
ollama run --nowordwrap --hidethinking <model>:<size> "<prompt>"
```

**Interactive**:
```
ollama run --hidethinking <model>:<size> "<prompt>"
```

### Model:size tag format

Unlike other providers, Ollama models are specified as `<model>:<size>` (e.g., `qwen3.5:9b`). The model and size are separate concepts:
- **Model**: the model family (e.g., `qwen3.5`, `llama3`, `codellama`)
- **Size**: parameter count (e.g., `0.8b`, `2b`, `4b`, `9b`, `27b`, `35b`, `122b`)

The display name combines both: `model_display_name()` returns `"{model}:{size}"`.

### System prompt injection

Ollama has **no `--system` flag**. The system prompt is prepended to the user prompt with a double newline separator: `"{system_prompt}\n\n{user_prompt}"`. If Ollama adds a `--system` flag in a future release, this should be updated to use it instead.

### Model validation

Ollama skips model validation — it accepts any model name since users can pull any model from the Ollama registry. Validation is only done for size values against the `AVAILABLE_SIZES` list.

### Size resolution

Size comes from multiple sources with this priority:
1. CLI `--size` flag
2. Config file `ollama.size` key
3. Default: `"9b"`

The builder in `builder.rs` has Ollama-specific logic to resolve size from config.

### Docker sandbox

Ollama has special sandbox support that wraps the command in a Docker shell:
```
docker sandbox run --name <name> <template> <workspace> -- \
  -c "ollama run --hidethinking <model>:<size> ..."
```
Shell escaping is applied for special characters in the prompt.

### Permission handling

Ollama always auto-approves — `set_skip_permissions()` forces `skip_permissions = true` regardless of the input. This is because Ollama runs locally and doesn't have a permission model.

### Session resume

Ollama does **not** support session resume. The `run_resume()` method returns an error: "Ollama does not support session resume." If Ollama adds conversation history/resume in a future release, this should be implemented.

### Updating the default model

The default model (`qwen3.5`) may need updating when better small models become available. Consider:
1. Model quality for coding tasks
2. Availability in the Ollama library
3. Reasonable download size for the default size (9b)

### Adding a new size

1. Add size string to `AVAILABLE_SIZES` array
2. Update `model_for_size()` if the new size should be a size alias target
3. Update size validation logic

### Adding a new flag

1. Add field to `Ollama` struct
2. Wire into `build_run_args()`
3. Handle both direct and sandbox execution paths
4. Add setter method following the existing pattern

## Update Checklist

- [ ] Update the `// provider-updated: YYYY-MM-DD` comment at the top of `ollama.rs`
- [ ] Update `ollama.rs` — new flags in arg builders, model/size defaults, sandbox wrapper
- [ ] Update `ollama_tests.rs` — test new arg combinations, sandbox command construction
- [ ] Update `docs/providers.md` — feature matrix, available sizes, known limitations
- [ ] Update `docs/configuration.md` — if new Ollama-specific config keys added
- [ ] Update `zag-agent/man/run.md` and `zag-agent/man/exec.md` — if command behavior changes
- [ ] Update `README.md` — provider table, model size aliases
- [ ] Update `website/src/components/Providers.tsx` — feature tags, size lists
- [ ] Update `website/src/components/GettingStarted.tsx` — if install method changes
- [ ] If new builder option: update all six bindings (see parity checklist in CLAUDE.md and the `update-bindings` skill)

## Web Discovery Tips

- Ollama releases are frequent but most are minor (UI changes, model additions). Focus on releases that mention new CLI flags for `ollama run`.
- The Ollama releases page at `https://github.com/ollama/ollama/releases` is reliable and shows clear changelogs.
- Model additions (e.g., Gemma 4 in v0.20.0) don't require provider code changes since Ollama accepts any model name without validation.
- As of v0.20.2, Ollama still does **not** have a `--system` flag for `ollama run`. System prompts are still prepended to the user prompt.
- As of v0.20.2, Ollama still does **not** support session resume.

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
2. **Add discovery tips**: If you found useful search queries, source file paths, or workarounds, add them to the "Web Discovery Tips" section.
3. **Update implementation patterns**: If the actual code differs from what's documented here, update the patterns section.
4. **Record known limitations**: Update the version-specific notes in "Web Discovery Tips" (e.g., "as of v0.20.2, no `--system` flag") so future updates don't re-investigate settled questions.
5. **Commit the skill update** along with the provider update so the improvements are preserved.
