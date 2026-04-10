# Zag C# Binding

C# binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- .NET 8.0+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Installation

```bash
dotnet add package Zag
```

## Quick start

```csharp
using Zag;

var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove()
    .ExecAsync("write a hello world program");

Console.WriteLine(output.Result);
```

## Streaming

```csharp
using Zag;

await foreach (var evt in new ZagBuilder().Provider("claude").StreamAsync("analyze code"))
{
    Console.WriteLine($"{evt.Type}: {evt}");
}
```

## Builder methods

| Method | Description |
|--------|-------------|
| `.Provider(name)` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `.Model(name)` | Set model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `.SystemPrompt(text)` | Set a system prompt |
| `.Root(path)` | Set the working directory |
| `.AutoApprove()` | Skip permission prompts |
| `.AddDir(path)` | Add an additional directory (chainable) |
| `.File(path)` | Attach a file to the prompt (chainable) |
| `.Env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
| `.Json()` | Request JSON output |
| `.JsonSchema(schema)` | Validate output against a JSON schema (implies `.Json()`) |
| `.Worktree(name?)` | Run in an isolated git worktree |
| `.Sandbox(name?)` | Run in a Docker sandbox |
| `.SessionId(uuid)` | Use a specific session ID |
| `.OutputFormat(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.InputFormat(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.ReplayUserMessages()` | Re-emit user messages on stdout (Claude only) |
| `.IncludePartialMessages()` | Include partial message chunks (Claude only) |
| `.MaxTurns(n)` | Set the maximum number of agentic turns |
| `.Timeout(duration)` | Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. |
| `.McpConfig(config)` | MCP server config: JSON string or file path (Claude only) |
| `.ShowUsage()` | Show token usage statistics (JSON output mode) |
| `.Size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.Verbose()` | Enable verbose output |
| `.Quiet()` | Suppress non-essential output |
| `.Debug()` | Enable debug logging |
| `.Bin(path)` | Override the `zag` binary path |

### Provider support for streaming / MCP flags

Four builder methods that toggle streaming I/O details and per-invocation MCP configuration are only honored by the Claude provider. Passing them to any other provider is a no-op.

| Method | Claude | Codex | Gemini | Copilot | Ollama |
|--------|--------|-------|--------|---------|--------|
| `.InputFormat()` | Yes | No | No | No | No |
| `.ReplayUserMessages()` | Yes | No | No | No | No |
| `.IncludePartialMessages()` | Yes | No | No | No | No |
| `.McpConfig()` | Yes | No | No | No | No |

`.ExecStreaming()` is Claude-only and always sets `-i stream-json`, `-o stream-json`, and `--replay-user-messages`. By default it emits **one `assistant_message` event per complete assistant turn** — you get one event when the model finishes speaking, not a stream of token chunks. Call `.IncludePartialMessages(true)` to receive token-level partial `assistant_message` chunks instead. The default stays `false` so existing callers that render whole-turn bubbles are not broken.

At the end of every agent turn the session emits a **`turn_complete`** event (C# type `TurnCompleteEvent`) carrying the provider's `stop_reason` (`end_turn`, `tool_use`, `max_tokens`, `stop_sequence`, or `null`), a zero-based monotonic `turn_index`, and the turn's `usage`. A per-turn `result` event fires immediately after. New code should key turn-boundary UI off `turn_complete` — it is the authoritative signal and carries richer metadata than `result`. `result` continues to fire per-turn for backward compatibility.

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.ExecAsync(prompt)` | `Task<AgentOutput>` | Run non-interactively, return structured output |
| `.StreamAsync(prompt)` | `IAsyncEnumerable<Event>` | Stream NDJSON events |
| `.ExecStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only). Emits one `assistant_message` event per complete turn; pair with `.IncludePartialMessages(true)` for token-level chunks. |
| `.RunAsync(prompt?)` | `Task` | Start an interactive session (inherits stdio) |
| `.ResumeAsync(sessionId)` | `Task` | Resume a previous session by ID |
| `.ContinueLastAsync()` | `Task` | Resume the most recent session |
| `.ExecResumeAsync(sessionId, prompt)` | `Task<AgentOutput>` | Resume a session non-interactively with a follow-up prompt |
| `.ExecContinueAsync(prompt)` | `Task<AgentOutput>` | Resume the most recent session non-interactively |
| `.StreamResumeAsync(sessionId, prompt)` | `IAsyncEnumerable<Event>` | Resume a session in streaming mode |
| `.StreamContinueAsync(prompt)` | `IAsyncEnumerable<Event>` | Resume the most recent session in streaming mode |

## Version checking

The SDK automatically checks the installed `zag` CLI version before running commands. If you use a builder method that requires a newer CLI version than what's installed, a clear error is raised:

```
Env() requires zag CLI >= 0.6.0, but the installed version is 0.5.0.
Please update the zag binary.
```

The version is detected once (by running `zag --version`) and cached for the lifetime of the process.

| Method | Minimum CLI version |
|--------|-------------------|
| `.Env()` | 0.6.0 |
| `.McpConfig()` | 0.6.0 |

All other methods are available since the initial release (0.2.3).

## Discovery

Static async methods for discovering available providers, models, and capabilities:

```csharp
using Zag;

string[] providers = await ZagDiscover.ListProvidersAsync();
ProviderCapability cap = await ZagDiscover.GetCapabilityAsync("claude");
ProviderCapability[] all = await ZagDiscover.GetAllCapabilitiesAsync();
ResolvedModel resolved = await ZagDiscover.ResolveModelAsync("claude", "small");
```

| Method | Description |
|--------|-------------|
| `ZagDiscover.ListProvidersAsync(bin?, ct?)` | List available provider names |
| `ZagDiscover.GetCapabilityAsync(provider, bin?, ct?)` | Get capabilities for a provider |
| `ZagDiscover.GetAllCapabilitiesAsync(bin?, ct?)` | Get capabilities for all providers |
| `ZagDiscover.ResolveModelAsync(provider, model, bin?, ct?)` | Resolve a model alias |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed models. Zero external dependencies — only the .NET standard library.

## Testing

```bash
dotnet test
```

## See also

- [TypeScript SDK](../typescript/README.md)
- [Python SDK](../python/README.md)
- [Rust API (zag-agent)](../../zag-agent/README.md)
- [All bindings](../README.md)

## License

[MIT](../../LICENSE)
