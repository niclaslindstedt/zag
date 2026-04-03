# Language Bindings

Bindings for using [zag](https://github.com/niclaslindstedt/zag) from TypeScript, Python, and C#.

## SDKs

| Language | Directory | Docs |
|----------|-----------|------|
| TypeScript | `bindings/typescript/` | [README](typescript/README.md) |
| Python | `bindings/python/` | [README](python/README.md) |
| C# | `bindings/csharp/` | [README](csharp/README.md) |

> **Note:** These bindings are not published to any package registry. Use them directly from the source tree.

## Quick start

All three SDKs expose the same builder pattern. Here's the same task in each language:

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

Each SDK also supports streaming (NDJSON events), bidirectional streaming sessions (Claude only), and interactive sessions. See the individual README files for full API documentation.

## How they work

All SDKs spawn the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`), parse the JSON/NDJSON output into typed models, and expose a fluent builder API. Zero external runtime dependencies — only stdlib in each language.

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
```

## See also

- [Root README](../README.md) — Full CLI documentation
- [zag-lib](../zag-lib/README.md) — Rust library API (no CLI subprocess needed)

## License

[MIT](../LICENSE)
