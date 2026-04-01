# Claude Agent JSON Output Format

> **Note:** This is internal developer documentation describing Claude's raw JSON output format. For user-facing docs, see the [root README](../../README.md).

This document explains the JSON output format returned by the Claude CLI when running in verbose/JSON mode. Understanding this structure is essential for parsing and extracting information from Claude's responses.

## Overview

When running Claude with the `--output json` flag (which automatically adds `--verbose`), the output is a JSON array of **events**. Each event represents a step in the conversation flow between the user, Claude, and the tool execution system.

## Event Flow

A typical Claude session produces events in this sequence:

```
1. System Init Event      → Session setup and configuration
2. Assistant Event(s)     → Claude's thinking and tool calls
3. User Event(s)          → Tool execution results
4. Assistant Event(s)     → Claude's response to tool results
5. Result Event           → Final summary with usage and cost
```

## Event Types

### 1. System Event (Init)

Sent at the beginning of a session to establish context.

```json
{
  "type": "system",
  "subtype": "init",
  "session_id": "uuid",
  "cwd": "/path/to/working/directory",
  "model": "claude-sonnet-4-5-20250929",
  "tools": ["Task", "Bash", "Read", "Write", ...],
  "mcp_servers": [],
  "permissionMode": "default",
  "slash_commands": [...],
  "agents": [...],
  "skills": [],
  "plugins": [...],
  "uuid": "uuid"
}
```

**Key Fields:**
- `session_id`: Unique identifier for this session
- `cwd`: Current working directory
- `model`: The Claude model being used
- `tools`: Available tools Claude can invoke
- `permissionMode`: Permission handling mode

### 2. Assistant Event

Represents Claude's output, either as text or tool invocations.

```json
{
  "type": "assistant",
  "message": {
    "model": "claude-sonnet-4-5-20250929",
    "id": "msg_...",
    "type": "message",
    "role": "assistant",
    "content": [
      {
        "type": "text",
        "text": "I'll read the file for you."
      }
    ],
    "usage": {
      "input_tokens": 2,
      "cache_creation_input_tokens": 3318,
      "cache_read_input_tokens": 15848,
      "output_tokens": 78,
      "cache_creation": {
        "ephemeral_5m_input_tokens": 3318,
        "ephemeral_1h_input_tokens": 0
      },
      "service_tier": "standard"
    }
  },
  "parent_tool_use_id": null,
  "session_id": "uuid",
  "uuid": "uuid"
}
```

**Key Fields:**
- `message.content`: Array of content blocks (text, tool_use, etc.)
- `message.usage`: Token usage for this turn
- `session_id`: Links to the session

### 3. User Event (Tool Results)

Contains the results of tool executions.

```json
{
  "type": "user",
  "message": {
    "role": "user",
    "content": [
      {
        "tool_use_id": "toolu_...",
        "type": "tool_result",
        "content": "...",
        "is_error": false
      }
    ]
  },
  "session_id": "uuid",
  "uuid": "uuid",
  "tool_use_result": {
    // Tool-specific result structure
  }
}
```

**Key Fields:**
- `message.content[].tool_use_id`: Links to the tool_use that generated this result
- `message.content[].is_error`: Whether the tool execution failed
- `tool_use_result`: Structured result data (tool-specific format)

### 4. Result Event

Final summary of the session with aggregated statistics.

```json
{
  "type": "result",
  "subtype": "success",
  "is_error": false,
  "duration_ms": 5486,
  "duration_api_ms": 5431,
  "num_turns": 2,
  "result": "The final text response",
  "session_id": "uuid",
  "total_cost_usd": 0.025074,
  "usage": {
    "input_tokens": 7,
    "cache_creation_input_tokens": 3492,
    "cache_read_input_tokens": 35012,
    "output_tokens": 97,
    "server_tool_use": {
      "web_search_requests": 0,
      "web_fetch_requests": 0
    }
  },
  "modelUsage": {
    "claude-sonnet-4-5-20250929": {
      "inputTokens": 7,
      "outputTokens": 97,
      "cacheReadInputTokens": 35012,
      "cacheCreationInputTokens": 3492,
      "costUSD": 0.025074
    }
  },
  "permission_denials": [],
  "uuid": "uuid"
}
```

**Key Fields:**
- `result`: The final text output from the session
- `total_cost_usd`: Total cost of the session
- `usage`: Aggregated token usage across all turns
- `modelUsage`: Per-model breakdown of usage and cost
- `permission_denials`: Array of tools that were denied permission

## Content Block Types

Content blocks appear in the `message.content` array and can be one of several types:

### Text Block

Simple text response from Claude.

```json
{
  "type": "text",
  "text": "Here is my response..."
}
```

### Tool Use Block

Claude requesting to use a tool.

```json
{
  "type": "tool_use",
  "id": "toolu_...",
  "name": "Read",
  "input": {
    "file_path": "/path/to/file.txt"
  }
}
```

**Fields:**
- `id`: Unique identifier for this tool invocation
- `name`: Name of the tool (Bash, Read, Write, Grep, Glob, etc.)
- `input`: Tool-specific input parameters

### Tool Result Block

Result from a tool execution.

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_...",
  "content": "Result content or error message",
  "is_error": false
}
```

**Fields:**
- `tool_use_id`: References the tool_use that generated this result
- `content`: String representation of the result or error
- `is_error`: Whether the tool execution failed

## Tool-Specific Result Formats

The `tool_use_result` field in User events contains structured data specific to each tool type:

### Bash Tool

```json
{
  "stdout": "command output",
  "stderr": "",
  "interrupted": false,
  "isImage": false
}
```

### Read Tool

```json
{
  "type": "text",
  "file": {
    "filePath": "/path/to/file.txt",
    "content": "file contents",
    "numLines": 10,
    "startLine": 1,
    "totalLines": 10
  }
}
```

### Grep Tool

```json
{
  "mode": "content",
  "numFiles": 1,
  "filenames": [],
  "content": "file.txt:1:matching line",
  "numLines": 1
}
```

### Glob Tool

```json
{
  "filenames": ["/path/to/file1.json", "/path/to/file2.json"],
  "durationMs": 4,
  "numFiles": 2,
  "truncated": false
}
```

### Write/Edit Tool (Permission Denied)

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_...",
  "content": "Claude requested permissions to write to /path/file, but you haven't granted it yet.",
  "is_error": true
}
```

## Usage and Cost Information

Usage statistics are provided at multiple levels:

### Per-Turn Usage

In each assistant message:

```json
{
  "usage": {
    "input_tokens": 2,
    "cache_creation_input_tokens": 3318,
    "cache_read_input_tokens": 15848,
    "output_tokens": 78,
    "cache_creation": {
      "ephemeral_5m_input_tokens": 3318,
      "ephemeral_1h_input_tokens": 0
    },
    "service_tier": "standard"
  }
}
```

### Aggregated Usage

In the final result event:

```json
{
  "usage": {
    "input_tokens": 7,
    "cache_creation_input_tokens": 3492,
    "cache_read_input_tokens": 35012,
    "output_tokens": 97,
    "server_tool_use": {
      "web_search_requests": 0,
      "web_fetch_requests": 0
    }
  },
  "modelUsage": {
    "claude-sonnet-4-5-20250929": {
      "inputTokens": 7,
      "outputTokens": 97,
      "cacheReadInputTokens": 35012,
      "cacheCreationInputTokens": 3492,
      "webSearchRequests": 0,
      "costUSD": 0.025074,
      "contextWindow": 200000,
      "maxOutputTokens": 64000
    }
  }
}
```

## Permission Denials

When tools require permissions that haven't been granted, they appear in:

1. **Tool result** with `is_error: true`
2. **Result event** `permission_denials` array:

```json
{
  "permission_denials": [
    {
      "tool_name": "Write",
      "tool_use_id": "toolu_...",
      "tool_input": {
        "file_path": "/path/to/file",
        "content": "..."
      }
    }
  ]
}
```

## Event Relationships

Events are connected through identifiers:

```
session_id: All events in a session share the same session_id
    ↓
Assistant Event (tool_use)
    ↓ (via tool_use.id)
User Event (tool_result with tool_use_id)
    ↓
Assistant Event (response to tool result)
    ↓
Result Event (final summary)
```

## Parsing Strategy

To extract information from Claude's JSON output:

1. **Parse the JSON array** into a list of events
2. **Filter by event type** to process specific events
3. **For the final result**: Look for the `result` event with `subtype: "success"`
4. **For tool calls**: Match `tool_use` blocks with corresponding `tool_result` blocks via `id`/`tool_use_id`
5. **For costs**: Use the `modelUsage` in the result event
6. **For errors**: Check `is_error` flags and `permission_denials` array

## Example Session Flow

```json
[
  {
    "type": "system",
    "subtype": "init",
    "session_id": "abc123",
    ...
  },
  {
    "type": "assistant",
    "session_id": "abc123",
    "message": {
      "content": [{"type": "text", "text": "I'll read the file"}]
    }
  },
  {
    "type": "assistant",
    "session_id": "abc123",
    "message": {
      "content": [
        {
          "type": "tool_use",
          "id": "tool1",
          "name": "Read",
          "input": {"file_path": "test.txt"}
        }
      ]
    }
  },
  {
    "type": "user",
    "session_id": "abc123",
    "message": {
      "content": [
        {
          "tool_use_id": "tool1",
          "type": "tool_result",
          "content": "...",
          "is_error": false
        }
      ]
    }
  },
  {
    "type": "assistant",
    "session_id": "abc123",
    "message": {
      "content": [{"type": "text", "text": "The file contains..."}]
    }
  },
  {
    "type": "result",
    "subtype": "success",
    "session_id": "abc123",
    "result": "The file contains...",
    "total_cost_usd": 0.02,
    ...
  }
]
```

## Implementation Notes

When implementing parsers for this format:

1. **Use enums** for event types and content block types
2. **Make fields optional** where they may not always be present (e.g., `parent_tool_use_id`)
3. **Handle tool-specific results** with separate structs or enums
4. **Preserve UUIDs** for debugging and tracing
5. **Consider streaming** for large outputs - events arrive sequentially

## Future Considerations

- Additional tool types may have different result formats
- MCP servers may introduce new event types
- Permission system may evolve
- Additional fields may be added for telemetry

## References

- Main documentation: [Root README](../../README.md)
- Struct implementations: [`zag-lib/src/providers/claude/models.rs`](../../zag-lib/src/providers/claude/models.rs)
- Agent implementation: [`zag-lib/src/providers/claude/mod.rs`](../../zag-lib/src/providers/claude/mod.rs)
