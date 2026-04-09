# Zag C# Binding -- Reference Manual

C# SDK for programmatic access to AI coding agents through the zag unified CLI.

- **Package:** `Zag` (NuGet)
- **Import:** `using Zag;`
- **Prerequisites:** .NET 8.0+, `zag` CLI binary on `PATH` or `ZAG_BIN` environment variable
- **Install:** `dotnet add package Zag`
- **Dependencies:** None (uses `System.Text.Json` from the .NET standard library)

---

## Quick Start

```csharp
using Zag;

// One-shot execution
var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove()
    .ExecAsync("write a hello world program");

Console.WriteLine(output.Result);        // agent's final text
Console.WriteLine(output.SessionId);     // session UUID
Console.WriteLine(output.TotalCostUsd);  // cost (if available)
```

---

## Builder API -- `ZagBuilder`

**Constructor:** `new ZagBuilder()`

All configuration methods return `ZagBuilder` for chaining. Terminal methods are async (suffix `Async`) except `ExecStreaming` which returns a `StreamingSession`.

### Configuration Methods

Every method below returns `ZagBuilder`.

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `Bin` | `Bin(string path)` | N/A | Override the zag binary path (default: `ZAG_BIN` env or `"zag"`) |
| `Provider` | `Provider(string name)` | `-p` | Set provider (`"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`) |
| `Model` | `Model(string name)` | `--model` | Model name or size alias (`"sonnet"`, `"opus"`, `"small"`, `"large"`) |
| `SystemPrompt` | `SystemPrompt(string text)` | `--system-prompt` | System prompt to configure agent behavior |
| `Root` | `Root(string path)` | `--root` | Working directory for the agent |
| `AutoApprove` | `AutoApprove(bool a = true)` | `--auto-approve` | Skip permission prompts |
| `AddDir` | `AddDir(string path)` | `--add-dir` | Additional directory for the agent (repeatable) |
| `File` | `File(string path)` | `--file` | Attach a file to the prompt (repeatable) |
| `Env` | `Env(string key, string value)` | `--env` | Environment variable for the agent subprocess (repeatable, CLI >= 0.6.0) |
| `Json` | `Json()` | `--json` | Request JSON output |
| `JsonSchema` | `JsonSchema(object schema)` | `--json-schema` | JSON schema for structured output validation (implies `Json()`) |
| `JsonStream` | `JsonStream()` | `--json-stream` | Enable NDJSON streaming output |
| `Worktree` | `Worktree(string? name = null)` | `-w` | Git worktree isolation; optional name |
| `Sandbox` | `Sandbox(string? name = null)` | `--sandbox` | Docker sandbox isolation; optional name |
| `Verbose` | `Verbose(bool v = true)` | `--verbose` | Verbose output |
| `Quiet` | `Quiet(bool q = true)` | `--quiet` | Suppress non-essential output |
| `Debug` | `Debug(bool d = true)` | `--debug` | Debug logging |
| `SessionId` | `SessionId(string uuid)` | `--session` | Pre-set a session ID (UUID) |
| `OutputFormat` | `OutputFormat(string fmt)` | `-o` | Output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `InputFormat` | `InputFormat(string fmt)` | `-i` | Input format (`"text"`, `"stream-json"` -- Claude only) |
| `ReplayUserMessages` | `ReplayUserMessages(bool r = true)` | `--replay-user-messages` | Re-emit user messages on stdout (Claude only) |
| `IncludePartialMessages` | `IncludePartialMessages(bool i = true)` | `--include-partial-messages` | Include partial message chunks in streaming (Claude only) |
| `MaxTurns` | `MaxTurns(int n)` | `--max-turns` | Maximum number of agentic turns |
| `Timeout` | `Timeout(string duration)` | `--timeout` | Timeout duration (e.g. `"30s"`, `"5m"`, `"1h"`); kills agent if exceeded |
| `McpConfig` | `McpConfig(string config)` | `--mcp-config` | MCP server config: JSON string or file path (Claude only, CLI >= 0.6.0) |
| `ShowUsage` | `ShowUsage(bool s = true)` | `--show-usage` | Show token usage statistics (JSON output mode) |
| `Size` | `Size(string size)` | `--size` | Ollama model parameter size (e.g. `"2b"`, `"9b"`, `"35b"` -- Ollama only) |

---

## Terminal Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `ExecAsync` | `Task<AgentOutput> ExecAsync(string prompt, CancellationToken ct = default)` | Non-interactive execution; returns structured output |
| `StreamAsync` | `IAsyncEnumerable<Event> StreamAsync(string prompt, CancellationToken ct = default)` | Stream NDJSON events as they arrive |
| `ExecStreaming` | `Task<StreamingSession> ExecStreaming(string prompt)` | Bidirectional streaming with piped stdin/stdout (Claude only) |
| `RunAsync` | `Task RunAsync(string? prompt = null, CancellationToken ct = default)` | Interactive session (inherits stdio) |
| `ResumeAsync` | `Task ResumeAsync(string sessionId, CancellationToken ct = default)` | Resume a previous session by ID |
| `ContinueLastAsync` | `Task ContinueLastAsync(CancellationToken ct = default)` | Resume the most recent session |
| `ExecResumeAsync` | `Task<AgentOutput> ExecResumeAsync(string sessionId, string prompt, CancellationToken ct = default)` | Resume a session non-interactively with a follow-up prompt |
| `ExecContinueAsync` | `Task<AgentOutput> ExecContinueAsync(string prompt, CancellationToken ct = default)` | Resume the most recent session non-interactively |
| `StreamResumeAsync` | `IAsyncEnumerable<Event> StreamResumeAsync(string sessionId, string prompt, CancellationToken ct = default)` | Resume a session in streaming mode |
| `StreamContinueAsync` | `IAsyncEnumerable<Event> StreamContinueAsync(string prompt, CancellationToken ct = default)` | Resume the most recent session in streaming mode |

All async methods accept a `CancellationToken`. `ExecStreaming` is async (returns `Task<StreamingSession>`) but does not take a `CancellationToken` at construction -- use `StreamingSession.Events(ct)` and `StreamingSession.Terminate()` instead.

---

## StreamingSession

Returned by `ExecStreaming()`. Implements `IDisposable`.

```csharp
public sealed class StreamingSession : IDisposable
{
    // Send a raw NDJSON line to the agent's stdin
    void Send(string message);

    // Send a structured user message to the agent
    void SendUserMessage(string content);

    // Close stdin to signal no more input
    void CloseInput();

    // Async iterator over parsed Event objects from stdout
    IAsyncEnumerable<Event> Events(CancellationToken ct = default);

    // Whether the underlying process is still running
    bool IsRunning { get; }

    // Terminate the underlying process (no-op if already exited)
    void Terminate();

    // Wait for the process to exit; throws ZagException on non-zero exit
    Task WaitAsync(CancellationToken ct = default);

    void Dispose();
}
```

`SendUserMessage(content)` serializes to `{"type":"user_message","content":"..."}` and writes it as a single NDJSON line. Use `Send(message)` for arbitrary NDJSON payloads.

`WaitAsync()` calls `CloseInput()` internally before waiting.

---

## Types

### AgentOutput

Returned by `ExecAsync()`. Contains the full session result.

```csharp
public record AgentOutput
{
    string Agent { get; init; }          // provider name (e.g. "claude")
    string SessionId { get; init; }      // session UUID
    List<Event> Events { get; init; }    // all session events
    string? Result { get; init; }        // final text result
    bool IsError { get; init; }          // whether the session ended in error
    int? ExitCode { get; init; }         // process exit code
    string? ErrorMessage { get; init; }  // error description (if IsError)
    double? TotalCostUsd { get; init; }  // total cost in USD (if available)
    Usage? Usage { get; init; }          // aggregate token usage
}
```

### Usage

```csharp
public record Usage
{
    long InputTokens { get; init; }
    long OutputTokens { get; init; }
    long? CacheReadTokens { get; init; }
    long? CacheCreationTokens { get; init; }
    int? WebSearchRequests { get; init; }
    int? WebFetchRequests { get; init; }
}
```

### Events

Events are deserialized from NDJSON using a `"type"` discriminator field. The base class is abstract:

```csharp
public abstract record Event
{
    public abstract string Type { get; }
}
```

Concrete event types:

| Type string | C# class | Key properties |
|-------------|----------|----------------|
| `"init"` | `InitEvent` | `Model`, `Tools` (List\<string\>), `WorkingDirectory`, `Metadata` (Dictionary\<string, JsonElement\>) |
| `"user_message"` | `UserMessageEvent` | `Content` (List\<ContentBlock\>) |
| `"assistant_message"` | `AssistantMessageEvent` | `Content` (List\<ContentBlock\>), `Usage` |
| `"tool_execution"` | `ToolExecutionEvent` | `ToolName`, `ToolId`, `Input` (JsonElement?), `Result` (ToolResult) |
| `"result"` | `ResultEvent` | `Success`, `Message`, `DurationMs`, `NumTurns` |
| `"error"` | `ErrorEvent` | `Message`, `Details` (JsonElement?) |
| `"permission_request"` | `PermissionRequestEvent` | `ToolName`, `Description`, `Granted` |

Unknown event types throw `JsonException` during deserialization.

### Content Blocks

```csharp
public abstract record ContentBlock
{
    public abstract string Type { get; }
}

public record TextBlock : ContentBlock
{
    // Type => "text"
    string Text { get; init; }
}

public record ToolUseBlock : ContentBlock
{
    // Type => "tool_use"
    string Id { get; init; }
    string Name { get; init; }
    JsonElement? Input { get; init; }
}
```

### ToolResult

```csharp
public record ToolResult
{
    bool Success { get; init; }
    string? Output { get; init; }
    string? Error { get; init; }
    JsonElement? Data { get; init; }
}
```

### ZagException

Thrown when the zag subprocess exits with a non-zero code or output cannot be parsed.

```csharp
public class ZagException : Exception
{
    int? ExitCode { get; }
    string Stderr { get; }

    ZagException(string message, int? exitCode, string stderr);
}
```

---

## Discovery API -- `ZagDiscover`

Static async methods for querying provider capabilities via the `zag discover` subcommand.

```csharp
public static class ZagDiscover
{
    // List all available provider names
    static Task<string[]> ListProvidersAsync(
        string? bin = null, CancellationToken ct = default);

    // Get capability declarations for a specific provider
    static Task<ProviderCapability> GetCapabilityAsync(
        string provider, string? bin = null, CancellationToken ct = default);

    // Get capability declarations for all providers
    static Task<ProviderCapability[]> GetAllCapabilitiesAsync(
        string? bin = null, CancellationToken ct = default);

    // Resolve a model alias (e.g. "small" -> actual model name)
    static Task<ResolvedModel> ResolveModelAsync(
        string provider, string model, string? bin = null, CancellationToken ct = default);
}
```

### Discovery Types

```csharp
public record ProviderCapability
{
    string Provider { get; init; }
    string DefaultModel { get; init; }
    List<string> AvailableModels { get; init; }
    SizeMappings SizeMappings { get; init; }
    Features Features { get; init; }
}

public record ResolvedModel
{
    string Input { get; init; }       // the alias or name you passed in
    string Resolved { get; init; }    // the resolved model identifier
    bool IsAlias { get; init; }       // whether the input was an alias
    string Provider { get; init; }    // provider name
}

public record SizeMappings
{
    string Small { get; init; }
    string Medium { get; init; }
    string Large { get; init; }
}

public record Features
{
    FeatureSupport Interactive { get; init; }
    FeatureSupport NonInteractive { get; init; }
    FeatureSupport Resume { get; init; }
    FeatureSupport ResumeWithPrompt { get; init; }
    SessionLogSupport SessionLogs { get; init; }
    FeatureSupport JsonOutput { get; init; }
    FeatureSupport StreamJson { get; init; }
    FeatureSupport JsonSchema { get; init; }
    FeatureSupport InputFormat { get; init; }
    FeatureSupport StreamingInput { get; init; }
    FeatureSupport Worktree { get; init; }
    FeatureSupport Sandbox { get; init; }
    FeatureSupport SystemPrompt { get; init; }
    FeatureSupport AutoApprove { get; init; }
    FeatureSupport Review { get; init; }
    FeatureSupport AddDirs { get; init; }
    FeatureSupport MaxTurns { get; init; }
}

public record FeatureSupport
{
    bool Supported { get; init; }
    bool Native { get; init; }
}

public record SessionLogSupport
{
    bool Supported { get; init; }
    bool Native { get; init; }
    string? Completeness { get; init; }
}
```

---

## Examples

### ExecAsync -- Non-Interactive Execution

```csharp
using Zag;

var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .SystemPrompt("You are a senior C# developer.")
    .Root("/home/user/myproject")
    .AutoApprove()
    .MaxTurns(5)
    .ExecAsync("refactor the Program.cs file to use dependency injection");

if (output.IsError)
{
    Console.Error.WriteLine($"Error: {output.ErrorMessage}");
}
else
{
    Console.WriteLine(output.Result);
    Console.WriteLine($"Session: {output.SessionId}");
    Console.WriteLine($"Cost: ${output.TotalCostUsd:F4}");

    if (output.Usage is { } usage)
    {
        Console.WriteLine($"Tokens: {usage.InputTokens} in, {usage.OutputTokens} out");
    }
}
```

### StreamAsync -- Streaming Events

```csharp
using Zag;

var builder = new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove();

await foreach (var evt in builder.StreamAsync("analyze this codebase"))
{
    switch (evt)
    {
        case InitEvent init:
            Console.WriteLine($"Model: {init.Model}, Tools: {string.Join(", ", init.Tools)}");
            break;

        case AssistantMessageEvent msg:
            foreach (var block in msg.Content)
            {
                if (block is TextBlock text)
                    Console.Write(text.Text);
            }
            break;

        case ToolExecutionEvent tool:
            Console.WriteLine($"[tool] {tool.ToolName}: {(tool.Result.Success ? "ok" : "fail")}");
            break;

        case ResultEvent result:
            Console.WriteLine($"\nDone in {result.DurationMs}ms, {result.NumTurns} turns");
            break;

        case ErrorEvent error:
            Console.Error.WriteLine($"Error: {error.Message}");
            break;
    }
}
```

### ExecStreaming -- Bidirectional Streaming (Claude Only)

```csharp
using Zag;

await using var session = await new ZagBuilder()
    .Provider("claude")
    .AutoApprove()
    .ExecStreaming("You are a coding assistant. Wait for instructions.");

// Read events in the background
var readTask = Task.Run(async () =>
{
    await foreach (var evt in session.Events())
    {
        if (evt is AssistantMessageEvent msg)
        {
            foreach (var block in msg.Content)
            {
                if (block is TextBlock text)
                    Console.Write(text.Text);
            }
            Console.WriteLine();
        }
    }
});

// Send follow-up messages
session.SendUserMessage("List all files in the current directory");

// Wait for response, then send another
await Task.Delay(5000);
session.SendUserMessage("Now count the lines of code");

// Signal no more input and wait for completion
session.CloseInput();
await readTask;
await session.WaitAsync();
```

### JsonSchema -- Structured Output

```csharp
using Zag;

var schema = new
{
    type = "object",
    properties = new
    {
        summary = new { type = "string" },
        issues = new
        {
            type = "array",
            items = new
            {
                type = "object",
                properties = new
                {
                    file = new { type = "string" },
                    line = new { type = "integer" },
                    severity = new { type = "string", @enum = new[] { "error", "warning", "info" } },
                    message = new { type = "string" }
                },
                required = new[] { "file", "severity", "message" }
            }
        }
    },
    required = new[] { "summary", "issues" }
};

var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove()
    .JsonSchema(schema)
    .ExecAsync("review the code in src/ for bugs and style issues");

// output.Result contains JSON conforming to the schema
Console.WriteLine(output.Result);
```

### Error Handling

```csharp
using Zag;

try
{
    var output = await new ZagBuilder()
        .Provider("claude")
        .Timeout("30s")
        .ExecAsync("do something");

    if (output.IsError)
    {
        // Agent completed but reported an error
        Console.Error.WriteLine($"Agent error: {output.ErrorMessage}");
        Console.Error.WriteLine($"Exit code: {output.ExitCode}");
    }
}
catch (ZagException ex)
{
    // Process-level failure (non-zero exit, binary not found, parse failure)
    Console.Error.WriteLine($"Zag failed: {ex.Message}");
    Console.Error.WriteLine($"Exit code: {ex.ExitCode}");
    Console.Error.WriteLine($"Stderr: {ex.Stderr}");
}
```

### Cancellation

```csharp
using Zag;

using var cts = new CancellationTokenSource(TimeSpan.FromMinutes(2));

try
{
    var output = await new ZagBuilder()
        .Provider("claude")
        .AutoApprove()
        .ExecAsync("long running task", cts.Token);
}
catch (OperationCanceledException)
{
    Console.WriteLine("Operation was cancelled");
}
```

### Discovery

```csharp
using Zag;

// List all providers
string[] providers = await ZagDiscover.ListProvidersAsync();
Console.WriteLine($"Available: {string.Join(", ", providers)}");

// Get capabilities for a specific provider
var cap = await ZagDiscover.GetCapabilityAsync("claude");
Console.WriteLine($"Default model: {cap.DefaultModel}");
Console.WriteLine($"Models: {string.Join(", ", cap.AvailableModels)}");
Console.WriteLine($"Size mappings: small={cap.SizeMappings.Small}, medium={cap.SizeMappings.Medium}, large={cap.SizeMappings.Large}");
Console.WriteLine($"Supports JSON output: {cap.Features.JsonOutput.Supported}");
Console.WriteLine($"Supports sandbox: {cap.Features.Sandbox.Supported}");

// Resolve a model alias
var resolved = await ZagDiscover.ResolveModelAsync("claude", "small");
Console.WriteLine($"{resolved.Input} -> {resolved.Resolved} (alias: {resolved.IsAlias})");
```

### Environment Variables and MCP Config

```csharp
using Zag;

// Pass environment variables (requires CLI >= 0.6.0)
var output = await new ZagBuilder()
    .Provider("claude")
    .AutoApprove()
    .Env("DATABASE_URL", "postgres://localhost/mydb")
    .Env("API_KEY", "sk-xxx")
    .ExecAsync("connect to the database and list tables");

// MCP server config (requires CLI >= 0.6.0, Claude only)
var output2 = await new ZagBuilder()
    .Provider("claude")
    .AutoApprove()
    .McpConfig("/path/to/mcp-config.json")
    .ExecAsync("use the configured MCP tools to fetch data");
```

### Worktree and Sandbox Isolation

```csharp
using Zag;

// Run in an isolated git worktree (auto-named)
var output = await new ZagBuilder()
    .Provider("claude")
    .AutoApprove()
    .Worktree()
    .ExecAsync("make experimental changes to the codebase");

// Run in a named Docker sandbox
var output2 = await new ZagBuilder()
    .Provider("claude")
    .AutoApprove()
    .Sandbox("my-sandbox")
    .ExecAsync("install dependencies and run tests");
```

---

## Internals

### CLI Argument Construction

The builder constructs CLI arguments in two groups:

**Global args** (applied before the subcommand): provider, model, system-prompt, root, auto-approve, add-dir, file, env, worktree, sandbox, verbose, quiet, debug, session, max-turns, mcp-config, show-usage, size.

**Exec args** (applied after `exec`): json, json-schema, json-stream, output format, input format, replay-user-messages, include-partial-messages, timeout, then the prompt.

### Implicit Defaults

- `ExecAsync()` adds `-o json` automatically unless `OutputFormat()`, `JsonStream()`, or `Json()` was set. This ensures output is parseable as `AgentOutput`.
- `StreamAsync()` always adds `--json-stream`.
- `ExecStreaming()` forces `-i stream-json -o stream-json --replay-user-messages`. If `IncludePartialMessages()` was called, `--include-partial-messages` is also added.
- `RunAsync()` does not redirect stdio (the process inherits the terminal).
- `JsonSchema()` calls `Json()` internally, setting `--json` in addition to `--json-schema`.

### Version Checking

Before any terminal method executes, the SDK runs `zag --version` to detect the installed CLI version. The result is cached per binary path for the lifetime of the process. If a configured method requires a newer CLI version, `ZagException` is thrown with an actionable message before the subprocess is started.

### Process Management

All methods spawn the `zag` CLI as a child process via `System.Diagnostics.Process`. Standard output and standard error are captured where applicable. Non-zero exit codes throw `ZagException` with the exit code and stderr content.

### JSON Deserialization

All JSON parsing uses `System.Text.Json` with `PropertyNameCaseInsensitive = true`. Event and ContentBlock types use custom `JsonConverter<T>` implementations that dispatch on the `"type"` discriminator field.

---

## Version Requirements

| Method | Minimum CLI Version |
|--------|-------------------|
| `Env()` | 0.6.0 |
| `McpConfig()` | 0.6.0 |
| All others | 0.2.3 |

---

## Provider Notes

**Claude only:**
- `InputFormat()` -- set input format to `"stream-json"` for bidirectional streaming
- `ReplayUserMessages()` -- re-emit user messages from stdin on stdout
- `IncludePartialMessages()` -- include partial message chunks in streaming output
- `McpConfig()` -- MCP server configuration (JSON string or file path)
- `ExecStreaming()` -- bidirectional streaming via piped stdin/stdout

**Ollama only:**
- `Size()` -- set the model parameter size (e.g. `"2b"`, `"9b"`, `"35b"`)

**All providers:**
- All other builder methods work across all providers
- Use `ZagDiscover.GetCapabilityAsync()` to check feature support at runtime before calling provider-specific methods
