# Zag TypeScript Binding

TypeScript binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- Node.js 18+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Installation

```bash
npm install @nlindstedt/zag-agent
```

### Development setup

To work with the binding from source:

```bash
cd bindings/typescript
npm install
npm run build
```

## Quick start

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

// Non-interactive execution
const output = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .autoApprove()
  .exec("write a hello world program");

console.log(output.result);
```

## Streaming

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

// Stream events as they arrive (NDJSON)
for await (const event of new ZagBuilder().provider("claude").stream("analyze code")) {
  console.log(event.type, event);
}
```

## Builder methods

| Method | Description |
|--------|-------------|
| `.provider(name)` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `.model(name)` | Set model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `.systemPrompt(text)` | Set a system prompt |
| `.root(path)` | Set the working directory |
| `.autoApprove()` | Skip permission prompts |
| `.addDir(path)` | Add an additional directory (chainable) |
| `.file(path)` | Attach a file to the prompt (chainable) |
| `.env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
| `.json()` | Request JSON output |
| `.jsonSchema(schema)` | Validate output against a JSON schema (implies `.json()`) |
| `.worktree(name?)` | Run in an isolated git worktree |
| `.sandbox(name?)` | Run in a Docker sandbox |
| `.sessionId(uuid)` | Use a specific session ID |
| `.outputFormat(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.inputFormat(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replayUserMessages()` | Re-emit user messages on stdout (Claude only) |
| `.includePartialMessages()` | Include partial message chunks (Claude only) |
| `.maxTurns(n)` | Set the maximum number of agentic turns |
| `.timeout(duration)` | Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. |
| `.mcpConfig(config)` | MCP server config: JSON string or file path (Claude only) |
| `.showUsage()` | Show token usage statistics (JSON output mode) |
| `.size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `Promise<AgentOutput>` | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncGenerator<Event>` | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only) |
| `.run(prompt?)` | `Promise<void>` | Start an interactive session (inherits stdio) |
| `.resume(sessionId)` | `Promise<void>` | Resume a previous session by ID |
| `.continueLast()` | `Promise<void>` | Resume the most recent session |
| `.execResume(sessionId, prompt)` | `Promise<AgentOutput>` | Resume a session non-interactively with a follow-up prompt |
| `.execContinue(prompt)` | `Promise<AgentOutput>` | Resume the most recent session non-interactively |
| `.streamResume(sessionId, prompt)` | `AsyncGenerator<Event>` | Resume a session in streaming mode |
| `.streamContinue(prompt)` | `AsyncGenerator<Event>` | Resume the most recent session in streaming mode |

## Version checking

The SDK automatically checks the installed `zag` CLI version before running commands. If you use a builder method that requires a newer CLI version than what's installed, a clear error is thrown:

```
env() requires zag CLI >= 0.6.0, but the installed version is 0.5.0.
Please update the zag binary.
```

The version is detected once (by running `zag --version`) and cached for the lifetime of the process.

| Method | Minimum CLI version |
|--------|-------------------|
| `.env()` | 0.6.0 |
| `.mcpConfig()` | 0.6.0 |

All other methods are available since the initial release (0.2.3).

## Capability checking

The SDK also validates that the configured provider actually supports each
feature-gated builder method before spawning the agent subprocess. If you
call, say, `execStreaming()` on a provider without `streaming_input` support,
a typed `ZagFeatureUnsupportedError` is thrown with an actionable message:

```
execStreaming() is not supported by provider 'ollama' (feature: streaming_input).
Supported providers: claude
```

`ZagFeatureUnsupportedError` extends `ZagError`, so existing `catch (err:
ZagError)` handlers still catch it; you can also branch on it specifically:

```ts
import { ZagFeatureUnsupportedError } from "zag-agent";

try {
  await new ZagBuilder().provider("ollama").addDir("/extra").exec("...");
} catch (err) {
  if (err instanceof ZagFeatureUnsupportedError) {
    console.error(`pick another provider from: ${err.supportedProviders.join(", ")}`);
  }
}
```

Capability data is loaded once per `(bin, provider)` and cached. Checks are
skipped when no provider is set (auto-detect) or when the provider is
`"mock"`.

| Method | Required capability |
|--------|-------------------|
| `.execStreaming()` | `streaming_input` |
| `.worktree()` | `worktree` |
| `.sandbox()` | `sandbox` |
| `.systemPrompt()` | `system_prompt` |
| `.addDir()` | `add_dirs` |

## Discovery

Standalone functions for discovering available providers, models, and capabilities:

```ts
import { listProviders, getCapability, getAllCapabilities, resolveModel } from "zag-agent";

const providers = await listProviders();
const cap = await getCapability("claude");
const all = await getAllCapabilities();
const resolved = await resolveModel("claude", "small"); // { input: "small", resolved: "haiku", is_alias: true }
```

| Function | Description |
|----------|-------------|
| `listProviders(bin?)` | List available provider names |
| `getCapability(provider, bin?)` | Get capabilities for a provider |
| `getAllCapabilities(bin?)` | Get capabilities for all providers |
| `resolveModel(provider, model, bin?)` | Resolve a model alias |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed models. Zero external runtime dependencies — only Node.js built-ins.

## Testing

```bash
npm run build && npm test
```

## See also

- [Python SDK](../python/README.md)
- [C# SDK](../csharp/README.md)
- [Rust API (zag-agent)](../../zag-agent/README.md)
- [All bindings](../README.md)

## License

[MIT](../../LICENSE)
