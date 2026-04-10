# Zag Java Binding -- Reference Manual

Java SDK for zag, a unified CLI for AI coding agents. Spawns the `zag` CLI as a subprocess and parses structured JSON output into typed Java records.

---

## Quick Start

### Prerequisites

- Java 17+
- Maven 3.8+
- The `zag` CLI installed and on `PATH`, or its location set via the `ZAG_BIN` environment variable

### Installation

Maven:

```xml
<dependency>
    <groupId>io.zag</groupId>
    <artifactId>zag</artifactId>
    <version>0.2.4</version>
</dependency>
```

Gradle:

```groovy
implementation 'io.zag:zag:0.2.4'
```

The SDK depends on Jackson (`com.fasterxml.jackson.core:jackson-databind:2.17.0`) for JSON parsing. It is declared as a transitive dependency and pulled in automatically.

### Basic Example

```java
import io.zag.ZagBuilder;
import io.zag.AgentOutput;

AgentOutput output = new ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program");

System.out.println(output.result());
```

All methods are synchronous. Every terminal method blocks until the subprocess completes (or, for streaming, until the iterator is exhausted).

---

## Builder API

### Constructor

```java
ZagBuilder builder = new ZagBuilder();
```

Creates a builder with no configuration. The binary path defaults to the `ZAG_BIN` environment variable, falling back to `"zag"`.

### Configuration Methods

All setters return `ZagBuilder` for method chaining. Boolean options have both a no-arg overload (sets `true`) and a `boolean` overload.

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `bin` | `ZagBuilder bin(String path)` | N/A | Override the zag binary path. Default: `ZAG_BIN` env or `"zag"`. |
| `provider` | `ZagBuilder provider(String name)` | `-p` | Set the provider (`"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`). |
| `model` | `ZagBuilder model(String name)` | `--model` | Model name or size alias (`"small"`, `"medium"`, `"large"`, `"sonnet"`, etc.). |
| `systemPrompt` | `ZagBuilder systemPrompt(String text)` | `--system-prompt` | System prompt to configure agent behavior. |
| `root` | `ZagBuilder root(String path)` | `--root` | Working directory for the agent. |
| `autoApprove` | `ZagBuilder autoApprove()` | `--auto-approve` | Skip permission prompts. |
| `autoApprove` | `ZagBuilder autoApprove(boolean v)` | `--auto-approve` | Enable or disable auto-approve. |
| `addDir` | `ZagBuilder addDir(String path)` | `--add-dir` | Add an additional directory. Repeatable -- each call appends. |
| `file` | `ZagBuilder file(String path)` | `--file` | Attach a file to the prompt. Repeatable -- each call appends. |
| `env` | `ZagBuilder env(String key, String value)` | `--env` | Set an environment variable for the subprocess. Repeatable. Requires CLI >= 0.6.0. |
| `json` | `ZagBuilder json()` | `--json` | Request JSON output. |
| `jsonSchema` | `ZagBuilder jsonSchema(Object schema)` | `--json-schema` | JSON schema for structured output validation. Implies `json()`. The object is serialized via Jackson. |
| `worktree` | `ZagBuilder worktree()` | `-w` | Run in an isolated git worktree (auto-named). |
| `worktree` | `ZagBuilder worktree(String name)` | `-w <name>` | Run in a named git worktree. |
| `sandbox` | `ZagBuilder sandbox()` | `--sandbox` | Run in a Docker sandbox (auto-named). |
| `sandbox` | `ZagBuilder sandbox(String name)` | `--sandbox <name>` | Run in a named Docker sandbox. |
| `verbose` | `ZagBuilder verbose()` | `--verbose` | Enable verbose output. |
| `verbose` | `ZagBuilder verbose(boolean v)` | `--verbose` | Enable or disable verbose output. |
| `quiet` | `ZagBuilder quiet()` | `--quiet` | Suppress non-essential output. |
| `quiet` | `ZagBuilder quiet(boolean v)` | `--quiet` | Enable or disable quiet mode. |
| `debug` | `ZagBuilder debug()` | `--debug` | Enable debug logging. |
| `debug` | `ZagBuilder debug(boolean v)` | `--debug` | Enable or disable debug logging. |
| `sessionId` | `ZagBuilder sessionId(String uuid)` | `--session` | Use a specific session ID. |
| `outputFormat` | `ZagBuilder outputFormat(String fmt)` | `-o` | Output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`). |
| `inputFormat` | `ZagBuilder inputFormat(String fmt)` | `-i` | Input format (`"text"`, `"stream-json"`). Claude only. |
| `replayUserMessages` | `ZagBuilder replayUserMessages()` | `--replay-user-messages` | Re-emit user messages on stdout. Claude only. |
| `replayUserMessages` | `ZagBuilder replayUserMessages(boolean v)` | `--replay-user-messages` | Enable or disable replay. Claude only. |
| `includePartialMessages` | `ZagBuilder includePartialMessages()` | `--include-partial-messages` | Include partial message chunks. Claude only. |
| `includePartialMessages` | `ZagBuilder includePartialMessages(boolean v)` | `--include-partial-messages` | Enable or disable partial messages. Claude only. |
| `maxTurns` | `ZagBuilder maxTurns(int n)` | `--max-turns` | Maximum number of agentic turns. |
| `timeout` | `ZagBuilder timeout(String duration)` | `--timeout` | Timeout duration (e.g., `"30s"`, `"5m"`, `"1h"`). Kills the agent if exceeded. |
| `mcpConfig` | `ZagBuilder mcpConfig(String config)` | `--mcp-config` | MCP server config: JSON string or file path. Claude only. Requires CLI >= 0.6.0. |
| `showUsage` | `ZagBuilder showUsage()` | `--show-usage` | Show token usage statistics in JSON output. |
| `showUsage` | `ZagBuilder showUsage(boolean v)` | `--show-usage` | Enable or disable usage stats. |
| `size` | `ZagBuilder size(String size)` | `--size` | Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`). Ollama only. |

---

## Terminal Methods

Terminal methods consume the builder configuration and execute the agent. All are synchronous and block until completion.

| Method | Signature | Description |
|--------|-----------|-------------|
| `exec` | `AgentOutput exec(String prompt) throws ZagException` | Run non-interactively. Returns structured output. Defaults to `-o json`. |
| `stream` | `Iterable<Event> stream(String prompt) throws ZagException` | Stream NDJSON events. Returns an `Iterable<Event>` backed by a lazy iterator that reads lines from stdout. Adds `-o stream-json`. |
| `execStreaming` | `StreamingSession execStreaming(String prompt) throws ZagException` | Bidirectional streaming. Claude only. Automatically sets `-i stream-json -o stream-json --replay-user-messages`. |
| `run` | `void run(String prompt) throws ZagException` | Interactive session with inherited stdio. |
| `run` | `void run() throws ZagException` | Interactive session without an initial prompt. |
| `resume` | `void resume(String sessionId) throws ZagException` | Resume a previous session by ID. |
| `continueLast` | `void continueLast() throws ZagException` | Resume the most recent session. |
| `execResume` | `AgentOutput execResume(String sessionId, String prompt) throws ZagException` | Resume a session non-interactively with a follow-up prompt. |
| `execContinue` | `AgentOutput execContinue(String prompt) throws ZagException` | Resume the most recent session non-interactively. |
| `streamResume` | `Iterable<Event> streamResume(String sessionId, String prompt) throws ZagException` | Resume a session in streaming mode. |
| `streamContinue` | `Iterable<Event> streamContinue(String prompt) throws ZagException` | Resume the most recent session in streaming mode. |

---

## StreamingSession

Returned by `execStreaming()`. Implements `AutoCloseable`. Provides bidirectional communication with the agent subprocess over piped stdin/stdout.

```java
public class StreamingSession implements AutoCloseable {
    void send(String message) throws IOException
    void sendUserMessage(String content) throws IOException
    void closeInput() throws IOException
    Iterable<Event> events()
    boolean isRunning()
    void terminate()
    void await() throws ZagException, InterruptedException
    void close()  // from AutoCloseable -- calls destroyForcibly()
}
```

### Methods

| Method | Description |
|--------|-------------|
| `send(String message)` | Write a raw NDJSON line to the agent's stdin and flush. |
| `sendUserMessage(String content)` | Convenience method. Serializes `{"type":"user_message","content":"..."}` and calls `send()`. |
| `closeInput()` | Close stdin to signal no more input. |
| `events()` | Returns an `Iterable<Event>` that lazily parses NDJSON lines from stdout. Unparseable lines are skipped. |
| `isRunning()` | Returns `true` if the underlying process is still alive. |
| `terminate()` | Send a graceful termination signal to the process (`Process.destroy()`). |
| `await()` | Close stdin, then block until the process exits. Throws `ZagException` if the exit code is non-zero. |
| `close()` | Forcibly kill the process (`Process.destroyForcibly()`). Called automatically in try-with-resources. |

---

## Types

All data types are Java records in the `io.zag` package. JSON deserialization uses Jackson with `@JsonIgnoreProperties(ignoreUnknown = true)` on all types, so the SDK is forward-compatible with new fields from the CLI.

### AgentOutput

```java
public record AgentOutput(
    String agent,           // provider name (e.g., "claude")
    String sessionId,       // session identifier
    List<Event> events,     // all events from the session
    String result,          // final text result (nullable)
    boolean isError,        // whether the session ended in error
    Integer exitCode,       // process exit code (nullable)
    String errorMessage,    // error description (nullable)
    Double totalCostUsd,    // total cost in USD (nullable)
    Usage usage             // aggregate token usage (nullable)
)
```

JSON field mapping: `session_id`, `is_error`, `exit_code`, `error_message`, `total_cost_usd`. Accessor methods follow Java record conventions: `agent()`, `sessionId()`, `events()`, `result()`, `isError()`, `exitCode()`, `errorMessage()`, `totalCostUsd()`, `usage()`.

### Event (sealed interface)

```java
public sealed interface Event {
    String type();
}
```

Discriminated by the `"type"` JSON field. Seven concrete implementations:

#### Event.Init

```java
record Init(
    String model,
    List<String> tools,
    String workingDirectory,
    Map<String, JsonNode> metadata
) implements Event
```

Type string: `"init"`. Emitted once at session start.

#### Event.UserMessage

```java
record UserMessage(
    List<ContentBlock> content
) implements Event
```

Type string: `"user_message"`. Emitted when `--replay-user-messages` is enabled.

#### Event.AssistantMessage

```java
record AssistantMessage(
    List<ContentBlock> content,
    Usage usage
) implements Event
```

Type string: `"assistant_message"`. Contains the assistant's response and per-message token usage.

#### Event.ToolExecution

```java
record ToolExecution(
    String toolName,
    String toolId,
    JsonNode input,
    ToolResult result
) implements Event
```

Type string: `"tool_execution"`. The `input` field is a Jackson `JsonNode` (arbitrary JSON).

#### Event.Result

```java
record Result(
    boolean success,
    String message,
    Long durationMs,
    Integer numTurns
) implements Event
```

Type string: `"result"`. Final session outcome. `durationMs` and `numTurns` are nullable.

#### Event.Error

```java
record Error(
    String message,
    JsonNode details
) implements Event
```

Type string: `"error"`. The `details` field is a Jackson `JsonNode` (nullable).

#### Event.PermissionRequest

```java
record PermissionRequest(
    String toolName,
    String description,
    boolean granted
) implements Event
```

Type string: `"permission_request"`. Indicates whether a tool permission was requested and granted.

### ContentBlock (sealed interface)

```java
public sealed interface ContentBlock {
    String type();
}
```

Discriminated by the `"type"` JSON field. Two concrete implementations:

#### ContentBlock.Text

```java
record Text(String text) implements ContentBlock
```

Type string: `"text"`. Plain text content.

#### ContentBlock.ToolUse

```java
record ToolUse(
    String id,
    String name,
    JsonNode input
) implements ContentBlock
```

Type string: `"tool_use"`. Tool invocation content. The `input` field is a Jackson `JsonNode`.

### Usage

```java
public record Usage(
    long inputTokens,
    long outputTokens,
    Long cacheReadTokens,        // nullable, Claude-specific
    Long cacheCreationTokens,    // nullable, Claude-specific
    Integer webSearchRequests,   // nullable, Gemini-specific
    Integer webFetchRequests     // nullable, Gemini-specific
)
```

Token usage statistics. Primitive `long` for guaranteed fields; boxed `Long`/`Integer` for provider-specific nullable fields.

### ToolResult

```java
public record ToolResult(
    boolean success,
    String output,     // nullable
    String error,      // nullable
    JsonNode data      // nullable, arbitrary JSON
)
```

### ZagException

```java
public class ZagException extends Exception {
    public Integer exitCode()    // process exit code, or null if the process could not start
    public String stderr()       // captured stderr output
}
```

Thrown by all terminal methods and discovery methods on failure. Extends `Exception` (checked), not `RuntimeException`.

---

## Discovery API

Static methods on `ZagDiscover` for querying available providers, models, and capabilities. Each method has a one-arg overload accepting a custom binary path and a no-arg (or fewer-arg) overload that uses the default binary.

```java
import io.zag.ZagDiscover;
import io.zag.ProviderCapability;
import io.zag.ResolvedModel;
```

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `listProviders` | `static List<String> listProviders() throws ZagException` | List available provider names using the default binary. |
| `listProviders` | `static List<String> listProviders(String bin) throws ZagException` | List available provider names using a custom binary. |
| `getCapability` | `static ProviderCapability getCapability(String provider) throws ZagException` | Get capabilities for a specific provider. |
| `getCapability` | `static ProviderCapability getCapability(String provider, String bin) throws ZagException` | Get capabilities using a custom binary. |
| `getAllCapabilities` | `static List<ProviderCapability> getAllCapabilities() throws ZagException` | Get capabilities for all providers. |
| `getAllCapabilities` | `static List<ProviderCapability> getAllCapabilities(String bin) throws ZagException` | Get all capabilities using a custom binary. |
| `resolveModel` | `static ResolvedModel resolveModel(String provider, String model) throws ZagException` | Resolve a model alias (e.g., `"small"` to `"haiku"`). |
| `resolveModel` | `static ResolvedModel resolveModel(String provider, String model, String bin) throws ZagException` | Resolve a model alias using a custom binary. |

### Discovery Types

#### ProviderCapability

```java
public record ProviderCapability(
    String provider,
    String defaultModel,
    List<String> availableModels,
    SizeMappings sizeMappings,
    Features features
)
```

#### ProviderCapability.SizeMappings

```java
public record SizeMappings(
    String small,
    String medium,
    String large
)
```

#### ProviderCapability.Features

```java
public record Features(
    FeatureSupport interactive,
    FeatureSupport nonInteractive,
    FeatureSupport resume,
    FeatureSupport resumeWithPrompt,
    SessionLogSupport sessionLogs,
    FeatureSupport jsonOutput,
    FeatureSupport streamJson,
    FeatureSupport jsonSchema,
    FeatureSupport inputFormat,
    StreamingInputSupport streamingInput,
    FeatureSupport worktree,
    FeatureSupport sandbox,
    FeatureSupport systemPrompt,
    FeatureSupport autoApprove,
    FeatureSupport review,
    FeatureSupport addDirs,
    FeatureSupport maxTurns
)
```

17 feature fields, each indicating whether the provider supports the feature and whether the support is native to the upstream CLI.

#### ProviderCapability.FeatureSupport

```java
public record FeatureSupport(
    boolean supported,
    boolean native_      // accessor: native_()
)
```

Note: the accessor is `native_()` (with trailing underscore) because `native` is a reserved word in Java.

#### ProviderCapability.SessionLogSupport

```java
public record SessionLogSupport(
    boolean supported,
    boolean native_,
    String completeness  // nullable, e.g., "full"
)
```

#### ProviderCapability.StreamingInputSupport

```java
public record StreamingInputSupport(
    boolean supported,
    boolean native_,
    String semantics  // nullable; "queue" | "interrupt" | "between-turns-only"
)
```

`semantics` describes what happens when a user message is sent mid-turn via
`StreamingSession.send_user_message`:

- `"queue"` — buffered and delivered at the next turn boundary; the current
  turn runs to completion.
- `"interrupt"` — cancels the current turn and starts a new one.
- `"between-turns-only"` — mid-turn sends are an error or no-op.

`null` when `supported` is `false`.

#### ResolvedModel

```java
public record ResolvedModel(
    String input,        // the original input (e.g., "large")
    String resolved,     // the resolved model name (e.g., "opus")
    boolean isAlias,     // true if input was a size alias
    String provider      // provider name
)
```

---

## Examples

### exec -- Non-interactive Execution

```java
import io.zag.ZagBuilder;
import io.zag.AgentOutput;
import io.zag.Event;

AgentOutput output = new ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .systemPrompt("You are a helpful assistant.")
    .root("/home/user/project")
    .autoApprove()
    .maxTurns(5)
    .timeout("2m")
    .exec("refactor the utils module");

System.out.println("Agent: " + output.agent());
System.out.println("Session: " + output.sessionId());
System.out.println("Result: " + output.result());
System.out.println("Error: " + output.isError());

if (output.usage() != null) {
    System.out.println("Tokens in: " + output.usage().inputTokens());
    System.out.println("Tokens out: " + output.usage().outputTokens());
}

for (Event event : output.events()) {
    if (event instanceof Event.ToolExecution tool) {
        System.out.println("Tool: " + tool.toolName() + " -> " + tool.result().success());
    }
}
```

### stream -- Streaming Events

```java
import io.zag.ZagBuilder;
import io.zag.Event;
import io.zag.ContentBlock;

for (Event event : new ZagBuilder()
        .provider("claude")
        .model("sonnet")
        .autoApprove()
        .stream("analyze this codebase")) {

    switch (event) {
        case Event.Init init ->
            System.out.println("Model: " + init.model() + ", Tools: " + init.tools());
        case Event.AssistantMessage msg -> {
            for (ContentBlock block : msg.content()) {
                if (block instanceof ContentBlock.Text text) {
                    System.out.print(text.text());
                }
            }
        }
        case Event.ToolExecution tool ->
            System.out.println("[tool] " + tool.toolName() + ": " + tool.result().output());
        case Event.Result result ->
            System.out.println("Done in " + result.durationMs() + "ms, " + result.numTurns() + " turns");
        case Event.Error error ->
            System.err.println("Error: " + error.message());
        default -> {}
    }
}
```

### execStreaming -- Bidirectional Streaming (Claude Only)

```java
import io.zag.ZagBuilder;
import io.zag.StreamingSession;
import io.zag.Event;

try (StreamingSession session = new ZagBuilder()
        .provider("claude")
        .autoApprove()
        .execStreaming("you are a coding assistant")) {

    // Send a follow-up message
    session.sendUserMessage("list all files in the current directory");

    // Read events as they arrive
    for (Event event : session.events()) {
        System.out.println(event.type());
        if (event instanceof Event.Result) {
            break;
        }
    }

    // Signal that no more input will be sent
    session.closeInput();

    // Wait for the process to finish
    session.await();
}
```

### jsonSchema -- Structured Output

```java
import io.zag.ZagBuilder;
import io.zag.AgentOutput;
import java.util.Map;
import java.util.List;

Object schema = Map.of(
    "type", "object",
    "properties", Map.of(
        "name", Map.of("type", "string"),
        "languages", Map.of(
            "type", "array",
            "items", Map.of("type", "string")
        )
    ),
    "required", List.of("name", "languages")
);

AgentOutput output = new ZagBuilder()
    .provider("claude")
    .autoApprove()
    .jsonSchema(schema)
    .exec("describe this project");

// output.result() contains JSON conforming to the schema
System.out.println(output.result());
```

### Error Handling

```java
import io.zag.ZagBuilder;
import io.zag.ZagException;
import io.zag.AgentOutput;

try {
    AgentOutput output = new ZagBuilder()
        .provider("claude")
        .timeout("30s")
        .exec("do something");

    if (output.isError()) {
        System.err.println("Agent error: " + output.errorMessage());
    } else {
        System.out.println(output.result());
    }
} catch (ZagException e) {
    // Process-level failure (non-zero exit, binary not found, version mismatch, etc.)
    System.err.println("ZagException: " + e.getMessage());
    if (e.exitCode() != null) {
        System.err.println("Exit code: " + e.exitCode());
    }
    if (!e.stderr().isEmpty()) {
        System.err.println("Stderr: " + e.stderr());
    }
}
```

### Discovery

```java
import io.zag.ZagDiscover;
import io.zag.ProviderCapability;
import io.zag.ResolvedModel;
import io.zag.ZagException;

try {
    // List available providers
    List<String> providers = ZagDiscover.listProviders();
    System.out.println("Providers: " + providers);

    // Get capabilities for a specific provider
    ProviderCapability cap = ZagDiscover.getCapability("claude");
    System.out.println("Default model: " + cap.defaultModel());
    System.out.println("Available: " + cap.availableModels());
    System.out.println("Size small: " + cap.sizeMappings().small());
    System.out.println("Supports streaming input: " + cap.features().streamingInput().supported());
    System.out.println("Session logs completeness: " + cap.features().sessionLogs().completeness());

    // Resolve a size alias
    ResolvedModel resolved = ZagDiscover.resolveModel("claude", "large");
    System.out.println(resolved.input() + " -> " + resolved.resolved());
    System.out.println("Is alias: " + resolved.isAlias());

    // Use a custom binary path
    ProviderCapability custom = ZagDiscover.getCapability("claude", "/usr/local/bin/zag");
} catch (ZagException e) {
    System.err.println("Discovery failed: " + e.getMessage());
}
```

---

## Internals

### Subprocess Architecture

The SDK does not communicate with AI providers directly. It spawns the `zag` CLI as a child process via `ProcessBuilder` and parses the structured output.

- `exec()` runs `zag [global-flags] exec -o json <prompt>` and reads all of stdout into an `AgentOutput`.
- `stream()` runs `zag [global-flags] exec -o stream-json <prompt>` and returns a lazy `Iterable<Event>` that parses NDJSON lines from stdout one at a time.
- `execStreaming()` runs `zag [global-flags] exec -i stream-json -o stream-json --replay-user-messages <prompt>` with piped stdin and stdout.
- `run()`, `resume()`, and `continueLast()` use `ProcessBuilder.inheritIO()` so the user interacts with the agent directly in the terminal.

### JSON Parsing

Uses a shared Jackson `ObjectMapper` configured with `DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES = false`. This ensures forward compatibility -- new fields from newer CLI versions are silently ignored.

Event polymorphism is handled via `@JsonTypeInfo` / `@JsonSubTypes` annotations on the `Event` sealed interface, discriminated by the `"type"` JSON field.

The `jsonSchema` field on the builder accepts any `Object` and serializes it via `ObjectMapper.writeValueAsString()` before passing it as a CLI argument.

### Worktree and Sandbox Storage

The `worktree` and `sandbox` fields are stored as `Object` internally. A no-arg call stores `Boolean.TRUE`; a `String` call stores the name. During argument building, the type is dispatched via `instanceof`.

### Version Checking

Before any terminal method executes, the SDK checks version requirements by running `zag --version` and parsing the semver output. The detected version is cached per binary path in a `ConcurrentHashMap` for the lifetime of the JVM process.

If a method requiring a newer CLI version is configured (e.g., `env()` requires >= 0.6.0), the SDK throws `ZagException` with a descriptive message before spawning the process.

### Error Propagation

- `ZagException` is a checked exception (extends `Exception`).
- In `stream()`, if the process exits with a non-zero code after the iterator is exhausted, the error is wrapped in a `RuntimeException` (since `Iterator.hasNext()` cannot throw checked exceptions).
- Unparseable NDJSON lines in both `stream()` and `StreamingSession.events()` are silently skipped.

---

## Version Requirements

| Method | Minimum CLI Version |
|--------|-------------------|
| `env()` | 0.6.0 |
| `mcpConfig()` | 0.6.0 |
| All others | 0.2.3 |

---

## Provider Notes

### Claude Only

The following features are exclusive to the Claude provider:

- `inputFormat()` -- set input format to `"text"` or `"stream-json"`
- `replayUserMessages()` -- re-emit user messages on stdout
- `includePartialMessages()` -- include partial message chunks in streaming
- `mcpConfig()` -- MCP server configuration
- `execStreaming()` -- bidirectional streaming via piped stdin/stdout

### Ollama Only

- `size()` -- set the Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`)
