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

## Error handling

`ZagBuilder` terminal methods throw a `ZagError` when the underlying `zag`
subprocess fails. The error carries the process exit code and the captured
stderr, which is usually the quickest way to see what the agent CLI complained
about:

```typescript
import { ZagBuilder, ZagError } from "@nlindstedt/zag-agent";

try {
  const output = await new ZagBuilder()
    .provider("claude")
    .timeout("30s")
    .exec("do something");
  console.log(output.result);
} catch (err) {
  if (err instanceof ZagError) {
    console.error("zag failed:");
    console.error("  exit code:", err.exitCode); // number | null
    console.error("  stderr:   ", err.stderr);
    console.error("  message:  ", err.message);
  } else {
    throw err;
  }
}
```

A `ZagError` is thrown when the `zag` binary cannot be spawned, when it exits
non-zero, when its JSON output cannot be parsed, or when a builder method
requires a newer CLI version than the one installed.

### Re-exporting `ZagError` from a barrel file

If you wrap `@nlindstedt/zag-agent` in your own module, a plain re-export does
**not** make `ZagError` usable as a value inside that same file — you can
export it for downstream consumers, but `instanceof ZagError` checks *within*
the barrel file will fail unless you also `import` it:

```typescript
// barrel.ts
import { ZagError } from "@nlindstedt/zag-agent";             // needed for instanceof here
export { ZagBuilder, ZagError } from "@nlindstedt/zag-agent"; // re-export for consumers

export async function run(prompt: string) {
  try {
    return await new ZagBuilder().provider("claude").exec(prompt);
  } catch (err) {
    if (err instanceof ZagError) {
      console.error(err.stderr);
    }
    throw err;
  }
}
```

This is a standard TypeScript/ESM re-export semantics quirk, not a zag-specific
behavior, but it's easy to trip over when setting up error handling for the
first time.

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
