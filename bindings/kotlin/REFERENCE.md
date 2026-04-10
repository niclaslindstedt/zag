# Kotlin Binding Reference

Complete API reference for the zag Kotlin binding. Covers every builder method, type definition, and execution pattern needed to integrate a system with this SDK.

## Quick Start

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.github.niclaslindstedt:zag:0.1.0")
}
```

```kotlin
import zag.ZagBuilder

val output = ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

println(output.result)
```

**Package**: `com.github.niclaslindstedt:zag`
**Import**: `import zag.ZagBuilder`
**Requires**: JDK 21+, Gradle 8+, `zag` CLI on `PATH` (or `ZAG_BIN` env var)
**JSON**: `kotlinx.serialization`
**Async**: `kotlinx.coroutines` -- terminal methods are `suspend` functions, streaming uses `Flow<Event>`

## Builder API

Constructor: `ZagBuilder()`

All configuration methods use `= apply { }` pattern for chaining. Uses camelCase.

### Configuration Methods

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `bin` | `fun bin(path: String) = apply { ... }` | _(binding-only)_ | Override zag binary path (default: `ZAG_BIN` env or `"zag"`) |
| `provider` | `fun provider(name: String) = apply { ... }` | `-p, --provider` | Provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `model` | `fun model(name: String) = apply { ... }` | `--model` | Model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `systemPrompt` | `fun systemPrompt(text: String) = apply { ... }` | `--system-prompt` | System prompt for agent behavior |
| `root` | `fun root(path: String) = apply { ... }` | `--root` | Working directory for the agent |
| `autoApprove` | `fun autoApprove(v: Boolean = true) = apply { ... }` | `--auto-approve` | Skip permission prompts |
| `addDir` | `fun addDir(path: String) = apply { ... }` | `--add-dir` | Add additional directory (repeatable) |
| `file` | `fun file(path: String) = apply { ... }` | `--file` | Attach a file to the prompt (repeatable) |
| `env` | `fun env(key: String, value: String) = apply { ... }` | `--env KEY=VALUE` | Add environment variable _(CLI >= 0.6.0)_ |
| `json` | `fun json(v: Boolean = true) = apply { ... }` | `--json` | Request JSON output |
| `jsonSchema` | `fun jsonSchema(schema: String) = apply { ... }` | `--json-schema` | JSON schema for validation (implies `json()`) |
| `worktree` | `fun worktree(name: String? = null) = apply { ... }` | `-w, --worktree [NAME]` | Git worktree isolation (auto-named if no arg) |
| `sandbox` | `fun sandbox(name: String? = null) = apply { ... }` | `--sandbox [NAME]` | Docker sandbox isolation (auto-named if no arg) |
| `verbose` | `fun verbose(v: Boolean = true) = apply { ... }` | `--verbose` | Enable verbose output |
| `quiet` | `fun quiet(v: Boolean = true) = apply { ... }` | `--quiet` | Suppress non-essential output |
| `debug` | `fun debug(v: Boolean = true) = apply { ... }` | `--debug` | Enable debug logging _(binding-only)_ |
| `sessionId` | `fun sessionId(uuid: String) = apply { ... }` | `--session UUID` | Pre-set session ID |
| `outputFormat` | `fun outputFormat(fmt: String) = apply { ... }` | `-o, --output` | Output format: `"text"`, `"json"`, `"json-pretty"`, `"stream-json"` |
| `inputFormat` | `fun inputFormat(fmt: String) = apply { ... }` | `-i, --input-format` | Input format: `"text"`, `"stream-json"` _(Claude only)_ |
| `replayUserMessages` | `fun replayUserMessages(v: Boolean = true) = apply { ... }` | `--replay-user-messages` | Re-emit user messages on stdout _(Claude only)_ |
| `includePartialMessages` | `fun includePartialMessages(v: Boolean = true) = apply { ... }` | `--include-partial-messages` | Include partial message chunks _(Claude only)_ |
| `maxTurns` | `fun maxTurns(n: Int) = apply { ... }` | `--max-turns` | Maximum number of agentic turns |
| `timeout` | `fun timeout(duration: String) = apply { ... }` | `--timeout` | Timeout duration (e.g., `"30s"`, `"5m"`, `"1h"`). Kills agent if exceeded. |
| `mcpConfig` | `fun mcpConfig(config: String) = apply { ... }` | `--mcp-config` | MCP server config: JSON string or file path _(Claude only, CLI >= 0.6.0)_ |
| `showUsage` | `fun showUsage(v: Boolean = true) = apply { ... }` | `--show-usage` | Show token usage statistics (JSON output mode) |
| `size` | `fun size(size: String) = apply { ... }` | `--size` | Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |

### Terminal Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `exec` | `suspend fun exec(prompt: String): AgentOutput` | Non-interactive execution, returns structured output |
| `stream` | `fun stream(prompt: String): Flow<Event>` | Stream NDJSON events via cold Kotlin Flow |
| `execStreaming` | `suspend fun execStreaming(prompt: String): StreamingSession` | Bidirectional streaming _(Claude only)_. Emits one `assistant_message` per complete turn by default; call `includePartialMessages(true)` on the builder for token-level chunks. |
| `run` | `suspend fun run(prompt: String? = null)` | Interactive session (inherits stdio) |
| `resume` | `suspend fun resume(sessionId: String)` | Resume previous session by ID |
| `continueLast` | `suspend fun continueLast()` | Resume most recent session |
| `execResume` | `suspend fun execResume(sessionId: String, prompt: String): AgentOutput` | Resume a session non-interactively with a follow-up prompt |
| `execContinue` | `suspend fun execContinue(prompt: String): AgentOutput` | Resume the most recent session non-interactively |
| `streamResume` | `fun streamResume(sessionId: String, prompt: String): Flow<Event>` | Resume a session in streaming mode |
| `streamContinue` | `fun streamContinue(prompt: String): Flow<Event>` | Resume the most recent session in streaming mode |

## StreamingSession

Returned by `execStreaming()`. Provides bidirectional communication with the agent process.

```kotlin
class StreamingSession {
    /** Send a raw NDJSON line to the agent's stdin. */
    suspend fun send(message: String)

    /** Send a user message to the agent (serializes to NDJSON). */
    suspend fun sendUserMessage(content: String)

    /** Close stdin to signal no more input. */
    fun closeInput()

    /** Flow of parsed Event objects from stdout. */
    fun events(): Flow<Event>

    /** Whether the child process is still running. */
    val isRunning: Boolean

    /** Kill the child process. */
    fun terminate()

    /** Suspend until the process exits. Throws ZagError on non-zero exit. */
    suspend fun wait()
}
```

## Types

All types are `@Serializable` data classes in the `zag` package using `kotlinx.serialization`.

### AgentOutput

```kotlin
@Serializable
data class AgentOutput(
    val agent: String,                      // Agent/provider name
    val sessionId: String,                  // Session UUID
    val events: List<Event>,                // All session events
    val result: String?,                    // Final result text
    val isError: Boolean,                   // Whether session ended in error
    val exitCode: Int?,                     // Process exit code
    val errorMessage: String?,              // Error message if isError
    val totalCostUsd: Double?,              // Cost in USD (if available)
    val usage: Usage?                       // Aggregate token usage
)
```

### Usage

```kotlin
@Serializable
data class Usage(
    val inputTokens: Long,
    val outputTokens: Long,
    val cacheReadTokens: Long? = null,       // Claude-specific
    val cacheCreationTokens: Long? = null,   // Claude-specific
    val webSearchRequests: Int? = null,       // Gemini-specific
    val webFetchRequests: Int? = null         // Gemini-specific
)
```

### Events

Events are a sealed class hierarchy discriminated on the `type` field.

```kotlin
@Serializable
sealed class Event {
    abstract val type: String
}

@Serializable
data class InitEvent(
    val model: String,                      // Model used
    val tools: List<String>,                // Available tool names
    val workingDirectory: String?,          // Agent working directory
    val metadata: Map<String, JsonElement>  // Provider-specific metadata
) : Event() {
    override val type = "init"
}

@Serializable
data class UserMessageEvent(
    val content: List<ContentBlock>          // User message content
) : Event() {
    override val type = "user_message"
}

@Serializable
data class AssistantMessageEvent(
    val content: List<ContentBlock>,         // Assistant response content
    val usage: Usage?                        // Token usage for this message
) : Event() {
    override val type = "assistant_message"
}

@Serializable
data class ToolExecutionEvent(
    val toolName: String,                    // Tool that was invoked
    val toolId: String,                      // Unique invocation ID
    val input: JsonElement?,                 // Tool input parameters
    val result: ToolResult                   // Tool execution result
) : Event() {
    override val type = "tool_execution"
}

@Serializable
data class ResultEvent(
    val success: Boolean,                    // Whether session succeeded
    val message: String?,                    // Final result message
    val durationMs: Long?,                   // Total duration in milliseconds
    val numTurns: Int?                       // Number of agentic turns
) : Event() {
    override val type = "result"
}

@Serializable
data class ErrorEvent(
    val message: String,                     // Error message
    val details: JsonElement?                // Additional error details
) : Event() {
    override val type = "error"
}

@Serializable
data class PermissionRequestEvent(
    val toolName: String,                    // Tool requesting permission
    val description: String,                 // What the tool wants to do
    val granted: Boolean                     // Whether permission was granted
) : Event() {
    override val type = "permission_request"
}
```

### Content Blocks

```kotlin
@Serializable
sealed class ContentBlock {
    abstract val type: String
}

@Serializable
data class TextBlock(
    val text: String
) : ContentBlock() {
    override val type = "text"
}

@Serializable
data class ToolUseBlock(
    val id: String,                          // Tool use ID
    val name: String,                        // Tool name
    val input: JsonElement?                  // Tool input
) : ContentBlock() {
    override val type = "tool_use"
}
```

### ToolResult

```kotlin
@Serializable
data class ToolResult(
    val success: Boolean,
    val output: String?,
    val error: String?,
    val data: JsonElement?
)
```

### ZagError

```kotlin
class ZagError(
    message: String,
    val exitCode: Int?,
    val stderr: String
) : RuntimeException(message)
```

### ZagFeatureUnsupportedException

```kotlin
class ZagFeatureUnsupportedException(
    message: String,
    val provider: String,
    val feature: String,
    val method: String,
    val supportedProviders: List<String>,
) : ZagException(message, null, "")
```

Thrown by the capability preflight when a builder option is set whose underlying feature is not supported by the configured provider. See [Capability checking](#capability-checking) below.

### Discovery Types

```kotlin
@Serializable
data class ProviderCapability(
    val provider: String,
    val defaultModel: String,
    val availableModels: List<String>,
    val sizeMappings: SizeMappings,
    val features: Features
)

@Serializable
data class ResolvedModel(
    val input: String,
    val resolved: String,
    val isAlias: Boolean,
    val provider: String
)

@Serializable
data class SizeMappings(
    val small: String,
    val medium: String,
    val large: String
)

@Serializable
data class FeatureSupport(
    val supported: Boolean,
    val native: Boolean
)

@Serializable
data class SessionLogSupport(
    val supported: Boolean,
    val native: Boolean,
    val completeness: String?
)

@Serializable
data class StreamingInputSupport(
    val supported: Boolean,
    val native: Boolean,
    // "queue" | "interrupt" | "between-turns-only" | null
    val semantics: String?
)

@Serializable
data class Features(
    val interactive: FeatureSupport,
    val nonInteractive: FeatureSupport,
    val resume: FeatureSupport,
    val resumeWithPrompt: FeatureSupport,
    val sessionLogs: SessionLogSupport,
    val jsonOutput: FeatureSupport,
    val streamJson: FeatureSupport,
    val jsonSchema: FeatureSupport,
    val inputFormat: FeatureSupport,
    val streamingInput: StreamingInputSupport,
    val worktree: FeatureSupport,
    val sandbox: FeatureSupport,
    val systemPrompt: FeatureSupport,
    val autoApprove: FeatureSupport,
    val review: FeatureSupport,
    val addDirs: FeatureSupport,
    val maxTurns: FeatureSupport
)
```

## Discovery API

Suspend functions on the `ZagDiscover` object for querying available providers and models.

```kotlin
import zag.ZagDiscover

// Function signatures
object ZagDiscover {
    suspend fun listProviders(bin: String? = null): List<String>
    suspend fun getCapability(provider: String, bin: String? = null): ProviderCapability
    suspend fun getAllCapabilities(bin: String? = null): List<ProviderCapability>
    suspend fun resolveModel(provider: String, model: String, bin: String? = null): ResolvedModel
}
```

## Examples

### Non-interactive execution

```kotlin
val output = ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .root("/path/to/project")
    .autoApprove()
    .maxTurns(10)
    .exec("refactor the auth module")

if (output.isError) {
    println(output.errorMessage)
} else {
    println(output.result)
    println("Cost: $${output.totalCostUsd}")
}
```

### Streaming events (Flow)

```kotlin
ZagBuilder()
    .provider("claude")
    .stream("analyze this codebase")
    .collect { event ->
        when (event) {
            is AssistantMessageEvent -> {
                for (block in event.content) {
                    if (block is TextBlock) println(block.text)
                }
            }
            is ToolExecutionEvent ->
                println("Tool: ${event.toolName} -> ${event.result.output}")
            is ResultEvent ->
                println("Done in ${event.durationMs}ms")
            else -> {}
        }
    }
```

### Bidirectional streaming (Claude only)

```kotlin
val session = ZagBuilder()
    .provider("claude")
    .execStreaming("start a conversation")

// Send additional messages
session.sendUserMessage("now do something else")

// Read events
session.events().collect { event ->
    println(event.type)
}

// Wait for completion
session.wait()
```

### JSON schema output

```kotlin
val output = ZagBuilder()
    .provider("claude")
    .jsonSchema("""
        {
            "type": "object",
            "properties": {
                "summary": { "type": "string" },
                "issues": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["summary", "issues"]
        }
    """.trimIndent())
    .exec("analyze code quality")

println(output.result) // Structured JSON
```

### Error handling

```kotlin
import zag.ZagBuilder
import zag.ZagError

try {
    val output = ZagBuilder()
        .provider("claude")
        .exec("do something")
} catch (e: ZagError) {
    println("Exit code: ${e.exitCode}")
    println("Stderr: ${e.stderr}")
}
```

### Discovery

```kotlin
import zag.ZagDiscover

val providers = ZagDiscover.listProviders()
// ["claude", "codex", "gemini", "copilot", "ollama"]

val cap = ZagDiscover.getCapability("claude")
println(cap.defaultModel)              // "sonnet"
println(cap.availableModels)           // ["opus", "sonnet", "haiku", ...]
println(cap.features.worktree)         // FeatureSupport(supported=true, native=true)

val resolved = ZagDiscover.resolveModel("claude", "small")
// ResolvedModel(input=small, resolved=haiku, isAlias=true, provider=claude)
```

### Running in a coroutine scope

```kotlin
import kotlinx.coroutines.runBlocking
import zag.ZagBuilder

fun main() = runBlocking {
    val output = ZagBuilder()
        .provider("claude")
        .exec("hello world")

    println(output.result)
}
```

## Internals

### How it works

The SDK spawns the `zag` CLI as a subprocess using `ProcessBuilder` and parses JSON/NDJSON output using `kotlinx.serialization`. Async operations use `kotlinx.coroutines`.

### CLI argument construction

Arguments are split into two groups:

**Global args** (before the subcommand): `--provider`, `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir`, `--file`, `--env`, `-w`/`--worktree`, `--sandbox`, `--verbose`, `--quiet`, `--debug`, `--session`, `--max-turns`, `--mcp-config`, `--show-usage`, `--size`

**Exec args** (after `exec`): `--json`, `--json-schema`, `-o`/`--output`, `-i`/`--input-format`, `--replay-user-messages`, `--include-partial-messages`, `--timeout`

### Default behaviors

- `exec()` automatically adds `-o json` when no explicit `outputFormat` is set, so the output can be parsed as structured `AgentOutput`.
- `stream()` adds `-o stream-json` for NDJSON event output (unless an explicit `outputFormat` is set). Returns a cold `Flow<Event>` (lazy, collects on demand).
- `execStreaming()` forces `-i stream-json`, `-o stream-json`, and `--replay-user-messages` for bidirectional communication.
- `run()` inherits stdin/stdout/stderr for interactive terminal use.
- `resume()` dispatches to `run --resume <id>`.
- `continueLast()` dispatches to `run --continue`.

### Worktree and sandbox internals

Stored as `Any?` (holds `true` or `String`). Dispatched via `when` expression:
- `worktree()` (no name) stores `true` -> emits `-w`
- `worktree("name")` stores `"name"` -> emits `-w name`

### Version checking

The SDK checks the installed `zag` CLI version (via `zag --version`) once per process and caches the result. Methods that require newer CLI versions throw a clear error:

| Method | Minimum CLI version |
|--------|-------------------|
| `env()` | 0.6.0 |
| `mcpConfig()` | 0.6.0 |
| All others | 0.2.3 |

### Capability checking

After the version check, every terminal method runs a capability preflight against the provider declared by `provider()` (skipped when no provider is set). The preflight loads the capability matrix from `zag discover --json` (cached per binary path for the lifetime of the JVM process) and verifies that every active feature-gated builder option is supported by the configured provider's `Features` block. On the first unsupported feature it throws `ZagFeatureUnsupportedException` with a message of the form:

```
Provider 'ollama' does not support streaming_input (required by execStreaming()). Supported providers: claude
```

Gated methods and their `Features` keys:

| Method | Feature key |
|--------|-------------|
| `worktree()` | `worktree` |
| `sandbox()` | `sandbox` |
| `systemPrompt()` | `system_prompt` |
| `addDir()` | `add_dirs` |
| `maxTurns()` | `max_turns` |
| `execStreaming()` | `streaming_input` |

If `zag discover` itself fails the preflight silently returns and the subsequent CLI invocation surfaces the real error. `mcpConfig()` is intentionally not gated because no `Features` field tracks it.

## Provider-Specific Notes

- **Claude only**: `inputFormat()`, `replayUserMessages()`, `includePartialMessages()`, `mcpConfig()`, `execStreaming()`
- **Ollama only**: `size()`
- Size aliases (`"small"`, `"medium"`, `"large"`) are resolved by the CLI to provider-specific model names.
- Providers: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`. Use `"auto"` for automatic provider selection.
