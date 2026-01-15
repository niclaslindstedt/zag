# Agent JSON Output Exploration Methodology

This document describes the methodology for exploring and understanding the JSON output formats of different AI coding agents integrated into this CLI.

## Purpose

When integrating a new agent or understanding an existing agent's output format, it's essential to systematically explore the JSON structures it produces. This allows us to:

1. Build accurate Rust structs for deserialization
2. Handle different event types appropriately
3. Extract relevant information for display or processing
4. Handle error cases and permission denials

## Methodology

### 1. Create a Temporary Test Environment

Create an isolated directory for testing to avoid polluting the main project:

```bash
mkdir -p /tmp/agent-exploration-test
cd /tmp/agent-exploration-test
```

### 2. Run Agent Commands with JSON Output

Execute the agent with the `-p` (print mode) and `--output json` flags to capture structured output:

```bash
# Simple text-only responses
agent <agent-name> -p --output json "Say hello world" > simple.json

# Responses requiring tool usage
agent <agent-name> -p --output json "List files in current directory" > bash_tool.json
agent <agent-name> -p --output json "Read the test.txt file" > read_tool.json
agent <agent-name> -p --output json "Write 'content' to output.txt" > write_tool.json
agent <agent-name> -p --output json "Search for 'pattern' in files" > grep_tool.json
agent <agent-name> -p --output json "Find all json files" > glob_tool.json
```

### 3. Test Different Scenarios

Cover various use cases:

- **Simple prompts**: Text-only responses without tool usage
- **File operations**: Read, Write, Edit operations
- **Search operations**: Grep, Glob patterns
- **Command execution**: Bash commands
- **Web operations**: WebSearch, WebFetch (if applicable)
- **Permission denials**: Operations that require approval
- **Error cases**: Invalid inputs or failing operations
- **Multi-turn conversations**: Complex tasks requiring multiple steps

### 4. Analyze the JSON Structure

Examine the captured JSON files to identify:

#### Event Types

The top-level structure is an array of events. Common event types:

- **System events**: Session initialization, configuration
- **Assistant messages**: Responses from the AI, including text and tool calls
- **User messages**: Tool results and user inputs
- **Result events**: Final summaries with usage statistics and costs

#### Message Structure

Messages contain:
- Metadata: `model`, `id`, `role`, `type`
- Content blocks: Array of text, tool uses, or tool results
- Usage information: Token counts, cache statistics
- Session information: `session_id`, `uuid`

#### Content Block Types

Different types of content blocks:
- **Text blocks**: Simple text responses
- **Tool use blocks**: Tool invocations with name and input parameters
- **Tool result blocks**: Results from tool execution

#### Tool-Specific Results

Each tool returns results in a specific format:
- **Bash**: `{ stdout, stderr, interrupted, isImage }`
- **Read**: `{ type: "text", file: { filePath, content, numLines, startLine, totalLines } }`
- **Write/Edit**: Permission request or success confirmation
- **Grep**: `{ mode, numFiles, filenames, content, numLines }`
- **Glob**: `{ filenames, durationMs, numFiles, truncated }`

#### Usage and Cost Tracking

Results include detailed usage information:
- Token counts (input, output, cache creation, cache read)
- Per-model usage breakdown
- Cost in USD
- Service tier and context management details

#### Permission Handling

Permission denials are tracked:
- In tool results: `{ is_error: true, content: "..." }`
- In final result: `permission_denials` array with tool details

### 5. Document Findings

Create comprehensive documentation:

1. **README in agent folder**: Explain the output format structure, event types, and relationships
2. **Rust structs**: Build strongly-typed structures for deserialization
3. **Examples**: Include sample JSON snippets for each event type
4. **Edge cases**: Document error handling and special cases

### 6. Build Type-Safe Structures

Create Rust structs with:
- Proper serde annotations for JSON deserialization
- Enums for variant types (events, content blocks, tool results)
- Optional fields for nullable values
- Documentation comments explaining each field

## Example: Claude Agent Output Structure

Based on exploration of the Claude agent, the output consists of:

```
Array<Event>
├─ SystemEvent { subtype: "init", session_id, tools, model, ... }
├─ AssistantEvent { message: Message, session_id, uuid, ... }
├─ UserEvent { message: Message (tool results), session_id, uuid, ... }
└─ ResultEvent { subtype: "success", result, usage, cost, permission_denials, ... }
```

Each Message contains:
```
Message {
  model, id, type, role,
  content: Array<ContentBlock>,
  usage: { input_tokens, output_tokens, cache_*, ... }
}
```

Content blocks can be:
```
ContentBlock::Text { text }
ContentBlock::ToolUse { id, name, input }
ContentBlock::ToolResult { tool_use_id, type, content, is_error }
```

## Tips for Exploration

1. **Start simple**: Begin with basic text prompts before complex tool usage
2. **Be systematic**: Test each tool type separately to isolate its output format
3. **Test edge cases**: Permission denials, errors, large outputs
4. **Compare outputs**: Run the same prompt multiple times to identify consistent vs. variable fields
5. **Use jq**: For quick JSON inspection: `cat output.json | jq .`
6. **Document as you go**: Don't try to understand everything at once—document incrementally

## Agent-Specific Considerations

### Claude
- Uses verbose JSON output with `--verbose` flag
- Includes detailed usage and caching statistics
- Supports many tools: Task, Bash, Read, Write, Grep, Glob, etc.
- Has permission system for file operations

### Codex
- May use `--json` flag for JSON output
- Output format might differ from Claude

### Gemini
- Uses `-o json` for output format
- Structure might vary from Claude's format

### Copilot
- May not support JSON output format
- Requires special flags in non-interactive mode

## Conclusion

Systematic exploration of agent JSON outputs enables building robust integrations. By following this methodology, you can quickly understand any agent's output format and create appropriate Rust structures for parsing and processing the results.
