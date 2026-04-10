import type { AgentOutput, Event } from "./types.js";
import {
  defaultBin,
  execZag,
  runZag,
  streamZag,
  streamWithInput,
} from "./process.js";
import type { StreamingSession } from "./process.js";
import {
  checkVersion,
  type VersionRequirement,
} from "./version.js";
import {
  checkCapability,
  type FeatureRequirement,
} from "./capability.js";

/**
 * Fluent builder for configuring and running zag agent sessions.
 *
 * @example
 * ```ts
 * import { ZagBuilder } from "zag-agent";
 *
 * const output = await new ZagBuilder()
 *   .provider("claude")
 *   .model("sonnet")
 *   .autoApprove()
 *   .exec("write a hello world program");
 *
 * console.log(output.result);
 * ```
 */
export class ZagBuilder {
  private _bin: string = defaultBin();
  private _provider?: string;
  private _model?: string;
  private _systemPrompt?: string;
  private _root?: string;
  private _autoApprove = false;
  private _addDirs: string[] = [];
  private _files: string[] = [];
  private _envVars: string[] = [];
  private _json = false;
  private _jsonSchema?: object;
  private _worktree?: string | true;
  private _sandbox?: string | true;
  private _verbose = false;
  private _quiet = false;
  private _debug = false;
  private _sessionId?: string;
  private _outputFormat?: string;
  private _inputFormat?: string;
  private _replayUserMessages = false;
  private _includePartialMessages = false;
  private _maxTurns?: number;
  private _timeout?: string;
  private _mcpConfig?: string;
  private _showUsage = false;
  private _size?: string;

  /** Override the zag binary path (default: `ZAG_BIN` env or `"zag"`). */
  bin(path: string): this {
    this._bin = path;
    return this;
  }

  /** Set the provider (e.g., "claude", "codex", "gemini", "copilot", "ollama"). */
  provider(p: string): this {
    this._provider = p;
    return this;
  }

  /** Set the model (e.g., "sonnet", "opus", "small", "large"). */
  model(m: string): this {
    this._model = m;
    return this;
  }

  /** Set a system prompt to configure agent behavior. */
  systemPrompt(p: string): this {
    this._systemPrompt = p;
    return this;
  }

  /** Set the root directory for the agent to operate in. */
  root(r: string): this {
    this._root = r;
    return this;
  }

  /** Enable auto-approve mode (skip permission prompts). */
  autoApprove(a = true): this {
    this._autoApprove = a;
    return this;
  }

  /** Add an additional directory for the agent to include. */
  addDir(d: string): this {
    this._addDirs.push(d);
    return this;
  }

  /** Attach a file to the prompt (chainable). */
  file(path: string): this {
    this._files.push(path);
    return this;
  }

  /** Add an environment variable for the agent subprocess. */
  env(key: string, value: string): this {
    this._envVars.push(`${key}=${value}`);
    return this;
  }

  /** Request JSON output from the agent. */
  json(): this {
    this._json = true;
    return this;
  }

  /** Set a JSON schema for structured output validation. Implies json(). */
  jsonSchema(s: object): this {
    this._jsonSchema = s;
    this._json = true;
    return this;
  }

  /** Enable worktree mode with an optional name. */
  worktree(name?: string): this {
    this._worktree = name ?? true;
    return this;
  }

  /** Enable sandbox mode with an optional name. */
  sandbox(name?: string): this {
    this._sandbox = name ?? true;
    return this;
  }

  /** Enable verbose output. */
  verbose(v = true): this {
    this._verbose = v;
    return this;
  }

  /** Enable quiet mode. */
  quiet(q = true): this {
    this._quiet = q;
    return this;
  }

  /** Enable debug logging. */
  debug(d = true): this {
    this._debug = d;
    return this;
  }

  /** Pre-set a session ID (UUID). */
  sessionId(id: string): this {
    this._sessionId = id;
    return this;
  }

  /** Set the output format (e.g., "text", "json", "json-pretty", "stream-json"). */
  outputFormat(f: string): this {
    this._outputFormat = f;
    return this;
  }

  /** Set the input format (Claude only, e.g., "text", "stream-json"). */
  inputFormat(f: string): this {
    this._inputFormat = f;
    return this;
  }

  /** Re-emit user messages from stdin on stdout (Claude only). */
  replayUserMessages(r = true): this {
    this._replayUserMessages = r;
    return this;
  }

  /** Include partial message chunks in streaming output (Claude only). */
  includePartialMessages(i = true): this {
    this._includePartialMessages = i;
    return this;
  }

  /** Set the maximum number of agentic turns. */
  maxTurns(n: number): this {
    this._maxTurns = n;
    return this;
  }

  /** Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. */
  timeout(t: string): this {
    this._timeout = t;
    return this;
  }

  /** Set MCP server config for this invocation: JSON string or file path (Claude only). */
  mcpConfig(c: string): this {
    this._mcpConfig = c;
    return this;
  }

  /** Show token usage statistics (only applies to JSON output mode). */
  showUsage(s = true): this {
    this._showUsage = s;
    return this;
  }

  /** Set the Ollama model parameter size (e.g., "2b", "9b", "35b"). */
  size(s: string): this {
    this._size = s;
    return this;
  }

  /** Collect version requirements for features that were added after the initial release. */
  private versionRequirements(): VersionRequirement[] {
    return [
      { method: "env()", version: "0.6.0", isSet: this._envVars.length > 0 },
      { method: "mcpConfig()", version: "0.6.0", isSet: this._mcpConfig != null },
    ];
  }

  /**
   * Collect provider-capability requirements for options that are only
   * supported by a subset of providers. The preflight `checkCapability()`
   * helper uses this list to fail fast with a typed
   * `ZagFeatureUnsupportedError`.
   *
   * Note: `mcpConfig()` is intentionally omitted — there is no `mcp_config`
   * field on the provider `Features` struct yet, so there is nothing to
   * validate against. Track that gap separately.
   */
  private capabilityRequirements(): FeatureRequirement[] {
    return [
      {
        method: "worktree()",
        feature: "worktree",
        isSet: this._worktree != null,
      },
      {
        method: "sandbox()",
        feature: "sandbox",
        isSet: this._sandbox != null,
      },
      {
        method: "systemPrompt()",
        feature: "system_prompt",
        isSet: this._systemPrompt != null,
      },
      {
        method: "addDir()",
        feature: "add_dirs",
        isSet: this._addDirs.length > 0,
      },
    ];
  }

  /** Build the shared CLI flags (provider, model, session isolation, etc.). */
  private buildGlobalArgs(): string[] {
    const args: string[] = [];
    if (this._provider) args.push("-p", this._provider);
    if (this._model) args.push("--model", this._model);
    if (this._systemPrompt) args.push("--system-prompt", this._systemPrompt);
    if (this._root) args.push("--root", this._root);
    if (this._autoApprove) args.push("--auto-approve");
    for (const d of this._addDirs) args.push("--add-dir", d);
    for (const f of this._files) args.push("--file", f);
    for (const e of this._envVars) args.push("--env", e);
    if (this._worktree === true) {
      args.push("-w");
    } else if (typeof this._worktree === "string") {
      args.push("-w", this._worktree);
    }
    if (this._sandbox === true) {
      args.push("--sandbox");
    } else if (typeof this._sandbox === "string") {
      args.push("--sandbox", this._sandbox);
    }
    if (this._verbose) args.push("--verbose");
    if (this._quiet) args.push("--quiet");
    if (this._debug) args.push("--debug");
    if (this._sessionId) args.push("--session", this._sessionId);
    if (this._maxTurns != null) args.push("--max-turns", String(this._maxTurns));
    if (this._mcpConfig) args.push("--mcp-config", this._mcpConfig);
    if (this._showUsage) args.push("--show-usage");
    if (this._size) args.push("--size", this._size);
    return args;
  }

  /** Build CLI args for exec mode. */
  private buildExecArgs(prompt: string, streaming: boolean): string[] {
    const args = ["exec", ...this.buildGlobalArgs()];
    if (this._json) args.push("--json");
    if (this._jsonSchema) {
      args.push("--json-schema", JSON.stringify(this._jsonSchema));
    }
    if (this._outputFormat) {
      args.push("-o", this._outputFormat);
    } else if (streaming) {
      args.push("-o", "stream-json");
    } else {
      // For non-streaming exec, default to json output for structured parsing
      args.push("-o", "json");
    }
    if (this._inputFormat) args.push("-i", this._inputFormat);
    if (this._replayUserMessages) args.push("--replay-user-messages");
    if (this._includePartialMessages) args.push("--include-partial-messages");
    if (this._timeout) args.push("--timeout", this._timeout);
    args.push(prompt);
    return args;
  }

  /**
   * Run the agent non-interactively and return structured output.
   *
   * @example
   * ```ts
   * const output = await new ZagBuilder()
   *   .provider("claude")
   *   .exec("say hello");
   * console.log(output.result);
   * ```
   */
  async exec(prompt: string): Promise<AgentOutput> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = this.buildExecArgs(prompt, false);
    return execZag(this._bin, args);
  }

  /**
   * Run the agent in streaming mode, yielding events as they arrive.
   *
   * @example
   * ```ts
   * for await (const event of new ZagBuilder()
   *   .provider("claude")
   *   .stream("analyze this code")) {
   *   console.log(event.type);
   * }
   * ```
   */
  async *stream(prompt: string): AsyncGenerator<Event> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = this.buildExecArgs(prompt, true);
    yield* streamZag(this._bin, args);
  }

  /**
   * Run the agent with streaming input and output (Claude only).
   *
   * Returns a StreamingSession with piped stdin for sending NDJSON messages
   * and an async iterator for reading events. Automatically enables
   * `--input-format stream-json`, `--replay-user-messages`, and
   * `-o stream-json`.
   *
   * @example
   * ```ts
   * const session = new ZagBuilder()
   *   .provider("claude")
   *   .execStreaming("initial prompt");
   *
   * session.send('{"type":"user_message","content":"hello"}');
   *
   * for await (const event of session.events()) {
   *   console.log(event.type);
   * }
   *
   * await session.wait();
   * ```
   */
  async execStreaming(prompt: string): Promise<StreamingSession> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(this._bin, this._provider, [
      ...this.capabilityRequirements(),
      {
        method: "execStreaming()",
        feature: "streaming_input",
        isSet: true,
      },
    ]);
    const args = ["exec", ...this.buildGlobalArgs()];
    args.push("-i", "stream-json");
    args.push("-o", "stream-json");
    args.push("--replay-user-messages");
    if (this._includePartialMessages) args.push("--include-partial-messages");
    if (this._outputFormat) args.push("-o", this._outputFormat);
    args.push(prompt);
    return streamWithInput(this._bin, args);
  }

  /**
   * Start an interactive agent session.
   * Inherits stdin/stdout/stderr.
   */
  async run(prompt?: string): Promise<void> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = ["run", ...this.buildGlobalArgs()];
    if (this._json) args.push("--json");
    if (this._jsonSchema) {
      args.push("--json-schema", JSON.stringify(this._jsonSchema));
    }
    if (prompt) args.push(prompt);
    return runZag(this._bin, args);
  }

  /** Resume a previous session by ID. */
  async resume(sessionId: string): Promise<void> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = ["run", ...this.buildGlobalArgs(), "--resume", sessionId];
    return runZag(this._bin, args);
  }

  /** Resume the most recent session. */
  async continueLast(): Promise<void> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = ["run", ...this.buildGlobalArgs(), "--continue"];
    return runZag(this._bin, args);
  }

  /**
   * Resume a previous session non-interactively with a follow-up prompt.
   *
   * @example
   * ```ts
   * const output = await new ZagBuilder()
   *   .provider("claude")
   *   .execResume("session-id", "what about tests?");
   * console.log(output.result);
   * ```
   */
  async execResume(sessionId: string, prompt: string): Promise<AgentOutput> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = this.buildExecArgs(prompt, false);
    // Insert --resume before the prompt positional arg
    const promptIdx = args.lastIndexOf(prompt);
    args.splice(promptIdx, 0, "--resume", sessionId);
    return execZag(this._bin, args);
  }

  /**
   * Resume the most recent session non-interactively with a follow-up prompt.
   *
   * @example
   * ```ts
   * const output = await new ZagBuilder()
   *   .provider("claude")
   *   .execContinue("what about tests?");
   * console.log(output.result);
   * ```
   */
  async execContinue(prompt: string): Promise<AgentOutput> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = this.buildExecArgs(prompt, false);
    const promptIdx = args.lastIndexOf(prompt);
    args.splice(promptIdx, 0, "--continue");
    return execZag(this._bin, args);
  }

  /** Resume a previous session in streaming mode with a follow-up prompt. */
  async *streamResume(
    sessionId: string,
    prompt: string,
  ): AsyncGenerator<Event> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = this.buildExecArgs(prompt, true);
    const promptIdx = args.lastIndexOf(prompt);
    args.splice(promptIdx, 0, "--resume", sessionId);
    yield* streamZag(this._bin, args);
  }

  /** Resume the most recent session in streaming mode with a follow-up prompt. */
  async *streamContinue(prompt: string): AsyncGenerator<Event> {
    await checkVersion(this._bin, this.versionRequirements());
    await checkCapability(
      this._bin,
      this._provider,
      this.capabilityRequirements(),
    );
    const args = this.buildExecArgs(prompt, true);
    const promptIdx = args.lastIndexOf(prompt);
    args.splice(promptIdx, 0, "--continue");
    yield* streamZag(this._bin, args);
  }
}
