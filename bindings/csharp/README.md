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
| `.Env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
| `.Json()` | Request JSON output |
| `.JsonSchema(schema)` | Validate output against a JSON schema (implies `.Json()`) |
| `.JsonStream()` | Enable streaming NDJSON output |
| `.Worktree(name?)` | Run in an isolated git worktree |
| `.Sandbox(name?)` | Run in a Docker sandbox |
| `.SessionId(uuid)` | Use a specific session ID |
| `.OutputFormat(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.InputFormat(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.ReplayUserMessages()` | Re-emit user messages on stdout (Claude only) |
| `.IncludePartialMessages()` | Include partial message chunks (Claude only) |
| `.MaxTurns(n)` | Set the maximum number of agentic turns |
| `.McpConfig(config)` | MCP server config: JSON string or file path (Claude only) |
| `.ShowUsage()` | Show token usage statistics (JSON output mode) |
| `.Size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.Verbose()` | Enable verbose output |
| `.Quiet()` | Suppress non-essential output |
| `.Debug()` | Enable debug logging |
| `.Bin(path)` | Override the `zag` binary path |

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.ExecAsync(prompt)` | `Task<AgentOutput>` | Run non-interactively, return structured output |
| `.StreamAsync(prompt)` | `IAsyncEnumerable<Event>` | Stream NDJSON events |
| `.ExecStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only) |
| `.RunAsync(prompt?)` | `Task` | Start an interactive session (inherits stdio) |
| `.ResumeAsync(sessionId)` | `Task` | Resume a previous session by ID |
| `.ContinueLastAsync()` | `Task` | Resume the most recent session |

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
