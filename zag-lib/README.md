# zag

A unified Rust library for driving AI coding agents — Claude, Codex, Gemini, Copilot, and Ollama.

[![Crates.io](https://img.shields.io/crates/v/zag.svg)](https://crates.io/crates/zag)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

## Overview

`zag` provides a programmatic Rust API for spawning and interacting with AI coding agents. It wraps multiple provider CLIs behind a common `Agent` trait and exposes a fluent `AgentBuilder` for ergonomic use. Write your agent logic once and swap providers without changing code.

## Quick start

```toml
[dependencies]
zag = "0.1"
tokio = { version = "1", features = ["full"] }
```

```rust
use zag::builder::AgentBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let output = AgentBuilder::new()
        .provider("claude")
        .model("sonnet")
        .auto_approve(true)
        .exec("write a hello world program")
        .await?;

    println!("{}", output.result.unwrap_or_default());
    Ok(())
}
```

## Features

- **Multi-provider support**: Claude, Codex, Gemini, Copilot, and Ollama
- **Model size aliases**: Use `small`, `medium`, `large` across any provider
- **JSON output with schema validation**: Request structured output and validate against JSON Schema
- **Streaming sessions**: Bidirectional NDJSON communication (Claude)
- **Custom progress handlers**: Plug in your own progress reporting
- **Session management**: Track, resume, and search session logs
- **Worktree and sandbox isolation**: Run agents in isolated environments

## Examples

### JSON schema validation

```rust
use zag::builder::AgentBuilder;

let schema = serde_json::json!({
    "type": "object",
    "required": ["colors"],
    "properties": {
        "colors": { "type": "array", "items": { "type": "string" } }
    }
});

let output = AgentBuilder::new()
    .provider("gemini")
    .json_schema(schema)
    .exec("list 3 colors")
    .await?;
```

### Streaming (Claude only)

```rust
use zag::builder::AgentBuilder;

let mut session = AgentBuilder::new()
    .provider("claude")
    .exec_streaming("initial prompt")
    .await?;

session.send_user_message("follow-up question").await?;
while let Some(event) = session.next_event().await? {
    println!("{:?}", event);
}
session.wait().await?;
```

### Custom progress handler

```rust
use zag::progress::ProgressHandler;

struct MyProgress;
impl ProgressHandler for MyProgress {
    fn on_success(&self, msg: &str) { println!("OK: {}", msg); }
    fn on_error(&self, msg: &str) { eprintln!("ERR: {}", msg); }
}

AgentBuilder::new()
    .on_progress(Box::new(MyProgress))
    .exec("hello")
    .await?;
```

### Worktree isolation

```rust
use zag::builder::AgentBuilder;

// Run the agent in an isolated git worktree
let output = AgentBuilder::new()
    .provider("claude")
    .worktree(Some("my-feature"))
    .auto_approve(true)
    .exec("implement the feature")
    .await?;
```

### Limiting agentic turns

```rust
use zag::builder::AgentBuilder;

let output = AgentBuilder::new()
    .provider("claude")
    .max_turns(5)
    .exec("fix the failing test")
    .await?;
```

## Builder methods

| Method | Description |
|--------|-------------|
| `.provider(name)` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `.model(name)` | Set model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `.system_prompt(text)` | Set a system prompt |
| `.root(path)` | Set the root directory for the agent |
| `.auto_approve(bool)` | Skip permission prompts |
| `.add_dir(path)` | Add an additional directory (chainable) |
| `.json()` | Request JSON output |
| `.json_schema(schema)` | Validate output against a JSON schema (implies `.json()`) |
| `.json_stream()` | Enable streaming NDJSON output |
| `.worktree(name)` | Run in an isolated git worktree |
| `.sandbox(name)` | Run inside a Docker sandbox |
| `.session_id(uuid)` | Pre-set a session ID |
| `.output_format(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.input_format(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replay_user_messages(bool)` | Re-emit user messages on stdout (Claude only) |
| `.include_partial_messages(bool)` | Include partial message chunks (Claude only) |
| `.max_turns(n)` | Maximum number of agentic turns |
| `.size(size)` | Ollama parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.show_usage(bool)` | Show token usage statistics |
| `.verbose(bool)` | Enable verbose output |
| `.quiet(bool)` | Suppress non-essential output |
| `.on_progress(handler)` | Set a custom progress handler |

### Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `Result<AgentOutput>` | Run non-interactively, return structured output |
| `.exec_streaming(prompt)` | `Result<StreamingSession>` | Bidirectional streaming (Claude only) |
| `.run(prompt)` | `Result<()>` | Start an interactive session (inherits stdio) |
| `.resume(session_id)` | `Result<()>` | Resume a previous session by ID |
| `.continue_last()` | `Result<()>` | Resume the most recent session |

## CLI

The companion binary crate [`zag-cli`](https://crates.io/crates/zag-cli) provides a full CLI:

```bash
cargo install zag-cli
```

See the [repository](https://github.com/niclaslindstedt/zag) for full documentation.

For complete projects using this API, see the [examples directory](../examples/) — especially [cv-review](../examples/cv-review/) which demonstrates `AgentBuilder`, JSON schema validation, and parallel agent invocations.

## See also

- [Root README](../README.md) — Full CLI documentation
- [zag-orch](../zag-orch/) — Orchestration primitives (spawn, wait, collect, pipe)
- [Language bindings](../bindings/) — TypeScript, Python, and C# SDKs
- [Examples](../examples/) — Complete projects demonstrating zag usage

## License

[MIT](../LICENSE)
