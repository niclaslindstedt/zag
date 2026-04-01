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
