# Language Bindings

Bindings for using [zag](https://github.com/niclaslindstedt/zag) from Rust, TypeScript, Python, C#, Swift, and Java.

## SDKs

| Language | Directory | Docs |
|----------|-----------|------|
| Rust | `bindings/rust/` | [README](rust/README.md) |
| TypeScript | `bindings/typescript/` | [README](typescript/README.md) |
| Python | `bindings/python/` | [README](python/README.md) |
| C# | `bindings/csharp/` | [README](csharp/README.md) |
| Swift | `bindings/swift/` | [README](swift/README.md) |
| Java | `bindings/java/` | [README](java/README.md) |

> **Note:** The Rust binding is the published `zag` crate that re-exports `zag-agent` and `zag-orch`. The TypeScript, Python, C#, Swift, and Java bindings are not published to any package registry.

## Quick start

All six SDKs expose the same builder pattern. Here's the same task in each language:

**Rust**

```rust
use zag::builder::AgentBuilder;

let output = AgentBuilder::new()
    .provider("claude")
    .model("sonnet")
    .auto_approve(true)
    .exec("write a hello world program")
    .await?;

println!("{}", output.result);
```

**TypeScript**

```typescript
import { ZagBuilder } from "zag-agent";

const output = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .autoApprove()
  .exec("write a hello world program");

console.log(output.result);
```

**Python**

```python
from zag import ZagBuilder

output = await ZagBuilder() \
    .provider("claude") \
    .model("sonnet") \
    .auto_approve() \
    .exec("write a hello world program")

print(output.result)
```

**C#**

```csharp
using Zag;

var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove()
    .ExecAsync("write a hello world program");

Console.WriteLine(output.Result);
```

**Swift**

```swift
import Zag

let output = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

print(output.result ?? "")
```

**Java**

```java
import io.zag.ZagBuilder;

var output = new ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program");

System.out.println(output.result());
```

Each SDK also supports streaming (NDJSON events), bidirectional streaming sessions (Claude only), and interactive sessions. See the individual README files for full API documentation.

## How they work

The **Rust** binding directly depends on the `zag-agent` and `zag-orch` workspace crates, giving native access to all types and async APIs.

The **TypeScript**, **Python**, **C#**, **Swift**, and **Java** SDKs spawn the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`), parse the JSON/NDJSON output into typed models, and expose a fluent builder API. Zero external runtime dependencies — only stdlib in each language.

## Prerequisites

The `zag` CLI binary must be installed and on your `PATH`. You can also set the `ZAG_BIN` environment variable to point to the binary.

```bash
cargo install --path zag-cli
```

## Testing

```bash
# TypeScript
cd bindings/typescript && npm run build && npm test

# Python
cd bindings/python && pip install pytest pytest-asyncio && pytest

# C#
cd bindings/csharp && dotnet test

# Swift
cd bindings/swift && swift test

# Java
cd bindings/java && mvn test
```

## See also

- [Root README](../README.md) — Full CLI documentation
- [zag (Rust binding)](rust/README.md) — Published Rust crate
- [zag-agent](../zag-agent/README.md) — Core agent library
- [zag-orch](../zag-orch/README.md) — Orchestration library

## License

[MIT](../LICENSE)
