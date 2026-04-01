# Examples

Complete projects demonstrating [zag](https://github.com/niclaslindstedt/zag) usage patterns.

## Prerequisites

- [zag](../) installed and on your `PATH`
- At least one provider configured (e.g., `ANTHROPIC_API_KEY` for Claude)
- Rust 1.85+ (for the cv-review example)
- Node.js 18+ (for the react-claude-interface example)

## Which example should I start with?

- **New to zag?** Start with [orchestration/01-sequential-pipeline.sh](orchestration/01-sequential-pipeline.sh) — it's a simple three-stage pipeline you can run immediately.
- **Want to use zag as a Rust library?** See [cv-review](cv-review/) — it demonstrates `AgentBuilder`, JSON schema validation, and parallel agent invocations.
- **Building a web UI on top of zag?** See [react-claude-interface](react-claude-interface/) — a full React app with streaming NDJSON events over SSE.
- **Exploring multi-agent patterns?** The [orchestration](orchestration/) directory has 7 scripts covering fan-out, generator-critic, coordinator dispatch, DAG workflows, and agent-to-agent messaging.

## Examples

| Example | Language | Description | Key features |
|---------|----------|-------------|-------------|
| [cv-review](cv-review/) | Rust | Two-pass CV review pipeline: recruiter screen + hiring committee | `AgentBuilder` API, JSON schema validation, parallel agents, custom progress handler |
| [orchestration](orchestration/) | Shell | 7 multi-agent pattern scripts (sequential, fan-out, generator-critic, coordinator, hierarchical, composite, arena) | `spawn`, `wait`, `pipe`, `collect`, `input`, `broadcast`, `watch`, `cancel`, `summary` |
| [react-claude-interface](react-claude-interface/) | TypeScript/React | Claude Code-like web chat interface with multi-turn conversations | `zag exec`, `zag input`, SSE streaming, NDJSON events, collapsible tool/thinking blocks |

## Customizing the provider

All examples default to Claude. To use a different provider:

```bash
# Orchestration scripts
ZAG_PROVIDER=gemini ./examples/orchestration/01-sequential-pipeline.sh

# CV review (edit the provider in the Rust source, or set it via zag config)
zag config provider gemini
cargo run -p cv-review -- --cv cvs/01_alex_chen.txt --job jobs/senior_backend.txt
```

## See also

- [Root README](../README.md) — Full CLI documentation
- [zag-lib](../zag-lib/README.md) — Rust library API
- [Language bindings](../bindings/) — TypeScript, Python, and C# SDKs
