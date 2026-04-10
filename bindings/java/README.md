# Zag Java Binding

Java binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- Java 17+
- Maven 3.8+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Installation

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>io.zag</groupId>
    <artifactId>zag</artifactId>
    <version>0.2.4</version>
</dependency>
```

Or with Gradle:

```groovy
implementation 'io.zag:zag:0.2.4'
```

## Quick start

```java
import io.zag.ZagBuilder;

var output = new ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program");

System.out.println(output.result());
```

## Streaming

```java
import io.zag.ZagBuilder;

for (var event : new ZagBuilder().provider("claude").stream("analyze code")) {
    System.out.println(event.type());
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
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output |
| `.stream(prompt)` | `Iterable<Event>` | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only) |
| `.run(prompt?)` | `void` | Start an interactive session (inherits stdio) |
| `.resume(sessionId)` | `void` | Resume a previous session by ID |
| `.continueLast()` | `void` | Resume the most recent session |
| `.execResume(sessionId, prompt)` | `AgentOutput` | Resume a session non-interactively with a follow-up prompt |
| `.execContinue(prompt)` | `AgentOutput` | Resume the most recent session non-interactively |
| `.streamResume(sessionId, prompt)` | `Iterable<Event>` | Resume a session in streaming mode |
| `.streamContinue(prompt)` | `Iterable<Event>` | Resume the most recent session in streaming mode |

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
a typed `ZagFeatureUnsupportedException` (subclass of `ZagException`) is
thrown with an actionable message:

```
execStreaming() is not supported by provider 'ollama' (feature: streaming_input). Supported providers: claude
```

```java
try {
    new ZagBuilder().provider("ollama").addDir("/extra").exec("...");
} catch (ZagFeatureUnsupportedException e) {
    System.err.println("pick another provider from: " + String.join(", ", e.supportedProviders()));
} catch (ZagException e) {
    // other runtime errors
}
```

Capability data is loaded once per `(bin, provider)` and cached. Checks are
skipped when no provider is set (auto-detect) or when the provider is `"mock"`.

| Method | Required capability |
|--------|---------------------|
| `.execStreaming()` | `streaming_input` |
| `.worktree()` | `worktree` |
| `.sandbox()` | `sandbox` |
| `.systemPrompt()` | `system_prompt` |
| `.addDir()` | `add_dirs` |

## Discovery

Static methods for discovering available providers, models, and capabilities:

```java
import io.zag.ZagDiscover;

List<String> providers = ZagDiscover.listProviders();
ProviderCapability cap = ZagDiscover.getCapability("claude");
List<ProviderCapability> all = ZagDiscover.getAllCapabilities();
ResolvedModel resolved = ZagDiscover.resolveModel("claude", "small");
```

| Method | Description |
|--------|-------------|
| `ZagDiscover.listProviders(bin?)` | List available provider names |
| `ZagDiscover.getCapability(provider, bin?)` | Get capabilities for a provider |
| `ZagDiscover.getAllCapabilities(bin?)` | Get capabilities for all providers |
| `ZagDiscover.resolveModel(provider, model, bin?)` | Resolve a model alias |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed models. Uses Jackson for JSON parsing.

## Testing

```bash
cd bindings/java && mvn test
```

## See also

- [TypeScript SDK](../typescript/README.md)
- [Python SDK](../python/README.md)
- [C# SDK](../csharp/README.md)
- [Swift SDK](../swift/README.md)
- [Rust API (zag-agent)](../../zag-agent/README.md)
- [All bindings](../README.md)

## License

[MIT](../../LICENSE)
