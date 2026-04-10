# TypeScript Binding Reference -- @nlindstedt/zag-agent

Comprehensive reference for the TypeScript binding of zag, a unified CLI for AI coding agents.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Builder API](#builder-api)
  - [Configuration Methods](#configuration-methods)
  - [Terminal Methods](#terminal-methods)
- [StreamingSession](#streamingsession)
- [Types](#types)
  - [AgentOutput](#agentoutput)
  - [Usage](#usage)
  - [Event](#event)
  - [ContentBlock](#contentblock)
  - [ToolResult](#toolresult)
  - [ZagError](#zagerror)
  - [Discovery Types](#discovery-types)
- [Discovery API](#discovery-api)
- [Examples](#examples)
  - [Basic Exec](#basic-exec)
  - [Streaming Events](#streaming-events)
  - [Bidirectional Streaming](#bidirectional-streaming)
  - [JSON Schema Validation](#json-schema-validation)
  - [Error Handling](#error-handling)
  - [Provider Discovery](#provider-discovery)
- [Internals](#internals)
  - [CLI Arg Construction](#cli-arg-construction)
  - [Default Behaviors](#default-behaviors)
  - [Version Checking](#version-checking)
- [Provider Notes](#provider-notes)

---

## Quick Start

**Prerequisites:** Node.js 18+, `zag` CLI binary on `PATH` (or set via `ZAG_BIN` environment variable).

```bash
npm install @nlindstedt/zag-agent
```

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

const output = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .autoApprove()
  .exec("write a hello world program");

console.log(output.result);
```

The package has zero external dependencies (Node.js built-ins only). It works by spawning the `zag` CLI as a subprocess.

---

## Builder API

Constructor: `new ZagBuilder()`

All setter methods return `this` for chaining.

### Configuration Methods

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `bin` | `bin(path: string): this` | N/A (binding-only) | Override zag binary path. Default: `ZAG_BIN` env var, or `"zag"`. |
| `provider` | `provider(p: string): this` | `-p, --provider` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`. |
| `model` | `model(m: string): this` | `--model` | Set model name or size alias (`"small"`, `"medium"`, `"large"`). |
| `systemPrompt` | `systemPrompt(p: string): this` | `--system-prompt` | Set system prompt for agent behavior. |
| `root` | `root(r: string): this` | `--root` | Set working directory for the agent. |
| `autoApprove` | `autoApprove(a = true): this` | `--auto-approve` | Skip permission prompts. Pass `false` to disable. |
| `addDir` | `addDir(d: string): this` | `--add-dir` | Add an additional directory. Chainable and repeatable; each call appends. |
| `file` | `file(path: string): this` | `--file` | Attach a file to the prompt. Chainable and repeatable; each call appends. |
| `env` | `env(key: string, value: string): this` | `--env KEY=VALUE` | Add an environment variable for the agent subprocess. Requires CLI >= 0.6.0. |
| `json` | `json(): this` | `--json` | Request JSON output from the agent. |
| `jsonSchema` | `jsonSchema(s: object): this` | `--json-schema` | Set a JSON schema for structured output validation. Implies `.json()`. |
| `worktree` | `worktree(name?: string): this` | `-w, --worktree [NAME]` | Git worktree isolation. No argument = auto-generated name. |
| `sandbox` | `sandbox(name?: string): this` | `--sandbox [NAME]` | Docker sandbox isolation. No argument = auto-generated name. |
| `verbose` | `verbose(v = true): this` | `--verbose` | Enable verbose output. Pass `false` to disable. |
| `quiet` | `quiet(q = true): this` | `--quiet` | Suppress non-essential output. Pass `false` to disable. |
| `debug` | `debug(d = true): this` | `--debug` | Enable debug logging (binding-only). Pass `false` to disable. |
| `sessionId` | `sessionId(id: string): this` | `--session UUID` | Pre-set a session ID. |
| `outputFormat` | `outputFormat(f: string): this` | `-o, --output` | Output format: `"text"`, `"json"`, `"json-pretty"`, `"stream-json"`. |
| `inputFormat` | `inputFormat(f: string): this` | `-i, --input-format` | Input format: `"text"`, `"stream-json"`. Claude only. |
| `replayUserMessages` | `replayUserMessages(r = true): this` | `--replay-user-messages` | Re-emit user messages on stdout. Claude only. |
| `includePartialMessages` | `includePartialMessages(i = true): this` | `--include-partial-messages` | Include partial message chunks. Claude only. |
| `maxTurns` | `maxTurns(n: number): this` | `--max-turns` | Maximum number of agentic turns. |
| `timeout` | `timeout(t: string): this` | `--timeout` | Timeout duration (e.g., `"30s"`, `"5m"`, `"1h"`). Kills the agent if exceeded. |
| `mcpConfig` | `mcpConfig(c: string): this` | `--mcp-config` | MCP server config: JSON string or file path. Claude only. Requires CLI >= 0.6.0. |
| `showUsage` | `showUsage(s = true): this` | `--show-usage` | Show token usage statistics (JSON output mode). |
| `size` | `size(s: string): this` | `--size` | Ollama parameter size (e.g., `"2b"`, `"9b"`, `"35b"`). Ollama only. |
| `autoCleanup` | `autoCleanup(enabled = true): this` | *(binding-only)* | Opt in to process-wide orphan cleanup for `StreamingSession`s produced by this builder. On first use, installs idempotent `exit` / `SIGINT` / `SIGTERM` / `SIGHUP` / `uncaughtException` handlers that SIGTERM every tracked live child on parent exit. Off by default. |

### Terminal Methods

These methods execute the builder configuration. Each spawns a `zag` subprocess.

| Method | Signature | Description |
|--------|-----------|-------------|
| `exec` | `async exec(prompt: string): Promise<AgentOutput>` | Non-interactive execution. Returns structured output parsed from JSON. |
| `stream` | `async *stream(prompt: string): AsyncGenerator<Event>` | Stream NDJSON events as they arrive. Yields parsed `Event` objects. |
| `execStreaming` | `async execStreaming(prompt: string): Promise<StreamingSession>` | Bidirectional streaming session (Claude only). Returns a `StreamingSession` for sending input and reading events. |
| `run` | `async run(prompt?: string): Promise<void>` | Interactive session. Inherits stdin/stdout/stderr from the parent process. Prompt is optional. |
| `resume` | `async resume(sessionId: string): Promise<void>` | Resume a previous session by its ID. Runs interactively. |
| `continueLast` | `async continueLast(): Promise<void>` | Resume the most recent session. Runs interactively. |
| `execResume` | `async execResume(sessionId: string, prompt: string): Promise<AgentOutput>` | Resume a session non-interactively with a follow-up prompt. |
| `execContinue` | `async execContinue(prompt: string): Promise<AgentOutput>` | Resume the most recent session non-interactively with a follow-up prompt. |
| `streamResume` | `async *streamResume(sessionId: string, prompt: string): AsyncGenerator<Event>` | Resume a session in streaming mode with a follow-up prompt. |
| `streamContinue` | `async *streamContinue(prompt: string): AsyncGenerator<Event>` | Resume the most recent session in streaming mode with a follow-up prompt. |

---

## StreamingSession

Returned by `execStreaming()`. Provides bidirectional communication with a running agent process.

```typescript
interface StreamingSession {
  send(message: string): void;
  sendUserMessage(content: string): void;
  closeInput(): void;
  events(): AsyncGenerator<Event>;
  readonly isRunning: boolean;
  terminate(): void;
  wait(): Promise<void>;
  close(options?: { timeout?: number | string }): Promise<void>;
}
```

| Method / Property | Signature | Description |
|-------------------|-----------|-------------|
| `send` | `send(message: string): void` | Write a raw NDJSON line to the agent's stdin. A trailing newline is appended automatically. |
| `sendUserMessage` | `sendUserMessage(content: string): void` | Send a `user_message` JSON object to the agent. Serializes `{ type: "user_message", content }` and writes it as NDJSON. |
| `closeInput` | `closeInput(): void` | Close stdin to signal that no more input will be sent. |
| `events` | `events(): AsyncGenerator<Event>` | Async iterator over parsed `Event` objects from stdout. Unparseable lines are silently skipped. |
| `isRunning` | `readonly isRunning: boolean` | Whether the child process is still running. |
| `terminate` | `terminate(): void` | Send `SIGTERM` to the child process. No-op if already exited. |
| `wait` | `wait(): Promise<void>` | Wait for the process to exit. Resolves on exit code 0. Throws `ZagError` on non-zero exit. |
| `close` | `close(options?: { timeout?: number \| string }): Promise<void>` | Graceful shutdown helper. Closes stdin, waits up to half the budget, escalates to `SIGTERM`, and finally `SIGKILL`. `timeout` is a number in ms or a humantime string (`"5s"`, `"500ms"`, `"1m"`, `"1h"`); defaults to 5000 ms. Resolves on exit regardless of exit code. Idempotent — concurrent calls share the same promise. |

---

## Types

All types are importable from `@nlindstedt/zag-agent`:

```typescript
import type {
  AgentOutput,
  Usage,
  Event,
  InitEvent,
  UserMessageEvent,
  AssistantMessageEvent,
  ToolExecutionEvent,
  ResultEvent,
  ErrorEvent,
  PermissionRequestEvent,
  ContentBlock,
  TextBlock,
  ToolUseBlock,
  ToolResult,
  ProviderCapability,
  Features,
  FeatureSupport,
  SizeMappings,
  SessionLogSupport,
  ResolvedModel,
} from "@nlindstedt/zag-agent";

import { ZagError } from "@nlindstedt/zag-agent";
```

### AgentOutput

Unified output from a non-interactive agent session.

```typescript
interface AgentOutput {
  /** Provider name (e.g., "claude", "codex"). */
  agent: string;

  /** Unique session identifier. */
  session_id: string;

  /** Ordered list of events that occurred during the session. */
  events: Event[];

  /** Final text result, or null if the session produced no text output. */
  result: string | null;

  /** Whether the session ended with an error. */
  is_error: boolean;

  /** Process exit code, if available. */
  exit_code?: number | null;

  /** Human-readable error message, if the session failed. */
  error_message?: string | null;

  /** Total estimated cost in USD, or null if unavailable. */
  total_cost_usd: number | null;

  /** Aggregated token usage for the session, or null if unavailable. */
  usage: Usage | null;
}
```

### Usage

Token usage statistics for a session.

```typescript
interface Usage {
  /** Number of input tokens consumed. */
  input_tokens: number;

  /** Number of output tokens generated. */
  output_tokens: number;

  /** Tokens read from cache (Claude-specific). */
  cache_read_tokens?: number;

  /** Tokens written to cache (Claude-specific). */
  cache_creation_tokens?: number;

  /** Number of web search requests made (Gemini-specific). */
  web_search_requests?: number;

  /** Number of web fetch requests made (Gemini-specific). */
  web_fetch_requests?: number;
}
```

### Event

Tagged union of all event types. Discriminate on the `type` field.

```typescript
type Event =
  | InitEvent
  | UserMessageEvent
  | AssistantMessageEvent
  | ToolExecutionEvent
  | ResultEvent
  | ErrorEvent
  | PermissionRequestEvent;
```

#### InitEvent

Emitted once at the start of a session.

```typescript
interface InitEvent {
  type: "init";

  /** Model used for this session. */
  model: string;

  /** Names of tools available to the agent. */
  tools: string[];

  /** Working directory the agent is operating in. */
  working_directory: string | null;

  /** Provider-specific metadata. */
  metadata: Record<string, unknown>;
}
```

#### UserMessageEvent

A user message sent to the agent.

```typescript
interface UserMessageEvent {
  type: "user_message";

  /** Content blocks composing the user message. */
  content: ContentBlock[];
}
```

#### AssistantMessageEvent

A response from the agent.

```typescript
interface AssistantMessageEvent {
  type: "assistant_message";

  /** Content blocks composing the assistant response. */
  content: ContentBlock[];

  /** Token usage for this individual message. */
  usage: Usage | null;
}
```

#### ToolExecutionEvent

A tool invocation and its result.

```typescript
interface ToolExecutionEvent {
  type: "tool_execution";

  /** Name of the tool that was invoked. */
  tool_name: string;

  /** Unique identifier for this tool invocation. */
  tool_id: string;

  /** Input parameters passed to the tool. */
  input: unknown;

  /** Result returned by the tool. */
  result: ToolResult;
}
```

#### ResultEvent

Final summary of the session.

```typescript
interface ResultEvent {
  type: "result";

  /** Whether the session completed successfully. */
  success: boolean;

  /** Final result message. */
  message: string | null;

  /** Total session duration in milliseconds. */
  duration_ms: number | null;

  /** Number of agentic turns taken. */
  num_turns: number | null;
}
```

#### ErrorEvent

An error that occurred during the session.

```typescript
interface ErrorEvent {
  type: "error";

  /** Human-readable error message. */
  message: string;

  /** Additional structured error details. */
  details: unknown | null;
}
```

#### PermissionRequestEvent

A permission prompt and its resolution.

```typescript
interface PermissionRequestEvent {
  type: "permission_request";

  /** Tool that requested permission. */
  tool_name: string;

  /** Description of what the tool wants to do. */
  description: string;

  /** Whether permission was granted. */
  granted: boolean;
}
```

### ContentBlock

A block of content within a message.

```typescript
type ContentBlock = TextBlock | ToolUseBlock;

interface TextBlock {
  type: "text";

  /** The text content. */
  text: string;
}

interface ToolUseBlock {
  type: "tool_use";

  /** Unique identifier for this tool use. */
  id: string;

  /** Name of the tool being used. */
  name: string;

  /** Input parameters for the tool. */
  input: unknown;
}
```

### ToolResult

Result from a tool execution.

```typescript
interface ToolResult {
  /** Whether the tool executed successfully. */
  success: boolean;

  /** Tool output text, if any. */
  output: string | null;

  /** Error message, if the tool failed. */
  error: string | null;

  /** Structured data returned by the tool, if any. */
  data: unknown | null;
}
```

### ZagError

Error class thrown when the `zag` process fails. This is a concrete class, not an interface.

```typescript
class ZagError extends Error {
  /** Process exit code, or null if the process failed to spawn. */
  readonly exitCode: number | null;

  /** Captured stderr output from the process. */
  readonly stderr: string;

  constructor(message: string, exitCode: number | null, stderr: string);
}
```

### Discovery Types

```typescript
interface FeatureSupport {
  /** Whether the feature is supported at all. */
  supported: boolean;

  /** Whether support is native to the provider (vs. emulated by zag). */
  native: boolean;
}

interface SessionLogSupport {
  supported: boolean;
  native: boolean;

  /** Completeness level of session logs (e.g., "full", "partial"). */
  completeness?: string;
}

interface SizeMappings {
  /** Model name for the "small" alias. */
  small: string;

  /** Model name for the "medium" alias. */
  medium: string;

  /** Model name for the "large" alias. */
  large: string;
}

interface Features {
  interactive: FeatureSupport;
  non_interactive: FeatureSupport;
  resume: FeatureSupport;
  resume_with_prompt: FeatureSupport;
  session_logs: SessionLogSupport;
  json_output: FeatureSupport;
  stream_json: FeatureSupport;
  json_schema: FeatureSupport;
  input_format: FeatureSupport;
  streaming_input: FeatureSupport;
  worktree: FeatureSupport;
  sandbox: FeatureSupport;
  system_prompt: FeatureSupport;
  auto_approve: FeatureSupport;
  review: FeatureSupport;
  add_dirs: FeatureSupport;
  max_turns: FeatureSupport;
}

interface ProviderCapability {
  /** Provider name (e.g., "claude"). */
  provider: string;

  /** Default model used when none is specified. */
  default_model: string;

  /** All models available for this provider. */
  available_models: string[];

  /** Mapping of size aliases to concrete model names. */
  size_mappings: SizeMappings;

  /** Feature support declarations. */
  features: Features;
}

interface ResolvedModel {
  /** The input that was passed to resolveModel(). */
  input: string;

  /** The resolved concrete model name. */
  resolved: string;

  /** Whether the input was a size alias that got resolved. */
  is_alias: boolean;

  /** The provider this model belongs to. */
  provider: string;
}
```

---

## Discovery API

Standalone async functions for querying provider capabilities. These are top-level exports, not methods on `ZagBuilder`.

```typescript
import {
  listProviders,
  getCapability,
  getAllCapabilities,
  resolveModel,
} from "@nlindstedt/zag-agent";
```

### listProviders

```typescript
async function listProviders(bin?: string): Promise<string[]>
```

Returns an array of available provider names (e.g., `["claude", "codex", "gemini", "copilot", "ollama"]`). Internally calls `getAllCapabilities()` and extracts provider names.

### getCapability

```typescript
async function getCapability(provider: string, bin?: string): Promise<ProviderCapability>
```

Returns the full capability declaration for a specific provider. Runs `zag discover -p <provider> --json`.

### getAllCapabilities

```typescript
async function getAllCapabilities(bin?: string): Promise<ProviderCapability[]>
```

Returns capability declarations for all providers. Runs `zag discover --json`.

### resolveModel

```typescript
async function resolveModel(provider: string, model: string, bin?: string): Promise<ResolvedModel>
```

Resolves a model name or size alias (`small`/`s`, `medium`/`m`/`default`, `large`/`l`/`max`) to the provider-specific concrete model name. Non-alias names pass through unchanged. Runs `zag discover -p <provider> --resolve <model> --json`.

All discovery functions accept an optional `bin` parameter to override the zag binary path (default: `ZAG_BIN` env var or `"zag"`).

---

## Examples

### Basic Exec

Run a prompt non-interactively and get structured output:

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

const output = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .autoApprove()
  .root("/path/to/project")
  .maxTurns(10)
  .exec("refactor the auth module");

if (output.is_error) {
  console.error(output.error_message);
} else {
  console.log(output.result);
  console.log(`Session: ${output.session_id}`);
  console.log(`Cost: $${output.total_cost_usd}`);
  console.log(`Tokens: ${output.usage?.input_tokens} in, ${output.usage?.output_tokens} out`);
}
```

### Streaming Events

Process events as they arrive:

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

const builder = new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .autoApprove();

for await (const event of builder.stream("analyze this codebase")) {
  switch (event.type) {
    case "init":
      console.log(`Model: ${event.model}, Tools: ${event.tools.join(", ")}`);
      break;
    case "assistant_message":
      for (const block of event.content) {
        if (block.type === "text") {
          process.stdout.write(block.text);
        }
      }
      break;
    case "tool_execution":
      console.log(`Tool: ${event.tool_name} -> ${event.result.success ? "ok" : "fail"}`);
      break;
    case "result":
      console.log(`Done in ${event.duration_ms}ms, ${event.num_turns} turns`);
      break;
    case "error":
      console.error(`Error: ${event.message}`);
      break;
  }
}
```

### Bidirectional Streaming

Send follow-up messages during a session (Claude only):

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

const session = await new ZagBuilder()
  .provider("claude")
  .autoApprove()
  .execStreaming("start a code review");

// Read events in the background
const eventStream = session.events();

(async () => {
  for await (const event of eventStream) {
    if (event.type === "assistant_message") {
      for (const block of event.content) {
        if (block.type === "text") {
          process.stdout.write(block.text);
        }
      }
    }
  }
})();

// Send a follow-up message
session.sendUserMessage("now focus on error handling");

// When done, close input and wait for exit
session.closeInput();
await session.wait();
```

### Graceful shutdown and orphan cleanup

`StreamingSession.close({ timeout })` encapsulates the full shutdown dance
so you don't have to: it closes stdin, waits for the child to exit on its
own, escalates to SIGTERM, and finally SIGKILL. It never throws for
non-zero exit codes — it is purely a cleanup helper. Use it inside
request handlers (SSE, WebSocket) when the client disconnects:

```typescript
const session = await new ZagBuilder()
  .provider("claude")
  .execStreaming("initial prompt");

request.on("close", async () => {
  await session.close({ timeout: "5s" });
});
```

For long-running Node servers where the parent process itself can die
unexpectedly (uncaught exception, SIGINT, container shutdown), opt in to
`.autoCleanup()` on the builder. The SDK will install process-wide
shutdown handlers once and SIGTERM every tracked live session:

```typescript
const builder = new ZagBuilder().provider("claude").autoCleanup();
const session = await builder.execStreaming("hello");
// No orphan `zag`/agent processes left behind if the Node parent crashes.
```

### JSON Schema Validation

Request structured output matching a schema:

```typescript
import { ZagBuilder } from "@nlindstedt/zag-agent";

const schema = {
  type: "object",
  properties: {
    summary: { type: "string" },
    issues: {
      type: "array",
      items: {
        type: "object",
        properties: {
          file: { type: "string" },
          line: { type: "number" },
          severity: { type: "string", enum: ["error", "warning", "info"] },
          message: { type: "string" },
        },
        required: ["file", "line", "severity", "message"],
      },
    },
  },
  required: ["summary", "issues"],
};

const output = await new ZagBuilder()
  .provider("claude")
  .autoApprove()
  .jsonSchema(schema)
  .exec("review this codebase for common issues");

// output.result contains a JSON string conforming to the schema
const review = JSON.parse(output.result!);
console.log(review.summary);
for (const issue of review.issues) {
  console.log(`${issue.severity}: ${issue.file}:${issue.line} - ${issue.message}`);
}
```

### Error Handling

```typescript
import { ZagBuilder, ZagError } from "@nlindstedt/zag-agent";

try {
  const output = await new ZagBuilder()
    .provider("claude")
    .timeout("30s")
    .exec("do something");

  if (output.is_error) {
    console.error("Agent reported an error:", output.error_message);
  } else {
    console.log(output.result);
  }
} catch (err) {
  if (err instanceof ZagError) {
    console.error("Process failed:");
    console.error("  Exit code:", err.exitCode);
    console.error("  Stderr:", err.stderr);
    console.error("  Message:", err.message);
  } else {
    throw err;
  }
}
```

A `ZagError` is thrown when:

- The `zag` binary cannot be spawned (exit code is `null`).
- The process exits with a non-zero exit code.
- The JSON output cannot be parsed.
- A version requirement is not met.

### Provider Discovery

Query available providers and their capabilities:

```typescript
import {
  listProviders,
  getCapability,
  getAllCapabilities,
  resolveModel,
} from "@nlindstedt/zag-agent";

// List all provider names
const providers = await listProviders();
console.log(providers); // ["claude", "codex", "gemini", "copilot", "ollama"]

// Get capabilities for a specific provider
const claude = await getCapability("claude");
console.log(claude.default_model);
console.log(claude.available_models);
console.log(claude.features.json_output);    // { supported: true, native: true }
console.log(claude.features.worktree);       // { supported: true, native: true }
console.log(claude.size_mappings);           // { small: "...", medium: "...", large: "..." }

// Resolve a size alias
const resolved = await resolveModel("claude", "large");
console.log(resolved.input);      // "large"
console.log(resolved.resolved);   // concrete model name
console.log(resolved.is_alias);   // true
console.log(resolved.provider);   // "claude"

// Get all provider capabilities at once
const all = await getAllCapabilities();
for (const cap of all) {
  console.log(`${cap.provider}: ${cap.available_models.length} models`);
}
```

---

## Internals

### CLI Arg Construction

The builder constructs a CLI argument array split into two groups:

**Global args** (placed before the subcommand):

These are built by `buildGlobalArgs()` and include: `--provider`, `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir` (repeatable), `--file` (repeatable), `--env` (repeatable), `-w`/`--worktree`, `--sandbox`, `--verbose`, `--quiet`, `--debug`, `--session`, `--max-turns`, `--mcp-config`, `--show-usage`, `--size`.

**Exec args** (placed after the `exec` subcommand):

These are built by `buildExecArgs()` and include: `--json`, `--json-schema`, `-o` (output format), `-i` (input format), `--replay-user-messages`, `--include-partial-messages`, `--timeout`. The prompt string is always the last argument.

The final CLI invocation has the form:

```
zag [global-args...] exec [exec-args...] <prompt>
```

For `run`, `resume`, and `continueLast`, only global args are used:

```
zag [global-args...] run [prompt]
zag [global-args...] run --resume <sessionId>
zag [global-args...] run --continue
```

### Default Behaviors

- **`exec()`**: When no explicit `outputFormat` is set, automatically adds `-o json` to enable structured JSON parsing of the output.
- **`stream()`**: When no explicit `outputFormat` is set, automatically adds `-o stream-json` for NDJSON event streaming.
- **`execStreaming()`**: Forces `-i stream-json -o stream-json --replay-user-messages`. If `includePartialMessages` was set, that flag is also added. Does not use `buildExecArgs()` -- it constructs exec-specific args directly.
- **`run()`**: Inherits stdio from the parent process (interactive terminal mode). Supports optional `--json` and `--json-schema` flags. No output format is forced.
- **`resume()`**: Calls `zag [global-args...] run --resume <id>` with inherited stdio.
- **`continueLast()`**: Calls `zag [global-args...] run --continue` with inherited stdio.
- **`execResume()`**: Calls `zag exec [exec-args...] --resume <id> <prompt>` non-interactively. Returns structured `AgentOutput`.
- **`execContinue()`**: Calls `zag exec [exec-args...] --continue <prompt>` non-interactively. Returns structured `AgentOutput`.
- **`streamResume()`**: Like `execResume()` but with `-o stream-json` for streaming events.
- **`streamContinue()`**: Like `execContinue()` but with `-o stream-json` for streaming events.

### Version Checking

All terminal methods call `checkVersion()` before spawning the process. This function:

1. Collects active version requirements from the builder configuration.
2. If any requirements are active, detects the CLI version by running `zag --version`.
3. Parses the output (expected format: `zag-cli X.Y.Z` or just `X.Y.Z`).
4. Compares the detected version against each requirement using semver comparison.
5. Throws a `ZagError` with a descriptive message if any requirement is not met.

The detected version is cached per binary path for the lifetime of the process, so `zag --version` is invoked at most once per distinct binary path. If no features requiring version checks are configured, no version detection occurs at all.

| Method | Minimum CLI Version |
|--------|---------------------|
| `env()` | 0.6.0 |
| `mcpConfig()` | 0.6.0 |
| All others | 0.2.3 |

---

## Provider Notes

Features that are restricted to specific providers:

**Claude only:**

- `inputFormat()` -- set input format to `"stream-json"` for streaming input.
- `replayUserMessages()` -- re-emit user messages on stdout.
- `includePartialMessages()` -- include partial message chunks in streaming output.
- `mcpConfig()` -- configure MCP servers for the session.
- `execStreaming()` -- bidirectional streaming sessions.

**Ollama only:**

- `size()` -- set the Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`).

These methods can technically be called for any provider, but they will only have an effect (or be accepted by the CLI) for the listed providers. Use the Discovery API to check provider feature support at runtime before calling provider-specific methods.

Size aliases (`"small"`, `"medium"`, `"large"`) are resolved by the CLI to provider-specific model names. Use `resolveModel()` to preview the resolution without starting a session.
