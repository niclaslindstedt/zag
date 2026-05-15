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

### Bidirectional streaming sessions

`execStreaming()` returns a `StreamingSession` with piped stdin for sending
user messages mid-flight (Claude only). Call `.close({ timeout })` when you
are done to shut the session down gracefully — it closes stdin, waits for
the child to exit, escalates to SIGTERM, and finally SIGKILL if the child
refuses to stop:

```typescript
const session = await new ZagBuilder()
  .provider("claude")
  .execStreaming("initial prompt");

session.sendUserMessage("follow-up question");

for await (const event of session.events()) {
  console.log(event.type);
}

// Graceful shutdown — replaces the closeInput/wait/terminate/kill dance.
await session.close({ timeout: "5s" });
```

### Automatic orphan cleanup

Long-running Node servers (Next.js, Express) can leak `zag`/agent
subprocesses if the parent process dies unexpectedly. Opt in to
`.autoCleanup()` on the builder to install process-wide shutdown handlers
(`exit`, `SIGINT`, `SIGTERM`, `SIGHUP`, `uncaughtException`) that SIGTERM
every tracked live session:

```typescript
const session = await new ZagBuilder()
  .provider("claude")
  .autoCleanup()
  .execStreaming("hello");
```

Off by default so the SDK imposes no global side effects on consumers that
don't need them.

## Builder methods

| Method | Description |
|--------|-------------|
| `.provider(name)` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `.model(name)` | Set model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `.systemPrompt(text)` | Set a system prompt |
| `.root(path)` | Set the working directory |
| `.autoApprove()` | Skip permission prompts |
| `.headless()` | Hide the provider's TUI by attaching it to a private PTY (requires `--exit` and `--auto-approve` at the CLI) |
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
| `.autoCleanup(enabled?)` | Install process-wide shutdown handlers that SIGTERM tracked `StreamingSession` children on parent exit (opt-in) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

### Provider support for streaming / MCP flags

Four builder methods that toggle streaming I/O details and per-invocation MCP configuration are only honored by the Claude provider. Passing them to any other provider is a no-op.

| Method | Claude | Codex | Gemini | Copilot | Ollama |
|--------|--------|-------|--------|---------|--------|
| `.inputFormat()` | Yes | No | No | No | No |
| `.replayUserMessages()` | Yes | No | No | No | No |
| `.includePartialMessages()` | Yes | No | No | No | No |
| `.mcpConfig()` | Yes | No | No | No | No |

`.execStreaming()` is Claude-only and always sets `-i stream-json`, `-o stream-json`, and `--replay-user-messages`. By default it emits **one `assistant_message` event per complete assistant turn** — you get one event when the model finishes speaking, not a stream of token chunks. Call `.includePartialMessages(true)` to receive token-level partial `assistant_message` chunks instead. The default stays `false` so existing callers that render whole-turn bubbles are not broken.

At the end of every agent turn the session emits a **`turn_complete`** event carrying the provider's `stop_reason` (`end_turn`, `tool_use`, `max_tokens`, `stop_sequence`, or `null`), a zero-based monotonic `turn_index`, and the turn's `usage`. A per-turn `result` event fires immediately after. New code should key turn-boundary UI off `turn_complete` — it is the authoritative signal and carries richer metadata than `result`. `result` continues to fire per-turn for backward compatibility.

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `Promise<AgentOutput>` | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncGenerator<Event>` | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only). Emits one `assistant_message` event per complete turn; pair with `.includePartialMessages(true)` for token-level chunks. |
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
