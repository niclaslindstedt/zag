# Python Binding Reference

Complete API reference for the zag Python binding. Covers every builder method, type definition, and execution pattern needed to integrate a system with this SDK.

## Quick Start

```python
from zag import ZagBuilder

output = await ZagBuilder() \
    .provider("claude") \
    .model("sonnet") \
    .auto_approve() \
    .exec("write a hello world program")

print(output.result)
```

**Package**: `zag-agent`
**Install**: `pip install zag-agent`
**Requires**: Python 3.10+, `zag` CLI on `PATH` (or `ZAG_BIN` env var)
**Dependencies**: None (Python standard library only)

## Builder API

Constructor: `ZagBuilder()`

All configuration methods return `ZagBuilder` for chaining. Uses `snake_case`.

### Configuration Methods

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `bin` | `def bin(self, path: str) -> ZagBuilder` | _(binding-only)_ | Override zag binary path (default: `ZAG_BIN` env or `"zag"`) |
| `provider` | `def provider(self, p: str) -> ZagBuilder` | `-p, --provider` | Provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `model` | `def model(self, m: str) -> ZagBuilder` | `--model` | Model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `system_prompt` | `def system_prompt(self, p: str) -> ZagBuilder` | `--system-prompt` | System prompt for agent behavior |
| `root` | `def root(self, r: str) -> ZagBuilder` | `--root` | Working directory for the agent |
| `auto_approve` | `def auto_approve(self, a: bool = True) -> ZagBuilder` | `--auto-approve` | Skip permission prompts |
| `add_dir` | `def add_dir(self, d: str) -> ZagBuilder` | `--add-dir` | Add additional directory (repeatable) |
| `file` | `def file(self, path: str) -> ZagBuilder` | `--file` | Attach a file to the prompt (repeatable) |
| `env` | `def env(self, key: str, value: str) -> ZagBuilder` | `--env KEY=VALUE` | Add environment variable _(CLI >= 0.6.0)_ |
| `json_mode` | `def json_mode(self) -> ZagBuilder` | `--json` | Request JSON output |
| `json_schema` | `def json_schema(self, s: dict) -> ZagBuilder` | `--json-schema` | JSON schema for validation (implies `json_mode()`) |
| `worktree` | `def worktree(self, name: str \| None = None) -> ZagBuilder` | `-w, --worktree [NAME]` | Git worktree isolation (auto-named if no arg) |
| `sandbox` | `def sandbox(self, name: str \| None = None) -> ZagBuilder` | `--sandbox [NAME]` | Docker sandbox isolation (auto-named if no arg) |
| `verbose` | `def verbose(self, v: bool = True) -> ZagBuilder` | `--verbose` | Enable verbose output |
| `quiet` | `def quiet(self, q: bool = True) -> ZagBuilder` | `--quiet` | Suppress non-essential output |
| `debug` | `def debug(self, d: bool = True) -> ZagBuilder` | `--debug` | Enable debug logging _(binding-only)_ |
| `session_id` | `def session_id(self, id: str) -> ZagBuilder` | `--session UUID` | Pre-set session ID |
| `output_format` | `def output_format(self, f: str) -> ZagBuilder` | `-o, --output` | Output format: `"text"`, `"json"`, `"json-pretty"`, `"stream-json"` |
| `input_format` | `def input_format(self, f: str) -> ZagBuilder` | `-i, --input-format` | Input format: `"text"`, `"stream-json"` _(Claude only)_ |
| `replay_user_messages` | `def replay_user_messages(self, r: bool = True) -> ZagBuilder` | `--replay-user-messages` | Re-emit user messages on stdout _(Claude only)_ |
| `include_partial_messages` | `def include_partial_messages(self, i: bool = True) -> ZagBuilder` | `--include-partial-messages` | Include partial message chunks _(Claude only)_ |
| `max_turns` | `def max_turns(self, n: int) -> ZagBuilder` | `--max-turns` | Maximum number of agentic turns |
| `timeout` | `def timeout(self, t: str) -> ZagBuilder` | `--timeout` | Timeout duration (e.g., `"30s"`, `"5m"`, `"1h"`). Kills agent if exceeded. |
| `mcp_config` | `def mcp_config(self, c: str) -> ZagBuilder` | `--mcp-config` | MCP server config: JSON string or file path _(Claude only, CLI >= 0.6.0)_ |
| `show_usage` | `def show_usage(self, s: bool = True) -> ZagBuilder` | `--show-usage` | Show token usage statistics (JSON output mode) |
| `size` | `def size(self, s: str) -> ZagBuilder` | `--size` | Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |

### Terminal Methods

All terminal methods are `async`.

| Method | Signature | Description |
|--------|-----------|-------------|
| `exec` | `async def exec(self, prompt: str) -> AgentOutput` | Non-interactive execution, returns structured output |
| `stream` | `async def stream(self, prompt: str) -> AsyncGenerator[Event, None]` | Stream NDJSON events as they arrive |
| `exec_streaming` | `async def exec_streaming(self, prompt: str) -> StreamingSession` | Bidirectional streaming _(Claude only)_. Emits one `assistant_message` per complete turn by default; pair with `include_partial_messages(True)` for token-level chunks. |
| `run` | `async def run(self, prompt: str \| None = None) -> None` | Interactive session (inherits stdio) |
| `resume` | `async def resume(self, session_id: str) -> None` | Resume previous session by ID |
| `continue_last` | `async def continue_last(self) -> None` | Resume most recent session |
| `exec_resume` | `async def exec_resume(self, session_id: str, prompt: str) -> AgentOutput` | Resume a session non-interactively with a follow-up prompt |
| `exec_continue` | `async def exec_continue(self, prompt: str) -> AgentOutput` | Resume the most recent session non-interactively |
| `stream_resume` | `async def stream_resume(self, session_id: str, prompt: str) -> AsyncGenerator[Event]` | Resume a session in streaming mode |
| `stream_continue` | `async def stream_continue(self, prompt: str) -> AsyncGenerator[Event]` | Resume the most recent session in streaming mode |

## StreamingSession

Returned by `exec_streaming()`. Provides bidirectional communication with the agent process.

```python
class StreamingSession:
    async def send(self, message: str) -> None
        """Send a raw NDJSON line to the agent's stdin."""

    async def send_user_message(self, content: str) -> None
        """Send a user message to the agent (serializes to NDJSON)."""

    def close_input(self) -> None
        """Close stdin to signal no more input."""

    def terminate(self) -> None
        """Send SIGTERM to the subprocess."""

    @property
    def is_running(self) -> bool
        """Whether the subprocess is still running."""

    async def events(self) -> AsyncGenerator[Event, None]
        """Async iterator over parsed Event objects from stdout."""

    async def wait(self) -> None
        """Wait for the process to exit. Raises ZagError on non-zero exit."""
```

## Types

All types are dataclasses from the `zag` package. Import with `from zag import AgentOutput, Event, Usage, ZagError` etc.

### AgentOutput

```python
@dataclass
class AgentOutput:
    agent: str                          # Agent/provider name
    session_id: str                     # Session UUID
    events: list[Event]                 # All session events
    result: str | None                  # Final result text
    is_error: bool                      # Whether session ended in error
    exit_code: int | None               # Process exit code
    error_message: str | None           # Error message if is_error
    total_cost_usd: float | None        # Cost in USD (if available)
    usage: Usage | None                 # Aggregate token usage
```

### Usage

```python
@dataclass
class Usage:
    input_tokens: int
    output_tokens: int
    cache_read_tokens: int | None       # Claude-specific
    cache_creation_tokens: int | None   # Claude-specific
    web_search_requests: int | None     # Gemini-specific
    web_fetch_requests: int | None      # Gemini-specific
```

### Events

Events are a union type discriminated on the `type` field.

```python
Event = (
    InitEvent | UserMessageEvent | AssistantMessageEvent
    | ToolExecutionEvent | TurnCompleteEvent | ResultEvent
    | ErrorEvent | PermissionRequestEvent
)
```

```python
@dataclass
class InitEvent:
    type: str                           # "init"
    model: str                          # Model used
    tools: list[str]                    # Available tool names
    working_directory: str | None       # Agent working directory
    metadata: dict[str, Any]            # Provider-specific metadata

@dataclass
class UserMessageEvent:
    type: str                           # "user_message"
    content: list[ContentBlock]         # User message content

@dataclass
class AssistantMessageEvent:
    type: str                           # "assistant_message"
    content: list[ContentBlock]         # Assistant response content
    usage: Usage | None                 # Token usage for this message

@dataclass
class ToolExecutionEvent:
    type: str                           # "tool_execution"
    tool_name: str                      # Tool that was invoked
    tool_id: str                        # Unique invocation ID
    input: Any                          # Tool input parameters
    result: ToolResult                  # Tool execution result

@dataclass
class TurnCompleteEvent:
    type: str                           # "turn_complete"
    stop_reason: str | None             # "end_turn"|"tool_use"|"max_tokens"|...
    turn_index: int                     # Zero-based monotonic turn index
    usage: Usage | None                 # Usage for this turn only

@dataclass
class ResultEvent:
    type: str                           # "result"
    success: bool                       # Whether session succeeded
    message: str | None                 # Final result message
    duration_ms: int | None             # Total duration in milliseconds
    num_turns: int | None               # Number of agentic turns

@dataclass
class ErrorEvent:
    type: str                           # "error"
    message: str                        # Error message
    details: Any                        # Additional error details

@dataclass
class PermissionRequestEvent:
    type: str                           # "permission_request"
    tool_name: str                      # Tool requesting permission
    description: str                    # What the tool wants to do
    granted: bool                       # Whether permission was granted
```

### Content Blocks

```python
ContentBlock = TextBlock | ToolUseBlock

@dataclass
class TextBlock:
    type: str                           # "text"
    text: str

@dataclass
class ToolUseBlock:
    type: str                           # "tool_use"
    id: str                             # Tool use ID
    name: str                           # Tool name
    input: Any                          # Tool input
```

### ToolResult

```python
@dataclass
class ToolResult:
    success: bool
    output: str | None
    error: str | None
    data: Any
```

### ZagError

```python
class ZagError(Exception):
    exit_code: int | None
    stderr: str
```

### ZagFeatureUnsupportedError

Subclass of `ZagError` raised when a builder option requires a provider feature that the configured provider does not support. The capability preflight raises this before spawning the CLI, so callers receive a typed, catchable error instead of a cryptic non-zero exit code.

```python
class ZagFeatureUnsupportedError(ZagError):
    provider: str               # the provider that does not support the feature
    feature: str                # feature key (e.g. "streaming_input")
    method: str                 # builder method that requires it (e.g. "exec_streaming()")
    supported_providers: list[str]  # providers that do support the feature
```

### Discovery Types

```python
@dataclass
class FeatureSupport:
    supported: bool
    native: bool

@dataclass
class SessionLogSupport:
    supported: bool
    native: bool
    completeness: str | None

@dataclass
class StreamingInputSupport:
    supported: bool
    native: bool
    # "queue" | "interrupt" | "between-turns-only" | None
    semantics: str | None

@dataclass
class SizeMappings:
    small: str
    medium: str
    large: str

@dataclass
class Features:
    interactive: FeatureSupport
    non_interactive: FeatureSupport
    resume: FeatureSupport
    resume_with_prompt: FeatureSupport
    session_logs: SessionLogSupport
    json_output: FeatureSupport
    stream_json: FeatureSupport
    json_schema: FeatureSupport
    input_format: FeatureSupport
    streaming_input: StreamingInputSupport
    worktree: FeatureSupport
    sandbox: FeatureSupport
    system_prompt: FeatureSupport
    auto_approve: FeatureSupport
    review: FeatureSupport
    add_dirs: FeatureSupport
    max_turns: FeatureSupport

@dataclass
class ProviderCapability:
    provider: str
    default_model: str
    available_models: list[str]
    size_mappings: SizeMappings
    features: Features

@dataclass
class ResolvedModel:
    input: str
    resolved: str
    is_alias: bool
    provider: str
```

## Discovery API

Standalone async functions for querying available providers and models.

```python
from zag import list_providers, get_capability, get_all_capabilities, resolve_model

async def list_providers(bin: str | None = None) -> list[str]
async def get_capability(provider: str, bin: str | None = None) -> ProviderCapability
async def get_all_capabilities(bin: str | None = None) -> list[ProviderCapability]
async def resolve_model(provider: str, model: str, bin: str | None = None) -> ResolvedModel
```

## Examples

### Non-interactive execution

```python
output = await ZagBuilder() \
    .provider("claude") \
    .model("sonnet") \
    .root("/path/to/project") \
    .auto_approve() \
    .max_turns(10) \
    .exec("refactor the auth module")

if output.is_error:
    print(output.error_message)
else:
    print(output.result)
    print(f"Cost: ${output.total_cost_usd}")
```

### Streaming events

```python
async for event in await ZagBuilder() \
    .provider("claude") \
    .stream("analyze this codebase"):
    if event.type == "assistant_message":
        for block in event.content:
            if isinstance(block, TextBlock):
                print(block.text)
    elif event.type == "tool_execution":
        print(f"Tool: {event.tool_name} -> {event.result.output}")
    elif event.type == "result":
        print(f"Done in {event.duration_ms}ms")
```

### Bidirectional streaming (Claude only)

```python
session = await ZagBuilder() \
    .provider("claude") \
    .exec_streaming("start a conversation")

# Send additional messages
await session.send_user_message("now do something else")

# Read events
async for event in await session.events():
    print(event.type)

# Wait for completion
await session.wait()
```

### JSON schema output

```python
import json

output = await ZagBuilder() \
    .provider("claude") \
    .json_schema({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "issues": {"type": "array", "items": {"type": "string"}},
        },
        "required": ["summary", "issues"],
    }) \
    .exec("analyze code quality")

parsed = json.loads(output.result)
print(parsed["summary"])
```

### Error handling

```python
from zag import ZagBuilder, ZagError, ZagFeatureUnsupportedError

try:
    output = await ZagBuilder() \
        .provider("claude") \
        .exec("do something")
except ZagFeatureUnsupportedError as e:
    print(f"{e.method} not supported on {e.provider}; try: {e.supported_providers}")
except ZagError as e:
    print(f"Exit code: {e.exit_code}")
    print(f"Stderr: {e.stderr}")
```

Capability-gated builder methods (`worktree()`, `sandbox()`, `system_prompt()`, `add_dir()`, `max_turns()`, `exec_streaming()`) trigger a preflight against the provider's capability matrix from `zag discover --json`. If the configured provider doesn't support a feature, `ZagFeatureUnsupportedError` is raised before the subprocess is spawned, with the message:

```
Provider 'ollama' does not support streaming_input (required by exec_streaming()). Supported providers: claude
```

The capability matrix is cached per binary path for the life of the process.

### Discovery

```python
from zag import list_providers, get_capability, resolve_model

providers = await list_providers()
# ["claude", "codex", "gemini", "copilot", "ollama"]

cap = await get_capability("claude")
print(cap.default_model)            # "sonnet"
print(cap.available_models)         # ["opus", "sonnet", "haiku", ...]
print(cap.features.worktree)        # FeatureSupport(supported=True, native=True)

resolved = await resolve_model("claude", "small")
# ResolvedModel(input="small", resolved="haiku", is_alias=True, provider="claude")
```

## Internals

### How it works

The SDK spawns the `zag` CLI as a subprocess using `asyncio.create_subprocess_exec()` and parses JSON/NDJSON output into typed dataclasses.

### CLI argument construction

Arguments are split into two groups:

**Global args** (before the subcommand): `--provider`, `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir`, `--file`, `--env`, `-w`/`--worktree`, `--sandbox`, `--verbose`, `--quiet`, `--debug`, `--session`, `--max-turns`, `--mcp-config`, `--show-usage`, `--size`

**Exec args** (after `exec`): `--json`, `--json-schema`, `-o`/`--output`, `-i`/`--input-format`, `--replay-user-messages`, `--include-partial-messages`, `--timeout`

### Default behaviors

- `exec()` automatically adds `-o json` when no explicit `output_format` is set, so the output can be parsed as structured `AgentOutput`.
- `stream()` adds `-o stream-json` for NDJSON event output (unless an explicit `output_format` is set).
- `exec_streaming()` forces `-i stream-json`, `-o stream-json`, and `--replay-user-messages` for bidirectional communication.
- `run()` inherits stdin/stdout/stderr for interactive terminal use.
- `resume()` dispatches to `run --resume <id>`.
- `continue_last()` dispatches to `run --continue`.
- `exec_resume()` dispatches to `exec [exec-args...] --resume <id> <prompt>` non-interactively. Returns structured `AgentOutput`.
- `exec_continue()` dispatches to `exec [exec-args...] --continue <prompt>` non-interactively. Returns structured `AgentOutput`.
- `stream_resume()` like `exec_resume()` but with `-o stream-json` for streaming events.
- `stream_continue()` like `exec_continue()` but with `-o stream-json` for streaming events.

### Version checking

The SDK checks the installed `zag` CLI version (via `zag --version`) once per process and caches the result. Methods that require newer CLI versions raise a clear error:

| Method | Minimum CLI version |
|--------|-------------------|
| `env()` | 0.6.0 |
| `mcp_config()` | 0.6.0 |
| All others | 0.2.3 |

## Provider-Specific Notes

- **Claude only**: `input_format()`, `replay_user_messages()`, `include_partial_messages()`, `mcp_config()`, `exec_streaming()`
- **Ollama only**: `size()`
- Size aliases (`"small"`, `"medium"`, `"large"`) are resolved by the CLI to provider-specific model names.
- Providers: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`. Use `"auto"` for automatic provider selection.
