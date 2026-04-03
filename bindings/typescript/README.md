# Zag TypeScript Binding

TypeScript binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- Node.js 18+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Setup

This binding is not published to npm. Use it directly from the source tree:

```bash
cd bindings/typescript
npm install
npm run build
```

## Quick start

```typescript
import { ZagBuilder } from "zag-agent";

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
import { ZagBuilder } from "zag-agent";

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
| `.json()` | Request JSON output |
| `.jsonSchema(schema)` | Validate output against a JSON schema (implies `.json()`) |
| `.jsonStream()` | Enable streaming NDJSON output |
| `.worktree(name?)` | Run in an isolated git worktree |
| `.sandbox(name?)` | Run in a Docker sandbox |
| `.sessionId(uuid)` | Use a specific session ID |
| `.outputFormat(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.inputFormat(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replayUserMessages()` | Re-emit user messages on stdout (Claude only) |
| `.includePartialMessages()` | Include partial message chunks (Claude only) |
| `.maxTurns(n)` | Set the maximum number of agentic turns |
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
