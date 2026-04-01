# zag-orch

[![Crates.io](https://img.shields.io/crates/v/zag-orch.svg)](https://crates.io/crates/zag-orch)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

Orchestration library for zag — multi-session coordination for AI coding agents.

## Overview

`zag-orch` provides the Rust implementation behind zag's orchestration CLI commands. It is the programmatic layer for launching, synchronizing, and collecting results from multiple agent sessions.

This crate depends on [`zag`](../zag-lib/) (zag-lib) for shared types and agent execution.

## Modules

### Session lifecycle

| Module | CLI command | Description |
|--------|------------|-------------|
| `spawn` | `zag spawn` | Launch a background agent session, return session ID |
| `cancel` | `zag cancel` | Graceful session cancellation with clean log entry |
| `retry` | `zag retry` | Re-run failed sessions with the same configuration |
| `gc` | `zag gc` | Clean up old session data, logs, and process entries |

### Coordination

| Module | CLI command | Description |
|--------|------------|-------------|
| `wait` | `zag wait` | Block until session(s) complete |
| `collect` | `zag collect` | Gather results from multiple sessions |
| `pipe` | `zag pipe` | Chain session results into a new agent session |
| `status` | `zag status` | Machine-readable session health check |

### Observation

| Module | CLI command | Description |
|--------|------------|-------------|
| `events` | `zag events` | Structured event query API for session logs |
| `listen` | `zag listen` | Session log tailing and event formatting |
| `watch` | `zag watch` | Event-driven reactions on session log events |
| `subscribe` | `zag subscribe` | Multiplexed event stream from all active sessions |
| `summary` | `zag summary` | Log-based session summarization and stats |
| `output_cmd` | `zag output` | Extract final result text from sessions |
| `log_cmd` | `zag log` | Append custom structured events to session logs |
| `search` | `zag search` | CLI argument wiring for session log search |

### Introspection

| Module | CLI command | Description |
|--------|------------|-------------|
| `ps` | `zag ps` | List, inspect, and kill agent processes |
| `whoami` | `zag whoami` | Session identity introspection via env vars |
| `env` | `zag env` | Export session environment variables |
| `lifecycle` | *(internal)* | Filesystem lifecycle markers (`.started`/`.ended`) |

## Usage

Most users interact with these primitives through the `zag` CLI. Library consumers who need orchestration beyond `AgentBuilder` can depend on this crate directly:

```toml
[dependencies]
zag-orch = "0.1"
zag = "0.1"
tokio = { version = "1", features = ["full"] }
```

## See also

- [Root README](../README.md) — Full CLI documentation and orchestration examples
- [Orchestration shell scripts](../examples/orchestration/) — Runnable multi-agent patterns
- [zag-lib README](../zag-lib/README.md) — Core library with `AgentBuilder` API

## License

[MIT](../LICENSE)
