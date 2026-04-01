# Language Bindings

SDK packages for using [zag](https://github.com/niclaslindstedt/zag) from TypeScript, Python, and C#.

## SDKs

| Language | Package | Install | Docs |
|----------|---------|---------|------|
| TypeScript | `zag-agent` | `npm install zag-agent` | [README](typescript/README.md) |
| Python | `zag-agent` | `pip install zag-agent` | [README](python/README.md) |
| C# | `Zag` | `dotnet add package Zag` | [README](csharp/README.md) |

## How they work

All SDKs spawn the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`), parse the JSON/NDJSON output into typed models, and expose a fluent builder API. Zero external runtime dependencies — only stdlib in each language.

## Prerequisites

The `zag` CLI binary must be installed and on your `PATH`. You can also set the `ZAG_BIN` environment variable to point to the binary.

```bash
cargo install zag-cli
```

## See also

- [Root README](../README.md) — Full CLI documentation
- [zag-lib](../zag-lib/README.md) — Rust library API (no CLI subprocess needed)

## License

[MIT](../LICENSE)
