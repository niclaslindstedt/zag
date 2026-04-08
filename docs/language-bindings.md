# Language Bindings

zag provides SDK bindings for seven languages. Each binding exposes a fluent `ZagBuilder` API that mirrors the Rust source of truth.

## Overview

| Language | Package | Min version | Install |
|----------|---------|-------------|---------|
| **Rust** | `zag` (crate) | Rust 1.75+ | `cargo add zag` |
| **TypeScript** | `@nlindstedt/zag-agent` | Node.js 18+ | `npm install @nlindstedt/zag-agent` |
| **Python** | `zag-agent` | Python 3.10+ | `pip install zag-agent` |
| **C#** | `Zag` | .NET 8.0+ | `dotnet add package Zag` |
| **Swift** | SPM package | Swift 5.9+ | Add to `Package.swift` |
| **Java** | `io.zag:zag` | Java 17+ | Maven/Gradle dependency |
| **Kotlin** | `com.github.niclaslindstedt:zag` | JDK 21+ | Gradle dependency |

All non-Rust bindings work by spawning the `zag` CLI as a subprocess. The CLI binary must be on your `PATH` or specified via the `ZAG_BIN` environment variable.

The **Rust** binding is native -- it re-exports the workspace crates directly with zero subprocess overhead.

## Quick start

### Rust

```rust
use zag::AgentBuilder;

let output = AgentBuilder::new()
    .provider("claude")
    .model("sonnet")
    .auto_approve()
    .exec("write a hello world program")
    .await?;

println!("{}", output.result.unwrap_or_default());
```

### TypeScript

```typescript
import { ZagBuilder } from '@nlindstedt/zag-agent';

const output = await new ZagBuilder()
  .provider('claude')
  .model('sonnet')
  .autoApprove()
  .exec('write a hello world program');

console.log(output.result);
```

### Python

```python
from zag import ZagBuilder

output = await ZagBuilder() \
    .provider("claude") \
    .model("sonnet") \
    .auto_approve() \
    .exec("write a hello world program")

print(output.result)
```

### C\#

```csharp
using Zag;

var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove()
    .ExecAsync("write a hello world program");

Console.WriteLine(output.Result);
```

### Swift

```swift
import Zag

let output = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

print(output.result ?? "")
```

### Java

```java
import io.zag.ZagBuilder;

var output = new ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program");

System.out.println(output.getResult());
```

### Kotlin

```kotlin
import zag.ZagBuilder

val output = ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

println(output.result)
```

## Builder methods

All bindings support the same set of builder methods (naming follows each language's conventions):

| Method | Description |
|--------|-------------|
| `provider` | Set the AI provider |
| `model` | Set the model name or size alias |
| `system_prompt` | Set a custom system prompt |
| `root` | Set the working directory |
| `auto_approve` | Skip permission prompts |
| `add_dir` | Add a directory to include |
| `file` | Attach a file to the prompt |
| `env` | Set an environment variable |
| `max_turns` | Limit agentic turns |
| `json` | Request JSON output |
| `json_schema` | Set a JSON schema for validation |
| `json_stream` | Enable NDJSON streaming |
| `worktree` | Enable git worktree isolation |
| `sandbox` | Enable Docker sandbox isolation |
| `session_id` | Set a pre-determined session ID |
| `output_format` | Set the output format |
| `mcp_config` | Set MCP server configuration |
| `name` | Set the session name |
| `description` | Set the session description |
| `tag` | Add a session tag |

### Terminal methods

| Method | Description |
|--------|-------------|
| `exec` | Non-interactive execution with a prompt |
| `run` | Start an interactive session |
| `resume` | Resume a previous session |
| `stream` | Stream NDJSON events |

## Streaming

All bindings support streaming NDJSON events:

```typescript
// TypeScript
for await (const event of new ZagBuilder()
  .provider('claude')
  .stream('analyze this codebase')) {
  console.log(event.type);
}
```

```python
# Python
async for event in ZagBuilder() \
    .provider("claude") \
    .stream("analyze this codebase"):
    print(event.type)
```

## Remote mode (Swift)

The Swift binding uniquely supports remote mode, connecting to a `zag serve` instance via HTTP/WebSocket without needing the CLI binary locally:

```swift
let output = try await ZagBuilder()
    .remote(url: "https://server:2100", token: "abc123")
    .provider("claude")
    .exec("analyze the code")
```

This enables iOS apps and other platforms where the CLI can't run locally. See [Remote Access](remote-access.md) for server setup.

## Binding READMEs

Each binding has its own detailed README with full API documentation:

- [Rust](../bindings/rust/README.md)
- [TypeScript](../bindings/typescript/README.md)
- [Python](../bindings/python/README.md)
- [C#](../bindings/csharp/README.md)
- [Swift](../bindings/swift/README.md)
- [Java](../bindings/java/README.md)
- [Kotlin](../bindings/kotlin/README.md)

## Related

- [Getting Started](getting-started.md) -- CLI quickstart
- [Providers](providers.md) -- Available providers and models
- [Events & Logging](events-and-logging.md) -- Event format for streaming
