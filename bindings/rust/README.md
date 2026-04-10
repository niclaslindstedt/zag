# zag (Rust)

The published Rust crate for [zag](https://github.com/niclaslindstedt/zag) — a unified interface for AI coding agents.

This crate re-exports `zag-agent` (core agent library) and `zag-orch` (orchestration primitives) under a single `zag` package name.

## Usage

```rust
use zag::builder::AgentBuilder;
use zag::config::Config;
use zag::orch::spawn;
```

Agent types are available directly under `zag::` (from `zag-agent`), while orchestration primitives live under `zag::orch::` (from `zag-orch`).

See [`docs/providers.md`](../../docs/providers.md) for the per-provider flag support matrix (e.g. `input_format`, `replay_user_messages`, `include_partial_messages`, and `mcp_config` are honored only by the Claude provider) and for the default `assistant_message` emission granularity of `exec_streaming`.

## How it differs from the other bindings

The TypeScript, Python, and C# bindings spawn the `zag` CLI as a subprocess. This Rust crate directly depends on the workspace libraries, giving you native access to all types and async APIs with no subprocess overhead.

## See also

- [zag-agent](../../zag-agent/) — Core agent library
- [zag-orch](../../zag-orch/) — Orchestration library
- [TypeScript bindings](../typescript/) — CLI subprocess wrapper
- [Python bindings](../python/) — CLI subprocess wrapper
- [C# bindings](../csharp/) — CLI subprocess wrapper

## License

[MIT](../../LICENSE)
