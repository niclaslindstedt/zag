# Integration Guide for Claude JSON Output Parsing

This document explains how to integrate the Claude JSON output parsing into the agent CLI for improved observability and logging.

## Current State

The following components have been created:

1. **Unified Output Format** (`src/output.rs`)
   - `AgentOutput`: Universal structure for all agent outputs
   - `Event`: Common event types across agents
   - `LogEntry`: Simplified logging interface
   - `LogLevel`: Debug/Info/Warn/Error categorization

2. **Claude-Specific Models** (`src/claude/models.rs`)
   - `ClaudeOutput`: Deserializes Claude's JSON format
   - `ClaudeEvent`: All event types from Claude CLI
   - Conversion: `From<ClaudeOutput> for AgentOutput`

3. **Documentation**
   - `AGENT_EXPLORATION.md`: Methodology for exploring agent outputs
   - `src/claude/README.md`: Claude output format reference

## Integration Steps

To integrate JSON parsing and improved logging:

### 1. Capture JSON Output

Modify the `execute()` method in `src/claude/mod.rs` to capture stdout when in print mode with JSON output:

```rust
use tokio::process::Command;
use std::process::Stdio;

async fn execute_with_capture(&self, interactive: bool, prompt: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("claude");

    // Configure command as before...

    if !interactive && self.output_format == Some("json".to_string()) {
        // Capture output instead of inheriting
        cmd.stdout(Stdio::piped());

        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!("Claude command failed");
        }

        Ok(String::from_utf8(output.stdout)?)
    } else {
        // Existing behavior for interactive mode
        cmd.stdout(Stdio::inherit());
        cmd.status().await?;
        Ok(String::new())
    }
}
```

### 2. Parse and Convert Output

When JSON output is captured, parse it into the unified format:

```rust
use crate::claude::models::ClaudeOutput;
use crate::output::AgentOutput;

if let Some(json_output) = captured_output {
    // Parse Claude's JSON
    let claude_output: ClaudeOutput = serde_json::from_str(&json_output)?;

    // Convert to unified format
    let agent_output: AgentOutput = claude_output.into();

    // Use the structured output
    process_agent_output(&agent_output)?;
}
```

### 3. Implement Logging Based on Events

Extract and display events at different log levels:

```rust
use crate::output::{AgentOutput, LogLevel};

fn process_agent_output(output: &AgentOutput) -> Result<()> {
    // Get log level from CLI or config
    let min_level = if debug { LogLevel::Debug } else { LogLevel::Info };

    // Extract log entries
    let log_entries = output.to_log_entries(min_level);

    // Display logs
    for entry in log_entries {
        match entry.level {
            LogLevel::Debug => log::debug!("{}", entry.message),
            LogLevel::Info => log::info!("{}", entry.message),
            LogLevel::Warn => log::warn!("{}", entry.message),
            LogLevel::Error => log::error!("{}", entry.message),
        }
    }

    // Display final result
    if let Some(result) = output.final_result() {
        println!("{}", result);
    }

    // Display cost if available
    if let Some(cost) = output.total_cost_usd {
        log::info!("Total cost: ${:.4}", cost);
    }

    Ok(())
}
```

### 4. Add Observability Features

Use the structured output for enhanced observability:

```rust
// Show tool executions
for tool_event in output.tool_executions() {
    if let Event::ToolExecution { tool_name, result, .. } = tool_event {
        if result.success {
            log::debug!("✓ Tool '{}' executed successfully", tool_name);
        } else {
            log::warn!("✗ Tool '{}' failed: {}", tool_name,
                result.error.as_deref().unwrap_or("unknown"));
        }
    }
}

// Show errors
for error_event in output.errors() {
    if let Event::Error { message, .. } = error_event {
        log::error!("Agent error: {}", message);
    }
}

// Show usage statistics
if let Some(usage) = &output.usage {
    log::debug!("Token usage - Input: {}, Output: {}",
        usage.input_tokens, usage.output_tokens);
    if let Some(cache) = usage.cache_read_tokens {
        log::debug!("Cache hit: {} tokens", cache);
    }
}
```

### 5. Add CLI Flags for Output Control

Add new CLI options to control output processing:

```rust
#[derive(Parser)]
struct Cli {
    /// Minimum log level (debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Show token usage statistics
    #[arg(long)]
    show_usage: bool,

    /// Show tool execution details
    #[arg(long)]
    show_tools: bool,

    // ... existing fields
}
```

## Example Usage

Once integrated, users can get enhanced output:

```bash
# Normal execution (existing behavior)
agent claude "write a hello world program"

# With debug logging and usage stats
agent claude --log-level debug --show-usage "write a hello world program"

# JSON output with structured processing
agent claude -p --output json --show-tools "list files and read README"
```

Example output:

```
✓ Claude initialized with model sonnet
[INFO] Starting non-interactive session
[DEBUG] Tool 'Bash' executed successfully
[DEBUG] Tool 'Read' executed successfully
[INFO] Session completed
[INFO] Token usage - Input: 1250, Output: 450
[INFO] Cache hit: 15000 tokens
[INFO] Total cost: $0.0234
[INFO] Session terminated

The README contains...
```

## Benefits

1. **Better observability**: See exactly what tools were called and their results
2. **Debug support**: Detailed logging when things go wrong
3. **Cost tracking**: See token usage and costs per session
4. **Consistent interface**: Same output format across all agents
5. **Structured data**: Can be piped to other tools for analysis

## Next Steps

1. Implement output capture in `claude/mod.rs`
2. Add similar parsing for other agents (Codex, Gemini, Copilot)
3. Create a shared output processor in `main.rs`
4. Add CLI flags for output control
5. Add tests for parsing and conversion
6. Consider streaming support for long-running sessions

## Testing

Test the parsing with real Claude output:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_real_output() {
        let json = std::fs::read_to_string("test_data/claude_output.json").unwrap();
        let claude_output: ClaudeOutput = serde_json::from_str(&json).unwrap();
        let agent_output: AgentOutput = claude_output.into();

        assert_eq!(agent_output.agent, "claude");
        assert!(!agent_output.is_error);
        assert!(agent_output.total_cost_usd.is_some());
    }
}
```

## References

- Unified output structures: `src/output.rs`
- Claude-specific models: `src/claude/models.rs`
- Output format documentation: `src/claude/README.md`
- Exploration methodology: `AGENT_EXPLORATION.md`
